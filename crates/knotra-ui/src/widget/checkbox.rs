//! Checkbox primitive, built from `KnotraTheme` (RFC-035 Handoff 019 §7.3).
//!
//! iced's own `checkbox::Checkbox` has a `size` setter but no `height`
//! setter — its layout is always `Length::Shrink` (`checkbox.rs`'s own
//! `size()` fn), so a drawn box alone cannot be widened into a 44px target
//! the way a button can. The interactive target here is a [`BOX_SIZE`]
//! drawn box centred inside a [`TARGET_SIZE`] `mouse_area`, matching
//! `layout::BUTTON_HEIGHT`'s existing 44px convention.
//!
//! A real `iced::widget::checkbox` still draws the box/check glyph and
//! still owns `on_toggle` directly, so a click landing on the small drawn
//! box is captured there (`checkbox.rs`'s `update()` calls
//! `shell.capture_event()`) and the wrapping `mouse_area` — which only acts
//! when the inner content did not capture the event
//! (`mouse_area.rs`'s `update()`) — supplies the same toggle for clicks in
//! the surrounding padding. No double-toggle: exactly one of the two fires
//! per click.
//!
//! Carries an `accessible_label`, shown as a hover tooltip, following
//! `buttons.rs`'s `icon_button_maybe` — the same reasoning applies here:
//! nothing on-screen names the control besides its checked state.

use iced::widget::{checkbox as iced_checkbox, container, mouse_area, text, tooltip};
use snora::design::Tokens;

use super::layout::{Element, FONT_SMALL, Length};
use super::ring::ring_color_for;

/// The drawn box size — the 44px target is the *interactive* area, not
/// necessarily the visible box (RFC-035 Handoff 019 §7.3).
const BOX_SIZE: f32 = 18.0;

/// The minimum interactive target, matching `layout::BUTTON_HEIGHT`.
const TARGET_SIZE: f32 = 44.0;

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// A themed checkbox with a 44px hit target around an 18px drawn box.
#[must_use]
pub fn checkbox<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    accessible_label: impl Into<String>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let toggle_message = on_toggle(!is_checked);

    let box_widget: Element<'a, Message> = iced_checkbox::Checkbox::new(is_checked)
        .size(BOX_SIZE)
        .on_toggle(on_toggle)
        .style(move |_theme, status| style(&t, status, is_focused))
        .into();

    let target = container(box_widget).center(Length::Fixed(TARGET_SIZE));

    tooltip(
        mouse_area(target).on_press(toggle_message),
        super::overlay::raised_card(tokens, text(accessible_label.into()).size(FONT_SMALL)),
        tooltip::Position::Bottom,
    )
    .into()
}

/// `surface`/`accent` background, `border` at rest, `text_primary` icon
/// colour when unchecked and `accent_text` when checked (the same
/// on-accent pairing `chip::filter`'s selected state and `select`'s
/// selected menu row both use) — then the focus ring on top when
/// `is_focused`, reusing [`ring_color_for`] rather than a second
/// implementation (see `select.rs`'s module doc for why `with_focus_ring`
/// itself cannot be called directly here).
fn style(tokens: &Tokens, status: iced_checkbox::Status, is_focused: bool) -> iced_checkbox::Style {
    use iced_checkbox::Status;

    let (is_checked, is_disabled) = match status {
        Status::Active { is_checked } | Status::Hovered { is_checked } => (is_checked, false),
        Status::Disabled { is_checked } => (is_checked, true),
    };

    let (background, icon_color) = if is_checked {
        (
            to_iced(tokens.palette.accent),
            to_iced(tokens.palette.accent_text),
        )
    } else {
        (
            to_iced(tokens.palette.surface),
            to_iced(tokens.palette.text_primary),
        )
    };
    let background = if is_disabled {
        iced::Color {
            a: 0.5,
            ..background
        }
    } else {
        background
    };

    let base = iced_checkbox::Style {
        background: iced::Background::Color(background),
        icon_color,
        border: iced::Border {
            color: to_iced(tokens.palette.border),
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
        text_color: None,
    };

    if !is_focused || is_disabled {
        return base;
    }

    iced_checkbox::Style {
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
    use iced_checkbox::Status;

    /// RFC-035 Handoff 019 §8: focused and unfocused styles must differ,
    /// for both the checked and unchecked backgrounds (`accent` and
    /// `surface` are different colours, so the ring-vs-background contrast
    /// decision runs independently for each).
    #[test]
    fn focused_and_unfocused_styles_differ() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            for is_checked in [false, true] {
                let unfocused = style(&tokens, Status::Active { is_checked }, false);
                let focused = style(&tokens, Status::Active { is_checked }, true);
                assert_ne!(unfocused.border, focused.border, "is_checked={is_checked}");
            }
        }
    }

    /// A disabled checkbox never draws a ring, matching
    /// `buttons.rs`'s `filled_background`'s exclusion of `Disabled`
    /// (RFC-036 Stage 6 closure — see `theme.rs`'s test of the same name
    /// for why).
    #[test]
    fn disabled_style_is_unaffected_by_is_focused() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            let a = style(&tokens, Status::Disabled { is_checked: true }, false);
            let b = style(&tokens, Status::Disabled { is_checked: true }, true);
            assert_eq!(a.border, b.border);
        }
    }
}
