//! Common widget helpers built on top of `iced`.
//!
//! These are thin wrappers that enforce consistent spacing, typography, and
//! accessible defaults across all screens without embedding business logic.

// Re-export iced primitives so callers need only one import.
pub use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Column, Row},
    Alignment, Color, Element, Length, Padding,
};

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

/// Body text size for non-technical-facing content.
pub const FONT_BODY: f32 = 15.0;

/// Small text size for metadata, timestamps, and captions.
pub const FONT_SMALL: f32 = 13.0;
