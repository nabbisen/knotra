//! Select (dropdown) primitive, built from `KnotraTheme` (RFC-035 Handoff
//! 019 §7.2). `snora` ships no select styling — `snora-widgets-0.25.2`'s
//! `design/` module has `button`, `card`, `chip`, `notice`, `progress`, no
//! `select` — so this is written directly from the RFC-033 D7 roles:
//! surface, border, text, and accent for the open/selected row.
//!
//! The focus ring reuses [`super::ring::ring_color_for`] — the same
//! RFC-036 Stage 6 decision `with_focus_ring` (`buttons.rs`) applies to
//! buttons — rather than a second implementation. `with_focus_ring` itself
//! cannot be called directly: it is typed to `iced::widget::button::Style`,
//! and `pick_list_widget::Style` / `menu::Style` are different shapes (no
//! `shadow` or `snap` field, an extra `placeholder_color`/`handle_color` on
//! the field, and a `selected_background`/`selected_text_color` pair on the
//! menu). `ring_color_for` only needs a background and returns a colour, so
//! it composes with any of them.
//!
//! `pick_list`'s `options` take owned `(T, String)` pairs, not `&'a [T]`
//! (RFC-035 Handoff 022 commit 2a, `100`'s ruling on the blocker reported
//! against Stage 2 §7.3). The original `&'a [T]` signature narrowed iced's
//! own `PickList::new`, which is generic over `L: Borrow<[T]> + 'a`,
//! including an owned `Vec`. That narrowing made the wrapper unable to
//! express its own normal call pattern: a per-render option list whose
//! labels come from the caller's current locale via `state.t(...)`, which
//! has no `'static` source to borrow from. Widening to `impl Borrow<[T]> +
//! 'a` alone would have unblocked that, but every call site would still
//! need `T: ToString`, hand-rolling a `Display` wrapper just to pair a
//! domain value with its label, for what RFC-033 D4 makes the universal
//! case: every select in this app pairs a value with a locale-derived
//! label, not an edge case. So the pairing lives here, once, instead.

use iced::widget::overlay::menu;
use iced::widget::pick_list as pick_list_widget;
use snora::design::Tokens;

use super::layout::Element;
use super::ring::ring_color_for;

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// A value paired with its already-localized label — the shape `pick_list`
/// builds internally from the `(T, String)` pairs callers pass in, so `T`
/// itself never needs a `Display` impl.
#[derive(Clone, PartialEq)]
struct LabeledOption<T> {
    value: T,
    label: String,
}

impl<T> std::fmt::Display for LabeledOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// A themed select menu, sized the same way [`super::field::styled_text_
/// input`] sizes a field: the closed control is exactly `BUTTON_HEIGHT`
/// tall, its text vertically centred, through the same `(BUTTON_HEIGHT -
/// line_height_px) / 2.0` symmetric vertical padding, `body` size and
/// line height.
///
/// Amendment A5 §3: this is the one `pick_list` constructor
/// `knotra-ui`/`knotra-app` call — `styled_text_input`'s own sibling in
/// name as well as shape, not a second, differently-named wrapper beside
/// it — so every select in the app (nine call sites today: both Arrange-
/// menu selects in `toolbar.rs`, and the language/theme/zoom/refresh-
/// interval/simultaneous-reads/editor-default-pick/log-size/fs-debounce
/// selects in `settings.rs`) is sized this way with no separate call.
///
/// **The open list's own row height is not fully this function's to set**:
/// `iced_widget::pick_list::PickList::overlay()` forwards this builder's
/// `padding` and `text_size` into the `Menu` overlay it constructs, but
/// not `text_line_height` — iced has no public seam for that, confirmed by
/// reading `pick_list.rs`'s own `overlay()`. The open row's line height is
/// therefore iced's own default, `LineHeight::Relative(1.3)`, not the
/// `body` token's `1.4` the closed field uses — at `body`'s `16px` text
/// size that is `16 × 1.3 = 20.8px`, plus the forwarded padding
/// (`(44 − 16 × 1.4) / 2 ≈ 10.8px` top and bottom) = **42.4px**, still
/// comfortably at least `SMALL_BUTTON_HEIGHT` (36) without any further
/// setting.
///
/// `options` pairs each value with its already-localized label, built fresh
/// per render — see the module doc for why this is owned data rather than a
/// borrowed slice.
#[must_use]
pub fn styled_pick_list<'a, T, Message>(
    tokens: &Tokens,
    options: Vec<(T, String)>,
    selected: Option<T>,
    is_focused: bool,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: PartialEq + Clone + 'a,
    Message: Clone + 'a,
{
    let labeled_options: Vec<LabeledOption<T>> = options
        .into_iter()
        .map(|(value, label)| LabeledOption { value, label })
        .collect();
    let selected = selected.and_then(|value| {
        labeled_options
            .iter()
            .find(|option| option.value == value)
            .cloned()
    });

    let t_field = tokens.clone();
    let t_menu = tokens.clone();

    let line_height_px = tokens.typography.body.size * tokens.typography.body.line_height;

    pick_list_widget::PickList::new(
        labeled_options,
        selected,
        move |option: LabeledOption<T>| on_select(option.value),
    )
    .padding([(super::layout::BUTTON_HEIGHT - line_height_px) / 2.0, 12.0])
    .text_size(snora::design::style::text::body_size(tokens))
    .text_line_height(snora::design::style::text::body_line_height(tokens))
    .style(move |_theme, status| field_style(&t_field, status, is_focused))
    .menu_style(move |_theme| menu_style(&t_menu, is_focused))
    .into()
}

/// The closed (and opening) control's style: `surface` background,
/// `text_primary` text, `border` at rest, `accent` border when hovered or
/// open — then the focus ring on top when `is_focused`.
///
/// `pub(crate)` so `theme.rs`'s contrast test can drive the real function
/// across every `Status` rather than reconstructing its background from a
/// flat palette role (RFC-035 Handoff 022 §7.5, same reason `chip::style`
/// was made `pub(crate)` in Handoff 020).
pub(crate) fn field_style(
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
pub(crate) fn menu_style(tokens: &Tokens, is_focused: bool) -> menu::Style {
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
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let unfocused = field_style(&tokens, pick_list_widget::Status::Active, false);
            let focused = field_style(&tokens, pick_list_widget::Status::Active, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }

    #[test]
    fn menu_focused_and_unfocused_styles_differ() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let unfocused = menu_style(&tokens, false);
            let focused = menu_style(&tokens, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }
}
