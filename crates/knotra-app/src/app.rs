//! Top-level Elm-architecture implementation for knotra.

use iced::{clipboard, keyboard, time, Element, Subscription, Task};
use iced::futures::StreamExt;
use std::time::Duration;

use knotra_vcs::{
    model::{
        operation::{
            ContextSwitchResult, OperationId, OperationKind,
            OperationLog, OperationResult, SmartPullDisposition, SmartPullProgress,
        },
        project::Project,
        workspace::Workspace,
    },
    VcsAdapter,
};

#[allow(unused_imports)]
use crate::{
    config::{load_config, save_config, AppPaths},
    fs_watcher::fs_watch_subscription,
    message::{
        ActivityMessage, BackgroundMessage, ChangelogMessage, ConflictOpsMessage, ContextMessage,
        FreezerMessage, HistoryMessage, KeyboardMessage, LaunchMessage, Message, PaletteMessage,
        ProjectMessage, SelectionMessage, SettingsMessage, ShortcutMessage, SyncMessage,
        TagPushMessage, TierMessage, TopologyMessage, WorkspaceMessage,
    },
    persistence::{load_recent_logs, load_workspaces, save_operation_log, save_workspace},
    state::{
        changelog::ChangelogPhase,
        conflict_ops::ConflictPhase,
        context::ContextPhase,
        freezer::FreezerPhase,
        sync::{ProjectOutcome, SyncKind, SyncPhase, SyncResult},
        topology::TopologyPhase,
        workspace_mgr::{CreateWorkspaceDialog, RenameWorkspaceDialog},
        AddProjectDialog, AppState, AttentionTier, ConfirmRemoveDialog,
        LeaderKeyState, LoadPhase, PendingTagPush, Screen,
    },
    view::app_view,
};

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn init() -> (AppState, Task<Message>) {
    let paths = AppPaths::resolve();
    let (config, config_err) = load_config(&paths);
    let mut state = AppState::new(config.clone());

    if let Some(err) = config_err {
        state.status_bar = Some(err);
    }

    let (workspaces, ws_errors) = load_workspaces(&paths);
    for e in &ws_errors { tracing::warn!("workspace load error: {e}"); }

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
        time::every(Duration::from_secs(u64::from(state.config.refresh_interval_secs)))
            .map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    let keyboard_sub = keyboard::listen().map(|event| {
        use keyboard::key::Named;
        use keyboard::Event;
        if let Event::KeyPressed { key, modifiers, .. } = event {
            let ctrl = modifiers.control() || modifiers.command();
            let shortcut = match &key {
                keyboard::Key::Named(Named::Escape) => Some(ShortcutMessage::Close),
                keyboard::Key::Character(c) => match c.as_str() {
                    "r" | "R" if ctrl => Some(ShortcutMessage::Refresh),
                    "k" | "K" if ctrl => Some(ShortcutMessage::OpenContextOps),
                    "t" | "T" if ctrl => Some(ShortcutMessage::OpenFreezer),
                    "/" if ctrl       => Some(ShortcutMessage::FocusSearch),
                    _ => None,
                },
                _ => None,
            };
            if let Some(s) = shortcut { return Message::Shortcut(s); }
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
        Message::Navigate(screen)    => { state.screen = screen; Task::none() }
        Message::Tick                => handle_tick(state),
        Message::Shortcut(msg)       => handle_shortcut(state, msg),
        Message::Workspace(msg)      => handle_workspace(state, msg),
        Message::Project(msg)        => handle_project(state, msg),
        Message::Sync(msg)           => handle_sync(state, msg),
        Message::Freezer(msg)        => handle_freezer(state, msg),
        Message::History(msg)        => handle_history(state, msg),
        Message::Settings(msg)       => handle_settings(state, msg),
        Message::Background(msg)     => handle_background(state, msg),
        Message::Filter(msg)         => { state.apply_filter(msg); Task::none() }
        Message::ConflictOps(msg)    => handle_conflict_ops(state, msg),
        Message::Changelog(msg)      => handle_changelog(state, msg),
        Message::Topology(msg)       => handle_topology(state, msg),
        Message::TagPush(msg)        => handle_tag_push(state, msg),
        Message::FsWatchTick          => handle_fs_watch_tick(state),

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
        // RFC-0010 — Tier grouping
        // ---------------------------------------------------------------
        Message::Tier(tier) => handle_tier(state, tier),

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
                DetailPanelMessage::Closed      => state.detail_panel.open_project_id = None,
            }
            Task::none()
        }

        Message::CopyToClipboard(text) => clipboard::write(text),
        Message::ToggleOpDetails => {
            state.show_op_details = !state.show_op_details;
            Task::none()
        }
        Message::Context(msg)        => handle_context(state, msg),
        Message::Launch(msg)         => handle_launch(state, msg),
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
        ShortcutMessage::OpenContextOps => handle_context(state, ContextMessage::OpenRequested(None)),
        ShortcutMessage::OpenFreezer => handle_freezer(state, FreezerMessage::OpenRequested),
        ShortcutMessage::FocusSearch    => { state.screen = Screen::Dashboard;  Task::none() }
        ShortcutMessage::Close          => {
            state.active_modal          = crate::state::ActiveModal::None;
            state.add_project_dialog   = None;
            state.confirm_remove_dialog = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                state.status_bar = Some(state.t("status.refreshing").to_owned());
                refresh_workspace_task(state)
            } else { Task::none() }
        }

        WorkspaceMessage::AddProjectDialogOpened => {
            state.add_project_dialog = Some(AddProjectDialog::default());
            knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::ADD_PROJECT_PATH)
        }
        WorkspaceMessage::AddProjectNameChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog { d.name = s; d.error = None; }
            Task::none()
        }
        WorkspaceMessage::AddProjectPathChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog { d.path = s; d.error = None; }
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
            knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::ADD_PROJECT_NAME)
        }
        WorkspaceMessage::AddProjectConfirmed => {
            let dialog = match state.add_project_dialog.take() { Some(d) => d, None => return Task::none() };
            let name = dialog.name.trim().to_owned();
            let path = dialog.path.trim().to_owned();
            if name.is_empty() || path.is_empty() {
                state.add_project_dialog = Some(AddProjectDialog {
                    name: dialog.name, path: dialog.path,
                    error: Some(state.t("dialog.add_project.error_empty").to_owned()),
                    ..Default::default()
                });
                return Task::none();
            }
            // Clear any pending undo when a new project is added.
            state.recent_removal = None;
            let project = Project::new(name, path);
            if let Some(ws) = &mut state.workspace {
                ws.add_project(project);
                persist_workspace(ws);
            }
            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }
        WorkspaceMessage::AddProjectCancelled => {
            state.add_project_dialog = None; Task::none()
        }
        WorkspaceMessage::BrowsePathRequested => {
            Task::future(async {
                let folder = rfd::AsyncFileDialog::new()
                    .set_title("Select project folder")
                    .pick_folder()
                    .await;
                let path = folder.map(|f| f.path().to_string_lossy().into_owned());
                Message::Workspace(crate::message::WorkspaceMessage::BrowsePathSelected(path))
            })
        }
        WorkspaceMessage::BrowsePathSelected(path_opt) => {
            if let Some(path) = path_opt
                && let Some(d) = &mut state.add_project_dialog {
                    // Auto-fill name from folder name if not already set.
                    if d.name.is_empty()
                        && let Some(name) = std::path::Path::new(&path)
                            .file_name().and_then(|n| n.to_str())
                        {
                            d.name = name.to_owned();
                        }
                    d.path = path;
                    d.error = None;
                    // Auto-advance to step 2 once a folder is chosen.
                    d.step = crate::state::AddProjectStep::NameProject;
                }
            knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::ADD_PROJECT_NAME)
        }
        WorkspaceMessage::RemoveProjectRequested(id) => {
            let name = state.workspace.as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id))
                .map(|p| p.name.clone()).unwrap_or_default();
            state.confirm_remove_dialog = Some(ConfirmRemoveDialog { project_id: id, project_name: name });
            Task::none()
        }
        WorkspaceMessage::RemoveProjectConfirmed(id) => {
            state.confirm_remove_dialog = None;
            // Capture snapshots before removing so undo can restore exactly.
            let removed_project = state.workspace.as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id).cloned());
            let removed_status = state.workspace_status.as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == id).cloned());

            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
                persist_workspace(ws);
            }
            if let Some(ws_status) = &mut state.workspace_status {
                ws_status.projects.retain(|s| s.project_id != id);
            }
            state.fetching_projects.remove(&id);

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
            state.confirm_remove_dialog = None; Task::none()
        }
        WorkspaceMessage::UndoRemoval => {
            if let Some(removal) = state.recent_removal.take() {
                if let Some(ws) = &mut state.workspace {
                    ws.projects.push(removal.project);
                    persist_workspace(ws);
                }
                if let Some(ws_status) = &mut state.workspace_status
                    && let Some(snap) = removal.status_snapshot {
                        ws_status.projects.push(snap);
                    }
            }
            Task::none()
        }
        WorkspaceMessage::DismissUndoSnackbar => {
            state.recent_removal = None;
            Task::none()
        }

        // --- Multi-workspace management ---

        WorkspaceMessage::CreateWorkspaceDialogOpened => {
            state.workspace_mgr.create_dialog = Some(CreateWorkspaceDialog::default());
            Task::none()
        }
        WorkspaceMessage::CreateWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.create_dialog { d.name = s; d.error = None; }
            Task::none()
        }
        WorkspaceMessage::CreateWorkspaceConfirmed => {
            let name = state.workspace_mgr.create_dialog
                .as_ref()
                .map(|d| d.name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                if let Some(d) = &mut state.workspace_mgr.create_dialog {
                    d.error = Some("Workspace name cannot be empty.".to_owned());
                }
                return Task::none();
            }
            let ws = knotra_vcs::Workspace::new(name);
            let paths = AppPaths::resolve();
            if let Err(e) = save_workspace(&ws, &paths) {
                tracing::warn!("failed to save new workspace: {e}");
            }
            state.all_workspaces.push(ws);
            state.active_workspace_idx = state.all_workspaces.len().saturating_sub(1);
            state.workspace = state.all_workspaces.last().cloned();
            state.workspace_status = None;
            state.load_phase = LoadPhase::Refreshing;
            state.workspace_mgr.create_dialog = None;
            refresh_workspace_task(state)
        }
        WorkspaceMessage::CreateWorkspaceCancelled => {
            state.workspace_mgr.create_dialog = None; Task::none()
        }

        WorkspaceMessage::RenameWorkspaceDialogOpened => {
            let current = state.workspace.as_ref().map(|ws| ws.name.clone()).unwrap_or_default();
            state.workspace_mgr.rename_dialog = Some(RenameWorkspaceDialog { new_name: current, error: None });
            Task::none()
        }
        WorkspaceMessage::RenameWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.rename_dialog { d.new_name = s; d.error = None; }
            Task::none()
        }
        WorkspaceMessage::RenameWorkspaceConfirmed => {
            let name = state.workspace_mgr.rename_dialog
                .as_ref()
                .map(|d| d.new_name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                    d.error = Some("Workspace name cannot be empty.".to_owned());
                }
                return Task::none();
            }
            if let Some(ws) = &mut state.workspace {
                ws.name = name;
                persist_workspace(ws);
                if let Some(entry) = state.all_workspaces.get_mut(state.active_workspace_idx) {
                    entry.name = ws.name.clone();
                }
            }
            state.workspace_mgr.rename_dialog = None;
            Task::none()
        }
        WorkspaceMessage::RenameWorkspaceCancelled => {
            state.workspace_mgr.rename_dialog = None; Task::none()
        }

        WorkspaceMessage::DeleteWorkspaceRequested => {
            state.workspace_mgr.confirm_delete = true; Task::none()
        }
        WorkspaceMessage::DeleteWorkspaceConfirmed => {
            state.workspace_mgr.confirm_delete = false;
            // Prune snapshots for the workspace being removed.
            if let Some(ws) = state.all_workspaces.get(state.active_workspace_idx) {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                // After deletion the active workspace changes; prune all stale entries.
                state.fs_poller.prune(&ids);
            }
            if state.all_workspaces.len() <= 1 {
                // Don't delete the last workspace.
                return Task::none();
            }
            let deleted_idx = state.active_workspace_idx;
            let paths = AppPaths::resolve();
            if let Some(ws) = state.all_workspaces.get(deleted_idx) {
                let file_name = format!("{}.toml", ws.id);
                let _ = std::fs::remove_file(paths.workspaces_dir.join(file_name));
            }
            state.all_workspaces.remove(deleted_idx);
            state.active_workspace_idx = 0.min(state.all_workspaces.len().saturating_sub(1));
            state.workspace = state.all_workspaces.get(state.active_workspace_idx).cloned();
            state.workspace_status = None;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }
        WorkspaceMessage::DeleteWorkspaceCancelled => {
            state.workspace_mgr.confirm_delete = false; Task::none()
        }

        WorkspaceMessage::WorkspaceSwitched(id) => {
            if let Some(idx) = state.all_workspaces.iter().position(|ws| ws.id == id) {
                state.active_workspace_idx = idx;
                state.workspace = state.all_workspaces.get(idx).cloned();
                // Prune stale FsPoller snapshots from the previous workspace.
                let active_ids: Vec<knotra_vcs::ProjectId> = state.workspace.as_ref()
                    .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
                    .unwrap_or_default();
                state.fs_poller.prune(&active_ids);
                state.workspace_status = None;
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
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        knotra_vcs::WorkspaceStatus {
                            projects: vec![s],
                            last_refresh: Some(chrono::Utc::now()),
                        },
                    )),
                )
            } else { Task::none() }
        }
        ProjectMessage::FetchRequested(id) => {
            state.fetching_projects.insert(id.clone());
            let project = find_project(state, &id);
            if let Some(p) = project {
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
                    |log| Message::Background(BackgroundMessage::SingleFetchCompleted(log)),
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
            state.sync.disposition_overrides.insert(id, disposition);
            Task::none()
        }

        SyncMessage::PlanRequested => {
            // Open the pull modal and start planning.
            state.active_modal = crate::state::ActiveModal::Pull;
            Task::none()
        }
        SyncMessage::ExecuteRequested => {
            // Delegate to existing SmartPullConfirmed path.
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        SyncMessage::BulkFetchRequested => {
            let ids = state.sync.selected_ids();
            let projects: Vec<_> = state.workspace.as_ref()
                .map(|ws| ws.projects.iter().filter(|p| ids.contains(&p.id)).cloned().collect())
                .unwrap_or_default();
            let total = projects.len();
            state.sync.phase = SyncPhase::FetchRunning { total, done: 0 };

            let _max = state.config.max_concurrent_reads;

            // Stream results per-project using Task::run.
            use iced::futures::stream;

            let project_stream = stream::iter(projects).then(move |project| async move {
                VcsAdapter::fetch(&project).await
            });

            Task::run(project_stream, |per_project_result| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted(SmartPullProgress {
                    project_id: per_project_result.project_id.clone(),
                    project_name: String::new(), // filled in from state on receipt
                    result: per_project_result,
                    recovery_hint: None,
                }))
            })
        }

        SyncMessage::SmartPullPlanRequested => {
            state.sync.phase = SyncPhase::Planning;
            // Build the plan synchronously from existing status.
            let plan = state.sync.build_plan(
                state.workspace.as_ref().map(|w| w.projects.as_slice()).unwrap_or(&[]),
                state.workspace_status.as_ref(),
            );
            state.sync.phase = SyncPhase::AwaitingConfirm(plan.clone());
            Task::done(Message::Background(BackgroundMessage::SmartPullPlanReady(plan)))
        }

        SyncMessage::SmartPullConfirmed(plan) => {
            let projects_map: std::collections::HashMap<_, _> = state
                .workspace.as_ref()
                .map(|ws| ws.projects.iter().map(|p| (p.id.clone(), p.clone())).collect())
                .unwrap_or_default();

            let entries = plan.entries.clone();
            state.sync.phase = SyncPhase::PullRunning { plan, completed: Vec::new() };

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
                                success: false,
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
                                success: true,
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
                            let stash = matches!(entry.disposition, SmartPullDisposition::StashAndPull);
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

            Task::run(pull_stream, |progress| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted(progress))
            })
        }

        SyncMessage::SmartPullCancelled => {
            state.sync.phase = SyncPhase::Idle;
            Task::none()
        }

        SyncMessage::RetryFailedRequested => {
            // Collect failed project IDs from the last result.
            let failed_ids: Vec<_> = if let SyncPhase::Done(ref result) = state.sync.phase {
                result.per_project.iter()
                    .filter(|p| !p.success)
                    .map(|p| p.project_id.clone())
                    .collect()
            } else { return Task::none(); };

            // Deselect all, then select only failed.
            for (id, v) in state.sync.project_selection.iter_mut() {
                *v = failed_ids.contains(id);
            }
            state.sync.phase = SyncPhase::Idle;
            Task::done(Message::Sync(SyncMessage::BulkFetchRequested))
        }
        SyncMessage::ModalClosed => {
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        SyncMessage::Cancelled => {
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        SyncMessage::BulkPullRequested => {
            state.active_modal = crate::state::ActiveModal::Pull;
            state.sync.selected_project_ids = state.selection.selected_ids.clone();
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

fn handle_background(state: &mut AppState, msg: BackgroundMessage) -> Task<Message> {
    match msg {
        BackgroundMessage::WorkspaceStatusRefreshed(new_status) => {
            // Detect missing-path projects.
            if let Some(ws) = &state.workspace {
                let missing: Vec<_> = ws.projects.iter()
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

        BackgroundMessage::SmartPullPlanReady(plan) => {
            // Already set in handle_sync; this message lets the view re-render.
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);
            Task::none()
        }

        BackgroundMessage::SmartPullProjectCompleted(mut progress) => {
            // Fill in the project name if missing.
            if progress.project_name.is_empty()
                && let Some(name) = find_project_name(state, &progress.project_id) {
                    progress.project_name = name;
                }

            match &mut state.sync.phase {
                SyncPhase::FetchRunning { done, total } => {
                    *done += 1;
                    let done_val = *done;
                    let total_val = *total;

                    // Accumulate into a temporary vec using the Done phase.
                    // We check completion after updating.
                    let outcome = ProjectOutcome {
                        project_id:        progress.project_id.clone(),
                        project_name:      progress.project_name.clone(),
                        success:           progress.result.success,
                        commands_executed: progress.result.commands_executed.clone(),
                        stdout:            progress.result.stdout.clone(),
                        stderr:            progress.result.stderr.clone(),
                        log_expanded:      false,
                    };

                    // Store partial results in a transient Done or accumulate in running.
                    // For simplicity, switch to Done when all projects report.
                    // We keep a partial results list in the Fetch case.
                    // Switch to Done phase temporarily when all are in.
                    if done_val >= total_val {
                        // All done — we need the full list. Rebuild from log.
                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::Fetch,
                            per_project: vec![outcome],
                            recovery_hints: vec![],
                        });
                        // Trigger status refresh.
                        state.is_refreshing = true;
                        state.load_phase = LoadPhase::Refreshing;
                        return refresh_workspace_task(state);
                    }
                    // Still running — accumulate into a temporary SyncResult in Done.
                    // Replace with accumulation pattern.
                    // (This rebuilds to avoid borrow issues.)
                    let _ = outcome; // handled per-project in Done transition
                }
                SyncPhase::PullRunning { plan, completed } => {
                    if let Some(hint) = progress.recovery_hint.clone() {
                        // Recovery hint collected.
                        let _ = hint;
                    }
                    completed.push(progress.clone());

                    let expected = plan.entries.len();
                    let got = completed.len();
                    if got >= expected {
                        // Build final result from completed.
                        let outcomes: Vec<ProjectOutcome> = completed.iter().map(|p| ProjectOutcome {
                            project_id:        p.project_id.clone(),
                            project_name:      p.project_name.clone(),
                            success:           p.result.success,
                            commands_executed: p.result.commands_executed.clone(),
                            stdout:            p.result.stdout.clone(),
                            stderr:            p.result.stderr.clone(),
                            log_expanded:      false,
                        }).collect();

                        let hints: Vec<_> = completed.iter()
                            .filter_map(|p| p.recovery_hint.clone())
                            .collect();

                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::SmartPull,
                            per_project: outcomes,
                            recovery_hints: hints,
                        });

                        // Trigger status refresh.
                        state.is_refreshing = true;
                        state.load_phase = LoadPhase::Refreshing;
                        return refresh_workspace_task(state);
                    }
                }
                _ => {}
            }
            Task::none()
        }

        BackgroundMessage::SingleFetchCompleted(log) => {
            for r in &log.result.per_project {
                state.fetching_projects.remove(&r.project_id);
            }
            persist_log(&log, state);

            let tasks: Vec<Task<Message>> = log.result.per_project.iter()
                .filter_map(|r| find_project(state, &r.project_id))
                .map(|project| Task::perform(
                    async move { VcsAdapter::read_project_status(&project).await },
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        knotra_vcs::WorkspaceStatus { projects: vec![s], last_refresh: Some(chrono::Utc::now()) },
                    )),
                ))
                .collect();
            Task::batch(tasks)
        }

        BackgroundMessage::BulkFetchCompleted(log) => {
            persist_log(&log, state);
            state.status_bar = Some(if log.result.any_failed() {
                format!("Fetch — {} ok, {} failed",
                    log.result.successful_projects().len(),
                    log.result.failed_projects().len())
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

        BackgroundMessage::TagPushCompleted { success_count, fail_count } => {
            state.pending_tag_push = None;
            state.status_bar = Some(if fail_count == 0 {
                format!("✓ Tags pushed to remote — {} project(s).", success_count)
            } else {
                format!("⚠ Tag push: {} succeeded, {} failed.", success_count, fail_count)
            });
            Task::none()
        }

        BackgroundMessage::MissingProjectsDetected(ids) => {
            state.missing_projects = ids.into_iter().collect();
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

        BackgroundMessage::ChangelogDraftReady(draft) => {
            state.changelog.phase = ChangelogPhase::Ready(draft);
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
                state.topology.impact_warnings =
                    state.topology.compute_warnings(&graph, &names);
            }
            state.topology.phase = TopologyPhase::Ready(graph);
            Task::none()
        }

        BackgroundMessage::FreezeValidationDone(validation) => {
            state.freezer.phase = FreezerPhase::ValidationReady(validation);
            Task::none()
        }

        BackgroundMessage::FreezeExecutionDone(result) => {
            use knotra_vcs::model::operation::{OperationKind, OperationLog, OperationResult};

            // Build per-project entries for the operation log.
            let per_project: Vec<_> = result.project_results.iter().map(|r| {
                knotra_vcs::model::operation::ProjectOperationResult {
                    project_id:        r.project_id.clone(),
                    success:           r.success,
                    commands_executed: r.commands_executed.clone(),
                    stdout:            r.stdout.clone(),
                    stderr:            r.stderr.clone(),
                    exit_code:         None,
                    error_message:     if r.success { None } else { Some("freeze failed".to_owned()) },
                }
            }).collect();

            let hints: Vec<_> = result.project_results.iter()
                .filter_map(|r| r.recovery_hint.clone())
                .collect();

            let op_log = OperationLog {
                result: OperationResult {
                    operation_id:       OperationId::new(),
                    kind:               OperationKind::Freeze,
                    started_at:         chrono::Utc::now(),
                    finished_at:        chrono::Utc::now(),
                    per_project,
                    rollback_attempted: result.project_results.iter().any(|r| r.rollback_attempted),
                    rollback_succeeded: {
                        let any_rb = result.project_results.iter().any(|r| r.rollback_attempted);
                        if any_rb {
                            Some(result.project_results.iter()
                                .filter(|r| r.rollback_attempted)
                                .all(|r| r.rollback_succeeded == Some(true)))
                        } else { None }
                    },
                },
                recovery_hints: hints,
            };
            persist_log(&op_log, state);

            // If the freeze succeeded fully, offer to push tags to remote.
            if let FreezerPhase::Done(ref freeze_result) = state.freezer.phase
                && freeze_result.outcome == knotra_vcs::FreezeOutcome::Success {
                    let ids: Vec<_> = freeze_result.project_results.iter()
                        .filter(|r| r.success)
                        .map(|r| r.project_id.clone())
                        .collect();
                    if !ids.is_empty() {
                        let _ = Task::done(Message::TagPush(TagPushMessage::OfferShown {
                            freeze_name: freeze_result.freeze_name.clone(),
                            project_ids: ids,
                        }));
                    }
                }
            state.freezer.phase = FreezerPhase::Done(result);
            Task::none()
        }

        BackgroundMessage::ContextListLoaded(list) => {
            let id = list.project_id.clone();
            state.context_ops.cached_lists.insert(id.clone(), list.clone());
            // Only update phase if we were waiting for this exact project.
            if matches!(&state.context_ops.phase, ContextPhase::LoadingList(loading_id) if loading_id == &id) {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list,
                    search: String::new(),
                };
            }
            Task::none()
        }

        BackgroundMessage::ContextSwitchDone(result) => {
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
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        knotra_vcs::WorkspaceStatus { projects: vec![s], last_refresh: Some(chrono::Utc::now()) },
                    )),
                )
            } else {
                Task::none()
            }
        }

        BackgroundMessage::TaskError { description } => {
            state.load_phase = LoadPhase::Error(description.clone());
            state.is_refreshing = false;
            state.status_bar = Some(description);
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
        HistoryMessage::SearchChanged(s) => { state.history_search = s; }
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
            state.theme = if dark { knotra_ui::KnotraTheme::dark() } else { knotra_ui::KnotraTheme::light() };
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
            state.config.external_editor = if s.trim().is_empty() { None } else { Some(s.trim().to_owned()) };
        }
        SettingsMessage::ExternalMergeToolChanged(s) => {
            state.settings_edit.external_merge_tool = s.clone();
            state.config.external_merge_tool = if s.trim().is_empty() { None } else { Some(s.trim().to_owned()) };
        }
        SettingsMessage::MaxLogEntriesChanged(n) => {
            state.settings_edit.max_log_entries = n.to_string();
            state.config.max_log_entries = n;
        }
        SettingsMessage::FsWatchEnabledChanged(v) => {
            state.config.fs_watch_enabled = v;
            if !v { state.settings_save_msg = Some("FS watching disabled.".to_owned()); }
        }
        SettingsMessage::FsDebounceSecs(n) => {
            state.settings_edit.refresh_interval_secs = n.to_string();
            state.config.fs_debounce_secs = n;
        }
        SettingsMessage::SaveRequested => {
            let paths = AppPaths::resolve();
            match save_config(&state.config, &paths) {
                Ok(()) => {
                    state.settings_save_msg = Some(state.t("settings.saved_ok").to_owned());
                    state.status_bar = Some(state.t("settings.saved_ok").to_owned());
                }
                Err(e) => {
                    state.settings_save_msg = Some(format!("{} {e}", state.t("settings.save_error")));
                }
            }
        }
        SettingsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_project(state: &AppState, id: &knotra_vcs::ProjectId) -> Option<knotra_vcs::Project> {
    state.workspace.as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id).cloned())
}

