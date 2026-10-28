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
        selection_bar, shell, shortcuts_overlay, workspace_manager,
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
/// The three workspace dialogs, Save release point, Resolve, Generate
/// notes, Change work area, Get latest safely, the command palette, Add
/// project, the workspace switcher menu and the shortcuts sheet all have
/// their own real, full `layer_focus` now (each window's own module), per
/// phase or step. Add project's order staying step-scoped (the folder
/// field's controls while choosing one, the name field's once chosen —
/// see `add_project_modal::layer_focus`) is also the Add-project defect's
/// fix: a failed "Next" changes neither the step nor the layer, so the
/// held target never falls out of its own step's order, and focus simply
/// stays on the folder field.
///
/// Remove project is still here — it had no open-site focus call before
/// this RFC either, so `None` really is "as today," pending Stage 3's own
/// real dialog (D6).
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
        Layer::RemoveProject => focus::LayerFocus {
            order: Vec::new(),
            entry: None,
        },
    }
}

pub(super) fn advance_focus(state: &mut AppState, direction: focus::Direction) -> Task<Message> {
    if let Some(order) = overlay_focus_order(state) {
        advance_in(&order, &mut state.overlay_focus, direction)
    } else {
        let order = shell_and_dashboard_focus_order(state);
        advance_in(&order, &mut state.dashboard_focus, direction)
    }
}

pub(super) fn activate_focused(state: &mut AppState) -> Task<Message> {
    if let Some(order) = overlay_focus_order(state) {
        activate_in(&order, &state.overlay_focus)
    } else {
        let order = shell_and_dashboard_focus_order(state);
        activate_in(&order, &state.dashboard_focus)
    }
}

/// RFC-035 R22's card arrow-navigation (Handoff 032): `↑`/`↓`/`j`/`k` move
/// between `dashboard::card_focus_order`'s row-name targets — a distinct,
/// narrower traversal from Tab's, not a change to `advance_focus` or the
/// order it walks. A no-op on any screen but the dashboard, and while an
/// overlay holds Tab/Enter routing (`overlay_focus_order(state).is_some()`)
/// — the same two conditions that already gate `advance_focus` itself, kept
/// here rather than folded into it.
///
/// **Entry from off-card focus** (the toolbar, a section header, the
/// selection bar, or the search field — none of which are members of
/// `card_focus_order`) **always lands on the first card, for either
/// direction.** This is deliberately *not* `advance_in`/`focus::advance`
/// applied directly: `focus::resolve`'s stale-target fallback already
/// returns the order's first entry for an unrecognized `current`, and
/// `focus::advance` would then step **one further** from that resolved
/// position — landing on the *second* card, not the first, for
/// `Direction::Next`. Handoff 032 §3 left this entry behavior to this
/// handoff's judgment rather than specifying it; "first card either way" was
/// chosen for being simpler to predict than a direction-dependent entry
/// point, and nothing else in this codebase enters a list from its far end.
/// Once focus is already on a card, this defers to `focus::advance`'s own
/// wrap-around semantics unchanged — same wrap Tab already uses, chosen for
/// consistency over a stop-at-the-edge alternative (Handoff 032 §3).
pub(super) fn advance_card_focus(
    state: &mut AppState,
    direction: focus::Direction,
) -> Task<Message> {
    if overlay_focus_order(state).is_some() || !matches!(state.screen, Screen::Dashboard) {
        return Task::none();
    }

    let order = dashboard::card_focus_order(state);
    if order.is_empty() {
        return Task::none();
    }

    let previous = state.dashboard_focus.clone();
    let already_on_a_card = previous
        .as_ref()
        .is_some_and(|target| order.iter().any(|(t, _)| t == target));

    let next = if already_on_a_card {
        focus::advance(&order, previous.as_ref(), direction).cloned()
    } else {
        order.first().map(|(target, _)| target.clone())
    };

    state.dashboard_focus = next.clone();
    reconciliation_task(focus::reconcile(previous.as_ref(), next.as_ref()))
}

