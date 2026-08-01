//! The shared focus-ring colour decision (RFC-036 Stage 6, generalized by
//! RFC-035 Handoff 020 §7.2).
//!
//! Moved out of `buttons.rs`, where it originated as part of
//! `style::with_focus_ring`, once it had three callers outside the button
//! path (`select`, `checkbox`, and now `chip`) with more expected
//! (RFC-037, RFC-038). `buttons::style::ring_color_for` read as a
//! button-specific function other modules borrowed, which was backwards —
//! this is the one place every control's ring-colour choice is made.
//!
//! `with_focus_ring` itself stays in `buttons.rs`, and so do its three
//! tests: they exercise the `button::Style`-specific wrapper, not this
//! function directly, so they are not orphaned by the move.

use iced::{Background, Color};
use snora::design::Tokens;

fn from_iced_color(c: Color) -> snora::design::Color {
    snora::design::Color::rgba(c.r, c.g, c.b, c.a)
}

/// Returns the style's background color only when it is fully opaque —
/// a genuine filled control, not a translucent or partially-transparent
/// one. See `buttons.rs`'s `with_focus_ring` doc (pre-move) for the full
/// reasoning: `contrast_ratio` ignores alpha entirely, so anything short of
/// exact opacity would let a translucent color through uncomposited.
fn filled_background(background: Option<Background>) -> Option<Color> {
    match background {
        Some(Background::Color(c)) if c.a >= 1.0 => Some(c),
        _ => None,
    }
}

/// Picks between `ring_color` and `accent_text` by measured contrast against
/// an opaque background — the RFC-036 Stage 6 decision, generalized so every
/// control that draws a focus ring makes the same choice instead of a
/// second implementation.
pub(crate) fn ring_color_for(tokens: &Tokens, background: Option<Background>) -> Color {
    let ring = tokens.focus;
    let default_ring_color = snora::design::style::color::to_iced_color(ring.ring_color);

    match filled_background(background) {
        Some(bg) => {
            let bg = from_iced_color(bg);
            let default_contrast = snora::design::contrast::contrast_ratio(ring.ring_color, bg);
            let alt_contrast =
                snora::design::contrast::contrast_ratio(tokens.palette.accent_text, bg);
            if alt_contrast > default_contrast {
                snora::design::style::color::to_iced_color(tokens.palette.accent_text)
            } else {
                default_ring_color
            }
        }
        None => default_ring_color,
    }
}
