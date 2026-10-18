//! Filter chip primitive, built from `KnotraTheme` (RFC-035 Handoff 019
//! §7.1, rebuilt per Handoff 020 §7.1). `snora::design::chip::filter`
//! returns an `Element`, not a `Style`, and its two style functions
//! (`chip_style_selected`/`chip_style_unselected`) are private — there is
//! no seam to compose a focus ring onto a pass-through (094 Finding 1), so
//! this is built from `KnotraTheme` directly, like `select` and
//! `checkbox`, matching snora's visible appearance as closely as the token
//! set allows.
//!
//! A chip's style type is `button::Style`, so — unlike `select`/`checkbox`
//! — this reuses `buttons.rs`'s `with_focus_ring` directly rather than
//! `ring_color_for`.

use iced::widget::button;
use iced::{Border, Color, Shadow};
use snora::design::Tokens;

use super::buttons::style::with_focus_ring;
use super::layout::Element;

fn to_iced(color: snora::design::Color) -> Color {
    snora::design::style::color::to_iced_color(color)
}

/// Blends toward black by `amount` — mirrors `snora::design::chip`'s own
/// private `darken` helper (used for hover/press states), which is not
/// exported, so this is the same 4-line reimplementation `snora-widgets`
/// itself keeps in two places (`design/chip.rs` and `design/style/button.rs`).
fn darken(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}

/// A toggle chip for filtering or categorizing. Solid `accent` background +
/// `accent_text` foreground when `selected`; neutral `surface`/`border`/
/// `text_secondary` at rest — the same appearance as
/// `snora::design::chip::filter`, plus a focus ring `snora`'s own pass-through
/// could not carry.
#[must_use]
pub fn filter<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    selected: bool,
    is_focused: bool,
    on_toggle: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(iced::widget::text(label.into()).size(snora::design::style::text::label_size(tokens)))
        .on_press_maybe(on_toggle.into())
        .padding([tokens.spacing.xs, tokens.spacing.sm])
        .style(move |_theme, status| style(&t, selected, status, is_focused))
        .into()
}

/// `pub(crate)` so `theme.rs`'s contrast test can exercise the real
/// function rather than re-deriving its colours (RFC-035 Handoff 020 §7.3).
pub(crate) fn style(
    tokens: &Tokens,
    selected: bool,
    status: button::Status,
    is_focused: bool,
) -> button::Style {
    let base = if selected {
        selected_style(tokens, status)
    } else {
        unselected_style(tokens, status)
    };
    with_focus_ring(tokens, is_focused, base)
}

/// Dims `color`'s alpha when `status` is `Disabled`, leaves every other
/// status untouched — RFC-062 D4: the fill already signalled `Disabled`
/// this way (`Color { a: 0.5, ..base }`); text and border did not, so a
/// disabled chip read as a fully legible, pressable one. snora's own 0.52
/// fix dims text and border the same way, by the same factor; snora's
/// palette has no dedicated disabled colour to read instead.
///
/// Disabled controls are exempt from WCAG's contrast minimums (SC 1.4.3,
/// SC 1.4.11 both exclude inactive components), so dropping below 4.5:1
/// here is correct, not an oversight — no contrast-floor assertion is
/// added for this state.
fn dim_if_disabled(status: button::Status, color: Color) -> Color {
    match status {
        button::Status::Disabled => Color { a: 0.5, ..color },
        _ => color,
    }
}

/// Mirrors `snora::design::chip`'s private `chip_style_selected`: solid
/// `accent` background, `accent_text` foreground, `accent`-colored pill
/// border.
fn selected_style(tokens: &Tokens, status: button::Status) -> button::Style {
    let accent = to_iced(tokens.palette.accent);
    let accent_text = to_iced(tokens.palette.accent_text);
    let bg = match status {
        button::Status::Active => accent,
        button::Status::Hovered => darken(accent, 0.06),
        button::Status::Pressed => darken(accent, 0.12),
        button::Status::Disabled => Color { a: 0.5, ..accent },
    };
    button::Style {
        background: Some(bg.into()),
        text_color: dim_if_disabled(status, accent_text),
        border: Border::default()
            .rounded(tokens.radius.pill)
            .color(dim_if_disabled(status, accent))
            .width(1.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Mirrors `snora::design::chip`'s private `chip_style_unselected`: neutral
/// `surface` background, `text_secondary` foreground, `border`-colored pill
/// border.
fn unselected_style(tokens: &Tokens, status: button::Status) -> button::Style {
    let border_color = to_iced(tokens.palette.border);
    let text_color = to_iced(tokens.palette.text_secondary);
    let surface = to_iced(tokens.palette.surface);
    let bg = match status {
        button::Status::Active => surface,
        button::Status::Hovered => darken(surface, 0.04),
        button::Status::Pressed => darken(surface, 0.08),
        button::Status::Disabled => Color { a: 0.5, ..surface },
    };
    button::Style {
        background: Some(bg.into()),
        text_color: dim_if_disabled(status, text_color),
        border: Border::default()
            .rounded(tokens.radius.pill)
            .color(dim_if_disabled(status, border_color))
            .width(1.0),
        shadow: Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same shape as `select`'s and `checkbox`'s: focused and unfocused
    /// styles must differ, for both the selected and unselected background
    /// (RFC-035 Handoff 020 §8).
    #[test]
    fn focused_and_unfocused_styles_differ() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            for selected in [false, true] {
                let unfocused = style(&tokens, selected, button::Status::Active, false);
                let focused = style(&tokens, selected, button::Status::Active, true);
                assert_ne!(unfocused.border, focused.border, "selected={selected}");
            }
        }
    }

    /// RFC-062 D4: a disabled chip must not read as a fully legible,
    /// pressable one. Before this fix, only the fill signalled `Disabled`
    /// — text and border were byte-identical to the active chip's. Checked
    /// in both themes and for both `selected` states, since the two style
    /// functions each read their own tokens.
    #[test]
    fn disabled_chip_text_and_border_differ_from_enabled() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            for selected in [false, true] {
                let enabled = style(&tokens, selected, button::Status::Active, false);
                let disabled = style(&tokens, selected, button::Status::Disabled, false);
                assert_ne!(
                    enabled.text_color, disabled.text_color,
                    "selected={selected}: disabled text must not match enabled text"
                );
                assert_ne!(
                    enabled.border.color, disabled.border.color,
                    "selected={selected}: disabled border must not match enabled border"
                );
            }
        }
    }
}
