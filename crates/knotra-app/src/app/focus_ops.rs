//! RFC-075 stage 3 §3: entry and return, in one place. Replaces RFC-066
//! D3's own `settle_layer_focus` (knotra's declared-order model) with one
//! that returns `operation::focus(id)` directly against real snora focus —
//! every window's own controls are real `Control`/`text_input` targets now
//! (stage 3 §2), so there is no declared order left to advance through.
//!
//! What stays from before this stage (named explicitly, RFC-066 D1/D3's own
//! `Layer`/`top_layer` machinery): [`close_topmost_layer`] (Escape/the
//! scrim), [`smart_pull_is_running`]/[`freezer_is_running`] (Task 095/
//! RFC-067's own running-operation guards). What retires (handoff §5):
//! `overlay_focus_order`, `layer_focus` (the order+entry dispatcher),
//! `advance_focus`, `activate_focused`, `ring()`, and every window's own
//! `layer_focus`/`focus_order` — replaced by each window's own `entry_id`,
//! dispatched here.

use iced::Task;
use iced::widget::operation;

use crate::{
    message::{ChangelogMessage, ContextMessage, FreezerMessage, Message, SyncMessage},
    state::{
        AppState, Screen,
        freezer::FreezerPhase,
        layer::{self, Layer},
        sync::SyncPhase,
    },
    view::{
        add_project_modal, command_palette, dashboard,
        overlays::{
            changelog_entry_id, conflict_close_message, conflict_entry_id, context_switch_entry_id,
            freezer_entry_id, smart_pull_entry_id,
        },
        remove_project_dialog, shell, shortcuts_overlay, workspace_manager,
    },
};

/// RFC-075 stage 3 §3: `layer`'s own entry control, for whichever phase it
/// is in right now — the one place every window's own `entry_id` is looked
/// up, so [`settle_layer_focus`] always asks the same question every
/// window's own view module answers for itself. `pub(super)`: `app::update`
/// also calls this directly, *before* dispatching, to snapshot the entry
/// the layer on top had before the message — see [`settle_layer_focus`]'s
/// own doc comment for why.
pub(super) fn entry_id(state: &AppState, layer: &Layer) -> Option<iced::widget::Id> {
    match layer {
        Layer::WorkspaceCreate | Layer::WorkspaceRename | Layer::WorkspaceDelete => {
            workspace_manager::entry_id(state)
        }
        Layer::Palette => command_palette::entry_id(state),
        Layer::AddProject => add_project_modal::entry_id(state),
        Layer::SaveReleasePoint => freezer_entry_id(state),
        Layer::Resolve(id) => conflict_entry_id(state, id),
        Layer::GenerateNotes => changelog_entry_id(state),
        Layer::ChangeWorkArea => context_switch_entry_id(state),
        Layer::GetLatest => smart_pull_entry_id(state),
        Layer::WorkspaceMenu => shell::switcher_menu_entry_id(state),
        Layer::ShortcutsSheet => Some(shortcuts_overlay::entry_id()),
        Layer::RemoveProject => remove_project_dialog::entry_id(state),
    }
}

fn focus_entry(state: &AppState, layer: &Layer) -> Task<Message> {
    match entry_id(state, layer) {
        Some(id) => operation::focus(id),
        None => Task::none(),
    }
}

