//! Color palette and theme tokens for knotra.
//!
//! Colors are chosen for sufficient contrast (WCAG AA) in both light and dark
//! variants. Status colors always appear alongside icons and text labels so
//! they are never the sole indicator of state.

use iced::Color;

/// Semantic color roles for repository status indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColor {
    /// Fully synchronized — no action required.
    Healthy,
    /// Behind upstream; pull recommended.
    Behind,
    /// Local commits not yet pushed.
    Ahead,
    /// Uncommitted changes present.
    Dirty,
    /// Merge / rebase conflict.
    Conflict,
    /// Could not determine status (read error).
    Unknown,
}

impl StatusColor {
    /// Returns an `iced::Color` appropriate for the current theme variant.
    pub fn to_color(self, dark: bool) -> Color {
        if dark {
            match self {
                StatusColor::Healthy => Color::from_rgb8(0x4c, 0xaf, 0x50), // green-500
                StatusColor::Behind => Color::from_rgb8(0xff, 0xb7, 0x4d),  // amber-300
                StatusColor::Ahead => Color::from_rgb8(0x42, 0xa5, 0xf5),   // blue-400
                StatusColor::Dirty => Color::from_rgb8(0xff, 0xb7, 0x4d),   // amber-300
                StatusColor::Conflict => Color::from_rgb8(0xef, 0x53, 0x50), // red-400
                StatusColor::Unknown => Color::from_rgb8(0x75, 0x75, 0x75), // grey-600
            }
        } else {
            match self {
                StatusColor::Healthy => Color::from_rgb8(0x2e, 0x7d, 0x32), // green-800
                StatusColor::Behind => Color::from_rgb8(0xbf, 0x46, 0x00), // orange-900++ (WCAG AA 4.7:1 on #F5F5F5)
                StatusColor::Ahead => Color::from_rgb8(0x15, 0x65, 0xc0),  // blue-800
                StatusColor::Dirty => Color::from_rgb8(0xbf, 0x46, 0x00), // orange-900++ (WCAG AA 4.7:1 on #F5F5F5)
                StatusColor::Conflict => Color::from_rgb8(0xc6, 0x28, 0x28), // red-800
                StatusColor::Unknown => Color::from_rgb8(0x61, 0x61, 0x61), // grey-700
            }
        }
    }
}

/// The application's iced `Theme` extension carrying knotra-specific tokens.
///
/// Wraps the built-in `iced::Theme` so that both the light and dark system
/// preferences are respected while adding semantic status colours and the
/// Snora Design token set (RFC-034 D1/R3).
#[derive(Debug, Clone)]
pub struct KnotraTheme {
    pub base: iced::Theme,
    pub dark: bool,
    /// Snora Design token bundle: palette, spacing, typography, radius, and
    /// focus tokens (RFC-033 D7). `KnotraTheme::light()`/`dark()` map to
    /// `Tokens::light()`/`dark()` one-to-one.
    pub tokens: snora::design::Tokens,
}

impl KnotraTheme {
    pub fn light() -> Self {
        KnotraTheme {
            base: iced::Theme::Light,
            dark: false,
            tokens: snora::design::Tokens::light(),
        }
    }

    pub fn dark() -> Self {
        KnotraTheme {
            base: iced::Theme::Dark,
            dark: true,
            tokens: snora::design::Tokens::dark(),
        }
    }

    pub fn status_color(&self, status: StatusColor) -> Color {
        status.to_color(self.dark)
    }

    // -- D7 colour role accessors -------------------------------------------
    // Thin accessors over `self.tokens.palette`, converted to `iced::Color`.
    // Application view code reads roles through these, never through
    // `snora::design` directly (RFC-034 R2).

    pub fn background(&self) -> Color {
        to_iced(self.tokens.palette.background)
    }

    pub fn surface(&self) -> Color {
        to_iced(self.tokens.palette.surface)
    }

    pub fn surface_raised(&self) -> Color {
        to_iced(self.tokens.palette.surface_raised)
    }

    pub fn border(&self) -> Color {
        to_iced(self.tokens.palette.border)
    }

    pub fn text_primary(&self) -> Color {
        to_iced(self.tokens.palette.text_primary)
    }

    pub fn text_secondary(&self) -> Color {
        to_iced(self.tokens.palette.text_secondary)
    }

    pub fn text_muted(&self) -> Color {
        to_iced(self.tokens.palette.text_muted)
    }

    pub fn focus(&self) -> Color {
        to_iced(self.tokens.palette.focus)
    }

