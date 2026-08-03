//! RFC-0009 — Selection bar view.
//!
//! Rendered as a sticky row at the bottom of the main content area whenever
//! ≥ 1 project is selected.  Displays the count and primary action buttons.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, guided_button};

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

    // Plain-language, goal-oriented labels. Expert terms (Fetch / Pull / Tag /
    // Switch) remain available behind "Show details" in result views.
    let choose_reason = state.t("plain.disabled.choose_one");
    let busy = state.operation_interlock.is_busy();
    let fetch_msg = (count > 0 && !summary.fetchable_ids.is_empty() && !busy)
        .then_some(Message::Sync(SyncMessage::BulkFetchRequested));
    let fetch_reason = if busy {
        Some(state.t("plain.activity.busy"))
    } else if count == 0 {
        Some(choose_reason)
    } else if summary.fetchable_ids.is_empty() {
        Some(state.t("plain.selection.none_fetchable"))
    } else {
        None
    };
    let fetch_btn = guided_button(state.t("plain.check_for_updates"), fetch_msg, fetch_reason);

    let pull_reason = if busy {
        Some(state.t("plain.activity.busy"))
    } else if count == 0 {
        Some(choose_reason)
    } else if !summary.has_upstream {
        Some(state.t("plain.disabled.no_upstream"))
    } else {
        None
    };
    let pull_btn = guided_button(
        state.t("plain.get_latest"),
        (count > 0 && summary.has_upstream && !busy)
            .then_some(Message::Sync(SyncMessage::BulkPullRequested)),
        pull_reason,
    );

    let tag_btn = guided_button(
        state.t("plain.save_release_point"),
        (count > 0 && !busy).then_some(Message::Freezer(FreezerMessage::BulkOpenRequested)),
        if busy {
            Some(state.t("plain.activity.busy"))
        } else {
            (count == 0).then_some(choose_reason)
        },
    );

    let switch_btn = guided_button(
        state.t("plain.change_work_area"),
        (count == 1 && !busy).then_some(Message::Context(ContextMessage::BulkOpenRequested)),
        if busy {
            Some(state.t("plain.activity.busy"))
        } else if count == 0 {
            Some(choose_reason)
        } else if count > 1 {
            Some(state.t("plain.selection.choose_one_work_area"))
        } else {
            None
        },
    );

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

    let bar =
        container(column![command_row, actions].spacing(6).padding([6, 12])).width(Length::Fill);

    Some(bar.into())
}
