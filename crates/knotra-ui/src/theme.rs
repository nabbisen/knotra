//! Color palette and theme tokens for knotra.
//!
//! Colors are chosen for sufficient contrast (WCAG AA) in both light and dark
//! variants. Status colors always appear alongside icons and text labels so
//! they are never the sole indicator of state.

use iced::Color;

/// The four themes knotra offers (RFC-070 D1). Saved form is snake_case
/// (`"light"` / `"dark"` / `"high_contrast_light"` / `"high_contrast_dark"`),
/// distinct from [`crate::i18n::Locale`]'s own PascalCase saved form — D1
/// names this exact saved form, and `DashboardGrouping`/`DashboardSort`
/// (`knotra-app`'s `config.rs`) already use the same `rename_all` for the
/// same reason: a value a user never sees, read back from their own saved
/// file.
///
/// Task 104: `Light` is the default, not `Dark` — never a recorded design
/// decision before this, just the shape the very first commit happened to
/// take. For many older eyes, dark text on a light background is easier to
/// read: more light reaching the eye, and less blooming of light text
/// against a dark field for anyone with astigmatism. A user who prefers
/// dark still chooses it in Settings, and `load_config`'s own
/// `dark_theme = true` compatibility mapping (RFC-070 D1) means anyone who
/// already chose it — before or after this change — keeps it; only a
/// genuinely unconfigured, brand-new installation is affected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    #[default]
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
}

/// The application's iced `Theme` extension carrying knotra-specific tokens.
///
/// Wraps the built-in `iced::Theme` so that both the light and dark system
/// preferences are respected while adding semantic status colours and the
/// Snora Design token set (RFC-034 D1/R3).
#[derive(Debug, Clone)]
pub struct KnotraTheme {
    pub base: iced::Theme,
    /// Snora Design token bundle: palette, spacing, typography, radius, and
    /// focus tokens (RFC-033 D7). `KnotraTheme::light()`/`dark()`/
    /// `high_contrast_light()`/`high_contrast_dark()` map to
    /// `Tokens::light()`/`dark()`/`high_contrast_light()`/
    /// `high_contrast_dark()` one-to-one.
    pub tokens: snora::design::Tokens,
}