/// RFC-075 stage 3 §3: one step, run after every `update()` call with the
/// layer that was on top *before* the message was handled, and whether
/// that message itself was a keyboard shortcut (`Message::Shortcut`,
/// produced only from a real `keyboard::Event` — never from a mouse
/// click).
///
/// - **A different layer is on top** (`before != current`):
///   - **no layer now** (the window closed): nothing to do here. With
///     `snora::Inert`, the page's own `FollowsId` wrap already freezes its
///     memory of what was focused the moment the survey it reads goes
///     empty (the window taking focus is what empties it) and resumes
///     tracking the instant the survey is non-empty again — so "return to
///     the control that opened it, or its neighbour" happens with no
///     explicit code here (see `knotra_ui::widget::keyed::FollowsId::
///     follow`'s own doc). `Inert`'s own native opener-focus restore
///     cannot do this job itself: it parks whatever the page's content
///     holds at the next redraw, but by then this step has already moved
///     focus onto the window's own entry, so the page's own content holds
///     nothing for `Inert` to park — the return must come from the page's
///     own memory, not the engine's.
///   - **a window is now on top, replacing a different one** (both
///     `before` and `current` are `Some`, e.g. a palette command opening a
///     different window outright): the old window's own `FollowsId` state
///     is destroyed, not frozen, when a different window's content
///     replaces it in the same `AppLayout::dialog` slot — there is nothing
///     to inherit from it, so this always focuses the new layer's entry,
///     unconditionally. This is "from another window that holds it" (§3's
///     own bullet).
///   - **a window just opened from nothing** (`before` is `None`): opened
///     from the keyboard — by a shortcut (`is_shortcut`), or while a page
///     control held real focus (asked of `knotra_ui::widget::keyed::
///     tab_anchor`, which reflects the page's own last-known focus even
///     while frozen) — focuses the entry; opened by a click, nothing is
///     focused (RFC-121: a click presses without focusing, so there is
///     nothing for `tab_anchor` to report and no shortcut to have fired).
/// - **The same layer is on top, still open:** compares `before_entry`
///   (the layer's own entry *before* the message, snapshotted by
///   `app::update` while `state` still held it) against this layer's
///   entry *now*. A real phase transition changes this for every window
///   in this file (each one's own `entry_id` is phase-keyed) — a plain
///   edit inside an unchanged phase (typing in a field, toggling a
///   disposition `reserved` already kept focus on) never does. Only on a
///   genuine change does this step check whether anything is really
///   focused right now and, if not, move focus to the new entry — never
///   unconditionally, so a window opened by a click with nothing focused
///   is never silently re-focused by some *later*, unrelated message
///   (a background tick, say) finding the same "nothing focused" state
///   and mistaking it for a phase change.
pub(super) fn settle_layer_focus(
    state: &AppState,
    before: Option<Layer>,
    before_entry: Option<iced::widget::Id>,
    is_shortcut: bool,
) -> Task<Message> {
    let current = layer::top_layer(state);

    if current != before {
        return match (&before, &current) {
            (_, None) => Task::none(),
            (Some(_), Some(layer)) => focus_entry(state, layer),
            (None, Some(layer)) => {
                if is_shortcut {
                    return focus_entry(state, layer);
                }
                // Computed now, while `state` is still in hand — the
                // `Task::then` closure below runs later, against
                // whatever `tab_anchor` reports, with no `AppState` of
                // its own to ask again.
                let entry = entry_id(state, layer);
                knotra_ui::widget::keyed::tab_anchor().then(move |anchor| {
                    if anchor.is_some_and(|a| a.focused.is_some()) {
                        match &entry {
                            Some(id) => operation::focus(id.clone()),
                            None => Task::none(),
                        }
                    } else {
                        Task::none()
                    }
                })
            }
        };
    }

    let Some(layer) = current else {
        return Task::none();
    };
    let entry_now = entry_id(state, &layer);
    if entry_now == before_entry {
        return Task::none();
    }
    knotra_ui::widget::keyboard::focused_id().then(move |focused| {
        if focused.is_none() {
            match &entry_now {
                Some(id) => operation::focus(id.clone()),
                None => Task::none(),
            }
        } else {
            Task::none()
        }
    })
}

/// RFC-075 stage 3 §1: the detail panel's own entry and return — it is
/// not a [`Layer`]/window (it renders inline in the page, under stage 2's
/// one page-wide `focus_by_id` wrap, not through `AppLayout::dialog`/
/// `.sheet`), so [`settle_layer_focus`] never sees it and `snora::Inert`
/// never freezes the page's own memory for it either. It opens only by a
/// press on a project's own name control, or from a palette command — no
/// bare global shortcut reaches it with nothing already focused — so
/// "from the keyboard" here is decided purely by `tab_anchor`: a click
/// never focuses (RFC-121), so only a real prior focus (the row itself,
/// or the palette's own field) can be behind this.
///
/// **Return in the common case**: the panel's own entry
/// (`detail_panel::close()`) is itself one of the page's own tracked ids
/// (the panel is inline, not a separate wrap), so moving focus onto it
/// when the panel opens overwrites the page's own memory of the row that
/// opened it — unlike the window case, nothing here freezes that memory
/// first. So the return is computed explicitly, from `before`'s own
/// project id (`DetailPanelMessage::Opened` is the only way in, always
/// from that exact row's name control) rather than relied on from the
/// page's own tracking. If that row is gone (its project was removed
/// while the panel was open), `operation::focus` on its id simply finds
/// nothing, and the page's own `FollowsId` wrap's general neighbour
/// fallback (A1's own rule, tracking whatever it last remembered) takes
/// over from there — not a precise "neighbour of this exact row", but
/// focus lands somewhere sensible rather than nowhere.
pub(super) fn settle_detail_panel_focus(
    state: &AppState,
    before: Option<knotra_vcs::ProjectId>,
) -> Task<Message> {
    let current = state.detail_panel.open_project_id.clone();
    if current == before {
        return Task::none();
    }
    match (before, current) {
        (None, Some(_)) => knotra_ui::widget::keyed::tab_anchor().then(|anchor| {
            if anchor.is_some_and(|a| a.focused.is_some()) {
                operation::focus(crate::view::ids::detail_panel::close())
            } else {
                Task::none()
            }
        }),
        (Some(opener), None) => operation::focus(crate::view::ids::row::name(&opener)),
        _ => Task::none(),
    }
}

