//! Top-level Elm-architecture implementation for knotra.

mod focus_ops;
mod shared;

use focus_ops::{
    activate_focused, advance_focus, close_overlay_focus, close_topmost_layer,
    current_target_is_text_input, enter_overlay_focus, focus_search, freezer_is_running,
    open_overlay_focus, smart_pull_is_running, workspace_dialog_open,
};
use shared::{
    acquire_operation, cancel_freezer_validation, clear_sync_retry_context, find_project,
    invalidate_retry_preparation, refresh_workspace_task,
};

use iced::futures::StreamExt;
use iced::{Element, Subscription, Task, clipboard, keyboard, time};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use knotra_vcs::{
    VcsAdapter, VcsKind,
    model::{
        operation::{
            ContextSwitchResult, OperationId, OperationKind, OperationLog, OperationResult,
            ProjectOperationOutcome, ProjectOperationResult, RetryExclusionReason,
            SmartPullDisposition, SmartPullProgress, SmartPullSkipReason,
        },
        project::Project,
        workspace::Workspace,
    },
};

use crate::{
    config::{AppPaths, DashboardGrouping, load_config, save_config},
    fs_watcher::fs_watch_subscription,
    message::{
        ActivityMessage, BackgroundMessage, ChangelogMessage, ConflictOpsMessage, ContextMessage,
        DashboardMessage, FreezerMessage, HistoryMessage, KeyboardMessage, LaunchMessage, Message,
        PaletteMessage, ProjectMessage, SelectionMessage, SettingsMessage, ShortcutMessage,
        SyncMessage, TagPushMessage, TopologyMessage, WorkspaceMessage,
    },
    persistence::{
        delete_workspace_file, load_recent_logs, load_workspaces, save_operation_log,
        save_workspace,
    },
    state::{
        ActivityRetryAction, AddProjectDialog, AppState, ConfirmRemoveDialog, LeaderKeyState,
        LoadPhase, OperationLeaseId, OperationOwner, PendingTagPush, RetryAvailability,
        RetryExclusion, RetryUnavailableReason, Screen,
        changelog::ChangelogPhase,
        conflict_ops::ConflictPhase,
        context::ContextPhase,
        focus,
        freezer::FreezerPhase,
        sync::{ProjectOutcome, SmartPullRetryPreparation, SyncKind, SyncPhase, SyncResult},
        topology::TopologyPhase,
        workspace_mgr::{
            CreateWorkspaceDialog, DeleteWorkspaceDialog, RenameWorkspaceDialog,
            next_active_index_after_delete, validate_workspace_name,
        },
    },
    view::app_view,
};

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn init() -> (AppState, Task<Message>) {
    let paths = AppPaths::resolve();
    let (config, config_err) = load_config(&paths);
    let mut state = AppState::new_with_paths(config.clone(), paths.clone());

    if let Some(err) = config_err {
        state.status_bar = Some(err);
    }

    let (workspaces, ws_errors) = load_workspaces(&paths);
    for e in &ws_errors {
        tracing::warn!("workspace load error: {e}");
    }

    let mut ws_list = workspaces;
    if ws_list.is_empty() {
        ws_list.push(Workspace::new("My Workspace"));
    }
    state.all_workspaces = ws_list;
    state.active_workspace_idx = 0;
    state.workspace = state.all_workspaces.first().cloned();
    state.load_phase = LoadPhase::Refreshing;
    state.is_refreshing = true;
    state.operation_logs = load_recent_logs(&paths, config.max_log_entries);

    let task = refresh_workspace_task(&state);
    (state, task)
}

// ---------------------------------------------------------------------------
// Subscription
// ---------------------------------------------------------------------------

