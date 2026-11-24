//! Checkbox primitive, built from `KnotraTheme` (RFC-035 Handoff 019 §7.3).
//!
//! iced's own `checkbox::Checkbox` has a `size` setter but no `height`
//! setter — its layout is always `Length::Shrink` (`checkbox.rs`'s own
//! `size()` fn), so a drawn box alone cannot be widened into a 44px target
//! the way a button can. The interactive target here is a [`BOX_SIZE`]
//! drawn box centred inside a [`TARGET_SIZE`] `mouse_area`, matching
//! `layout::BUTTON_HEIGHT`'s existing 44px convention.
//!
//! A real `iced::widget::checkbox` still draws the box/check glyph and
//! still owns `on_toggle` directly, so a click landing on the small drawn
//! box is captured there (`checkbox.rs`'s `update()` calls
//! `shell.capture_event()`) and the wrapping `mouse_area` — which only acts
//! when the inner content did not capture the event
//! (`mouse_area.rs`'s `update()`) — supplies the same toggle for clicks in
//! the surrounding padding. No double-toggle: exactly one of the two fires
//! per click.
//!
//! Carries an `accessible_label`, shown as a hover tooltip, following
//! `buttons.rs`'s `icon_button_maybe` — the same reasoning applies here:
//! nothing on-screen names the control besides its checked state.

use iced::widget::{checkbox as iced_checkbox, container, mouse_area, text, tooltip};
use snora::design::Tokens;

use super::layout::{Element, Length};
use super::ring::ring_color_for;

/// The drawn box size — the 44px target is the *interactive* area, not
/// necessarily the visible box (RFC-035 Handoff 019 §7.3).
const BOX_SIZE: f32 = 18.0;

/// The minimum interactive target, matching `layout::BUTTON_HEIGHT`.
const TARGET_SIZE: f32 = 44.0;

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// A themed checkbox with a 44px hit target around an 18px drawn box.
#[must_use]
pub fn checkbox<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    accessible_label: impl Into<String>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let toggle_message = on_toggle(!is_checked);

    let box_widget: Element<'a, Message> = iced_checkbox::Checkbox::new(is_checked)
        .size(BOX_SIZE)
        .on_toggle(on_toggle)
        .style(move |_theme, status| style(&t, status, is_focused))
        .into();

    let target = container(box_widget).center(Length::Fixed(TARGET_SIZE));

    tooltip(
        mouse_area(target).on_press(toggle_message),
        super::overlay::raised_card(
            tokens,
            // RFC-071 Amendment A3 D14: a tooltip is a transient hint, not
            // primary content read to act — kept `body_small`, same as
            // D10's header icon-button tooltips ("today's tooltip",
            // unchanged by this amendment).
            text(accessible_label.into())
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        ),
        tooltip::Position::Bottom,
    )
    .into()
}

/// RFC-075 D1/Stage 1: [`checkbox`]'s own twin on `snora::design::checkbox`
/// — see `buttons.rs`'s `_control` section doc for the naming reasoning.
/// Unlike [`checkbox`], there is no separate mouse-area/tooltip composition
/// here: `snora::design::checkbox` is already one control with its own
/// built-in minimum target (`Tokens::targets.min`, wired to
/// [`super::layout::BUTTON_HEIGHT`] for every theme this crate ships —
/// `theme.rs`) and its own accessible label, through
/// [`snora_widgets::form::Checkbox::label`]'s own `label` parameter — a
/// real part of the control's target, not a hover-only tooltip, so `label`
/// below is shown, not merely announced.
#[must_use]
pub fn checkbox_control<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    snora::design::checkbox(tokens, is_checked, super::layout::APP_DIRECTION)
        .label(label)
        .on_toggle(on_toggle)
        .id(id)
        .into()
}

/// [`checkbox_control`] without `.label(...)` at all — RFC-075 stage 2 §3
/// (Review 298 F4): a dashboard row's checkbox draws no label of its own
/// (the project name beside it already says which row it is; a drawn
/// label here would repeat it). The caller supplies today's tooltip text
/// the same way the old [`checkbox`] did (a hover-only accessible label),
/// by wrapping this element itself — `snora::design::checkbox` has no
/// separate "accessible label, not drawn" parameter of its own, only
/// `.label(...)`, which this skips entirely rather than calling with an
/// empty string.
#[must_use]
pub fn checkbox_control_unlabeled<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    snora::design::checkbox(tokens, is_checked, super::layout::APP_DIRECTION)
        .on_toggle(on_toggle)
        .id(id)
        .into()
}

