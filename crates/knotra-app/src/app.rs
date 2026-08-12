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

// `resolve_project_file_path` moved to `conflict_ops` (its only in-crate,
// non-test caller); re-exported here, gated the same as its only consumer
// (`tests.rs`, `#[cfg(test)]` in main.rs), so `crate::app::resolve_project_file_path`
// keeps resolving for the three call sites there (R3) without an unused-import
// warning in a normal build, where nothing else uses this path.
#[cfg(test)]
pub(crate) use conflict_ops::resolve_project_file_path;

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
    state::{AppState, LoadPhase, focus},
    view::app_view,
    view::dashboard::WidthMode,
};

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn init() -> (AppState, Task<Message>) {
    let (paths, paths_warning) = AppPaths::resolve();
    let (config, config_err) = load_config(&paths);
    let mut state = AppState::new_with_paths(config.clone(), paths.clone());

    // Path resolution and config parsing are independent failure modes; if
    // both produced a warning, concatenate rather than let one silently
    // overwrite the other (Handoff 033 §3) — a user whose config directory
    // could not be resolved *and* whose config.toml failed to parse needs to
    // see both, not just whichever was assigned to `status_bar` last.
    let startup_warning = match (paths_warning, config_err) {
        (Some(p), Some(c)) => Some(format!("{p}\n\n{c}")),
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };
    if let Some(warning) = startup_warning {
        state.status_bar = Some(warning);
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
    let loaded_logs = load_recent_logs(&paths, config.max_log_entries);
    state.operation_logs = loaded_logs.logs;
    state.history_unreadable_count = loaded_logs.unreadable;
    state.history_directory_unreadable = loaded_logs.directory_unreadable;

    let task = Task::batch([
        shared::refresh_workspace_task(&state),
        shared::scan_topology_task(&mut state),
    ]);
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
                // RFC-035 R22: card arrow-navigation. Confirmed against
                // `iced_widget::text_input`'s own key handling (0.14.2) that
                // it has no `ArrowUp`/`ArrowDown` branch at all — these are
                // gated in `handle_shortcut` anyway, for the user's typing
                // context rather than a widget-level conflict (see
                // `ShortcutMessage::CardFocusNext`'s own doc comment).
                keyboard::Key::Named(Named::ArrowDown) => Some(ShortcutMessage::CardFocusNext),
                keyboard::Key::Named(Named::ArrowUp) => Some(ShortcutMessage::CardFocusPrevious),
                keyboard::Key::Character(c) => match c.as_str() {
                    "r" | "R" if ctrl => Some(ShortcutMessage::Refresh),
                    "k" | "K" if ctrl => Some(ShortcutMessage::OpenContextOps),
                    "t" | "T" if ctrl => Some(ShortcutMessage::OpenFreezer),
                    "/" if ctrl => Some(ShortcutMessage::FocusSearch),
                    // RFC-036 R4: bare `/` (no modifier) — gated in
                    // `handle_shortcut`, not here, since only `AppState`
                    // knows whether a text input currently holds focus.
                    "/" => Some(ShortcutMessage::FocusSearchBare),
                    // RFC-035 R22: bare `j`/`k` (vim-style down/up), gated
                    // in `handle_shortcut` the same way. `k`/`K` with `Ctrl`
                    // is `OpenContextOps` above and must stay first in this
                    // match so the guarded arm claims it before this one.
                    "j" | "J" => Some(ShortcutMessage::CardFocusNext),
                    "k" | "K" => Some(ShortcutMessage::CardFocusPrevious),
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

    // RFC-035 R8/Handoff 029: iced's own documented resize subscription
    // shape (`iced-0.14.0/src/lib.rs:358`) — feeds `state.width_mode`,
    // reversed from the original `responsive`-based mechanism (see
    // `width_mode.rs`'s module doc for why).
    let resize_sub = iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size));

    Subscription::batch([tick_sub, keyboard_sub, fs_sub, resize_sub])
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
        Message::Workspace(msg) => workspace::handle_workspace(state, msg),
        Message::Project(msg) => misc::handle_project(state, msg),
        Message::Sync(msg) => sync::handle_sync(state, msg),
        Message::Freezer(msg) => freezer::handle_freezer(state, msg),
        Message::History(msg) => misc::handle_history(state, msg),
        Message::Settings(msg) => misc::handle_settings(state, msg),
        Message::Background(msg) => background::handle_background(state, msg),
        Message::Filter(msg) => {
            state.apply_filter(msg);
            state.reconcile_selection_with_display();
            Task::none()
        }
        Message::ConflictOps(msg) => conflict_ops::handle_conflict_ops(state, msg),
        Message::Changelog(msg) => changelog::handle_changelog(state, msg),
        Message::TagPush(msg) => misc::handle_tag_push(state, msg),
        Message::FsWatchTick => handle_fs_watch_tick(state),

        // ---------------------------------------------------------------
        // RFC-0009 — Selection
        // ---------------------------------------------------------------
        Message::Selection(sel) => misc::handle_selection(state, sel),

        // ---------------------------------------------------------------
        // RFC-0011 — Activity strip
        // ---------------------------------------------------------------
        Message::Activity(act) => activity::handle_activity(state, act),

        // ---------------------------------------------------------------
        // RFC-0012 — Command palette
        // ---------------------------------------------------------------
        Message::Palette(pal) => misc::handle_palette(state, pal),

        // ---------------------------------------------------------------
        // RFC-032 — Dashboard display controls
        // ---------------------------------------------------------------
        Message::Dashboard(msg) => misc::handle_dashboard(state, msg),

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
        Message::Context(msg) => context::handle_context(state, msg),
        Message::Launch(msg) => misc::handle_launch(state, msg),
        Message::WindowResized(size) => {
            state.width_mode = WidthMode::from_width(size.width);
            Task::none()
        }
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
        shared::refresh_workspace_task(state)
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
            context::handle_context(state, ContextMessage::OpenRequested(None))
        }
        ShortcutMessage::OpenFreezer => {
            freezer::handle_freezer(state, FreezerMessage::OpenRequested)
        }
        ShortcutMessage::FocusSearch => focus_ops::focus_search(state),
        ShortcutMessage::FocusSearchBare => {
            // R4: a bare `/` must not shadow a literal `/` being typed into
            // a field that already holds knotra-focus. `Ctrl+/` is
            // unconditional (unchanged from before RFC-036) because a
            // modified `/` was never a literal-typing conflict in the first
            // place.
            if focus_ops::current_target_is_text_input(state) {
                return Task::none();
            }
            focus_ops::focus_search(state)
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
            let was_open = focus_ops::workspace_dialog_open(state);
            let task = focus_ops::close_topmost_layer(state);
            if was_open && !focus_ops::workspace_dialog_open(state) {
                Task::batch([task, focus_ops::close_overlay_focus(state)])
            } else {
                task
            }
        }
        ShortcutMessage::FocusNext => focus_ops::advance_focus(state, focus::Direction::Next),
        ShortcutMessage::FocusPrevious => {
            focus_ops::advance_focus(state, focus::Direction::Previous)
        }
        ShortcutMessage::ActivateFocused => focus_ops::activate_focused(state),
        // RFC-035 R22: gated the same shape as `FocusSearchBare` — a text
        // input holding focus absorbs the keystroke instead of it moving
        // card focus out from under the user's typing.
        ShortcutMessage::CardFocusNext => {
            if focus_ops::current_target_is_text_input(state) {
                return Task::none();
            }
            focus_ops::advance_card_focus(state, focus::Direction::Next)
        }
        ShortcutMessage::CardFocusPrevious => {
            if focus_ops::current_target_is_text_input(state) {
                return Task::none();
            }
            focus_ops::advance_card_focus(state, focus::Direction::Previous)
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
            .filter_map(|e| shared::find_project(state, &e.project_id))
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
        shared::refresh_workspace_task(state)
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
    }
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::{AppState, focus};

    /// RFC-035 R22/Handoff 032 §4: the regression the handoff calls out as
    /// "most annoying and least obvious" — `j`/`k`/`↓`/`↑` reaching
    /// `handle_shortcut` while the search field holds focus must not move
    /// card focus out from under the user's typing, the same gate
    /// `FocusSearchBare` already uses.
    #[test]
    fn card_focus_shortcuts_do_nothing_while_a_text_input_holds_focus() {
        let mut state = AppState::new(AppConfig::default());
        let search_target =
            focus::FocusTarget::text_input(knotra_ui::widget::focus_id::SEARCH.clone());
        state.dashboard_focus = Some(search_target.clone());

        let _ = handle_shortcut(&mut state, ShortcutMessage::CardFocusNext);
        assert_eq!(state.dashboard_focus, Some(search_target.clone()));

        let _ = handle_shortcut(&mut state, ShortcutMessage::CardFocusPrevious);
        assert_eq!(state.dashboard_focus, Some(search_target));
    }
}
