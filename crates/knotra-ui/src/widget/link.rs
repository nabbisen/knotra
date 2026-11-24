//! The name-link primitive (RFC-071 D4), rebuilt on `snora::design::control`
//! (RFC-075 stage 2 §3) — stage 1 had no primitive for it, since it was
//! written directly inline in `knotra-app`'s `row.rs` before this stage.
//!
//! **The ring difference, reported rather than worked around (handoff's own
//! instruction):** at rest, on hover, and on press, this looks exactly like
//! the link it replaces — same text, same trailing chevron cue, same
//! accent colour on hover/press. On **keyboard focus**, the old link also
//! turned its text accent and underlined it; `Control`'s own style closure
//! takes only `(Theme, button::Status)`, the same shape a plain button's
//! does, with no `Focused` variant to key off of — real focus is drawn
//! entirely by `Control`'s own ring, outside the control, which the old
//! link never had at all. So on focus here: the ring appears (new), the
//! text stays `text_primary` and un-underlined (lost). RFC-071 D4's "never
//! only on hover" is still met — the ring alone already satisfies it — but
//! the underline specifically does not carry over, and is not reproduced by
//! any other means.

use snora::design::Tokens;

use super::STRONG_FONT;
use super::buttons::centered_button_row;
use super::icon;
use super::layout::{BUTTON_HEIGHT, Element};

/// A project name, or any other link-weight row heading, as a Tab stop
/// that presses once on Enter/Space (D1) — `row.rs`'s own `name_button`
/// twin. `label` is a plain string: the old span's conditional colour/
/// underline-on-focus is gone (see this module's own doc comment), so
/// there is no longer a reason to build a `rich_text`/`Span` here instead
/// of a plain `text`.
#[must_use]
pub fn name_link<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    label: impl Into<String>,
    on_press: Message,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let t = tokens.clone();

    let text_label = iced::widget::text(label.into())
        .font(STRONG_FONT)
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens));

    // RFC-071 A1 D13 item 2: the trailing "›" cue, always visible, in
    // `text_secondary` — unchanged from the old link.
    let cue = icon::colored_icon_element(
        &icon::chevron_right(),
        snora::design::style::text::body_small_size(tokens).0,
        snora::design::style::color::to_iced_color(tokens.palette.text_secondary),
    );

    // RFC-071 D8: `left: 0.0` — the old link's own override of iced's
    // `button::DEFAULT_PADDING`, kept byte-for-byte: the VCS-kind text
    // beneath it (`identity_details`, no padding of its own) must still
    // line up with this control's own glyphs at the identity column's
    // left edge, and `button::DEFAULT_PADDING`'s right/top/bottom are
    // part of "the same look at rest" this primitive must keep.
    snora::design::control(
        tokens,
        centered_button_row(iced::widget::row![text_label, cue].spacing(2)),
    )
    .height(BUTTON_HEIGHT)
    .id(id)
    .padding(iced::Padding {
        left: 0.0,
        ..iced::widget::button::DEFAULT_PADDING
    })
    .on_press(on_press)
    .style(move |_theme, status| name_link_style(&t, status))
    .into()
}

