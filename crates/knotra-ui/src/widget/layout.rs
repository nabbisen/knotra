//! Spacing, typography, and size tokens, plus re-exported `iced` primitives.

// Re-export iced primitives so callers need only one import.
pub use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Column, Row, button, column, container, row, scrollable, text, text_input},
};

/// RFC-071 D2 / Amendment A2: the one place knotra ever names a text
/// weight. No view names a `Weight` itself; `text_weight_guard.rs` is the
/// guard that enforces it (the shape of `text_size_guard.rs`, scanning for
/// `Weight::` outside this file).
///
/// **Bold, not Semibold** (Review 248 §3): asking for weight 600 does not
/// just miss a face, it *changes the family*. On this project's own Linux
/// dev machine, Noto Sans CJK JP ships Regular/Medium/Bold but no SemiBold,
/// while Noto Serif CJK JP does — so a `Weight::Semibold` request on
/// Japanese text resolved to a serif (Mincho) face, silently, while every
/// regular line around it stayed sans (Gothic). Bold (700) exists in the
/// sans family on all three platforms this app ships for: Noto Sans / Noto
/// Sans CJK JP Bold on Linux, Segoe UI / Yu Gothic UI Bold on Windows, the
/// system font / Hiragino Sans W6-W7 on macOS. Weight only ever reinforces
/// another signal (D2: every strong text also differs in role or position
/// from its neighbours) — a system font with no bold face loses only
/// emphasis, never meaning, and iced falls back to the nearest available
/// face on its own; the Semibold case is why "nearest available face" is
/// not by itself a safe assumption, and must be checked by eye, by
/// typeface, not inferred from the request.
pub const STRONG_FONT: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::DEFAULT
};

/// Chains onto a `text(...)` the caller already built with its own
/// `.size(...)`/`.line_height(...)` — the same composition
/// `centered_button_label` applies for a button's own label — setting
/// [`STRONG_FONT`]. A `rich_text` span that needs the same weight (RFC-071
/// D4's project name, which also needs a colour that changes with hover/
/// focus — a `Text` widget's `.color()` is static, a `Span`'s is not) calls
/// `.font(STRONG_FONT)` directly rather than through this function, since
/// `strong` is typed to `Text`, not `Span` — the constant is what the guard
/// actually pins, not this wrapper.
#[must_use]
pub fn strong<'a>(
    label: iced::widget::Text<'a, iced::Theme, iced::Renderer>,
) -> iced::widget::Text<'a, iced::Theme, iced::Renderer> {
    label.font(STRONG_FONT)
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
