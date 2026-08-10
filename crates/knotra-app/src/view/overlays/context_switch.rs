//! 3. "Change work area" modal (Context Switch) — RFC-037 Stage 4.
//!
//! `context_switch.rs`, not `context.rs` (RFC-041 D5 naming, applied here) —
//! `app/context.rs` already exists one level up. Different module tree, so
//! no collision, but the name avoids reading ambiguously.
//!
//! `modal_shell` replaced with `knotra_ui::widget::overlay::surface`.
//! `guided_button`/`guided_field_focused` call sites are untouched (D6/R11,
//! D7/R12). Each phase's own local `footer` row (`BrowsingList`,
//! `ConfirmSwitch`, `Done`) maps directly onto `surface()`'s `footer`
//! parameter, the same mapping Stage 2 (`conflict.rs`) and Stage 4
//! (`freezer.rs`) used; phases with no such row (`Idle`, `LoadingList`,
//! `Switching`) pass an empty `Space` instead.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, Tokens, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};
use knotra_vcs::ContextTarget;

use crate::{
    message::{ContextMessage, Message},
    state::AppState,
};

fn context_target_kind_key(target: &ContextTarget) -> &'static str {
    match target {
        ContextTarget::GitLocalBranch { .. } | ContextTarget::Manual { .. } => {
            "plain.switch.kind_local"
        }
        ContextTarget::GitRemoteBranch { .. } => "plain.switch.kind_shared",
        ContextTarget::JjBookmark { .. } => "plain.switch.kind_saved_name",
        ContextTarget::JjChange { .. } => "plain.switch.kind_change",
    }
}

pub fn switch_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::context::ContextPhase;

    let tokens = &state.theme.tokens;
    let ctx = &state.context_ops;

    let (inner, footer): (Element<'_, Message>, Element<'_, Message>) = match &ctx.phase {
        ContextPhase::Idle => (
            column![text(state.t("plain.switch.no_project")).size(FONT_BODY)]
                .spacing(8)
                .into(),
            Space::new().into(),
        ),

        ContextPhase::LoadingList(_) => (
            column![
                text(state.t("plain.status.checking")).size(FONT_BODY),
                text(state.t("plain.switch.loading_hint")).size(FONT_SMALL),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::BrowsingList {
            project_id, search, ..
        } => {
            let search_field = guided_field_focused(
                state.t("plain.switch.search_label"),
                state.t("plain.switch.search_hint"),
                search,
                |s| Message::Context(ContextMessage::SearchChanged(s)),
                None,
                knotra_ui::widget::focus_id::SWITCH_TARGET.clone(),
            );

            let mut rows = column![search_field].spacing(8);
            let candidates = ctx.filtered_candidates();
            if candidates.is_empty() {
                rows = rows.push(text(state.t("plain.switch.no_targets")).size(FONT_SMALL));
            } else {
                let mut list = column![].spacing(4);
                for candidate in candidates {
                    let reason_key = candidate
                        .is_current
                        .then_some("plain.switch.reason_current");
                    let kind = state.t(context_target_kind_key(&candidate.target));
                    let detail = candidate.target.display_target();
                    let label = column![
                        text(candidate.label.as_str()).size(FONT_BODY),
                        text(format!("{kind} · {detail}")).size(FONT_SMALL),
                    ]
                    .spacing(2);
                    let press = reason_key.is_none().then_some(Message::Context(
                        ContextMessage::SwitchTargetChosen(
                            project_id.clone(),
                            candidate.target.clone(),
                            candidate.label.clone(),
                        ),
                    ));
                    let t = tokens.clone();
                    let candidate_btn = iced::widget::button(label)
                        .width(Length::Fill)
                        .on_press_maybe(press)
                        .style(move |_theme, status| {
                            style::with_focus_ring(&t, false, style::ghost(&t, status))
                        });
                    let mut row_col = column![candidate_btn];
                    if let Some(reason_key) = reason_key {
                        row_col = row_col.push(text(state.t(reason_key)).size(FONT_SMALL));
                    }
                    list = list.push(row_col.spacing(2));
                }
                // No inner `scrollable` around the candidate list (unlike
                // the pre-migration `.height(Length::Fixed(220.0))` box) —
                // `surface()`'s own body scrollable now covers the whole
                // body, same reasoning as Stage 2/3 (review `132` §4).
                rows = rows.push(list);
            }

            let footer = row![
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Context(ContextMessage::BulkModalClosed)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);
            (rows.into(), footer.into())
        }

        ContextPhase::ConfirmSwitch {
            project_name,
            target_label,
            target,
            is_dirty,
            disabled_reason_key,
            ..
        } => {
            let caution = if let Some(reason_key) = disabled_reason_key {
                state.t(reason_key)
            } else if *is_dirty {
                state.t("plain.switch.dirty_hint")
            } else {
                state.t("plain.no_next_step")
            };
            let switch_msg = (disabled_reason_key.is_none()
                && !state.operation_interlock.is_busy())
            .then_some(Message::Context(ContextMessage::SwitchConfirmed));
            let footer = row![
                reasoned(
                    tokens,
                    state.t("plain.change_work_area"),
                    switch_msg,
                    if state.operation_interlock.is_busy() {
                        Some(state.t("plain.activity.busy"))
                    } else {
                        disabled_reason_key.map(|key| state.t(key))
                    },
                    false,
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Context(ContextMessage::SwitchCancelled)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    text(project_name).size(FONT_BODY),
                    text(target_label).size(FONT_BODY),
                    text(state.t(context_target_kind_key(target))).size(FONT_SMALL),
                    text(caution).size(FONT_SMALL),
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
        }

        ContextPhase::Switching { target_label, .. } => (
            column![
                text(state.t("plain.switch.working")).size(FONT_BODY),
                text(target_label).size(FONT_SMALL),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

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
                text(result.target.as_str()).size(FONT_SMALL),
                text(body).size(FONT_BODY),
            ]
            .spacing(8);

            if !result.operation_result.success && state.show_op_details {
                if let Some(hint) = &result.recovery_hint {
                    detail_col = detail_col.push(text(hint.situation.as_str()).size(FONT_SMALL));
                    for cmd in &hint.suggested_commands {
                        detail_col = detail_col.push(text(format!("  {}", cmd)).size(FONT_SMALL));
                    }
                }
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
                    Some(Message::Context(ContextMessage::BulkModalClosed)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            (detail_col.into(), footer.into())
        }
    };

    // R2/§2: unchanged from before this migration.
    let close_msg = (!matches!(ctx.phase, ContextPhase::Switching { .. }))
        .then_some(Message::Context(ContextMessage::BulkModalClosed));

    surface(
        tokens,
        OverlayWidth::Large,
        state.t("plain.change_work_area"),
        close_msg,
        false,
        inner,
        footer,
    )
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (Stage 2),
/// `changelog.rs` (Stage 3), and `freezer.rs` (Stage 4) use. `is_focused` is
/// always `false`: no real focus-order wiring exists or is permitted for
/// this overlay this stage (R3 forbids `app/`/`state/`).
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