pub fn subscription(state: &AppState) -> Subscription<Message> {
    let tick_sub = if state.config.refresh_interval_secs > 0 {
        time::every(Duration::from_secs(u64::from(
            state.config.refresh_interval_secs,
        )))
        .map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    let keyboard_sub = keyboard::listen().map(|event| {
        use keyboard::Event;
        use keyboard::key::Named;
        if let Event::KeyPressed { key, modifiers, .. } = event {
            let ctrl = modifiers.control() || modifiers.command();
            let shortcut = match &key {
                keyboard::Key::Named(Named::Escape) => Some(ShortcutMessage::Close),
                // RFC-036 R1: Tab/Shift-Tab traversal.
                keyboard::Key::Named(Named::Tab) if modifiers.shift() => {
                    Some(ShortcutMessage::FocusPrevious)
                }
                keyboard::Key::Named(Named::Tab) => Some(ShortcutMessage::FocusNext),
                // RFC-036 R3/R3a: Enter/Space activate the focused control,
                // gated in `handle_shortcut` so a focused text input still
                // receives the keystroke instead of activating a control.
                keyboard::Key::Named(Named::Enter) => Some(ShortcutMessage::ActivateFocused),
                keyboard::Key::Named(Named::Space) => Some(ShortcutMessage::ActivateFocused),
                keyboard::Key::Character(c) => match c.as_str() {
                    "r" | "R" if ctrl => Some(ShortcutMessage::Refresh),
                    "k" | "K" if ctrl => Some(ShortcutMessage::OpenContextOps),
                    "t" | "T" if ctrl => Some(ShortcutMessage::OpenFreezer),
                    "/" if ctrl => Some(ShortcutMessage::FocusSearch),
                    // RFC-036 R4: bare `/` (no modifier) — gated in
                    // `handle_shortcut`, not here, since only `AppState`
                    // knows whether a text input currently holds focus.
                    "/" => Some(ShortcutMessage::FocusSearchBare),
                    _ => None,
                },
                _ => None,
            };
            if let Some(s) = shortcut {
                return Message::Shortcut(s);
            }
        }
        Message::Tick
    });

    let fs_sub = fs_watch_subscription(state);
    Subscription::batch([tick_sub, keyboard_sub, fs_sub])
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Navigate(screen) => {
            state.screen = screen;
            Task::none()
        }
        Message::Tick => handle_tick(state),
        Message::Shortcut(msg) => handle_shortcut(state, msg),
        Message::Workspace(msg) => handle_workspace(state, msg),
        Message::Project(msg) => handle_project(state, msg),
        Message::Sync(msg) => handle_sync(state, msg),
        Message::Freezer(msg) => handle_freezer(state, msg),
        Message::History(msg) => handle_history(state, msg),
        Message::Settings(msg) => handle_settings(state, msg),
        Message::Background(msg) => handle_background(state, msg),
        Message::Filter(msg) => {
            state.apply_filter(msg);
            state.reconcile_selection_with_display();
            Task::none()
        }
        Message::ConflictOps(msg) => handle_conflict_ops(state, msg),
        Message::Changelog(msg) => handle_changelog(state, msg),
        Message::Topology(msg) => handle_topology(state, msg),
        Message::TagPush(msg) => handle_tag_push(state, msg),
        Message::FsWatchTick => handle_fs_watch_tick(state),

        // ---------------------------------------------------------------
        // RFC-0009 — Selection
        // ---------------------------------------------------------------
        Message::Selection(sel) => handle_selection(state, sel),

        // ---------------------------------------------------------------
        // RFC-0011 — Activity strip
        // ---------------------------------------------------------------
        Message::Activity(act) => handle_activity(state, act),

        // ---------------------------------------------------------------
        // RFC-0012 — Command palette
        // ---------------------------------------------------------------
        Message::Palette(pal) => handle_palette(state, pal),

        // ---------------------------------------------------------------
        // RFC-032 — Dashboard display controls
        // ---------------------------------------------------------------
        Message::Dashboard(msg) => handle_dashboard(state, msg),

        // ---------------------------------------------------------------
        // RFC-0016 — Keyboard events
        // ---------------------------------------------------------------
        Message::KeyEvent(ke) => handle_key_event(state, ke),

        // ---------------------------------------------------------------
        // RFC-0014 — Detail panel
        // ---------------------------------------------------------------
        Message::DetailPanel(dp) => {
            use crate::message::DetailPanelMessage;
            match dp {
                DetailPanelMessage::Opened(id) => state.detail_panel.open_project_id = Some(id),
                DetailPanelMessage::Closed => state.detail_panel.open_project_id = None,
            }
            Task::none()
        }

        Message::CopyToClipboard(text) => clipboard::write(text),
        Message::ToggleOpDetails => {
            state.show_op_details = !state.show_op_details;
            Task::none()
        }
        Message::Context(msg) => handle_context(state, msg),
        Message::Launch(msg) => handle_launch(state, msg),
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    app_view(state)
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

fn handle_tick(state: &mut AppState) -> Task<Message> {
    if !state.is_refreshing {
        state.is_refreshing = true;
        state.load_phase = LoadPhase::Refreshing;
        refresh_workspace_task(state)
    } else {
        Task::none()
    }
}

// ---------------------------------------------------------------------------
// Shortcut
// ---------------------------------------------------------------------------

fn handle_shortcut(state: &mut AppState, msg: ShortcutMessage) -> Task<Message> {
    match msg {
        ShortcutMessage::Refresh => handle_tick(state),
        ShortcutMessage::OpenContextOps => {
            handle_context(state, ContextMessage::OpenRequested(None))
        }
        ShortcutMessage::OpenFreezer => handle_freezer(state, FreezerMessage::OpenRequested),
        ShortcutMessage::FocusSearch => focus_search(state),
        ShortcutMessage::FocusSearchBare => {
            // R4: a bare `/` must not shadow a literal `/` being typed into
            // a field that already holds knotra-focus. `Ctrl+/` is
            // unconditional (unchanged from before RFC-036) because a
            // modified `/` was never a literal-typing conflict in the first
            // place.
            if current_target_is_text_input(state) {
                return Task::none();
            }
            focus_search(state)
        }
        ShortcutMessage::Close => {
            // The workspace switcher (RFC-034 R12) is an `AppLayout::header_menu`,
            // not a `dialog`/`sheet`, so it is deliberately outside
            // `close_topmost_layer`'s branch ordering (that function's contract
            // is unchanged). It is still the topmost visible layer when open,
            // so Escape must close it — checked here, one level above, rather
            // than by adding a branch to that function.
            if state.workspace_mgr.switcher_open {
                state.workspace_mgr.switcher_open = false;
                return Task::none();
            }
            // R7: capture whether an in-scope overlay (the three
            // workspace-manager dialogs) was open before Escape/scrim closes
            // it, so focus return only fires on an actual open->closed
            // transition — not when a non-cancellable overlay (out of this
            // stage's order-building scope) absorbs the close and stays up.
            let was_open = workspace_dialog_open(state);
            let task = close_topmost_layer(state);
            if was_open && !workspace_dialog_open(state) {
                Task::batch([task, close_overlay_focus(state)])
            } else {
                task
            }
        }
        ShortcutMessage::FocusNext => advance_focus(state, focus::Direction::Next),
        ShortcutMessage::FocusPrevious => advance_focus(state, focus::Direction::Previous),
        ShortcutMessage::ActivateFocused => activate_focused(state),
    }
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            if !state.is_refreshing {
                state.dashboard_error_details_open = false;
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                state.status_bar = Some(state.t("status.refreshing").to_owned());
                refresh_workspace_task(state)
            } else {
                Task::none()
            }
        }

        WorkspaceMessage::AddProjectDialogOpened => {
            state.add_project_dialog = Some(AddProjectDialog::default());
            open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone(),
                ),
            )
        }
        WorkspaceMessage::AddProjectNameChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectPathChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.path = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectNextStep => {
            let err_msg = state.t("plain.add_project.error_no_folder").to_owned();
            if let Some(d) = &mut state.add_project_dialog {
                if d.path.trim().is_empty() {
                    d.error = Some(err_msg);
                } else {
                    d.error = None;
                    d.step = crate::state::AddProjectStep::NameProject;
                }
            }
            open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
                ),
            )
        }
        WorkspaceMessage::AddProjectConfirmed => {
            let dialog = match state.add_project_dialog.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            let name = dialog.name.trim().to_owned();
            let path = dialog.path.trim().to_owned();
            if name.is_empty() || path.is_empty() {
                state.add_project_dialog = Some(AddProjectDialog {
                    name: dialog.name,
                    path: dialog.path,
                    error: Some(state.t("dialog.add_project.error_empty").to_owned()),
                    ..Default::default()
                });
                return Task::none();
            }
            // Clear any pending undo when a new project is added.
            state.recent_removal = None;
            let project = Project::new(name, path);
            let paths = state.paths.clone();
            if let Some(ws) = &mut state.workspace {
                ws.add_project(project);
                persist_workspace(&paths, ws);
            }
            state.reconcile_selection_with_display();
            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }
        WorkspaceMessage::AddProjectCancelled => {
            state.add_project_dialog = None;
            Task::none()
        }
        WorkspaceMessage::BrowsePathRequested => Task::future(async {
            let folder = rfd::AsyncFileDialog::new()
                .set_title("Select project folder")
                .pick_folder()
                .await;
            let path = folder.map(|f| f.path().to_string_lossy().into_owned());
            Message::Workspace(crate::message::WorkspaceMessage::BrowsePathSelected(path))
        }),
        WorkspaceMessage::BrowsePathSelected(path_opt) => {
            if let Some(path) = path_opt
                && let Some(d) = &mut state.add_project_dialog
            {
                // Auto-fill name from folder name if not already set.
                if d.name.is_empty()
                    && let Some(name) = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                {
                    d.name = name.to_owned();
                }
                d.path = path;
                d.error = None;
                // Auto-advance to step 2 once a folder is chosen.
                d.step = crate::state::AddProjectStep::NameProject;
            }
            open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
                ),
            )
        }
        WorkspaceMessage::RemoveProjectRequested(id) => {
            let name = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            state.confirm_remove_dialog = Some(ConfirmRemoveDialog {
                project_id: id,
                project_name: name,
            });
            Task::none()
        }
        WorkspaceMessage::RemoveProjectConfirmed(id) => {
            state.confirm_remove_dialog = None;
            // Capture snapshots before removing so undo can restore exactly.
            let removed_project = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id).cloned());
            let removed_status = state
                .workspace_status
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == id).cloned());

            let paths = state.paths.clone();
            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
                persist_workspace(&paths, ws);
            }
            if let Some(ws_status) = &mut state.workspace_status {
                ws_status.projects.retain(|s| s.project_id != id);
            }
            state.fetching_projects.remove(&id);
            state.reconcile_selection_with_display();
            if state.selection.selected_ids.is_empty() {
                state.selection_mode = false;
            }

            // Store undo opportunity. Cleared by next user action or explicit dismiss.
            if let Some(project) = removed_project {
                state.recent_removal = Some(crate::state::UndoableRemoval {
                    project,
                    status_snapshot: removed_status,
                });
            }
            Task::none()
        }
        WorkspaceMessage::RemoveProjectCancelled => {
            state.confirm_remove_dialog = None;
            Task::none()
        }
        WorkspaceMessage::UndoRemoval => {
            if let Some(removal) = state.recent_removal.take() {
                let paths = state.paths.clone();
                if let Some(ws) = &mut state.workspace {
                    ws.projects.push(removal.project);
                    persist_workspace(&paths, ws);
                }
                if let Some(ws_status) = &mut state.workspace_status
                    && let Some(snap) = removal.status_snapshot
                {
                    ws_status.projects.push(snap);
                }
                state.reconcile_selection_with_display();
            }
            Task::none()
        }
        WorkspaceMessage::DismissUndoSnackbar => {
            state.recent_removal = None;
            Task::none()
        }

        // --- Multi-workspace management ---
        WorkspaceMessage::CreateWorkspaceDialogOpened => {
            state.workspace_mgr.switcher_open = false;
            state.workspace_mgr.create_dialog = Some(CreateWorkspaceDialog::default());
            enter_overlay_focus(state)
        }
        WorkspaceMessage::CreateWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.create_dialog {
                d.name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::CreateWorkspaceConfirmed => {
            let raw_name = state
                .workspace_mgr
                .create_dialog
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();

            let name = match validate_workspace_name(&raw_name, &state.all_workspaces, None) {
                Ok(name) => name,
                Err(err) => {
                    let msg = state.t(err.i18n_key()).to_owned();
                    if let Some(d) = &mut state.workspace_mgr.create_dialog {
                        d.error = Some(msg);
                    }
                    return Task::none();
                }
            };

            let ws = knotra_vcs::Workspace::new(name);
            if let Err(e) = save_workspace(&ws, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.save_failed"));
                if let Some(d) = &mut state.workspace_mgr.create_dialog {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.all_workspaces.push(ws);
            state.active_workspace_idx = state.all_workspaces.len().saturating_sub(1);
            state.workspace = state.all_workspaces.last().cloned();
            state.clear_selection_mode();
            state.workspace_status = None;
            state.dashboard_error_details_open = false;
            state.load_phase = LoadPhase::Refreshing;
            state.is_refreshing = true;
            state.workspace_mgr.create_dialog = None;
            Task::batch([refresh_workspace_task(state), close_overlay_focus(state)])
        }
        WorkspaceMessage::CreateWorkspaceCancelled => {
            state.workspace_mgr.create_dialog = None;
            close_overlay_focus(state)
        }

        WorkspaceMessage::RenameWorkspaceDialogOpened => {
            state.workspace_mgr.switcher_open = false;
            let current = state
                .workspace
                .as_ref()
                .map(|ws| ws.name.clone())
                .unwrap_or_default();
            state.workspace_mgr.rename_dialog = Some(RenameWorkspaceDialog {
                new_name: current,
                error: None,
            });
            enter_overlay_focus(state)
        }
        WorkspaceMessage::RenameWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                d.new_name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::RenameWorkspaceConfirmed => {
            let raw_name = state
                .workspace_mgr
                .rename_dialog
                .as_ref()
                .map(|d| d.new_name.clone())
                .unwrap_or_default();

            let current_id = state.workspace.as_ref().map(|ws| ws.id.clone());
            let name = match validate_workspace_name(
                &raw_name,
                &state.all_workspaces,
                current_id.as_ref(),
            ) {
                Ok(name) => name,
                Err(err) => {
                    let msg = state.t(err.i18n_key()).to_owned();
                    if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                        d.error = Some(msg);
                    }
                    return Task::none();
                }
            };

            let mut renamed = match state.workspace.clone() {
                Some(ws) => ws,
                None => return Task::none(),
            };
            renamed.name = name;
            if let Err(e) = save_workspace(&renamed, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.save_failed"));
                if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.workspace = Some(renamed.clone());
            if let Some(entry) = state.all_workspaces.get_mut(state.active_workspace_idx) {
                *entry = renamed;
            }
            state.workspace_mgr.rename_dialog = None;
            close_overlay_focus(state)
        }
        WorkspaceMessage::RenameWorkspaceCancelled => {
            state.workspace_mgr.rename_dialog = None;
            close_overlay_focus(state)
        }

        WorkspaceMessage::DeleteWorkspaceRequested => {
            state.workspace_mgr.switcher_open = false;
            if state.all_workspaces.len() <= 1 {
                if let Some(ws) = state.workspace.as_ref() {
                    state.workspace_mgr.confirm_delete = Some(DeleteWorkspaceDialog {
                        workspace_id: ws.id.clone(),
                        workspace_name: ws.name.clone(),
                        project_count: ws.projects.len(),
                        error: Some(state.t("workspace.delete.disabled_last").to_owned()),
                    });
                }
                return enter_overlay_focus(state);
            }

            if let Some(ws) = state.workspace.as_ref() {
                state.workspace_mgr.confirm_delete = Some(DeleteWorkspaceDialog {
                    workspace_id: ws.id.clone(),
                    workspace_name: ws.name.clone(),
                    project_count: ws.projects.len(),
                    error: None,
                });
            }
            enter_overlay_focus(state)
        }
        WorkspaceMessage::DeleteWorkspaceConfirmed => {
            if state.all_workspaces.len() <= 1 {
                let msg = state.t("workspace.delete.disabled_last").to_owned();
                if let Some(d) = &mut state.workspace_mgr.confirm_delete {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            let delete_id = state
                .workspace_mgr
                .confirm_delete
                .as_ref()
                .map(|d| d.workspace_id.clone());
            let deleted_idx = delete_id
                .as_ref()
                .and_then(|id| state.all_workspaces.iter().position(|ws| &ws.id == id))
                .unwrap_or(state.active_workspace_idx);
            let Some(deleted_ws) = state.all_workspaces.get(deleted_idx).cloned() else {
                return Task::none();
            };

            if let Err(e) = delete_workspace_file(&deleted_ws, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.delete_failed"));
                if let Some(d) = &mut state.workspace_mgr.confirm_delete {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.all_workspaces.remove(deleted_idx);
            state.active_workspace_idx =
                next_active_index_after_delete(deleted_idx, state.all_workspaces.len() + 1);
            state.workspace = state
                .all_workspaces
                .get(state.active_workspace_idx)
                .cloned();
            state.clear_selection_mode();
            state.workspace_status = None;
            state.dashboard_error_details_open = false;
            let active_ids: Vec<knotra_vcs::ProjectId> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
                .unwrap_or_default();
            state.fs_poller.prune(&active_ids);
            state.load_phase = LoadPhase::Refreshing;
            state.is_refreshing = true;
            state.workspace_mgr.confirm_delete = None;
            Task::batch([refresh_workspace_task(state), close_overlay_focus(state)])
        }
        WorkspaceMessage::DeleteWorkspaceCancelled => {
            state.workspace_mgr.confirm_delete = None;
            close_overlay_focus(state)
        }

        WorkspaceMessage::SwitcherToggled => {
            state.workspace_mgr.switcher_open = !state.workspace_mgr.switcher_open;
            Task::none()
        }
        WorkspaceMessage::WorkspaceSwitched(id) => {
            state.workspace_mgr.switcher_open = false;
            if let Some(idx) = state.all_workspaces.iter().position(|ws| ws.id == id) {
                clear_sync_retry_context(state);
                state.active_workspace_idx = idx;
                state.workspace = state.all_workspaces.get(idx).cloned();
                state.clear_selection_mode();
                // Prune stale FsPoller snapshots from the previous workspace.
                let active_ids: Vec<knotra_vcs::ProjectId> = state
                    .workspace
                    .as_ref()
                    .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
                    .unwrap_or_default();
                state.fs_poller.prune(&active_ids);
                state.workspace_status = None;
                state.dashboard_error_details_open = false;
                state.load_phase = LoadPhase::Refreshing;
                state.is_refreshing = true;
                return refresh_workspace_task(state);
            }
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

fn handle_project(state: &mut AppState, msg: ProjectMessage) -> Task<Message> {
    match msg {
        ProjectMessage::StatusRefreshRequested(id) => {
            let project = find_project(state, &id);
            if let Some(p) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&p).await },
                    |s| {
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                            knotra_vcs::WorkspaceStatus {
                                projects: vec![s],
                                last_refresh: Some(chrono::Utc::now()),
                            },
                        ))
                    },
                )
            } else {
                Task::none()
            }
        }
        ProjectMessage::FetchRequested(id) => {
            let project = find_project(state, &id);
            if let Some(p) = project {
                let Some(lease_id) = acquire_operation(state, OperationOwner::SingleFetch) else {
                    return Task::none();
                };
                state.fetching_projects.insert(id.clone());
                Task::perform(
                    async move {
                        let started = chrono::Utc::now();
                        let op_id = OperationId::new();
                        let result = VcsAdapter::fetch(&p).await;
                        OperationLog {
                            result: OperationResult {
                                operation_id: op_id,
                                kind: OperationKind::Fetch,
                                started_at: started,
                                finished_at: chrono::Utc::now(),
                                per_project: vec![result],
                                rollback_attempted: false,
                                rollback_succeeded: None,
                            },
                            recovery_hints: vec![],
                        }
                    },
                    move |log| {
                        Message::Background(BackgroundMessage::SingleFetchCompleted {
                            lease_id,
                            log,
                        })
                    },
                )
            } else {
                state.fetching_projects.remove(&id);
                Task::none()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

fn handle_sync(state: &mut AppState, msg: SyncMessage) -> Task<Message> {
    match msg {
        SyncMessage::OpenRequested => {
            clear_sync_retry_context(state);
            if let Some(ws) = &state.workspace {
                state.sync.init_selection(&ws.projects);
            }
            state.active_modal = crate::state::ActiveModal::Pull;
            Task::none()
        }

        SyncMessage::ProjectToggled(id, included) => {
            state.sync.project_selection.insert(id, included);
            Task::none()
        }

        SyncMessage::DispositionChanged(id, disposition) => {
            state
                .sync
                .disposition_overrides
                .insert(id.clone(), disposition.clone());
            if let SyncPhase::AwaitingConfirm(plan) = &mut state.sync.phase
                && let Some(entry) = plan.entries.iter_mut().find(|entry| entry.project_id == id)
            {
                entry.disposition = disposition;
                entry.skip_reason = None;
            }
            Task::none()
        }

        SyncMessage::PlanRequested => {
            if state.sync.retry_preparation.is_none() {
                state.sync.retry_exclusions.clear();
            }
            // Open the pull modal and start planning.
            state.active_modal = crate::state::ActiveModal::Pull;
            state.sync.phase = SyncPhase::Planning;
            Task::done(Message::Sync(SyncMessage::SmartPullPlanRequested))
        }
        SyncMessage::ExecuteRequested => {
            if let SyncPhase::AwaitingConfirm(plan) = &state.sync.phase {
                Task::done(Message::Sync(SyncMessage::SmartPullConfirmed(plan.clone())))
            } else {
                Task::none()
            }
        }
        SyncMessage::BulkFetchRequested => {
            let (ids, fetchable_ids): (Vec<_>, Vec<_>) = if state.selection_mode {
                let summary = state.selection_summary();
                state.sync.selected_project_ids = summary.selected_ids.iter().cloned().collect();
                if let Some(ws) = &state.workspace {
                    state
                        .sync
                        .set_selection(ws.projects.as_slice(), &state.selection.selected_ids);
                }
                (summary.selected_ids, summary.fetchable_ids)
            } else {
                let ids = state.sync.selected_ids();
                let fetchable_ids = ids
                    .iter()
                    .filter(|id| !state.missing_projects.contains(*id))
                    .cloned()
                    .collect();
                (ids, fetchable_ids)
            };
            start_bulk_fetch(state, ids, fetchable_ids)
        }

        SyncMessage::BulkFetchAllRequested => {
            let Some(ws) = &state.workspace else {
                return Task::none();
            };
            let ids: Vec<_> = ws
                .projects
                .iter()
                .map(|project| project.id.clone())
                .collect();
            let fetchable_ids: Vec<_> = ids
                .iter()
                .filter(|id| !state.missing_projects.contains(*id))
                .cloned()
                .collect();
            state.sync.selected_project_ids = fetchable_ids.iter().cloned().collect();
            state.sync.project_selection.clear();
            for project in &ws.projects {
                state
                    .sync
                    .project_selection
                    .insert(project.id.clone(), fetchable_ids.contains(&project.id));
            }
            start_bulk_fetch(state, ids, fetchable_ids)
        }

        SyncMessage::SmartPullPlanRequested => {
            let Some(lease_id) = acquire_operation(state, OperationOwner::SmartPullPreparation)
            else {
                state.sync.phase = SyncPhase::Idle;
                return Task::none();
            };
            state.sync.phase = SyncPhase::Planning;
            // Build the plan synchronously from existing status.
            let selected_projects: Vec<Project> = state
                .workspace
                .as_ref()
                .map(|w| {
                    if state.sync.selected_project_ids.is_empty() {
                        w.projects.clone()
                    } else {
                        w.projects
                            .iter()
                            .filter(|p| state.sync.selected_project_ids.contains(&p.id))
                            .cloned()
                            .collect()
                    }
                })
                .unwrap_or_default();
            let plan = state
                .sync
                .build_plan(&selected_projects, state.workspace_status.as_ref());
            state.sync.phase = SyncPhase::AwaitingConfirm(plan.clone());
            state.operation_interlock.release_if_matches(lease_id);
            Task::done(Message::Background(BackgroundMessage::SmartPullPlanReady(
                plan,
            )))
        }

        SyncMessage::SmartPullConfirmed(plan) => {
            let Some(lease_id) = acquire_operation(state, OperationOwner::SmartPullExecution)
            else {
                return Task::none();
            };
            let projects_map: std::collections::HashMap<_, _> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .map(|p| (p.id.clone(), p.clone()))
                        .collect()
                })
                .unwrap_or_default();

            let entries = plan.entries.clone();
            state.sync.phase = SyncPhase::PullRunning {
                plan,
                lease_id,
                started_at: chrono::Utc::now(),
                completed: Vec::new(),
            };

            use iced::futures::stream;

            let pull_stream = stream::iter(entries).then(move |entry| {
                let project = projects_map.get(&entry.project_id).cloned();
                async move {
                    let Some(project) = project else {
                        return SmartPullProgress {
                            project_id: entry.project_id.clone(),
                            project_name: entry.project_name.clone(),
                            result: knotra_vcs::model::operation::ProjectOperationResult {
                                project_id: entry.project_id,
                                outcome: ProjectOperationOutcome::Failed,
                                success: false,
                                skip_reason: None,
                                commands_executed: vec![],
                                stdout: String::new(),
                                stderr: String::new(),
                                exit_code: None,
                                error_message: Some("project not found".to_owned()),
                            },
                            recovery_hint: None,
                        };
                    };

                    match entry.disposition {
                        SmartPullDisposition::Excluded => SmartPullProgress {
                            project_id: project.id.clone(),
                            project_name: entry.project_name.clone(),
                            result: knotra_vcs::model::operation::ProjectOperationResult {
                                project_id: project.id.clone(),
                                outcome: ProjectOperationOutcome::Skipped,
                                success: true,
                                skip_reason: entry
                                    .skip_reason
                                    .as_ref()
                                    .map(smart_pull_skip_reason_text)
                                    .map(str::to_owned),
                                commands_executed: vec![],
                                stdout: "[excluded]".to_owned(),
                                stderr: String::new(),
                                exit_code: Some(0),
                                error_message: None,
                            },
                            recovery_hint: None,
                        },
                        SmartPullDisposition::FetchOnly => {
                            let r = VcsAdapter::fetch(&project).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: None,
                            }
                        }
                        SmartPullDisposition::Pull | SmartPullDisposition::StashAndPull => {
                            let stash =
                                matches!(entry.disposition, SmartPullDisposition::StashAndPull);
                            let (r, hint) = VcsAdapter::smart_pull(&project, stash).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: hint,
                            }
                        }
                    }
                }
            });

            Task::run(pull_stream, move |progress| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted {
                    lease_id,
                    progress,
                })
            })
        }

        SyncMessage::SmartPullCancelled => {
            clear_sync_retry_context(state);
            state.sync.phase = SyncPhase::Idle;
            Task::none()
        }

        SyncMessage::ModalClosed => {
            if !smart_pull_is_running(state) {
                clear_sync_retry_context(state);
                state.active_modal = crate::state::ActiveModal::None;
            }
            Task::none()
        }
        SyncMessage::Cancelled => {
            if !smart_pull_is_running(state) {
                clear_sync_retry_context(state);
                state.active_modal = crate::state::ActiveModal::None;
            }
            Task::none()
        }
        SyncMessage::BulkPullRequested => {
            clear_sync_retry_context(state);
            state.active_modal = crate::state::ActiveModal::Pull;
            state.sync.phase = SyncPhase::Planning;
            state.sync.selected_project_ids = state.selection.selected_ids.clone();
            if let Some(ws) = &state.workspace {
                state
                    .sync
                    .set_selection(&ws.projects, &state.selection.selected_ids);
            }
            Task::done(Message::Sync(SyncMessage::SmartPullPlanRequested))
        }
    }
}

