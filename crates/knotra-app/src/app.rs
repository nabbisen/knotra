//! Top-level Elm-architecture implementation for knotra.

mod activity;
mod background;
mod changelog;
mod conflict_ops;
mod context;
mod detail_panel;
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

// RFC-066 D4: `ring` is how every view module draws the focus outline —
// `focus_ops` itself stays a private submodule (nothing else in it is
// meant for `view` to reach), so this one function is re-exported at the
// `app` boundary instead of widening the whole module's visibility.
pub(crate) use focus_ops::ring;

use iced::{Element, Subscription, Task, clipboard, keyboard, time};
use std::time::Duration;

use knotra_vcs::model::workspace::Workspace;

use crate::{
    config::{AppPaths, load_config},
    fs_watcher::fs_watch_subscription,
    message::{
        BackgroundMessage, ContextMessage, FreezerMessage, KeyboardMessage, Message,
        ShortcutMessage, WorkspaceMessage,
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

    // RFC-066 Amendment A2: `filter_map`, not `map` — a key event that is
    // not a shortcut (a release, a modifiers change, any other letter)
    // produces no message at all now, where it used to fall through to
    // `Message::Tick` and start a full workspace status read. See
    // `shortcut_for`'s own doc comment for the defect this replaces.
    let keyboard_sub =
        keyboard::listen().filter_map(|event| shortcut_for_event(event).map(Message::Shortcut));

    let fs_sub = fs_watch_subscription(state);

    // RFC-035 R8/Handoff 029: iced's own documented resize subscription
    // shape (`iced-0.14.0/src/lib.rs:358`) — feeds `state.width_mode`,
    // reversed from the original `responsive`-based mechanism (see
    // `width_mode.rs`'s module doc for why).
    let resize_sub = iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size));

    // RFC-066 D4: clears the focus outline on a mouse button press, from
    // *anywhere* — `event::listen_with`, not `event::listen()`: verified by
    // reading `iced_futures-0.14.0/src/event.rs` directly, `listen()`
    // hardcodes a filter to `Status::Ignored` only (an event no widget
    // consumed), while `listen_with`'s callback receives every
    // `Interaction` event's real `Status` — `Captured` or `Ignored` — and
    // decides for itself. This callback ignores `status` entirely, so a
    // press a button widget captures still clears the outline, which a
    // plain `listen()` could not have delivered at all.
    let mouse_sub = iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => {
            Some(Message::Shortcut(ShortcutMessage::MousePressed))
        }
        _ => None,
    });

    Subscription::batch([tick_sub, keyboard_sub, fs_sub, resize_sub, mouse_sub])
}

/// The `Event`-level half of the keyboard subscription's mapping: only a
/// key *press* can be a shortcut (RFC-066 Amendment A2) — a release or a
/// modifiers-change event reaches this subscription too, and must produce
/// no message, not `Message::Tick`'s old fallback. Split from
/// [`shortcut_for`] so the latter's pure `Key`/`Modifiers` -> `ShortcutMessage`
/// mapping is independently testable without constructing a full
/// `keyboard::Event`, and keeps the shape Stage 3's own `shortcut_for`
/// (RFC D7) reuses directly.
fn shortcut_for_event(event: keyboard::Event) -> Option<ShortcutMessage> {
    match event {
        keyboard::Event::KeyPressed { key, modifiers, .. } => shortcut_for(&key, modifiers),
        keyboard::Event::KeyReleased { .. } | keyboard::Event::ModifiersChanged(_) => None,
    }
}

