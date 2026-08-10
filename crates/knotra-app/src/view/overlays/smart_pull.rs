//! 1. "Get latest safely" modal (Smart Pull) — RFC-037 Stage 1.
//!
//! Flow: Idle/Planning → Plan review → Running → Result
//! Plain wording at every step; technical detail behind "Show details".

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, row, scrollable, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button};
use knotra_vcs::{
    ProjectId, model::operation::ProjectOperationOutcome, model::operation::SmartPullDisposition,
};

use super::modal_shell;
use crate::{
    message::{Message, SyncMessage},
    state::AppState,
};

pub fn pull_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::sync::SyncPhase;

    let sync = &state.sync;

    let inner: Element<'_, Message> = match &sync.phase {
        // ── Step 0: Planning (computing the plan) ────────────────────────
        SyncPhase::Idle | SyncPhase::Planning => column![
            text(state.t("plain.get_latest.preparing")).size(FONT_BODY),
            text(state.t("plain.get_latest.preparing_hint")).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),

        SyncPhase::RetryPreparing => column![
            text(state.t("plain.activity.retry_preparing")).size(FONT_BODY),
            text(state.t("plain.get_latest.preparing_hint")).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),

        SyncPhase::RetryPreparationFailed => {
            let retry_message = match &state.activity.latest {
                crate::state::LatestOpState::Completed {
                    log,
                    retry:
                        crate::state::RetryAvailability::Available(
                            crate::state::ActivityRetryAction::ReviewSmartPull { .. },
                        ),
                } => Some(Message::Activity(
                    crate::message::ActivityMessage::RetryRequested {
                        source_operation_id: log.result.operation_id.clone(),
                    },
                )),
                _ => None,
            };
            column![
                text(state.t("plain.activity.retry_prepare_failed")).size(FONT_BODY),
                row![
                    guided_button(state.t("plain.activity.review_retry"), retry_message, None,),
                    Space::new().width(Length::Fill),
                    button(text(state.t("action.close")).size(FONT_BODY))
                        .on_press(Message::Sync(SyncMessage::ModalClosed)),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .into()
        }

        // ── Step 1: Review the plan ───────────────────────────────────────
        SyncPhase::AwaitingConfirm(plan) => {
            let mut rows: Vec<Element<'_, Message>> = Vec::new();

            // Header row
            rows.push(
                row![
                    text(state.t("plain.project"))
                        .size(FONT_SMALL)
                        .width(Length::FillPortion(3)),
                    text(state.t("plain.what_will_happen"))
                        .size(FONT_SMALL)
                        .width(Length::FillPortion(2)),
                    text(state.t("plain.note"))
                        .size(FONT_SMALL)
                        .width(Length::FillPortion(3)),
                ]
                .spacing(8)
                .into(),
            );

            for entry in &plan.entries {
                let (action_label, note) = disposition_plain(
                    state,
                    entry.disposition.clone(),
                    entry.is_dirty,
                    entry.has_conflict,
                    entry.skip_reason.as_ref().map(|reason| reason.i18n_key()),
                );

                // Disposition override buttons for dirty (non-conflicted) projects
                let action_cell: Element<'_, Message> =
                    if entry.is_dirty && !entry.has_conflict && entry.skip_reason.is_none() {
                        let curr = &entry.disposition;
                        row![
                            pick_disposition_btn(
                                state,
                                &entry.project_id,
                                SmartPullDisposition::FetchOnly,
                                state.t("plain.get_latest.check_only"),
                                curr == &SmartPullDisposition::FetchOnly
                            ),
                            pick_disposition_btn(
                                state,
                                &entry.project_id,
                                SmartPullDisposition::StashAndPull,
                                state.t("plain.get_latest.get_anyway"),
                                curr == &SmartPullDisposition::StashAndPull
                            ),
                        ]
                        .spacing(4)
                        .into()
                    } else {
                        text(action_label)
                            .size(FONT_BODY)
                            .width(Length::FillPortion(2))
                            .into()
                    };

                rows.push(
                    row![
                        text(&entry.project_name)
                            .size(FONT_BODY)
                            .width(Length::FillPortion(3)),
                        action_cell,
                        text(note).size(FONT_SMALL).width(Length::FillPortion(3)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                );
            }

            for exclusion in &sync.retry_exclusions {
                let name = state
                    .workspace
                    .as_ref()
                    .and_then(|workspace| {
                        workspace
                            .projects
                            .iter()
                            .find(|project| project.id == exclusion.project_id)
                    })
                    .map(|project| project.name.as_str())
                    .unwrap_or_else(|| state.t("plain.project"));
                rows.push(
                    row![
                        text(name).size(FONT_BODY).width(Length::FillPortion(3)),
                        text(state.t("plain.activity.skipped"))
                            .size(FONT_BODY)
                            .width(Length::FillPortion(2)),
                        text(state.t(exclusion.reason.i18n_key()))
                            .size(FONT_SMALL)
                            .width(Length::FillPortion(3)),
                    ]
                    .spacing(8)
                    .into(),
                );
            }

            let has_work = plan
                .entries
                .iter()
                .any(|e| !matches!(e.disposition, SmartPullDisposition::Excluded));
            let can_start = has_work && !state.operation_interlock.is_busy();

            let start_reason = if can_start {
                None
            } else if state.operation_interlock.is_busy() {
                Some(state.t("plain.activity.busy"))
            } else {
                Some(state.t("plain.disabled.choose_one"))
            };

            let footer = row![
                guided_button(
                    state.t("plain.get_latest.start"),
                    can_start.then_some(Message::Sync(SyncMessage::ExecuteRequested)),
                    start_reason,
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Sync(SyncMessage::Cancelled)),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.get_latest.review_heading")).size(FONT_BODY),
                scrollable(column(rows).spacing(6)).height(Length::Fixed(240.0)),
                footer,
            ]
            .spacing(12)
            .into()
        }

        // ── Step 2: In progress ───────────────────────────────────────────
        SyncPhase::FetchRunning { done, total, .. } => {
            let progress_text = format!(
                "{} — {} {} {}",
                state.t("plain.get_latest.working"),
                done,
                state.t("plain.of"),
                total
            );
            column![text(progress_text).size(FONT_BODY),]
                .spacing(8)
                .into()
        }

        SyncPhase::PullRunning {
            completed, plan, ..
        } => {
            let total = plan.entries.len();
            let done = completed.len();
            let mut result_rows: Vec<Element<'_, Message>> = completed
                .iter()
                .map(|p| {
                    let (icon, msg) = match p.result.effective_outcome() {
                        ProjectOperationOutcome::Succeeded => {
                            ("✓", state.t("plain.get_latest.done_row"))
                        }
                        ProjectOperationOutcome::Failed => {
                            ("!", state.t("plain.get_latest.needs_help_row"))
                        }
                        ProjectOperationOutcome::Skipped => {
                            ("-", state.t("plain.get_latest.skipped_row"))
                        }
                    };
                    row![
                        text(icon).size(FONT_BODY).width(Length::Fixed(20.0)),
                        text(&p.project_name)
                            .size(FONT_BODY)
                            .width(Length::FillPortion(2)),
                        text(msg).size(FONT_BODY).width(Length::FillPortion(2)),
                    ]
                    .spacing(8)
                    .into()
                })
                .collect();

            // Waiting rows for not-yet-started projects
            for entry in plan.entries.iter().skip(done) {
                result_rows.push(
                    row![
                        text("○").size(FONT_BODY).width(Length::Fixed(20.0)),
                        text(&entry.project_name)
                            .size(FONT_BODY)
                            .width(Length::FillPortion(2)),
                        text(state.t("plain.waiting"))
                            .size(FONT_SMALL)
                            .width(Length::FillPortion(2)),
                    ]
                    .spacing(8)
                    .into(),
                );
            }

            let progress_label = format!("{} of {} done", done, total);
            column![
                text(state.t("plain.get_latest.working")).size(FONT_BODY),
                text(progress_label).size(FONT_SMALL),
                scrollable(column(result_rows).spacing(6)).height(Length::Fixed(240.0)),
            ]
            .spacing(12)
            .into()
        }

        // ── Step 3: Result ────────────────────────────────────────────────
        SyncPhase::Done(result) => pull_result_view(state, result),
    };

    let close_msg = if matches!(sync.phase, SyncPhase::PullRunning { .. }) {
        None
    } else {
        Some(Message::Sync(SyncMessage::ModalClosed))
    };

    modal_shell(state.t("plain.get_latest"), close_msg, inner)
}

/// Render the result step for Get latest safely.
fn pull_result_view<'a>(
    state: &'a AppState,
    result: &'a crate::state::sync::SyncResult,
) -> Element<'a, Message> {
    let ok = result.success_count();
    let fail = result.fail_count();
    let skipped = result.skipped_count();

    let summary = if fail == 0 && skipped == 0 {
        format!(
            "{} {} {}.",
            state.t("plain.get_latest.all_done_prefix"),
            ok,
            state.t("plain.get_latest.all_done_suffix")
        )
    } else if fail == 0 {
        format!(
            "{} {}. {} {}.",
            ok,
            state.t("plain.get_latest.done_count"),
            skipped,
            state.t("plain.get_latest.skipped_count")
        )
    } else {
        format!(
            "{} {}. {} {}. {} {}.",
            ok,
            state.t("plain.get_latest.done_count"),
            fail,
            state.t("plain.get_latest.needs_help_count"),
            skipped,
            state.t("plain.get_latest.skipped_count")
        )
    };

    let body = if fail == 0 {
        state.t("plain.no_next_step")
    } else {
        state.t("plain.get_latest.review_help_rows")
    };

    // Per-project result rows
    let rows: Vec<Element<'_, Message>> = result
        .per_project
        .iter()
        .map(|pp| {
            let (icon, msg) = match pp.outcome {
                ProjectOperationOutcome::Succeeded => ("✓", state.t("plain.get_latest.done_row")),
                ProjectOperationOutcome::Failed => ("!", state.t("plain.needs_help")),
                ProjectOperationOutcome::Skipped => (
                    "-",
                    pp.skip_reason
                        .as_deref()
                        .unwrap_or(state.t("plain.get_latest.skipped_row")),
                ),
            };

            let mut row_col = column![
                row![
                    text(icon).size(FONT_BODY).width(Length::Fixed(20.0)),
                    text(&pp.project_name)
                        .size(FONT_BODY)
                        .width(Length::FillPortion(2)),
                    text(msg).size(FONT_BODY).width(Length::FillPortion(2)),
                ]
                .spacing(8),
            ]
            .spacing(4);

            // Show commands under "Show details" if failed
            if pp.outcome == ProjectOperationOutcome::Failed
                && !pp.commands_executed.is_empty()
                && state.show_op_details
            {
                for cmd in &pp.commands_executed {
                    row_col = row_col.push(text(format!("  {}", cmd)).size(FONT_SMALL));
                }
                if !pp.stderr.is_empty() {
                    row_col = row_col.push(
                        text(format!("  {}", pp.stderr.lines().next().unwrap_or("")))
                            .size(FONT_SMALL),
                    );
                }
            }

            row_col.into()
        })
        .collect();

    let details_label = if state.show_op_details {
        state.t("plain.hide_details")
    } else {
        state.t("plain.show_details")
    };

    let footer = row![
        button(text(details_label).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::ToggleOpDetails),
        Space::new().width(Length::Fill),
        button(text(state.t("action.close")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Sync(SyncMessage::ModalClosed)),
    ]
    .align_y(Alignment::Center);

    column![
        text(summary).size(FONT_BODY + 2.0),
        text(body).size(FONT_BODY),
        scrollable(column(rows).spacing(8)).height(Length::Fixed(240.0)),
        footer,
    ]
    .spacing(12)
    .into()
}

/// Small inline toggle button for disposition choice in the plan view.
fn pick_disposition_btn<'a>(
    _state: &'a AppState,
    project_id: &'a ProjectId,
    disposition: SmartPullDisposition,
    label: &'a str,
    selected: bool,
) -> Element<'a, Message> {
    let btn = button(text(label).size(FONT_SMALL))
        .height(32.0)
        .padding([0, 10]);
    let btn: Element<'a, Message> = if selected {
        btn.into() // visually "active" — iced styling applies
    } else {
        btn.on_press(Message::Sync(SyncMessage::DispositionChanged(
            project_id.clone(),
            disposition,
        )))
        .into()
    };
    btn
}

/// Map a `SmartPullDisposition` to plain-language action label + contextual note.
fn disposition_plain(
    state: &AppState,
    d: SmartPullDisposition,
    is_dirty: bool,
    has_conflict: bool,
    skip_reason_key: Option<&'static str>,
) -> (&'static str, &'static str) {
    match d {
        SmartPullDisposition::Pull => (state.t("plain.get_latest.action_get"), ""),
        SmartPullDisposition::FetchOnly => (
            state.t("plain.get_latest.action_check"),
            if is_dirty {
                state.t("plain.get_latest.note_unsaved")
            } else {
                ""
            },
        ),
        SmartPullDisposition::StashAndPull => (
            state.t("plain.get_latest.action_get_anyway"),
            state.t("plain.get_latest.note_save_restore"),
        ),
        SmartPullDisposition::Excluded => (
            state.t("plain.get_latest.action_skip"),
            if let Some(key) = skip_reason_key {
                state.t(key)
            } else if has_conflict {
                state.t("plain.get_latest.note_needs_choice")
            } else {
                ""
            },
        ),
    }
}