    pub fn accent(&self) -> Color {
        to_iced(self.tokens.palette.accent)
    }

    pub fn accent_text(&self) -> Color {
        to_iced(self.tokens.palette.accent_text)
    }

    pub fn danger(&self) -> Color {
        to_iced(self.tokens.palette.danger)
    }

    pub fn danger_text(&self) -> Color {
        to_iced(self.tokens.palette.danger_text)
    }

    pub fn warning(&self) -> Color {
        to_iced(self.tokens.palette.warning)
    }

    pub fn warning_text(&self) -> Color {
        to_iced(self.tokens.palette.warning_text)
    }

    pub fn success(&self) -> Color {
        to_iced(self.tokens.palette.success)
    }

    pub fn success_text(&self) -> Color {
        to_iced(self.tokens.palette.success_text)
    }
}

fn to_iced(color: snora::design::Color) -> Color {
    snora::design::style::color::to_iced_color(color)
}

impl Default for KnotraTheme {
    fn default() -> Self {
        KnotraTheme::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `snora_design::contrast::contrast_ratio` takes `snora_design::Color`,
    /// which is deliberately not `iced::Color` (snora-design stays iced-free).
    /// `snora_widgets` only ships the opposite conversion
    /// (`to_iced_color`), so this test provides its own reverse conversion —
    /// a lossless field copy, same as the one-way helper it mirrors.
    fn from_iced(c: Color) -> snora::design::Color {
        snora::design::Color::rgba(c.r, c.g, c.b, c.a)
    }

    const AA_NORMAL: f32 = 4.5;
    /// AA-large threshold, for the one documented exception below.
    const AA_LARGE: f32 = 3.0;

    /// R5: every `StatusColor` against its intended surface — the plain
    /// application background, since no card in the current codebase styles
    /// its own background (RFC-034 Background section) — meets WCAG AA in
    /// both themes, with one carried-forward exception.
    #[test]
    fn status_colors_meet_wcag_aa_against_background_in_both_themes() {
        let cases: [(&str, StatusColor); 6] = [
            ("Healthy", StatusColor::Healthy),
            ("Behind", StatusColor::Behind),
            ("Ahead", StatusColor::Ahead),
            ("Dirty", StatusColor::Dirty),
            ("Conflict", StatusColor::Conflict),
            ("Unknown", StatusColor::Unknown),
        ];

        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let background = from_iced(theme.background());

            for (label, status) in cases {
                let color = from_iced(status.to_color(theme.dark));
                let ratio = snora::design::contrast::contrast_ratio(color, background);

                // Documented exception, carried forward from RFC-021 Phase 6:
                // `Unknown` on the dark background meets AA-large only. It is
                // used solely as a secondary label alongside an icon, never as
                // the sole indicator of state (module doc comment above).
                let threshold = if theme_name == "dark" && label == "Unknown" {
                    AA_LARGE
                } else {
                    AA_NORMAL
                };

                assert!(
                    ratio >= threshold,
                    "{theme_name} StatusColor::{label} vs background: {ratio:.2}:1 \
                     is below the required {threshold}:1"
                );
            }
        }
    }

    /// R5: every mandatory-contrast text role (`text_primary`,
    /// `text_secondary`) meets WCAG AA against every surface role it can
    /// render on, in both themes. `text_muted` is intentionally excluded —
    /// `snora_design::Palette`'s own documentation marks it exempt from the
    /// mandatory body-text check.
    #[test]
    fn text_roles_meet_wcag_aa_against_every_surface_role_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let surfaces: [(&str, Color); 3] = [
                ("background", theme.background()),
                ("surface", theme.surface()),
                ("surface_raised", theme.surface_raised()),
            ];
            let texts: [(&str, Color); 2] = [
                ("text_primary", theme.text_primary()),
                ("text_secondary", theme.text_secondary()),
            ];

