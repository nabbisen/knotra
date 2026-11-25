//! Button helpers: the RFC-034 semantic control vocabulary (R6/R7).
//!
//! The semantic variants are thin wrappers around `snora::design::button`.
//! Application view code must import them from here, never from
//! `snora::design` directly (RFC-034 R2) — this module is the single crossing
//! point.
//!
//! `reasoned` (RFC-037 D7) is the semantic vocabulary's missing half:
//! `primary_maybe`/`secondary_maybe`/`ghost_maybe`/`danger_maybe` all carry
//! `Option<Message>`, but none carries the reason-beneath-when-disabled
//! composition `guided_button` (deleted, RFC-037 Stage 6 R9) used to provide,
//! and none exposes a focus ring. `reasoned` is a token-aware,
//! focus-ring-capable, reason-carrying button, built once here so RFC-037
//! Stage 6 did not leave ten call sites each duplicating the composition.

use snora::design::Tokens;

use super::layout::{BUTTON_HEIGHT, Element, Length};

/// Centres a button's label text vertically, filling the button's fixed
/// height.
///
/// **RFC-062 D1**: iced 0.14's `button` layout (`layout::padded`) places its
/// content at the padding offset — top-left — and never centres it, which
/// only shows once a button's fixed height ([`BUTTON_HEIGHT`]/
/// `SMALL_BUTTON_HEIGHT`) exceeds its label's own line height. Apply this to
/// the `text(...)` each call site already builds, after its own
/// `.size(...)`/`.line_height(...)` (or, for the one site with neither, in
/// place of them) — the label's content, size and line height are
/// untouched; only where it sits inside the button changes.
///
/// Width is deliberately never set here: every one of knotra's fixed-height
/// buttons is content-sized horizontally by its own `[0, 18]` padding
/// (already centred by that symmetric padding), and a filled width would
/// stretch it across its row.
///
/// RFC-073 Amendment A7 §1: `Wrapping::None` too — without it, a button
/// placed in a row too narrow for its own label (the detail panel's
/// Actions row, after Stage 4 lengthened "Fetch" to "Check for updates")
/// let the label wrap onto a second line *inside* the button's own fixed
/// height, clipping it, rather than reporting its true natural width and
/// making the row (now `.wrap()`'d at each such call site) carry the
/// button to its own line instead. The label guard (`label_overflow_
/// guard.rs`) covers the three detail-panel buttons this protects,
/// confirmed to still pass everywhere else this helper is already used.
#[must_use]
pub fn centered_button_label<'a>(
    label: iced::widget::Text<'a, iced::Theme, iced::Renderer>,
) -> iced::widget::Text<'a, iced::Theme, iced::Renderer> {
    label
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .wrapping(iced::advanced::text::Wrapping::None)
}

/// The same centring [`centered_button_label`] gives a bare `Text`, for a
/// `Row` instead — `header_control`'s own construction (icon beside text),
/// generalized (RFC-073 Amendment A2) so every icon-and-text fixed-height
/// button shares one helper rather than each hand-rolling the same two
/// calls.
#[must_use]
pub fn centered_button_row<'a, Message: 'a>(
    row: iced::widget::Row<'a, Message, iced::Theme, iced::Renderer>,
) -> iced::widget::Row<'a, Message, iced::Theme, iced::Renderer> {
    row.height(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
}

/// A token-aware, focus-ring-capable button that shows a plain-text reason
/// beneath it when disabled — the composition the deleted `guided_button`
/// used to provide, built on the semantic style vocabulary instead of the
/// default theme (RFC-037 D7). On `snora::design::control` (RFC-075 stage
/// 3) — a real focus target, drawing its own ring from real focus, with no
/// `is_focused` parameter to supply.
#[must_use]
pub fn reasoned_control_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    reason: Option<&'a str>,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    use iced::widget::{column, text};

    let show_reason = reason_row_needed(on_press.is_none(), reason.is_some());
    let btn = make_control(tokens, label, on_press, style_fn, id);

    match reason {
        // RFC-071 Amendment A3 D14: a reason is read to understand why a
        // control can't be used right now — `body`, not incidental
        // metadata.
        Some(r) if show_reason => column![
            btn,
            text(r)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ]
        .spacing(6)
        .into(),
        _ => btn,
    }
}

