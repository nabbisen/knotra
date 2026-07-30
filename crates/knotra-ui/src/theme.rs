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
}
