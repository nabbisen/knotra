//! Button helpers: the legacy `guided_button` plus the RFC-034 semantic
//! control vocabulary (R6/R7).
//!
//! The semantic variants are thin wrappers around `snora::design::button`.
//! Application view code must import them from here, never from
//! `snora::design` directly (RFC-034 R2) — this module is the single crossing
//! point.

use snora::design::Tokens;

use super::icon::icon_element;
use super::layout::{BUTTON_HEIGHT, Element, FONT_BODY, FONT_SMALL, Length};

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
    use iced::widget::{button, column, text};

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

// ---------------------------------------------------------------------------
// Semantic control vocabulary (RFC-034 R6/R7).
//
// Additive: these do not replace `guided_button`. Existing call sites keep
// using `guided_button` until their own migration RFC; new call sites
// (RFC-034 stages 3-4 onward) use the variant matching their action's role.
// ---------------------------------------------------------------------------

/// Filled accent button — the single strongest action on a surface.
#[must_use]
pub fn primary<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    snora::design::button::primary(tokens, label, on_press)
}

/// Filled accent button with an optional press handler (disabled when `None`).
#[must_use]
pub fn primary_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    snora::design::button::primary_maybe(tokens, label, on_press)
}

/// Outlined accent button — a secondary action alongside a [`primary`].
#[must_use]
pub fn secondary<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    snora::design::button::secondary(tokens, label, on_press)
}

/// Outlined accent button with an optional press handler.
#[must_use]
pub fn secondary_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    snora::design::button::secondary_maybe(tokens, label, on_press)
}

/// Ghost button — no fill or border at rest; ordinary/tertiary action.
#[must_use]
pub fn ghost<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    snora::design::button::ghost(tokens, label, on_press)
}

/// Ghost button with an optional press handler.
#[must_use]
pub fn ghost_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    snora::design::button::ghost_maybe(tokens, label, on_press)
}

/// Danger / destructive button — irreversible actions only (delete, revoke).
#[must_use]
pub fn danger<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
) -> Element<'a, Message> {
    snora::design::button::danger(tokens, label, on_press)
}

/// Danger button with an optional press handler.
#[must_use]
pub fn danger_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    snora::design::button::danger_maybe(tokens, label, on_press)
}

/// `iced` style functions for the semantic button variants, for callers that
/// build their own `button(...)` rather than using the `Element` constructors
/// above — e.g. because the content is not a plain label, or because a fixed
/// [`iced::widget::button::Status`] is needed (see [`current_or`]).
pub mod style {
    use iced::widget::button::{Status, Style};
    use snora::design::Tokens;

    pub fn primary(tokens: &Tokens, status: Status) -> Style {
        snora::design::style::button::primary(tokens, status)
    }

    pub fn secondary(tokens: &Tokens, status: Status) -> Style {
        snora::design::style::button::secondary(tokens, status)
    }

    pub fn ghost(tokens: &Tokens, status: Status) -> Style {
        snora::design::style::button::ghost(tokens, status)
    }

    pub fn danger(tokens: &Tokens, status: Status) -> Style {
        snora::design::style::button::danger(tokens, status)
    }

    /// Composes a visible focus ring (RFC-033 D7 `FocusTokens`) onto an
    /// already-computed style, when `is_focused` is true.
    ///
    /// iced's `button::Status` has no `Focused` variant (RFC-036 D1) — only
    /// `text_input` gets that from iced. The application must therefore know
    /// a control is focused independently and pass it in here; this function
    /// does not read any ambient focus state.
    ///
    /// `FocusTokens::ring_offset` (the gap between the control edge and the
    /// ring) is not applied: `iced::widget::button::Style` has only a single
    /// flush `border`, with no outer/offset ring primitive. The ring is drawn
    /// as a border override instead — a knotra-specific simplification, not
    /// a workaround copied from anywhere upstream.
    pub fn with_focus_ring(tokens: &Tokens, is_focused: bool, style: Style) -> Style {
        if !is_focused {
            return style;
        }
        let ring = tokens.focus;
        Style {
            border: iced::Border {
                color: snora::design::style::color::to_iced_color(ring.ring_color),
                width: ring.ring_width,
                radius: style.border.radius,
            },
            ..style
        }
    }
}

/// Style a control that is *current* rather than pressable: the active
/// variant always renders at full strength, never faded by iced's
/// `Status::Disabled` (RFC-033 D4; RFC-034 R12).
///
/// `on_press`-suppression and visual state are independent concerns for a
/// "you are here" indicator, but iced conflates them by default — a button
/// with no `on_press` reports `Status::Disabled`, which every style function
/// fades. This feeds the *active* branch a fixed `Status::Active` so it
/// always renders at full strength regardless of interactivity, while the
/// *inactive* branch keeps the real `status` so hover/press feedback still
/// works on it.
///
/// ```rust,ignore
/// button(text(label))
///     .on_press_maybe((!active).then_some(message))
///     .style(move |_theme, status| current_or(active, &tokens, status, is_focused))
/// ```
pub fn current_or(
    active: bool,
    tokens: &snora::design::Tokens,
    status: iced::widget::button::Status,
    is_focused: bool,
) -> iced::widget::button::Style {
    let base = if active {
        style::secondary(tokens, iced::widget::button::Status::Active)
    } else {
        style::ghost(tokens, status)
    };
    style::with_focus_ring(tokens, is_focused, base)
}

/// Icon-only button for familiar global commands (refresh, settings, close,
/// overflow). Always carries an `accessible_label`, shown as a hover tooltip,
/// so the control's purpose is never conveyed by the icon glyph alone.
///
/// There is no non-`_maybe` form: an icon-only control with no way to
/// indicate why it is inert would give the user no explanation at all, so
/// callers always thread their disabled state through `on_press`.
///
/// `is_focused` draws the RFC-036 focus ring when true — see
/// [`style::with_focus_ring`].
#[must_use]
pub fn icon_button_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    icon: &snora::Icon,
    accessible_label: impl Into<String>,
    on_press: Option<Message>,
    is_focused: bool,
) -> Element<'a, Message> {
    use iced::widget::{button, text, tooltip};

    let t = tokens.clone();
    let btn = button(icon_element(icon))
        .width(Length::Fixed(BUTTON_HEIGHT))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .on_press_maybe(on_press)
        .style(move |_theme, status| {
            style::with_focus_ring(&t, is_focused, style::ghost(&t, status))
        });

    tooltip(
        btn,
        super::overlay::raised_card(tokens, text(accessible_label.into()).size(FONT_SMALL)),
        tooltip::Position::Bottom,
    )
    .into()
}
