#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0009 — Selection bar view.
//!
//! Rendered as a sticky row at the bottom of the main content area whenever
//! ≥ 1 project is selected.  Displays the count and primary action buttons.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, guided_button};

use crate::{
    message::{
        ActivityMessage, ContextMessage, FreezerMessage, Message, SelectionMessage, SyncMessage,
    },
    state::AppState,
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
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
    let fetch_msg = (count > 0 && !summary.fetchable_ids.is_empty())
        .then_some(Message::Sync(SyncMessage::BulkFetchRequested));
    let fetch_reason = if count == 0 {
        Some(choose_reason)
    } else if summary.fetchable_ids.is_empty() {
        Some(state.t("plain.selection.none_fetchable"))
    } else {
        None
    };
    let fetch_btn = guided_button(state.t("plain.check_for_updates"), fetch_msg, fetch_reason);

    let pull_reason = if count == 0 {
        Some(choose_reason)
    } else if !summary.has_upstream {
        Some(state.t("plain.disabled.no_upstream"))
    } else {
        None
    };
    let pull_btn = guided_button(
        state.t("plain.get_latest"),
        (count > 0 && summary.has_upstream)
            .then_some(Message::Sync(SyncMessage::BulkPullRequested)),
        pull_reason,
    );

    let tag_btn = guided_button(
        state.t("plain.save_release_point"),
        (count > 0).then_some(Message::Freezer(FreezerMessage::BulkOpenRequested)),
        (count == 0).then_some(choose_reason),
    );

    let switch_btn = guided_button(
        state.t("plain.change_work_area"),
        (count == 1).then_some(Message::Context(ContextMessage::BulkOpenRequested)),
        if count == 0 {
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

    let bar = container(
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            fetch_btn,
            pull_btn,
            tag_btn,
            switch_btn,
            clear_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([6, 12]),
    )
    .width(Length::Fill);

    Some(bar.into())
}
