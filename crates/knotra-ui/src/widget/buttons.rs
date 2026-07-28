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

/// Icon-only button for familiar global commands (refresh, settings, close,
/// overflow). Always carries an `accessible_label`, shown as a hover tooltip,
/// so the control's purpose is never conveyed by the icon glyph alone.
///
/// There is no non-`_maybe` form: an icon-only control with no way to
/// indicate why it is inert would give the user no explanation at all, so
/// callers always thread their disabled state through `on_press`.
#[must_use]
pub fn icon_button_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    icon: &snora::Icon,
    accessible_label: impl Into<String>,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    use iced::widget::{button, text, tooltip};

    let t = tokens.clone();
    let btn = button(icon_element(icon))
        .width(Length::Fixed(BUTTON_HEIGHT))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .on_press_maybe(on_press)
        .style(move |_theme, status| snora::design::style::button::ghost(&t, status));

    tooltip(
        btn,
        snora::design::card::raised(tokens, text(accessible_label.into()).size(FONT_SMALL)),
        tooltip::Position::Bottom,
    )
    .into()
}