/// RFC-035 R22's card arrow-navigation (Handoff 032), RFC-075 D6: `↑`/`↓`/
/// `j`/`k` move real focus among `dashboard::card_focus_ids`'s row-name
/// ids — a distinct, narrower traversal from Tab's. A no-op on any screen
/// but the dashboard, and while a window is open.
///
/// **Not synchronous any more**: with no `state.dashboard_focus` left to
/// read (RFC-075 retires it — nothing on the page tracks its own focus any
/// more, snora's real widget focus is the only copy), "which card, if any,
/// holds focus right now" can only be asked of the live tree, the same way
/// [`knotra_ui::widget::keyboard::traverse_wrapping`] asks it. Also guards
/// the one case D6 and the handoff's §2 both call out: the search field is
/// a real Tab target too (an ordinary `text_input`), and arrow keys must
/// not steal focus away from it — compared by id, since it is the one and
/// only text field in this traversal's reach.
///
/// **Entry from off-card focus** (the toolbar, a section header, the
/// selection bar, or nothing at all — none of which are members of
/// `card_focus_ids`) **always lands on the first card, for either
/// direction** — [`knotra_ui::widget::keyboard::focus_among`]'s own "focus
/// on none of `ids` lands on `ids[0]`" rule, unchanged from the retired
/// synchronous version's own entry behaviour (Handoff 032 §3: "first card
/// either way", chosen for being simpler to predict than a
/// direction-dependent entry point).
pub(super) fn advance_card_focus(state: &AppState, forward: bool) -> Task<Message> {
    if layer::top_layer(state).is_some() || !matches!(state.screen, Screen::Dashboard) {
        return Task::none();
    }

    let ids = dashboard::card_focus_ids(state);
    if ids.is_empty() {
        return Task::none();
    }
    let search_id = knotra_ui::widget::focus_id::SEARCH.clone();

    knotra_ui::widget::keyboard::focused_id().then(move |current| {
        if current.as_ref() == Some(&search_id) {
            Task::none()
        } else {
            knotra_ui::widget::keyboard::focus_among(ids.clone(), forward)
        }
    })
}

/// RFC-075 stage 3 §4: ↓/↑ while the command palette is open move its own
/// highlight, not real focus (§4's one exception — the results are plain
/// iced buttons, not Tab stops).
pub(super) fn palette_highlight_moved(state: &mut AppState, forward: bool) -> Task<Message> {
    super::misc::handle_palette(
        state,
        crate::message::PaletteMessage::HighlightMoved(forward),
    )
}

/// RFC-075 stage 3 §4: the same shape as [`palette_highlight_moved`], for
/// Change work area's "Find work area" field and its candidates.
pub(super) fn context_switch_highlight_moved(state: &mut AppState, forward: bool) -> Task<Message> {
    super::context::handle_context(state, ContextMessage::HighlightMoved(forward))
}

/// RFC-075 stage 3 §4: the workspace menu is "a list on its own" — its
/// rows are real snora controls and Tab stops, so ↓/↑ move real focus
/// between them (`knotra_ui::widget::keyboard::focus_among`, the same
/// wrapping/skip-disabled primitive the dashboard's own card navigation
/// uses), rather than a second, non-focus highlight.
pub(super) fn switcher_menu_focus_moved(state: &AppState, forward: bool) -> Task<Message> {
    let ids = shell::switcher_menu_ids(state);
    if ids.is_empty() {
        return Task::none();
    }
    knotra_ui::widget::keyboard::focus_among(ids, forward)
}

