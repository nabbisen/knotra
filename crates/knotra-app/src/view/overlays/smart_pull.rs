//! 1. "Get latest safely" modal (Smart Pull) — RFC-037 Stage 5.
//!
//! Flow: Idle/Planning → Plan review → Running → Result
//! Plain wording at every step; technical detail behind "Show details".
//!
//! `modal_shell` replaced with `knotra_ui::widget::overlay::surface` — the
//! last of its five callers, so `mod.rs` deletes it in the same commit
//! (R6). `guided_button` call sites are untouched (D7/R12 — Stage 6
//! migrates them after `knotra-ui` grows a reason-carrying replacement).
//! Each phase's own footer content (`RetryPreparationFailed`,
//! `AwaitingConfirm`, `Done`) maps onto `surface()`'s `footer` parameter,
//! the same mapping Stages 2 and 4 used; phases with no completing action
//! (`Idle`/`Planning`, `RetryPreparing`, `FetchRunning`, `PullRunning`) pass
//! an empty `Space` instead.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, SMALL_BUTTON_HEIGHT, Tokens, centered_button_label, current_or, list,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};
use knotra_vcs::{
    ProjectId, model::operation::ProjectOperationOutcome, model::operation::SmartPullDisposition,
};

use crate::{
    message::{Message, SyncMessage},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "get_latest.close";
    pub const REVIEW_RETRY: &str = "get_latest.review_retry";
    pub const RETRY_FAILED_CLOSE: &str = "get_latest.retry_failed_close";
    pub const START: &str = "get_latest.start";
    pub const CANCEL: &str = "get_latest.cancel";
    pub const DETAILS: &str = "get_latest.details";
    pub const DONE_CLOSE: &str = "get_latest.done_close";
}

/// RFC-066 D2: order and entry for Get latest safely.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    use crate::state::sync::SyncPhase;

    let sync = &state.sync;
    let close_target = FocusTarget::control(focus_target::CLOSE);

    match &sync.phase {
        SyncPhase::Idle
        | SyncPhase::Planning
        | SyncPhase::RetryPreparing
        | SyncPhase::FetchRunning { .. }
        | SyncPhase::PullRunning { .. } => LayerFocus {
            order: Vec::new(),
            entry: None,
        },
        SyncPhase::RetryPreparationFailed => {
            let close_msg = Some(Message::Sync(SyncMessage::ModalClosed));
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
            let review_target = FocusTarget::control(focus_target::REVIEW_RETRY);
            let close_target_footer = FocusTarget::control(focus_target::RETRY_FAILED_CLOSE);
            LayerFocus {
                order: vec![
                    (close_target, close_msg.clone()),
                    (review_target.clone(), retry_message.clone()),
                    (close_target_footer.clone(), close_msg),
                ],
                entry: Some(if retry_message.is_some() {
                    review_target
                } else {
                    close_target_footer
                }),
            }
        }
        SyncPhase::AwaitingConfirm(plan) => {
            let close_msg = Some(Message::Sync(SyncMessage::ModalClosed));
            let mut order = vec![(close_target, close_msg.clone())];
            let mut first_choice = None;
            for entry in &plan.entries {
                if entry.is_dirty && !entry.has_conflict && entry.skip_reason.is_none() {
                    let fetch_target = FocusTarget::control_dynamic(format!(
                        "get_latest.{}.fetch_only",
                        entry.project_id
                    ));
                    if first_choice.is_none() {
                        first_choice = Some(fetch_target.clone());
                    }
                    order.push((
                        fetch_target,
                        (entry.disposition
                            != knotra_vcs::model::operation::SmartPullDisposition::FetchOnly)
                            .then_some(Message::Sync(SyncMessage::DispositionChanged(
                                entry.project_id.clone(),
                                knotra_vcs::model::operation::SmartPullDisposition::FetchOnly,
                            ))),
                    ));
                    let stash_target = FocusTarget::control_dynamic(format!(
                        "get_latest.{}.stash_and_pull",
                        entry.project_id
                    ));
                    order.push((
                        stash_target,
                        (entry.disposition
                            != knotra_vcs::model::operation::SmartPullDisposition::StashAndPull)
                            .then_some(Message::Sync(SyncMessage::DispositionChanged(
                                entry.project_id.clone(),
                                knotra_vcs::model::operation::SmartPullDisposition::StashAndPull,
                            ))),
                    ));
                }
            }
            let has_work = plan
                .entries
                .iter()
                .any(|e| !matches!(e.disposition, SmartPullDisposition::Excluded));
            let can_start = has_work && !state.operation_interlock.is_busy();
            order.push((
                FocusTarget::control(focus_target::START),
                can_start.then_some(Message::Sync(SyncMessage::ExecuteRequested)),
            ));
            let cancel_target = FocusTarget::control(focus_target::CANCEL);
            order.push((
                cancel_target.clone(),
                Some(Message::Sync(SyncMessage::Cancelled)),
            ));
            LayerFocus {
                order,
                // Never "Start": `ExecuteRequested` starts a change (D2).
                entry: Some(first_choice.unwrap_or(cancel_target)),
            }
        }
        SyncPhase::Done(_) => {
            let close_msg = Some(Message::Sync(SyncMessage::ModalClosed));
            let footer_close = FocusTarget::control(focus_target::DONE_CLOSE);
            LayerFocus {
                order: vec![
                    (close_target, close_msg.clone()),
                    (
                        FocusTarget::control(focus_target::DETAILS),
                        Some(Message::ToggleOpDetails),
                    ),
                    (footer_close.clone(), close_msg),
                ],
                entry: Some(footer_close),
            }
        }
    }
}

