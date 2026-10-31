// RFC-052 A1: `unused_imports`/`unused_variables` masked nothing in any
// target and are gone. `dead_code` is narrowed to the test build only —
// `view()` (and `Binding::keys`, only read inside it) is called from
// `view.rs` in the real binary, but no `#[test]` in this crate calls into
// the render tree, so the test compilation's call graph never reaches it
// and flags both as dead. The binary build carries no suppression at all.
#![cfg_attr(test, allow(dead_code))]
//! RFC-0016 — Keyboard shortcuts cheat-sheet overlay.
//!
//! RFC-066 D7: the table became the RFC's own nine rows — exactly what
//! `app::shortcut_for` recognizes, not a hand-maintained list that could
//! drift from it. `app.rs`'s own test module asserts each row's example
//! key press maps through `shortcut_for` to the row's message, and that
//! every `ShortcutMessage` `shortcut_for` can produce is exercised by some
//! row (the one exception, `MousePressed`, is never produced by
//! `shortcut_for` — it comes from the separate mouse subscription, not a
//! keyboard row).
//!
//! RFC-049: `Binding`'s `where`/`action` fields are catalog keys, resolved
//! at render time — the same `label_key` shape `StatusSummary` and
//! `RetryExclusionReason::i18n_key()` already use, so RFC-048's
//! text-outside-the-catalog guard can see them. `keys` stays a literal
//! (D1): `Esc`, `Ctrl` are hardware legends, identical on a Japanese
//! keyboard, and translating them would break the correspondence between
//! the overlay and the key being hunted for.
//!
//! Task 098 Amendment A2: content goes through `surface()`, so this
//! renders into `AppLayout::dialog` (`view.rs`) instead of its own centred
//! `iced::widget::stack` layer — the opaque surface, the full-window dim,
//! and the scrim's click-to-close all come from the engine now. The old
//! header's own "✕  Close" button becomes `surface()`'s close; its
//! "Close" text (`action.close`) does not carry over, since `surface()`'s
//! close is always a bare icon.

use iced::{
    Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    list,
    overlay::{OverlayWidth, surface},
};

use crate::{
    message::{KeyboardMessage, Message},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "shortcuts.close";
}

/// RFC-066 D2: order and entry for the shortcuts sheet — just "Close".
pub fn layer_focus() -> LayerFocus<Message> {
    let close_target = FocusTarget::control(focus_target::CLOSE);
    LayerFocus {
        order: vec![(
            close_target.clone(),
            Some(Message::KeyEvent(KeyboardMessage::CheatSheetToggled)),
        )],
        entry: Some(close_target),
    }
}

pub(crate) struct Binding {
    pub(crate) keys: &'static str,
    where_key: &'static str,
    action_key: &'static str,
}

/// RFC D7's own nine rows, in the RFC's own order.
pub(crate) const BINDINGS: &[Binding] = &[
    Binding {
        keys: "Tab / Shift+Tab",
        where_key: "shortcut.where_everywhere",
        action_key: "shortcut.action_focus_move",
    },
    Binding {
        keys: "Enter / Space",
        where_key: "shortcut.where_everywhere",
        action_key: "shortcut.action_press",
    },
    Binding {
        keys: "Esc",
        where_key: "shortcut.where_everywhere",
        action_key: "shortcut.action_close_or_clear",
    },
    Binding {
        keys: "↑ / ↓",
        where_key: "shortcut.where_window",
        action_key: "shortcut.action_focus_move",
    },
    Binding {
        keys: "↑ / ↓ / j / k",
        where_key: "shortcut.ctx_dashboard",
        action_key: "shortcut.action_move_between_projects",
    },
    Binding {
        keys: "/ or Ctrl+/",
        where_key: "shortcut.ctx_dashboard",
        action_key: "shortcut.action_go_to_search",
    },
    Binding {
        keys: "Ctrl+R",
        where_key: "shortcut.where_main_screens",
        action_key: "shortcut.action_check_now",
    },
    Binding {
        keys: "Ctrl+K",
        where_key: "shortcut.where_main_screens",
        action_key: "shortcut.action_change_work_area",
    },
    Binding {
        keys: "Ctrl+T",
        where_key: "shortcut.where_main_screens",
        action_key: "shortcut.action_save_release_point",
    },
];