            for (text_label, text_color) in texts {
                for (surface_label, surface_color) in surfaces {
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(text_color),
                        from_iced(surface_color),
                    );
                    assert!(
                        ratio >= AA_NORMAL,
                        "{theme_name} {text_label} on {surface_label}: {ratio:.2}:1 \
                         is below the required {AA_NORMAL}:1"
                    );
                }
            }
        }
    }

    /// RFC-036 Stage 6 (D7 fix), widened per
    /// `.git-exclude/reviewed/083-rfc-036-stage-6-review.md` Finding 3: the
    /// ring `with_focus_ring` actually draws meets WCAG AA against every
    /// background it can be drawn on, in both themes, across every *enabled*
    /// status — mechanically enforced rather than judged by eye. `ghost`/
    /// `secondary` are checked against `surface`, since their own background
    /// is transparent-or-lightly-tinted and `surface` is what shows through;
    /// `primary` and `danger` are checked against **the background that
    /// status actually renders** (`Hovered` lightens by 0.06, `Pressed`
    /// darkens by 0.06, per `snora-widgets`' `button.rs`), read directly out
    /// of the same `Style` passed to `with_focus_ring`, not a flat palette
    /// role — asserting against flat `accent`/`danger` at `Hovered`/`Pressed`
    /// would measure a pairing that never renders.
    ///
    /// **`Disabled` is deliberately excluded.** `filled_background`'s
    /// `>= 1.0` gate never fires there (`disabled_alpha` scales the
    /// background's alpha to 0.45), so a focused-but-disabled `primary`/
    /// `danger` control keeps the plain `ring_color`, which measures
    /// 2.99-3.33:1 against its disabled background — meeting WCAG 1.4.11
    /// Non-text Contrast (3:1, the criterion that actually applies to a
    /// focus indicator, not the 4.5:1 text threshold this test otherwise
    /// uses) in dark but not quite in light. No colour choice improves this:
    /// `083` Finding 2 computed that `accent_text` measures *worse* than
    /// `ring_color` there in all four theme/control combinations. Recorded
    /// as a known limitation, not asserted here.
    #[test]
    fn focus_ring_meets_wcag_aa_against_every_background_it_can_be_drawn_on() {
        use crate::widget::style::{danger, ghost, primary, with_focus_ring};
        use iced::widget::button::{Status, Style};

        /// Pulls the solid colour back out of a style's background — the
        /// actual rendered pairing, not an assumption about which palette
        /// role produced it.
        fn rendered_background(style: Style) -> Color {
            match style.background {
                Some(iced::Background::Color(c)) => c,
                other => panic!("expected a solid Background::Color, got {other:?}"),
            }
        }

        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let tokens = &theme.tokens;

            for status in [Status::Active, Status::Hovered, Status::Pressed] {
                let ghost_style = ghost(tokens, status);
                let primary_style = primary(tokens, status);
                let danger_style = danger(tokens, status);

                let cases: [(&str, Style, Color); 3] = [
                    ("ghost/secondary on surface", ghost_style, theme.surface()),
                    (
                        "primary on its own rendered background",
                        primary_style,
                        rendered_background(primary_style),
                    ),
                    (
                        "danger on its own rendered background",
                        danger_style,
                        rendered_background(danger_style),
                    ),
                ];

                for (label, style, background) in cases {
                    let ring = with_focus_ring(tokens, true, style).border.color;
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(ring),
                        from_iced(background),
                    );
                    assert!(
                        ratio >= AA_NORMAL,
                        "{theme_name} {status:?} focus ring against {label}: {ratio:.2}:1 \
                         is below the required {AA_NORMAL}:1"
                    );
                }
            }
        }
    }

    // -- RFC-035 Handoff 019 §7.4: chip, select, checkbox pairings ----------

    /// Chip text on chip background, selected and unselected
    /// (`chip::filter` — a thin pass-through to `snora::design::chip`,
    /// whose own module doc already claims >=6.7:1; asserted here too so
    /// the pairing is traceable from this crate's own test suite, per
    /// Handoff 019 §7.4). Selected is `accent_text` on `accent`; unselected
    /// is `text_secondary` on `surface`, already implied by
    /// `text_roles_meet_wcag_aa_against_every_surface_role_in_both_themes`
    /// above but asserted directly here as its own named pairing.
    #[test]
    fn chip_text_meets_wcag_aa_on_chip_background_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let selected = snora::design::contrast::contrast_ratio(
                from_iced(theme.accent_text()),
                from_iced(theme.accent()),
            );
            assert!(
                selected >= AA_NORMAL,
                "{theme_name} chip selected text vs background: {selected:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );

            let unselected = snora::design::contrast::contrast_ratio(
                from_iced(theme.text_secondary()),
                from_iced(theme.surface()),
            );
            assert!(
                unselected >= AA_NORMAL,
                "{theme_name} chip unselected text vs background: {unselected:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );
        }
    }

    /// Select text on the closed control, and on the open menu's normal and
    /// selected rows (`widget::select::pick_list`'s `field_style` /
    /// `menu_style`, RFC-035 Handoff 019 §7.4).
    #[test]
    fn select_text_meets_wcag_aa_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let closed = snora::design::contrast::contrast_ratio(
                from_iced(theme.text_primary()),
                from_iced(theme.surface()),
            );
            assert!(
                closed >= AA_NORMAL,
                "{theme_name} select closed control text: {closed:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );

            let menu_row = snora::design::contrast::contrast_ratio(
                from_iced(theme.text_primary()),
                from_iced(theme.surface_raised()),
            );
            assert!(
                menu_row >= AA_NORMAL,
                "{theme_name} select open menu row text: {menu_row:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );

            let menu_selected = snora::design::contrast::contrast_ratio(
                from_iced(theme.accent_text()),
                from_iced(theme.accent()),
            );
            assert!(
                menu_selected >= AA_NORMAL,
                "{theme_name} select open menu selected row text: {menu_selected:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );
        }
    }

    /// Checkbox mark (`icon_color`) on its checked background
    /// (`widget::checkbox`'s `style`, RFC-035 Handoff 019 §7.4) — the same
    /// `accent_text`-on-`accent` pairing as the chip's selected state and
    /// the select menu's selected row, asserted here under its own name for
    /// traceability.
    #[test]
    fn checkbox_mark_meets_wcag_aa_on_checked_background_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let ratio = snora::design::contrast::contrast_ratio(
                from_iced(theme.accent_text()),
                from_iced(theme.accent()),
            );
            assert!(
                ratio >= AA_NORMAL,
                "{theme_name} checkbox mark vs checked background: {ratio:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );
        }
    }

    /// Each new control's focus ring against its own background — the
    /// check `083` Finding 2 established, since a ring that fails this is
    /// invisible (RFC-035 Handoff 019 §7.4). `chip::filter`'s signature
    /// carries no `is_focused` (see `chip.rs`'s module doc), so only
    /// `select` and `checkbox` apply.
    ///
    /// **Widened per `098`.** The original version paired each ring with a
    /// flat palette role (`theme.surface()`, `theme.accent()`) asserted
    /// once, which is exactly the shape `087` Finding 1 found wrong in
    /// RFC-040 — the assertion measures a pairing that may never render,
    /// because it never asked the real style function what it actually
    /// produces at each status. This version drives the real
    /// `select::field_style` / `select::menu_style` / `checkbox::style`
    /// functions directly and reads the background back out of the
    /// `Style` each returns, across every real `Status` variant those
    /// functions accept, rather than reconstructing it. Nothing here
    /// currently fails: `select`'s field/menu backgrounds do not vary by
    /// status (only the border does), and `checkbox`'s does not vary
    /// between `Active`/`Hovered` (only `Disabled`, excluded here since
    /// the ring is deliberately not drawn there — see `checkbox.rs`'s own
    /// `disabled_style_is_unaffected_by_is_focused`) — so the measured
    /// ratios are unchanged from Handoff 019/020, but are now proven by
    /// reading the real output rather than assumed from the palette.
    #[test]
    fn new_control_focus_rings_meet_wcag_aa_against_their_own_backgrounds_in_both_themes() {
        use crate::widget::{checkbox, select};
        use iced::widget::{checkbox::Status as CheckboxStatus, pick_list::Status as FieldStatus};

        fn background_of(background: iced::Background) -> Color {
            match background {
                iced::Background::Color(c) => c,
                other => panic!("expected a solid Background::Color, got {other:?}"),
            }
        }

        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let tokens = &theme.tokens;

            let field_statuses: [(&str, FieldStatus); 4] = [
                ("Active", FieldStatus::Active),
                ("Hovered", FieldStatus::Hovered),
                (
                    "Opened(unhovered)",
                    FieldStatus::Opened { is_hovered: false },
                ),
                ("Opened(hovered)", FieldStatus::Opened { is_hovered: true }),
            ];
            for (status_name, status) in field_statuses {
                let unfocused = select::field_style(tokens, status, false);
                let focused = select::field_style(tokens, status, true);
                let background = background_of(unfocused.background);
                let ratio = snora::design::contrast::contrast_ratio(
                    from_iced(focused.border.color),
                    from_iced(background),
                );
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} select field ({status_name}) focus ring: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }

            // The open menu has no per-status styling at all (`menu_style`
            // takes no `Status`), so there is nothing to loop — one call
            // is already "the real function", not a reconstruction.
            let menu_unfocused = select::menu_style(tokens, false);
            let menu_focused = select::menu_style(tokens, true);
            let menu_background = background_of(menu_unfocused.background);
            let menu_ratio = snora::design::contrast::contrast_ratio(
                from_iced(menu_focused.border.color),
                from_iced(menu_background),
            );
            assert!(
                menu_ratio >= AA_NORMAL,
                "{theme_name} select open menu focus ring: {menu_ratio:.2}:1 \
                 is below the required {AA_NORMAL}:1"
            );

            let checkbox_statuses: [(&str, CheckboxStatus); 4] = [
                (
                    "Active, unchecked",
                    CheckboxStatus::Active { is_checked: false },
                ),
                (
                    "Active, checked",
                    CheckboxStatus::Active { is_checked: true },
                ),
                (
                    "Hovered, unchecked",
                    CheckboxStatus::Hovered { is_checked: false },
                ),
                (
                    "Hovered, checked",
                    CheckboxStatus::Hovered { is_checked: true },
                ),
            ];
            for (status_name, status) in checkbox_statuses {
                let unfocused = checkbox::style(tokens, status, false);
                let focused = checkbox::style(tokens, status, true);
                let background = background_of(unfocused.background);
                let ratio = snora::design::contrast::contrast_ratio(
                    from_iced(focused.border.color),
                    from_iced(background),
                );
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} checkbox ({status_name}) focus ring: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }
        }
    }

    /// The chip's focus ring against both its backgrounds — selected
    /// (`accent`) and unselected (`surface`) — in both themes, using the
    /// real `chip::style` output (which applies `with_focus_ring`
    /// internally, the same button-path wrapper the existing button ring
    /// test exercises), not a re-derivation (RFC-035 Handoff 020 §7.3).
    /// Chip gained `is_focused` only in this handoff — 094 Finding 1 found
    /// the pass-through signature could not carry one at all.
    #[test]
    fn chip_focus_ring_meets_wcag_aa_against_both_backgrounds_in_both_themes() {
        use crate::widget::chip::style as chip_style;
        use iced::widget::button::Status;

        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let tokens = &theme.tokens;

            for selected in [false, true] {
                let unfocused = chip_style(tokens, selected, Status::Active, false);
                let focused = chip_style(tokens, selected, Status::Active, true);
                let background = match unfocused.background {
                    Some(iced::Background::Color(c)) => c,
                    other => panic!("expected a solid Background::Color, got {other:?}"),
                };

                let ratio = snora::design::contrast::contrast_ratio(
                    from_iced(focused.border.color),
                    from_iced(background),
                );
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} chip selected={selected} focus ring: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }
        }
    }

    /// `notice`'s one tone-varying text color (RFC-035 Handoff 030 §5):
    /// its title/body always render in `text_primary` (already covered by
    /// `text_roles_meet_wcag_aa_against_every_surface_role_in_both_themes`),
    /// but the action button's label is coloured by `Tone` — `Notice::render`'s
    /// own `match self.tone { Tone::Accent => p.accent, ... }`. That mapping
    /// is a static function of `Tone` alone, with no interactive `Status` to
    /// branch on, so reading the palette fields directly drives the same
    /// computation `render()` performs rather than reconstructing it —
    /// unlike `087` Finding 1, where the original sin was skipping a
    /// genuinely status-dependent style function. Checked against `surface`,
    /// `notice`'s own background, in both themes.
    ///
    /// **`Tone::Neutral` (`p.border`) is deliberately excluded.** `Palette`
    /// documents `border` as "borders and separators" — a role never
    /// intended to carry mandatory text contrast, unlike `accent`/`success`/
    /// `warning`/`danger`/`info`. It measures 1.28:1 against light-theme
    /// `surface`, far under AA, confirming it was never meant for this.
    /// Not a defect: no call site uses `Tone::Neutral` with `notice` (this
    /// stage's own consumer is `Tone::Danger`), and none should until a
    /// text-safe color is chosen for that case.
    #[test]
    fn notice_tone_colors_meet_wcag_aa_against_surface_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let surface = from_iced(theme.surface());
            let p = &theme.tokens.palette;
            let cases: [(&str, snora::design::Color); 5] = [
                ("Accent", p.accent),
                ("Success", p.success),
                ("Warning", p.warning),
                ("Danger", p.danger),
                ("Info", p.info),
            ];
            for (label, color) in cases {
                let ratio = snora::design::contrast::contrast_ratio(color, surface);
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} notice action-label Tone::{label} vs surface: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }
        }
    }
}
