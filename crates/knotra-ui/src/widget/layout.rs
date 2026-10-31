//! Spacing, typography, and size tokens, plus re-exported `iced` primitives.

// Re-export iced primitives so callers need only one import.
pub use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Column, Row, button, column, container, row, scrollable, text, text_input},
};

/// RFC-071 D2: the one place knotra ever names a text weight. Chains onto a
/// `text(...)` the caller already built with its own `.size(...)`/
/// `.line_height(...)` — the same composition `centered_button_label`
/// applies for a button's own label — setting
/// `Font { weight: Semibold, ..Font::DEFAULT }`. No view names a `Weight`
/// itself; `text_weight_guard.rs` is the guard that enforces it (the shape
/// of `text_size_guard.rs`, scanning for `Weight::` outside this function).
///
/// Weight only ever reinforces another signal (D2: every strong text also
/// differs in role or position from its neighbours), so a system font with
/// no semibold face loses emphasis, never meaning — iced falls back to the
/// nearest available face on its own.
#[must_use]
pub fn strong<'a>(
    label: iced::widget::Text<'a, iced::Theme, iced::Renderer>,
) -> iced::widget::Text<'a, iced::Theme, iced::Renderer> {
    label.font(iced::Font {
        weight: iced::font::Weight::Semibold,
        ..iced::Font::DEFAULT
    })
}

/// Standard corner radius for cards and panels.
pub const CARD_RADIUS: f32 = 8.0;

/// Standard gap between cards in the dashboard grid.
pub const CARD_GAP: f32 = 12.0;

/// Standard padding inside a card.
pub const CARD_PADDING: Padding = Padding {
    top: 14.0,
    right: 16.0,
    bottom: 14.0,
    left: 16.0,
};

/// Sidebar width in pixels.
pub const SIDEBAR_WIDTH: f32 = 180.0;

/// Minimum card width in the dashboard grid.
pub const CARD_MIN_WIDTH: f32 = 240.0;

// --- Accessibility tokens (UX review) ---------------------------------------
// Non-technical users benefit from larger hit targets and more readable body
// text. Primary and secondary actions use BUTTON_HEIGHT; small/inline controls
// inside dense read-only rows may use SMALL_BUTTON_HEIGHT.

/// Minimum height for any primary or secondary action control.
pub const BUTTON_HEIGHT: f32 = 44.0;

/// Reduced height permitted only for inline controls in dense, read-only rows.
pub const SMALL_BUTTON_HEIGHT: f32 = 36.0;
