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
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, Tokens, guided_field, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};
use knotra_vcs::model::operation::FreezeOutcome;

use crate::{
    message::{FreezerMessage, Message, TagPushMessage},
    state::AppState,
};

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
                knotra_ui::widget::focus_id::RELEASE_NAME.clone(),
            );

            let msg_field = guided_field(
                tokens,
                state.t("plain.release.note_label"),
                state.t("plain.release.note_hint"),
                &freezer.tag_message,
                |s| Message::Freezer(FreezerMessage::TagMessageChanged(s)),
                None,
            );

            let validate_or_spinner: Element<'_, Message> =
                if matches!(freezer.phase, FreezerPhase::Validating { .. }) {
                    text(state.t("plain.release.checking"))
                        .size(snora::design::style::text::body_size(tokens))
                        .into()
                } else if freezer.freeze_name_is_valid() {
                    reasoned(
                        tokens,
                        state.t("plain.release.check_readiness"),
                        (!state.operation_interlock.is_busy())
                            .then_some(Message::Freezer(FreezerMessage::ValidateRequested)),
                        state
                            .operation_interlock
                            .is_busy()
                            .then_some(state.t("plain.activity.busy")),
                        false,
                        style::primary,
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
                        text(icon)
                            .size(snora::design::style::text::body_size(tokens))
                            .width(Length::Fixed(22.0)),
                        text(&entry.project_name)
                            .size(snora::design::style::text::body_size(tokens))
                            .width(Length::FillPortion(2)),
                        text(msg)
                            .size(snora::design::style::text::body_small_size(tokens))
                            .width(Length::FillPortion(3)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into()
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

            let footer = row![
                reasoned(
                    tokens,
                    state.t("plain.save_release_point"),
                    can_save.then_some(Message::Freezer(FreezerMessage::ExecuteConfirmed)),
                    save_reason,
                    false,
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Freezer(FreezerMessage::BulkModalClosed)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            // No inner `scrollable` around the row list (unlike the
            // pre-migration `.height(Length::Fixed(200.0))` box) —
            // `surface()`'s own body scrollable now covers the whole body,
            // same reasoning as Stage 2/3 (review `132` §4).
            (
                column![
                    text(state.t("plain.release.ready_check"))
                        .size(snora::design::style::text::body_size(tokens)),
                    column(val_rows).spacing(6),
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
                    .size(snora::design::style::text::body_size(tokens)),
                text(state.t("plain.release.saving_hint"))
                    .size(snora::design::style::text::body_small_size(tokens)),
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
                                .width(Length::Fixed(22.0)),
                            text(&pr.project_name)
                                .size(snora::design::style::text::body_size(tokens))
                                .width(Length::FillPortion(2)),
                            text(msg)
                                .size(snora::design::style::text::body_size(tokens))
                                .width(Length::FillPortion(2)),
                        ]
                        .spacing(8),
                    ]
                    .spacing(4);

                    if !pr.success
                        && state.show_op_details
                        && let Some(hint) = &pr.recovery_hint
                    {
                        for cmd in &hint.suggested_commands {
                            row_col = row_col.push(
                                text(format!("  {}", cmd))
                                    .size(snora::design::style::text::body_small_size(tokens)),
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

            let push_offer: Element<'_, Message> = match &state.pending_tag_push {
                Some(push)
                    if push.freeze_name == result.freeze_name && !push.project_ids.is_empty() =>
                {
                    if push.is_pushing {
                        text(state.t("plain.release.sharing"))
                            .size(snora::design::style::text::body_size(tokens))
                            .into()
                    } else {
                        column![
                            text(state.t("plain.release.share_offer"))
                                .size(snora::design::style::text::body_size(tokens)),
                            row![
                                styled_button(
                                    tokens,
                                    state.t("plain.release.share_action"),
                                    (!state.operation_interlock.is_busy())
                                        .then_some(Message::TagPush(TagPushMessage::PushConfirmed)),
                                    style::primary,
                                ),
                                styled_button(
                                    tokens,
                                    state.t("plain.release.share_decline"),
                                    Some(Message::TagPush(TagPushMessage::PushDeclined)),
                                    style::ghost,
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
                    (!push_is_running).then_some(Message::Freezer(FreezerMessage::BulkModalClosed)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            // No inner `scrollable` around the row list — same reasoning as
            // the `ValidationReady` branch above.
            (
                column![
                    text(outcome_title).size(snora::design::style::text::title_size(tokens)),
                    text(outcome_body).size(snora::design::style::text::body_size(tokens)),
                    column(rows).spacing(8),
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

    surface(
        tokens,
        OverlayWidth::Large.resolve(state.window_width),
        state.t("plain.save_release_point"),
        close_msg,
        false,
        inner,
        footer,
    )
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (Stage 2) and
/// `changelog.rs` (Stage 3) use. `is_focused` is always `false`: no real
/// focus-order wiring exists or is permitted for this overlay this stage
/// (R3 forbids `app/`/`state/`).
fn styled_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    iced::widget::button(text(label).size(snora::design::style::text::body_size(tokens)))
        .height(BUTTON_HEIGHT)
        .padding([0, 18])
        .on_press_maybe(on_press)
        .style(move |_theme, status| style::with_focus_ring(&t, false, style_fn(&t, status)))
        .into()
}

/// RFC-044 D2/D3: dependency impact, beside the per-project blockers above.
/// `state.freezer.topology_checked` distinguishes "not checked" from
/// "checked, found nothing" — both are stated explicitly, never left as an
/// empty section a silent absence could be misread from (D3, R3).
fn impact_warnings_section<'a>(tokens: &Tokens, state: &'a AppState) -> Element<'a, Message> {
    if !state.freezer.topology_checked {
        return text(state.t("plain.release.impact_unchecked"))
            .size(snora::design::style::text::body_small_size(tokens))
            .into();
    }
    if state.freezer.impact_warnings.is_empty() {
        return text(state.t("plain.release.impact_clear"))
            .size(snora::design::style::text::body_small_size(tokens))
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
                    .width(Length::Fixed(22.0)),
                text(&w.frozen_project_name)
                    .size(snora::design::style::text::body_size(tokens))
                    .width(Length::FillPortion(2)),
                text(format!(
                    "{}: {}",
                    state.t("plain.release.impact_depended_on_by"),
                    w.dependent_projects.join(", ")
                ))
                .size(snora::design::style::text::body_small_size(tokens))
                .width(Length::FillPortion(3)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        })
        .collect();

    column![
        text(state.t("plain.release.impact_title"))
            .size(snora::design::style::text::body_small_size(tokens)),
        column(rows).spacing(6),
    ]
    .spacing(6)
    .into()
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
