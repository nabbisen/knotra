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
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, Tokens, current_or,
    overlay::{OverlayWidth, surface},
    primary_maybe, reasoned, style,
};
use knotra_vcs::{
    ProjectId, model::operation::ProjectOperationOutcome, model::operation::SmartPullDisposition,
};

use crate::{
    message::{Message, SyncMessage},
    state::AppState,
};

pub fn pull_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::sync::SyncPhase;

    let tokens = &state.theme.tokens;
    let sync = &state.sync;

    let (inner, footer): (Element<'_, Message>, Element<'_, Message>) = match &sync.phase {
        // ── Step 0: Planning (computing the plan) ────────────────────────
        SyncPhase::Idle | SyncPhase::Planning => (
            column![
                text(state.t("plain.get_latest.preparing")).size(FONT_BODY),
                text(state.t("plain.get_latest.preparing_hint")).size(FONT_SMALL),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        SyncPhase::RetryPreparing => (
            column![
                text(state.t("plain.activity.retry_preparing")).size(FONT_BODY),
                text(state.t("plain.get_latest.preparing_hint")).size(FONT_SMALL),
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
            // `reason` was always `None` at this call site (no path here
            // ever supplied one) — the plain `primary_maybe` constructor is
            // the better target than `reasoned`, per the handoff's §1b note
            // that not every site needs the reason-carrying form.
            let footer = row![
                primary_maybe(
                    tokens,
                    state.t("plain.activity.review_retry"),
                    retry_message
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.close"),
                    Some(Message::Sync(SyncMessage::ModalClosed)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![text(state.t("plain.activity.retry_prepare_failed")).size(FONT_BODY)]
                    .into(),
                footer.into(),
            )
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
                                tokens,
                                &entry.project_id,
                                SmartPullDisposition::FetchOnly,
                                state.t("plain.get_latest.check_only"),
                                curr == &SmartPullDisposition::FetchOnly
                            ),
                            pick_disposition_btn(
                                tokens,
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
                reasoned(
                    tokens,
                    state.t("plain.get_latest.start"),
                    can_start.then_some(Message::Sync(SyncMessage::ExecuteRequested)),
                    start_reason,
                    false,
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Sync(SyncMessage::Cancelled)),
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
                    text(state.t("plain.get_latest.review_heading")).size(FONT_BODY),
                    column(rows).spacing(6),
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
                column![text(progress_text).size(FONT_BODY),]
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
            // No inner `scrollable` here either — this phase has no footer
            // to justify the removal via Stage 4's "footer moved outside
            // the scroll region" argument, but the more basic reason still
            // applies unchanged: `surface()` provides exactly one bounded
            // scrollable for the whole body, and a second one nested inside
            // it would be the same redundant-scroll-region anti-pattern
            // Stage 2 first identified.
            (
                column![
                    text(state.t("plain.get_latest.working")).size(FONT_BODY),
                    text(progress_label).size(FONT_SMALL),
                    column(result_rows).spacing(6),
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
        OverlayWidth::Large,
        state.t("plain.get_latest"),
        close_msg,
        false,
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
        styled_button(
            tokens,
            details_label,
            Some(Message::ToggleOpDetails),
            style::ghost,
        ),
        Space::new().width(Length::Fill),
        styled_button(
            tokens,
            state.t("action.close"),
            Some(Message::Sync(SyncMessage::ModalClosed)),
            style::ghost,
        ),
    ]
    .align_y(Alignment::Center);

    // No inner `scrollable` around the row list — same reasoning as
    // `AwaitingConfirm` above.
    let body = column![
        text(summary).size(FONT_BODY + 2.0),
        text(body_text).size(FONT_BODY),
        column(rows).spacing(8),
    ]
    .spacing(12);

    (body.into(), footer.into())
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (Stage 2),
/// `changelog.rs` (Stage 3), and `freezer.rs`/`context_switch.rs` (Stage 4)
/// use. `is_focused` is always `false`: no real focus-order wiring exists
/// or is permitted for this overlay this stage (R3 forbids `app/`/
/// `state/`).
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
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    iced::widget::button(text(label).size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 18])
        .on_press_maybe(on_press)
        .style(move |_theme, status| style::with_focus_ring(&t, false, style_fn(&t, status)))
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
) -> Element<'a, Message> {
    let t = tokens.clone();
    let msg = (!selected).then_some(Message::Sync(SyncMessage::DispositionChanged(
        project_id.clone(),
        disposition,
    )));
    iced::widget::button(text(label).size(FONT_SMALL))
        .height(32.0)
        .padding([0, 10])
        .on_press_maybe(msg)
        .style(move |_theme, status| current_or(selected, &t, status, false))
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
