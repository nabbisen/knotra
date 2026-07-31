//! RFC-036 keyboard focus traversal: orchestration on top of the focus
//! model in `crate::state::focus`. Moved out of `app.rs` verbatim by
//! RFC-040 Stage 1.

use iced::Task;

use crate::{
    message::Message,
    state::{
        AppState, Screen, conflict_ops::ConflictPhase, context::ContextPhase, focus,
        freezer::FreezerPhase, sync::SyncPhase,
    },
    view::{dashboard, shell, workspace_manager},
};

use super::shared::{cancel_freezer_validation, clear_sync_retry_context};

/// Whether Tab/Shift-Tab/Enter operate on an overlay's order right now, and
/// if so, which one (R5's confinement). `None` falls through to the
/// shell/dashboard context.
///
/// Only the three workspace-manager dialogs get a real, multi-target order
/// this stage (Stage 3's explicit change scope). Every other overlay this
/// app can show — the mutating-workflow overlays RFC-037 owns, the
/// add-project dialog, the command palette, the shortcuts cheat sheet, the
/// confirm-remove dialog, and the switcher menu — gets an *empty* order
/// here instead of `None`. That is a deliberate safety net, not an
/// oversight: without it, Tab pressed while one of those covers the screen
/// would fall through to the shell/dashboard underneath, and Enter could
/// activate a hidden background control the user cannot see. An empty order
/// makes Tab/Enter safe no-ops there instead. Each of those overlays still
/// gets the seven-site R12 fix at its own focus_input call site — see
/// `open_overlay_focus` — so knotra-focus and iced-focus never diverge for
/// them either; what they don't get is a navigable order, which RFC-037 (or
/// a later RFC-036 stage, for the palette/add-project/cheat-sheet layers
/// `view.rs` already notes this RFC is expected to migrate) can add without
/// touching this function's shape.
fn overlay_focus_order(state: &AppState) -> Option<focus::FocusOrder<Message>> {
    if let Some(order) = workspace_manager::focus_order(state) {
        return Some(order);
    }
    if any_other_overlay_is_open(state) {
        return Some(Vec::new());
    }
    None
}

fn any_other_overlay_is_open(state: &AppState) -> bool {
    !matches!(state.active_modal, crate::state::ActiveModal::None)
        || state.add_project_dialog.is_some()
        || state.palette.open
        || state.keyboard.cheat_sheet_open
        || state.confirm_remove_dialog.is_some()
        || state.workspace_mgr.switcher_open
}

/// Whether one of the three workspace-manager dialogs specifically is open —
/// the subset of overlays this stage gives a real order, trap, and focus
/// return to.
pub(super) fn workspace_dialog_open(state: &AppState) -> bool {
    state.workspace_mgr.create_dialog.is_some()
        || state.workspace_mgr.rename_dialog.is_some()
        || state.workspace_mgr.confirm_delete.is_some()
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

/// The shell's order, plus dashboard-row targets (RFC-036 R2, Stage 4) when
/// the dashboard is the active screen. The toolbar (filter chips, group/sort,
/// search, select) is not included - it stays RFC-035's, unstyled and
/// unwired, matching Stage 2's precedent.
fn shell_and_dashboard_focus_order(state: &AppState) -> focus::FocusOrder<Message> {
    let mut order = shell::focus_order(state);
    if matches!(state.screen, Screen::Dashboard) {
        order.extend(dashboard::focus_order(state));
    }
    order
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

/// R6: sets knotra-focus to the first target in the *current* overlay's
/// declared order — used by the three workspace-manager dialogs, whose
/// order-builders deliberately place the desired entry control first (the
/// name field for create/rename, Cancel — the safe action — for delete).
/// A no-op if no overlay order applies right now.
pub(super) fn enter_overlay_focus(state: &mut AppState) -> Task<Message> {
    let Some(order) = overlay_focus_order(state) else {
        return Task::none();
    };
    let Some((entry, _)) = order.first().cloned() else {
        return Task::none();
    };
    open_overlay_focus(state, entry)
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
/// directly rather than going through `advance`/`enter_overlay_focus`.
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

pub(super) fn close_topmost_layer(state: &mut AppState) -> Task<Message> {
    if state.keyboard.cheat_sheet_open {
        state.keyboard.cheat_sheet_open = false;
    } else if state.palette.open {
        state.palette.open = false;
    } else if state.add_project_dialog.is_some() {
        state.add_project_dialog = None;
    } else if state.workspace_mgr.create_dialog.is_some() {
        state.workspace_mgr.create_dialog = None;
    } else if state.workspace_mgr.rename_dialog.is_some() {
        state.workspace_mgr.rename_dialog = None;
    } else if state.workspace_mgr.confirm_delete.is_some() {
        state.workspace_mgr.confirm_delete = None;
    } else if matches!(state.active_modal, crate::state::ActiveModal::Pull)
        && matches!(state.sync.phase, SyncPhase::RetryPreparing)
    {
        clear_sync_retry_context(state);
        state.active_modal = crate::state::ActiveModal::None;
    } else if matches!(state.active_modal, crate::state::ActiveModal::Tag)
        && matches!(state.freezer.phase, FreezerPhase::Validating { .. })
    {
        cancel_freezer_validation(state);
        state.active_modal = crate::state::ActiveModal::None;
    } else if smart_pull_is_running(state)
        || freezer_is_running(state)
        || context_switch_is_running(state)
        || conflict_is_running(state)
    {
        return Task::none();
    } else if !matches!(state.active_modal, crate::state::ActiveModal::None) {
        state.active_modal = crate::state::ActiveModal::None;
    } else if state.confirm_remove_dialog.is_some() {
        state.confirm_remove_dialog = None;
    } else if state.selection_mode {
        state.clear_selection_mode();
    }
    Task::none()
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

fn context_switch_is_running(state: &AppState) -> bool {
    matches!(state.active_modal, crate::state::ActiveModal::Switch)
        && matches!(state.context_ops.phase, ContextPhase::Switching { .. })
}

fn conflict_is_running(state: &AppState) -> bool {
    matches!(state.active_modal, crate::state::ActiveModal::Resolve(_))
        && matches!(state.conflict_ops.phase, ConflictPhase::Operating { .. })
}