/// R4, RFC-075 D6: switches to the dashboard and moves real focus onto the
/// search field. No knotra-focus value to set any more — the search field
/// is a real, ordinary `text_input`, so `operation::focus` alone (via
/// `focus_input`) is enough; nothing reconciles against it because nothing
/// else on the page holds a value that could disagree with it.
pub(super) fn focus_search(state: &mut AppState) -> Task<Message> {
    state.screen = Screen::Dashboard;
    knotra_ui::widget::focus_input(&knotra_ui::widget::focus_id::SEARCH)
}

/// RFC-066 D1/R1: one exhaustive match over [`layer::top_layer`], sending
/// that window's own Close message in its current phase — Task 095's and
/// RFC-067's per-window Close principle, now keyed by `Layer` instead of a
/// hand-ordered chain of `if`/`else` over individual state fields (which
/// could, and did, drift from `view.rs`'s real stacking order). Each
/// handler still no-ops while its own operation is running; the
/// selection-clearing fallback stays for `None`. This also absorbs the
/// pre-RFC-066 workspace-switcher special case in `app.rs` — `WorkspaceMenu`
/// is a layer like any other now, so `app.rs` no longer checks it before
/// calling this function.
pub(super) fn close_topmost_layer(state: &mut AppState) -> Task<Message> {
    use crate::message::{PaletteMessage, WorkspaceMessage};

    match layer::top_layer(state) {
        None => {
            if state.selection_mode {
                state.clear_selection_mode();
            }
            Task::none()
        }
        Some(Layer::ShortcutsSheet) => {
            state.keyboard.cheat_sheet_open = false;
            Task::none()
        }
        Some(Layer::Palette) => super::misc::handle_palette(state, PaletteMessage::Closed),
        Some(Layer::AddProject) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::AddProjectCancelled)
        }
        Some(Layer::WorkspaceCreate) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::CreateWorkspaceCancelled)
        }
        Some(Layer::WorkspaceRename) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::RenameWorkspaceCancelled)
        }
        Some(Layer::WorkspaceDelete) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::DeleteWorkspaceCancelled)
        }
        Some(Layer::GetLatest) => super::sync::handle_sync(state, SyncMessage::ModalClosed),
        Some(Layer::SaveReleasePoint) => {
            super::freezer::handle_freezer(state, FreezerMessage::BulkModalClosed)
        }
        Some(Layer::ChangeWorkArea) => {
            super::context::handle_context(state, ContextMessage::BulkModalClosed)
        }
        Some(Layer::GenerateNotes) => {
            super::changelog::handle_changelog(state, ChangelogMessage::ModalClosed)
        }
        Some(Layer::Resolve(id)) => match conflict_close_message(state, &id) {
            Some(msg) => super::conflict_ops::handle_conflict_ops(state, msg),
            None => Task::none(),
        },
        // `WorkspaceToggled` toggles, not closes-only — safe here because
        // `top_layer` only returns `WorkspaceMenu` when `switcher_open` is
        // already `true` (`state/layer.rs`), so this always closes it.
        Some(Layer::WorkspaceMenu) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::SwitcherToggled)
        }
        Some(Layer::RemoveProject) => {
            super::workspace::handle_workspace(state, WorkspaceMessage::RemoveProjectCancelled)
        }
    }
}

pub(super) fn smart_pull_is_running(state: &AppState) -> bool {
    matches!(state.active_modal, crate::state::ActiveModal::Pull)
        && matches!(state.sync.phase, SyncPhase::PullRunning { .. })
}

pub(super) fn freezer_is_running(state: &AppState) -> bool {
    matches!(state.freezer.phase, FreezerPhase::Executing)
        || state
            .pending_tag_push
            .as_ref()
            .is_some_and(|push| push.is_pushing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use knotra_vcs::{Project, Workspace};

    fn workspace_with_projects(names: &[&str]) -> Workspace {
        Workspace {
            projects: names.iter().map(|n| Project::new(*n, "/tmp")).collect(),
            ..Workspace::new("Test")
        }
    }

    fn is_noop(task: Task<Message>) -> bool {
        iced_test::runtime::task::into_stream(task).is_none()
    }

    #[test]
    fn advance_card_focus_is_a_noop_off_the_dashboard_screen() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(workspace_with_projects(&["alpha"]));
        state.screen = Screen::History;

        assert!(is_noop(advance_card_focus(&state, true)));
    }

    #[test]
    fn advance_card_focus_is_a_noop_with_nothing_to_focus() {
        let state = AppState::new(AppConfig::default());
        assert!(is_noop(advance_card_focus(&state, true)));
    }
}
