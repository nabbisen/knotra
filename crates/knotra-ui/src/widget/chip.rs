//! Filter chip primitive, built from `KnotraTheme` (RFC-035 Handoff 019
//! §7.1, rebuilt per Handoff 020 §7.1). `snora::design::chip::filter`
//! returns an `Element`, not a `Style`, and has no seam to compose a focus
//! ring onto a pass-through (094 Finding 1), so this keeps its own button
//! built from `KnotraTheme`'s own layout (badge, label, count, focus ring,
//! sizing), like `select` and `checkbox` — only the *colours* now come
//! from snora.
//!
//! **Task 112:** the two colour functions this module called were, until
//! now, a hand-kept mirror of `snora::design::chip`'s own private style
//! functions (`chip_style_selected`/`chip_style_unselected`) — so 0.62's
//! pressed-state contrast fix for `light_aaa`/`dark_aaa` (RFC-129), made to
//! snora's own copy, never reached this one. 0.62 made the real functions
//! public for exactly this case (`snora::design::style::chip::{selected,
//! unselected}`), so this module now calls them directly and keeps no
//! copy of its own.
//!
//! A chip's style type is `button::Style`, so — unlike `select`/`checkbox`
//! — this reuses `buttons.rs`'s `with_focus_ring` directly rather than
//! `ring_color_for`.

use iced::widget::{Space, button, row, text};
use iced::{Color, Length};
use snora::design::Tokens;

use super::buttons::centered_button_row;
use super::buttons::style::with_focus_ring;
use super::layout::{BUTTON_HEIGHT, Element};
use super::status::{Tone, status_badge};

fn to_iced(color: snora::design::Color) -> Color {
    snora::design::style::color::to_iced_color(color)
}

/// A toggle chip for filtering or categorizing, on `snora::design::control`
/// (RFC-075 stage 3). Solid `accent` background + `accent_text` foreground
/// when `selected`; neutral `surface`/`border`/`text_secondary` at rest —
/// the same appearance as `snora::design::chip::filter`, plus a focus ring
/// `snora`'s own pass-through could not carry (094 Finding 1 — see the
/// module doc).
///
/// RFC-071 D5: every chip shows its status's badge (the same `status_badge`
/// a dashboard row's own status line uses, never a second vocabulary for
/// the same meaning — D3) and its count. A chip at `count == 0` is never
/// hidden — the row of chips must not jump as projects change state — but
/// is drawn in `text_secondary` regardless of `selected`, so an empty
/// filter reads as present-but-inactive rather than as a live,
/// equally-weighted choice. Reuses [`style`] unchanged: `Control::style`
/// takes the identical closure shape `button::style` did, so Task 112's
/// contrast suite already covers this chip's colours too, by construction
/// — see that test's own extended doc comment.
#[must_use]
pub fn filter_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    badge: (Tone, &snora::Icon),
    count: usize,
    selected: bool,
    on_toggle: impl Into<Option<Message>>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let (tone, glyph) = badge;
    let t = tokens.clone();
    let label_size = snora::design::style::text::body_size(tokens);
    let label_line_height = snora::design::style::text::body_line_height(tokens);
    let diameter = tokens.typography.body.size * tokens.typography.body.line_height;
    let on_toggle = on_toggle.into();

    snora::design::control(
        tokens,
        centered_button_row(
            row![
                status_badge(tokens, tone, glyph, diameter),
                text(label.into())
                    .size(label_size)
                    .line_height(label_line_height)
                    .wrapping(iced::advanced::text::Wrapping::None),
                Space::new().width(Length::Fixed(tokens.spacing.xs)),
                text(count.to_string())
                    .size(label_size)
                    .line_height(label_line_height),
            ]
            .spacing(tokens.spacing.xs),
        ),
    )
    .height(BUTTON_HEIGHT)
    .id(id)
    .padding([0.0, tokens.spacing.sm])
    .on_press_maybe(on_toggle)
    .style(move |_theme, status| style(&t, selected, count, status, false))
    .into()
}

