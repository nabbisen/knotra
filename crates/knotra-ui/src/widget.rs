//! Common widget helpers built on top of `iced`.
//!
//! These are thin wrappers that enforce consistent spacing, typography, and
//! accessible defaults across all screens without embedding business logic.

// Re-export iced primitives so callers need only one import.
pub use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Column, Row, button, column, container, row, scrollable, text, text_input},
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
