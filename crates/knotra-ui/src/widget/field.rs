//! Safe field helpers (Phase 2 — non-technical UX).
//!
//! These are thin view-layer helpers, not new types, so they slot into the
//! existing widget module without adding architectural complexity.

use super::layout::{FONT_BODY, FONT_SMALL, Length};

/// A labelled text input with persistent label above and optional inline error.
///
/// The label always stays visible (not a placeholder that disappears on
/// focus). An error message, when present, appears beneath the field in a
/// small, clearly distinct style.
pub fn guided_field<'a, Message: Clone + 'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
) -> iced::Element<'a, Message> {
    guided_field_with_id(label, placeholder, value, on_change, error, None)
}

/// Same as [`guided_field`] but assigns a [`iced::widget::Id`] to the input for
/// programmatic focus (e.g. auto-focus when a dialog opens).
pub fn guided_field_focused<'a, Message: Clone + 'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    id: iced::widget::Id,
) -> iced::Element<'a, Message> {
    guided_field_with_id(label, placeholder, value, on_change, error, Some(id))
}

fn guided_field_with_id<'a, Message: Clone + 'a>(
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
        .size(FONT_BODY);

    if let Some(widget_id) = id {
        field = field.id(widget_id);
    }

    let mut group = column![text(label).size(FONT_BODY), field,].spacing(8);

    if let Some(err) = error {
        group = group.push(text(err).size(FONT_SMALL));
    }

    group.width(Length::Fill).into()
}