/// `pub(crate)` so `theme.rs`'s contrast test can exercise the real
/// function rather than re-deriving its colours (RFC-035 Handoff 020 §7.3).
pub(crate) fn style(
    tokens: &Tokens,
    selected: bool,
    count: usize,
    status: button::Status,
    is_focused: bool,
) -> button::Style {
    let base = if selected {
        snora::design::style::chip::selected(tokens, status)
    } else {
        snora::design::style::chip::unselected(tokens, status)
    };
    // RFC-071 D5: a chip at count 0 reads as present-but-inactive, in
    // `text_secondary`, regardless of `selected` — never hidden, never
    // styled as if it were a live equal choice.
    let base = if count == 0 {
        button::Style {
            text_color: dim_if_disabled(status, to_iced(tokens.palette.text_secondary)),
            ..base
        }
    } else {
        base
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

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC-075 handoff stage 1 §3, row 3 — see `buttons.rs`'s own test of
    /// the same name for the full reasoning; this is `filter_control`'s
    /// copy of it.
    #[test]
    fn filter_control_is_a_tab_stop_enter_and_space_press_it_disabled_is_skipped() {
        use crate::test_support::Harness;
        use iced::widget::{Id, column};

        #[derive(Debug, Clone, PartialEq)]
        enum Msg {
            Toggled,
        }

        let tokens = Tokens::light();
        let icon: snora::Icon = snora::lucide::Check.into();
        let view = |enabled: bool| -> iced::Element<'static, Msg> {
            column![
                filter_control(
                    &tokens,
                    "All set",
                    (Tone::Success, &icon),
                    2,
                    false,
                    enabled.then_some(Msg::Toggled),
                    Id::new("chip"),
                ),
                filter_control(
                    &tokens,
                    "Next",
                    (Tone::Success, &icon),
                    1,
                    false,
                    Some(Msg::Toggled),
                    Id::new("next"),
                ),
            ]
            .spacing(8)
            .into()
        };
        let size = iced_test::core::Size {
            width: 400.0,
            height: 400.0,
        };

        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(Id::new("chip")));
        h.enter();
        assert_eq!(h.messages(), [Msg::Toggled]);

        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        h.space();
        assert_eq!(h.messages(), [Msg::Toggled]);

        let mut h = Harness::new(view(false), size, None);
        h.tab(false);
        assert_eq!(
            h.focused_id(),
            Some(Id::new("next")),
            "a disabled chip must not be a Tab stop"
        );
    }

    /// Same shape as `select`'s and `checkbox`'s: focused and unfocused
    /// styles must differ, for both the selected and unselected background
    /// (RFC-035 Handoff 020 §8).
    #[test]
    fn focused_and_unfocused_styles_differ() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            for selected in [false, true] {
                let unfocused = style(&tokens, selected, 1, button::Status::Active, false);
                let focused = style(&tokens, selected, 1, button::Status::Active, true);
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
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            for selected in [false, true] {
                let enabled = style(&tokens, selected, 1, button::Status::Active, false);
                let disabled = style(&tokens, selected, 1, button::Status::Disabled, false);
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

    /// RFC-071 D5: a chip at count 0 is drawn in `text_secondary`,
    /// regardless of `selected` — including when `selected` would otherwise
    /// draw `accent_text` on a filled background.
    #[test]
    fn zero_count_chip_text_is_secondary_regardless_of_selected() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let expected = to_iced(tokens.palette.text_secondary);
            for selected in [false, true] {
                let zero = style(&tokens, selected, 0, button::Status::Active, false);
                assert_eq!(
                    zero.text_color, expected,
                    "selected={selected}: count 0 must use text_secondary"
                );
            }
            // A selected chip's own non-zero text colour (`accent_text`) is
            // the one case `text_secondary` must visibly differ from —
            // unselected already uses `text_secondary` at any count, so
            // only `selected=true` is a meaningful contrast to check here.
            let zero_selected = style(&tokens, true, 0, button::Status::Active, false);
            let non_zero_selected = style(&tokens, true, 1, button::Status::Active, false);
            assert_ne!(
                zero_selected.text_color, non_zero_selected.text_color,
                "a selected chip at count 0 must look different from one with a real count"
            );
        }
    }
}
