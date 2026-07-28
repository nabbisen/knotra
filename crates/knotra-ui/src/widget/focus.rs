//! Focus IDs and keyboard focus tasks (Phase 6 — accessibility).

/// Stable widget IDs for text inputs that must be programmatically focusable.
pub mod focus_id {
    use iced::widget::Id;
    use std::sync::LazyLock;

    pub static SEARCH: LazyLock<Id> = LazyLock::new(|| Id::new("dashboard-search"));
    pub static PALETTE_QUERY: LazyLock<Id> = LazyLock::new(|| Id::new("palette-query"));
    pub static ADD_PROJECT_PATH: LazyLock<Id> = LazyLock::new(|| Id::new("add-project-path"));
    pub static ADD_PROJECT_NAME: LazyLock<Id> = LazyLock::new(|| Id::new("add-project-name"));
    pub static WORKSPACE_NAME: LazyLock<Id> = LazyLock::new(|| Id::new("workspace-name"));
    pub static RELEASE_NAME: LazyLock<Id> = LazyLock::new(|| Id::new("release-name"));
    pub static SWITCH_TARGET: LazyLock<Id> = LazyLock::new(|| Id::new("switch-target"));
}

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