fn start_bulk_fetch(
    state: &mut AppState,
    ids: Vec<knotra_vcs::ProjectId>,
    fetchable_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    if ids.is_empty() {
        return Task::none();
    }

    let project_map: std::collections::HashMap<_, _> = state
        .workspace
        .as_ref()
        .map(|ws| {
            ws.projects
                .iter()
                .map(|p| (p.id.clone(), p.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut skipped = Vec::new();
    let mut skipped_results = Vec::new();
    let projects: Vec<_> = fetchable_ids
        .iter()
        .filter_map(|id| project_map.get(id).cloned())
        .collect();
    for id in ids {
        if !project_map.contains_key(&id) || state.missing_projects.contains(&id) {
            let project_name = project_map
                .get(&id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| state.t("plain.project").to_owned());
            let result = ProjectOperationResult {
                project_id: id.clone(),
                outcome: ProjectOperationOutcome::Skipped,
                success: true,
                skip_reason: Some(state.t("plain.fetch.skipped_unavailable").to_owned()),
                commands_executed: Vec::new(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: None,
            };
            skipped_results.push(result.clone());
            skipped.push(ProjectOutcome {
                project_id: id,
                project_name,
                outcome: result.effective_outcome(),
                success: result.success,
                skip_reason: result.skip_reason,
                commands_executed: result.commands_executed,
                stdout: result.stdout,
                stderr: result.stderr,
                log_expanded: false,
            });
        }
    }
    let total = projects.len() + skipped.len();
    if total == 0 {
        return Task::none();
    }
    let done = skipped.len();
    if projects.is_empty() {
        state.sync.phase = SyncPhase::Done(SyncResult {
            kind: SyncKind::Fetch,
            per_project: skipped,
            recovery_hints: Vec::new(),
        });
        return Task::none();
    }
    let Some(lease_id) = acquire_operation(state, OperationOwner::BulkFetch) else {
        return Task::none();
    };
    let operation_id = OperationId::new();
    state.sync.phase = SyncPhase::FetchRunning {
        operation_id,
        lease_id,
        started_at: chrono::Utc::now(),
        total,
        done,
        completed: skipped,
        operation_results: skipped_results,
    };

    use iced::futures::stream;

    let project_stream = stream::iter(projects)
        .then(move |project| async move { VcsAdapter::fetch(&project).await });

    Task::run(project_stream, move |per_project_result| {
        Message::Background(BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            progress: SmartPullProgress {
                project_id: per_project_result.project_id.clone(),
                project_name: String::new(),
                result: per_project_result,
                recovery_hint: None,
            },
        })
    })
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

fn handle_background(state: &mut AppState, msg: BackgroundMessage) -> Task<Message> {
    match msg {
        BackgroundMessage::WorkspaceStatusRefreshed(new_status) => {
            state.dashboard_error_details_open = false;
            // Detect missing-path projects.
            if let Some(ws) = &state.workspace {
                let missing: Vec<_> = ws
                    .projects
                    .iter()
                    .filter(|p| !knotra_vcs::VcsAdapter::repo_exists(p))
                    .map(|p| p.id.clone())
                    .collect();
                if missing != state.missing_projects.iter().cloned().collect::<Vec<_>>() {
                    state.missing_projects = missing.into_iter().collect();
                }
            }
            merge_workspace_status(state, new_status);
            state.load_phase = LoadPhase::Ready;
            state.is_refreshing = false;
            state.status_bar = None;
            Task::none()
        }

        BackgroundMessage::ActivityFetchRetryProjectCompleted {
            lease_id,
            operation_id,
            result,
        } => {
            let Some(mut run) = state.activity.fetch_retry.take() else {
                return Task::none();
            };
            if run.lease_id != lease_id || run.operation_id != operation_id {
                state.activity.fetch_retry = Some(run);
                return Task::none();
            }
            run.completed.push(result);
            let done = run.completed.len() + run.exclusions.len();
            if let crate::state::LatestOpState::Running {
                operation_id: active_id,
                done: active_done,
                ..
            } = &mut state.activity.latest
                && *active_id == operation_id
            {
                *active_done = done;
            }
            let expected = run.total.saturating_sub(run.exclusions.len());
            if run.completed.len() < expected {
                state.activity.fetch_retry = Some(run);
                return Task::none();
            }

            let mut per_project = run.completed;
            per_project.extend(run.exclusions.iter().map(skipped_retry_result));
            let log = OperationLog {
                result: OperationResult {
                    operation_id: run.operation_id,
                    kind: OperationKind::Fetch,
                    started_at: run.started_at,
                    finished_at: chrono::Utc::now(),
                    per_project,
                    rollback_attempted: false,
                    rollback_succeeded: None,
                },
                recovery_hints: Vec::new(),
            };
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            persist_log(&log, state);
            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }

        BackgroundMessage::SmartPullRetryStatusReady {
            request_id,
            workspace_id,
            lease_id,
            statuses,
        } => {
            let Some(preparation) = state.sync.retry_preparation.clone() else {
                return Task::none();
            };
            let current_workspace_matches = state
                .workspace
                .as_ref()
                .is_some_and(|workspace| workspace.id == workspace_id);
            if preparation.id != request_id
                || preparation.workspace_id != workspace_id
                || preparation.lease_id != lease_id
            {
                return Task::none();
            }
            let source_matches = matches!(
                &state.activity.latest,
                crate::state::LatestOpState::Completed { log, .. }
                    if log.result.operation_id == preparation.source_operation_id
            );
            let expected_ids: std::collections::HashSet<_> =
                preparation.eligible_ids.iter().cloned().collect();
            let returned_ids: std::collections::HashSet<_> = statuses
                .iter()
                .map(|status| status.project_id.clone())
                .collect();
            let status_ids_match = statuses.len() == expected_ids.len()
                && returned_ids.len() == statuses.len()
                && returned_ids == expected_ids;
            if !source_matches
                || !status_ids_match
                || !current_workspace_matches
                || state.active_modal != crate::state::ActiveModal::Pull
                || !matches!(state.sync.phase, SyncPhase::RetryPreparing)
            {
                state.sync.retry_preparation = None;
                state.operation_interlock.release_if_matches(lease_id);
                if state.active_modal == crate::state::ActiveModal::Pull
                    && current_workspace_matches
                {
                    state.sync.phase = SyncPhase::RetryPreparationFailed;
                }
                return Task::none();
            }

            let mut exclusions = preparation.exclusions;
            let mut readable = Vec::new();
            for status in statuses {
                if status.read_error.is_some() {
                    exclusions.push(RetryExclusion {
                        project_id: status.project_id.clone(),
                        reason: RetryExclusionReason::StatusUnavailable,
                    });
                } else {
                    readable.push(status);
                }
            }
            state.sync.retry_preparation = None;
            state.operation_interlock.release_if_matches(lease_id);
            state.sync.retry_exclusions = exclusions;

            if readable.is_empty() {
                state.sync.phase = SyncPhase::RetryPreparationFailed;
                return Task::none();
            }

            let readable_ids: std::collections::HashSet<_> = readable
                .iter()
                .map(|status| status.project_id.clone())
                .collect();
            merge_workspace_status(
                state,
                knotra_vcs::WorkspaceStatus {
                    projects: readable,
                    last_refresh: Some(chrono::Utc::now()),
                },
            );
            state.sync.selected_project_ids = readable_ids.clone();
            if let Some(workspace) = &state.workspace {
                for project in &workspace.projects {
                    state
                        .sync
                        .project_selection
                        .insert(project.id.clone(), readable_ids.contains(&project.id));
                }
            }
            let selected_projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|workspace| {
                    workspace
                        .projects
                        .iter()
                        .filter(|project| readable_ids.contains(&project.id))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            let plan = state
                .sync
                .build_plan(&selected_projects, state.workspace_status.as_ref());
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);
            Task::none()
        }

        BackgroundMessage::SmartPullPlanReady(plan) => {
            // Already set in handle_sync; this message lets the view re-render.
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);
            Task::none()
        }

        BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            mut progress,
        } => {
            // Fill in the project name if missing.
            if progress.project_name.is_empty()
                && let Some(name) = find_project_name(state, &progress.project_id)
            {
                progress.project_name = name;
            }
            let retry_exclusions = state.sync.retry_exclusions.clone();
            let retry_outcomes: Vec<ProjectOutcome> = retry_exclusions
                .iter()
                .map(|exclusion| ProjectOutcome {
                    project_id: exclusion.project_id.clone(),
                    project_name: find_project_name(state, &exclusion.project_id)
                        .unwrap_or_else(|| state.t("plain.project").to_owned()),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some(exclusion.reason.code().to_owned()),
                    commands_executed: Vec::new(),
                    stdout: String::new(),
                    stderr: String::new(),
                    log_expanded: false,
                })
                .collect();

            let mut completed_log: Option<OperationLog> = None;
            let mut completed_lease: Option<OperationLeaseId> = None;

            match &mut state.sync.phase {
                SyncPhase::FetchRunning {
                    operation_id,
                    lease_id: phase_lease_id,
                    started_at,
                    done,
                    total,
                    completed,
                    operation_results,
                } => {
                    if *phase_lease_id != lease_id {
                        return Task::none();
                    }
                    *done += 1;
                    let done_val = *done;
                    let total_val = *total;

                    let outcome = ProjectOutcome {
                        project_id: progress.project_id.clone(),
                        project_name: progress.project_name.clone(),
                        outcome: progress.result.effective_outcome(),
                        success: progress.result.success,
                        skip_reason: progress.result.skip_reason.clone(),
                        commands_executed: progress.result.commands_executed.clone(),
                        stdout: progress.result.stdout.clone(),
                        stderr: progress.result.stderr.clone(),
                        log_expanded: false,
                    };
                    operation_results.push(progress.result.clone());
                    completed.push(outcome);

                    if done_val >= total_val {
                        let per_project = completed.clone();
                        completed_log = Some(OperationLog {
                            result: OperationResult {
                                operation_id: operation_id.clone(),
                                kind: OperationKind::Fetch,
                                started_at: *started_at,
                                finished_at: chrono::Utc::now(),
                                per_project: operation_results.clone(),
                                rollback_attempted: false,
                                rollback_succeeded: None,
                            },
                            recovery_hints: Vec::new(),
                        });
                        completed_lease = Some(lease_id);
                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::Fetch,
                            per_project,
                            recovery_hints: vec![],
                        });
                    }
                }
                SyncPhase::PullRunning {
                    plan,
                    lease_id: phase_lease_id,
                    started_at,
                    completed,
                } => {
                    if *phase_lease_id != lease_id {
                        return Task::none();
                    }
                    if let Some(hint) = progress.recovery_hint.clone() {
                        // Recovery hint collected.
                        let _ = hint;
                    }
                    completed.push(progress.clone());

                    let expected = plan.entries.len();
                    let got = completed.len();
                    if got >= expected {
                        // Build final result from completed.
                        let mut outcomes: Vec<ProjectOutcome> = completed
                            .iter()
                            .map(|p| ProjectOutcome {
                                project_id: p.project_id.clone(),
                                project_name: p.project_name.clone(),
                                outcome: p.result.effective_outcome(),
                                success: p.result.success,
                                skip_reason: p.result.skip_reason.clone(),
                                commands_executed: p.result.commands_executed.clone(),
                                stdout: p.result.stdout.clone(),
                                stderr: p.result.stderr.clone(),
                                log_expanded: false,
                            })
                            .collect();
                        outcomes.extend(retry_outcomes);

                        let hints: Vec<_> = completed
                            .iter()
                            .filter_map(|p| p.recovery_hint.clone())
                            .collect();

                        let mut logged_results: Vec<_> =
                            completed.iter().map(|p| p.result.clone()).collect();
                        logged_results.extend(retry_exclusions.iter().map(skipped_retry_result));
                        completed_log = Some(OperationLog {
                            result: OperationResult {
                                operation_id: plan.id.clone(),
                                kind: OperationKind::SmartPull,
                                started_at: started_at.to_owned(),
                                finished_at: chrono::Utc::now(),
                                per_project: logged_results,
                                rollback_attempted: false,
                                rollback_succeeded: None,
                            },
                            recovery_hints: hints.clone(),
                        });

                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::SmartPull,
                            per_project: outcomes,
                            recovery_hints: hints,
                        });
                        state.sync.retry_exclusions.clear();
                        completed_lease = Some(lease_id);

                        // Trigger status refresh.
                        state.is_refreshing = true;
                        state.load_phase = LoadPhase::Refreshing;
                    }
                }
                _ => {}
            }
            if let Some(log) = completed_log {
                if let Some(lease_id) = completed_lease {
                    state.operation_interlock.release_if_matches(lease_id);
                }
                persist_log(&log, state);
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                return refresh_workspace_task(state);
            }
            Task::none()
        }

        BackgroundMessage::SingleFetchCompleted { lease_id, log } => {
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            for r in &log.result.per_project {
                state.fetching_projects.remove(&r.project_id);
            }
            persist_log(&log, state);

            let tasks: Vec<Task<Message>> = log
                .result
                .per_project
                .iter()
                .filter_map(|r| find_project(state, &r.project_id))
                .map(|project| {
                    Task::perform(
                        async move { VcsAdapter::read_project_status(&project).await },
                        |s| {
                            Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                                knotra_vcs::WorkspaceStatus {
                                    projects: vec![s],
                                    last_refresh: Some(chrono::Utc::now()),
                                },
                            ))
                        },
                    )
                })
                .collect();
            Task::batch(tasks)
        }

        BackgroundMessage::BulkFetchCompleted(log) => {
            persist_log(&log, state);
            state.status_bar = Some(if log.result.any_failed() {
                format!(
                    "Fetch — {} ok, {} failed",
                    log.result.successful_projects().len(),
                    log.result.failed_projects().len()
                )
            } else {
                format!("Fetch complete — {} projects", log.result.per_project.len())
            });
            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }

        BackgroundMessage::SmartPullCompleted(log)
        | BackgroundMessage::ContextSwitchCompleted(log)
        | BackgroundMessage::FreezeCompleted(log) => {
            persist_log(&log, state);
            Task::none()
        }

        BackgroundMessage::TagPushCompleted {
            lease_id,
            success_count,
            fail_count,
        } => {
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            state.pending_tag_push = None;
            state.status_bar = Some(if fail_count == 0 {
                format!(
                    "{} — {} {}",
                    state.t("plain.release.shared_status"),
                    success_count,
                    state.t("plain.release.projects_suffix")
                )
            } else {
                format!(
                    "{}: {} {} {} {}",
                    state.t("plain.release.share_failed_status"),
                    success_count,
                    state.t("plain.release.succeeded_suffix"),
                    fail_count,
                    state.t("plain.release.failed_suffix")
                )
            });
            Task::none()
        }

        BackgroundMessage::MissingProjectsDetected(ids) => {
            state.missing_projects = ids.into_iter().collect();
            state.reconcile_selection_with_display();
            Task::none()
        }

        BackgroundMessage::ConflictFilesLoaded(detail) => {
            let id = detail.project_id.clone();
            state.conflict_ops.cached.insert(id.clone(), detail.clone());
            state.conflict_ops.phase = ConflictPhase::Browsing {
                project_id: id,
                detail,
            };
            Task::none()
        }

        BackgroundMessage::ConflictOperationCompleted {
            lease_id,
            result,
            detail,
        } => {
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            let id = detail.project_id.clone();
            let success = result.success;
            let message = if success {
                state.t("plain.resolve.done").to_owned()
            } else {
                state.t("plain.resolve.failed").to_owned()
            };
            state.conflict_ops.cached.insert(id.clone(), detail.clone());
            if success {
                state.conflict_ops.phase = ConflictPhase::Browsing {
                    project_id: id,
                    detail,
                };
            } else {
                state.conflict_ops.phase = ConflictPhase::Done {
                    project_id: id,
                    success,
                    message,
                    result: Some(result),
                };
            }
            Task::none()
        }

        BackgroundMessage::ChangelogDraftReady { request_id, draft } => {
            if state.changelog.active_request_id == Some(request_id) {
                state.changelog.active_request_id = None;
                state.changelog.phase = ChangelogPhase::Ready(draft);
            }
            Task::none()
        }

        BackgroundMessage::TagsLoaded(tags) => {
            state.changelog.available_tags = tags;
            Task::none()
        }

        BackgroundMessage::TopologyScanned(graph) => {
            // Compute impact warnings for the Freezer.
            if let Some(ws) = &state.workspace {
                let names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();
                state.topology.impact_warnings = state.topology.compute_warnings(&graph, &names);
            }
            state.topology.phase = TopologyPhase::Ready(graph);
            Task::none()
        }

        BackgroundMessage::FreezeValidationDone {
            lease_id,
            validation,
        } => {
            if !matches!(
                state.freezer.phase,
                FreezerPhase::Validating {
                    lease_id: active_lease
                } if active_lease == lease_id
            ) {
                return Task::none();
            }
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            state.freezer.phase = FreezerPhase::ValidationReady(validation);
            Task::none()
        }

        BackgroundMessage::FreezeExecutionDone { lease_id, result } => {
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            use knotra_vcs::model::operation::{OperationKind, OperationLog, OperationResult};

            let started_at = state
                .freezer
                .execution_started_at
                .take()
                .unwrap_or_else(chrono::Utc::now);
            let finished_at = chrono::Utc::now();

            // Build per-project entries for the operation log.
            let per_project: Vec<_> = result
                .project_results
                .iter()
                .map(|r| knotra_vcs::model::operation::ProjectOperationResult {
                    project_id: r.project_id.clone(),
                    outcome: ProjectOperationOutcome::from_success(r.success),
                    success: r.success,
                    skip_reason: None,
                    commands_executed: r.commands_executed.clone(),
                    stdout: r.stdout.clone(),
                    stderr: r.stderr.clone(),
                    exit_code: None,
                    error_message: if r.success {
                        None
                    } else {
                        Some("freeze failed".to_owned())
                    },
                })
                .collect();

            let hints: Vec<_> = result
                .project_results
                .iter()
                .filter_map(|r| r.recovery_hint.clone())
                .collect();

            let op_log = OperationLog {
                result: OperationResult {
                    operation_id: OperationId::new(),
                    kind: OperationKind::Freeze,
                    started_at,
                    finished_at,
                    per_project,
                    rollback_attempted: result.project_results.iter().any(|r| r.rollback_attempted),
                    rollback_succeeded: {
                        let any_rb = result.project_results.iter().any(|r| r.rollback_attempted);
                        if any_rb {
                            Some(
                                result
                                    .project_results
                                    .iter()
                                    .filter(|r| r.rollback_attempted)
                                    .all(|r| r.rollback_succeeded == Some(true)),
                            )
                        } else {
                            None
                        }
                    },
                },
                recovery_hints: hints,
            };
            persist_log(&op_log, state);

            let push_offer = git_push_offer_for_freeze(state, &result);
            state.freezer.phase = FreezerPhase::Done(result);

            state.pending_tag_push = push_offer.map(|(freeze_name, project_ids)| PendingTagPush {
                freeze_name,
                project_ids,
                is_pushing: false,
            });
            Task::none()
        }

        BackgroundMessage::ContextListLoaded(list) => {
            let id = list.project_id.clone();
            state
                .context_ops
                .cached_lists
                .insert(id.clone(), list.clone());
            // Only update phase if we were waiting for this exact project.
            if matches!(&state.context_ops.phase, ContextPhase::LoadingList(loading_id) if loading_id == &id)
            {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list,
                    search: String::new(),
                };
            }
            Task::none()
        }

        BackgroundMessage::ContextSwitchDone { lease_id, result } => {
            if !state.operation_interlock.release_if_matches(lease_id) {
                return Task::none();
            }
            use knotra_vcs::model::operation::{OperationKind, OperationLog, OperationResult};

            // Build an operation log entry.
            let op_log = OperationLog {
                result: OperationResult {
                    operation_id: OperationId::new(),
                    kind: OperationKind::ContextSwitch,
                    started_at: chrono::Utc::now(),
                    finished_at: chrono::Utc::now(),
                    per_project: vec![result.operation_result.clone()],
                    rollback_attempted: false,
                    rollback_succeeded: None,
                },
                recovery_hints: result.recovery_hint.clone().into_iter().collect(),
            };
            persist_log(&op_log, state);

            state.context_ops.phase = ContextPhase::Done(result);

            // Refresh the project's status card after a switch.
            let project = match &state.context_ops.phase {
                ContextPhase::Done(r) => find_project(state, &r.project_id),
                _ => None,
            };
            if let Some(p) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&p).await },
                    |s| {
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                            knotra_vcs::WorkspaceStatus {
                                projects: vec![s],
                                last_refresh: Some(chrono::Utc::now()),
                            },
                        ))
                    },
                )
            } else {
                Task::none()
            }
        }

        BackgroundMessage::TaskError { description } => {
            state.load_phase = LoadPhase::Error(description.clone());
            state.is_refreshing = false;
            state.dashboard_error_details_open = false;
            state.status_bar = Some(state.t("dashboard.load_failed").to_owned());
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Freezer / History / Settings
// ---------------------------------------------------------------------------

// handle_freezer is defined below the background handler

fn handle_history(state: &mut AppState, msg: HistoryMessage) -> Task<Message> {
    match msg {
        HistoryMessage::SearchChanged(s) => {
            state.history_search = s;
        }
        HistoryMessage::EntryToggled(id) => {
            if state.history_expanded.contains(&id) {
                state.history_expanded.remove(&id);
            } else {
                state.history_expanded.insert(id);
            }
        }
        HistoryMessage::LogCopyRequested(_id) => {
            // Clipboard access is platform-dependent; Phase 7 can wire iced's clipboard API.
            // For now we record the intent and show a status-bar note.
            // Real clipboard write is handled by Message::CopyToClipboard.
            // This is a fallback status note in case no text was available.
            state.status_bar = Some("Copy command sent.".to_owned());
        }
        HistoryMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
        }
    }
    Task::none()
}

fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(l) => {
            state.config.locale = l;
            state.catalog = knotra_ui::i18n::Catalog::for_locale(l);
        }
        SettingsMessage::ThemeChanged(dark) => {
            state.config.dark_theme = dark;
            state.theme = if dark {
                knotra_ui::KnotraTheme::dark()
            } else {
                knotra_ui::KnotraTheme::light()
            };
        }
        SettingsMessage::RefreshIntervalChanged(s) => {
            state.settings_edit.refresh_interval_secs = s.to_string();
            state.config.refresh_interval_secs = s;
        }
        SettingsMessage::MaxConcurrentChanged(n) => {
            state.settings_edit.max_concurrent_reads = n.to_string();
            state.config.max_concurrent_reads = n;
        }
        SettingsMessage::ExternalEditorChanged(s) => {
            state.settings_edit.external_editor = s.clone();
            state.config.external_editor = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
        }
        SettingsMessage::ExternalMergeToolChanged(s) => {
            state.settings_edit.external_merge_tool = s.clone();
            state.config.external_merge_tool = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
        }
        SettingsMessage::MaxLogEntriesChanged(n) => {
            state.settings_edit.max_log_entries = n.to_string();
            state.config.max_log_entries = n;
        }
        SettingsMessage::FsWatchEnabledChanged(v) => {
            state.config.fs_watch_enabled = v;
            if !v {
                state.settings_save_msg = Some("FS watching disabled.".to_owned());
            }
        }
        SettingsMessage::FsDebounceSecs(n) => {
            state.settings_edit.refresh_interval_secs = n.to_string();
            state.config.fs_debounce_secs = n;
        }
        SettingsMessage::SaveRequested => match save_config(&state.config, &state.paths) {
            Ok(()) => {
                state.settings_save_msg = Some(state.t("settings.saved_ok").to_owned());
                state.status_bar = Some(state.t("settings.saved_ok").to_owned());
            }
            Err(e) => {
                state.settings_save_msg = Some(format!("{} {e}", state.t("settings.save_error")));
            }
        },
        SettingsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_project_name(state: &AppState, id: &knotra_vcs::ProjectId) -> Option<String> {
    find_project(state, id).map(|p| p.name)
}