/// Whether `reasoned` should render its reason text: only when there is no
/// press handler *and* a reason was supplied — the same rule the deleted
/// `guided_button` used. Split out as plain booleans so this one piece of
/// logic is unit-testable without constructing an `Element`.
fn reason_row_needed(on_press_is_none: bool, reason_is_some: bool) -> bool {
    on_press_is_none && reason_is_some
}

// ---------------------------------------------------------------------------
// Semantic control vocabulary (RFC-034 R6/R7).
// ---------------------------------------------------------------------------

/// `iced` style functions for the semantic control variants, for callers
/// that build their own `control(...)`/`button(...)` rather than using the
/// `Element` constructors below — e.g. because the content is not a plain
/// label, or because a fixed [`iced::widget::button::Status`] is needed
/// (see [`current_or_control`]).
pub mod style {
    use iced::widget::button::{Status, Style};
    use iced::{Border, Color};
    use snora::design::Tokens;

    use super::super::ring::ring_color_for;

    fn to_iced(color: snora::design::Color) -> Color {
        snora::design::style::color::to_iced_color(color)
    }

    /// RFC-071 Amendment A1 D13 item 1, corrected by Amendment A5: replaces
    /// snora's own `Disabled` result (a uniform 45% alpha fade — `≈2.2:1`
    /// for a filled `primary` button's label, below every WCAG level for
    /// text) with knotra's own: no fill, the label in `text_muted` at full
    /// opacity (4.9:1 / 5.4:1 light/dark — the theme's own AA contrast test
    /// already covers `text_muted`).
    ///
    /// **`outlined` is per variant, not universal (A5's correction to A1).**
    /// A1 gave every variant a 1px border when disabled, including `ghost`
    /// — whose *available* form has no border at all, no fill, nothing but
    /// coloured text. That made an unavailable `ghost` command gain a box
    /// its available form never has, so in the command palette the
    /// unavailable commands looked more button-like than the available
    /// ones: the opposite of the intent. The rule now: an unavailable
    /// control never gains a shape its available form lacks. `primary`/
    /// `secondary`/`danger` are already box-shaped when available (filled
    /// or outlined), so they stay box-shaped, outlined, when disabled.
    /// `ghost` is bare text when available, so it stays bare text —
    /// `outlined: false` — when disabled too; only its colour changes.
    fn readable_when_disabled(
        tokens: &Tokens,
        status: Status,
        computed: Style,
        outlined: bool,
    ) -> Style {
        if status != Status::Disabled {
            return computed;
        }
        let border = if outlined {
            Border {
                width: 1.0,
                color: to_iced(tokens.palette.border),
                radius: computed.border.radius,
            }
        } else {
            Border {
                width: 0.0,
                ..computed.border
            }
        };
        Style {
            background: None,
            text_color: to_iced(tokens.palette.text_muted),
            border,
            ..computed
        }
    }

    pub fn primary(tokens: &Tokens, status: Status) -> Style {
        readable_when_disabled(
            tokens,
            status,
            snora::design::style::button::primary(tokens, status),
            true,
        )
    }

    pub fn secondary(tokens: &Tokens, status: Status) -> Style {
        readable_when_disabled(
            tokens,
            status,
            snora::design::style::button::secondary(tokens, status),
            true,
        )
    }

    pub fn ghost(tokens: &Tokens, status: Status) -> Style {
        readable_when_disabled(
            tokens,
            status,
            snora::design::style::button::ghost(tokens, status),
            false,
        )
    }