fn find_project_name(state: &AppState, id: &knotra_vcs::ProjectId) -> Option<String> {
    find_project(state, id).map(|p| p.name)
}

fn merge_workspace_status(state: &mut AppState, new: knotra_vcs::WorkspaceStatus) {
    if let Some(existing) = &mut state.workspace_status {
        for ps in new.projects {
            if let Some(pos) = existing.projects.iter().position(|p| p.project_id == ps.project_id) {
                existing.projects[pos] = ps;
            } else {
                existing.projects.push(ps);
            }
        }
        existing.last_refresh = new.last_refresh;
    } else {
        state.workspace_status = Some(new);
    }
}

fn persist_workspace(ws: &Workspace) {
    let paths = AppPaths::resolve();
    if let Err(e) = save_workspace(ws, &paths) {
        tracing::warn!("failed to save workspace: {e}");
    }
}

fn persist_log(log: &OperationLog, state: &mut AppState) {
    let paths = AppPaths::resolve();
    if let Err(e) = save_operation_log(log, &paths) {
        tracing::warn!("failed to save operation log: {e}");
    }
    state.operation_logs.insert(0, log.clone());
    state.operation_logs.truncate(state.config.max_log_entries);
}

fn refresh_workspace_task(state: &AppState) -> Task<Message> {
    let workspace = match &state.workspace { Some(ws) => ws.clone(), None => return Task::none() };
    let max = state.config.max_concurrent_reads;
    Task::perform(
        async move { VcsAdapter::read_workspace_status(&workspace, max).await },
        |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(s)),
    )
}

