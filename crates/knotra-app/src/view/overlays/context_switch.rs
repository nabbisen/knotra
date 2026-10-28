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
    BUTTON_HEIGHT, Tokens, centered_button_label, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};
use knotra_vcs::ContextTarget;

use crate::{
    message::{ContextMessage, Message},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "switch.close";
    pub const BROWSING_CANCEL: &str = "switch.browsing_cancel";
    pub const CONFIRM_SWITCH: &str = "switch.confirm_switch";
    pub const CONFIRM_CANCEL: &str = "switch.confirm_cancel";
    pub const DETAILS: &str = "switch.details";
    pub const FOOTER_CLOSE: &str = "switch.footer_close";
}

/// RFC-066 D2: order and entry for Change work area.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    use crate::state::context::ContextPhase;

    let ctx = &state.context_ops;
    let close_msg = (!matches!(ctx.phase, ContextPhase::Switching { .. }))
        .then_some(Message::Context(ContextMessage::BulkModalClosed));
    let close_target = FocusTarget::control(focus_target::CLOSE);

    match &ctx.phase {
        ContextPhase::Idle | ContextPhase::LoadingList(_) | ContextPhase::Switching { .. } => {
            LayerFocus {
                order: Vec::new(),
                entry: None,
            }
        }
        ContextPhase::BrowsingList { .. } => {
            let search_field =
                FocusTarget::text_input(knotra_ui::widget::focus_id::SWITCH_TARGET.clone());
            let mut order = vec![
                (close_target, close_msg.clone()),
                (search_field.clone(), None),
            ];
            for candidate in ctx.filtered_candidates() {
                let target =
                    FocusTarget::control_dynamic(format!("switch.candidate.{}", candidate.label));
                let msg = (!candidate.is_current).then_some(Message::Context(
                    ContextMessage::SwitchTargetChosen(
                        // `project_id` lives on the phase, not the candidate —
                        // read it back from the phase to avoid threading a
                        // second copy through `filtered_candidates`.
                        match &ctx.phase {
                            ContextPhase::BrowsingList { project_id, .. } => project_id.clone(),
                            _ => unreachable!("matched above"),
                        },
                        candidate.target.clone(),
                        candidate.label.clone(),
                    ),
                ));
                order.push((target, msg));
            }
            order.push((
                FocusTarget::control(focus_target::BROWSING_CANCEL),
                close_msg,
            ));
            LayerFocus {
                order,
                entry: Some(search_field),
            }
        }
        ContextPhase::ConfirmSwitch {
            disabled_reason_key,
            ..
        } => {
            let switch_msg = (disabled_reason_key.is_none()
                && !state.operation_interlock.is_busy())
            .then_some(Message::Context(ContextMessage::SwitchConfirmed));
            let cancel = FocusTarget::control(focus_target::CONFIRM_CANCEL);
            LayerFocus {
                order: vec![
                    (close_target, close_msg),
                    (
                        FocusTarget::control(focus_target::CONFIRM_SWITCH),
                        switch_msg,
                    ),
                    (
                        cancel.clone(),
                        Some(Message::Context(ContextMessage::SwitchCancelled)),
                    ),
                ],
                // Never the entry: `SwitchConfirmed` starts a change (D2).
                entry: Some(cancel),
            }
        }
        ContextPhase::Done(_) => {
            let footer_close = FocusTarget::control(focus_target::FOOTER_CLOSE);
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
            column![
                text(state.t("plain.switch.no_project"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::LoadingList(_) => (
            column![
                text(state.t("plain.status.checking"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                text(state.t("plain.switch.loading_hint"))
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::BrowsingList {
            project_id, search, ..
        } => {
            let search_field = guided_field_focused(
                tokens,
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
                rows = rows.push(
                    text(state.t("plain.switch.no_targets"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                );
            } else {
                let mut list = column![].spacing(4);
                for candidate in candidates {
                    let reason_key = candidate
                        .is_current
                        .then_some("plain.switch.reason_current");
                    let kind = state.t(context_target_kind_key(&candidate.target));
                    let detail = candidate.target.display_target();
                    let label = column![
                        text(candidate.label.as_str())
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                        text(format!("{kind} · {detail}"))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens
                            )),
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
                    let candidate_focused = crate::app::ring(
                        state,
                        &FocusTarget::control_dynamic(format!(
                            "switch.candidate.{}",
                            candidate.label
                        )),
                    );
                    let candidate_btn = iced::widget::button(label)
                        .width(Length::Fill)
                        .on_press_maybe(press)
                        .style(move |_theme, status| {
                            style::with_focus_ring(&t, candidate_focused, style::ghost(&t, status))
                        });
                    let mut row_col = column![candidate_btn];
                    if let Some(reason_key) = reason_key {
                        row_col = row_col.push(
                            text(state.t(reason_key))
                                .size(snora::design::style::text::body_small_size(tokens))
                                .line_height(snora::design::style::text::body_small_line_height(
                                    tokens,
                                )),
                        );
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
                    crate::app::ring(state, &FocusTarget::control(focus_target::BROWSING_CANCEL)),
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
                    crate::app::ring(state, &FocusTarget::control(focus_target::CONFIRM_SWITCH)),
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Context(ContextMessage::SwitchCancelled)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::CONFIRM_CANCEL)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    text(project_name)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    text(target_label)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    text(state.t(context_target_kind_key(target)))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                    text(caution)
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
        }

        ContextPhase::Switching { target_label, .. } => (
            column![
                text(state.t("plain.switch.working"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                text(target_label)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
                text(title).size(snora::design::style::text::title_size(tokens)),
                text(result.target.as_str())
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
                text(body)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8);

            if !result.operation_result.success && state.show_op_details {
                if let Some(hint) = &result.recovery_hint {
                    detail_col = detail_col.push(
                        text(hint.situation.as_str())
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                    for cmd in &hint.suggested_commands {
                        detail_col = detail_col.push(
                            text(format!("  {}", cmd))
                                .size(snora::design::style::text::body_small_size(tokens))
                                .line_height(snora::design::style::text::body_small_line_height(
                                    tokens,
                                )),
                        );
                    }
                }
                for cmd in &result.operation_result.commands_executed {
                    detail_col = detail_col.push(
                        text(format!("  {}", cmd))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
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
                    crate::app::ring(state, &FocusTarget::control(focus_target::DETAILS)),
                    style::ghost,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.close"),
                    Some(Message::Context(ContextMessage::BulkModalClosed)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::FOOTER_CLOSE)),
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
        OverlayWidth::Large.resolve(state.window_width),
        state.t("plain.change_work_area"),
        close_msg,
        crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE)),
        inner,
        footer,
    )
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (Stage 2),
/// `changelog.rs` (Stage 3), and `freezer.rs` (Stage 4) use.
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
