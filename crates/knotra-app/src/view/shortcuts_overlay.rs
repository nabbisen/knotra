#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0016 — Keyboard shortcuts cheat-sheet overlay.

use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Element, Length,
};

use crate::{message::{KeyboardMessage, Message}, state::AppState};

struct Binding {
    keys:    &'static str,
    context: &'static str,
    desc:    &'static str,
}

const BINDINGS: &[Binding] = &[
    Binding { keys: "Ctrl+K / ⌘K", context: "Global",   desc: "Open command palette" },
    Binding { keys: "?",            context: "Global",   desc: "Show / hide this cheat sheet" },
    Binding { keys: "Ctrl+R",       context: "Global",   desc: "Refresh workspace" },
    Binding { keys: "⌘1 … ⌘9",     context: "Global",   desc: "Switch workspace by index" },
    Binding { keys: "/",            context: "Dashboard", desc: "Focus search field" },
    Binding { keys: "↑ / ↓ / j / k", context: "Dashboard", desc: "Move focus between cards" },
    Binding { keys: "Space",        context: "Dashboard", desc: "Toggle selection on focused card" },
    Binding { keys: "Shift+Space",  context: "Dashboard", desc: "Range-select to focused card" },
    Binding { keys: "Ctrl+A / ⌘A", context: "Dashboard", desc: "Select all visible projects" },
    Binding { keys: "Esc",          context: "Dashboard", desc: "Clear selection / close dialog" },
    Binding { keys: "f",            context: "Selection", desc: "Fetch selected projects" },
    Binding { keys: "p",            context: "Selection", desc: "Open Pull modal" },
    Binding { keys: "t",            context: "Selection", desc: "Open Tag modal" },
    Binding { keys: "b",            context: "Selection", desc: "Open Switch Branch modal" },
    Binding { keys: "g h",          context: "Global",   desc: "Go to History" },
    Binding { keys: "g s",          context: "Global",   desc: "Go to Settings" },
    Binding { keys: "Esc",          context: "Modal",    desc: "Close modal / palette" },
    Binding { keys: "↑ / ↓",        context: "Palette",  desc: "Navigate results" },
    Binding { keys: "Enter",        context: "Palette",  desc: "Confirm highlighted entry" },
];

pub fn view(state: &AppState) -> Element<'_, Message> {
    let close_btn = button(text("✕  Close").size(12))
        .on_press(Message::KeyEvent(KeyboardMessage::CheatSheetToggled));

    let header = row![
        text("Keyboard Shortcuts").size(15),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([0, 0]);

    let col_header = row![
        text("Keys").size(11),
        text("Context").size(11),
        text("Action").size(11),
    ]
    .spacing(16);

    let rows: Vec<Element<'_, Message>> = BINDINGS.iter().map(|b| {
        row![
            text(b.keys).size(12),
            text(b.context).size(11),
            text(b.desc).size(12),
        ]
        .spacing(16)
        .into()
    }).collect();

    let _ = state; // unused but kept for symmetry with other view functions

    container(
        column![
            header,
            col_header,
            column(rows).spacing(4),
        ]
        .spacing(8)
        .padding(20)
    )
    .width(Length::Fixed(600.0))
    .into()
}
