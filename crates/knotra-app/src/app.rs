//! Top-level Elm-architecture implementation for knotra.

mod activity;
mod background;
mod changelog;
mod conflict_ops;
mod context;
mod focus_ops;
mod freezer;
mod misc;
mod shared;
mod sync;
mod workspace;

use activity::handle_activity;
use background::handle_background;
use changelog::handle_changelog;
use conflict_ops::handle_conflict_ops;
use context::handle_context;
use misc::{
    handle_dashboard, handle_history, handle_launch, handle_palette, handle_project,
    handle_selection, handle_settings, handle_tag_push, handle_topology,
};
// `resolve_project_file_path` moved to `conflict_ops` (its only in-crate,
// non-test caller); re-exported here, gated the same as its only consumer
// (`tests.rs`, `#[cfg(test)]` in main.rs), so `crate::app::resolve_project_file_path`
// keeps resolving for the three call sites there (R3) without an unused-import
// warning in a normal build, where nothing else uses this path.
#[cfg(test)]
pub(crate) use conflict_ops::resolve_project_file_path;
use focus_ops::{
    activate_focused, advance_focus, close_overlay_focus, close_topmost_layer,
    current_target_is_text_input, focus_search, workspace_dialog_open,
};
use freezer::handle_freezer;
use shared::{find_project, refresh_workspace_task};
use sync::handle_sync;
use workspace::handle_workspace;

use iced::{Element, Subscription, Task, clipboard, keyboard, time};
use std::time::Duration;

use knotra_vcs::model::workspace::Workspace;

use crate::{
    config::{AppPaths, load_config},
    fs_watcher::fs_watch_subscription,
    message::{
        BackgroundMessage, ContextMessage, FreezerMessage, KeyboardMessage, Message,
        ShortcutMessage,
    },
    persistence::{load_recent_logs, load_workspaces},
    state::{AppState, LeaderKeyState, LoadPhase, focus},
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
