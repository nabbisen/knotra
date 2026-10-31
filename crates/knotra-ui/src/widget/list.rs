//! RFC-071 D8a: the one list style for all of knotra.
//!
//! Every list of items — the dashboard's project rows, History's entries,
//! the lists inside windows (the ready check, the "Get latest safely"
//! plan and its results, the command palette's results) — separates its
//! items with a 1px line in the `border` token, never above the first item
//! or below the last. No list item ever sits on its own panel:
//! `raised_card`/`overlay::surface` keep exactly one meaning — something
//! floating above the screen (a window, a menu, a tooltip) — not a row in
//! a list.
//!
//! **Why a line, not a panel** (against the owner's design philosophy,
//! RFC-071 D8a): a panel already means "a window, a menu or a tooltip";
//! giving every row one too would let one visual device carry two
//! meanings, and would stack panels inside panels for a list drawn inside
//! a window. A line survives a high-contrast theme (RFC-070) — drawn in
//! `border`, which such a theme strengthens — where a panel's only edge is
//! a surface tint that can vanish. A line also costs nothing beyond its
//! own pixel; padding and a visible edge on every row would work against
//! density.

use iced::widget::{Column, rule};
use iced::{Color, Element};
use snora::design::Tokens;

fn to_iced(color: snora::design::Color) -> Color {
    snora::design::style::color::to_iced_color(color)
}

/// Lays `items` out as a column, with a 1px `border`-coloured line between
/// each pair of adjacent items — none above the first, none below the
/// last. No spacing of its own beyond the line itself; an item's own
/// internal padding is that item's call site's concern, not this helper's.
#[must_use]
pub fn list<'a, Message: 'a>(
    tokens: &Tokens,
    items: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let line_color = to_iced(tokens.palette.border);
    let last_index = items.len().saturating_sub(1);

    let mut column = Column::new();
    for (index, item) in items.into_iter().enumerate() {
        column = column.push(item);
        if index != last_index {
            column = column.push(rule::horizontal(1.0).style(move |_theme| rule::Style {
                color: line_color,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: true,
            }));
        }
    }
    column.into()
}