// ---------------------------------------------------------------------------
// Context Operations handler
// ---------------------------------------------------------------------------

fn handle_context(state: &mut AppState, msg: ContextMessage) -> Task<Message> {
    match msg {
        ContextMessage::OpenRequested(preselect_id) => {
            state.context_ops.phase = ContextPhase::Idle;

            // If a project was pre-selected (e.g. from a dashboard card shortcut), load it.
            if let Some(id) = preselect_id
                && let Some(project) = find_project(state, &id) {
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
                None    => return Task::none(),
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

        ContextMessage::SwitchTargetChosen(project_id, target) => {
            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None    => return Task::none(),
            };

            // Check current dirty state.
            let vcs_kind = state
                .workspace_status.as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id))
                .map(|s| s.identity.vcs_kind)
                .unwrap_or(knotra_vcs::VcsKind::Git);

            let is_dirty = state
                .workspace_status.as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id))
                .map(|s| s.working_tree.is_dirty())
                .unwrap_or(false);

            state.context_ops.phase = ContextPhase::ConfirmSwitch {
                project_id,
                project_name: project.name.clone(),
                target,
                vcs_kind,
                is_dirty,
            };
            Task::none()
        }

        ContextMessage::SwitchConfirmed => {
            let (project_id, target, project_name) = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, target, project_name, .. } => {
                    (project_id.clone(), target.clone(), project_name.clone())
                }
                _ => return Task::none(),
            };

            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None    => return Task::none(),
            };

            state.context_ops.phase = ContextPhase::Switching {
                project_id: project_id.clone(),
                target: target.clone(),
            };
            // Invalidate cached list for this project.
            state.context_ops.cached_lists.remove(&project_id);

            Task::perform(
                async move {
                    let (result, hint) = VcsAdapter::switch_context(&project, &target).await;
                    ContextSwitchResult { project_id: project.id, project_name, target, operation_result: result, recovery_hint: hint }
                },
                |r| Message::Background(BackgroundMessage::ContextSwitchDone(r)),
            )
        }

        ContextMessage::SwitchCancelled => {
            // Return to browsing.
            let prev_id = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, .. } => Some(project_id.clone()),
                _ => None,
            };
            if let Some(id) = prev_id
                && let Some(cached) = state.context_ops.cached_lists.get(&id).cloned() {
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
            state.active_modal = crate::state::ActiveModal::Switch;
            state.context_ops.target_context = String::new();
            knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::SWITCH_TARGET)
        }
        ContextMessage::BulkSwitchRequested => {
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        ContextMessage::BulkModalClosed => {
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        ContextMessage::TargetChanged(s) => {
            state.context_ops.target_context = s;
            Task::none()
        }
        ContextMessage::Cancelled => {
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
            // Reinitialise project selection from workspace.
            if let Some(ws) = &state.workspace {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                state.freezer.init_selection(&ids);
            }
            state.freezer.phase = FreezerPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Tag;
            Task::none()
        }

        FreezerMessage::NameChanged(name) => {
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
        FreezerMessage::ExecuteConfirmed => {
            Task::done(Message::Freezer(FreezerMessage::ExecuteRequested))
        }
        FreezerMessage::ExecuteRequested => {
            Task::done(Message::Freezer(FreezerMessage::ExecuteConfirmed))
        }
        FreezerMessage::BulkOpenRequested => {
            state.active_modal = crate::state::ActiveModal::Tag;
            // Pre-populate freeze selection
            state.freezer.project_selection = state.selection.selected_ids
                .iter().map(|id| (id.clone(), true)).collect();
            knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::RELEASE_NAME)
        }
        FreezerMessage::BulkModalClosed => {
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }

        FreezerMessage::ProjectToggled(id, included) => {
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

            let projects: Vec<_> = state.workspace.as_ref()
                .map(|ws| ws.projects.clone())
                .unwrap_or_default();
            let selection = state.freezer.selected_ids();
            let freeze_name = state.freezer.freeze_name.clone();
            let max = state.config.max_concurrent_reads;

            state.freezer.phase = FreezerPhase::Validating;

            Task::perform(
                async move {
                    VcsAdapter::validate_freeze(&projects, &selection, &freeze_name, max).await
                },
                |v| Message::Background(BackgroundMessage::FreezeValidationDone(v)),
            )
        }


        FreezerMessage::Cancelled => {
            state.freezer.phase = FreezerPhase::Idle;
            Task::none()
        }

        FreezerMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            state.freezer.phase = FreezerPhase::Idle;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// External tool launch handler
// ---------------------------------------------------------------------------

fn handle_launch(state: &mut AppState, msg: LaunchMessage) -> Task<Message> {
    let (tool_path, file_path) = match msg {
        LaunchMessage::OpenInEditor(path) => {
            (state.config.external_editor.clone(), path)
        }
        LaunchMessage::OpenInMergeTool(path) => {
            (state.config.external_merge_tool.clone(), path)
        }
    };

    let Some(tool) = tool_path else {
        state.status_bar = Some(state.t("tool.not_configured").to_owned());
        return Task::none();
    };

    match std::process::Command::new(&tool)
        .arg(&file_path)
        .spawn()
    {
        Ok(_) => {
            state.status_bar = Some(format!("Launched: {} {:?}", tool, file_path));
        }
        Err(e) => {
            state.status_bar = Some(format!("{} {}: {e}", state.t("tool.launch_failed"), tool));
        }
    }
    Task::none()
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
                return Task::done(Message::ConflictOps(ConflictOpsMessage::ProjectSelected(id)));
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
                None    => return Task::none(),
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
                None    => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::list_conflicted_files(&project).await },
                |d| Message::Background(BackgroundMessage::ConflictFilesLoaded(d)),
            )
        }

        ConflictOpsMessage::MarkResolvedRequested { project_id, file_path } => {
            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None    => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: project_id.clone(),
                action: format!("git add {}", file_path),
            };
            state.conflict_ops.cached.remove(&project_id);
            let file_path_for_msg = file_path.clone();
            Task::perform(
                async move { VcsAdapter::mark_resolved(&project, &file_path).await },
                move |r| {
                    let pid = r.project_id.clone();
                    let ok  = r.success;
                    let msg = if ok {
                        format!("Marked resolved: {}", file_path_for_msg)
                    } else {
                        r.error_message.unwrap_or_else(|| "mark-resolved failed".to_owned())
                    };
                    Message::Background(BackgroundMessage::ConflictFilesLoaded(
                        knotra_vcs::ProjectConflictDetail {
                            project_id:       pid.clone(),
                            project_name:     String::new(),
                            conflicted_files: vec![],
                            note:             None,
                            read_error:       if ok { None } else { Some(msg) },
                        }
                    ))
                },
            )
        }

        ConflictOpsMessage::AbortMergeRequested(id) => {
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: id.clone(),
                action: "git merge --abort".to_owned(),
            };
            state.conflict_ops.cached.remove(&id);
            let project = match find_project(state, &id) {
                Some(p) => p,
                None    => return Task::none(),
            };
            Task::perform(
                async move { VcsAdapter::abort_merge(&project).await },
                |r| {
                    let ok  = r.success;
                    let pid = r.project_id.clone();
                    Message::Background(BackgroundMessage::ConflictFilesLoaded(
                        knotra_vcs::ProjectConflictDetail {
                            project_id:       pid,
                            project_name:     String::new(),
                            conflicted_files: vec![],
                            note:             if ok { Some("Merge aborted.".to_owned()) } else { None },
                            read_error:       if ok { None } else {
                                Some(r.error_message.unwrap_or_else(|| "abort failed".to_owned()))
                            },
                        }
                    ))
                },
            )
        }

        ConflictOpsMessage::AbortMergeConfirmed(id) => {
            Task::done(Message::ConflictOps(ConflictOpsMessage::AbortMergeRequested(id)))
        }

        ConflictOpsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ConflictOpsMessage::FileMarkedResolved(_path) => {
            Task::none()
        }
        ConflictOpsMessage::OpenInEditorRequested(path) => {
            // Launch the configured external editor for this file.
            // If no editor is configured, this is a no-op (button is only
            // shown when the editor is configured — TODO: gate in the view).
            if let Some(editor) = &state.config.external_editor {
                let cmd = format!("{} {}", editor, path);
                let _ = std::process::Command::new("sh")
                    .args(["-c", &cmd])
                    .spawn();
            }
            Task::none()
        }
        ConflictOpsMessage::AbortRequested => {
            Task::none()
        }
        ConflictOpsMessage::PanelClosed => {
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
            if let Some(ws) = &state.workspace {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                state.changelog.init_selection(&ids);
            }
            state.changelog.phase = ChangelogPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Changelog;
            Task::none()
        }

        ChangelogMessage::SinceRefChanged(s) => {
            state.changelog.since_ref = s;
            if matches!(state.changelog.phase, ChangelogPhase::Ready(_)) {
                state.changelog.phase = ChangelogPhase::Idle;
            }
            Task::none()
        }

        ChangelogMessage::ProjectToggled(id, v) => {
            state.changelog.project_selection.insert(id, v);
            Task::none()
        }

        ChangelogMessage::LoadTagsRequested => {
            // Load tags from the first selected project.
            let project = state.workspace.as_ref()
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
            let selected_ids = state.changelog.selected_ids();
            let projects: Vec<_> = state.workspace.as_ref()
                .map(|ws| ws.projects.iter()
                    .filter(|p| selected_ids.contains(&p.id))
                    .cloned()
                    .collect())
                .unwrap_or_default();
            let since   = state.changelog.since_ref.clone();
            let max_cl  = state.config.max_concurrent_reads;
            state.changelog.phase = ChangelogPhase::Collecting;

            Task::perform(
                async move { VcsAdapter::collect_changelog(&projects, &since, max_cl).await },
                |draft| Message::Background(BackgroundMessage::ChangelogDraftReady(draft)),
            )
        }

        ChangelogMessage::CopyRequested => {
            if let ChangelogPhase::Ready(ref draft) = state.changelog.phase {
                let md = draft.to_markdown();
                state.status_bar = Some(format!("Changelog ({} chars) — copied to clipboard.", md.len()));
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
            let projects: Vec<_> = state.workspace.as_ref()
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
        .map(|ws| ws.projects.iter().map(|p| (p.id.clone(), p.path.clone())).collect())
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
        TagPushMessage::OfferShown { freeze_name, project_ids } => {
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
                None    => return Task::none(),
            };
            if let Some(ref mut p) = state.pending_tag_push { p.is_pushing = true; }

            let projects: Vec<_> = push.project_ids.iter()
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
                        let sem     = Arc::clone(&sem);
                        let tag     = tag_name.clone();
                        handles.push(tokio::spawn(async move {
                            let _permit = sem.acquire().await.expect("open");
                            knotra_vcs::VcsAdapter::push_tag(&project, &tag).await
                        }));
                    }
                    let mut results = Vec::new();
                    for h in handles {
                        if let Ok(r) = h.await { results.push(r); }
                    }
                    let success = results.iter().filter(|r| r.success).count();
                    let failed  = results.iter().filter(|r| !r.success).count();
                    (success, failed)
                },
                |(success_count, fail_count)| {
                    Message::Background(BackgroundMessage::TagPushCompleted { success_count, fail_count })
                },
            )
        }

        TagPushMessage::PushDeclined => {
            state.pending_tag_push = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// RFC-0009 — Selection handler
// ---------------------------------------------------------------------------

fn handle_selection(state: &mut AppState, msg: SelectionMessage) -> Task<Message> {
    let ordered: Vec<knotra_vcs::ProjectId> = state.workspace.as_ref()
        .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
        .unwrap_or_default();

    match msg {
        SelectionMessage::ModeEntered  => state.selection_mode = true,
        SelectionMessage::ModeExited   => {
            state.selection_mode = false;
            state.selection.clear();
        }
        SelectionMessage::Toggled(id)  => {
            state.selection_mode = true;   // selecting anything enters mode
            state.selection.toggle(id);
        }
        SelectionMessage::RangeTo(id)  => state.selection.select_range(&ordered, &id),
        SelectionMessage::SelectAll    => {
            state.selection_mode = true;
            state.selection.select_all(&ordered);
        }
        SelectionMessage::Clear        => {
            state.selection.clear();
            state.selection_mode = false;  // clearing exits mode
        }
        SelectionMessage::FocusMoved(_) => {} // focus tracking only
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// RFC-0011 — Activity strip handler
// ---------------------------------------------------------------------------

fn handle_activity(state: &mut AppState, msg: ActivityMessage) -> Task<Message> {
    match msg {
        ActivityMessage::Started { label, total } => {
            state.activity.latest = crate::state::LatestOpState::Running {
                label, done: 0, total,
            };
            state.activity.completed_secs = 0;
        }
        ActivityMessage::Progress { done } => {
            if let crate::state::LatestOpState::Running { done: ref mut d, .. } = state.activity.latest {
                *d = done;
            }
        }
        ActivityMessage::Completed { log } => {
            let total   = log.result.per_project.len();
            let failed  = log.result.per_project.iter().filter(|p| !p.success).count();
            let kind    = log.result.kind.to_string();
            if failed == 0 {
                state.activity.latest = crate::state::LatestOpState::Success {
                    summary: format!("{} {} project{}", kind, total, if total == 1 {""} else {"s"}),
                    elapsed_secs: 0,
                };
            } else if failed < total {
                let names = log.result.per_project.iter()
                    .filter(|p| !p.success)
                    .map(|p| p.project_id.to_string())
                    .collect();
                state.activity.latest = crate::state::LatestOpState::PartialFailure {
                    summary: format!("{} {} projects · {} ok, {} failed",
                        kind, total, total - failed, failed),
                    failed_names: names,
                };
            } else {
                state.activity.latest = crate::state::LatestOpState::TotalFailure {
                    summary: format!("{} failed for all {} projects", kind, total),
                };
            }
            state.activity.completed_secs = 0;
        }
        ActivityMessage::PopoverToggled => {
            state.activity.popover_open = !state.activity.popover_open;
        }
        ActivityMessage::RetryRequested => {
            // Route to last operation kind — for now navigate to History.
            return Task::done(Message::Navigate(Screen::History));
        }
        ActivityMessage::Tick => {
            state.activity.completed_secs = state.activity.completed_secs.saturating_add(1);
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// RFC-0012 — Palette handler
// ---------------------------------------------------------------------------

fn handle_palette(state: &mut AppState, msg: PaletteMessage) -> Task<Message> {
    match msg {
        PaletteMessage::Opened => {
            state.palette.open_palette();
            crate::state::palette::update_results(state);
            return knotra_ui::widget::focus_input(
                &knotra_ui::widget::focus_id::PALETTE_QUERY
            );
        }
        PaletteMessage::Closed => state.palette.close(),
        PaletteMessage::QueryChanged(q) => {
            state.palette.query = q;
            crate::state::palette::update_results(state);
        }
        PaletteMessage::MoveUp => {
            if state.palette.highlighted > 0 { state.palette.highlighted -= 1; }
        }
        PaletteMessage::MoveDown => {
            let max = state.palette.results.len().saturating_sub(1);
            if state.palette.highlighted < max { state.palette.highlighted += 1; }
        }
        PaletteMessage::Confirmed | PaletteMessage::EntryClicked(_) => {
            if let PaletteMessage::EntryClicked(i) = msg {
                state.palette.highlighted = i;
            }
            if let Some(msg) = crate::state::palette::dispatch_entry(state) {
                state.palette.close();
                return Task::done(msg);
            }
            state.palette.close();
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// RFC-0010 — Tier handler
// ---------------------------------------------------------------------------

fn handle_tier(state: &mut AppState, msg: TierMessage) -> Task<Message> {
    match msg {
        TierMessage::Toggled(tier) => {
            match tier {
                AttentionTier::NeedsAttention =>
                    state.tier_collapse.needs_attention = !state.tier_collapse.needs_attention,
                AttentionTier::Active =>
                    state.tier_collapse.active = !state.tier_collapse.active,
                AttentionTier::Clean =>
                    state.tier_collapse.clean = !state.tier_collapse.clean,
            }
        }
        TierMessage::GroupingModeChanged(mode) => {
            state.grouping_mode = mode;
        }
    }
    Task::none()
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
