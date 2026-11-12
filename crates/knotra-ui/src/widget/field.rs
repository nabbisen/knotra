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

use super::layout::{BUTTON_HEIGHT, Element, Length};

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// RFC-071 D6: the one `text_input` style in `knotra-ui` — every
/// `text_input` in `crates/` goes through this. Placeholder is
/// `text_muted`, typed value is `text_primary`; everything else (fill,
/// border, selection, the icon slot) comes from the active theme's tokens
/// too, so the field restyles with RFC-070's high-contrast presets like
/// every other control, rather than keeping iced's own hard-coded default.
///
/// **RFC-070 Amendment A2: focus is the ring, not a border recolour.**
/// Before A2, `Focused` only swapped the 1px border's colour to `accent` —
/// in HC dark, `accent` and `border` are both near white, so a keyboard
/// user could not see which field had focus (WCAG 2.4.7). `Focused` now
/// draws [`super::ring::ring_color_for`]'s choice at `tokens.focus.ring_width`
/// (2px normal, 3px HC) — the same ring every other control
/// (`buttons::style::with_focus_ring`, `select::menu_style`) already draws.
///
/// **A2: `Disabled` stays readable.** Before A2, the background and the
/// value both faded to 50% opacity — the same faint-unavailable-control
/// defect RFC-071 Amendment A1 D13 fixed for buttons, never applied here.
/// `Disabled` now keeps full opacity throughout: `value` moves to
/// `text_muted` (full alpha) rather than a faded `text_primary`, and the
/// border and background are untouched, same as `Active`.
pub fn text_input_style(
    tokens: &Tokens,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input::{Status, Style};
    use iced::{Background, Border, Color};

    let background = to_iced(tokens.palette.surface);
    let border_color = to_iced(tokens.palette.border);
    let accent = to_iced(tokens.palette.accent);
    let radius = tokens.radius.md;

    let active = Style {
        background: Background::Color(background),
        border: Border {
            radius: radius.into(),
            width: 1.0,
            color: border_color,
        },
        icon: to_iced(tokens.palette.text_secondary),
        placeholder: to_iced(tokens.palette.text_muted),
        value: to_iced(tokens.palette.text_primary),
        selection: Color { a: 0.3, ..accent },
    };

    match status {
        Status::Active => active,
        Status::Hovered => Style {
            border: Border {
                color: to_iced(tokens.palette.text_secondary),
                ..active.border
            },
            ..active
        },
        Status::Focused { .. } => Style {
            border: Border {
                color: super::ring::ring_color_for(tokens, Some(active.background)),
                width: tokens.focus.ring_width,
                ..active.border
            },
            ..active
        },
        Status::Disabled => Style {
            value: to_iced(tokens.palette.text_muted),
            ..active
        },
    }
}

