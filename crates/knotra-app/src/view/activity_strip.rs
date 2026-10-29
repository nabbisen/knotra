//! Activity strip and project-removal undo snackbar.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, text},
};

use knotra_ui::widget::{SMALL_BUTTON_HEIGHT, centered_button_label, style};

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
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
                Space::new().width(Length::Fill),
                // Task 100 §3: styled like every other knotra button —
                // `style::secondary`/`style::ghost` plus `with_focus_ring`,
                // the same shape `freezer.rs::styled_button` uses. Neither
                // button is a focus target (the snackbar sits outside the
                // `Layer`/`layer_focus` system entirely), so `is_focused` is
                // hardcoded `false` rather than left out — the same choice
                // `freezer.rs`'s own `styled_button` doc comment makes for
                // this stage.
                button(centered_button_label(
                    text(state.t("plain.undo.undo"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens))
                ))
                .height(SMALL_BUTTON_HEIGHT)
                .padding([0, 12])
                .on_press(Message::Workspace(WorkspaceMessage::UndoRemoval))
                .style({
                    let t = tokens.clone();
                    move |_theme, status| {
                        style::with_focus_ring(&t, false, style::secondary(&t, status))
                    }
                }),
                button(centered_button_label(
                    text(state.t("plain.undo.dismiss"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens))
                ))
                .height(SMALL_BUTTON_HEIGHT)
                .padding([0, 12])
                .on_press(Message::Workspace(WorkspaceMessage::DismissUndoSnackbar))
                .style({
                    let t = tokens.clone();
                    move |_theme, status| {
                        style::with_focus_ring(&t, false, style::ghost(&t, status))
                    }
                }),
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
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens
                            )),
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
            let icon = if failed == 0 {
                "ⓘ"
            } else if succeeded == 0 {
                "✗"
            } else {
                "⚠"
            };
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
                "{}  {}: {}",
                icon,
                super::operation_kind_label(state, &log.result.kind),
                segments.join(", "),
            );

            let mut content = row![
                text(label)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
                    content = content.push(
                        button(
                            text(label)
                                .size(snora::design::style::text::body_small_size(tokens))
                                .line_height(snora::design::style::text::body_small_line_height(
                                    tokens,
                                )),
                        )
                        .on_press_maybe(retry),
                    );
                    if state.operation_interlock.is_busy() {
                        content = content.push(
                            text(state.t("plain.activity.busy"))
                                .size(snora::design::style::text::body_small_size(tokens))
                                .line_height(snora::design::style::text::body_small_line_height(
                                    tokens,
                                )),
                        );
                    }
                }
                RetryAvailability::Unavailable(reason) => {
                    content = content.push(
                        text(state.t(reason.i18n_key()))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                }
                RetryAvailability::NotApplicable => {}
            }

            content = content.push(
                button(
                    text(state.t("plain.activity.details"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                )
                .on_press(Message::Activity(ActivityMessage::DetailsRequested {
                    operation_id: log.result.operation_id.clone(),
                })),
            );
            Some(
                container(content.padding([4, 12]))
                    .width(Length::Fill)
                    .into(),
            )
        }
    }
}