impl KnotraTheme {
    /// RFC-070 D2: the only constructor the app itself calls — every other
    /// constructor below exists for this to dispatch to (and for this
    /// file's own tests, which legitimately need one preset at a time).
    /// `R2` pins this as a standing rule with a guard, not only a style
    /// preference: `git grep` finds no `KnotraTheme::light()`/`dark()`/
    /// `high_contrast_light()`/`high_contrast_dark()` call anywhere under
    /// `crates/knotra-app`.
    #[must_use]
    pub fn from_choice(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::Light => Self::light(),
            ThemeChoice::Dark => Self::dark(),
            ThemeChoice::HighContrastLight => Self::high_contrast_light(),
            ThemeChoice::HighContrastDark => Self::high_contrast_dark(),
        }
    }

    /// Task 107 Commit 3 (the owner's decision, Review 272 §5.1, accepted
    /// 2026-11-09): built from `Tokens::light_aaa()`, not `Tokens::light()`
    /// — every declared text pair reaches 7:1, not 4.5:1, with the same
    /// backgrounds and overall look (snora's own module doc for
    /// `light_aaa`). `ThemeChoice::Light` still names this constructor;
    /// nothing elsewhere needed to change to pick up the stricter preset.
    pub fn light() -> Self {
        let mut tokens = snora::design::Tokens::light_aaa();
        // Task 107 Commit 2: every snora-drawn control's own pointer
        // target is at least `BUTTON_HEIGHT` — the same floor knotra
        // already holds its own widgets to (RFC-073 D8) — from this one
        // value, in every theme.
        tokens.targets.min = crate::widget::BUTTON_HEIGHT;
        KnotraTheme {
            // RFC-057 D1/R1: `base` is what `main.rs` hands to iced as the
            // application theme — every widget with no explicit `.style()`
            // (five confirmed `scrollable` sites, more unmeasured — R2)
            // renders from it. Built from the same `tokens` stored below
            // rather than a second `Tokens::light_aaa()` call, so the two
            // fields cannot drift from each other.
            base: snora::design::theme(&tokens),
            tokens,
        }
    }

    /// Task 107 Commit 3: `Tokens::dark_aaa()`, the `light_aaa` decision's
    /// Dark counterpart — see [`Self::light`].
    pub fn dark() -> Self {
        let mut tokens = snora::design::Tokens::dark_aaa();
        tokens.targets.min = crate::widget::BUTTON_HEIGHT;
        KnotraTheme {
            base: snora::design::theme(&tokens),
            tokens,
        }
    }

    /// RFC-070 D1/D4: snora's own high-contrast light preset, held to the
    /// RFC's own stricter floors (see this file's contrast suite test) —
    /// not a knotra-authored palette.
    pub fn high_contrast_light() -> Self {
        let mut tokens = snora::design::Tokens::high_contrast_light();
        tokens.targets.min = crate::widget::BUTTON_HEIGHT;
        KnotraTheme {
            base: snora::design::theme(&tokens),
            tokens,
        }
    }

    /// RFC-070 D1/D4: snora's own high-contrast dark preset, same reasoning
    /// as [`Self::high_contrast_light`].
    pub fn high_contrast_dark() -> Self {
        let mut tokens = snora::design::Tokens::high_contrast_dark();
        tokens.targets.min = crate::widget::BUTTON_HEIGHT;
        KnotraTheme {
            base: snora::design::theme(&tokens),
            tokens,
        }
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

    /// Task 104: pinned by name, not merely "whatever `#[default]` happens
    /// to mark" — the whole point of this task is that the default moved,
    /// so the test must name the value it moved *to*.
    #[test]
    fn theme_choice_default_is_light() {
        assert_eq!(ThemeChoice::default(), ThemeChoice::Light);
    }

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
    /// Task 107 Commit 3: Light and Dark are now built from snora's
    /// `light_aaa`/`dark_aaa` presets (the owner's decision, Review 272
    /// §5.1) — every text pair below that used to assert `AA_NORMAL`
    /// (4.5:1) now asserts this, 7:1, the floor those presets actually
    /// deliver.
    const AAA_NORMAL: f32 = 7.0;

    /// R5: every mandatory-contrast text role (`text_primary`,
    /// `text_secondary`, `text_muted`) meets WCAG AA against every surface
    /// role it can render on, in both themes.
    ///
    /// **`text_muted` was excluded until RFC-056 Stage 1** on the strength
    /// of `snora_design::Palette`'s own documentation, which marked it
    /// exempt from the mandatory body-text check. snora 0.34.0 withdrew
    /// that exemption: WCAG grants no such exemption in the first place —
    /// its actual exemptions are incidental, decorative or invisible text,
    /// logotypes, and large text — and knotra's own use of `text_muted`
    /// (`select.rs`'s `placeholder_color`) is none of those; it is text a
    /// user reads to know what a field expects. Included here now for the
    /// same reason `text_primary`/`text_secondary` always were.
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
            let texts: [(&str, Color); 3] = [
                ("text_primary", theme.text_primary()),
                ("text_secondary", theme.text_secondary()),
                ("text_muted", theme.text_muted()),
            ];

            for (text_label, text_color) in texts {
                for (surface_label, surface_color) in surfaces {
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(text_color),
                        from_iced(surface_color),
                    );
                    assert!(
                        ratio >= AAA_NORMAL,
                        "{theme_name} {text_label} on {surface_label}: {ratio:.2}:1 \
                         is below the required {AAA_NORMAL}:1"
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
                selected >= AAA_NORMAL,
                "{theme_name} chip selected text vs background: {selected:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
            );

            let unselected = snora::design::contrast::contrast_ratio(
                from_iced(theme.text_secondary()),
                from_iced(theme.surface()),
            );
            assert!(
                unselected >= AAA_NORMAL,
                "{theme_name} chip unselected text vs background: {unselected:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
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
                closed >= AAA_NORMAL,
                "{theme_name} select closed control text: {closed:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
            );

            let menu_row = snora::design::contrast::contrast_ratio(
                from_iced(theme.text_primary()),
                from_iced(theme.surface_raised()),
            );
            assert!(
                menu_row >= AAA_NORMAL,
                "{theme_name} select open menu row text: {menu_row:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
            );

            let menu_selected = snora::design::contrast::contrast_ratio(
                from_iced(theme.accent_text()),
                from_iced(theme.accent()),
            );
            assert!(
                menu_selected >= AAA_NORMAL,
                "{theme_name} select open menu selected row text: {menu_selected:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
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
                ratio >= AAA_NORMAL,
                "{theme_name} checkbox mark vs checked background: {ratio:.2}:1 \
                 is below the required {AAA_NORMAL}:1"
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
                let unfocused = chip_style(tokens, selected, 1, Status::Active, false);
                let focused = chip_style(tokens, selected, 1, Status::Active, true);
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
    /// **`Tone::Neutral` (`p.border`) is excluded from the loop below,
    /// deliberately — RFC-056 Stage 4 (A5/R13) removed the runtime check
    /// this comment used to describe.** `Palette` documents `border` as
    /// "borders and separators," a role never intended to carry mandatory
    /// text contrast, unlike `accent`/`success`/`warning`/`danger`/`info` —
    /// and at RFC-056 Stage 1 (snora 0.38) it measured 3.12:1 (light) /
    /// 3.50:1 (dark) against `surface`, under AA in both, up from
    /// 1.28:1/1.32:1 before snora 0.34.0's border-contrast repair. That
    /// history is why `notice.rs`'s `NoticeTone` (the wrapper's own public
    /// parameter type, narrower than `snora::design::Tone`) excludes
    /// `Neutral` at the type level — `Tone::Neutral` itself is unchanged
    /// and still reachable through `snora` directly, just not through
    /// `notice`.
    ///
    /// **The exclusion is no longer re-verified by a live contrast
    /// assertion here.** It previously asserted `neutral_ratio < AA_NORMAL`
    /// — a check that snora 0.38.1's `api-governance.md` states plainly a
    /// consumer must not write: every contrast threshold snora ships is a
    /// **floor**, no maximum is guaranteed, and the only permitted value
    /// change *raises* a failing ratio (border already moved once, 0.34.0).
    /// A future repair past 4.5 would have failed our assertion for
    /// improving a colour we bet on staying bad — snora's own letter cites
    /// this exact test by name as the instance that prompted the rule. The
    /// exclusion's justification is the type-level decision itself
    /// (`NoticeTone` has no `Neutral` variant to construct), not a runtime
    /// number that is snora's to move; the ratio above is a historical
    /// record of *why* the decision was made, not a bound this suite still
    /// enforces.
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

    /// RFC-056 Stage 4 (A4/R12): `border`'s *boundary* use — the reason
    /// snora raised it in 0.34.0 (WCAG SC 1.4.11, a 3:1 floor for a
    /// non-text visual boundary) — had no assertion of its own; only its
    /// *text* use (`notice_tone_colors_meet_wcag_aa_against_surface_in_
    /// both_themes`, above, which **previously** asserted a `< AA_NORMAL`
    /// ceiling for an unrelated reason — removed the same stage, R13 —
    /// and would not have caught a boundary regression even while it
    /// stood). A future `border` regression would pass every existing gate.
    ///
    /// **RFC-058 D2/R1 widened this from one pair per preset to all
    /// three** (`surface`, `surface_raised`, `background`) — six
    /// assertions, in place, not split into six tests (A1: either shape
    /// satisfies D2, this one keeps the file's established
    /// "loop over every surface" pattern, matching
    /// `text_roles_meet_wcag_aa_against_every_surface_role_in_both_themes`
    /// above). The original single-pair-per-preset version tracked only
    /// the *tightest* pairing per preset (`light` vs `surface`, `dark` vs
    /// `surface_raised`) and reasoned that the tightest pair is the one a
    /// regression hits first — true, but it left `border` vs `background`
    /// asserted in **neither** preset. `background` is the resolve
    /// `Sheet`'s own fill (its content sits on it, `view.rs`'s
    /// `ActiveModal::Resolve` arm — not cited by line number, which drifts
    /// on every edit above it), and is exactly the pair a silent
    /// regression there would need to be caught on (RFC-058 Problem: the
    /// `Sheet` itself has no style hook and no contrast test of its own —
    /// see that arm's comment and `overlay.rs::surface`'s for the two
    /// removable choices this assertion is the safety net under). All six
    /// values (RFC-058 A2, re-confirmed here):
    /// `surface` 3.1207/3.5047, `surface_raised` 3.3808/3.1653,
    /// `background` 3.3808/3.8079 (light/dark) — every one already clears
    /// 3.0 today; this locks in a property that already holds, not a fix.
    ///
    /// **Asserts `>= AA_LARGE` (3.0), not a tighter figure**, against
    /// **every** surface `border` can neighbour, not only the tightest
    /// one. Per RFC-056 A5/RFC-058 D4, every snora contrast threshold is a
    /// floor with no guaranteed ceiling, and a repair only has to clear
    /// the criterion it was judged against — asserting a number closer to
    /// any of the current values would be asserting a margin snora has
    /// not promised to hold.
    ///
    /// **Task 117 widened this from Light/Dark to all four presets** — the
    /// detail panel's new edge (`palette.border`, against both its own
    /// `surface` fill and the page's `background`) needed the same floor
    /// on `high_contrast_light`/`high_contrast_dark` too, which this test
    /// did not reach before. Measured, both new presets, all three
    /// surfaces: `surface` 21.0000/21.0000, `surface_raised`
    /// 21.0000/19.7981, `background` 21.0000/21.0000
    /// (`high_contrast_light`/`high_contrast_dark`) — every one already
    /// clears 3.0 by a wide margin, as the preset's own name promises.
    #[test]
    fn border_meets_the_wcag_1_4_11_boundary_floor_against_every_surface_in_every_preset() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
            ("high_contrast_light", KnotraTheme::high_contrast_light()),
            ("high_contrast_dark", KnotraTheme::high_contrast_dark()),
        ] {
            let p = &theme.tokens.palette;
            let surfaces: [(&str, snora::design::Color); 3] = [
                ("surface", p.surface),
                ("surface_raised", p.surface_raised),
                ("background", p.background),
            ];
            for (surface_label, surface) in surfaces {
                let ratio = snora::design::contrast::contrast_ratio(p.border, surface);
                assert!(
                    ratio >= AA_LARGE,
                    "{theme_name} border vs {surface_label}: {ratio:.4}:1 \
                     is below the required {AA_LARGE}:1 non-text boundary floor (SC 1.4.11)"
                );
            }
        }
    }

    /// RFC-057 D3/R3: before this test, nothing in this file ever read
    /// `.base` — it appeared only in the two constructors (confirmed by
    /// grep before this test was written), so a widget rendering unstyled
    /// (R2: five confirmed `scrollable` sites, every `text_input` in the
    /// tree, both `tooltip`'s own popup chrome) could fail contrast and
    /// pass every gate this suite has. `base.extended_palette()` is the
    /// actual `Extended` struct iced's stock widgets and snora's `Sheet`
    /// read (`iced_core-0.14.0/src/theme/palette.rs`) — every `Pair` it
    /// exposes is asserted here: all eight `background` tiers (what an
    /// unstyled `scrollable`/`text_input`/`tooltip` draws from) and the
    /// three tiers each of `primary`/`secondary`/`success`/`warning`/
    /// `danger` (what an unstyled stock button/status widget would, none
    /// currently exists, but the assertion does not depend on one existing
    /// to be worth having).
    #[test]
    fn base_extended_palette_pairs_meet_wcag_aa_in_both_themes() {
        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
        ] {
            let ext = theme.base.extended_palette();
            let cases: [(&str, iced::theme::palette::Pair); 23] = [
                ("background.base", ext.background.base),
                ("background.weakest", ext.background.weakest),
                ("background.weaker", ext.background.weaker),
                ("background.weak", ext.background.weak),
                ("background.neutral", ext.background.neutral),
                ("background.strong", ext.background.strong),
                ("background.stronger", ext.background.stronger),
                ("background.strongest", ext.background.strongest),
                ("primary.base", ext.primary.base),
                ("primary.weak", ext.primary.weak),
                ("primary.strong", ext.primary.strong),
                ("secondary.base", ext.secondary.base),
                ("secondary.weak", ext.secondary.weak),
                ("secondary.strong", ext.secondary.strong),
                ("success.base", ext.success.base),
                ("success.weak", ext.success.weak),
                ("success.strong", ext.success.strong),
                ("warning.base", ext.warning.base),
                ("warning.weak", ext.warning.weak),
                ("warning.strong", ext.warning.strong),
                ("danger.base", ext.danger.base),
                ("danger.weak", ext.danger.weak),
                ("danger.strong", ext.danger.strong),
            ];
            for (label, pair) in cases {
                let ratio = snora::design::contrast::contrast_ratio(
                    from_iced(pair.text),
                    from_iced(pair.color),
                );
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} base.extended_palette().{label}: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }
        }
    }

    /// RFC-071 D3: every status badge's symbol colour (`*_text`) against its
    /// own badge colour, in every preset — the pairing `status_badge` draws
    /// wherever it is placed (the dashboard, History, and — Task 117 — the
    /// detail panel's own recent-operations list), independent of whatever
    /// surface sits behind the badge itself.
    ///
    /// **Widened from Light/Dark to all four presets (Task 117)**, the
    /// detail panel's new background being the occasion — though this
    /// pairing never reads the page's own surface at all, so nothing about
    /// the panel's own fill could have broken it; the floor stays a flat
    /// `AA_NORMAL`, as before, not `text_contrast.ratio()`, since D4 never
    /// raised this one. Measured, both new presets: Success 8.3799/13.4062,
    /// Warning 8.4168/14.8507, Danger 8.8458/10.3409, Info 10.2454/11.7535
    /// (`high_contrast_light`/`high_contrast_dark`) — every one already
    /// clears 4.5 by a wide margin.
    #[test]
    fn status_badge_symbol_meets_wcag_aa_on_its_own_tone_in_every_preset() {
        use crate::widget::Tone;

        for (theme_name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
            ("high_contrast_light", KnotraTheme::high_contrast_light()),
            ("high_contrast_dark", KnotraTheme::high_contrast_dark()),
        ] {
            let cases = [
                ("Success", Tone::Success),
                ("Warning", Tone::Warning),
                ("Danger", Tone::Danger),
                ("Info", Tone::Info),
            ];
            for (label, tone) in cases {
                let (bg, fg) = tone.colors(&theme.tokens);
                let ratio = snora::design::contrast::contrast_ratio(from_iced(fg), from_iced(bg));
                assert!(
                    ratio >= AA_NORMAL,
                    "{theme_name} Tone::{label} symbol vs badge: {ratio:.2}:1 \
                     is below the required {AA_NORMAL}:1"
                );
            }
        }
    }

    /// RFC-071 Amendment A1 D13 item 3, extended by RFC-070 D4: **the**
    /// contrast suite — one test covering every colour knotra draws text or
    /// symbols with, against every surface it can appear on, at the floor
    /// its own category of use carries — now across all four themes, the
    /// two high-contrast presets held to D4's stricter floors. Earlier
    /// tests in this file each check one role against one or a few
    /// surfaces, for their own reason; this one exists so a reader (and a
    /// reviewer) can find every floor in one place, and so a role that
    /// falls short stops here, with its own name, rather than passing
    /// silently because some narrower pre-existing test happened not to
    /// cover that particular pairing.
    ///
    /// **A shortfall here is never adjusted silently.** `snora`'s palette is
    /// an upstream dependency; a role that comes in short is an upstream
    /// matter (D13 item 3 / RFC-070 D4), not a token this crate is allowed
    /// to override — RFC-070's own "out of scope" names this explicitly: a
    /// preset short of a floor goes to the snora team, never into a knotra
    /// override.
    #[test]
    fn the_contrast_suite_covers_every_colour_knotra_draws_text_or_symbols_with() {
        use crate::widget::Tone;

        /// 1.4.6-strength floor for the text a user reads to understand or
        /// act: names, statuses, messages, headings. RFC-070 D4: the same
        /// 7:1 in every theme — content text was never the floor D4 raises.
        const CONTENT_TEXT: f32 = 7.0;

        // RFC-070 D4 raised the two floors below for the high-contrast
        // themes alone; Task 107 Commit 3 raises `control_label_floor` for
        // Light and Dark too, now that both are built from snora's
        // `light_aaa`/`dark_aaa` presets, which deliver the same 7:1
        // `control_label_floor` already held every button label and
        // placeholder to in high-contrast. `non_text_floor` is untouched
        // for Light/Dark — borders, focus rings and badges are non-text
        // (SC 1.4.11), a criterion the AAA presets do not raise and this
        // commit's own task does not ask to.
        for (theme_name, theme, control_label_floor, non_text_floor) in [
            ("light", KnotraTheme::light(), CONTENT_TEXT, AA_LARGE),
            ("dark", KnotraTheme::dark(), CONTENT_TEXT, AA_LARGE),
            (
                "high_contrast_light",
                KnotraTheme::high_contrast_light(),
                CONTENT_TEXT,
                AA_NORMAL,
            ),
            (
                "high_contrast_dark",
                KnotraTheme::high_contrast_dark(),
                CONTENT_TEXT,
                AA_NORMAL,
            ),
        ] {
            let p = &theme.tokens.palette;
            let surfaces: [(&str, snora::design::Color); 3] = [
                ("background", p.background),
                ("surface", p.surface),
                ("surface_raised", p.surface_raised),
            ];

            // Content text (7:1 in every theme): text_primary, text_secondary
            // — never text_muted, which this RFC scopes to placeholders and
            // unavailable-control labels only (items 2/3 below), not to
            // anything a user reads to understand or act.
            for (text_label, text_color) in [
                ("text_primary", p.text_primary),
                ("text_secondary", p.text_secondary),
            ] {
                for (surface_label, surface_color) in surfaces {
                    let ratio = snora::design::contrast::contrast_ratio(text_color, surface_color);
                    assert!(
                        ratio >= CONTENT_TEXT,
                        "{theme_name} content text {text_label} on {surface_label}: \
                         {ratio:.2}:1 is below the required {CONTENT_TEXT}:1"
                    );
                }
            }

            // Labels of controls you can press (4.5:1 normal, 7:1 HC —
            // RFC-070 D4): accent as text (links, ghost/secondary labels),
            // and each fill's own `*_text` against its fill.
            for (surface_label, surface_color) in surfaces {
                let ratio = snora::design::contrast::contrast_ratio(p.accent, surface_color);
                assert!(
                    ratio >= control_label_floor,
                    "{theme_name} accent (as a pressable label) on {surface_label}: \
                     {ratio:.2}:1 is below the required {control_label_floor}:1"
                );
            }
            for (fill_label, fill, fill_text) in [
                ("accent", p.accent, p.accent_text),
                ("danger", p.danger, p.danger_text),
            ] {
                let ratio = snora::design::contrast::contrast_ratio(fill_text, fill);
                assert!(
                    ratio >= control_label_floor,
                    "{theme_name} {fill_label}_text on {fill_label}: {ratio:.2}:1 \
                     is below the required {control_label_floor}:1"
                );
            }

            // Labels of unavailable controls (4.5:1 normal, 7:1 HC — D13
            // item 1 / RFC-070 D4): a disabled button has no fill, so its
            // `text_muted` label sits directly on whichever surface is
            // behind the button. Placeholders (same floor, D6): `text_muted`
            // on the field's own `surface` fill — the same token, the same
            // floor, the same pairing a disabled label on a surface-backed
            // area already covers, stated here under its own name too.
            for (surface_label, surface_color) in surfaces {
                let ratio = snora::design::contrast::contrast_ratio(p.text_muted, surface_color);
                assert!(
                    ratio >= control_label_floor,
                    "{theme_name} text_muted (disabled label / placeholder) on \
                     {surface_label}: {ratio:.2}:1 is below the required \
                     {control_label_floor}:1"
                );
            }

            // Lines, outlines, the focus ring, a badge against the page
            // (3:1 normal, 4.5:1 HC, non-text, SC 1.4.11 / RFC-070 D4):
            // border and every tone's own badge fill, each against every
            // surface it can sit on.
            for (surface_label, surface_color) in surfaces {
                let ratio = snora::design::contrast::contrast_ratio(p.border, surface_color);
                assert!(
                    ratio >= non_text_floor,
                    "{theme_name} border on {surface_label}: {ratio:.2}:1 \
                     is below the required {non_text_floor}:1 non-text boundary floor"
                );
                let ring_ratio = snora::design::contrast::contrast_ratio(
                    theme.tokens.focus.ring_color,
                    surface_color,
                );
                assert!(
                    ring_ratio >= non_text_floor,
                    "{theme_name} focus ring on {surface_label}: {ring_ratio:.2}:1 \
                     is below the required {non_text_floor}:1 non-text boundary floor"
                );
                for (tone_label, tone) in [
                    ("Success", Tone::Success),
                    ("Warning", Tone::Warning),
                    ("Danger", Tone::Danger),
                    ("Info", Tone::Info),
                ] {
                    let (badge_bg, _) = tone.colors(&theme.tokens);
                    let badge_ratio =
                        snora::design::contrast::contrast_ratio(from_iced(badge_bg), surface_color);
                    assert!(
                        badge_ratio >= non_text_floor,
                        "{theme_name} Tone::{tone_label} badge on {surface_label}: \
                         {badge_ratio:.2}:1 is below the required {non_text_floor}:1 \
                         non-text boundary floor"
                    );
                }
            }

            // RFC-070 Amendment A2 item 3: the ring a focused text field
            // actually draws (`ring_color_for`'s choice, not necessarily
            // the raw `focus.ring_color` above) against the field's own
            // background (`surface` — the only background `text_input_style`
            // ever gives a field, confirmed by reading that function).
            let field_ring_color = crate::widget::ring::ring_color_for(
                &theme.tokens,
                Some(iced::Background::Color(to_iced(p.surface))),
            );
            let field_ring_ratio =
                snora::design::contrast::contrast_ratio(from_iced(field_ring_color), p.surface);
            assert!(
                field_ring_ratio >= non_text_floor,
                "{theme_name} a focused field's ring on surface: {field_ring_ratio:.2}:1 \
                 is below the required {non_text_floor}:1 non-text boundary floor"
            );
        }
    }

    /// Task 112 Commit 3: every control knotra draws with a label — buttons
    /// (through snora: `widget::style::{primary,secondary,ghost,danger}`,
    /// thin wrappers over `snora::design::style::button::*`), chips,
    /// the select's field and its open menu's selected row, and the
    /// checkbox — in every state (rest, hovered, pressed, and focused), on
    /// all four presets, against **the real floor each preset actually
    /// declares**: `tokens.palette.text_contrast.ratio()`, not a constant
    /// copied from the palette's own current values (7.0, for every preset
    /// knotra ships today — snora's own `TextContrast` table, 0.65 — but
    /// read live here rather than assumed, so a preset that ever changed
    /// its own declared level would move this test's floor with it, not
    /// require a second edit to keep it honest).
    ///
    /// Drives the real style functions, reading colours back out of the
    /// `Style` each one returns — the same discipline
    /// `focus_ring_meets_wcag_aa_against_every_background_it_can_be_drawn_on`
    /// above already applies to the ring, now applied to the label text and
    /// widened from two presets to four.
    ///
    /// **Disabled is excluded** (WCAG SC 1.4.3 exempts inactive
    /// components) — matching every other test in this file that makes the
    /// same exclusion.
    ///
    /// **Two real gaps in iced's own state model, reported rather than
    /// papered over:**
    /// - `checkbox::Status` (iced 0.14.2) is `Active`/`Hovered`/`Disabled`
    ///   only — there is no `Pressed` variant for a checkbox at all, so
    ///   that column reads "n/a" for it rather than a fabricated number.
    /// - `pick_list::Status`'s closest analogue to "pressed" is `Opened`
    ///   (the control engaged, menu open) — used for that column, since
    ///   the type has no literal `Pressed` either.
    ///
    /// **Focus never moves any of these numbers.** Every style function
    /// below applies its focus ring (when asked) to `border` only — never
    /// to `text_color`/`background`/`selected_text_color`/
    /// `selected_background` — confirmed by reading each one; the
    /// "focused" column's ratio is therefore always identical to its
    /// unfocused counterpart at the same status, asserted explicitly below
    /// rather than skipped, so the table states that fact instead of
    /// silently relying on it.
    #[test]
    fn every_labelled_control_meets_its_presets_own_text_floor() {
        use crate::widget::{checkbox, chip, select};
        use iced::widget::button::Status as ButtonStatus;
        use iced::widget::checkbox::Status as CheckboxStatus;
        use iced::widget::pick_list::Status as FieldStatus;

        /// The colour text actually renders against, for a control whose
        /// own fill may be partly or fully translucent (`ghost`'s rest
        /// background, confirmed by reading it back, is not `None` but
        /// `Some(Background::Color(transparent black))` — alpha 0, not a
        /// real black fill) or absent (`None`) entirely. Alpha-composites
        /// whatever colour the control's own `Style` carries over
        /// `fallback` (the real surface beneath it — the same choice
        /// `focus_ring_meets_wcag_aa_against_every_background_it_can_be_drawn_on`
        /// above already makes for `ghost`/`secondary`, by a hardcoded
        /// special case rather than reading the real alpha back; this
        /// reads it, via `snora::design::contrast::composite_over`, so it
        /// is correct for every alpha a status might render, not only 0
        /// and 1).
        fn bg_or(bg: Option<iced::Background>, fallback: Color) -> Color {
            let translucent = match bg {
                Some(iced::Background::Color(c)) => c,
                None => Color::TRANSPARENT,
                Some(other) => panic!("expected a solid Background::Color, got {other:?}"),
            };
            to_iced(snora::design::contrast::composite_over(
                from_iced(translucent),
                from_iced(fallback),
            ))
        }
        fn bg(bg: iced::Background) -> Color {
            match bg {
                iced::Background::Color(c) => c,
                other => panic!("expected a solid Background::Color, got {other:?}"),
            }
        }

        fn row(
            rows: &mut Vec<String>,
            failures: &mut Vec<String>,
            preset: &str,
            control: &str,
            state: &str,
            ratio: f32,
            floor: f32,
        ) {
            let line =
                format!("{preset:20} {control:28} {state:14} {ratio:5.2}:1  (floor {floor:.1}:1)");
            if ratio < floor {
                failures.push(format!("{line}  <-- UNDER FLOOR"));
            }
            rows.push(line);
        }

        let mut rows: Vec<String> = Vec::new();
        let mut failures: Vec<String> = Vec::new();

        for (preset, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
            ("high_contrast_light", KnotraTheme::high_contrast_light()),
            ("high_contrast_dark", KnotraTheme::high_contrast_dark()),
        ] {
            let tokens = &theme.tokens;
            let floor = tokens.palette.text_contrast.ratio();
            let surface = theme.surface();

            // -- Buttons (through snora) ----------------------------------
            type ButtonStyleFn =
                fn(&snora::design::Tokens, ButtonStatus) -> iced::widget::button::Style;
            let buttons: [(&str, ButtonStyleFn); 4] = [
                ("button:primary", crate::widget::style::primary),
                ("button:secondary", crate::widget::style::secondary),
                ("button:ghost", crate::widget::style::ghost),
                ("button:danger", crate::widget::style::danger),
            ];
            for (name, style_fn) in buttons {
                for (state, status) in [
                    ("rest", ButtonStatus::Active),
                    ("hovered", ButtonStatus::Hovered),
                    ("pressed", ButtonStatus::Pressed),
                ] {
                    let s = style_fn(tokens, status);
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(s.text_color),
                        from_iced(bg_or(s.background, surface)),
                    );
                    row(&mut rows, &mut failures, preset, name, state, ratio, floor);

                    let focused = crate::widget::style::with_focus_ring(
                        tokens,
                        true,
                        style_fn(tokens, status),
                    );
                    let focused_ratio = snora::design::contrast::contrast_ratio(
                        from_iced(focused.text_color),
                        from_iced(bg_or(focused.background, surface)),
                    );
                    row(
                        &mut rows,
                        &mut failures,
                        preset,
                        name,
                        "focused",
                        focused_ratio,
                        floor,
                    );
                    assert_eq!(
                        ratio, focused_ratio,
                        "{preset} {name} {state}: focus moved the label's own ratio, \
                         contradicting this test's own finding that it never does"
                    );
                }
            }

            // -- Chips -------------------------------------------------------
            for selected in [false, true] {
                let name = if selected {
                    "chip:selected"
                } else {
                    "chip:unselected"
                };
                for (state, status) in [
                    ("rest", ButtonStatus::Active),
                    ("hovered", ButtonStatus::Hovered),
                    ("pressed", ButtonStatus::Pressed),
                ] {
                    // count=1: the real, non-zero label path (RFC-071 D5's
                    // count==0 override is its own, separately asserted
                    // pairing — `zero_count_chip_text_is_secondary_
                    // regardless_of_selected` in `chip.rs` — not this
                    // floor's concern).
                    let s = chip::style(tokens, selected, 1, status, false);
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(s.text_color),
                        from_iced(bg_or(s.background, surface)),
                    );
                    row(&mut rows, &mut failures, preset, name, state, ratio, floor);

                    let focused = chip::style(tokens, selected, 1, status, true);
                    let focused_ratio = snora::design::contrast::contrast_ratio(
                        from_iced(focused.text_color),
                        from_iced(bg_or(focused.background, surface)),
                    );
                    row(
                        &mut rows,
                        &mut failures,
                        preset,
                        name,
                        "focused",
                        focused_ratio,
                        floor,
                    );
                    assert_eq!(
                        ratio, focused_ratio,
                        "{preset} {name} {state}: focus moved the label's own ratio"
                    );
                }
            }

            // -- Select: the closed field -------------------------------
            for (state, status) in [
                ("rest", FieldStatus::Active),
                ("hovered", FieldStatus::Hovered),
                ("pressed(open)", FieldStatus::Opened { is_hovered: true }),
            ] {
                let s = select::field_style(tokens, status, false);
                let ratio = snora::design::contrast::contrast_ratio(
                    from_iced(s.text_color),
                    from_iced(bg(s.background)),
                );
                row(
                    &mut rows,
                    &mut failures,
                    preset,
                    "select:field",
                    state,
                    ratio,
                    floor,
                );

                let focused = select::field_style(tokens, status, true);
                let focused_ratio = snora::design::contrast::contrast_ratio(
                    from_iced(focused.text_color),
                    from_iced(bg(focused.background)),
                );
                row(
                    &mut rows,
                    &mut failures,
                    preset,
                    "select:field",
                    "focused",
                    focused_ratio,
                    floor,
                );
                assert_eq!(
                    ratio, focused_ratio,
                    "{preset} select:field {state}: focus moved the label's own ratio"
                );
            }

            // -- Select: the open menu's selected row ---------------------
            // `menu_style` takes no `Status` at all — one real call already
            // is "the real function", nothing to loop over for rest/hovered/
            // pressed (the same reasoning
            // `new_control_focus_rings_meet_wcag_aa_...` above already
            // states for this exact function).
            let menu = select::menu_style(tokens, false);
            let menu_ratio = snora::design::contrast::contrast_ratio(
                from_iced(menu.selected_text_color),
                from_iced(bg(menu.selected_background)),
            );
            row(
                &mut rows,
                &mut failures,
                preset,
                "select:menu-selected-row",
                "rest/hovered/pressed",
                menu_ratio,
                floor,
            );
            let menu_focused = select::menu_style(tokens, true);
            let menu_focused_ratio = snora::design::contrast::contrast_ratio(
                from_iced(menu_focused.selected_text_color),
                from_iced(bg(menu_focused.selected_background)),
            );
            row(
                &mut rows,
                &mut failures,
                preset,
                "select:menu-selected-row",
                "focused",
                menu_focused_ratio,
                floor,
            );
            assert_eq!(
                menu_ratio, menu_focused_ratio,
                "{preset} select menu selected row: focus moved the label's own ratio"
            );

            // -- Checkbox ---------------------------------------------------
            for is_checked in [false, true] {
                let name = if is_checked {
                    "checkbox:checked"
                } else {
                    "checkbox:unchecked"
                };
                for (state, status) in [
                    ("rest", CheckboxStatus::Active { is_checked }),
                    ("hovered", CheckboxStatus::Hovered { is_checked }),
                ] {
                    let s = checkbox::style(tokens, status, false);
                    let ratio = snora::design::contrast::contrast_ratio(
                        from_iced(s.icon_color),
                        from_iced(bg(s.background)),
                    );
                    row(&mut rows, &mut failures, preset, name, state, ratio, floor);

                    let focused = checkbox::style(tokens, status, true);
                    let focused_ratio = snora::design::contrast::contrast_ratio(
                        from_iced(focused.icon_color),
                        from_iced(bg(focused.background)),
                    );
                    row(
                        &mut rows,
                        &mut failures,
                        preset,
                        name,
                        "focused",
                        focused_ratio,
                        floor,
                    );
                    assert_eq!(
                        ratio, focused_ratio,
                        "{preset} {name} {state}: focus moved the label's own ratio"
                    );
                }
                // iced's own `checkbox::Status` has no `Pressed` variant at
                // all (`Active`/`Hovered`/`Disabled` only, confirmed against
                // iced_widget 0.14.2's own enum) — recorded as "n/a", not a
                // fabricated number.
                rows.push(format!(
                    "{preset:20} {name:28} {state:14} n/a — iced's checkbox::Status has no Pressed",
                    state = "pressed"
                ));
            }
        }

        println!(
            "knotra label/control text-contrast matrix ({} rows):",
            rows.len()
        );
        for line in &rows {
            println!("{line}");
        }

        assert!(
            failures.is_empty(),
            "{} of {} label/control pairs fell under their own preset's text-contrast floor:\n{}",
            failures.len(),
            rows.len(),
            failures.join("\n")
        );
    }
}
