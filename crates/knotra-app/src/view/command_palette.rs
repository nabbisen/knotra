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
    Element, Length,
    widget::{Space, button, column, text},
};

use knotra_ui::widget::{
    SMALL_BUTTON_HEIGHT, centered_button_label, list,
    overlay::{OverlayWidth, surface},
    style, styled_text_input,
};

use crate::{
    message::{Message, PaletteMessage},
    state::{AppState, PaletteEntryKind},
};

/// A result button's own padding, set explicitly rather than left to iced's
/// default — the value is unchanged (`iced_widget::button`'s own
/// `DEFAULT_PADDING`), but now named so the disabled-reason indent below can
/// read its `left` field instead of carrying a second copy of `10` that
/// nothing would keep in step with it (Review 241 §3, Task 100 A1).
const RESULT_BUTTON_PADDING: iced::Padding = iced::Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};

/// RFC-075 stage 3 §3: the palette's own entry — the search field, always
/// (§4's table has one row for this window, no phase-dependent entries).
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    state.palette.open.then(crate::view::ids::palette::query)
}

/// Render the palette overlay (call only when `state.palette.open`).
///
/// Task 098 Amendment A2: content now goes through `surface()`, so this
/// renders directly into `AppLayout::dialog` (`view.rs`) instead of its
/// own centred `iced::widget::stack` layer — the opaque surface, the
/// full-window dim, and the scrim's click-to-close all come from the
/// engine now, the same as every other migrated window. The old header's
/// own "✕ Esc" button becomes `surface()`'s close — its "Esc" hint text
/// does not carry over, since `surface()`'s close is always a bare icon
/// (see `overlay.rs::surface`, unconditional across every caller).
pub fn view(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    // RFC-073 handoff Amendment A3 §2: `BUTTON_HEIGHT` through the shared
    // builder.
    let highlighted_on_submit = state
        .palette
        .highlighted
        .map(|i| Message::Palette(PaletteMessage::EntryClicked(i)));
    let input = styled_text_input(
        tokens,
        state.t("palette.search_placeholder"),
        &state.palette.query,
    )
    .id(crate::view::ids::palette::query())
    .on_input(|s| Message::Palette(PaletteMessage::QueryChanged(s)))
    .on_submit_maybe(highlighted_on_submit);

    let results: Vec<Element<'_, Message>> = state
        .palette
        .results
        .iter()
        .enumerate()
        .map(|(i, entry)| result_item(state, tokens, i, entry))
        .collect();

    // RFC-071 Amendment A3 D14: a notice and a "no matches" message — both
    // `body`.
    let mut body = column![input].spacing(6);
    if let Some(notice_key) = state.palette.notice_key {
        body = body.push(
            text(state.t(notice_key))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }
    if !results.is_empty() {
        // RFC-071 D8a: one border line between commands — each `results[i]`
        // already bundles a command with its own reason into one column,
        // so `list(...)`'s line falls only between those bundles, never
        // between a command and its reason.
        body = body.push(list(tokens, results));
    } else if !state.palette.query.is_empty() {
        body = body.push(
            text(state.t("palette.no_matches"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Standard.resolve(state.window_width),
            state.t("palette.title"),
            Some(Message::Palette(PaletteMessage::Closed)),
            crate::view::ids::palette::close(),
            body,
            Space::new(),
        ),
        &[],
    )
}

/// RFC-075 stage 3 §4: one result row. Not a snora control and not a Tab
/// stop — the arrows move [`AppState::palette`]'s own `highlighted` cursor
/// instead, drawn with snora's focus-ring tokens
/// ([`knotra_ui::widget::style::with_focus_ring`]) so it looks like focus
/// without being one. `SMALL_BUTTON_HEIGHT` is the same height guard every
/// other small control carries.
fn result_item<'a>(
    state: &'a AppState,
    tokens: &snora::design::Tokens,
    index: usize,
    entry: &'a crate::state::PaletteEntry,
) -> Element<'a, Message> {
    let prefix = match entry.kind {
        PaletteEntryKind::Action => "⚡ ",
        PaletteEntryKind::Project => "⎇  ",
        PaletteEntryKind::Workspace => "⊞  ",
    };
    let label = format!("{}{}", prefix, entry.label);
    let t = tokens.clone();
    let is_highlighted = state.palette.highlighted == Some(index);
    // RFC-071 Amendment A3 D14: a command's own label — `body`.
    // RFC-073 D8: `SMALL_BUTTON_HEIGHT` — `RESULT_BUTTON_PADDING`'s
    // `left`/`right` still govern the reason indent below; its
    // `top`/`bottom` no longer decide the row's height.
    let btn = button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(SMALL_BUTTON_HEIGHT)
    .on_press_maybe(
        entry
            .disabled_reason_key
            .is_none()
            .then_some(Message::Palette(PaletteMessage::EntryClicked(index))),
    )
    .width(Length::Fill)
    .padding(RESULT_BUTTON_PADDING)
    .style(move |_theme, status| {
        style::with_focus_ring(&t, is_highlighted, style::ghost(&t, status))
    });
    let mut row = column![btn].spacing(2);
    if let Some(reason_key) = entry.disabled_reason_key {
        // Task 100 §2 / Review 241 §3: `text_secondary` (a token,
        // never a literal — the theme's own WCAG AA contrast test
        // already covers it) replaces the main text colour, so the
        // reason no longer reads as an item of its own at the
        // window's edge. Indented by `RESULT_BUTTON_PADDING.left`,
        // the same named value the button above it is now set to
        // explicitly — not a second copy of the number — so the
        // reason starts where the command's own kind symbol starts,
        // inside the button's padding. It does not reach the
        // label's own start past the kind prefix ("⊞  " and the
        // like): that offset is baked into one `text()` run with the
        // label, and the three prefixes are neither the same glyph
        // nor the same length, so the extra distance cannot be
        // derived without measuring rendered text width — stopping
        // here rather than choosing a number for it (Review 241 §3
        // accepts the symbol's own start as the target, not the
        // label's).
        // RFC-071 Amendment A3 D14: a reason — `body`. The indent
        // math above is unaffected (it is derived from the button's
        // own padding, not the text's size).
        row = row.push(
            iced::widget::container(
                text(state.t(reason_key))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .color(snora::design::style::color::to_iced_color(
                        tokens.palette.text_secondary,
                    )),
            )
            .padding(iced::padding::left(RESULT_BUTTON_PADDING.left)),
        );
    }
    row.into()
}
