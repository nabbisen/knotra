// RFC-052 A1: `unused_imports`/`unused_variables` masked nothing in any
// target and are gone. `dead_code` is narrowed to the test build only —
// `view()` (and everything it alone reaches) is called from `view.rs` in
// the real binary, but no `#[test]` in this crate calls into the
// render tree, so the test compilation's call graph never reaches it and
// flags it as dead. The binary build carries no suppression at all.
#![cfg_attr(test, allow(dead_code))]
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
    let tokens = &state.theme.tokens;
    let input = text_input(state.t("palette.search_placeholder"), &state.palette.query)
        .id(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
        .on_input(|s| Message::Palette(PaletteMessage::QueryChanged(s)))
        .padding([8, 12])
        .size(snora::design::style::text::label_size(tokens));

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
            let btn = button(
                text(label)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
            )
            .on_press_maybe(
                entry
                    .disabled_reason_key
                    .is_none()
                    .then_some(Message::Palette(PaletteMessage::EntryClicked(i))),
            )
            .width(Length::Fill);
            let mut row = column![btn].spacing(2);
            if let Some(reason_key) = entry.disabled_reason_key {
                row = row.push(
                    text(state.t(reason_key))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                );
            }
            let _ = highlighted; // styling hook for later theming
            row.into()
        })
        .collect();

    let close_btn = button(
        text("✕ Esc")
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Palette(PaletteMessage::Closed));

    let header = row![
        text(state.t("palette.title"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([4, 0]);

    let mut body = column![header, input].spacing(6);
    if let Some(notice_key) = state.palette.notice_key {
        body = body.push(
            text(state.t(notice_key))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }
    if !results.is_empty() {
        body = body.push(column(results).spacing(2));
    } else if !state.palette.query.is_empty() {
        body = body.push(
            text(state.t("palette.no_matches"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }

    container(body.padding(16))
        .width(Length::Fixed(480.0))
        .into()
}
