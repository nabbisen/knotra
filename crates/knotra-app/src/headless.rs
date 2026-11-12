//! Shared harness for headless (`iced_test::Simulator`) UI tests — Task 108
//! Amendment A1 §1.
//!
//! `Simulator::new`'s own default `Settings` leaves two gaps versus the
//! real application (`main.rs`): it falls back to `Font::with_name("Fira
//! Sans")` whenever `default_font` is left at `Font::DEFAULT`
//! (`iced_test`'s own `Simulator::with_size`), while `main.rs` never sets
//! one either and so draws with whatever the system resolves
//! `Font::DEFAULT` (`Family::SansSerif`) to; and it never loads knotra's
//! icon font (`main.rs`'s own `.font(knotra_ui::widget::icon::
//! FONT_BYTES)`), so an icon glyph measures as whatever the fallback
//! font's missing-glyph box is, not the icon's own advance width. Either
//! gap makes every width this harness measures the wrong font's.
//!
//! "Noto Sans" is this machine's actual system sans-serif (confirmed with
//! `fc-match sans-serif`, via the `fonts-noto-core` package) and CI's
//! Linux test job installs the same package (Task 108 A1 §4) — so every
//! headless test measures the same widths this machine's, and CI's, real
//! window would. Japanese text falls back by script, through the
//! renderer's own font database, to whichever CJK face is installed —
//! `fonts-noto-cjk` on CI, confirmed locally to resolve to "Noto Sans CJK
//! JP" (see the CJK snapshot, `headless_probe.rs`).
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped
//! binary.

#[cfg(test)]
pub(crate) fn simulator<'a, Message, Theme, Renderer>(
    element: impl Into<iced_test::core::Element<'a, Message, Theme, Renderer>>,
    logical_size: impl Into<iced_test::core::Size>,
) -> iced_test::Simulator<'a, Message, Theme, Renderer>
where
    Theme: iced_test::core::theme::Base,
    Renderer: iced_test::core::Renderer + iced_test::core::renderer::Headless,
{
    let settings = iced_test::core::Settings {
        fonts: vec![std::borrow::Cow::Borrowed(
            knotra_ui::widget::icon::FONT_BYTES,
        )],
        default_font: iced_test::core::Font::with_name("Noto Sans"),
        default_text_size: iced_test::core::Pixels(16.0),
        ..iced_test::core::Settings::default()
    };
    iced_test::Simulator::with_size(settings, logical_size, element)
}

/// A real window's *logical* area at `zoom_percent`, derived from its
/// *physical* size — the same arithmetic `main.rs`'s own
/// `.scale_factor(|state| zoom_percent / 100)` hook applies, and the same
/// one `headless_probe.rs`'s own `zoom_is_simulated_by_a_smaller_logical_
/// size` test first demonstrated inline. Centralised here so every caller
/// that needs a zoomed logical size (the label-overflow guard's own
/// matrix included) derives it the same way, from the same physical size,
/// rather than re-deriving the formula at each call site.
#[cfg(test)]
pub(crate) fn logical_size_at_zoom(
    physical: iced_test::core::Size,
    zoom_percent: f32,
) -> iced_test::core::Size {
    iced_test::core::Size::new(
        physical.width / (zoom_percent / 100.0),
        physical.height / (zoom_percent / 100.0),
    )
}
