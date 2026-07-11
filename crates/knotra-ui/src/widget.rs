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

// --- Accessibility tokens (UX review) ---------------------------------------
// Non-technical users benefit from larger hit targets and more readable body
// text. Primary and secondary actions use BUTTON_HEIGHT; small/inline controls
// inside dense read-only rows may use SMALL_BUTTON_HEIGHT.

/// Minimum height for any primary or secondary action control.
pub const BUTTON_HEIGHT: f32 = 44.0;

/// Reduced height permitted only for inline controls in dense, read-only rows.
pub const SMALL_BUTTON_HEIGHT: f32 = 36.0;

/// Body text size for non-technical-facing content.
pub const FONT_BODY: f32 = 15.0;

/// Small text size for metadata, timestamps, and captions.
pub const FONT_SMALL: f32 = 13.0;

// ---------------------------------------------------------------------------
// Safe component helpers (Phase 2 — non-technical UX)
// ---------------------------------------------------------------------------
//
// These are thin view-layer helpers, not new types, so they slot into the
// existing widget module without adding architectural complexity.

/// A button that shows a plain-text reason beneath it when disabled.
///
/// When `on_press` is `None` and `reason` is `Some`, renders the reason as
/// small muted text below the button so the user always knows *why* they
/// cannot proceed. When `on_press` is `Some`, renders a plain button.
///
/// # Arguments
/// * `label`    — Button text.
/// * `on_press` — Message to emit, or `None` to disable.
/// * `reason`   — Optional explanation shown only when disabled.
pub fn guided_button<'a, Message: Clone + 'a>(
    label: &'a str,
    on_press: Option<Message>,
    reason: Option<&'a str>,
) -> Element<'a, Message> {
    use crate::widget::BUTTON_HEIGHT;
    use iced::widget::{column, text};

    let btn = button(text(label).size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 18]);

    let show_reason = on_press.is_none();

    let btn: Element<'a, Message> = match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    };

    match reason {
        Some(r) if show_reason => column![btn, text(r).size(FONT_SMALL)].spacing(6).into(),
        _ => btn,
    }
}

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
) -> Element<'a, Message> {
    guided_field_with_id(label, placeholder, value, on_change, error, None)
}

/// Same as [`guided_field`] but assigns a [`widget::Id`] to the input for
/// programmatic focus (e.g. auto-focus when a dialog opens).
pub fn guided_field_focused<'a, Message: Clone + 'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    guided_field_with_id(label, placeholder, value, on_change, error, Some(id))
}

fn guided_field_with_id<'a, Message: Clone + 'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    id: Option<iced::widget::Id>,
) -> Element<'a, Message> {
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

// ---------------------------------------------------------------------------
// Focus IDs and keyboard focus tasks (Phase 6 — accessibility)
// ---------------------------------------------------------------------------

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