/// The keyboard shortcuts this app recognizes, as a pure function of the key
/// and its modifiers — `None` for anything that is not one (RFC-066
/// Amendment A2's fix: an un-mapped key used to fall through to
/// `Message::Tick` in the subscription closure this was extracted from,
/// starting a full workspace status read on every unrelated keystroke,
/// including every key *release*).
fn shortcut_for(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<ShortcutMessage> {
    use keyboard::key::Named;
    let ctrl = modifiers.control() || modifiers.command();
    match key {
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
        // RFC-035 R22/RFC-066 D5: arrow-key navigation. Confirmed against
        // `iced_widget::text_input`'s own key handling (0.14.2) that
        // it has no `ArrowUp`/`ArrowDown` branch at all — these are
        // gated in `handle_shortcut` anyway, for the user's typing
        // context rather than a widget-level conflict (see
        // `ShortcutMessage::ArrowDown`'s own doc comment). Separate
        // messages from `CardFocusNext`/`Previous` (`j`/`k`) since D5
        // gives arrows a different meaning inside an open window.
        keyboard::Key::Named(Named::ArrowDown) => Some(ShortcutMessage::ArrowDown),
        keyboard::Key::Named(Named::ArrowUp) => Some(ShortcutMessage::ArrowUp),
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
    }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    // RFC-066 D3: one focus step, run after every message. `before` is the
    // layer on top when this call started; `settle_layer_focus` compares it
    // against the layer on top once `message` has been fully handled and
    // moves focus accordingly — the one place entry, re-entry on a phase
    // change, and return on close happen, replacing the focus calls each
    // open/close site used to make individually.
    let before_layer = crate::state::layer::top_layer(state);
    let task = update_inner(state, message);
    Task::batch([task, focus_ops::settle_layer_focus(state, before_layer)])
}

fn update_inner(state: &mut AppState, message: Message) -> Task<Message> {
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
        Message::DetailPanel(dp) => detail_panel::handle_detail_panel(state, dp),

        Message::CopyToClipboard(text) => clipboard::write(text),
        Message::ToggleOpDetails => {
            state.show_op_details = !state.show_op_details;
            Task::none()
        }
        Message::Context(msg) => context::handle_context(state, msg),
        Message::Launch(msg) => misc::handle_launch(state, msg),
        Message::WindowResized(size) => {
            state.width_mode = WidthMode::from_width(size.width);
            state.window_width = size.width;
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
    // RFC-066 D4: these are the keys that mean "the keyboard is in use" —
    // set unconditionally, before anything below decides whether the key
    // does something right now. A mouse press is the only thing that turns
    // it back off, handled next.
    if matches!(
        msg,
        ShortcutMessage::FocusNext
            | ShortcutMessage::FocusPrevious
            | ShortcutMessage::ArrowDown
            | ShortcutMessage::ArrowUp
            | ShortcutMessage::CardFocusNext
            | ShortcutMessage::CardFocusPrevious
    ) {
        state.focus_visible = true;
    }
    if matches!(msg, ShortcutMessage::MousePressed) {
        state.focus_visible = false;
        return Task::none();
    }

    // Task 095 commit 1: while any window is open, these shortcuts act on
    // the screen behind it — `Ctrl+T` and `Ctrl+K` can replace or hide a
    // *running* window (its result never shown), and the search/card-focus
    // keys move focus underneath what the user is looking at. Gated on the
    // same predicate that already means "a window is open" for Tab/Enter
    // (R5's confinement); Escape, Tab, Shift+Tab, Enter, Space, and
    // (RFC-066 D5) the arrows are deliberately not in this list — they are
    // how the open window itself is used.
    if crate::state::layer::top_layer(state).is_some()
        && matches!(
            msg,
            ShortcutMessage::Refresh
                | ShortcutMessage::OpenContextOps
                | ShortcutMessage::OpenFreezer
                | ShortcutMessage::FocusSearch
                | ShortcutMessage::FocusSearchBare
                | ShortcutMessage::CardFocusNext
                | ShortcutMessage::CardFocusPrevious
        )
    {
        return Task::none();
    }
    match msg {
        // RFC-066 D7: the same message the "Check now" button sends
        // (`workspace.rs`'s own `RefreshRequested` arm), not `handle_tick`
        // — the two were not quite identical (the button also sets
        // `status_bar`), so Ctrl+R used to skip that status line.
        ShortcutMessage::Refresh => {
            workspace::handle_workspace(state, WorkspaceMessage::RefreshRequested)
        }
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
        // RFC-066 D1: the workspace switcher is `WorkspaceMenu`, a layer
        // like any other now — `close_topmost_layer`'s own exhaustive match
        // closes it, so this no longer needs a special case above that
        // function. Focus return on close is `update()`'s own
        // `settle_layer_focus` step now, not spliced in here (R7).
        ShortcutMessage::Close => focus_ops::close_topmost_layer(state),
        ShortcutMessage::FocusNext => focus_ops::advance_focus(state, focus::Direction::Next),
        ShortcutMessage::FocusPrevious => {
            focus_ops::advance_focus(state, focus::Direction::Previous)
        }
        // RFC-066 D4: with the ring hidden, Enter/Space on a *control*
        // reveals where focus is instead of pressing it — a text input is
        // unaffected (R3a's own short-circuit inside `activate_focused`
        // already gives it the keystroke, so it is not re-checked here).
        ShortcutMessage::ActivateFocused => {
            if !state.focus_visible && !focus_ops::current_target_is_text_input(state) {
                state.focus_visible = true;
                return Task::none();
            }
            focus_ops::activate_focused(state)
        }
        // RFC-035 R22: gated the same shape as `FocusSearchBare` — a text
        // input holding focus absorbs the keystroke instead of it moving
        // card focus out from under the user's typing. RFC-066 D5: while a
        // window is open, these already do nothing — caught by the gate
        // above, since `advance_card_focus` itself also no-ops whenever
        // `top_layer` is `Some`.
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
        // RFC-066 D5: inside a window, the same as Tab/Shift-Tab — advances
        // that window's own order (so the palette, the workspace menu and
        // "Find work area" work as lists). Outside one, today's card
        // navigation, unchanged, with the same typing gate `CardFocusNext`
        // already has.
        ShortcutMessage::ArrowDown => {
            if crate::state::layer::top_layer(state).is_some() {
                return focus_ops::advance_focus(state, focus::Direction::Next);
            }
            if focus_ops::current_target_is_text_input(state) {
                return Task::none();
            }
            focus_ops::advance_card_focus(state, focus::Direction::Next)
        }
        ShortcutMessage::ArrowUp => {
            if crate::state::layer::top_layer(state).is_some() {
                return focus_ops::advance_focus(state, focus::Direction::Previous);
            }
            if focus_ops::current_target_is_text_input(state) {
                return Task::none();
            }
            focus_ops::advance_card_focus(state, focus::Direction::Previous)
        }
        ShortcutMessage::MousePressed => unreachable!("handled above"),
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

    fn released(key: keyboard::Key) -> keyboard::Event {
        keyboard::Event::KeyReleased {
            key,
            modified_key: keyboard::Key::Unidentified,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
        }
    }

    fn pressed(key: keyboard::Key, modifiers: keyboard::Modifiers) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            key,
            modified_key: keyboard::Key::Unidentified,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }
    }

    /// RFC-066 Amendment A2. **Seen to fail first**, against the fallback
    /// this replaced: the old subscription closure ended `Message::Tick`
    /// for anything that was not `Event::KeyPressed` at all, so a release
    /// — reconstructed here exactly as the baseline handled it — mapped to
    /// `Tick`, not nothing.
    #[test]
    fn a_key_release_produces_no_shortcut() {
        let event = released(keyboard::Key::Named(keyboard::key::Named::Tab));
        assert_eq!(shortcut_for_event(event), None);
    }

    #[test]
    fn a_modifiers_change_produces_no_shortcut() {
        let event = keyboard::Event::ModifiersChanged(keyboard::Modifiers::SHIFT);
        assert_eq!(shortcut_for_event(event), None);
    }

    #[test]
    fn a_letter_that_is_not_a_shortcut_produces_none() {
        let event = pressed(
            keyboard::Key::Character("x".into()),
            keyboard::Modifiers::empty(),
        );
        assert_eq!(shortcut_for_event(event), None);
    }

    #[test]
    fn a_real_shortcut_still_maps_through_filter_map() {
        let event = pressed(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            keyboard::Modifiers::empty(),
        );
        assert_eq!(shortcut_for_event(event), Some(ShortcutMessage::Close));
    }

    /// RFC-066 D7/R7: the sheet's nine rows, each with an example `(Key,
    /// Modifiers)` and the `ShortcutMessage` it must produce through
    /// `shortcut_for`. A row whose "Keys" column lists more than one key
    /// (e.g. "Tab / Shift+Tab") is exercised at every key it lists, not
    /// just one — the point of "an example per row" is that the sheet
    /// cannot lie, and a sheet that shows a key this test never checks
    /// could still lie about it. `shortcut_for` never produces
    /// `ShortcutMessage::MousePressed` (the mouse subscription constructs
    /// it directly, not through a keyboard row), so it is the one variant
    /// this test does not require a row for.
    #[test]
    fn every_sheet_row_maps_through_shortcut_for_and_every_shortcut_message_has_a_row() {
        use keyboard::key::Named;
        let ctrl = keyboard::Modifiers::CTRL;
        let none = keyboard::Modifiers::empty();
        let shift = keyboard::Modifiers::SHIFT;

        // (row index into `shortcuts_overlay::BINDINGS`, the key pressed,
        // its modifiers, the message `shortcut_for` must produce for it).
        let cases: &[(usize, keyboard::Key, keyboard::Modifiers, ShortcutMessage)] = &[
            (
                0,
                keyboard::Key::Named(Named::Tab),
                none,
                ShortcutMessage::FocusNext,
            ),
            (
                0,
                keyboard::Key::Named(Named::Tab),
                shift,
                ShortcutMessage::FocusPrevious,
            ),
            (
                1,
                keyboard::Key::Named(Named::Enter),
                none,
                ShortcutMessage::ActivateFocused,
            ),
            (
                1,
                keyboard::Key::Named(Named::Space),
                none,
                ShortcutMessage::ActivateFocused,
            ),
            (
                2,
                keyboard::Key::Named(Named::Escape),
                none,
                ShortcutMessage::Close,
            ),
            (
                3,
                keyboard::Key::Named(Named::ArrowUp),
                none,
                ShortcutMessage::ArrowUp,
            ),
            (
                3,
                keyboard::Key::Named(Named::ArrowDown),
                none,
                ShortcutMessage::ArrowDown,
            ),
            (
                4,
                keyboard::Key::Character("j".into()),
                none,
                ShortcutMessage::CardFocusNext,
            ),
            (
                4,
                keyboard::Key::Character("k".into()),
                none,
                ShortcutMessage::CardFocusPrevious,
            ),
            (
                5,
                keyboard::Key::Character("/".into()),
                none,
                ShortcutMessage::FocusSearchBare,
            ),
            (
                5,
                keyboard::Key::Character("/".into()),
                ctrl,
                ShortcutMessage::FocusSearch,
            ),
            (
                6,
                keyboard::Key::Character("r".into()),
                ctrl,
                ShortcutMessage::Refresh,
            ),
            (
                7,
                keyboard::Key::Character("k".into()),
                ctrl,
                ShortcutMessage::OpenContextOps,
            ),
            (
                8,
                keyboard::Key::Character("t".into()),
                ctrl,
                ShortcutMessage::OpenFreezer,
            ),
        ];

        assert_eq!(
            crate::view::shortcuts_overlay::BINDINGS.len(),
            9,
            "RFC D7 has nine rows — update `cases` above if this changes"
        );

        let mut covered: Vec<ShortcutMessage> = Vec::new();
        for (row, key, modifiers, expected) in cases {
            let row_label = crate::view::shortcuts_overlay::BINDINGS[*row].keys;
            assert_eq!(
                shortcut_for(key, *modifiers),
                Some(expected.clone()),
                "row {row} ({row_label}): {key:?} with {modifiers:?} must map to {expected:?}"
            );
            covered.push(expected.clone());
        }

        let all_producible = [
            ShortcutMessage::Refresh,
            ShortcutMessage::OpenContextOps,
            ShortcutMessage::OpenFreezer,
            ShortcutMessage::FocusSearch,
            ShortcutMessage::Close,
            ShortcutMessage::FocusNext,
            ShortcutMessage::FocusPrevious,
            ShortcutMessage::ActivateFocused,
            ShortcutMessage::FocusSearchBare,
            ShortcutMessage::CardFocusNext,
            ShortcutMessage::CardFocusPrevious,
            ShortcutMessage::ArrowDown,
            ShortcutMessage::ArrowUp,
        ];
        for message in &all_producible {
            assert!(
                covered.contains(message),
                "{message:?} is producible by shortcut_for but has no row \
                 (MousePressed is the one exception — see this test's doc comment)"
            );
        }
    }

    /// RFC-051 R2: `window_width` and `width_mode` are both set on the same
    /// line in `Message::WindowResized`'s handler, so they cannot drift in
    /// that handler today — but the only way they *could* drift is someone
    /// editing exactly that line later, which is exactly when nobody
    /// re-checks a property that was true "by construction". Drives the
    /// real `update()` dispatch rather than re-deriving the pairing by
    /// hand, so this fails the moment the handler actually stops setting
    /// them together, not only when a hand-written duplicate of the
    /// invariant would.
    #[test]
    fn window_resized_keeps_window_width_and_width_mode_paired() {
        let mut state = AppState::new(AppConfig::default());

        let _ = update(
            &mut state,
            Message::WindowResized(iced::Size::new(900.0, 700.0)),
        );

        assert_eq!(state.window_width, 900.0);
        assert_eq!(
            state.width_mode,
            crate::view::dashboard::WidthMode::from_width(state.window_width)
        );
    }
}
