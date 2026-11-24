//! RFC-036 keyboard focus traversal: orchestration on top of the focus
//! model in `crate::state::focus`. Moved out of `app.rs` verbatim by
//! RFC-040 Stage 1.

use iced::Task;

use crate::{
    message::{ChangelogMessage, ContextMessage, FreezerMessage, Message, SyncMessage},
    state::{
        AppState, Screen, focus,
        freezer::FreezerPhase,
        layer::{self, Layer},
        sync::SyncPhase,
    },
    view::{
        add_project_modal, command_palette, dashboard,
        overlays::{
            changelog_layer_focus, conflict_close_message, conflict_layer_focus,
            context_switch_layer_focus, freezer_layer_focus, smart_pull_layer_focus,
        },
        remove_project_dialog, shell, shortcuts_overlay, workspace_manager,
    },
};

/// Whether Tab/Shift-Tab/Enter operate on a window's order right now, and if
/// so, what that order is (R5's confinement). `None` falls through to the
/// shell/dashboard context; `Some` — even an empty order — means a window is
/// on top and Tab/Enter stay confined to it.
///
/// RFC-066 D1/R1: thin wrapper over [`layer::top_layer`] and
/// [`layer_focus`], replacing the old per-field `any_other_overlay_is_open`
/// safety net — every window not yet given a real order (every layer but
/// the three workspace dialogs, this stage) still gets an *empty* order
/// here, the same no-op-Tab/Enter guarantee the old safety net gave, now by
/// construction: `layer_focus`'s default is `order: Vec::new()`, not a
/// per-window opt-out someone could forget to add.
pub(super) fn overlay_focus_order(state: &AppState) -> Option<focus::FocusOrder<Message>> {
    layer::top_layer(state).map(|l| layer_focus(state, &l).order)
}

/// RFC-066 D2: the declared order and entry for `layer` — the one place
/// every layer's `layer_focus` is looked up, so [`overlay_focus_order`] and
/// [`settle_layer_focus`] always see the same answer.
///
/// Every layer now has its own real `layer_focus` (each window's own
/// module), per phase or step — Stage 2 completes the sweep Stage 1
/// started with the three workspace dialogs. Add project's order staying
/// step-scoped (the folder field's controls while choosing one, the name
/// field's once chosen — see `add_project_modal::layer_focus`) is also the
/// Add-project defect's fix: a failed "Next" changes neither the step nor
/// the layer, so the held target never falls out of its own step's order,
/// and focus simply stays on the folder field. Remove project is a real
/// dialog now too (RFC-066 D6), the same `AppLayout::dialog` slot "Delete
/// workspace" uses — its own module lives at `view::remove_project_dialog`
/// rather than under `view::dashboard`, since it is no longer dashboard-
/// specific.
pub(super) fn layer_focus(state: &AppState, layer: &Layer) -> focus::LayerFocus<Message> {
    match layer {
        Layer::WorkspaceCreate | Layer::WorkspaceRename | Layer::WorkspaceDelete => {
            workspace_manager::layer_focus(state)
        }
        Layer::Palette => command_palette::layer_focus(state),
        Layer::AddProject => add_project_modal::layer_focus(state),
        Layer::SaveReleasePoint => freezer_layer_focus(state),
        Layer::Resolve(id) => conflict_layer_focus(state, id),
        Layer::GenerateNotes => changelog_layer_focus(state),
        Layer::ChangeWorkArea => context_switch_layer_focus(state),
        Layer::GetLatest => smart_pull_layer_focus(state),
        Layer::WorkspaceMenu => shell::switcher_menu_layer_focus(state),
        Layer::ShortcutsSheet => shortcuts_overlay::layer_focus(),
        Layer::RemoveProject => remove_project_dialog::layer_focus(state),
    }
}

/// RFC-075 D1: Tab/Shift-Tab on the page itself no longer calls this at
/// all — `app.rs`'s `handle_shortcut` routes those through `knotra_ui::
/// widget::keyboard::traverse_wrapping` directly whenever no window is
/// open. This stays for exactly the case the handoff leaves untouched this
/// stage: a window is open, and still uses knotra's own declared order.
pub(super) fn advance_focus(state: &mut AppState, direction: focus::Direction) -> Task<Message> {
    let Some(order) = overlay_focus_order(state) else {
        return Task::none();
    };
    advance_in(&order, &mut state.overlay_focus, direction)
}

/// See [`advance_focus`]'s own doc comment — the page's own Enter/Space no
/// longer reaches this either (RFC-075 §2: snora's `Control` presses
/// itself, through the ordinary event path, before this subscription-level
/// message could even be produced).
pub(super) fn activate_focused(state: &mut AppState) -> Task<Message> {
    let Some(order) = overlay_focus_order(state) else {
        return Task::none();
    };
    activate_in(&order, &state.overlay_focus)
}