fn merge_workspace_status(state: &mut AppState, new: knotra_vcs::WorkspaceStatus) {
    if let Some(existing) = &mut state.workspace_status {
        for ps in new.projects {
            if let Some(pos) = existing
                .projects
                .iter()
                .position(|p| p.project_id == ps.project_id)
            {
                existing.projects[pos] = ps;
            } else {
                existing.projects.push(ps);
            }
        }
        existing.last_refresh = new.last_refresh;
    } else {
        state.workspace_status = Some(new);
    }
    state.reconcile_selection_with_display();
}

fn persist_workspace(paths: &AppPaths, ws: &Workspace) {
    if let Err(e) = save_workspace(ws, paths) {
        tracing::warn!("failed to save workspace: {e}");
    }
}

fn persist_log(log: &OperationLog, state: &mut AppState) {
    if let Err(e) = save_operation_log(log, &state.paths) {
        tracing::warn!("failed to save operation log: {e}");
        state.status_bar = Some(state.t("plain.activity.log_save_failed").to_owned());
    }
    state.operation_logs.insert(0, log.clone());
    state.operation_logs.truncate(state.config.max_log_entries);
    let failed_ids: Vec<_> = log
        .result
        .per_project
        .iter()
        .filter(|result| result.effective_outcome() == ProjectOperationOutcome::Failed)
        .map(|result| result.project_id.clone())
        .collect();
    let retry = if failed_ids.is_empty() {
        RetryAvailability::NotApplicable
    } else {
        match log.result.kind {
            OperationKind::Fetch => {
                RetryAvailability::Available(ActivityRetryAction::FetchFailed {
                    source_operation_id: log.result.operation_id.clone(),
                    project_ids: failed_ids,
                })
            }
            OperationKind::SmartPull => {
                RetryAvailability::Available(ActivityRetryAction::ReviewSmartPull {
                    source_operation_id: log.result.operation_id.clone(),
                    project_ids: failed_ids,
                })
            }
            OperationKind::ContextSwitch => {
                RetryAvailability::Unavailable(RetryUnavailableReason::ContextSwitch)
            }
            OperationKind::Freeze => RetryAvailability::Unavailable(RetryUnavailableReason::Freeze),
            OperationKind::FreezeRollback => {
                RetryAvailability::Unavailable(RetryUnavailableReason::FreezeRollback)
            }
            OperationKind::StatusRefresh => {
                RetryAvailability::Unavailable(RetryUnavailableReason::StatusRefresh)
            }
        }
    };
    state.activity.latest = crate::state::LatestOpState::Completed {
        log: log.clone(),
        retry,
    };
    state.activity.completed_secs = 0;
}

fn split_retry_targets(
    state: &AppState,
    ids: &[knotra_vcs::ProjectId],
) -> (Vec<Project>, Vec<RetryExclusion>) {
    let mut projects = Vec::new();
    let mut exclusions = Vec::new();
    for id in ids {
        let Some(project) = find_project(state, id) else {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::NotInActiveWorkspace,
            });
            continue;
        };
        if !Path::new(&project.path).exists() {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::ProjectPathMissing,
            });
        } else if !VcsAdapter::repo_exists(&project) {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::UnsupportedRepository,
            });
        } else {
            projects.push(project);
        }
    }
    (projects, exclusions)
}

fn skipped_retry_result(exclusion: &RetryExclusion) -> ProjectOperationResult {
    ProjectOperationResult {
        project_id: exclusion.project_id.clone(),
        outcome: ProjectOperationOutcome::Skipped,
        success: true,
        skip_reason: Some(exclusion.reason.code().to_owned()),
        commands_executed: Vec::new(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        error_message: None,
    }
}

fn git_push_offer_for_freeze(
    state: &AppState,
    result: &knotra_vcs::FreezeResult,
) -> Option<(String, Vec<knotra_vcs::ProjectId>)> {
    if result.outcome != knotra_vcs::FreezeOutcome::Success {
        return None;
    }

    let ids: Vec<_> = result
        .project_results
        .iter()
        .filter(|r| r.success && project_is_git_for_push(state, &r.project_id))
        .map(|r| r.project_id.clone())
        .collect();

    (!ids.is_empty()).then(|| (result.freeze_name.clone(), ids))
}

fn project_is_git_for_push(state: &AppState, project_id: &knotra_vcs::ProjectId) -> bool {
    if let Some(status) = state.workspace_status.as_ref().and_then(|ws| {
        ws.projects
            .iter()
            .find(|status| &status.project_id == project_id)
    }) {
        return status.identity.vcs_kind == VcsKind::Git;
    }

    let Some(project) = find_project(state, project_id) else {
        return false;
    };
    let path = std::path::Path::new(&project.path);
    !path.join(".jj").is_dir() && path.join(".git").exists()
}

fn smart_pull_skip_reason_text(reason: &SmartPullSkipReason) -> &'static str {
    match reason {
        SmartPullSkipReason::Deselected => "Not selected.",
        SmartPullSkipReason::NoUpstream => "No update source is configured.",
        SmartPullSkipReason::Conflict => "Needs your choice first.",
        SmartPullSkipReason::MissingStatus => "Status is not available.",
        SmartPullSkipReason::ProjectNotFound => "Project was not found.",
    }
}

fn context_switch_disabled_reason(
    status: Option<&knotra_vcs::ProjectStatus>,
) -> Option<&'static str> {
    let status = status?;
    if status.read_error.is_some() {
        Some("plain.switch.reason_unavailable")
    } else if status.conflict.has_conflict {
        Some("plain.switch.reason_conflict")
    } else if status.working_tree.is_dirty() {
        Some("plain.switch.reason_dirty")
    } else {
        None
    }
}

