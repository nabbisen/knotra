#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0009 — Selection bar view.
//!
//! Rendered as a sticky row at the bottom of the main content area whenever
//! ≥ 1 project is selected.  Displays the count and primary action buttons.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, text},
};

use knotra_ui::widget::BUTTON_HEIGHT;

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

    let count = state.selection.len();
    let label = format!("✓  {}  selected", count);

    // Determine which buttons are applicable.
    let has_upstream = state
        .workspace_status
        .as_ref()
        .map(|ws| {
            ws.projects
                .iter()
                .any(|ps| state.selection.contains(&ps.project_id) && ps.remote.upstream.is_some())
        })
        .unwrap_or(false);

    // Plain-language, goal-oriented labels. Expert terms (Fetch / Pull / Tag /
    // Switch) remain available behind "Show details" in result views.
    let fetch_btn = button(text(format!("⤓  {}", state.t("plain.check_for_updates"))).size(13))
        .height(BUTTON_HEIGHT)
        .on_press(Message::Sync(SyncMessage::BulkFetchRequested));

    let pull_btn = button(text(format!("⤒  {}", state.t("plain.get_latest"))).size(13))
        .height(BUTTON_HEIGHT)
        .on_press_maybe(if has_upstream {
            Some(Message::Sync(SyncMessage::BulkPullRequested))
        } else {
            None
        });

    let tag_btn = button(text(format!("⊘  {}", state.t("plain.save_release_point"))).size(13))
        .height(BUTTON_HEIGHT)
        .on_press(Message::Freezer(FreezerMessage::BulkOpenRequested));

    let switch_btn = button(text(format!("⇄  {}", state.t("plain.change_work_area"))).size(13))
        .height(BUTTON_HEIGHT)
        .on_press(Message::Context(ContextMessage::BulkOpenRequested));

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
