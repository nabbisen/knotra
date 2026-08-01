//! Select (dropdown) primitive, built from `KnotraTheme` (RFC-035 Handoff
//! 019 §7.2). `snora` ships no select styling — `snora-widgets-0.25.2`'s
//! `design/` module has `button`, `card`, `chip`, `notice`, `progress`, no
//! `select` — so this is written directly from the RFC-033 D7 roles:
//! surface, border, text, and accent for the open/selected row.
//!
//! The focus ring reuses [`super::buttons::style::ring_color_for`] — the
//! same RFC-036 Stage 6 decision `with_focus_ring` applies to buttons —
//! rather than a second implementation. `with_focus_ring` itself cannot be
//! called directly: it is typed to `iced::widget::button::Style`, and
//! `pick_list_widget::Style` / `menu::Style` are different shapes (no `shadow` or
//! `snap` field, an extra `placeholder_color`/`handle_color` on the field,
//! and a `selected_background`/`selected_text_color` pair on the menu).
//! `ring_color_for` only needs a background and returns a colour, so it
//! composes with any of them.

use iced::widget::overlay::menu;
use iced::widget::pick_list as pick_list_widget;
use snora::design::Tokens;

use super::buttons::style::ring_color_for;
use super::layout::Element;

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// A themed select menu. The focus ring is drawn on both the closed control
/// and the open menu when `is_focused` is true (Handoff 019 §7.2: "the
/// focus ring must be visible on the closed control and on the open menu").
#[must_use]
pub fn pick_list<'a, T, Message>(
    tokens: &Tokens,
    options: &'a [T],
    selected: Option<T>,
    is_focused: bool,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    Message: Clone + 'a,
{
    let t_field = tokens.clone();
    let t_menu = tokens.clone();

    pick_list_widget::PickList::new(options, selected, on_select)
        .style(move |_theme, status| field_style(&t_field, status, is_focused))
        .menu_style(move |_theme| menu_style(&t_menu, is_focused))
        .into()
}

/// The closed (and opening) control's style: `surface` background,
/// `text_primary` text, `border` at rest, `accent` border when hovered or
/// open — then the focus ring on top when `is_focused`.
fn field_style(
    tokens: &Tokens,
    status: pick_list_widget::Status,
    is_focused: bool,
) -> pick_list_widget::Style {
    let border_color = match status {
        pick_list_widget::Status::Active => to_iced(tokens.palette.border),
        pick_list_widget::Status::Hovered | pick_list_widget::Status::Opened { .. } => {
            to_iced(tokens.palette.accent)
        }
    };

    let base = pick_list_widget::Style {
        text_color: to_iced(tokens.palette.text_primary),
        placeholder_color: to_iced(tokens.palette.text_muted),
        handle_color: to_iced(tokens.palette.text_secondary),
        background: iced::Background::Color(to_iced(tokens.palette.surface)),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
    };

    if !is_focused {
        return base;
    }

    pick_list_widget::Style {
        border: iced::Border {
            color: ring_color_for(tokens, Some(base.background)),
            width: tokens.focus.ring_width,
            radius: base.border.radius,
        },
        ..base
    }
}

/// The open menu's style: `surface_raised` background (matching
/// `overlay::raised_card`'s elevated treatment), `accent`/`accent_text` for
/// the selected row — then the focus ring on top when `is_focused`, since
/// the field and the menu are the same logical control for keyboard-focus
/// purposes.
fn menu_style(tokens: &Tokens, is_focused: bool) -> menu::Style {
    let base = menu::Style {
        background: iced::Background::Color(to_iced(tokens.palette.surface_raised)),
        border: iced::Border {
            color: to_iced(tokens.palette.border),
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
        text_color: to_iced(tokens.palette.text_primary),
        selected_text_color: to_iced(tokens.palette.accent_text),
        selected_background: iced::Background::Color(to_iced(tokens.palette.accent)),
        shadow: iced::Shadow::default(),
    };

    if !is_focused {
        return base;
    }

    menu::Style {
        border: iced::Border {
            color: ring_color_for(tokens, Some(base.background)),
            width: tokens.focus.ring_width,
            radius: base.border.radius,
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same shape as `buttons.rs`'s
    /// `filled_accent_style_gets_a_different_ring_color_than_a_transparent_one`
    /// — RFC-035 Handoff 019 §8's required per-primitive check that focused
    /// and unfocused styles differ, which is what would have caught
    /// RFC-036's invisible-ring defect earlier.
    #[test]
    fn field_focused_and_unfocused_styles_differ() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            let unfocused = field_style(&tokens, pick_list_widget::Status::Active, false);
            let focused = field_style(&tokens, pick_list_widget::Status::Active, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }

    #[test]
    fn menu_focused_and_unfocused_styles_differ() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            let unfocused = menu_style(&tokens, false);
            let focused = menu_style(&tokens, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }
}