fn blocked_context_switch_result(project: &Project, reason: String) -> ProjectOperationResult {
    ProjectOperationResult {
        project_id: project.id.clone(),
        outcome: ProjectOperationOutcome::Failed,
        success: false,
        skip_reason: None,
        commands_executed: vec![],
        stdout: String::new(),
        stderr: reason.clone(),
        exit_code: Some(1),
        error_message: Some(reason),
    }
}

// ---------------------------------------------------------------------------
// Context Operations handler
// ---------------------------------------------------------------------------

fn handle_context(state: &mut AppState, msg: ContextMessage) -> Task<Message> {
    match msg {
        ContextMessage::OpenRequested(preselect_id) => {
            state.active_modal = crate::state::ActiveModal::Switch;
            state.context_ops.phase = ContextPhase::Idle;

            // If a project was pre-selected (e.g. from a dashboard card shortcut), load it.
            if let Some(id) = preselect_id
                && let Some(project) = find_project(state, &id)
            {
                state.context_ops.phase = ContextPhase::LoadingList(id.clone());
                return Task::perform(
                    async move { VcsAdapter::list_contexts(&project).await },
                    |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
                );
            }
            Task::none()
        }

        ContextMessage::ProjectSelected(id) => {
            let project = match find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };

            // Use cached list if present, otherwise fetch.
            if let Some(cached) = state.context_ops.cached_lists.get(&id).cloned() {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list: cached,
                    search: String::new(),
                };
                return Task::none();
            }

            state.context_ops.phase = ContextPhase::LoadingList(id.clone());
            Task::perform(
                async move { VcsAdapter::list_contexts(&project).await },
                |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
            )
        }

        ContextMessage::SearchChanged(s) => {
            if let ContextPhase::BrowsingList { search, .. } = &mut state.context_ops.phase {
                *search = s;
            }
            Task::none()
        }

        ContextMessage::SwitchTargetChosen(project_id, target, target_label) => {
            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let status = state
                .workspace_status
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id));
            let vcs_kind = status
                .map(|s| s.identity.vcs_kind)
                .unwrap_or(knotra_vcs::VcsKind::Git);

            let is_dirty = status.map(|s| s.working_tree.is_dirty()).unwrap_or(false);
            let disabled_reason_key = context_switch_disabled_reason(status);

            state.context_ops.phase = ContextPhase::ConfirmSwitch {
                project_id,
                project_name: project.name.clone(),
                target,
                target_label,
                vcs_kind,
                is_dirty,
                disabled_reason_key,
            };
            Task::none()
        }

        ContextMessage::SwitchConfirmed => {
            let (project_id, target, target_label, project_name, disabled_reason_key) =
                match &state.context_ops.phase {
                    ContextPhase::ConfirmSwitch {
                        project_id,
                        target,
                        target_label,
                        project_name,
                        disabled_reason_key,
                        ..
                    } => (
                        project_id.clone(),
                        target.clone(),
                        target_label.clone(),
                        project_name.clone(),
                        *disabled_reason_key,
                    ),
                    _ => return Task::none(),
                };
            if disabled_reason_key.is_some() {
                return Task::none();
            }

            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let Some(lease_id) = acquire_operation(state, OperationOwner::ContextSwitch) else {
                return Task::none();
            };

            state.context_ops.phase = ContextPhase::Switching {
                project_id: project_id.clone(),
                target: target.clone(),
                target_label: target_label.clone(),
            };
            // Invalidate cached list for this project.
            state.context_ops.cached_lists.remove(&project_id);

            let unavailable_reason = state.t("plain.switch.reason_unavailable").to_owned();
            let conflict_reason = state.t("plain.switch.reason_conflict").to_owned();
            let dirty_reason = state.t("plain.switch.reason_dirty").to_owned();

            Task::perform(
                async move {
                    let latest_status = VcsAdapter::read_project_status(&project).await;
                    let blocked_reason =
                        context_switch_disabled_reason(Some(&latest_status)).map(|key| match key {
                            "plain.switch.reason_unavailable" => unavailable_reason.clone(),
                            "plain.switch.reason_conflict" => conflict_reason.clone(),
                            "plain.switch.reason_dirty" => dirty_reason.clone(),
                            _ => key.to_owned(),
                        });
                    let (result, hint) = if let Some(reason) = blocked_reason {
                        (blocked_context_switch_result(&project, reason), None)
                    } else {
                        VcsAdapter::switch_context(&project, &target).await
                    };
                    ContextSwitchResult {
                        project_id: project.id,
                        project_name,
                        target: target_label,
                        operation_result: result,
                        recovery_hint: hint,
                    }
                },
                move |result| {
                    Message::Background(BackgroundMessage::ContextSwitchDone { lease_id, result })
                },
            )
        }

        ContextMessage::SwitchCancelled => {
            // Return to browsing.
            let prev_id = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, .. } => Some(project_id.clone()),
                _ => None,
            };
            if let Some(id) = prev_id
                && let Some(cached) = state.context_ops.cached_lists.get(&id).cloned()
            {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list: cached,
                    search: String::new(),
                };
                return Task::none();
            }
            state.context_ops.phase = ContextPhase::Idle;
            Task::none()
        }

        ContextMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ContextMessage::BulkOpenRequested => {
            let selected = state.selection_summary().selected_ids;
            let Some(project_id) = selected.first().cloned().filter(|_| selected.len() == 1) else {
                return Task::none();
            };
            let Some(project) = find_project(state, &project_id) else {
                return Task::none();
            };
            state.active_modal = crate::state::ActiveModal::Switch;
            state.context_ops.phase = ContextPhase::LoadingList(project_id);
            Task::perform(
                async move { VcsAdapter::list_contexts(&project).await },
                |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
            )
        }
        ContextMessage::BulkModalClosed => {
            if matches!(state.context_ops.phase, ContextPhase::Switching { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        ContextMessage::Cancelled => {
            if matches!(state.context_ops.phase, ContextPhase::Switching { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Freezer handler
// ---------------------------------------------------------------------------

fn handle_freezer(state: &mut AppState, msg: FreezerMessage) -> Task<Message> {
    #[allow(unreachable_patterns)]
    match msg {
        FreezerMessage::OpenRequested => {
            cancel_freezer_validation(state);
            // Reinitialise project selection from workspace.
            if let Some(ws) = &state.workspace {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                state.freezer.init_selection(&ids);
            }
            state.pending_tag_push = None;
            state.freezer.execution_started_at = None;
            state.freezer.phase = FreezerPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Tag;
            Task::none()
        }

        FreezerMessage::NameChanged(name) => {
            cancel_freezer_validation(state);
            state.freezer.freeze_name = name;
            // Reset to Idle when the name changes after validation.
            if matches!(state.freezer.phase, FreezerPhase::ValidationReady(_)) {
                state.freezer.phase = FreezerPhase::Idle;
            }
            Task::none()
        }

        FreezerMessage::TagMessageChanged(s) => {
            state.freezer.tag_message = s;
            Task::none()
        }
        FreezerMessage::ExecuteConfirmed | FreezerMessage::ExecuteRequested => {
            start_freeze_execution(state)
        }
        FreezerMessage::BulkOpenRequested => {
            cancel_freezer_validation(state);
            state.active_modal = crate::state::ActiveModal::Tag;
            state.freezer.phase = FreezerPhase::Idle;
            state.freezer.execution_started_at = None;
            state.pending_tag_push = None;
            // Pre-populate freeze selection
            state.freezer.project_selection = state
                .selection
                .selected_ids
                .iter()
                .map(|id| (id.clone(), true))
                .collect();
            open_overlay_focus(
                state,
                focus::FocusTarget::text_input(knotra_ui::widget::focus_id::RELEASE_NAME.clone()),
            )
        }
        FreezerMessage::BulkModalClosed => {
            if freezer_is_running(state) {
                return Task::none();
            }
            cancel_freezer_validation(state);
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }

        FreezerMessage::ProjectToggled(id, included) => {
            cancel_freezer_validation(state);
            state.freezer.project_selection.insert(id, included);
            // Invalidate validation when selection changes.
            if matches!(state.freezer.phase, FreezerPhase::ValidationReady(_)) {
                state.freezer.phase = FreezerPhase::Idle;
            }
            Task::none()
        }

        FreezerMessage::ValidateRequested | FreezerMessage::RevalidateRequested => {
            if !state.freezer.freeze_name_is_valid() {
                return Task::none(); // view blocks the button; defensive guard
            }

            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.clone())
                .unwrap_or_default();
            let selection = state.freezer.selected_ids();
            let freeze_name = state.freezer.freeze_name.clone();
            let max = state.config.max_concurrent_reads;
            let Some(lease_id) = acquire_operation(state, OperationOwner::FreezeValidation) else {
                return Task::none();
            };

            state.freezer.phase = FreezerPhase::Validating { lease_id };
            state.freezer.execution_started_at = None;

            Task::perform(
                async move {
                    VcsAdapter::validate_freeze(&projects, &selection, &freeze_name, max).await
                },
                move |validation| {
                    Message::Background(BackgroundMessage::FreezeValidationDone {
                        lease_id,
                        validation,
                    })
                },
            )
        }

        FreezerMessage::Cancelled => {
            if freezer_is_running(state) {
                return Task::none();
            }
            cancel_freezer_validation(state);
            state.freezer.execution_started_at = None;
            state.freezer.phase = FreezerPhase::Idle;
            Task::none()
        }

        FreezerMessage::BackToDashboard => {
            if freezer_is_running(state) {
                return Task::none();
            }
            cancel_freezer_validation(state);
            state.screen = Screen::Dashboard;
            state.freezer.execution_started_at = None;
            state.freezer.phase = FreezerPhase::Idle;
            Task::none()
        }
    }
}

fn start_freeze_execution(state: &mut AppState) -> Task<Message> {
    let validation = match &state.freezer.phase {
        FreezerPhase::ValidationReady(validation)
            if validation.all_ready() && validation.ready_count() > 0 =>
        {
            validation.clone()
        }
        _ => return Task::none(),
    };

    let projects: Vec<_> = state
        .workspace
        .as_ref()
        .map(|ws| ws.projects.clone())
        .unwrap_or_default();
    let tag_message = state.freezer.tag_message.trim().to_owned();
    let tag_message = (!tag_message.is_empty()).then_some(tag_message);
    let Some(lease_id) = acquire_operation(state, OperationOwner::FreezeExecution) else {
        return Task::none();
    };

    state.freezer.execution_started_at = Some(chrono::Utc::now());
    state.freezer.phase = FreezerPhase::Executing;
    state.pending_tag_push = None;

    Task::perform(
        async move {
            VcsAdapter::execute_freeze_with_message(&projects, &validation, tag_message.as_deref())
                .await
        },
        move |result| {
            Message::Background(BackgroundMessage::FreezeExecutionDone { lease_id, result })
        },
    )
}

// ---------------------------------------------------------------------------
// External tool launch handler
// ---------------------------------------------------------------------------

fn handle_launch(state: &mut AppState, msg: LaunchMessage) -> Task<Message> {
    let (tool_path, file_path) = match msg {
        LaunchMessage::OpenInEditor(path) => (state.config.external_editor.clone(), path),
        LaunchMessage::OpenInMergeTool(path) => (state.config.external_merge_tool.clone(), path),
    };

    let Some(tool) = tool_path else {
        state.status_bar = Some(state.t("tool.not_configured").to_owned());
        return Task::none();
    };

    match std::process::Command::new(&tool).arg(&file_path).spawn() {
        Ok(_) => {
            state.status_bar = Some(format!("Launched: {} {:?}", tool, file_path));
        }
        Err(e) => {
            state.status_bar = Some(format!("{} {}: {e}", state.t("tool.launch_failed"), tool));
        }
    }
    Task::none()
}

pub(crate) fn resolve_project_file_path(
    project: &Project,
    file_path: &str,
) -> Result<PathBuf, &'static str> {
    let root = std::fs::canonicalize(&project.path).map_err(|_| "plain.error.path_missing")?;
    let raw = Path::new(file_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        if raw.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err("plain.resolve.file_outside_project");
        }
        root.join(raw)
    };
    let resolved = std::fs::canonicalize(&candidate).map_err(|_| "plain.resolve.file_missing")?;
    if !resolved.starts_with(&root) {
        return Err("plain.resolve.file_outside_project");
    }
    Ok(resolved)
}

fn active_conflict_project_id(state: &AppState) -> Option<knotra_vcs::ProjectId> {
    match &state.conflict_ops.phase {
        ConflictPhase::Loading(id)
        | ConflictPhase::Browsing { project_id: id, .. }
        | ConflictPhase::Operating { project_id: id, .. }
        | ConflictPhase::Done { project_id: id, .. } => Some(id.clone()),
        ConflictPhase::Idle => match &state.active_modal {
            crate::state::ActiveModal::Resolve(id) => Some(id.clone()),
            _ => None,
        },
    }
}

fn project_supports_git_conflict_actions(
    state: &AppState,
    project_id: &knotra_vcs::ProjectId,
) -> bool {
    state
        .workspace_status
        .as_ref()
        .and_then(|ws| {
            ws.projects
                .iter()
                .find(|status| &status.project_id == project_id)
        })
        .map(|status| status.identity.vcs_kind == VcsKind::Git)
        .unwrap_or_else(|| {
            find_project(state, project_id)
                .map(|project| {
                    let path = Path::new(&project.path);
                    !path.join(".jj").is_dir() && path.join(".git").exists()
                })
                .unwrap_or(false)
        })
}

fn project_has_git_merge_state(state: &AppState, project_id: &knotra_vcs::ProjectId) -> bool {
    find_project(state, project_id)
        .map(|project| {
            let path = Path::new(&project.path);
            path.join(".git").join("MERGE_HEAD").exists()
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Conflict resolution handler
// ---------------------------------------------------------------------------

fn handle_conflict_ops(state: &mut AppState, msg: ConflictOpsMessage) -> Task<Message> {
    match msg {
        ConflictOpsMessage::OpenRequested(preselect) => {
            state.conflict_ops.phase = ConflictPhase::Idle;
            if let Some(id) = preselect {
                state.active_modal = crate::state::ActiveModal::Resolve(id.clone());
                return Task::done(Message::ConflictOps(ConflictOpsMessage::ProjectSelected(
                    id,
                )));
            }
            Task::none()
        }

        ConflictOpsMessage::ProjectSelected(id) => {
            if let Some(cached) = state.conflict_ops.cached.get(&id).cloned() {
                state.conflict_ops.phase = ConflictPhase::Browsing {
                    project_id: id,
                    detail: cached,
                };
                return Task::none();
            }
            let project = match find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::list_conflicted_files(&project).await },
                |d| Message::Background(BackgroundMessage::ConflictFilesLoaded(d)),
            )
        }

        ConflictOpsMessage::RecheckRequested(id) => {
            state.conflict_ops.cached.remove(&id);
            let project = match find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::list_conflicted_files(&project).await },
                |d| Message::Background(BackgroundMessage::ConflictFilesLoaded(d)),
            )
        }

        ConflictOpsMessage::MarkResolvedRequested {
            project_id,
            file_path,
        } => {
            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            if !project_supports_git_conflict_actions(state, &project_id) {
                state.conflict_ops.phase = ConflictPhase::Done {
                    project_id,
                    success: false,
                    message: state.t("plain.resolve.unsupported").to_owned(),
                    result: None,
                };
                return Task::none();
            }
            let Some(lease_id) = acquire_operation(state, OperationOwner::ConflictMutation) else {
                return Task::none();
            };
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: project_id.clone(),
                action: state.t("plain.resolve.marking").to_owned(),
            };
            state.conflict_ops.cached.remove(&project_id);
            Task::perform(
                async move {
                    let result = VcsAdapter::mark_resolved(&project, &file_path).await;
                    let detail = VcsAdapter::list_conflicted_files(&project).await;
                    (result, detail)
                },
                move |(result, detail)| {
                    Message::Background(BackgroundMessage::ConflictOperationCompleted {
                        lease_id,
                        result,
                        detail,
                    })
                },
            )
        }

        ConflictOpsMessage::AbortMergeRequested(id) => {
            if !project_supports_git_conflict_actions(state, &id)
                || !project_has_git_merge_state(state, &id)
            {
                state.conflict_ops.phase = ConflictPhase::Done {
                    project_id: id,
                    success: false,
                    message: state.t("plain.resolve.stop_unavailable").to_owned(),
                    result: None,
                };
                return Task::none();
            }
            let project = match find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let Some(lease_id) = acquire_operation(state, OperationOwner::ConflictMutation) else {
                return Task::none();
            };
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: id.clone(),
                action: state.t("plain.resolve.stopping").to_owned(),
            };
            state.conflict_ops.cached.remove(&id);
            Task::perform(
                async move {
                    let result = VcsAdapter::abort_merge(&project).await;
                    let detail = VcsAdapter::list_conflicted_files(&project).await;
                    (result, detail)
                },
                move |(result, detail)| {
                    Message::Background(BackgroundMessage::ConflictOperationCompleted {
                        lease_id,
                        result,
                        detail,
                    })
                },
            )
        }

        ConflictOpsMessage::AbortMergeConfirmed(id) => Task::done(Message::ConflictOps(
            ConflictOpsMessage::AbortMergeRequested(id),
        )),

        ConflictOpsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ConflictOpsMessage::FileMarkedResolved(path) => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            Task::done(Message::ConflictOps(
                ConflictOpsMessage::MarkResolvedRequested {
                    project_id,
                    file_path: path,
                },
            ))
        }
        ConflictOpsMessage::OpenInEditorRequested(path) => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            let Some(project) = find_project(state, &project_id) else {
                state.status_bar = Some(state.t("plain.error.path_missing").to_owned());
                return Task::none();
            };
            let resolved = match resolve_project_file_path(&project, &path) {
                Ok(path) => path,
                Err(key) => {
                    state.status_bar = Some(state.t(key).to_owned());
                    return Task::none();
                }
            };
            Task::done(Message::Launch(LaunchMessage::OpenInEditor(
                resolved.to_string_lossy().into_owned(),
            )))
        }
        ConflictOpsMessage::AbortRequested => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            Task::done(Message::ConflictOps(
                ConflictOpsMessage::AbortMergeRequested(project_id),
            ))
        }
        ConflictOpsMessage::PanelClosed => {
            if matches!(state.conflict_ops.phase, ConflictPhase::Operating { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Changelog handler
// ---------------------------------------------------------------------------

fn handle_changelog(state: &mut AppState, msg: ChangelogMessage) -> Task<Message> {
    match msg {
        ChangelogMessage::OpenRequested => {
            state.changelog.invalidate_collection();
            if let Some(ws) = &state.workspace {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                state.changelog.init_selection(&ids);
            }
            state.changelog.phase = ChangelogPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Changelog;
            Task::none()
        }

        ChangelogMessage::BulkOpenRequested => {
            let selected = state.selection_summary().selected_ids;
            if selected.is_empty() {
                return Task::none();
            }
            state.changelog.invalidate_collection();
            state.changelog.project_selection = selected.into_iter().map(|id| (id, true)).collect();
            state.changelog.phase = ChangelogPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Changelog;
            Task::none()
        }

        ChangelogMessage::SinceRefChanged(s) => {
            state.changelog.since_ref = s;
            if matches!(
                state.changelog.phase,
                ChangelogPhase::Ready(_) | ChangelogPhase::Collecting
            ) {
                state.changelog.phase = ChangelogPhase::Idle;
            }
            state.changelog.invalidate_collection();
            Task::none()
        }

        ChangelogMessage::ProjectToggled(id, v) => {
            state.changelog.project_selection.insert(id, v);
            if matches!(state.changelog.phase, ChangelogPhase::Ready(_)) {
                state.changelog.phase = ChangelogPhase::Idle;
            }
            state.changelog.invalidate_collection();
            Task::none()
        }

        ChangelogMessage::LoadTagsRequested => {
            // Load tags from the first selected project.
            let project = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.first().cloned());
            if let Some(project) = project {
                return Task::perform(
                    async move { VcsAdapter::list_tags(&project).await },
                    |tags| Message::Background(BackgroundMessage::TagsLoaded(tags)),
                );
            }
            Task::none()
        }

        ChangelogMessage::GenerateRequested => {
            if !state.changelog.is_ready_to_collect() {
                return Task::none();
            }
            let selected_ids = state.changelog.selected_ids();
            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .filter(|p| selected_ids.contains(&p.id))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if projects.is_empty() {
                return Task::none();
            }
            let since = state.changelog.since_ref.clone();
            let max_cl = state.config.max_concurrent_reads;
            let request_id = state.changelog.begin_collection();

            Task::perform(
                async move { VcsAdapter::collect_changelog(&projects, &since, max_cl).await },
                move |draft| {
                    Message::Background(BackgroundMessage::ChangelogDraftReady {
                        request_id,
                        draft,
                    })
                },
            )
        }

        ChangelogMessage::CopyRequested => {
            if let ChangelogPhase::Ready(ref draft) = state.changelog.phase {
                let md = draft.to_markdown();
                state.status_bar = Some(format!(
                    "{} {} {}",
                    state.t("plain.changelog.copied_prefix"),
                    md.len(),
                    state.t("plain.changelog.copied_suffix")
                ));
                return clipboard::write(md);
            }
            Task::none()
        }

        ChangelogMessage::BackToDashboard => {
            state.changelog.phase = ChangelogPhase::Idle;
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ChangelogMessage::CollectRequested => {
            Task::done(Message::Changelog(ChangelogMessage::GenerateRequested))
        }
        ChangelogMessage::ModalClosed => {
            state.changelog.invalidate_collection();
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Topology handler
// ---------------------------------------------------------------------------

fn handle_topology(state: &mut AppState, msg: TopologyMessage) -> Task<Message> {
    match msg {
        TopologyMessage::ScanRequested => {
            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.clone())
                .unwrap_or_default();
            state.topology.phase = TopologyPhase::Scanning;

            Task::perform(
                async move { VcsAdapter::scan_topology(&projects).await },
                |graph| Message::Background(BackgroundMessage::TopologyScanned(graph)),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// FS watch tick handler
// ---------------------------------------------------------------------------

fn handle_fs_watch_tick(state: &mut AppState) -> Task<Message> {
    // Skip if already refreshing or FS watching is disabled.
    if state.is_refreshing || !state.config.fs_watch_enabled {
        return Task::none();
    }

    // Build project list for the poller.
    let projects: Vec<(knotra_vcs::ProjectId, String)> = state
        .workspace
        .as_ref()
        .map(|ws| {
            ws.projects
                .iter()
                .map(|p| (p.id.clone(), p.path.clone()))
                .collect()
        })
        .unwrap_or_default();

    if projects.is_empty() {
        return Task::none();
    }

    // Prune stale snapshots.
    let ids: Vec<_> = projects.iter().map(|(id, _)| id.clone()).collect();
    state.fs_poller.prune(&ids);

    // Poll for changes.
    let changed = state.fs_poller.poll(&projects);
    if changed.is_empty() {
        return Task::none();
    }

    tracing::debug!(
        "FS change detected in {} project(s) — triggering status refresh",
        changed.len()
    );

    // If only a few projects changed, refresh them individually.
    // Otherwise fall back to a full workspace refresh.
    let _max = state.config.max_concurrent_reads;

    if changed.len() <= 3 {
        let changed_projects: Vec<_> = changed
            .iter()
            .filter_map(|e| find_project(state, &e.project_id))
            .collect();

        let tasks: Vec<Task<Message>> = changed_projects
            .into_iter()
            .map(|project| {
                Task::perform(
                    async move { knotra_vcs::VcsAdapter::read_project_status(&project).await },
                    |s| {
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                            knotra_vcs::WorkspaceStatus {
                                projects: vec![s],
                                last_refresh: Some(chrono::Utc::now()),
                            },
                        ))
                    },
                )
            })
            .collect();

        Task::batch(tasks)
    } else {
        // Full refresh for large change sets.
        state.is_refreshing = true;
        state.load_phase = LoadPhase::Refreshing;
        refresh_workspace_task(state)
    }
}

// ---------------------------------------------------------------------------
// Tag push handler
// ---------------------------------------------------------------------------

fn handle_tag_push(state: &mut AppState, msg: TagPushMessage) -> Task<Message> {
    match msg {
        TagPushMessage::OfferShown {
            freeze_name,
            project_ids,
        } => {
            state.pending_tag_push = Some(PendingTagPush {
                freeze_name,
                project_ids,
                is_pushing: false,
            });
            Task::none()
        }

        TagPushMessage::PushConfirmed => {
            let push = match &state.pending_tag_push {
                Some(p) => p.clone(),
                None => return Task::none(),
            };
            let Some(lease_id) = acquire_operation(state, OperationOwner::TagPush) else {
                return Task::none();
            };
            if let Some(ref mut p) = state.pending_tag_push {
                p.is_pushing = true;
            }

            let projects: Vec<_> = push
                .project_ids
                .iter()
                .filter_map(|id| find_project(state, id))
                .collect();
            let tag_name = push.freeze_name.clone();
            let max = state.config.max_concurrent_reads;

            Task::perform(
                async move {
                    use std::sync::Arc;
                    use tokio::sync::Semaphore;

                    let sem = Arc::new(Semaphore::new(max));
                    let mut handles = Vec::new();
                    for project in projects {
                        let sem = Arc::clone(&sem);
                        let tag = tag_name.clone();
                        handles.push(tokio::spawn(async move {
                            let _permit = sem.acquire().await.expect("open");
                            knotra_vcs::VcsAdapter::push_tag(&project, &tag).await
                        }));
                    }
                    let mut results = Vec::new();
                    for h in handles {
                        if let Ok(r) = h.await {
                            results.push(r);
                        }
                    }
                    let success = results.iter().filter(|r| r.success).count();
                    let failed = results.iter().filter(|r| !r.success).count();
                    (success, failed)
                },
                move |(success_count, fail_count)| {
                    Message::Background(BackgroundMessage::TagPushCompleted {
                        lease_id,
                        success_count,
                        fail_count,
                    })
                },
            )
        }

        TagPushMessage::PushDeclined => {
            if state
                .pending_tag_push
                .as_ref()
                .is_some_and(|push| push.is_pushing)
            {
                return Task::none();
            }
            state.pending_tag_push = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// RFC-0009 — Selection handler
// ---------------------------------------------------------------------------

fn handle_selection(state: &mut AppState, msg: SelectionMessage) -> Task<Message> {
    let ordered: Vec<knotra_vcs::ProjectId> = state.visible_project_ids();

    match msg {
        SelectionMessage::ModeEntered => state.selection_mode = true,
        SelectionMessage::ModeExited => state.clear_selection_mode(),
        SelectionMessage::Toggled(id) => {
            let active_ids: std::collections::HashSet<_> = ordered.iter().cloned().collect();
            if !active_ids.contains(&id) {
                return Task::none();
            }
            state.selection_mode = true; // selecting anything enters mode
            state.selection.toggle(id);
        }
        SelectionMessage::RangeTo(id) => {
            if !ordered.contains(&id) {
                return Task::none();
            }
            state.selection_mode = true;
            state.selection.select_range(&ordered, &id);
        }
        SelectionMessage::SelectAll => {
            state.selection_mode = true;
            let ids = state.visible_project_ids();
            state.selection.clear();
            state.selection.select_all(&ids);
        }
        SelectionMessage::Clear => state.clear_selection_mode(),
        SelectionMessage::FocusMoved(_) => {} // focus tracking only
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// RFC-0011 — Activity strip handler
// ---------------------------------------------------------------------------

fn handle_activity(state: &mut AppState, msg: ActivityMessage) -> Task<Message> {
    match msg {
        ActivityMessage::RetryRequested {
            source_operation_id,
        } => {
            let action = match &state.activity.latest {
                crate::state::LatestOpState::Completed {
                    retry: RetryAvailability::Available(action),
                    ..
                } => action.clone(),
                _ => return Task::none(),
            };
            match action {
                ActivityRetryAction::FetchFailed {
                    source_operation_id: expected,
                    project_ids,
                } if expected == source_operation_id => {
                    return start_activity_fetch_retry(state, expected, project_ids);
                }
                ActivityRetryAction::ReviewSmartPull {
                    source_operation_id: expected,
                    project_ids,
                } if expected == source_operation_id => {
                    return start_activity_smart_pull_review(state, expected, project_ids);
                }
                _ => return Task::none(),
            }
        }
        ActivityMessage::DetailsRequested { operation_id } => {
            state.history_expanded.insert(operation_id);
            state.screen = Screen::History;
        }
        ActivityMessage::Tick => {
            state.activity.completed_secs = state.activity.completed_secs.saturating_add(1);
        }
    }
    Task::none()
}

fn start_activity_fetch_retry(
    state: &mut AppState,
    source_operation_id: OperationId,
    project_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    let (projects, exclusions) = split_retry_targets(state, &project_ids);
    if projects.is_empty() {
        mark_activity_retry_unavailable(state, &source_operation_id);
        state.status_bar = Some(state.t("plain.activity.none_available").to_owned());
        return Task::none();
    }
    let Some(lease_id) = acquire_operation(state, OperationOwner::ActivityFetchRetry) else {
        return Task::none();
    };
    let operation_id = OperationId::new();
    let total = projects.len() + exclusions.len();
    state.activity.latest = crate::state::LatestOpState::Running {
        operation_id: operation_id.clone(),
        label: state.t("plain.activity.retrying_fetch").to_owned(),
        done: exclusions.len(),
        total,
    };
    state.activity.fetch_retry = Some(crate::state::FetchRetryRun {
        operation_id: operation_id.clone(),
        lease_id,
        started_at: chrono::Utc::now(),
        total,
        completed: Vec::new(),
        exclusions,
    });

    use iced::futures::stream;
    let stream = stream::iter(projects)
        .then(move |project| async move { VcsAdapter::fetch(&project).await });
    Task::run(stream, move |result| {
        Message::Background(BackgroundMessage::ActivityFetchRetryProjectCompleted {
            lease_id,
            operation_id: operation_id.clone(),
            result,
        })
    })
}

fn start_activity_smart_pull_review(
    state: &mut AppState,
    source_operation_id: OperationId,
    project_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    invalidate_retry_preparation(state);
    let (projects, exclusions) = split_retry_targets(state, &project_ids);
    if projects.is_empty() {
        mark_activity_retry_unavailable(state, &source_operation_id);
        state.status_bar = Some(state.t("plain.activity.none_available").to_owned());
        return Task::none();
    }
    let Some(workspace_id) = state
        .workspace
        .as_ref()
        .map(|workspace| workspace.id.clone())
    else {
        return Task::none();
    };
    let Some(lease_id) = acquire_operation(state, OperationOwner::ActivitySmartPullPreparation)
    else {
        return Task::none();
    };
    let request_id = state.sync.next_retry_preparation_id();
    let eligible_ids: Vec<_> = projects.iter().map(|project| project.id.clone()).collect();
    state.sync.selected_project_ids = eligible_ids.iter().cloned().collect();
    state.sync.project_selection.clear();
    if let Some(workspace) = &state.workspace {
        for project in &workspace.projects {
            state
                .sync
                .project_selection
                .insert(project.id.clone(), eligible_ids.contains(&project.id));
        }
    }
    state.sync.disposition_overrides.clear();
    state.sync.retry_exclusions = exclusions.clone();
    state.sync.retry_preparation = Some(SmartPullRetryPreparation {
        id: request_id,
        workspace_id: workspace_id.clone(),
        source_operation_id,
        lease_id,
        eligible_ids,
        exclusions,
    });
    state.sync.phase = SyncPhase::RetryPreparing;
    state.active_modal = crate::state::ActiveModal::Pull;

    Task::perform(
        async move {
            let mut statuses = Vec::with_capacity(projects.len());
            for project in projects {
                statuses.push(VcsAdapter::read_project_status(&project).await);
            }
            statuses
        },
        move |statuses| {
            Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
                request_id,
                workspace_id: workspace_id.clone(),
                lease_id,
                statuses,
            })
        },
    )
}

fn mark_activity_retry_unavailable(state: &mut AppState, source_operation_id: &OperationId) {
    if let crate::state::LatestOpState::Completed { log, retry } = &mut state.activity.latest
        && &log.result.operation_id == source_operation_id
    {
        *retry = RetryAvailability::Unavailable(RetryUnavailableReason::NoEligibleTargets);
    }
}

// ---------------------------------------------------------------------------
// RFC-0012 — Palette handler
// ---------------------------------------------------------------------------

fn handle_palette(state: &mut AppState, msg: PaletteMessage) -> Task<Message> {
    match msg {
        PaletteMessage::Opened => {
            state.palette.open_palette();
            crate::state::palette::update_results(state);
            return open_overlay_focus(
                state,
                focus::FocusTarget::text_input(knotra_ui::widget::focus_id::PALETTE_QUERY.clone()),
            );
        }
        PaletteMessage::Closed => state.palette.close(),
        PaletteMessage::QueryChanged(q) => {
            state.palette.query = q;
            state.palette.notice_key = None;
            crate::state::palette::update_results(state);
        }
        PaletteMessage::MoveUp => {
            if state.palette.highlighted > 0 {
                state.palette.highlighted -= 1;
            }
        }
        PaletteMessage::MoveDown => {
            let max = state.palette.results.len().saturating_sub(1);
            if state.palette.highlighted < max {
                state.palette.highlighted += 1;
            }
        }
        PaletteMessage::Confirmed | PaletteMessage::EntryClicked(_) => {
            if let PaletteMessage::EntryClicked(i) = msg {
                state.palette.highlighted = i;
            }
            match crate::state::palette::dispatch_entry(state) {
                crate::state::palette::PaletteDispatch::Dispatched(msg) => {
                    state.palette.close();
                    return Task::done(msg);
                }
                crate::state::palette::PaletteDispatch::Disabled(reason) => {
                    state.palette.notice_key = Some(reason);
                }
                crate::state::palette::PaletteDispatch::Noop => {
                    state.palette.notice_key = Some("palette.disabled.unavailable");
                }
            }
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// RFC-032 — Dashboard display handler
// ---------------------------------------------------------------------------

fn handle_dashboard(state: &mut AppState, msg: DashboardMessage) -> Task<Message> {
    match msg {
        DashboardMessage::GroupingChanged(grouping) => {
            state.config.dashboard_grouping = grouping;
            persist_dashboard_preferences(state);
            state.reconcile_selection_with_display();
        }
        DashboardMessage::SortChanged(sort) => {
            state.config.dashboard_sort = sort;
            persist_dashboard_preferences(state);
        }
        DashboardMessage::TierToggled(tier) => {
            if state.config.dashboard_grouping == DashboardGrouping::Attention {
                match tier {
                    crate::state::dashboard::DashboardTier::NeedsHelp => {}
                    crate::state::dashboard::DashboardTier::InProgress => {
                        state.config.dashboard_in_progress_collapsed =
                            !state.config.dashboard_in_progress_collapsed;
                    }
                    crate::state::dashboard::DashboardTier::AllSet => {
                        state.config.dashboard_all_set_collapsed =
                            !state.config.dashboard_all_set_collapsed;
                    }
                }
                persist_dashboard_preferences(state);
                state.reconcile_selection_with_display();
            }
        }
        DashboardMessage::ErrorDetailsToggled => {
            if matches!(state.load_phase, LoadPhase::Error(_)) {
                state.dashboard_error_details_open = !state.dashboard_error_details_open;
            }
        }
        DashboardMessage::ErrorRetryRequested => {
            if matches!(state.load_phase, LoadPhase::Error(_)) && state.workspace.is_some() {
                state.is_refreshing = false;
                return handle_workspace(state, WorkspaceMessage::RefreshRequested);
            }
        }
    }
    Task::none()
}

fn persist_dashboard_preferences(state: &mut AppState) {
    if let Err(error) = save_config(&state.config, &state.paths) {
        tracing::warn!("failed to persist dashboard preferences: {error}");
        state.status_bar = Some(state.t("dashboard.preference_save_failed").to_owned());
    }
}

// ---------------------------------------------------------------------------
// RFC-0016 — Keyboard / leader-key handler
// ---------------------------------------------------------------------------

fn handle_key_event(state: &mut AppState, msg: KeyboardMessage) -> Task<Message> {
    match msg {
        KeyboardMessage::CheatSheetToggled => {
            state.keyboard.cheat_sheet_open = !state.keyboard.cheat_sheet_open;
        }
        KeyboardMessage::LeaderGPressed => {
            state.keyboard.leader = LeaderKeyState::G;
        }
        KeyboardMessage::LeaderCancelled => {
            state.keyboard.leader = LeaderKeyState::None;
        }
    }
    Task::none()
}
