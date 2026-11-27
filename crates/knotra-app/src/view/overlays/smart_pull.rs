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
    widget::{Space, column, container, row, text},
};

use knotra_ui::widget::{
    SMALL_BUTTON_HEIGHT, Tokens, body_badge_diameter, centered_button_label, current_or_control,
    ghost_control, list,
    overlay::{OverlayWidth, surface},
    primary_control_maybe, status_badge,
};
use knotra_vcs::{
    ProjectId, model::operation::ProjectOperationOutcome, model::operation::SmartPullDisposition,
    model::operation::SmartPullPlan,
};

use crate::{
    message::{Message, SyncMessage},
    state::{AppState, dashboard::badge_for_operation_outcome},
};

/// RFC-075 stage 3 §3: Get latest safely's own entry per phase.
/// `ExecuteRequested` starts a change, so "Start" is never the entry — the
/// first dirty project's own disposition choice stands in for it, falling
/// back to Cancel when there is none. `Idle`/`Planning`/`RetryPreparing`/
/// `FetchRunning`/`PullRunning` have no controls.
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    use crate::state::sync::SyncPhase;

    let sync = &state.sync;
    match &sync.phase {
        SyncPhase::Idle
        | SyncPhase::Planning
        | SyncPhase::RetryPreparing
        | SyncPhase::FetchRunning { .. }
        | SyncPhase::PullRunning { .. } => None,
        SyncPhase::RetryPreparationFailed => {
            let retry_available = matches!(
                &state.activity.latest,
                crate::state::LatestOpState::Completed {
                    retry: crate::state::RetryAvailability::Available(
                        crate::state::ActivityRetryAction::ReviewSmartPull { .. },
                    ),
                    ..
                }
            );
            Some(if retry_available {
                crate::view::ids::smart_pull::review_retry()
            } else {
                crate::view::ids::smart_pull::retry_failed_close()
            })
        }
        SyncPhase::AwaitingConfirm(plan) => {
            let first_choice = plan
                .entries
                .iter()
                .enumerate()
                .find(|(_, e)| e.is_dirty && !e.has_conflict && e.skip_reason.is_none())
                .map(|(i, _)| crate::view::ids::smart_pull::check_only(i));
            Some(first_choice.unwrap_or_else(crate::view::ids::smart_pull::cancel))
        }
        SyncPhase::Done(_) => Some(crate::view::ids::smart_pull::done_close()),
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
                        source_operation_id: log.result.operation_id,
                    },
                )),
                _ => None,
            };
            let footer = row![
                primary_control_maybe(
                    tokens,
                    state.t("plain.activity.review_retry"),
                    retry_message,
                    crate::view::ids::smart_pull::review_retry(),
                ),
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.close"),
                    Message::Sync(SyncMessage::ModalClosed),
                    crate::view::ids::smart_pull::retry_failed_close(),
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
            ]
            .spacing(8)
            .into();

            for (i, entry) in plan.entries.iter().enumerate() {
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
                                crate::view::ids::smart_pull::check_only(i),
                            ),
                            pick_disposition_btn(
                                tokens,
                                &entry.project_id,
                                SmartPullDisposition::StashAndPull,
                                state.t("plain.get_latest.get_anyway"),
                                curr == &SmartPullDisposition::StashAndPull,
                                crate::view::ids::smart_pull::get_anyway(i),
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

                let mut entry_col = column![
                    row![
                        text(&entry.project_name)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::FillPortion(3)),
                        action_cell,
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(2);
                if !note.is_empty() {
                    // Task 118: a note gets its own line, spanning the row's
                    // full width (not squeezed into a narrow column) —
                    // Japanese has no spaces to break at, so a narrow
                    // column broke mid-word. Aligned with the project name
                    // above it by construction: both start at this column's
                    // own left edge.
                    // RFC-071 Amendment A3 D14: the row's own note/reason —
                    // `body`, matching the rest of the row.
                    entry_col = entry_col.push(
                        text(note)
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::Fill),
                    );
                }
                rows.push(entry_col.into());
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
                    column![
                        row![
                            text(name)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(3)),
                            text(state.t("plain.activity.skipped"))
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(2)),
                        ]
                        .spacing(8),
                        // Task 118: the reason gets its own line, same as a
                        // plan entry's note above.
                        // RFC-071 Amendment A3 D14: a reason — `body`.
                        text(state.t(exclusion.reason.i18n_key()))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .width(Length::Fill),
                    ]
                    .spacing(2)
                    .into(),
                );
            }

            let has_work = plan
                .entries
                .iter()
                .any(|e| !matches!(e.disposition, SmartPullDisposition::Excluded));
            let can_start = has_work && !state.operation_interlock.is_busy();

            // RFC-074 Stage 1 "Decided": every project here was chosen
            // (this is a plan, not an empty selection) — `plain.disabled.
            // choose_one` ("Choose at least one project.") would be wrong,
            // so the all-skipped case gets its own reason.
            let start_reason = if can_start {
                None
            } else if state.operation_interlock.is_busy() {
                Some(state.t("plain.activity.busy"))
            } else {
                Some(state.t("plain.get_latest.all_skipped"))
            };

            let start_on_press = can_start.then_some(Message::Sync(SyncMessage::ExecuteRequested));
            let start_btn = primary_control_maybe(
                tokens,
                state.t("plain.get_latest.start"),
                start_on_press.clone(),
                crate::view::ids::smart_pull::start(),
            );
            let start: Element<'_, Message> = if start_on_press.is_none() && start_reason.is_some()
            {
                column![
                    start_btn,
                    // RFC-071 Amendment A3 D14: a reason — `body`.
                    text(start_reason.unwrap_or_default())
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                ]
                .spacing(6)
                .into()
            } else {
                start_btn
            };
            let footer = row![
                start,
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.cancel"),
                    Message::Sync(SyncMessage::Cancelled),
                    crate::view::ids::smart_pull::cancel(),
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
                    // RFC-074 Stage 1 D2: how many projects, and which,
                    // before "Start".
                    plan_summary_line(state, tokens, plan),
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
                    // RFC-073 D9: `status_badge`, not a plain-text glyph.
                    let (tone, glyph) = badge_for_operation_outcome(p.result.effective_outcome());
                    let msg = match p.result.effective_outcome() {
                        ProjectOperationOutcome::Succeeded => state.t("plain.get_latest.done_row"),
                        ProjectOperationOutcome::Failed => {
                            state.t("plain.get_latest.needs_help_row")
                        }
                        ProjectOperationOutcome::Skipped => state.t("plain.get_latest.skipped_row"),
                    };
                    row![
                        container(status_badge(
                            tokens,
                            tone,
                            &glyph,
                            body_badge_diameter(tokens)
                        ))
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

    // RFC-075 stage 3 §2: every dirty entry's pair of disposition toggles
    // self-disables the one just pressed while re-enabling its sibling —
    // both of a pair join `reserved` so focus keeps the row rather than
    // jumping to a neighbour (A2 §3).
    let reserved: Vec<iced::widget::Id> = match &sync.phase {
        SyncPhase::AwaitingConfirm(plan) => plan
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_dirty && !e.has_conflict && e.skip_reason.is_none())
            .flat_map(|(i, _)| {
                [
                    crate::view::ids::smart_pull::check_only(i),
                    crate::view::ids::smart_pull::get_anyway(i),
                ]
            })
            .collect(),
        _ => Vec::new(),
    };

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Large.resolve(state.window_width),
            state.t("plain.get_latest"),
            close_msg,
            crate::view::ids::smart_pull::close(),
            inner,
            footer,
        ),
        &reserved,
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
            // RFC-073 D9: `status_badge`, not a plain-text glyph.
            let (tone, glyph) = badge_for_operation_outcome(pp.outcome.clone());
            let msg = match pp.outcome {
                ProjectOperationOutcome::Succeeded => state.t("plain.get_latest.done_row"),
                ProjectOperationOutcome::Failed => state.t("plain.needs_help"),
                ProjectOperationOutcome::Skipped =>
                // RFC-046 A1/D6/R10: map a persisted code through the
                // catalog rather than rendering it raw — this overlay
                // was showing `retry:not_in_active_workspace` verbatim
                // beside a translated sentence from another writer
                // before this fix. `None` (no reason recorded) keeps
                // its own, unrelated fallback below.
                {
                    pp.skip_reason
                        .as_deref()
                        .map(|reason| crate::view::skip_reason_display(state, reason))
                        .unwrap_or(state.t("plain.get_latest.skipped_row"))
                }
            };

            let mut row_col = column![
                row![
                    container(status_badge(
                        tokens,
                        tone,
                        &glyph,
                        body_badge_diameter(tokens)
                    ))
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
        ghost_control(
            tokens,
            details_label,
            Message::ToggleOpDetails,
            crate::view::ids::smart_pull::show_details(),
        ),
        Space::new().width(Length::Fill),
        ghost_control(
            tokens,
            state.t("action.close"),
            Message::Sync(SyncMessage::ModalClosed),
            crate::view::ids::smart_pull::done_close(),
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

/// Small inline toggle control for disposition choice in the plan view —
/// a real snora control (RFC-075 stage 3 §2), its own Tab stop, so focus
/// follows the row it belongs to even as the plan's own rows move (§7's
/// own test). The already-selected disposition has no press (there is
/// nothing to change to) and so is briefly not a Tab stop, same as every
/// other self-disabling control (A2 §3) — its own call site puts both of
/// a dirty entry's ids in the window's `reserved` list for exactly this,
/// since pressing one disables itself and re-enables the other.
/// [`current_or_control`]'s own doc comment gives the "you are here" style
/// reasoning.
fn pick_disposition_btn<'a>(
    tokens: &Tokens,
    project_id: &ProjectId,
    disposition: SmartPullDisposition,
    label: &'a str,
    selected: bool,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let msg = (!selected).then_some(Message::Sync(SyncMessage::DispositionChanged(
        project_id.clone(),
        disposition,
    )));
    // RFC-071 Amendment A3 D14: a button label — `body`.
    snora::design::control(
        tokens,
        centered_button_label(
            text(label)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ),
    )
    .height(SMALL_BUTTON_HEIGHT)
    .padding([0.0, 10.0])
    .id(id)
    .on_press_maybe(msg)
    .style(move |_theme, status| current_or_control(selected, &t, status))
    .into()
}

/// RFC-074 Stage 1 D2: one line above the plan's rows, counting each
/// disposition by its own existing name (`plain.get_latest.action_get`,
/// `.action_check`, `.skipped_row` — the same names each row's own action
/// cell already shows, not a fourth vocabulary). Built from `plan.entries`
/// through the plan's own count methods — the same source the rows
/// themselves iterate — so the line can never disagree with them. A count
/// of zero is left out.
///
/// **Never wraps inside a name:** each segment's internal spaces are
/// non-breaking (`\u{a0}`), so if the line itself wraps in a narrow
/// window, it only wraps between segments, at the ` · ` separator — the
/// same separator the dashboard's rows already use (`row.rs`) — never in
/// the middle of, say, "Only check for updates".
/// Each disposition's own name and count, as `"<name>: <n>"`, in order,
/// with a zero count left out — the text content of [`plan_summary_line`]
/// before it is split into one chip per segment. Built from `plan.entries`
/// through the plan's own count methods — the same source the rows
/// themselves iterate — so it can never disagree with them.
pub(crate) fn plan_summary_segments(state: &AppState, plan: &SmartPullPlan) -> Vec<String> {
    let counts = [
        (state.t("plain.get_latest.action_get"), plan.pull_count()),
        (
            state.t("plain.get_latest.action_check"),
            plan.fetch_only_count(),
        ),
        (
            state.t("plain.get_latest.skipped_row"),
            plan.excluded_count(),
        ),
    ];

    counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| format!("{name}: {count}"))
        .collect()
}

/// RFC-074 Stage 1 D2: one line above the plan's rows, counting each
/// disposition by its own existing name (`plain.get_latest.action_get`,
/// `.action_check`, `.skipped_row` — the same names each row's own action
/// cell already shows, not a fourth vocabulary).
///
/// **Never wraps inside a name.** First tried as a single flowing
/// `text()` (the same shape `changelog.rs`'s own `changelog_summary_text`
/// joins with " · "), with each name's own spaces made non-breaking
/// (`\u{a0}`) to keep "Only check for updates" together. That works for
/// English, but Japanese's three names have no spaces at all — cosmic
/// text's own CJK line-breaking allows a break between *any* two
/// characters there, so a narrow window still split a name like
/// "更新の確認のみ" mid-word (confirmed directly: a forced-narrow render
/// broke it after its 5th character). The fix used here is the one the
/// toolbar's own filter chips already use for the identical problem
/// (`toolbar.rs`'s `chips` row, `chip::filter_control`): each segment is
/// its own `text()` with `Wrapping::None` — unbreakable regardless of
/// script — joined in a `row(...).wrap()`, so the *row* wraps between
/// segments while no single segment ever wraps internally. The
/// separator is folded into each segment after the first (`"· {segment}"`),
/// so a wrap never isolates a lone `·` on its own line either.
fn plan_summary_line<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    plan: &SmartPullPlan,
) -> Element<'a, Message> {
    let size = snora::design::style::text::body_size(tokens);
    let line_height = snora::design::style::text::body_line_height(tokens);

    let children: Vec<Element<'_, Message>> = plan_summary_segments(state, plan)
        .into_iter()
        .enumerate()
        .map(|(i, segment)| {
            let content = if i == 0 {
                segment
            } else {
                format!("· {segment}")
            };
            text(content)
                .size(size)
                .line_height(line_height)
                .wrapping(iced::advanced::text::Wrapping::None)
                .into()
        })
        .collect();

    row(children).spacing(4).wrap().into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::AppState;
    use knotra_vcs::OperationId;
    use knotra_vcs::model::operation::{SmartPullPlanEntry, SmartPullSkipReason};

    fn entry(disposition: SmartPullDisposition) -> SmartPullPlanEntry {
        SmartPullPlanEntry {
            project_id: ProjectId::new(),
            project_name: "svc".to_owned(),
            is_dirty: matches!(
                disposition,
                SmartPullDisposition::FetchOnly | SmartPullDisposition::StashAndPull
            ),
            has_conflict: false,
            skip_reason: matches!(disposition, SmartPullDisposition::Excluded)
                .then_some(SmartPullSkipReason::Deselected),
            disposition,
        }
    }

    fn plan(entries: Vec<SmartPullPlanEntry>) -> SmartPullPlan {
        SmartPullPlan {
            id: OperationId::new(),
            entries,
        }
    }

    /// Handoff §1/§3 row 1: "the counts always equal the rows", built from
    /// one mix across both languages.
    #[test]
    fn plan_summary_segments_counts_match_the_rows_in_both_languages() {
        let mixed = plan(vec![
            entry(SmartPullDisposition::Pull),
            entry(SmartPullDisposition::StashAndPull),
            entry(SmartPullDisposition::FetchOnly),
            entry(SmartPullDisposition::Excluded),
            entry(SmartPullDisposition::Excluded),
        ]);

        let en = AppState::new(AppConfig::default());
        assert_eq!(
            plan_summary_segments(&en, &mixed),
            vec!["Get latest: 2", "Only check for updates: 1", "Skipped: 2"]
        );

        let mut ja = AppState::new(AppConfig::default());
        ja.catalog = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::Ja);
        assert_eq!(
            plan_summary_segments(&ja, &mixed),
            vec!["最新を取得: 2", "更新の確認のみ: 1", "スキップ: 2"]
        );
    }

    /// Handoff §3 row 2: a count of zero is left out entirely, not shown
    /// as "Only check for updates: 0".
    #[test]
    fn plan_summary_segments_leave_zero_counts_out() {
        let state = AppState::new(AppConfig::default());

        let only_get_latest = plan(vec![
            entry(SmartPullDisposition::Pull),
            entry(SmartPullDisposition::Pull),
        ]);
        assert_eq!(
            plan_summary_segments(&state, &only_get_latest),
            vec!["Get latest: 2"]
        );

        let only_skipped = plan(vec![
            entry(SmartPullDisposition::Excluded),
            entry(SmartPullDisposition::Excluded),
            entry(SmartPullDisposition::Excluded),
        ]);
        assert_eq!(
            plan_summary_segments(&state, &only_skipped),
            vec!["Skipped: 3"]
        );
    }

    /// Handoff §1: "Changing a row's choice updates the line at once" —
    /// both counts are read fresh from `plan.entries` on every call, so a
    /// changed disposition is reflected with no cache to invalidate.
    #[test]
    fn plan_summary_segments_update_when_a_rows_disposition_changes() {
        let state = AppState::new(AppConfig::default());
        let mut p = plan(vec![entry(SmartPullDisposition::FetchOnly)]);
        assert_eq!(
            plan_summary_segments(&state, &p),
            vec!["Only check for updates: 1"]
        );

        p.entries[0].disposition = SmartPullDisposition::StashAndPull;
        assert_eq!(plan_summary_segments(&state, &p), vec!["Get latest: 1"]);
    }

    /// Handoff §1: never wraps inside a name. Each segment renders as its
    /// own `text()` with `Wrapping::None` (the module doc on
    /// [`plan_summary_line`] explains why — the non-breaking-space version
    /// tried first did not hold for Japanese), so this is checked by
    /// rendering, not by inspecting the plain-text content — see
    /// `label_overflow_guard.rs`'s own
    /// `the_plan_summary_lines_segments_never_wrap_or_clamp`.
    #[test]
    fn plan_summary_line_renders_one_chip_per_segment() {
        let state = AppState::new(AppConfig::default());
        let p = plan(vec![
            entry(SmartPullDisposition::Pull),
            entry(SmartPullDisposition::FetchOnly),
        ]);
        let tokens = &state.theme.tokens;
        let mut ui = crate::headless::simulator(
            plan_summary_line(&state, tokens, &p),
            iced_test::core::Size::new(900.0, 200.0),
        );
        assert!(ui.find("Get latest: 1").is_ok());
        assert!(ui.find("· Only check for updates: 1").is_ok());
    }

    /// Handoff §1 "Decided": the all-skipped reason, read from the real
    /// `pull_modal` view (not the bare catalog key) so this also proves
    /// `plain.disabled.choose_one` — wrong here, since projects were
    /// chosen — no longer appears in this phase.
    #[test]
    fn all_skipped_plan_shows_the_new_reason_in_both_languages() {
        let cases = [
            (
                knotra_ui::i18n::Locale::En,
                "Every project is skipped, so nothing would change.",
            ),
            (
                knotra_ui::i18n::Locale::Ja,
                "すべてのプロジェクトがスキップされるため、変更はありません。",
            ),
        ];

        for (locale, reason) in cases {
            let mut state = AppState::new(AppConfig::default());
            state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
            state.sync.phase = crate::state::sync::SyncPhase::AwaitingConfirm(plan(vec![entry(
                SmartPullDisposition::Excluded,
            )]));

            let mut ui = crate::headless::simulator(
                pull_modal(&state),
                iced_test::core::Size::new(900.0, 700.0),
            );
            assert!(
                ui.find(reason).is_ok(),
                "locale {locale:?}: the all-skipped reason must be shown"
            );
            assert!(
                ui.find(state.t("plain.disabled.choose_one")).is_err(),
                "locale {locale:?}: the old, wrong reason (\"Choose at least one project.\") \
                 must not appear here"
            );
        }
    }
}