    pub fn danger(tokens: &Tokens, status: Status) -> Style {
        readable_when_disabled(
            tokens,
            status,
            snora::design::style::button::danger(tokens, status),
            true,
        )
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
    /// unreadable. [`ring_color_for`] (`widget/ring.rs`, moved there per
    /// RFC-035 Handoff 020 §7.2 once it had callers beyond buttons) picks
    /// whichever of `ring_color` or `accent_text` has the higher measured
    /// contrast against an opaque background, rather than assuming which one
    /// wins by control type. `ghost`/`secondary` never have an opaque
    /// background (ghost/secondary's own rest/hover/press backgrounds top
    /// out at alpha 0.14), so they always keep the plain `ring_color`,
    /// unchanged from Stage 2.
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
}

/// Style a control that is *current* rather than pressable: the active
/// variant always renders at full strength, never faded by iced's
/// `Status::Disabled` (RFC-033 D4; RFC-034 R12).
///
/// `on_press`-suppression and visual state are independent concerns for a
/// "you are here" indicator, but iced conflates them by default — a control
/// with no `on_press` reports `Status::Disabled`, which every style function
/// fades. This feeds the *active* branch a fixed `Status::Active` so it
/// always renders at full strength regardless of interactivity, while the
/// *inactive* branch keeps the real `status` so hover/press feedback still
/// works on it. On `snora::design::control`, which draws its own ring from
/// real focus — no `is_focused` parameter to supply, unlike `with_focus_
/// ring`'s own callers.
///
/// ```rust,ignore
/// control(tokens, text(label))
///     .on_press_maybe((!active).then_some(message))
///     .style(move |_theme, status| current_or_control(active, &tokens, status))
/// ```
pub fn current_or_control(
    active: bool,
    tokens: &snora::design::Tokens,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    if active {
        style::secondary(tokens, iced::widget::button::Status::Active)
    } else {
        style::ghost(tokens, status)
    }
}

// ---------------------------------------------------------------------------
// RFC-034 R6/R7: the semantic control vocabulary, on `snora::design::
// control` — each is a real focus target, with `control`'s own ring,
// Enter/Space press, and "a disabled control is no Tab stop" (RFC-075 D4
// b). RFC-075 stage 5 retired the earlier `iced::widget::button`-based
// family these once sat beside (`primary`/`secondary`/`ghost`/`danger`
// and their own `_maybe` forms, `make_button`) — these are now the only
// semantic button vocabulary this crate has, so the `_control` suffix no
// longer tells two families apart; it is kept anyway (RFC-075 handoff
// stage 5 §4), since renaming every call site back would touch hundreds
// of them for no reader's benefit, and the names stay unambiguous either
// way.
// ---------------------------------------------------------------------------

/// Shared construction for the `_control` variants, on `snora::design::
/// control`. `id` is required, not optional (RFC-075 D3: "every new
/// primitive takes an id, for stage 3's entries").
fn make_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let t = tokens.clone();
    snora::design::control(
        tokens,
        centered_button_label(
            iced::widget::text(label.into())
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ),
    )
    .height(BUTTON_HEIGHT)
    .id(id)
    .padding([0.0, 18.0])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style_fn(&t, status))
    .into()
}

/// Filled accent control — [`primary`]'s own twin, on `snora::design::control`.
#[must_use]
pub fn primary_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, Some(on_press), style::primary, id)
}

/// Filled accent control with an optional press handler (disabled, and not
/// a Tab stop, when `None` — D4 b).
#[must_use]
pub fn primary_control_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, on_press, style::primary, id)
}

/// Outlined accent control — [`secondary`]'s own twin.
#[must_use]
pub fn secondary_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, Some(on_press), style::secondary, id)
}

/// Outlined accent control with an optional press handler.
#[must_use]
pub fn secondary_control_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, on_press, style::secondary, id)
}

/// Ghost control — [`ghost`]'s own twin.
#[must_use]
pub fn ghost_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, Some(on_press), style::ghost, id)
}

/// Ghost control with an optional press handler.
#[must_use]
pub fn ghost_control_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, on_press, style::ghost, id)
}

/// Danger / destructive control — [`danger`]'s own twin.
#[must_use]
pub fn danger_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, Some(on_press), style::danger, id)
}

/// Danger control with an optional press handler.
#[must_use]
pub fn danger_control_maybe<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    make_control(tokens, label, on_press, style::danger, id)
}

#[cfg(test)]
mod tests {
    use super::reason_row_needed;
    use super::style::with_focus_ring;
    use iced::widget::button::Status;
    use snora::design::Tokens;

    /// RFC-075 handoff stage 1 §3, row 3: each `_control` variant is a Tab
    /// stop, Enter and Space press it (publishing the same message a click
    /// would), and a disabled one (`on_press: None`) is skipped — not just
    /// unpressable, genuinely no Tab stop at all (D4 b), through
    /// `knotra_ui::test_support::Harness`, the same `UserInterface`-level
    /// driving stage 0 found `iced_test::Simulator` cannot do for a
    /// `traverse`-routed Tab.
    #[test]
    fn primary_control_is_a_tab_stop_enter_and_space_press_it_disabled_is_skipped() {
        use crate::test_support::Harness;
        use iced::widget::{Id, column};

        #[derive(Debug, Clone, PartialEq)]
        enum Msg {
            Pressed,
        }

        let tokens = Tokens::light();
        let view = |pressable: bool| -> iced::Element<'static, Msg> {
            column![
                super::primary_control_maybe(
                    &tokens,
                    "Save",
                    pressable.then_some(Msg::Pressed),
                    Id::new("save"),
                ),
                super::ghost_control(&tokens, "Cancel", Msg::Pressed, Id::new("cancel")),
            ]
            .spacing(8)
            .into()
        };

