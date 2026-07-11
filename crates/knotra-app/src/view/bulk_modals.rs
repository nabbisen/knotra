//! RFC-0013 — Bulk action modal views.
//! RFC-0021 Phase 3+4 — Plain-language, guided flows with per-step views.
//!
//! Five modals replacing the dedicated screens for Pull, Tag, Switch,
//! Resolve (conflict), and Changelog workflows. Each modal opens over the
//! dashboard and closes on completion or Esc.
//!
//! # Language policy
//! First-level wording uses goal-oriented plain language (see RFC-0021).
//! Technical terms (fetch, pull, tag, branch, conflict, stash, rollback …)
//! appear only inside the "Show details" sections — never as primary labels,
//! titles, or button text. All user-visible strings are routed through
//! `state.t()` so they are available in English and Japanese.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button, guided_field, guided_field_focused,
};
use knotra_vcs::{
    ProjectId,
    model::operation::{FreezeOutcome, SmartPullDisposition},
};

use crate::{
    message::{
        ChangelogMessage, ConflictOpsMessage, ContextMessage, FreezerMessage, Message, SyncMessage,
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Modal shell
// ---------------------------------------------------------------------------

/// Shared shell with title bar used by all modals.
fn modal_shell<'a>(
    title: &'a str,
    close_msg: Message,
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    let close_btn = button(text("✕").size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 12])
        .on_press(close_msg);

    let header = row![
        text(title).size(FONT_BODY + 2.0),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    container(
        column![header, iced::widget::rule::horizontal(1), inner]
            .spacing(16)
            .padding(24),
    )
    .width(Length::Fill)
    .max_width(580.0)
    .into()
}

// ---------------------------------------------------------------------------
// 1. "Get latest safely" modal  (Smart Pull)
// ---------------------------------------------------------------------------
//
// Flow: Idle/Planning → Plan review → Running → Result
// Plain wording at every step; technical detail behind "Show details".

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
                );

                // Disposition override buttons for dirty (non-conflicted) projects
                let action_cell: Element<'_, Message> = if entry.is_dirty && !entry.has_conflict {
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

            let can_start = plan
                .entries
                .iter()
                .any(|e| !matches!(e.disposition, SmartPullDisposition::Excluded));

            let start_reason = if can_start {
                None
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
        SyncPhase::FetchRunning { done, total } => {
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

        SyncPhase::PullRunning { completed, plan } => {
            let total = plan.entries.len();
            let done = completed.len();
            let mut result_rows: Vec<Element<'_, Message>> = completed
                .iter()
                .map(|p| {
                    let (icon, msg) = if p.result.success {
                        ("✓", state.t("plain.get_latest.done_row"))
                    } else {
                        ("!", state.t("plain.get_latest.needs_help_row"))
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

    modal_shell(
        state.t("plain.get_latest"),
        Message::Sync(SyncMessage::ModalClosed),
        inner,
    )
}

/// Render the result step for Get latest safely.
fn pull_result_view<'a>(
    state: &'a AppState,
    result: &'a crate::state::sync::SyncResult,
) -> Element<'a, Message> {
    let ok = result.success_count();
    let fail = result.fail_count();

    let summary = if fail == 0 {
        format!(
            "{} {} {}.",
            state.t("plain.get_latest.all_done_prefix"),
            ok,
            state.t("plain.get_latest.all_done_suffix")
        )
    } else {
        format!(
            "{} {}. {} {}.",
            ok,
            state.t("plain.get_latest.done_count"),
            fail,
            state.t("plain.get_latest.needs_help_count")
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
            let icon = if pp.success { "✓" } else { "!" };
            let msg = if pp.success {
                state.t("plain.get_latest.done_row")
            } else {
                state.t("plain.needs_help")
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
            if !pp.success && !pp.commands_executed.is_empty() && state.show_op_details {
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
) -> (&str, &str) {
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
            if has_conflict {
                state.t("plain.get_latest.note_needs_choice")
            } else {
                ""
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// 2. "Save release point" modal  (Freezer / Tag)
// ---------------------------------------------------------------------------

pub fn tag_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::freezer::FreezerPhase;

    let freezer = &state.freezer;

    let inner: Element<'_, Message> = match &freezer.phase {
        // ── Input + auto-validation ───────────────────────────────────────
        FreezerPhase::Idle | FreezerPhase::Validating => {
            let name_error: Option<&str> = if freezer.freeze_name.is_empty() {
                None // no error until user has typed
            } else if !freezer.freeze_name_is_valid() {
                Some(state.t("plain.release.name_invalid"))
            } else {
                None
            };

            let name_field = guided_field_focused(
                state.t("plain.release.name_label"),
                state.t("plain.release.name_hint"),
                &freezer.freeze_name,
                |s| Message::Freezer(FreezerMessage::NameChanged(s)),
                name_error,
                knotra_ui::widget::focus_id::RELEASE_NAME.clone(),
            );

            let msg_field = guided_field(
                state.t("plain.release.note_label"),
                state.t("plain.release.note_hint"),
                &freezer.tag_message,
                |s| Message::Freezer(FreezerMessage::TagMessageChanged(s)),
                None,
            );

            let validate_or_spinner: Element<'_, Message> =
                if matches!(freezer.phase, FreezerPhase::Validating) {
                    text(state.t("plain.release.checking"))
                        .size(FONT_BODY)
                        .into()
                } else if freezer.freeze_name_is_valid() {
                    button(text(state.t("plain.release.check_readiness")).size(FONT_BODY))
                        .height(BUTTON_HEIGHT)
                        .padding([0, 18])
                        .on_press(Message::Freezer(FreezerMessage::ValidateRequested))
                        .into()
                } else {
                    Space::new().into()
                };

            column![name_field, msg_field, validate_or_spinner]
                .spacing(14)
                .into()
        }

        // ── Validation result + execute ───────────────────────────────────
        FreezerPhase::ValidationReady(validation) => {
            let blocked_count = validation.blocked_count();
            let can_save = validation.all_ready();

            let val_rows: Vec<Element<'_, Message>> = validation
                .entries
                .iter()
                .map(|entry| {
                    let (icon, msg) = if entry.is_blocked() {
                        (
                            "!",
                            entry
                                .blockers
                                .first()
                                .map(|s| plain_blocker(state, s.as_str()))
                                .unwrap_or(""),
                        )
                    } else if !entry.included {
                        ("—", state.t("plain.release.row_excluded"))
                    } else {
                        ("✓", state.t("plain.release.row_ready"))
                    };

                    row![
                        text(icon).size(FONT_BODY).width(Length::Fixed(22.0)),
                        text(&entry.project_name)
                            .size(FONT_BODY)
                            .width(Length::FillPortion(2)),
                        text(msg).size(FONT_SMALL).width(Length::FillPortion(3)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into()
                })
                .collect();

            let save_reason: Option<&str> = if can_save {
                None
            } else if blocked_count == 1 {
                Some(state.t("plain.release.fix_one"))
            } else {
                Some(state.t("plain.release.fix_some"))
            };

            let footer = row![
                guided_button(
                    state.t("plain.save_release_point"),
                    can_save.then_some(Message::Freezer(FreezerMessage::ExecuteConfirmed)),
                    save_reason,
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Freezer(FreezerMessage::BulkModalClosed)),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.release.ready_check")).size(FONT_BODY),
                scrollable(column(val_rows).spacing(6)).height(Length::Fixed(200.0)),
                footer,
            ]
            .spacing(12)
            .into()
        }

        // ── Executing ─────────────────────────────────────────────────────
        FreezerPhase::Executing => column![
            text(state.t("plain.release.saving")).size(FONT_BODY),
            text(state.t("plain.release.saving_hint")).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),

        // ── Result ────────────────────────────────────────────────────────
        FreezerPhase::Done(result) => {
            let outcome_title = match result.outcome {
                FreezeOutcome::Success => state.t("plain.release.outcome_success"),
                FreezeOutcome::RolledBack => state.t("plain.release.outcome_undone"),
                FreezeOutcome::RollbackFailed => state.t("plain.release.outcome_partial"),
                FreezeOutcome::NothingDone => state.t("plain.release.outcome_nothing"),
            };

            let outcome_body = match result.outcome {
                FreezeOutcome::Success => state.t("plain.no_next_step"),
                FreezeOutcome::RolledBack => state.t("plain.release.outcome_undone_hint"),
                FreezeOutcome::RollbackFailed => state.t("plain.release.outcome_partial_hint"),
                FreezeOutcome::NothingDone => "",
            };

            let rows: Vec<Element<'_, Message>> = result
                .project_results
                .iter()
                .map(|pr| {
                    let icon = if pr.success {
                        "✓"
                    } else if pr.rollback_attempted && pr.rollback_succeeded == Some(true) {
                        "⟲"
                    } else {
                        "!"
                    };
                    let msg = if pr.success {
                        state.t("plain.release.row_saved")
                    } else if pr.rollback_attempted && pr.rollback_succeeded == Some(true) {
                        state.t("plain.release.row_undone")
                    } else {
                        state.t("plain.needs_help")
                    };

                    let mut row_col = column![
                        row![
                            text(icon).size(FONT_BODY).width(Length::Fixed(22.0)),
                            text(&pr.project_name)
                                .size(FONT_BODY)
                                .width(Length::FillPortion(2)),
                            text(msg).size(FONT_BODY).width(Length::FillPortion(2)),
                        ]
                        .spacing(8),
                    ]
                    .spacing(4);

                    if !pr.success
                        && state.show_op_details
                        && let Some(hint) = &pr.recovery_hint
                    {
                        for cmd in &hint.suggested_commands {
                            row_col = row_col.push(text(format!("  {}", cmd)).size(FONT_SMALL));
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
                    .on_press(Message::Freezer(FreezerMessage::BulkModalClosed)),
            ]
            .align_y(Alignment::Center);

            column![
                text(outcome_title).size(FONT_BODY + 2.0),
                text(outcome_body).size(FONT_BODY),
                scrollable(column(rows).spacing(8)).height(Length::Fixed(200.0)),
                footer,
            ]
            .spacing(12)
            .into()
        }
    };

    modal_shell(
        state.t("plain.save_release_point"),
        Message::Freezer(FreezerMessage::BulkModalClosed),
        inner,
    )
}

/// Map a technical blocker string to a plain-language message.
fn plain_blocker<'a>(state: &'a AppState, blocker: &str) -> &'a str {
    let lower = blocker.to_lowercase();
    if lower.contains("tag") && lower.contains("exist") {
        state.t("plain.release.blocker_name_used")
    } else if lower.contains("conflict") || lower.contains("merge") {
        state.t("plain.release.blocker_needs_choice")
    } else if lower.contains("dirty") || lower.contains("uncommitted") {
        state.t("plain.release.blocker_unsaved")
    } else {
        state.t("plain.needs_help") // safe fallback
    }
}

// ---------------------------------------------------------------------------
// 3. "Change work area" modal  (Context Switch)
// ---------------------------------------------------------------------------

pub fn switch_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::context::ContextPhase;

    let ctx = &state.context_ops;

    let inner: Element<'_, Message> = match &ctx.phase {
        ContextPhase::Idle => {
            let field = guided_field_focused(
                state.t("plain.switch.target_label"),
                state.t("plain.switch.target_hint"),
                &ctx.target_context,
                |s| Message::Context(ContextMessage::TargetChanged(s)),
                None, // validated at switch attempt, not while typing
                knotra_ui::widget::focus_id::SWITCH_TARGET.clone(),
            );

            let switch_reason: Option<&str> = if ctx.target_context.trim().is_empty() {
                Some(state.t("plain.switch.reason_empty"))
            } else {
                None
            };

            let footer = row![
                guided_button(
                    state.t("plain.change_work_area"),
                    (!ctx.target_context.trim().is_empty())
                        .then_some(Message::Context(ContextMessage::BulkSwitchRequested)),
                    switch_reason,
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Context(ContextMessage::BulkModalClosed)),
            ]
            .align_y(Alignment::Center);

            column![field, footer].spacing(14).into()
        }

        ContextPhase::Switching { .. } => {
            text(state.t("plain.switch.working")).size(FONT_BODY).into()
        }

        ContextPhase::Done(result) => {
            let (title, body) = if result.operation_result.success {
                (
                    state.t("plain.switch.done_title"),
                    state.t("plain.no_next_step"),
                )
            } else {
                (
                    state.t("plain.switch.failed_title"),
                    state.t("plain.switch.failed_hint"),
                )
            };

            let mut detail_col = column![
                text(title).size(FONT_BODY + 2.0),
                text(body).size(FONT_BODY),
            ]
            .spacing(8);

            if !result.operation_result.success && state.show_op_details {
                for cmd in &result.operation_result.commands_executed {
                    detail_col = detail_col.push(text(format!("  {}", cmd)).size(FONT_SMALL));
                }
            }

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
                    .on_press(Message::Context(ContextMessage::BulkModalClosed)),
            ]
            .align_y(Alignment::Center);

            column![detail_col, footer].spacing(12).into()
        }

        _ => Space::new().into(),
    };

    modal_shell(
        state.t("plain.change_work_area"),
        Message::Context(ContextMessage::BulkModalClosed),
        inner,
    )
}

// ---------------------------------------------------------------------------
// 4. Conflict resolve panel  (right-docked sheet)
// ---------------------------------------------------------------------------

pub fn resolve_panel<'a>(state: &'a AppState, project_id: &'a ProjectId) -> Element<'a, Message> {
    let name = project_name_for(state, project_id);
    let ops = &state.conflict_ops;

    let file_rows: Vec<Element<'_, Message>> = ops
        .cached
        .values()
        .flat_map(|d| d.conflicted_files.iter())
        .map(|f| {
            row![
                text("!").size(FONT_BODY).width(Length::Fixed(22.0)),
                text(&f.path).size(FONT_BODY).width(Length::Fill),
                Space::new().width(Length::Fixed(8.0)),
                button(text(state.t("plain.resolve.open_editor")).size(FONT_SMALL + 1.0))
                    .height(36.0)
                    .padding([0, 10])
                    .on_press(Message::ConflictOps(
                        ConflictOpsMessage::OpenInEditorRequested(f.path.clone())
                    )),
                button(text(state.t("plain.resolve.mark_done")).size(FONT_SMALL + 1.0))
                    .height(36.0)
                    .padding([0, 10])
                    .on_press(Message::ConflictOps(
                        ConflictOpsMessage::FileMarkedResolved(f.path.clone()),
                    )),
            ]
            .align_y(Alignment::Center)
            .spacing(6)
            .into()
        })
        .collect();

    let footer = row![
        button(text(state.t("plain.resolve.stop_attempt")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::ConflictOps(ConflictOpsMessage::AbortRequested)),
        Space::new().width(Length::Fill),
        button(text(state.t("action.close")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::ConflictOps(ConflictOpsMessage::PanelClosed)),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(format!("{} — {}", state.t("plain.resolve.title"), name))
                    .size(FONT_BODY + 2.0),
                Space::new().width(Length::Fill),
                button(text("✕").size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 12])
                    .on_press(Message::ConflictOps(ConflictOpsMessage::PanelClosed)),
            ]
            .align_y(Alignment::Center),
            text(state.t("plain.resolve.instruction")).size(FONT_BODY),
            scrollable(column(file_rows).spacing(8)).height(Length::Fill),
            footer,
        ]
        .spacing(14)
        .padding(20),
    )
    .width(Length::Fixed(340.0))
    .height(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// 5. Generate notes modal  (Changelog)
// ---------------------------------------------------------------------------

pub fn changelog_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::changelog::ChangelogPhase;

    let cl = &state.changelog;

    let since_field = guided_field(
        state.t("plain.changelog.since_label"),
        state.t("plain.changelog.since_hint"),
        &cl.since_ref,
        |s| Message::Changelog(ChangelogMessage::SinceRefChanged(s)),
        None,
    );

    let content: Element<'_, Message> = match &cl.phase {
        ChangelogPhase::Idle => {
            let reason = cl
                .since_ref
                .trim()
                .is_empty()
                .then_some(state.t("plain.changelog.reason_empty"));
            guided_button(
                state.t("plain.changelog.generate"),
                (!cl.since_ref.trim().is_empty())
                    .then_some(Message::Changelog(ChangelogMessage::CollectRequested)),
                reason,
            )
        }

        ChangelogPhase::Collecting => text(state.t("plain.changelog.collecting"))
            .size(FONT_BODY)
            .into(),

        ChangelogPhase::Ready(draft) => {
            let content_text = format!("{:?}", draft); // TODO: render ChangelogDraft properly
            let copy_text = content_text.clone();
            column![
                scrollable(text(content_text).size(FONT_SMALL)).height(Length::Fixed(240.0)),
                row![
                    button(text(state.t("plain.changelog.copy")).size(FONT_BODY))
                        .height(BUTTON_HEIGHT)
                        .padding([0, 18])
                        .on_press(Message::CopyToClipboard(copy_text)),
                    Space::new().width(Length::Fill),
                    button(text(state.t("action.close")).size(FONT_BODY))
                        .height(BUTTON_HEIGHT)
                        .padding([0, 18])
                        .on_press(Message::Changelog(ChangelogMessage::ModalClosed)),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            ]
            .spacing(10)
            .into()
        }
    };

    let inner = column![since_field, content].spacing(14);

    modal_shell(
        state.t("plain.changelog.title"),
        Message::Changelog(ChangelogMessage::ModalClosed),
        inner.into(),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_name_for(state: &AppState, id: &ProjectId) -> String {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| id.to_string())
}
