//! RFC-0009 — Selection bar view.
//!
//! Rendered as a sticky row at the bottom of the main content area whenever
//! ≥ 1 project is selected. Displays the count and primary action buttons.
//!
//! RFC-035 R13/R14/Handoff 030 §6: the four actions used to each carry
//! their own `guided_button` reason cascade, so "Busy" or "Choose at least
//! one project" appeared up to four times in one viewport (Stage 4's own
//! 2x2 capture showed exactly this — `060` finding 5). R13's shared
//! reasons (busy, and choose-at-least-one) now render once, in a single
//! group-level slot below the count; R14's action-specific reasons (fetch
//! has nothing checkable, pull has no upstream, switch needs exactly one
//! project) render in one contextual slot beneath the actions, each
//! labelled by its own action, rather than per-button. Neither is dropped —
//! R14 requires they "remain available", just consolidated rather than
//! repeated.
//!
//! This moves the four actions off `guided_button` (whose
//! reason-beneath-itself composition is exactly what created the
//! duplication) onto a plain button — the same narrow migration
//! `select_mode_button` made in Stage 2 for the same reason.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL};

use crate::{
    message::{ContextMessage, FreezerMessage, Message, SelectionMessage, SyncMessage},
    state::AppState,
    view::dashboard::WidthMode,
};

pub fn view(state: &AppState, mode: WidthMode) -> Option<Element<'_, Message>> {
    // Selection bar only shown while in selection mode.
    if !state.selection_mode {
        return None;
    }

    let summary = state.selection_summary();
    let count = summary.selected_count;
    let label = if count == 0 {
        state.t("plain.selection.none").to_owned()
    } else {
        format!(
            "✓  {}  {}",
            count,
            state.t("plain.selection.selected_suffix")
        )
    };

    let busy = state.operation_interlock.is_busy();

    // R13: one shared reason for the whole group — busy takes precedence
    // over count-zero, matching the cascade order every action already
    // used individually before this change.
    let group_reason = if busy {
        Some(state.t("plain.activity.busy"))
    } else if count == 0 {
        Some(state.t("plain.disabled.choose_one"))
    } else {
        None
    };

    let can_act = count > 0 && !busy;
    let fetch_msg = (can_act && !summary.fetchable_ids.is_empty())
        .then_some(Message::Sync(SyncMessage::BulkFetchRequested));
    let pull_msg =
        (can_act && summary.has_upstream).then_some(Message::Sync(SyncMessage::BulkPullRequested));
    let tag_msg = can_act.then_some(Message::Freezer(FreezerMessage::BulkOpenRequested));
    let switch_msg =
        (count == 1 && !busy).then_some(Message::Context(ContextMessage::BulkOpenRequested));

    // R14: action-specific reasons, shown only when the shared (group)
    // reason above does not already explain the disablement. Each is
    // labelled by its own action so more than one can be listed at once
    // without ambiguity — one contextual slot, not a fourth `guided_button`
    // repeating the busy/choose-one text it would otherwise still carry.
    let mut contextual_reasons: Vec<String> = Vec::new();
    if can_act {
        if summary.fetchable_ids.is_empty() {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.check_for_updates"),
                state.t("plain.selection.none_fetchable")
            ));
        }
        if !summary.has_upstream {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.get_latest"),
                state.t("plain.disabled.no_upstream")
            ));
        }
        if count > 1 {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.change_work_area"),
                state.t("plain.selection.choose_one_work_area")
            ));
        }
    }

    let fetch_btn = action_button(state.t("plain.check_for_updates"), fetch_msg);
    let pull_btn = action_button(state.t("plain.get_latest"), pull_msg);
    let tag_btn = action_button(state.t("plain.save_release_point"), tag_msg);
    let switch_btn = action_button(state.t("plain.change_work_area"), switch_msg);

    let clear_btn = button(text(state.t("plain.exit_selection")).size(13))
        .height(BUTTON_HEIGHT)
        .on_press(Message::Selection(SelectionMessage::ModeExited));

    let command_row = row![
        text(label).size(13),
        Space::new().width(Length::Fill),
        clear_btn,
    ]
    .align_y(Alignment::Center);

    // RFC-035 R8/Handoff 028 §4: a 2x2 grid at compact width, chosen over an
    // action menu — these four actions are already always-visible in
    // standard mode, so hiding them behind a menu would cost an extra click
    // for something users already expect at a glance; two rows of two keeps
    // every action visible and reachable, just narrower. Standard/wide keep
    // the single four-wide row unchanged.
    let actions: Element<'_, Message> = match mode {
        WidthMode::Compact => column![
            row![
                container(fetch_btn).width(Length::FillPortion(1)),
                container(pull_btn).width(Length::FillPortion(1)),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
            row![
                container(tag_btn).width(Length::FillPortion(1)),
                container(switch_btn).width(Length::FillPortion(1)),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        ]
        .spacing(8)
        .into(),
        WidthMode::Standard | WidthMode::Wide => row![
            container(fetch_btn).width(Length::FillPortion(1)),
            container(pull_btn).width(Length::FillPortion(1)),
            container(tag_btn).width(Length::FillPortion(1)),
            container(switch_btn).width(Length::FillPortion(1)),
        ]
        .spacing(8)
        .align_y(Alignment::Start)
        .into(),
    };

    let mut content = column![command_row].spacing(6);
    if let Some(reason) = group_reason {
        content = content.push(text(reason).size(FONT_SMALL));
    }
    content = content.push(actions);
    for reason in contextual_reasons {
        content = content.push(text(reason).size(FONT_SMALL));
    }

    let bar = container(content.padding([6, 12])).width(Length::Fill);

    Some(bar.into())
}

/// A plain selection-bar action button, without `guided_button`'s
/// reason-beneath-itself composition — R13/R14 now render the reason
/// (shared or action-specific) in a slot outside the button, so a copy
/// here would just reintroduce the duplication this migration removes.
fn action_button<'a>(label: &'a str, on_press: Option<Message>) -> Element<'a, Message> {
    button(text(label).size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 18])
        .on_press_maybe(on_press)
        .into()
}
