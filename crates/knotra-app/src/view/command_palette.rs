#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0012 — Command palette overlay view.
//!
//! Rendered as a floating centered modal when `state.palette.open == true`.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text, text_input},
};

use crate::{
    message::{Message, PaletteMessage},
    state::{AppState, PaletteEntryKind},
};

/// Render the palette overlay (call only when `state.palette.open`).
pub fn view(state: &AppState) -> Element<'_, Message> {
    let input = text_input(state.t("palette.search_placeholder"), &state.palette.query)
        .id(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
        .on_input(|s| Message::Palette(PaletteMessage::QueryChanged(s)))
        .padding([8, 12])
        .size(14);

    let results: Vec<Element<'_, Message>> = state
        .palette
        .results
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let prefix = match entry.kind {
                PaletteEntryKind::Action => "⚡ ",
                PaletteEntryKind::Project => "⎇  ",
                PaletteEntryKind::Workspace => "⊞  ",
            };
            let label = format!("{}{}", prefix, entry.label);
            let highlighted = i == state.palette.highlighted;
            let btn = button(text(label).size(13))
                .on_press_maybe(
                    entry
                        .disabled_reason_key
                        .is_none()
                        .then_some(Message::Palette(PaletteMessage::EntryClicked(i))),
                )
                .width(Length::Fill);
            let mut row = column![btn].spacing(2);
            if let Some(reason_key) = entry.disabled_reason_key {
                row = row.push(text(state.t(reason_key)).size(11));
            }
            let _ = highlighted; // styling hook for later theming
            row.into()
        })
        .collect();

    let close_btn =
        button(text("✕ Esc").size(11)).on_press(Message::Palette(PaletteMessage::Closed));

    let header = row![
        text(state.t("palette.title")).size(13),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([4, 0]);

    let mut body = column![header, input].spacing(6);
    if let Some(notice_key) = state.palette.notice_key {
        body = body.push(text(state.t(notice_key)).size(12));
    }
    if !results.is_empty() {
        body = body.push(column(results).spacing(2));
    } else if !state.palette.query.is_empty() {
        body = body.push(text(state.t("palette.no_matches")).size(12));
    }

    container(body.padding(16))
        .width(Length::Fixed(480.0))
        .into()
}
