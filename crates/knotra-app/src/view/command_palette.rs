#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-012 — Command palette overlay view.
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
    let input = text_input(
        "Search actions, projects, workspaces…",
        &state.palette.query,
    )
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
                .on_press(Message::Palette(PaletteMessage::EntryClicked(i)))
                .width(Length::Fill);
            let _ = highlighted; // styling hook for later theming
            btn.into()
        })
        .collect();

    let close_btn =
        button(text("✕ Esc").size(11)).on_press(Message::Palette(PaletteMessage::Closed));

    let header = row![
        text("Command Palette").size(13),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([4, 0]);

    let mut body = column![header, input].spacing(6);
    if !results.is_empty() {
        body = body.push(column(results).spacing(2));
    } else if !state.palette.query.is_empty() {
        body = body.push(text("No matches.").size(12));
    }

    container(body.padding(16))
        .width(Length::Fixed(480.0))
        .into()
}
