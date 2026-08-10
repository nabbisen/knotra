//! 3. "Change work area" modal (Context Switch) — RFC-037 Stage 1.
//!
//! `context_switch.rs`, not `context.rs` (RFC-041 D5 naming, applied here) —
//! `app/context.rs` already exists one level up. Different module tree, so
//! no collision, but the name avoids reading ambiguously.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, row, scrollable, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button, guided_field_focused,
};
use knotra_vcs::ContextTarget;

use super::modal_shell;
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

    let ctx = &state.context_ops;

    let inner: Element<'_, Message> = match &ctx.phase {
        ContextPhase::Idle => column![text(state.t("plain.switch.no_project")).size(FONT_BODY)]
            .spacing(8)
            .into(),

        ContextPhase::LoadingList(_) => column![
            text(state.t("plain.status.checking")).size(FONT_BODY),
            text(state.t("plain.switch.loading_hint")).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),

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
                    let mut row_col =
                        column![button(label).width(Length::Fill).on_press_maybe(press)];
                    if let Some(reason_key) = reason_key {
                        row_col = row_col.push(text(state.t(reason_key)).size(FONT_SMALL));
                    }
                    list = list.push(row_col.spacing(2));
                }
                rows = rows.push(scrollable(list).height(Length::Fixed(220.0)));
            }

            let footer = row![
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Context(ContextMessage::BulkModalClosed)),
            ]
            .align_y(Alignment::Center);
            column![rows, footer].spacing(14).into()
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
                guided_button(
                    state.t("plain.change_work_area"),
                    switch_msg,
                    if state.operation_interlock.is_busy() {
                        Some(state.t("plain.activity.busy"))
                    } else {
                        disabled_reason_key.map(|key| state.t(key))
                    },
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Context(ContextMessage::SwitchCancelled)),
            ]
            .align_y(Alignment::Center);

            column![
                text(project_name).size(FONT_BODY),
                text(target_label).size(FONT_BODY),
                text(state.t(context_target_kind_key(target))).size(FONT_SMALL),
                text(caution).size(FONT_SMALL),
                footer,
            ]
            .spacing(12)
            .into()
        }

        ContextPhase::Switching { target_label, .. } => column![
            text(state.t("plain.switch.working")).size(FONT_BODY),
            text(target_label).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),

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
    };

    let close_msg = (!matches!(ctx.phase, ContextPhase::Switching { .. }))
        .then_some(Message::Context(ContextMessage::BulkModalClosed));
    modal_shell(state.t("plain.change_work_area"), close_msg, inner)
}