/// Column widths (Task 098 step 3): fixed, so the header and every row
/// line up. Chosen to comfortably hold the longest entry of each column
/// in both languages without wrapping — `KEYS_COL`/`WHERE_COL` fit their
/// longest literal/translated entry ("Tab / Shift+Tab" / "Main screens",
/// "メイン画面") on one line; `ACTION_COL` is wide enough for most rows'
/// sentences on one line, with the two longest ("Close the open window,
/// or clear the selection", "Move to the next or previous control")
/// wrapping onto a second line rather than forcing a much wider sheet for
/// their sake alone — checked by building and capturing both languages
/// (`.git-exclude/evidence/task-098/`), not by character count alone,
/// since a fixed-width CJK glyph measures roughly double a Latin one at
/// the same point size.
const KEYS_COL: f32 = 150.0;
const WHERE_COL: f32 = 130.0;
const ACTION_COL: f32 = 280.0;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    // RFC-071 Amendment A3 D14: this whole sheet is the "keyboard hints"
    // D14 names as its own kept example — header and every column of every
    // row stay `body_small`, not only the key-combo column. The fixed
    // `KEYS_COL`/`WHERE_COL`/`ACTION_COL` widths above were measured for
    // `body_small` text in both languages (Task 098); keeping the role
    // here keeps those measurements valid.
    let col_header = row![
        text(state.t("shortcut.column_keys"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .width(Length::Fixed(KEYS_COL)),
        text(state.t("shortcut.column_where"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .width(Length::Fixed(WHERE_COL)),
        text(state.t("shortcut.column_action"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .width(Length::Fixed(ACTION_COL)),
    ]
    .spacing(16);

    let rows: Vec<Element<'_, Message>> = BINDINGS
        .iter()
        .map(|b| {
            row![
                text(b.keys)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
                    .width(Length::Fixed(KEYS_COL)),
                text(state.t(b.where_key))
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
                    .width(Length::Fixed(WHERE_COL)),
                text(state.t(b.action_key))
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
                    .width(Length::Fixed(ACTION_COL)),
            ]
            .spacing(16)
            .into()
        })
        .collect();

    // RFC-071 D8a: the shortcuts are a list of items too — one border
    // line between adjacent rows, the header kept outside as it already
    // was.
    let body = column![col_header, list(tokens, rows)].spacing(8);

    surface(
        tokens,
        OverlayWidth::Large.resolve(state.window_width),
        state.t("shortcut.overlay_title"),
        Some(Message::KeyEvent(KeyboardMessage::CheatSheetToggled)),
        crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE)),
        body,
        Space::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_ui::i18n::{Catalog, Locale};

    /// RFC-049 D4/R6: `every_literal_t_call_names_an_existing_key` only sees
    /// literal `t(...)` calls written with the key inline — resolving
    /// `b.where_key`/`b.action_key` through a variable is invisible to it,
    /// the same gap RFC-038 needed a dedicated guard for (`label_en`,
    /// `061`/`062`). Driven from `BINDINGS` itself, not a hand-copied list
    /// of keys, so a new row added without adding its keys fails here
    /// rather than shipping a key rendered as its own name.
    #[test]
    fn every_binding_key_resolves_in_both_catalogs() {
        let en = Catalog::for_locale(Locale::En);
        let ja = Catalog::for_locale(Locale::Ja);

        let mut missing = Vec::new();
        for b in BINDINGS {
            for key in [b.where_key, b.action_key] {
                if !en.contains_key(key) {
                    missing.push(format!("{key} (missing from English)"));
                }
                if !ja.contains_key(key) {
                    missing.push(format!("{key} (missing from Japanese)"));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "these shortcut.* keys, referenced by BINDINGS, do not resolve: {missing:?}"
        );
    }
}
