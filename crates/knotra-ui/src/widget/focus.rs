//! Keyboard focus tasks (Phase 6 — accessibility).
//!
//! RFC-075 stage 5 §1: this file's own `focus_id` module (every text-field
//! id outside the main model) moved into `crate::view::ids` at `knotra-app`
//! — the one place every other control's own id already lived. Nothing here
//! builds an `Id` any more; `focus_input`/`clear_input_focus` only move or
//! clear focus by whatever `Id` their own caller already has.

/// Produce a `Task` that moves keyboard focus to the text input with the given ID.
pub fn focus_input<Message: 'static>(id: &iced::widget::Id) -> iced::Task<Message> {
    iced::widget::operation::focus(id.clone())
}

/// Produce a `Task` that clears iced's own text-input focus (RFC-036 R12).
///
/// `operation::focus` unfocuses every focusable widget that does not match
/// its target `Id`; passing a freshly minted unique `Id`, which cannot match
/// any real widget, unfocuses all of them. Used when knotra-focus moves off
/// a text input onto a non-text-input control, so the field does not keep
/// receiving keystrokes after the visible focus ring has moved away from it.
pub fn clear_input_focus<Message: 'static>() -> iced::Task<Message> {
    iced::widget::operation::focus(iced::widget::Id::unique())
}