pub fn pull_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::sync::SyncPhase;

    let tokens = &state.theme.tokens;
    let sync = &state.sync;

    let (inner, footer): (Element<'_, Message>, Element<'_, Message>) = match &sync.phase {
        // ── Step 0: Planning (computing the plan) ────────────────────────
        SyncPhase::Idle | SyncPhase::Planning => (
            column![
                text(state.t("plain.get_latest.preparing"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                // RFC-071 Amendment A3 D14: read to understand — `body`.
                text(state.t("plain.get_latest.preparing_hint"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        SyncPhase::RetryPreparing => (
            column![
                text(state.t("plain.activity.retry_preparing"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                // RFC-071 Amendment A3 D14: read to understand — `body`.
                text(state.t("plain.get_latest.preparing_hint"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

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
            // RFC-066 D4: moved off `primary_maybe` onto `styled_button` —
            // `*_maybe` helpers cannot draw the focus outline.
            let footer = row![
                styled_button(
                    tokens,
                    state.t("plain.activity.review_retry"),
                    retry_message,
                    crate::app::ring(state, &FocusTarget::control(focus_target::REVIEW_RETRY)),
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.close"),
                    Some(Message::Sync(SyncMessage::ModalClosed)),
                    crate::app::ring(
                        state,
                        &FocusTarget::control(focus_target::RETRY_FAILED_CLOSE)
                    ),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    text(state.t("plain.activity.retry_prepare_failed"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                ]
                .into(),
                footer.into(),
            )
        }

        // ── Step 1: Review the plan ───────────────────────────────────────
        SyncPhase::AwaitingConfirm(plan) => {
            let mut rows: Vec<Element<'_, Message>> = Vec::new();

            // Header row. RFC-071 Amendment A3 D14: column headers are field
            // labels for the plan's own list content — `body`, matching the
            // cells they head. RFC-071 D8a: kept outside `list(...)` below
            // — a column heading, not a list item, the same reasoning that
            // keeps the dashboard's section header out of its own row-list.
            let header_row: Element<'_, Message> = row![
                text(state.t("plain.project"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(Length::FillPortion(3)),
                text(state.t("plain.what_will_happen"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(Length::FillPortion(2)),
                text(state.t("plain.note"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(Length::FillPortion(3)),
            ]
            .spacing(8)
            .into();

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
                                tokens,
                                &entry.project_id,
                                SmartPullDisposition::FetchOnly,
                                state.t("plain.get_latest.check_only"),
                                curr == &SmartPullDisposition::FetchOnly,
                                crate::app::ring(
                                    state,
                                    &FocusTarget::control_dynamic(format!(
                                        "get_latest.{}.fetch_only",
                                        entry.project_id
                                    ))
                                )
                            ),
                            pick_disposition_btn(
                                tokens,
                                &entry.project_id,
                                SmartPullDisposition::StashAndPull,
                                state.t("plain.get_latest.get_anyway"),
                                curr == &SmartPullDisposition::StashAndPull,
                                crate::app::ring(
                                    state,
                                    &FocusTarget::control_dynamic(format!(
                                        "get_latest.{}.stash_and_pull",
                                        entry.project_id
                                    ))
                                )
                            ),
                        ]
                        .spacing(4)
                        .into()
                    } else {
                        text(action_label)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2))
                            .into()
                    };

                rows.push(
                    row![
                        text(&entry.project_name)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(3)),
                        action_cell,
                        // RFC-071 Amendment A3 D14: the row's own note/reason
                        // — `body`, matching the rest of the row.
                        text(note)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(3)),
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
                        text(name)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(3)),
                        text(state.t("plain.activity.skipped"))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2)),
                        // RFC-071 Amendment A3 D14: a reason — `body`.
                        text(state.t(exclusion.reason.i18n_key()))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
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
                reasoned(
                    tokens,
                    state.t("plain.get_latest.start"),
                    can_start.then_some(Message::Sync(SyncMessage::ExecuteRequested)),
                    start_reason,
                    crate::app::ring(state, &FocusTarget::control(focus_target::START)),
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Sync(SyncMessage::Cancelled)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::CANCEL)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            // No inner `scrollable` around the plan list (unlike the
            // pre-migration `.height(Length::Fixed(240.0))` box) —
            // `surface()`'s own body scrollable now covers the whole body,
            // same reasoning as Stages 2-4 (review `132` §4).
            (
                column![
                    text(state.t("plain.get_latest.review_heading"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    header_row,
                    // RFC-071 D8a: the plan's own entries are a list of
                    // items — one border line between adjacent entries.
                    list(tokens, rows),
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
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
            (
                column![
                    text(progress_text)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                ]
                .spacing(8)
                .into(),
                Space::new().into(),
            )
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
                        text(icon)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::Fixed(20.0)),
                        text(&p.project_name)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2)),
                        text(msg)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2)),
                    ]
                    .spacing(8)
                    .into()
                })
                .collect();

            // Waiting rows for not-yet-started projects
            for entry in plan.entries.iter().skip(done) {
                result_rows.push(
                    row![
                        text("○")
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::Fixed(20.0)),
                        text(&entry.project_name)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2)),
                        // RFC-071 Amendment A3 D14: a status word, same
                        // column the above rows' own status (`body`) uses.
                        text(state.t("plain.waiting"))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(2)),
                    ]
                    .spacing(8)
                    .into(),
                );
            }

            let progress_label = format!("{} of {} done", done, total);
            // No inner `scrollable` here either — this phase has no footer
            // to justify the removal via Stage 4's "footer moved outside
            // the scroll region" argument, but the more basic reason still
            // applies unchanged: `surface()` provides exactly one bounded
            // scrollable for the whole body, and a second one nested inside
            // it would be the same redundant-scroll-region anti-pattern
            // Stage 2 first identified.
            (
                column![
                    text(state.t("plain.get_latest.working"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    // RFC-071 Amendment A3 D14: a progress status — `body`.
                    text(progress_label)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    // RFC-071 D8a: each project's own progress row, listed.
                    list(tokens, result_rows),
                ]
                .spacing(12)
                .into(),
                Space::new().into(),
            )
        }

        // ── Step 3: Result ────────────────────────────────────────────────
        SyncPhase::Done(result) => pull_result_view(tokens, state, result),
    };

    // R2/§2: gated on `PullRunning` only, unchanged from before this
    // migration. `FetchRunning` (the read-only phase) stays closable —
    // adding it to this guard would silently diverge from both the Escape
    // path (`focus_ops.rs`'s `smart_pull_is_running`) and the close
    // handler (`sync.rs`), which both already gate on `PullRunning` alone.
    let close_msg = if matches!(sync.phase, SyncPhase::PullRunning { .. }) {
        None
    } else {
        Some(Message::Sync(SyncMessage::ModalClosed))
    };

    surface(
        tokens,
        OverlayWidth::Large.resolve(state.window_width),
        state.t("plain.get_latest"),
        close_msg,
        crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE)),
        inner,
        footer,
    )
}

/// Render the result step for Get latest safely.
fn pull_result_view<'a>(
    tokens: &Tokens,
    state: &'a AppState,
    result: &'a crate::state::sync::SyncResult,
) -> (Element<'a, Message>, Element<'a, Message>) {
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

    let body_text = if fail == 0 {
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
                    // RFC-046 A1/D6/R10: map a persisted code through the
                    // catalog rather than rendering it raw — this overlay
                    // was showing `retry:not_in_active_workspace` verbatim
                    // beside a translated sentence from another writer
                    // before this fix. `None` (no reason recorded) keeps
                    // its own, unrelated fallback below.
                    pp.skip_reason
                        .as_deref()
                        .map(|reason| crate::view::skip_reason_display(state, reason))
                        .unwrap_or(state.t("plain.get_latest.skipped_row")),
                ),
            };

            let mut row_col = column![
                row![
                    text(icon)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .width(Length::Fixed(20.0)),
                    text(&pp.project_name)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .width(Length::FillPortion(2)),
                    text(msg)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .width(Length::FillPortion(2)),
                ]
                .spacing(8),
            ]
            .spacing(4);

            // Show commands under "Show details" if failed. RFC-071
            // Amendment A3 D14: moved to `body` — this is the diagnostic
            // text a user opened "Show details" to read in order to
            // understand the failure, not the incidental metadata (a
            // timestamp, a count, a keyboard hint) D14 reserves
            // `body_small` for.
            if pp.outcome == ProjectOperationOutcome::Failed
                && !pp.commands_executed.is_empty()
                && state.show_op_details
            {
                for cmd in &pp.commands_executed {
                    row_col = row_col.push(
                        text(format!("  {}", cmd))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                    );
                }
                if !pp.stderr.is_empty() {
                    row_col = row_col.push(
                        text(format!("  {}", pp.stderr.lines().next().unwrap_or("")))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
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
        styled_button(
            tokens,
            details_label,
            Some(Message::ToggleOpDetails),
            crate::app::ring(state, &FocusTarget::control(focus_target::DETAILS)),
            style::ghost,
        ),
        Space::new().width(Length::Fill),
        styled_button(
            tokens,
            state.t("action.close"),
            Some(Message::Sync(SyncMessage::ModalClosed)),
            crate::app::ring(state, &FocusTarget::control(focus_target::DONE_CLOSE)),
            style::ghost,
        ),
    ]
    .align_y(Alignment::Center);

    // No inner `scrollable` around the row list — same reasoning as
    // `AwaitingConfirm` above.
    let body = column![
        text(summary).size(snora::design::style::text::title_size(tokens)),
        text(body_text)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        // RFC-071 D8a: the results are a list of items.
        list(tokens, rows),
    ]
    .spacing(12);

    (body.into(), footer.into())
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (Stage 2),
/// `changelog.rs` (Stage 3), and `freezer.rs`/`context_switch.rs` (Stage 4)
/// use. `is_focused` is `crate::app::ring(state, &target)` at every call
/// site (RFC-066 D4).
///
/// The `RetryPreparationFailed` Close button this replaces had no explicit
/// `.height(BUTTON_HEIGHT).padding([0, 18])` in the pre-migration code,
/// unlike every other Close/Cancel button in this file — an existing
/// inconsistency, not a deliberate smaller size. `styled_button` applies
/// the file's own standard sizing uniformly, which normalizes that one
/// button rather than preserving its original slightly-different footprint.
fn styled_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    is_focused: bool,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    iced::widget::button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 18])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style_fn(&t, status)))
    .into()
}