/// RFC-073 handoff Amendment A3 §2: the one `text_input(...)` builder every
/// field helper and ad-hoc call site goes through — `text_input` has no
/// `.height(...)` of its own (unlike `button`), so `BUTTON_HEIGHT` is
/// reached by padding instead: vertical padding of half of
/// `BUTTON_HEIGHT` minus `body`'s own line height, the same "shrink to fit,
/// then the padding gives the rest" layout `text_input` already uses, just
/// aimed at this crate's own floor rather than iced's unset default. The
/// caller still chains `.on_input(...)`, `.id(...)`, `.width(...)` — this
/// only fixes size, line height, padding and style, the four knobs this
/// crate has ever set by hand at each call site.
#[must_use]
pub fn styled_text_input<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    placeholder: &'a str,
    value: &'a str,
) -> iced::widget::TextInput<'a, Message> {
    use iced::widget::text_input;

    let line_height_px = tokens.typography.body.size * tokens.typography.body.line_height;

    text_input(placeholder, value)
        .padding([(BUTTON_HEIGHT - line_height_px) / 2.0, 12.0])
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens))
        .style({
            let t = tokens.clone();
            move |_theme, status| text_input_style(&t, status)
        })
}

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
    use iced::widget::{column, text};

    // RFC-073 handoff Amendment A3 §2: `BUTTON_HEIGHT` by padding, through
    // the one shared builder — see `styled_text_input`.
    let mut field = styled_text_input(tokens, placeholder, value)
        .on_input(on_change)
        .width(Length::Fill);

    if let Some(widget_id) = id {
        field = field.id(widget_id);
    }

    let mut group = column![
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        field,
    ]
    .spacing(8);

    if let Some(err) = error {
        // RFC-071 Amendment A3 D14: an error is read to understand and fix
        // a problem — `body`, not incidental metadata.
        group = group.push(
            text(err)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        );
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
    use iced::widget::{column, row, text};

    // RFC-073 handoff Amendment A3 §2: same shared builder as
    // `guided_field_with_id` above.
    let field = styled_text_input(tokens, placeholder, value)
        .on_input(on_change)
        .width(Length::Fill);

    let mut field_row = row![field].spacing(8).align_y(iced::Alignment::Center);
    if let Some(unit) = unit {
        // RFC-071 Amendment A3 D14: kept `body_small` — a terse unit tag
        // ("seconds") beside a value, the same kind of incidental qualifier
        // as a count inside metadata; the field's own label (already
        // `body`) carries what the value means.
        field_row = field_row.push(
            text(unit)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }

    let mut group = column![
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        field_row
    ]
    .spacing(8);

    if shows_error(error) {
        let danger = snora::design::style::color::to_iced_color(tokens.palette.danger_text);
        group = group.push(
            text(error.unwrap_or_default())
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
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
    use super::{shows_error, text_input_style};
    use iced::widget::text_input::Status;
    use snora::design::Tokens;

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

    fn all_four_themes() -> [(&'static str, Tokens); 4] {
        [
            ("light", crate::theme::KnotraTheme::light().tokens),
            ("dark", crate::theme::KnotraTheme::dark().tokens),
            ("high_contrast_light", Tokens::high_contrast_light()),
            ("high_contrast_dark", Tokens::high_contrast_dark()),
        ]
    }

    /// RFC-070 Amendment A2 item 1: a focused field draws the same ring
    /// every other control does — width `tokens.focus.ring_width` (2px
    /// normal, 3px HC), always wider than the unfocused/active 1px border.
    #[test]
    fn focused_border_width_is_the_rings_width_in_every_theme() {
        for (theme_name, tokens) in all_four_themes() {
            let active = text_input_style(&tokens, Status::Active);
            let focused = text_input_style(&tokens, Status::Focused { is_hovered: false });

            assert_eq!(
                focused.border.width, tokens.focus.ring_width,
                "{theme_name}: focused border width must equal the ring width"
            );
            assert!(
                focused.border.width > active.border.width,
                "{theme_name}: focused width {} must exceed the unfocused width {}",
                focused.border.width,
                active.border.width
            );
        }
    }

    /// RFC-070 Amendment A2 item 1: focus is never colour alone — the width
    /// change above already guarantees this (2px/3px vs. 1px), asserted
    /// here directly as its own requirement rather than left to follow
    /// from the width test by implication.
    #[test]
    fn focused_style_differs_from_unfocused_by_colour_or_width_in_every_theme() {
        for (theme_name, tokens) in all_four_themes() {
            let active = text_input_style(&tokens, Status::Active);
            let focused = text_input_style(&tokens, Status::Focused { is_hovered: false });

            assert!(
                focused.border.color != active.border.color
                    || focused.border.width != active.border.width,
                "{theme_name}: focused border must differ from unfocused by colour or width"
            );
        }
    }

    /// RFC-070 Amendment A2 item 2 (RFC-071 Amendment A1 D13, applied to
    /// fields): an unavailable field's value is `text_muted`, at full
    /// opacity — never the pre-A2 faded `text_primary` at alpha 0.5. The
    /// background is likewise untouched (same as `Active`), so neither the
    /// fill nor the text dims.
    #[test]
    fn disabled_value_is_text_muted_at_full_opacity_in_every_theme() {
        for (theme_name, tokens) in all_four_themes() {
            let active = text_input_style(&tokens, Status::Active);
            let disabled = text_input_style(&tokens, Status::Disabled);
            let expected_value =
                snora::design::style::color::to_iced_color(tokens.palette.text_muted);

            assert_eq!(
                disabled.value, expected_value,
                "{theme_name}: disabled value must be text_muted"
            );
            assert_eq!(
                disabled.value.a, 1.0,
                "{theme_name}: disabled value must be at full opacity"
            );
            assert_eq!(
                disabled.background, active.background,
                "{theme_name}: disabled background must not fade"
            );
        }
    }
}
