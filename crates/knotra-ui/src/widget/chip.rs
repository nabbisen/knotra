//! Filter chip primitive (RFC-035 Handoff 019 §7.1).
//!
//! `snora::design::chip::filter` already exists and is already token-styled
//! and contrast-verified (>=6.7:1 selected, per its own module doc). This
//! wrapper's only job is R19 compliance — giving `knotra-app` something to
//! call that is not `snora::design` directly — not adding behaviour.
//!
//! `snora::design::chip`'s own module doc documents that iced 0.14's
//! `button::Status` has no `Focused` variant, so — unlike [`super::select`]
//! and [`super::checkbox`] — this wrapper carries no `is_focused` parameter;
//! the signature matches the RFC's own `filter(tokens, label, selected,
//! on_toggle) -> Element` exactly.

use snora::design::Tokens;

use super::layout::Element;

/// A toggle chip for filtering or categorizing. Solid `accent` background +
/// `accent_text` foreground when `selected`; neutral `surface` + `text_secondary` at rest.
#[must_use]
pub fn filter<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    selected: bool,
    on_toggle: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    snora::design::chip::filter(tokens, label, selected, on_toggle)
}
