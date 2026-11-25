//! 2. "Save release point" modal (Freezer / Tag) — RFC-037 Stage 4.
//!
//! `modal_shell` replaced with `knotra_ui::widget::overlay::surface`.
//! `guided_field`/`guided_field_focused`/`guided_button` call sites are
//! untouched (D6/R11, D7/R12 — both are still-current vocabulary, not
//! legacy helpers to migrate off this stage). Each phase's own local
//! `footer` row (`ValidationReady`, `Done`) maps directly onto `surface()`'s
//! `footer` parameter, the same mapping Stage 2 used for `conflict.rs`;
//! phases with no such row (`Idle`/`Validating`, `Executing`) pass an empty
//! `Space` instead, the same choice Stage 3 made for `changelog.rs`'s
//! footer-less phases.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, container, row, text},
};

use knotra_ui::widget::{
    Tokens, Tone, body_badge_diameter, ghost_control, ghost_control_maybe, guided_field_focused,
    list,
    overlay::{OverlayWidth, surface},
    primary_control_maybe, reasoned_control_maybe, status_badge, status_symbol, style,
};
use knotra_vcs::model::operation::{FreezeBlocker, FreezeNote, FreezeOutcome};

use crate::{
    message::{FreezerMessage, Message, TagPushMessage},
    state::AppState,
};

/// The leading icon column every row built on it shares — the validation
/// rows, the result rows, the impact-warning rows, and `continuation_line`'s
/// own alignment beneath them. Named so the four sites stay in step with
/// each other (Review 241 §3, Task 100 A1); every other `.spacing(8)` in
/// this file is an unrelated layout that happens to use the same number.
const ICON_COLUMN_WIDTH: Length = Length::Fixed(22.0);
const ICON_COLUMN_SPACING: f32 = 8.0;

/// RFC-075 stage 3 §3: Save release point's own entry per phase.
/// `ExecuteConfirmed` starts a change, so "Save" is never the entry.
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    use crate::state::freezer::FreezerPhase;

    let freezer = &state.freezer;
    match &freezer.phase {
        FreezerPhase::Idle | FreezerPhase::Validating { .. } => {
            Some(crate::view::ids::freezer::release_name())
        }
        FreezerPhase::ValidationReady(validation) => {
            let has_unreadable = validation.entries.iter().any(|entry| {
                entry
                    .blockers
                    .iter()
                    .any(|b| matches!(b, FreezeBlocker::Unreadable { .. }))
            });
            Some(if has_unreadable {
                crate::view::ids::freezer::show_details()
            } else {
                crate::view::ids::freezer::cancel()
            })
        }
        FreezerPhase::Executing => None,
        FreezerPhase::Done(result) => {
            let share_offer_shown = state.pending_tag_push.as_ref().is_some_and(|push| {
                push.freeze_name == result.freeze_name
                    && !push.project_ids.is_empty()
                    && !push.is_pushing
            });
            Some(if share_offer_shown {
                crate::view::ids::freezer::not_now()
            } else {
                crate::view::ids::freezer::done_close()
            })
        }
    }
}