/// The shell's order, plus dashboard-row targets (RFC-036 R2, Stage 4) when
/// the dashboard is the active screen. The toolbar (filter chips, group/sort,
/// search, select) is not included - it stays RFC-035's, unstyled and
/// unwired, matching Stage 2's precedent.
///
/// `selection_bar::focus_order` is appended **after** `dashboard::focus_order`
/// (Handoff 031 Finding 1) — the bar is not part of `dashboard::view`'s own
/// tree (`view.rs` composes it alongside `dashboard::view`, not inside it),
/// so it needs its own call here rather than folding into `dashboard`'s.
/// Positioned last to match visual order: the bar renders beneath the
/// dashboard's content.
fn shell_and_dashboard_focus_order(state: &AppState) -> focus::FocusOrder<Message> {
    let mut order = shell::focus_order(state);
    if matches!(state.screen, Screen::Dashboard) {
        order.extend(dashboard::focus_order(state));
        order.extend(selection_bar::focus_order(state));
    }
    order
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
pub(crate) fn ring(state: &AppState, target: &focus::FocusTarget) -> bool {
    if !state.focus_visible {
        return false;
    }
    match layer::top_layer(state) {
        Some(layer) => {
            let lf = layer_focus(state, &layer);
            focus::resolve(&lf.order, state.overlay_focus.as_ref()) == Some(target)
        }
        None => {
            let order = shell_and_dashboard_focus_order(state);
            focus::resolve(&order, state.dashboard_focus.as_ref()) == Some(target)
        }
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

/// R4: switches to the dashboard and moves knotra-focus (and iced's own
/// text-input focus, in the same `Task`) onto the search field. The search
/// field is not part of any declared `FocusOrder` (the toolbar stays
/// RFC-035's, per Stage 2/4's scope), so this sets `dashboard_focus`
/// directly rather than going through `advance`/`settle_layer_focus`.
pub(super) fn focus_search(state: &mut AppState) -> Task<Message> {
    state.screen = Screen::Dashboard;
    let target = focus::FocusTarget::text_input(knotra_ui::widget::focus_id::SEARCH.clone());
    let previous = state.dashboard_focus.replace(target.clone());
    reconciliation_task(focus::reconcile(previous.as_ref(), Some(&target)))
}

/// Whether the *current* context's knotra-focus target is a text input,
/// checked directly rather than through `focus::resolve` — the search field
/// is not a member of any declared order, so `resolve`'s "is this target
/// still in the order" fallback would (wrongly) say no even while the field
/// genuinely holds focus. R3a's gate can stay order-relative because
/// activation always needs an order to look an activation message up in;
/// this gate only needs to know what was last explicitly focused.
pub(super) fn current_target_is_text_input(state: &AppState) -> bool {
    let current = if overlay_focus_order(state).is_some() {
        &state.overlay_focus
    } else {
        &state.dashboard_focus
    };
    matches!(current, Some(focus::FocusTarget::TextInput(_)))
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
///   `order` — its phase changed under it: focus moves to `entry` too;
/// - no layer is open any more: focus is cleared.
///
/// An *empty* order (every layer but the three workspace dialogs, this
/// stage) is deliberately exempted from the second case: with nothing
/// declared to check membership against, "has the held target fallen out
/// of the order" is undecidable, and the pre-RFC-066 answer — leave
/// whatever is held alone — is also the only one that does not clear focus
/// on every keystroke a text field inside such a window receives (its own
/// `*Changed` message does not move the layer, so without this exemption
/// the very next settle call would find that field "stale" against its
/// empty order and clear it under the user's cursor).
pub(super) fn settle_layer_focus(state: &mut AppState, before: Option<Layer>) -> Task<Message> {
    let Some(current) = layer::top_layer(state) else {
        return close_overlay_focus(state);
    };
    let lf = layer_focus(state, &current);

    let moves_to_entry = before.as_ref() != Some(&current)
        || (!lf.order.is_empty()
            && state
                .overlay_focus
                .as_ref()
                .is_some_and(|f| !lf.order.iter().any(|(t, _)| t == f)));

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

    #[test]
    fn advance_card_focus_moves_through_row_name_targets_only_and_wraps() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(workspace_with_projects(&["alpha", "beta", "gamma"]));

        let order = dashboard::card_focus_order(&state);
        assert_eq!(
            order.len(),
            3,
            "all three NeedsHelp rows should be card targets, nothing else"
        );

        for target in &order {
            let _ = advance_card_focus(&mut state, focus::Direction::Next);
            assert_eq!(state.dashboard_focus, Some(target.0.clone()));
        }

        // A fourth Next wraps back to the first target — consistent with
        // Tab's own `focus::advance` wrap semantics (Handoff 032 §3).
        let _ = advance_card_focus(&mut state, focus::Direction::Next);
        assert_eq!(state.dashboard_focus, Some(order[0].0.clone()));
    }

    #[test]
    fn advance_card_focus_from_off_card_focus_lands_on_the_first_card_either_direction() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(workspace_with_projects(&["alpha", "beta"]));
        let order = dashboard::card_focus_order(&state);

        // Simulate focus sitting on a toolbar control — a real target, but
        // not a member of `card_focus_order`.
        state.dashboard_focus = Some(focus::FocusTarget::control("dashboard.toolbar.select_mode"));
        let _ = advance_card_focus(&mut state, focus::Direction::Previous);
        assert_eq!(
            state.dashboard_focus,
            Some(order[0].0.clone()),
            "Previous from off-card focus must not land on the second card \
             the way resolve-then-advance's stale-target fallback would"
        );

        state.dashboard_focus = Some(focus::FocusTarget::control("dashboard.toolbar.select_mode"));
        let _ = advance_card_focus(&mut state, focus::Direction::Next);
        assert_eq!(state.dashboard_focus, Some(order[0].0.clone()));
    }

    #[test]
    fn advance_card_focus_is_a_noop_off_the_dashboard_screen() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(workspace_with_projects(&["alpha"]));
        state.screen = Screen::History;

        let _ = advance_card_focus(&mut state, focus::Direction::Next);
        assert_eq!(state.dashboard_focus, None);
    }
}
