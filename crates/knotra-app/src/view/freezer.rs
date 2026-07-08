//! Freezer view — atomic cross-repository tag/bookmark creation.

use knotra_vcs::{FreezeOutcome, FreezeResult, FreezeValidation, FreezeValidationEntry};
use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{FreezerMessage, Message, TopologyMessage},
    state::{freezer::FreezerPhase, AppState},
};

// ---------------------------------------------------------------------------
// Top-level dispatcher
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let body: Element<'_, Message> = match &state.freezer.phase {
        FreezerPhase::Idle              => view_idle(state),
        FreezerPhase::Validating        => centered(state.t("freezer.validating")),
        FreezerPhase::ValidationReady(v) => view_validation(state, v.clone()),
        FreezerPhase::Executing         => centered(state.t("freezer.executing")),
        FreezerPhase::Done(r)           => view_done(state, r.clone()),
    };

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn view_header(state: &AppState) -> Element<'_, Message> {
    row![
        button(text(state.t("freezer.back")))
            .on_press(Message::Freezer(FreezerMessage::BackToDashboard)),
        text(state.t("freezer.title")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Idle — name input + project selection
// ---------------------------------------------------------------------------

fn view_idle(state: &AppState) -> Element<'_, Message> {
    let projects = state.workspace.as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);

    let name_valid = state.freezer.freeze_name_is_valid();

    let name_input = text_input(
        state.t("freezer.name_hint"),
        &state.freezer.freeze_name,
    )
    .on_input(|s| Message::Freezer(FreezerMessage::NameChanged(s)))
    .width(300);

    let tag_msg_input = text_input(
        state.t("freezer.tag_message_hint"),
        &state.freezer.tag_message,
    )
    .on_input(|s| Message::Freezer(FreezerMessage::TagMessageChanged(s)))
    .width(350);

    let name_error: Element<'_, Message> = if !name_valid && !state.freezer.freeze_name.is_empty() {
        text(state.t("freezer.name_invalid")).size(11).into()
    } else {
        Space::new().into()
    };

    // Project checkboxes.
    let project_rows: Vec<Element<'_, Message>> = projects.iter().map(|p| {
        let included = state.freezer.is_selected(&p.id);
        let id = p.id.clone();
        row![
            checkbox(included)
                .label(p.name.as_str())
                .on_toggle(move |v| Message::Freezer(FreezerMessage::ProjectToggled(id.clone(), v))),
        ]
        .padding([2, 0])
        .into()
    }).collect();

    let scan_btn = button(text(state.t("topology.scan")).size(12))
        .on_press(Message::Topology(TopologyMessage::ScanRequested));

    let validate_btn = button(text(state.t("freezer.validate")))
        .on_press_maybe(
            if name_valid && !projects.is_empty() {
                Some(Message::Freezer(FreezerMessage::ValidateRequested))
            } else { None }
        );

    column![
        text(state.t("freezer.name_label")).size(13),
        name_input,
        text(state.t("freezer.tag_message_label")).size(13),
        tag_msg_input,
        name_error,
        text(state.t("freezer.projects_label")).size(13),
        column(project_rows).spacing(2),
        row![validate_btn, scan_btn].spacing(8).padding([8, 0]),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Validation results + confirm step
// ---------------------------------------------------------------------------

fn view_validation(state: &AppState, validation: FreezeValidation) -> Element<'_, Message> {
    let all_ready   = validation.all_ready();
    let freeze_name = validation.freeze_name.clone();

    let summary: &str = if all_ready {
        state.t("freezer.validation_ok")
    } else {
        state.t("freezer.validation_blocked")
    };

    // Topology impact warnings.
    let impact_warnings: Vec<Element<'_, Message>> = state.topology.impact_warnings.iter()
        .filter(|w| validation.entries.iter().any(|e| e.ready() && e.project_name == w.frozen_project_name))
        .map(|w| {
            text(format!("  {} {}", state.t("topology.warning_prefix"), w.description())).size(11).into()
        })
        .collect();

    let entry_rows: Vec<Element<'_, Message>> = validation.entries.into_iter()
        .map(|entry| view_validation_entry_owned(state, entry))
        .collect();

    let execute_btn = button(text(state.t("freezer.execute")))
        .on_press_maybe(
            if all_ready { Some(Message::Freezer(FreezerMessage::ExecuteConfirmed)) } else { None }
        );

    let revalidate_btn = button(text(state.t("freezer.revalidate")))
        .on_press(Message::Freezer(FreezerMessage::RevalidateRequested));

    let cancel_btn = button(text(state.t("freezer.cancel")))
        .on_press(Message::Freezer(FreezerMessage::Cancelled));

    let header_text = format!("{} — {}", state.t("freezer.title"), freeze_name);

    let topo_col = column(impact_warnings).spacing(2);

    column![
        text(header_text).size(16),
        text(summary).size(13),
        topo_col,
        column(entry_rows).spacing(4),
        row![execute_btn, revalidate_btn, cancel_btn].spacing(8).padding([8, 0]),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn view_validation_entry_owned(
    state: &AppState,
    entry: FreezeValidationEntry,
) -> Element<'static, Message> {
    let status_icon = if !entry.included { "○" } else if entry.is_blocked() { "✗" } else { "✓" };
    let status_label = if !entry.included { "excluded" } else if entry.is_blocked() { "blocked" } else { "ready" };
    let header = format!("{status_icon} {} — {status_label}", entry.project_name);
    let blocker_label = state.t("freezer.blocker_label").to_owned();
    let note_label    = state.t("freezer.note_label").to_owned();

    let mut blockers: Vec<Element<'static, Message>> = entry.blockers.into_iter()
        .map(|b| text(format!("  {} {}", blocker_label, b)).size(11).into())
        .collect();
    let mut notes: Vec<Element<'static, Message>> = entry.notes.into_iter()
        .map(|n| text(format!("  {} {}", note_label, n)).size(11).into())
        .collect();

    let mut items: Vec<Element<'static, Message>> = vec![text(header).size(13).into()];
    items.append(&mut blockers);
    items.append(&mut notes);
    column(items).spacing(2).into()
}

fn view_done(state: &AppState, result: FreezeResult) -> Element<'_, Message> {
    let outcome_label = match result.outcome {
        FreezeOutcome::Success       => state.t("freezer.done.success"),
        FreezeOutcome::RolledBack    => state.t("freezer.done.rolledback"),
        FreezeOutcome::RollbackFailed=> state.t("freezer.done.rollback_fail"),
        FreezeOutcome::NothingDone   => state.t("freezer.done.nothing"),
    };

    let mut col = column![
        text(outcome_label).size(18),
        text(format!("Freeze name: {}", result.freeze_name)).size(13),
        text(format!("{} succeeded, {} failed",
            result.success_count(), result.failed_count())).size(13),
    ]
    .spacing(6)
    .padding(24);

    // Per-project results.
    for pr in &result.project_results {
        let icon = if pr.success { "✓" } else if pr.rollback_attempted {
            if pr.rollback_succeeded == Some(true) { "↩" } else { "✗✗" }
        } else { "✗" };

        let rollback_note = if pr.rollback_attempted {
            if pr.rollback_succeeded == Some(true) { " (rolled back)" }
            else { " (ROLLBACK FAILED)" }
        } else { "" };

        col = col.push(
            text(format!("  {icon} {}{rollback_note}", pr.project_name)).size(12)
        );

        if !pr.commands_executed.is_empty() {
            col = col.push(text(state.t("freezer.done.commands")).size(11));
            for cmd in &pr.commands_executed {
                col = col.push(text(format!("    $ {cmd}")).size(10));
            }
        }
    }

    // Recovery hints.
    let hints = result.recovery_hints();
    if !hints.is_empty() {
        col = col.push(text(state.t("freezer.done.recovery")).size(13));
        for hint in hints {
            col = col.push(text(format!("  {}", hint.situation)).size(12));
            for cmd in &hint.suggested_commands {
                col = col.push(text(format!("    $ {cmd}")).size(10));
            }
        }
    }

    // Navigation.
    col = col.push(
        row![
            button(text(state.t("freezer.back")))
                .on_press(Message::Freezer(FreezerMessage::BackToDashboard)),
            button(text("Freeze Again"))
                .on_press(Message::Freezer(FreezerMessage::Cancelled)),
        ]
        .spacing(8)
        .padding([8, 0]),
    );

    col.into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn centered(msg: &str) -> Element<'_, Message> {
    container(text(msg.to_owned()).size(14))
        .width(Length::Fill)
        .height(250)
        .center_x(Length::Fill)
        .center_y(250)
        .into()
}
