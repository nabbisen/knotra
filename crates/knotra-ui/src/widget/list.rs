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

use iced::widget::{Column, keyed, rule};
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

/// A per-item key paired with [`list_keyed`]'s own divider rule, so the
/// rule after item `k` never collides with the key of the item itself.
#[derive(Clone, Copy, PartialEq)]
enum RowKey<Key> {
    Item(Key),
    Rule(Key),
}

/// Same look as [`list`], but reconciled by each item's own `key` rather
/// than its tree position (RFC-075 stage 4 §2).
///
/// A plain [`Column`] (what [`list`] builds) reuses a child's internal
/// state — including which row is *really* keyboard-focused — by
/// position: when an item is inserted or removed anywhere but the end,
/// every item after that point can be handed a different item's old
/// state, including a focus that was never really on it. `iced`'s own
/// `keyed::Column` exists exactly to avoid this ("adding an element at
/// the top may cause a loss of continuity on every element on the
/// list" — `iced_widget::keyed`'s own module docs): giving each item a
/// stable key lets it find and reuse its *own* old state wherever it
/// now sits, so a row's real focus travels with it across a reorder,
/// insertion, or removal.
///
/// Use this instead of [`list`] for any list whose own membership or
/// order can change while one of its rows holds real keyboard focus —
/// the two must still look identical, since both build the same
/// divider rule between items.
#[must_use]
pub fn list_keyed<'a, Key, Message>(
    tokens: &Tokens,
    items: Vec<(Key, Element<'a, Message>)>,
) -> Element<'a, Message>
where
    Key: Copy + PartialEq + 'static,
    Message: 'a,
{
    let line_color = to_iced(tokens.palette.border);
    let last_index = items.len().saturating_sub(1);

    let mut column = keyed::Column::new();
    for (index, (key, item)) in items.into_iter().enumerate() {
        column = column.push(RowKey::Item(key), item);
        if index != last_index {
            column = column.push(
                RowKey::Rule(key),
                Element::from(rule::horizontal(1.0).style(move |_theme| rule::Style {
                    color: line_color,
                    radius: 0.0.into(),
                    fill_mode: rule::FillMode::Full,
                    snap: true,
                })),
            );
        }
    }
    column.into()
}
