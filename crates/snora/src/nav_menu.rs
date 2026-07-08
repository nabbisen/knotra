//! Navigation menu primitives for knotra.
//!
//! Provides a horizontal `nav_bar` used as the main app navigation below
//! the workspace tabs.  Each [`NavItem`] is a labelled button that
//! highlights when active.

use iced::{
    widget::{button, container, row, text},
    Alignment, Element, Length, Padding,
};

/// Height of the navigation bar in pixels.
pub const NAV_BAR_HEIGHT: f32 = 36.0;

/// A single navigation entry.
pub struct NavItem<'a, Message: Clone> {
    pub label:    &'a str,
    pub active:   bool,
    pub message:  Message,
}

/// Build a horizontal navigation bar from a slice of items.
///
/// Active item is visually distinguished (underline / bold).
/// All items are full-height buttons for easy clicking.
pub fn nav_bar<'a, Message: Clone + 'a>(
    items: Vec<NavItem<'a, Message>>,
) -> Element<'a, Message> {
    let btns: Vec<Element<'a, Message>> = items
        .into_iter()
        .map(|item| {
            let label = if item.active {
                text(format!("• {}", item.label)).size(13)
            } else {
                text(item.label).size(13)
            };
            let mut btn = button(label).padding([4, 14]);
            if !item.active {
                btn = btn.on_press(item.message);
            }
            btn.into()
        })
        .collect();

    container(
        row(btns)
            .spacing(2)
            .align_y(Alignment::Center)
            .padding(Padding { top: 0.0, bottom: 0.0, left: 8.0, right: 8.0 }),
    )
    .width(Length::Fill)
    .height(NAV_BAR_HEIGHT)
    .into()
}
