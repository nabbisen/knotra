#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0016 — Keyboard shortcuts cheat-sheet overlay.
//!
//! RFC-049: `BINDINGS`' `context`/`desc` fields were `&'static str` English
//! text, invisible to RFC-048's text-outside-the-catalog guard because they
//! are field accesses, not literal call arguments (that guard's own doc
//! comment names this as a blind spot). `Binding` now stores `context_key`/
//! `desc_key` — catalog keys, resolved at render time — the same
//! `label_key` shape `StatusSummary` and `RetryExclusionReason::i18n_key()`
//! already use. `keys` stays a literal (D1): `Esc`, `Ctrl`, `⌘` are
//! hardware legends, identical on a Japanese keyboard, and translating them
//! would break the correspondence between the overlay and the key being
//! hunted for.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use crate::{
    message::{KeyboardMessage, Message},
    state::AppState,
};

struct Binding {
    keys: &'static str,
    context_key: &'static str,
    desc_key: &'static str,
}

const BINDINGS: &[Binding] = &[
    Binding {
        keys: "Ctrl+K / ⌘K",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_open_palette",
    },
    Binding {
        keys: "?",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_toggle_cheatsheet",
    },
    Binding {
        keys: "Ctrl+R",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_refresh_workspace",
    },
    Binding {
        keys: "⌘1 … ⌘9",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_switch_workspace_by_index",
    },
    Binding {
        keys: "/",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_focus_search",
    },
    Binding {
        keys: "↑ / ↓ / j / k",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_move_focus",
    },
    Binding {
        keys: "Space",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_toggle_card_selection",
    },
    Binding {
        keys: "Shift+Space",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_range_select",
    },
    Binding {
        keys: "Ctrl+A / ⌘A",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_select_all",
    },
    Binding {
        keys: "Esc",
        context_key: "shortcut.ctx_dashboard",
        desc_key: "shortcut.desc_clear_selection",
    },
    Binding {
        keys: "f",
        context_key: "shortcut.ctx_selection",
        desc_key: "shortcut.desc_fetch",
    },
    Binding {
        keys: "p",
        context_key: "shortcut.ctx_selection",
        desc_key: "shortcut.desc_pull",
    },
    Binding {
        keys: "t",
        context_key: "shortcut.ctx_selection",
        desc_key: "shortcut.desc_tag",
    },
    Binding {
        keys: "b",
        context_key: "shortcut.ctx_selection",
        desc_key: "shortcut.desc_switch_branch",
    },
    Binding {
        keys: "g h",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_goto_history",
    },
    Binding {
        keys: "g s",
        context_key: "shortcut.ctx_global",
        desc_key: "shortcut.desc_goto_settings",
    },
    Binding {
        keys: "Esc",
        context_key: "shortcut.ctx_modal",
        desc_key: "shortcut.desc_close_modal",
    },
    Binding {
        keys: "↑ / ↓",
        context_key: "shortcut.ctx_palette",
        desc_key: "shortcut.desc_navigate_results",
    },
    Binding {
        keys: "Enter",
        context_key: "shortcut.ctx_palette",
        desc_key: "shortcut.desc_confirm_entry",
    },
];

pub fn view(state: &AppState) -> Element<'_, Message> {
    // RFC-049 §2: `action.close` already exists in both catalogs — reused,
    // not duplicated. The glyph stays a literal; only the word is resolved.
    let close_btn = button(text(format!("✕  {}", state.t("action.close"))).size(12))
        .on_press(Message::KeyEvent(KeyboardMessage::CheatSheetToggled));

    let header = row![
        text(state.t("shortcut.overlay_title")).size(15),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([0, 0]);

    let col_header = row![
        text(state.t("shortcut.column_keys")).size(11),
        text(state.t("shortcut.column_context")).size(11),
        text(state.t("shortcut.column_action")).size(11),
    ]
    .spacing(16);

    let rows: Vec<Element<'_, Message>> = BINDINGS
        .iter()
        .map(|b| {
            row![
                text(b.keys).size(12),
                text(state.t(b.context_key)).size(11),
                text(state.t(b.desc_key)).size(12),
            ]
            .spacing(16)
            .into()
        })
        .collect();

    container(
        column![header, col_header, column(rows).spacing(4),]
            .spacing(8)
            .padding(20),
    )
    .width(Length::Fixed(600.0))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_ui::i18n::{Catalog, Locale};

    /// RFC-049 D4/R6: `every_literal_t_call_names_an_existing_key` only sees
    /// literal `t(...)` calls written with the key inline — resolving
    /// `b.context_key`/`b.desc_key` through a variable is invisible to it,
    /// the same gap RFC-038 needed a dedicated guard for (`label_en`,
    /// `061`/`062`). Driven from `BINDINGS` itself, not a hand-copied list
    /// of the 24 keys, so a new row added without adding its keys fails
    /// here rather than shipping a key rendered as its own name.
    #[test]
    fn every_binding_key_resolves_in_both_catalogs() {
        let en = Catalog::for_locale(Locale::En);
        let ja = Catalog::for_locale(Locale::Ja);

        let mut missing = Vec::new();
        for b in BINDINGS {
            for key in [b.context_key, b.desc_key] {
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