/// `surface`/`accent` background, `border` at rest, `text_primary` icon
/// colour when unchecked and `accent_text` when checked (the same
/// on-accent pairing `chip::filter`'s selected state and `select`'s
/// selected menu row both use) — then the focus ring on top when
/// `is_focused`, reusing [`ring_color_for`] rather than a second
/// implementation (see `select.rs`'s module doc for why `with_focus_ring`
/// itself cannot be called directly here).
///
/// `pub(crate)` so `theme.rs`'s contrast test can drive the real function
/// across every `Status` (RFC-035 Handoff 022 §7.5, same reason
/// `chip::style` was made `pub(crate)` in Handoff 020).
pub(crate) fn style(
    tokens: &Tokens,
    status: iced_checkbox::Status,
    is_focused: bool,
) -> iced_checkbox::Style {
    use iced_checkbox::Status;

    let (is_checked, is_disabled) = match status {
        Status::Active { is_checked } | Status::Hovered { is_checked } => (is_checked, false),
        Status::Disabled { is_checked } => (is_checked, true),
    };

    let (background, icon_color) = if is_checked {
        (
            to_iced(tokens.palette.accent),
            to_iced(tokens.palette.accent_text),
        )
    } else {
        (
            to_iced(tokens.palette.surface),
            to_iced(tokens.palette.text_primary),
        )
    };
    let background = if is_disabled {
        iced::Color {
            a: 0.5,
            ..background
        }
    } else {
        background
    };

    let base = iced_checkbox::Style {
        background: iced::Background::Color(background),
        icon_color,
        border: iced::Border {
            color: to_iced(tokens.palette.border),
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
        text_color: None,
    };

    if !is_focused || is_disabled {
        return base;
    }

    iced_checkbox::Style {
        border: iced::Border {
            color: ring_color_for(tokens, Some(base.background)),
            width: tokens.focus.ring_width,
            radius: base.border.radius,
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_checkbox::Status;

    /// RFC-075 handoff stage 1 §3, row 3 — see `buttons.rs`'s test of the
    /// same name for the full reasoning; this is `checkbox_control`'s copy
    /// of it. A checkbox's own "press" toggles it — this checks both
    /// directions (checked -> unchecked via Enter, unchecked -> checked
    /// via Space), not just one.
    #[test]
    fn checkbox_control_is_a_tab_stop_enter_and_space_press_it_disabled_is_skipped() {
        use crate::test_support::Harness;
        use iced::widget::{Id, column};

        #[derive(Debug, Clone, PartialEq)]
        enum Msg {
            Toggled(bool),
        }

        let tokens = Tokens::light();
        let view = |enabled: bool| -> iced::Element<'static, Msg> {
            let mut box_a =
                snora::design::checkbox(&tokens, true, super::super::layout::APP_DIRECTION)
                    .label("Include in bulk")
                    .id(Id::new("box_a"));
            if enabled {
                box_a = box_a.on_toggle(Msg::Toggled);
            }
            column![
                iced::Element::from(box_a),
                checkbox_control(&tokens, "Tags", false, Msg::Toggled, Id::new("box_b")),
            ]
            .spacing(8)
            .into()
        };
        let size = iced_test::core::Size {
            width: 400.0,
            height: 400.0,
        };

        // Enter toggles a checked box off.
        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(Id::new("box_a")));
        h.enter();
        assert_eq!(h.messages(), [Msg::Toggled(false)]);

        // Space toggles an unchecked box on.
        let mut h = Harness::new(view(true), size, None);
        h.tab(false);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(Id::new("box_b")));
        h.space();
        assert_eq!(h.messages(), [Msg::Toggled(true)]);

        // Disabled: Tab skips the first box entirely.
        let mut h = Harness::new(view(false), size, None);
        h.tab(false);
        assert_eq!(
            h.focused_id(),
            Some(Id::new("box_b")),
            "a disabled checkbox must not be a Tab stop"
        );
    }

    /// RFC-035 Handoff 019 §8: focused and unfocused styles must differ,
    /// for both the checked and unchecked backgrounds (`accent` and
    /// `surface` are different colours, so the ring-vs-background contrast
    /// decision runs independently for each).
    #[test]
    fn focused_and_unfocused_styles_differ() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            for is_checked in [false, true] {
                let unfocused = style(&tokens, Status::Active { is_checked }, false);
                let focused = style(&tokens, Status::Active { is_checked }, true);
                assert_ne!(unfocused.border, focused.border, "is_checked={is_checked}");
            }
        }
    }

    /// RFC-075 handoff stage 1 §3, row 2. **Found, and reported, rather
    /// than forced to pass**: `checkbox_control`'s drawn mark is not
    /// pixel-identical to `checkbox`'s, in two ways confirmed by rendering
    /// both and reading the pixels back (crop saved and read directly,
    /// not inferred):
    ///
    /// - **square corners, not rounded.** `checkbox`'s own box draws with
    ///   `tokens.radius.sm`; `snora::design::checkbox`'s does not expose a
    ///   radius at all (confirmed against `snora_widgets::form::Checkbox`'s
    ///   own builder, in the registry — no such method) and draws square.
    /// - **a darker border.** `checkbox`'s border is `tokens.palette.
    ///   border` (the 3:1 role); snora's own is `tokens.palette.
    ///   text_secondary` (`snora_widgets::design::form`'s own module doc,
    ///   `Skin::colors`'s `Look::Designed` arm, read directly: "**Not
    ///   `palette.border`**: that role is held at 3:1 against the page,
    ///   and a hovered or pressed row draws a tint behind the mark...
    ///   `text_secondary` is held at the text level" — snora's own
    ///   deliberate choice, not an oversight).
    ///
    /// Neither is the ring the RFC anticipated reporting (D1's "the focus
    /// ring may differ") — both are at rest. **Also found, and kept, not
    /// reported as a regression**: `checkbox`'s `accessible_label` is a
    /// hover-only tooltip (this file's own module doc); `snora::design::
    /// checkbox`'s `label` is drawn, always — a real target
    /// (`snora_widgets::form::Checkbox::label`'s own doc: "part of the
    /// target"), not hidden behind a hover. That is this primitive's own
    /// accessibility improvement over Handoff 019's original choice, not a
    /// "same look" violation to chase.
    ///
    /// What this test actually checks, since exact pixels are not the
    /// right bar here: both draw **some** mark (not a blank square — a
    /// render that silently drew nothing would pass a looser check and
    /// should not), and the **checked** fill differs visibly from the
    /// **unchecked** one for each control on its own terms — the state the
    /// box shows is legible, which full pixel identity is not actually
    /// needed to prove.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and reads back pixels — the pinned face (Noto Sans) is installed \
                  only on the Linux CI runner (Task 108 A1 §4, `crates/knotra-app/src/\
                  label_overflow_guard.rs`'s own precedent); what this measures would be \
                  some other font's on macOS/Windows"
    )]
    fn checkbox_control_draws_a_legible_mark_whose_checked_state_differs_from_unchecked() {
        use crate::test_support::Harness;
        use crate::theme::KnotraTheme;
        use iced::widget::Id;

        type Msg = ();
        let size = iced_test::core::Size {
            width: 220.0,
            height: 60.0,
        };
        const CROP: u32 = 30;

        fn crop(rgba: &[u8], w: u32, side: u32) -> Vec<u8> {
            let mut out = Vec::with_capacity((side * side * 4) as usize);
            for y in 0..side {
                let row_start = ((y * w) * 4) as usize;
                let row_end = row_start + (side * 4) as usize;
                out.extend_from_slice(&rgba[row_start..row_end]);
            }
            out
        }

        fn non_background_pixels(crop: &[u8], background: [u8; 4]) -> usize {
            crop.chunks_exact(4).filter(|p| *p != background).count()
        }

        for (_preset, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
            ("high_contrast_light", KnotraTheme::high_contrast_light()),
            ("high_contrast_dark", KnotraTheme::high_contrast_dark()),
        ] {
            let t = &theme.tokens;
            let background = {
                use iced::theme::Base;
                let c = theme.base.base().background_color;
                [
                    (c.r * 255.0).round() as u8,
                    (c.g * 255.0).round() as u8,
                    (c.b * 255.0).round() as u8,
                    (c.a * 255.0).round() as u8,
                ]
            };

            let mut crops: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            for (i, is_checked) in [false, true].into_iter().enumerate() {
                let old: iced::Element<'static, Msg> =
                    checkbox(t, "Include in bulk", is_checked, |_| (), false);
                let new: iced::Element<'static, Msg> =
                    checkbox_control(t, "Include in bulk", is_checked, |_| (), Id::new("box"));

                let mut h_old = Harness::new(old, size, None);
                let (old_rgba, ow, _) = h_old.screenshot(&theme.base);
                let mut h_new = Harness::new(new, size, None);
                let (new_rgba, nw, _) = h_new.screenshot(&theme.base);

                let old_mark = crop(&old_rgba, ow, CROP);
                let new_mark = crop(&new_rgba, nw, CROP);

                assert!(
                    non_background_pixels(&old_mark, background) > 20,
                    "is_checked={is_checked}: checkbox drew no mark"
                );
                assert!(
                    non_background_pixels(&new_mark, background) > 20,
                    "is_checked={is_checked}: checkbox_control drew no mark"
                );
                crops[i] = new_mark;
            }
            assert_ne!(
                crops[0], crops[1],
                "checkbox_control's checked mark must look different from its unchecked one"
            );
        }
    }

    /// A disabled checkbox never draws a ring, matching
    /// `buttons.rs`'s `filled_background`'s exclusion of `Disabled`
    /// (RFC-036 Stage 6 closure — see `theme.rs`'s test of the same name
    /// for why).
    #[test]
    fn disabled_style_is_unaffected_by_is_focused() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let a = style(&tokens, Status::Disabled { is_checked: true }, false);
            let b = style(&tokens, Status::Disabled { is_checked: true }, true);
            assert_eq!(a.border, b.border);
        }
    }
}