pub fn tag_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::freezer::FreezerPhase;

    let tokens = &state.theme.tokens;
    let freezer = &state.freezer;

    let (inner, footer): (Element<'_, Message>, Element<'_, Message>) = match &freezer.phase {
        // ── Input + auto-validation ───────────────────────────────────────
        FreezerPhase::Idle | FreezerPhase::Validating { .. } => {
            let name_error: Option<&str> = if freezer.freeze_name.is_empty() {
                None // no error until user has typed
            } else if !freezer.freeze_name_is_valid() {
                Some(state.t("plain.release.name_invalid"))
            } else {
                None
            };

            let name_field = guided_field_focused(
                tokens,
                state.t("plain.release.name_label"),
                state.t("plain.release.name_hint"),
                &freezer.freeze_name,
                |s| Message::Freezer(FreezerMessage::NameChanged(s)),
                name_error,
                crate::view::ids::freezer::release_name(),
            );

            let msg_field = guided_field_focused(
                tokens,
                state.t("plain.release.note_label"),
                state.t("plain.release.note_hint"),
                &freezer.tag_message,
                |s| Message::Freezer(FreezerMessage::TagMessageChanged(s)),
                None,
                crate::view::ids::freezer::release_note(),
            );

            let validate_or_spinner: Element<'_, Message> =
                if matches!(freezer.phase, FreezerPhase::Validating { .. }) {
                    text(state.t("plain.release.checking"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .into()
                } else if freezer.freeze_name_is_valid() {
                    reasoned_control_maybe(
                        tokens,
                        state.t("plain.release.check_readiness"),
                        (!state.operation_interlock.is_busy())
                            .then_some(Message::Freezer(FreezerMessage::ValidateRequested)),
                        state
                            .operation_interlock
                            .is_busy()
                            .then_some(state.t("plain.activity.busy")),
                        style::primary,
                        crate::view::ids::freezer::check_readiness(),
                    )
                } else {
                    Space::new().into()
                };

            (
                column![name_field, msg_field, validate_or_spinner]
                    .spacing(14)
                    .into(),
                Space::new().into(),
            )
        }

        // ── Validation result + execute ───────────────────────────────────
        FreezerPhase::ValidationReady(validation) => {
            let blocked_count = validation.blocked_count();
            let ready_count = validation.ready_count();
            let can_save =
                validation.all_ready() && ready_count > 0 && !state.operation_interlock.is_busy();

            // R9: an entry's `Unreadable` reason is a filesystem-derived
            // diagnostic, shown only under "Show details" — the same
            // threshold the result screen (`Done`, below) already applies
            // to its own recovery-hint commands.
            let has_unreadable = validation.entries.iter().any(|entry| {
                entry
                    .blockers
                    .iter()
                    .any(|b| matches!(b, FreezeBlocker::Unreadable { .. }))
            });

            let val_rows: Vec<Element<'_, Message>> = validation
                .entries
                .iter()
                .map(|entry| {
                    // RFC-073 D9: `status_badge`, not a plain-text glyph —
                    // blocked is `danger`/alert, ready is `success`/check,
                    // the same two outcomes `badge_for_operation_outcome`
                    // gives failure/success, named directly here rather
                    // than forced through that function's
                    // `ProjectOperationOutcome` (this is a readiness check,
                    // not a completed operation's outcome).
                    let (badge, first_line) = if entry.is_blocked() {
                        (
                            (Tone::Danger, status_symbol::alert()),
                            entry
                                .blockers
                                .first()
                                .map(|b| blocker_message(state, b))
                                .unwrap_or_default(),
                        )
                    } else {
                        (
                            (Tone::Success, status_symbol::all_set()),
                            state.t("plain.release.row_ready"),
                        )
                    };

                    let mut row_col = column![
                        row![
                            container(status_badge(
                                tokens,
                                badge.0,
                                &badge.1,
                                body_badge_diameter(tokens)
                            ))
                            .width(ICON_COLUMN_WIDTH),
                            text(&entry.project_name)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(2)),
                            // RFC-071 Amendment A3 D14: the row's own
                            // reason/status — `body`.
                            text(first_line)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(3)),
                        ]
                        .spacing(ICON_COLUMN_SPACING)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(4);

                    // D3: every blocker, not only the first, once each, in
                    // declaration order. Each sits under the first reason's
                    // own column, not at the row's left edge (Review 239 §4).
                    for blocker in entry.blockers.iter().skip(1) {
                        row_col = row_col.push(continuation_line(
                            tokens,
                            blocker_message(state, blocker),
                            3,
                        ));
                    }

                    // R9: `Unreadable`'s own diagnostic, only under "Show
                    // details" — same column as the reasons above it.
                    if state.show_op_details {
                        for blocker in &entry.blockers {
                            if let FreezeBlocker::Unreadable { reason } = blocker {
                                row_col =
                                    row_col.push(continuation_line(tokens, reason.as_str(), 3));
                            }
                        }
                    }

                    // D3/D5: notes are just as true on a blocked row as on a
                    // ready one, so they are not gated on `is_blocked()`.
                    for note in &entry.notes {
                        row_col =
                            row_col.push(continuation_line(tokens, note_message(state, note), 3));
                    }

                    row_col.into()
                })
                .collect();

            let save_reason: Option<&str> = if state.operation_interlock.is_busy() {
                Some(state.t("plain.activity.busy"))
            } else if can_save {
                None
            } else if ready_count == 0 && blocked_count == 0 {
                Some(state.t("plain.disabled.choose_one"))
            } else if blocked_count == 1 {
                Some(state.t("plain.release.fix_one"))
            } else {
                Some(state.t("plain.release.fix_some"))
            };

            // R9: the details button sits between the Fill space and
            // Cancel, only when at least one entry has something to show —
            // Save and Cancel neither move nor change size or style when it
            // appears.
            let save = reasoned_control_maybe(
                tokens,
                state.t("plain.save_release_point"),
                can_save.then_some(Message::Freezer(FreezerMessage::ExecuteConfirmed)),
                save_reason,
                style::primary,
                crate::view::ids::freezer::save(),
            );

            let mut footer_row = row![save, Space::new().width(Length::Fill),]
                .spacing(8)
                .align_y(Alignment::Center);

            if has_unreadable {
                let details_label = if state.show_op_details {
                    state.t("plain.hide_details")
                } else {
                    state.t("plain.show_details")
                };
                footer_row = footer_row.push(ghost_control(
                    tokens,
                    details_label,
                    Message::ToggleOpDetails,
                    crate::view::ids::freezer::show_details(),
                ));
            }

            let footer = footer_row.push(ghost_control(
                tokens,
                state.t("action.cancel"),
                Message::Freezer(FreezerMessage::BulkModalClosed),
                crate::view::ids::freezer::cancel(),
            ));

            // No inner `scrollable` around the row list (unlike the
            // pre-migration `.height(Length::Fixed(200.0))` box) —
            // `surface()`'s own body scrollable now covers the whole body,
            // same reasoning as Stage 2/3 (review `132` §4).
            (
                column![
                    text(state.t("plain.release.ready_check"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    // RFC-071 D8a: the ready check is a list of items — one
                    // border line between adjacent entries.
                    list(tokens, val_rows),
                    impact_warnings_section(tokens, state),
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
        }

        // ── Executing ─────────────────────────────────────────────────────
        FreezerPhase::Executing => (
            column![
                text(state.t("plain.release.saving"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                // RFC-071 Amendment A3 D14: read to understand — `body`.
                text(state.t("plain.release.saving_hint"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        // ── Result ────────────────────────────────────────────────────────
        FreezerPhase::Done(result) => {
            let push_is_running = state
                .pending_tag_push
                .as_ref()
                .is_some_and(|push| push.is_pushing);
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
                            text(icon)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(ICON_COLUMN_WIDTH),
                            text(&pr.project_name)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(2)),
                            text(msg)
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens))
                                .width(Length::FillPortion(2)),
                        ]
                        .spacing(ICON_COLUMN_SPACING),
                    ]
                    .spacing(4);

                    if !pr.success
                        && state.show_op_details
                        && let Some(hint) = &pr.recovery_hint
                    {
                        for cmd in &hint.suggested_commands {
                            row_col = row_col.push(continuation_line(tokens, cmd.as_str(), 2));
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

            let push_offer: Element<'_, Message> = match &state.pending_tag_push {
                Some(push)
                    if push.freeze_name == result.freeze_name && !push.project_ids.is_empty() =>
                {
                    if push.is_pushing {
                        text(state.t("plain.release.sharing"))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens))
                            .into()
                    } else {
                        column![
                            text(state.t("plain.release.share_offer"))
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens)),
                            row![
                                primary_control_maybe(
                                    tokens,
                                    state.t("plain.release.share_action"),
                                    (!state.operation_interlock.is_busy())
                                        .then_some(Message::TagPush(TagPushMessage::PushConfirmed)),
                                    crate::view::ids::freezer::share(),
                                ),
                                ghost_control(
                                    tokens,
                                    state.t("plain.release.share_decline"),
                                    Message::TagPush(TagPushMessage::PushDeclined),
                                    crate::view::ids::freezer::not_now(),
                                ),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(8)
                        .into()
                    }
                }
                _ => Space::new().height(Length::Fixed(0.0)).into(),
            };

            let footer = row![
                ghost_control(
                    tokens,
                    details_label,
                    Message::ToggleOpDetails,
                    crate::view::ids::freezer::show_details(),
                ),
                Space::new().width(Length::Fill),
                ghost_control_maybe(
                    tokens,
                    state.t("action.close"),
                    (!push_is_running).then_some(Message::Freezer(FreezerMessage::BulkModalClosed)),
                    crate::view::ids::freezer::done_close(),
                ),
            ]
            .align_y(Alignment::Center);

            // No inner `scrollable` around the row list — same reasoning as
            // the `ValidationReady` branch above.
            (
                column![
                    text(outcome_title).size(snora::design::style::text::title_size(tokens)),
                    text(outcome_body)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    // RFC-071 D8a: the results are a list of items too.
                    list(tokens, rows),
                    push_offer,
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
        }
    };

    // R2/§2: both clauses, unchanged. The second is not a phase check — it
    // reads `state.pending_tag_push` directly — so a close during a running
    // tag push stays blocked even though `freezer.phase` itself has already
    // moved past `Executing` into `Done`.
    let close_msg = if matches!(freezer.phase, FreezerPhase::Executing)
        || state
            .pending_tag_push
            .as_ref()
            .is_some_and(|push| push.is_pushing)
    {
        None
    } else {
        Some(Message::Freezer(FreezerMessage::BulkModalClosed))
    };

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Large.resolve(state.window_width),
            state.t("plain.save_release_point"),
            close_msg,
            crate::view::ids::freezer::close(),
            inner,
            footer,
        ),
        &[],
    )
}

/// RFC-044 D2/D3: dependency impact, beside the per-project blockers above.
/// `state.freezer.topology_checked` distinguishes "not checked" from
/// "checked, found nothing" — both are stated explicitly, never left as an
/// empty section a silent absence could be misread from (D3, R3).
fn impact_warnings_section<'a>(tokens: &Tokens, state: &'a AppState) -> Element<'a, Message> {
    // RFC-071 Amendment A3 D14: notices read to understand the release's
    // consequences — `body`.
    if !state.freezer.topology_checked {
        return text(state.t("plain.release.impact_unchecked"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into();
    }
    if state.freezer.impact_warnings.is_empty() {
        return text(state.t("plain.release.impact_clear"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into();
    }

    let rows: Vec<Element<'_, Message>> = state
        .freezer
        .impact_warnings
        .iter()
        .map(|w| {
            row![
                text("⚠")
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(ICON_COLUMN_WIDTH),
                text(&w.frozen_project_name)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(Length::FillPortion(2)),
                // RFC-071 Amendment A3 D14: a warning read to understand
                // the release's consequences — `body`.
                text(format!(
                    "{}: {}",
                    state.t("plain.release.impact_depended_on_by"),
                    w.dependent_projects.join(", ")
                ))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
                .width(Length::FillPortion(3)),
            ]
            .spacing(ICON_COLUMN_SPACING)
            .align_y(Alignment::Center)
            .into()
        })
        .collect();

    column![
        // RFC-071 Amendment A3 D14: introduces the section's own content —
        // `body`.
        text(state.t("plain.release.impact_title"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        // RFC-071 D8a: another list of items, same rule.
        list(tokens, rows),
    ]
    .spacing(6)
    .into()
}

/// A continuation line for a validation/result row — a further reason, a
/// note, an `Unreadable` diagnostic, or a recovery-hint command. Uses the
/// same icon/name/content column weights as the row's own first line
/// (`row_col`'s `row![...]` above), so it lines up under that line's content
/// column instead of starting at the row's left edge (Review 239 §4) — no
/// padding number guessed to match. `content_portion` is the first line's
/// own content column weight: 3 for validation rows, 2 for result rows.
///
/// **RFC-071 Amendment A3 D14:** every kind of continuation this helper
/// renders is read in order to understand why a project is blocked or how
/// to recover — `body`, matching the row's own first line. An `Unreadable`
/// diagnostic and a suggested recovery command sit behind "Show details",
/// but D14's `body_small` bucket is incidental metadata only (timestamps,
/// VCS kind, branch, counts, keyboard hints) — being gated behind a toggle
/// is not that; the text underneath is still the reason a user opened the
/// toggle to read.
fn continuation_line<'a>(
    tokens: &Tokens,
    content: impl Into<String>,
    content_portion: u16,
) -> Element<'a, Message> {
    let content_text: Element<'a, Message> = text(content.into())
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens))
        .width(Length::FillPortion(content_portion))
        .into();
    row![
        Space::new().width(ICON_COLUMN_WIDTH),
        Space::new().width(Length::FillPortion(2)),
        content_text,
    ]
    .spacing(ICON_COLUMN_SPACING)
    .into()
}

/// Maps a `FreezeBlocker` to its catalog message. Exhaustive — no `_`
/// arm — so a new variant without a mapping fails to compile (RFC-065
/// D2/R3). Replaces this file's old helper, which substring-matched a
/// producer's own diagnostic string and let a Jujutsu "bookmark … already
/// exists" fall through to a generic "Needs help" message since it only
/// knew the word `tag`.
fn blocker_message(state: &AppState, blocker: &FreezeBlocker) -> &'static str {
    match blocker {
        FreezeBlocker::UnresolvedConflict => state.t("plain.release.blocker_needs_choice"),
        FreezeBlocker::UncommittedChanges => state.t("plain.release.blocker_unsaved"),
        FreezeBlocker::NameInUse => state.t("plain.release.blocker_name_used"),
        FreezeBlocker::Unreadable { .. } => state.t("plain.release.blocker_unreadable"),
    }
}

/// Maps a `FreezeNote` to its rendered "label: count" line (RFC-038 Stage 4
/// §1b's established convention — no plural forms needed). Exhaustive for
/// the same reason as `blocker_message`.
fn note_message(state: &AppState, note: &FreezeNote) -> String {
    match note {
        FreezeNote::UntrackedFiles(n) => format!("{} {n}", state.t("plain.release.note_untracked")),
    }
}
