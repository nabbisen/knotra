//! Activity strip and project-removal undo snackbar.
//!
//! RFC-071 Amendment A3 D14: every text size here is `body` — a status
//! message, a button label, or a reason, never incidental metadata.

use iced::{
    Alignment, Element, Length,
    widget::{Space, container, row, text},
};

use knotra_ui::widget::{
    body_badge_diameter, ghost_control, secondary_control, secondary_control_maybe, status_badge,
};

use crate::{
    message::{ActivityMessage, Message, WorkspaceMessage},
    state::{ActivityRetryAction, AppState, LatestOpState, RetryAvailability},
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let tokens = &state.theme.tokens;
    if let Some(removal) = &state.recent_removal {
        let msg = format!(
            "{} {}.",
            state.t("plain.undo.removed_prefix"),
            removal.project.name,
        );
        let snackbar = container(
            row![
                text(msg)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                Space::new().width(Length::Fill),
                // RFC-075 stage 3 §1: real snora controls now, each its
                // own Tab stop, inside the page's own wrap.
                secondary_control(
                    tokens,
                    state.t("plain.undo.undo"),
                    Message::Workspace(WorkspaceMessage::UndoRemoval),
                    crate::view::ids::activity_strip::undo(),
                ),
                ghost_control(
                    tokens,
                    state.t("plain.undo.dismiss"),
                    Message::Workspace(WorkspaceMessage::DismissUndoSnackbar),
                    crate::view::ids::activity_strip::dismiss(),
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([4, 12]),
        )
        .width(Length::Fill);
        return Some(snackbar.into());
    }

    match &state.activity.latest {
        LatestOpState::Idle => None,
        LatestOpState::Running {
            label, done, total, ..
        } => {
            let progress_label = if *total > 0 {
                format!("⟳  {}  {}/{}", label, done, total)
            } else {
                format!("⟳  {}", label)
            };
            Some(
                container(
                    row![
                        text(progress_label)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                        Space::new().width(Length::Fill)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .padding([4, 12]),
                )
                .width(Length::Fill)
                .into(),
            )
        }
        LatestOpState::Completed { log, retry } => {
            let succeeded = log.result.successful_projects().len();
            let failed = log.result.failed_projects().len();
            let skipped = log.result.skipped_projects().len();
            // RFC-073 handoff Amendment A3 §1: `history`'s own aggregate
            // mapping (success / partial, `warning` / failure, with the
            // rollback cases a plain succeeded/failed/skipped count would
            // miss), not a second, narrower guess kept beside it.
            let (tone, glyph) = super::history::aggregate_badge(&log.result);
            // RFC-035 R16/Handoff 030 §4.2: `failed` is now guarded the same
            // way `skipped` already was — a fully successful run no longer
            // prints "0 failed" (`060` finding 4). `succeeded` stays
            // unconditional: it is the segment that anchors the count, and
            // a run with zero successes needs "0 succeeded" to say so.
            // Building the segments as a list and joining them (rather than
            // conditionally appending onto a base string) is what keeps the
            // separator from dangling when a segment is omitted.
            let mut segments = vec![format!(
                "{} {}",
                succeeded,
                state.t("plain.activity.succeeded")
            )];
            if failed > 0 {
                segments.push(format!("{} {}", failed, state.t("plain.activity.failed")));
            }
            if skipped > 0 {
                segments.push(format!("{} {}", skipped, state.t("plain.activity.skipped")));
            }
            let label = format!(
                "{}: {}",
                super::operation_kind_label(state, &log.result.kind),
                segments.join(", "),
            );

            let mut content = row![
                // RFC-073 D9/handoff Amendment A3 §1: `status_badge`, not a
                // plain-text glyph baked into the label string.
                row![
                    status_badge(tokens, tone, &glyph, body_badge_diameter(tokens)),
                    text(label)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                Space::new().width(Length::Fill)
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            match retry {
                RetryAvailability::Available(action) => {
                    let (label, source_operation_id) = match action {
                        ActivityRetryAction::FetchFailed {
                            source_operation_id,
                            ..
                        } => (
                            state.t("plain.activity.retry_failed_fetches"),
                            source_operation_id,
                        ),
                        ActivityRetryAction::ReviewSmartPull {
                            source_operation_id,
                            ..
                        } => (state.t("plain.activity.review_retry"), source_operation_id),
                    };
                    let retry = (!state.operation_interlock.is_busy()).then_some(
                        Message::Activity(ActivityMessage::RetryRequested {
                            source_operation_id: source_operation_id.clone(),
                        }),
                    );
                    // RFC-075 stage 3 §1: a real snora control now — not
                    // named in the handoff's own Undo/Dismiss/Details
                    // table (see `ids::activity_strip::retry`'s own doc
                    // comment).
                    content = content.push(secondary_control_maybe(
                        tokens,
                        label,
                        retry,
                        crate::view::ids::activity_strip::retry(),
                    ));
                    if state.operation_interlock.is_busy() {
                        content = content.push(
                            text(state.t("plain.activity.busy"))
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens)),
                        );
                    }
                }
                RetryAvailability::Unavailable(reason) => {
                    content = content.push(
                        text(state.t(reason.i18n_key()))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                    );
                }
                RetryAvailability::NotApplicable => {}
            }

            // RFC-071 D5: "Details" becomes `secondary` — the retry button
            // beside it (when shown) is the row's one mutating action.
            // Not a focus target today (no focus-order wiring exists for
            // the activity strip), so `false` is passed explicitly.
            // RFC-073 D8: `SMALL_BUTTON_HEIGHT`, matching the retry button.
            content = content.push(secondary_control(
                tokens,
                state.t("plain.activity.details"),
                Message::Activity(ActivityMessage::DetailsRequested {
                    operation_id: log.result.operation_id.clone(),
                }),
                crate::view::ids::activity_strip::details(),
            ));
            Some(
                container(content.padding([4, 12]))
                    .width(Length::Fill)
                    .into(),
            )
        }
    }
}