/// RFC-035 R22's card arrow-navigation (Handoff 032), RFC-075 D6: `↑`/`↓`/
/// `j`/`k` move real focus among `dashboard::card_focus_ids`'s row-name
/// ids — a distinct, narrower traversal from Tab's. A no-op on any screen
/// but the dashboard, and while a window is open (D6: "the dashboard's own
/// keys stay knotra's" means *this* mechanism; a window's own order is
/// untouched, handled entirely by `handle_shortcut`'s own window branch
/// before this is ever called).
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
pub(super) fn advance_card_focus(state: &AppState, direction: focus::Direction) -> Task<Message> {
    if layer::top_layer(state).is_some() || !matches!(state.screen, Screen::Dashboard) {
        return Task::none();
    }

    let ids = dashboard::card_focus_ids(state);
    if ids.is_empty() {
        return Task::none();
    }
    let forward = matches!(direction, focus::Direction::Next);
    let search_id = knotra_ui::widget::focus_id::SEARCH.clone();

    knotra_ui::widget::keyboard::focused_id().then(move |current| {
        if current.as_ref() == Some(&search_id) {
            Task::none()
        } else {
            knotra_ui::widget::keyboard::focus_among(ids.clone(), forward)
        }
    })
}

/// RFC-066 D4: whether `target`'s focus outline should be drawn right now —
/// `focus_visible`, and `target` is the *resolved* current target of
/// whichever context is active (a window's own order if one is open, the
/// shell/dashboard order otherwise), through the same `focus::resolve` every
/// other focus decision in this app already goes through (the stale-target
/// fallback included, so a ring never silently vanishes just because the
/// view rebuilt). `pub(crate)`, not `pub(super)`: every window's own view
/// module calls this directly for its own controls' outlines, the one
/// shared implementation D4 asks for in place of the three per-module
/// `is_focused` copies this replaces (`shell.rs`, `selection_bar.rs`,
/// `workspace_manager.rs`).
/// RFC-075 §3: the page no longer has a knotra-drawn ring at all — every
/// control there is a snora `Control`/`select_control`/`checkbox_control`
/// now, each drawing its own ring from real focus directly, with no
/// `FocusTarget`/`ring()` call of its own. `None` (no window open) is kept
/// only because every `Layer::WorkspaceMenu` call still reaches this
/// function through the `Some` arm below (the switcher's own dropdown
/// stays knotra's model this stage) — a `target` can never resolve while
/// nothing is open, so this just states that rather than building an order
/// with nothing left to populate it.
pub(crate) fn ring(state: &AppState, target: &focus::FocusTarget) -> bool {
    if !state.focus_visible {
        return false;
    }
    match layer::top_layer(state) {
        Some(layer) => {
            let lf = layer_focus(state, &layer);
            focus::resolve(&lf.order, state.overlay_focus.as_ref()) == Some(target)
        }
        None => false,
    }
}

fn advance_in(
    order: &focus::FocusOrder<Message>,
    current: &mut Option<focus::FocusTarget>,
    direction: focus::Direction,
) -> Task<Message> {
    let previous = current.clone();
    let next = focus::advance(order, previous.as_ref(), direction).cloned();
    *current = next.clone();
    reconciliation_task(focus::reconcile(previous.as_ref(), next.as_ref()))
}

fn activate_in(
    order: &focus::FocusOrder<Message>,
    current: &Option<focus::FocusTarget>,
) -> Task<Message> {
    if focus::is_text_input_focused(order, current.as_ref()) {
        // R3a: a focused text input receives Enter/Space as a keystroke; it
        // must not also activate whatever control the ring last sat on.
        return Task::none();
    }
    focus::activation_message(order, current.as_ref())
        .map(Task::done)
        .unwrap_or_else(Task::none)
}

/// Applies a `focus::Reconciliation` by issuing the matching iced text-input
/// operation (R12). This is the one place `operation::focus`/
/// `clear_input_focus` is called on knotra-focus's behalf — see Guardrail 2.
fn reconciliation_task(reconciliation: focus::Reconciliation) -> Task<Message> {
    match reconciliation {
        focus::Reconciliation::None => Task::none(),
        focus::Reconciliation::FocusTextInput(id) => knotra_ui::widget::focus_input(&id),
        focus::Reconciliation::ClearTextInputFocus => knotra_ui::widget::clear_input_focus(),
    }
}

/// R6: sets knotra-focus to `target`, reconciling iced's text-input focus in
/// the same `Task` (R12). This is the one path every overlay-open call site
/// uses to move focus onto its opening target — the "seven-site" fix.
pub(super) fn open_overlay_focus(
    state: &mut AppState,
    target: focus::FocusTarget,
) -> Task<Message> {
    let previous = state.overlay_focus.replace(target.clone());
    reconciliation_task(focus::reconcile(previous.as_ref(), Some(&target)))
}

