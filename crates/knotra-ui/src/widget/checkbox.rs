//! Checkbox primitive, on `snora::design::checkbox` (RFC-075 stage 3) —
//! already one control with its own built-in minimum target
//! (`Tokens::targets.min`, wired to [`super::layout::BUTTON_HEIGHT`] for
//! every theme this crate ships, `theme.rs`) and its own accessible label,
//! drawn as a real part of the target rather than a hover-only tooltip.

use snora::design::Tokens;

use super::layout::Element;

/// A themed checkbox, on `snora::design::checkbox` (RFC-075 stage 3) —
/// already one control with its own
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
/// (a hover-only accessible label) by wrapping this element itself —
/// `snora::design::checkbox` has no
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// `checkbox_control` draws a legible mark, in every theme: not a blank
    /// square (a render that silently drew nothing would pass a looser
    /// check and should not), and the **checked** fill differs visibly
    /// from the **unchecked** one.
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
                let el: iced::Element<'static, Msg> =
                    checkbox_control(t, "Include in bulk", is_checked, |_| (), Id::new("box"));

                let mut h = Harness::new(el, size, None);
                let (rgba, w, _) = h.screenshot(&theme.base);
                let mark = crop(&rgba, w, CROP);

                assert!(
                    non_background_pixels(&mark, background) > 20,
                    "is_checked={is_checked}: checkbox_control drew no mark"
                );
                crops[i] = mark;
            }
            assert_ne!(
                crops[0], crops[1],
                "checkbox_control's checked mark must look different from its unchecked one"
            );
        }
    }
}
