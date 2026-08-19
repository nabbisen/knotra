//! Safe field helpers (Phase 2 — non-technical UX), plus `validated_field`
//! (RFC-038 D1) — the field half of RFC-034 R7's promise. R7 said new
//! controls would be "added alongside `guided_button` and `guided_field`";
//! it happened for buttons (`buttons.rs`'s semantic vocabulary) and never
//! for fields, which is why RFC-037 D6 could not delete `guided_field` —
//! there was nothing to migrate its eight call sites onto. This module
//! still does not delete or change `guided_field`/`guided_field_focused`;
//! `validated_field` is added beside them, scoped to what RFC-038's own
//! consumer (Settings) needs, not to every field knotra might ever want.

use snora::design::Tokens;

use super::layout::{Element, Length};

/// A labelled text input with persistent label above and optional inline error.
///
/// The label always stays visible (not a placeholder that disappears on
/// focus). An error message, when present, appears beneath the field in a
/// small, clearly distinct style.
pub fn guided_field<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
) -> iced::Element<'a, Message> {
    guided_field_with_id(tokens, label, placeholder, value, on_change, error, None)
}

/// Same as [`guided_field`] but assigns a [`iced::widget::Id`] to the input for
/// programmatic focus (e.g. auto-focus when a dialog opens).
pub fn guided_field_focused<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    id: iced::widget::Id,
) -> iced::Element<'a, Message> {
    guided_field_with_id(
        tokens,
        label,
        placeholder,
        value,
        on_change,
        error,
        Some(id),
    )
}

fn guided_field_with_id<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    id: Option<iced::widget::Id>,
) -> iced::Element<'a, Message> {
    use iced::widget::{column, text, text_input};

    let mut field = text_input(placeholder, value)
        .on_input(on_change)
        .padding([0, 12])
        .width(Length::Fill)
        .size(snora::design::style::text::body_size(tokens));

    if let Some(widget_id) = id {
        field = field.id(widget_id);
    }

    let mut group = column![
        text(label).size(snora::design::style::text::body_size(tokens)),
        field,
    ]
    .spacing(8);

    if let Some(err) = error {
        group = group.push(text(err).size(snora::design::style::text::body_small_size(tokens)));
    }

    group.width(Length::Fill).into()
}

/// A validated field (RFC-038 D1): a label, the current text, an optional
/// unit suffix, and a persistent validation error.
///
/// This primitive does **not** parse or validate. `error` is the caller's
/// own verdict, computed however the caller decides — the same shape
/// [`super::notice::notice`] takes a tone rather than deciding one itself.
/// That keeps this testable without knowing what a valid refresh interval
/// is, and keeps validation policy in the view layer, where RFC-038 §1c's
/// "no commit-on-save" constraint already has to live.
///
/// **The error is persistent by construction**, not by convention: this
/// function carries no internal widget state (no primitive in this module
/// does — every one is a pure function of its arguments, re-evaluated on
/// every render), and its signature has no focus/blur/interaction
/// parameter at all. There is nothing here through which a caller could
/// wire up "hide while focused" or "clear after N seconds" even by
/// accident — `error` renders whenever it is `Some`, every call,
/// unconditionally. [`shows_error`] is the one bit of decision logic that
/// exists, split out so that fact is pinned by a test rather than resting
/// on this doc comment alone.
///
/// `unit`, when present, renders beside the field (e.g. "seconds") rather
/// than inside the input as part of the value — keeping the raw text the
/// input carries exactly what the user typed, with nothing appended that
/// `on_change` would then have to strip back out.
#[must_use]
pub fn validated_field<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    unit: Option<&'a str>,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    use iced::widget::{column, row, text, text_input};

    let field = text_input(placeholder, value)
        .on_input(on_change)
        .padding([0, 12])
        .width(Length::Fill)
        .size(snora::design::style::text::body_size(tokens));

    let mut field_row = row![field].spacing(8).align_y(iced::Alignment::Center);
    if let Some(unit) = unit {
        field_row =
            field_row.push(text(unit).size(snora::design::style::text::body_small_size(tokens)));
    }

    let mut group = column![
        text(label).size(snora::design::style::text::body_size(tokens)),
        field_row
    ]
    .spacing(8);

    if shows_error(error) {
        let danger = snora::design::style::color::to_iced_color(tokens.palette.danger_text);
        group = group.push(
            text(error.unwrap_or_default())
                .size(snora::design::style::text::body_small_size(tokens))
                .color(danger),
        );
    }

    group.width(Length::Fill).into()
}

/// Whether [`validated_field`] renders its error line. There is no input to
/// this decision beyond `error` itself — no focus, blur, or render-count
/// state, because [`validated_field`] carries none to give it. Split out
/// so RFC-038 R4's "does not silently coerce, error persists" requirement
/// is checkable without constructing an `Element`.
fn shows_error(error: Option<&str>) -> bool {
    error.is_some()
}

#[cfg(test)]
mod tests {
    use super::shows_error;

    /// The one decision `validated_field` makes about its error line:
    /// show it exactly when the caller supplied one. RFC-038 R4 — proven
    /// here rather than only documented, and by analogy with RFC-042 R3,
    /// this was watched to fail before the fix (see the review request)
    /// the same way `buttons.rs`'s `reason_row_needed` test was.
    #[test]
    fn error_shows_whenever_one_is_given_and_never_otherwise() {
        assert!(shows_error(Some("refresh interval must be a number")));
        assert!(!shows_error(None));
    }
}