        let size = iced_test::core::Size {
            width: 400.0,
            height: 400.0,
        };

        // Enabled: Tab reaches it, Enter presses it.
        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(Id::new("save")));
        h.enter();
        assert_eq!(h.messages(), [Msg::Pressed]);

        // Enabled: Space presses it too.
        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        h.space();
        assert_eq!(h.messages(), [Msg::Pressed]);

        // Disabled: Tab skips it entirely and lands on "Cancel" instead.
        let mut h = Harness::new(view(false), size, None);
        h.tab(false);
        assert_eq!(
            h.focused_id(),
            Some(Id::new("cancel")),
            "a disabled control must not be a Tab stop (RFC-075 D4 b)"
        );
    }

    /// `reasoned_control_maybe`'s reason text renders in exactly the same case
    /// `guided_button` used: no press handler, and a reason was supplied.
    /// Either condition failing hides it.
    #[test]
    fn reason_shows_only_when_disabled_and_a_reason_was_given() {
        assert!(reason_row_needed(true, true), "disabled + reason: shows");
        assert!(
            !reason_row_needed(true, false),
            "disabled, no reason: nothing to show"
        );
        assert!(
            !reason_row_needed(false, true),
            "pressable: reason stays hidden even if supplied"
        );
        assert!(
            !reason_row_needed(false, false),
            "pressable, no reason: hidden"
        );
    }

    /// RFC-036 Stage 6: a filled-accent (`primary`) style must not receive
    /// the same ring color as a transparent (`ghost`/`secondary`) one, in
    /// either theme — that sameness (or near-sameness) was Finding 1's root
    /// cause.
    #[test]
    fn filled_accent_style_gets_a_different_ring_color_than_a_transparent_one() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
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
        let tokens = crate::theme::KnotraTheme::dark().tokens;
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
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
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

    /// RFC-071 Amendment A1 D13 item 1: every semantic variant's `Disabled`
    /// result has no fill, a 1px border, and a `text_muted` label — not
    /// snora's own faded, still-tinted result. Checked for all four
    /// variants, in both themes, so none can keep the old fade silently.
    #[test]
    fn disabled_is_outline_only_with_text_muted_label_for_every_variant() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let expected_text =
                snora::design::style::color::to_iced_color(tokens.palette.text_muted);
            let expected_border = snora::design::style::color::to_iced_color(tokens.palette.border);

            type StyleFn = fn(&Tokens, Status) -> iced::widget::button::Style;
            // Amendment A5: `outlined` is per variant. `primary`/`secondary`/
            // `danger` are box-shaped when available (filled or outlined),
            // so they stay box-shaped, with a 1px border, when disabled.
            // `ghost` has no border at all when available (bare text), so
            // it must not gain one when disabled either.
            let variants: [(&str, StyleFn, bool); 4] = [
                ("primary", super::style::primary, true),
                ("secondary", super::style::secondary, true),
                ("ghost", super::style::ghost, false),
                ("danger", super::style::danger, true),
            ];
            for (name, style_fn, outlined) in variants {
                let disabled = style_fn(&tokens, Status::Disabled);
                let active = style_fn(&tokens, Status::Active);

                assert_eq!(
                    disabled.background, None,
                    "{name}: Disabled must have no fill"
                );
                assert_eq!(
                    disabled.text_color, expected_text,
                    "{name}: Disabled label must be `text_muted`, at full opacity"
                );
                assert_eq!(
                    expected_text.a, 1.0,
                    "{name}: text_muted itself must not already carry a reduced alpha"
                );
                assert_ne!(
                    disabled.text_color, active.text_color,
                    "{name}: Disabled must look different from Active"
                );

                if outlined {
                    assert_eq!(
                        disabled.border.width, 1.0,
                        "{name}: Disabled must have a 1px border"
                    );
                    assert_eq!(
                        disabled.border.color, expected_border,
                        "{name}: Disabled border must be the `border` token"
                    );
                } else {
                    assert_eq!(
                        disabled.border.width, 0.0,
                        "{name}: Disabled must not gain a border its available form lacks"
                    );
                    assert_eq!(
                        disabled.border.width, active.border.width,
                        "{name}: Disabled border width must match Active's own \
                         (never a shape the available form lacks)"
                    );
                }
            }
        }
    }
}
