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
mod escape_first;
mod field;
mod focus;
pub mod icon;
pub mod keyboard;
pub mod keyed;
mod layout;
mod link;
mod list;
pub mod notice;
pub mod overlay;
mod record;
pub(crate) mod ring;
pub mod select;
pub mod status;

pub use buttons::{
    centered_button_label, centered_button_row, current_or, current_or_control, danger,
    danger_control, danger_control_maybe, danger_maybe, ghost, ghost_control, ghost_control_maybe,
    ghost_maybe, header_control, icon_button_maybe, primary, primary_control,
    primary_control_maybe, primary_maybe, reasoned, reasoned_control_maybe, secondary,
    secondary_control, secondary_control_maybe, secondary_maybe, style,
};
pub use checkbox::{checkbox, checkbox_control, checkbox_control_unlabeled};
pub use escape_first::escape_first;
pub use field::{
    guided_field, guided_field_entry, guided_field_focused, styled_text_input, text_input_style,
    validated_field,
};
pub use focus::{clear_input_focus, focus_id, focus_input};
pub use keyed::focus_by_id;
pub use layout::{
    APP_DIRECTION, Alignment, BUTTON_HEIGHT, Color, Column, Element, Length, Padding, Row,
    SMALL_BUTTON_HEIGHT, STRONG_FONT, WIDE_CONTENT_WIDTH, button, column, container, row,
    scrollable, strong, text, text_input,
};
pub use link::name_link;
pub use list::list;
pub use notice::{NoticeAction, NoticeTone, notice};
pub use record::record_row;
/// Focus-ring tokens (RFC-033 D7). Re-exported for the same reason as
/// `Tokens` above — RFC-036 R11 is the first consumer.
pub use snora::design::FocusTokens;
/// Snora Design token bundle. Re-exported so view-layer signatures can name
/// `Tokens` without importing `snora::design` directly (RFC-034 R2).
pub use snora::design::Tokens;
pub use status::{Tone, body_badge_diameter, status_badge, symbol as status_symbol};
