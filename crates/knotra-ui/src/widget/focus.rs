//! Keyboard focus tasks (Phase 6 — accessibility).
//!
//! RFC-075 stage 5 §1: this file's own `focus_id` module (every text-field
//! id outside the main model) moved into `crate::view::ids` at `knotra-app`
//! — the one place every other control's own id already lived. Nothing here
//! builds an `Id` any more; `focus_input` only moves focus by whatever `Id`
//! its own caller already has.

/// Produce a `Task` that moves keyboard focus to the text input with the given ID.
pub fn focus_input<Message: 'static>(id: &iced::widget::Id) -> iced::Task<Message> {
    iced::widget::operation::focus(id.clone())
}
