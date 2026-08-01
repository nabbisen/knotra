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

    /// Converts an `iced::Color` back to `snora::design::Color` — the
    /// reverse of `to_iced_color`. Needed here, and only here, to run
    /// contrast math on a button style's already-computed background
    /// (RFC-036 Stage 6); `snora-widgets` only ships the one-way conversion,
    /// same reason `knotra-ui/src/theme.rs`'s tests define their own.
    fn from_iced_color(c: iced::Color) -> snora::design::Color {
        snora::design::Color::rgba(c.r, c.g, c.b, c.a)
    }

    /// Returns the style's background color only when it is fully opaque —
    /// a genuine filled control (`primary`/`danger` at `Active`/`Hovered`/
    /// `Pressed`, all alpha `1.0`) rather than `ghost`/`secondary`'s
    /// transparent-at-rest or lightly-tinted hover/press background (max
    /// observed alpha there is 0.14), or a filled control's own
    /// disabled-state background (alpha 0.45, `disabled_alpha`).
    ///
    /// This must be exact opacity, not merely "high": `contrast_ratio`
    /// ignores alpha entirely (per `snora_design::contrast`'s own module
    /// docs, which say to `composite_over` first for a translucent color) —
    /// so a hypothetical future style at, say, alpha 0.7 would silently have
    /// its contrast winner computed against the raw, un-composited color if
    /// this let it through. Requiring `1.0` makes that impossible by
    /// construction rather than by a threshold that has to stay
    /// comfortably clear of every real alpha value in play (RFC-036 Stage 6
    /// closure, `.git-exclude/reviewed/083-...md` Finding 4). The disabled
    /// exclusion falls out of this definition rather than needing its own
    /// justification — see `theme.rs`'s WCAG test for why disabled controls
    /// specifically must not take this branch.
    fn filled_background(background: Option<iced::Background>) -> Option<iced::Color> {
        match background {
            Some(iced::Background::Color(c)) if c.a >= 1.0 => Some(c),
            _ => None,
        }
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
    ///
    /// **RFC-036 Stage 6 (D7 fix):** `tokens.focus.ring_color` is not
    /// automatically high-contrast against every background — in both
    /// presets it sits close in luminance to `accent` (and, in the light
    /// preset, is the *same* color), so a `primary` button's own ring was
    /// unreadable. When the incoming style has an opaque (filled) background,
    /// this picks whichever of `ring_color` or `accent_text` — the palette
    /// role already defined to read on top of `accent` — has the higher
    /// measured contrast against that specific background, rather than
    /// assuming which one wins by control type. `ghost`/`secondary` never
    /// have an opaque background (ghost/secondary's own rest/hover/press
    /// backgrounds top out at alpha 0.14), so they always keep the plain
    /// `ring_color`, unchanged from Stage 2.
    pub fn with_focus_ring(tokens: &Tokens, is_focused: bool, style: Style) -> Style {
        if !is_focused {
            return style;
        }
        Style {
            border: iced::Border {
                color: ring_color_for(tokens, style.background),
                width: tokens.focus.ring_width,
                radius: style.border.radius,
            },
            ..style
        }
    }

    /// The RFC-036 Stage 6 ring-color decision (`ring_color` vs `accent_text`,
    /// whichever has the higher measured contrast against an opaque
    /// background), generalized so every control that draws a focus ring —
    /// not only buttons — makes the same choice instead of a second
    /// implementation. `select` and `checkbox` (RFC-035 Handoff 019 §7.2)
    /// reuse this directly, since their own `Style` types are not
    /// `button::Style` and cannot go through [`with_focus_ring`] itself.
    pub(crate) fn ring_color_for(
        tokens: &Tokens,
        background: Option<iced::Background>,
    ) -> iced::Color {
        let ring = tokens.focus;
        let default_ring_color = snora::design::style::color::to_iced_color(ring.ring_color);

        match filled_background(background) {
            Some(bg) => {
                let bg = from_iced_color(bg);
                let default_contrast = snora::design::contrast::contrast_ratio(ring.ring_color, bg);
                let alt_contrast =
                    snora::design::contrast::contrast_ratio(tokens.palette.accent_text, bg);
                if alt_contrast > default_contrast {
                    snora::design::style::color::to_iced_color(tokens.palette.accent_text)
                } else {
                    default_ring_color
                }
            }
            None => default_ring_color,
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

#[cfg(test)]
mod tests {
    use super::style::with_focus_ring;
    use iced::widget::button::Status;
    use snora::design::Tokens;

    /// RFC-036 Stage 6: a filled-accent (`primary`) style must not receive
    /// the same ring color as a transparent (`ghost`/`secondary`) one, in
    /// either theme — that sameness (or near-sameness) was Finding 1's root
    /// cause.
    #[test]
    fn filled_accent_style_gets_a_different_ring_color_than_a_transparent_one() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            let filled = super::style::primary(&tokens, Status::Active);
            let transparent = super::style::ghost(&tokens, Status::Active);

            let filled_ring = with_focus_ring(&tokens, true, filled).border.color;
            let transparent_ring = with_focus_ring(&tokens, true, transparent).border.color;

            assert_ne!(
                filled_ring, transparent_ring,
                "primary and ghost rings must differ once the primary background \
                 is close enough in luminance to the plain ring color to need a winner"
            );
        }
    }

    /// `is_focused: false` must never touch the ring color decision at all —
    /// same guarantee Stage 2 established, re-asserted here now that the
    /// decision has a branch.
    #[test]
    fn unfocused_style_is_untouched() {
        let tokens = Tokens::dark();
        let base = super::style::primary(&tokens, Status::Active);
        let result = with_focus_ring(&tokens, false, base);
        assert_eq!(result.border, base.border);
    }

    /// `ghost` and `secondary` never have an opaque background at any
    /// status (`Active`/`Hovered`/`Pressed` top out at alpha 0.14,
    /// `Disabled` is fully transparent), so Stage 6's contrast branch must
    /// never fire for them — their ring stays exactly
    /// `tokens.focus.ring_color`, byte-identical to the pre-Stage-6
    /// unconditional formula, in both themes. Backs the acceptance
    /// criterion that ghost/secondary rendering is unchanged.
    #[test]
    fn ghost_and_secondary_ring_color_is_unchanged_by_the_contrast_branch() {
        for tokens in [Tokens::dark(), Tokens::light()] {
            let expected = snora::design::style::color::to_iced_color(tokens.focus.ring_color);

            for status in [
                Status::Active,
                Status::Hovered,
                Status::Pressed,
                Status::Disabled,
            ] {
                let ghost_ring =
                    with_focus_ring(&tokens, true, super::style::ghost(&tokens, status))
                        .border
                        .color;
                let secondary_ring =
                    with_focus_ring(&tokens, true, super::style::secondary(&tokens, status))
                        .border
                        .color;

                assert_eq!(ghost_ring, expected, "ghost ring changed for {status:?}");
                assert_eq!(
                    secondary_ring, expected,
                    "secondary ring changed for {status:?}"
                );
            }
        }
    }
}
