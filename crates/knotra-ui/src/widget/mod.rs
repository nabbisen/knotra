//! Common widget helpers built on top of `iced` and `snora::design`.
//!
//! These are thin wrappers that enforce consistent spacing, typography, and
//! accessible defaults across all screens without embedding business logic.
//! Split from a single `widget.rs` file in RFC-034: this `mod.rs` re-exports
//! every path the rest of the workspace already imports, so the split itself
//! changes no call site.

mod buttons;
pub(crate) mod checkbox;
pub mod chip;
mod field;
mod focus;
pub mod icon;
mod layout;
pub mod notice;
pub mod overlay;
pub(crate) mod ring;
pub mod select;

pub use buttons::{
    current_or, danger, danger_maybe, ghost, ghost_maybe, guided_button, icon_button_maybe,
    primary, primary_maybe, secondary, secondary_maybe, style,
};
pub use checkbox::checkbox;
pub use field::{guided_field, guided_field_focused};
pub use focus::{clear_input_focus, focus_id, focus_input};
pub use layout::{
    Alignment, BUTTON_HEIGHT, CARD_GAP, CARD_MIN_WIDTH, CARD_PADDING, CARD_RADIUS, Color, Column,
    Element, FONT_BODY, FONT_SMALL, Length, Padding, Row, SIDEBAR_WIDTH, SMALL_BUTTON_HEIGHT,
    button, column, container, row, scrollable, text, text_input,
};
pub use notice::{NoticeAction, notice};
/// Focus-ring tokens (RFC-033 D7). Re-exported for the same reason as
/// `Tokens` above — RFC-036 R11 is the first consumer.
pub use snora::design::FocusTokens;
/// Snora Design token bundle. Re-exported so view-layer signatures can name
/// `Tokens` without importing `snora::design` directly (RFC-034 R2).
pub use snora::design::Tokens;
/// Notice tone (RFC-032). Re-exported so call sites can name `Tone` without
/// importing `snora::design` directly (RFC-035 R19) — `notice`'s own Stage 5
/// consumer.
pub use snora::design::Tone;