/// R7: focus return. Clears `overlay_focus`; `dashboard_focus` was never
/// touched while the overlay held focus (Tab/Enter routed to
/// `overlay_focus` the whole time via `overlay_focus_order`), so it is
/// already exactly what it was when the overlay opened — return happens by
/// construction, not by capturing and restoring a separate value. If the
/// overlay's last target was a text input, this also clears iced's own
/// text-input focus (R12), since nothing else will.
pub(super) fn close_overlay_focus(state: &mut AppState) -> Task<Message> {
    let previous = state.overlay_focus.take();
    reconciliation_task(focus::reconcile(previous.as_ref(), None))
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

/// Whether a window's own current target is a text input (R3a's gate) —
/// windows only; the page has nothing left to ask this of (RFC-075: no
/// `dashboard_focus` any more, and `ShortcutMessage::FocusSearchBare`'s own
/// page-only call site no longer needs this check at all, see its own
/// comment in `app.rs`).
pub(super) fn current_target_is_text_input(state: &AppState) -> bool {
    matches!(state.overlay_focus, Some(focus::FocusTarget::TextInput(_)))
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

/// RFC-066 D3: the one place focus moves, run after every `update()` call
/// with the layer that was on top *before* the message was handled.
/// Compares it against the layer on top *now*:
///
/// - a different layer is on top (including "a window just opened" —
///   `before` was `None`): focus moves to its `entry`;
/// - the same layer, but the held target has fallen out of its (non-empty)
///   `order` — its phase changed under it: focus moves to `entry` too.
///   Holding nothing (`overlay_focus` is `None`) counts as "fallen out" of
///   any non-empty order — there is no real target to preserve, so a
///   phase that goes from no controls to real ones (e.g. Get latest
///   safely's `Planning` into `AwaitingConfirm`, or Change work area's
///   `LoadingList` into `BrowsingList`) still settles onto the new phase's
///   entry instead of leaving focus unset. Found exactly this way: the
///   two cases above passed every hand-built-state test (which calls a
///   window's `layer_focus` directly) but left `overlay_focus` at `None`
///   through the real `update()` path, caught by a Tab-cycle test that
///   goes through `dispatch` instead;
/// - no layer is open any more: focus is cleared.
///
/// An *empty* order is still exempted from the second case for a held,
/// *real* target: with nothing declared to check its membership against,
/// "has the held target fallen out of the order" is undecidable, and the
/// pre-RFC-066 answer — leave whatever is held alone — is also the only
/// one that does not clear focus on every keystroke a text field inside
/// such a window receives (its own `*Changed` message does not move the
/// layer, so without this exemption the very next settle call would find
/// that field "stale" against its empty order and clear it under the
/// user's cursor).
pub(super) fn settle_layer_focus(state: &mut AppState, before: Option<Layer>) -> Task<Message> {
    let Some(current) = layer::top_layer(state) else {
        return close_overlay_focus(state);
    };
    let lf = layer_focus(state, &current);

    let held_is_stale = match &state.overlay_focus {
        None => true,
        Some(f) => !lf.order.iter().any(|(t, _)| t == f),
    };
    let moves_to_entry =
        before.as_ref() != Some(&current) || (!lf.order.is_empty() && held_is_stale);

    if moves_to_entry {
        match lf.entry {
            Some(target) => open_overlay_focus(state, target),
            None => close_overlay_focus(state),
        }
    } else {
        Task::none()
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

    /// Projects with no status entry classify as `NeedsHelp`/`StatusUnknown`
    /// (`state/dashboard.rs::classify`'s `else` branch) — the one tier
    /// `build_dashboard_display` never collapses regardless of config, so
    /// these fixtures need no `WorkspaceStatus` at all to land in a visible
    /// section.
    fn workspace_with_projects(names: &[&str]) -> Workspace {
        Workspace {
            projects: names.iter().map(|n| Project::new(*n, "/tmp")).collect(),
            ..Workspace::new("Test")
        }
    }

    /// Whether `task` is a plain no-op — `Task::none()`, or one of this
    /// function's own early-return guards, never issues any work at all.
    fn is_noop(task: Task<Message>) -> bool {
        iced_test::runtime::task::into_stream(task).is_none()
    }

    /// RFC-075 D6: `advance_card_focus`'s own guards, checked without
    /// needing to drive its real-focus query — each of these returns
    /// before ever building one. The full moves-and-wraps behaviour (this
    /// function's old synchronous test, before RFC-075) is now an
    /// acceptance test driven through the real view and harness instead
    /// (`crates/knotra-app/src/keyboard_acceptance.rs`, RFC-075 stage 2
    /// §5), since there is no `state.dashboard_focus` left to read here —
    /// only the live tree knows where focus is.
    #[test]
    fn advance_card_focus_is_a_noop_off_the_dashboard_screen() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(workspace_with_projects(&["alpha"]));
        state.screen = Screen::History;

        assert!(is_noop(advance_card_focus(&state, focus::Direction::Next)));
    }

    #[test]
    fn advance_card_focus_is_a_noop_with_nothing_to_focus() {
        let state = AppState::new(AppConfig::default());
        assert!(is_noop(advance_card_focus(&state, focus::Direction::Next)));
    }
}