/// `text_primary` at rest, `accent` on hover or press — the colour half of
/// the old link's look, unchanged. See the module doc for what keyboard
/// focus no longer changes.
pub(crate) fn name_link_style(
    tokens: &Tokens,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    use iced::widget::button::Status;

    let text_primary = snora::design::style::color::to_iced_color(tokens.palette.text_primary);
    let accent = snora::design::style::color::to_iced_color(tokens.palette.accent);
    let text_color = match status {
        Status::Hovered | Status::Pressed => accent,
        _ => text_primary,
    };
    iced::widget::button::Style {
        background: None,
        text_color,
        ..iced::widget::button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Pressed,
    }

    fn tokens() -> Tokens {
        crate::theme::KnotraTheme::light().tokens
    }

    #[test]
    fn rest_is_text_primary_hover_and_press_are_accent() {
        use iced::widget::button::Status;
        let t = tokens();
        let rest = name_link_style(&t, Status::Active);
        let hovered = name_link_style(&t, Status::Hovered);
        let pressed = name_link_style(&t, Status::Pressed);
        assert_ne!(rest.text_color, hovered.text_color);
        assert_eq!(hovered.text_color, pressed.text_color);
    }

    /// The exact pre-stage-2 construction (`knotra-app`'s `row.rs`, before
    /// RFC-075 §3), reproduced here **only** for this one comparison —
    /// never called by any live view. Kept as a plain `fn`, not reused
    /// from `knotra-app` (this crate cannot depend on it, and the point
    /// is to freeze what the old look *was*, not to keep re-deriving it).
    fn old_name_button<'a, Message: Clone + 'a>(
        tokens: &Tokens,
        label: &'a str,
        on_press: Message,
    ) -> iced::Element<'a, Message> {
        use iced::widget::button::Status;

        let text_primary = snora::design::style::color::to_iced_color(tokens.palette.text_primary);
        let accent = snora::design::style::color::to_iced_color(tokens.palette.accent);

        let text_label = iced::widget::text(label)
            .font(STRONG_FONT)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens));
        let cue = icon::colored_icon_element(
            &icon::chevron_right(),
            snora::design::style::text::body_small_size(tokens).0,
            snora::design::style::color::to_iced_color(tokens.palette.text_secondary),
        );

        iced::widget::button(centered_button_row(
            iced::widget::row![text_label, cue].spacing(2),
        ))
        .height(BUTTON_HEIGHT)
        .padding(iced::Padding {
            left: 0.0,
            ..iced::widget::button::DEFAULT_PADDING
        })
        .on_press(on_press)
        .style(move |_theme, status| iced::widget::button::Style {
            background: None,
            text_color: match status {
                Status::Hovered | Status::Pressed => accent,
                _ => text_primary,
            },
            ..iced::widget::button::Style::default()
        })
        .into()
    }

    /// RFC-075 stage 2 §3, Level 2: `name_link` must look exactly like
    /// the old inline link at rest, in every theme — rendered side by
    /// side and compared byte-for-byte, the same bar stage 1 held
    /// `*_control` to.
    ///
    /// **Tried and found not sensitive to one thing it was meant to
    /// guard**: this control draws no background or border (`background:
    /// None`), so changing its own padding (tried directly: swapped
    /// `button::DEFAULT_PADDING` for an arbitrary `[0, 18]`) moves no
    /// visible pixel — only the invisible click target's own size
    /// changes, which this comparison cannot see. The padding is still
    /// matched to the old link byte-for-byte (its own doc comment says
    /// why), just not provably so by this test; reported rather than
    /// quietly assumed covered.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and reads back pixels — the pinned face (Noto Sans) is installed \
                  only on the Linux CI runner (Task 108 A1 §4, `crates/knotra-app/src/\
                  label_overflow_guard.rs`'s own precedent); the pixels this compares would \
                  be some other font's on macOS/Windows"
    )]
    fn name_link_draws_identically_to_the_old_inline_link_at_rest_in_every_theme() {
        use crate::test_support::{Harness, pixel_diff};

        let size = iced_test::core::Size {
            width: 300.0,
            height: 60.0,
        };
        let icon_font = Some(crate::widget::icon::FONT_BYTES);

        for (name, theme) in [
            ("light", crate::theme::KnotraTheme::light()),
            ("dark", crate::theme::KnotraTheme::dark()),
            (
                "high_contrast_light",
                crate::theme::KnotraTheme::high_contrast_light(),
            ),
            (
                "high_contrast_dark",
                crate::theme::KnotraTheme::high_contrast_dark(),
            ),
        ] {
            let t = &theme.tokens;
            let old: iced::Element<'static, Msg> =
                old_name_button(t, "billing-service", Msg::Pressed);
            let new: iced::Element<'static, Msg> = name_link(
                t,
                "billing-service",
                Msg::Pressed,
                iced::widget::Id::new("link"),
            );

            let mut h_old = Harness::new(old, size, icon_font);
            let (old_rgba, ow, oh) = h_old.screenshot(&theme.base);
            let mut h_new = Harness::new(new, size, icon_font);
            let (new_rgba, nw, nh) = h_new.screenshot(&theme.base);

            if let Err(diff) = pixel_diff(&old_rgba, (ow, oh), &new_rgba, (nw, nh)) {
                panic!("{name}: not identical at rest: {diff}");
            }
        }
    }

    #[test]
    fn name_link_is_a_tab_stop_enter_and_space_press_it() {
        use crate::test_support::Harness;

        let t = tokens();
        let el: iced::Element<'static, Msg> = name_link(
            &t,
            "billing-service",
            Msg::Pressed,
            iced::widget::Id::new("link"),
        );
        let size = iced_test::core::Size {
            width: 300.0,
            height: 100.0,
        };
        let mut h = Harness::new(el, size, None);
        assert_eq!(h.focused_id(), None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("link")));
        h.enter();
        assert_eq!(h.messages(), [Msg::Pressed]);
        h.space();
        assert_eq!(h.messages(), [Msg::Pressed, Msg::Pressed]);
    }
}
