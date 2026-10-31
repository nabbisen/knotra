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
mod list;
pub mod notice;
pub mod overlay;
mod record;
pub(crate) mod ring;
pub mod select;
pub mod status;

pub use buttons::{
    centered_button_label, current_or, danger, danger_maybe, ghost, ghost_maybe, header_control,
    icon_button_maybe, primary, primary_maybe, reasoned, secondary, secondary_maybe, style,
};
pub use checkbox::checkbox;
pub use field::{guided_field, guided_field_focused, text_input_style, validated_field};
pub use focus::{clear_input_focus, focus_id, focus_input};
pub use layout::{
    Alignment, BUTTON_HEIGHT, Color, Column, Element, Length, Padding, Row, SMALL_BUTTON_HEIGHT,
    STRONG_FONT, WIDE_CONTENT_WIDTH, button, column, container, row, scrollable, strong, text,
    text_input,
};
pub use list::list;
pub use notice::{NoticeAction, NoticeTone, notice};
pub use record::record_row;
/// Focus-ring tokens (RFC-033 D7). Re-exported for the same reason as
/// `Tokens` above — RFC-036 R11 is the first consumer.
pub use snora::design::FocusTokens;
/// Snora Design token bundle. Re-exported so view-layer signatures can name
/// `Tokens` without importing `snora::design` directly (RFC-034 R2).
pub use snora::design::Tokens;
pub use status::{Tone, status_badge, symbol as status_symbol};