/// Small inline toggle button for disposition choice in the plan view.
///
/// Restyled onto `knotra_ui::widget::current_or` (RFC-033 D4/RFC-034 R12) —
/// the selected disposition is a "you are here" indicator, not a disabled
/// control, and `current_or` exists precisely so that state renders at full
/// strength instead of being faded by iced's default `Status::Disabled`
/// styling for a button with no `on_press`. The pre-migration version relied
/// on that default fade to distinguish the selected option, which
/// `current_or`'s own doc comment identifies as exactly the problem it
/// fixes.
fn pick_disposition_btn<'a>(
    tokens: &Tokens,
    project_id: &'a ProjectId,
    disposition: SmartPullDisposition,
    label: &'a str,
    selected: bool,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let msg = (!selected).then_some(Message::Sync(SyncMessage::DispositionChanged(
        project_id.clone(),
        disposition,
    )));
    // Task 098 Amendment A1: `button_label_guard`'s literal-height check
    // also found this one (`.height(32.0)`, no `centered_button_label`) —
    // migrated onto `SMALL_BUTTON_HEIGHT` and `centered_button_label`,
    // same as the others. Kept as a hand-built `button(...)`, not
    // `reasoned`: `current_or`'s own per-button `selected` state has no
    // room in `reasoned`'s plain `fn(&Tokens, Status) -> Style` style
    // slot, which carries no extra state beyond the tokens and the
    // button's own interaction status.
    // RFC-071 Amendment A3 D14: a button label — `body`.
    iced::widget::button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(SMALL_BUTTON_HEIGHT)
    .padding([0, 10])
    .on_press_maybe(msg)
    .style(move |_theme, status| current_or(selected, &t, status, is_focused))
    .into()
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
