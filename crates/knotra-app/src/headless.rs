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

/// Task 122 §0 (Review 311 §2): `Simulator` reads `ICED_TEST_BACKEND` from
/// the environment at construction, not from anything this function
/// passes it directly (`iced_test`'s own `Settings` has no backend field —
/// confirmed by reading its full field list, Task 123's own report). The
/// workspace's `.cargo/config.toml` sets it to `"tiny-skia"` for every
/// process *cargo* spawns — but a test binary run directly (from an IDE or
/// a debugger, not through `cargo test`) never passes through cargo at
/// all, so the variable is simply absent there, and `Simulator` falls back
/// to wgpu without a word, with Task 123's own segfault (Review 310 F2)
/// right behind it. Checked once, here, rather than left to fail
/// unexplained deep inside wgpu's own Vulkan loader.
#[cfg(test)]
fn require_tiny_skia_backend() {
    let backend = std::env::var("ICED_TEST_BACKEND").unwrap_or_default();
    assert!(
        backend == "tiny-skia",
        "ICED_TEST_BACKEND is {backend:?}, not \"tiny-skia\" -- a headless test must never \
         start wgpu (Task 123 §1, Review 310 F2: it touches a real Vulkan instance and the \
         host's display, and several test threads doing that at once segfaults). Run tests \
         through `cargo test`, which picks up `.cargo/config.toml`'s own `[env]` automatically; \
         a test binary invoked directly needs `ICED_TEST_BACKEND=tiny-skia` exported in the \
         same shell first."
    );
}

#[cfg(test)]
pub(crate) fn simulator<'a, Message, Theme, Renderer>(
    element: impl Into<iced_test::core::Element<'a, Message, Theme, Renderer>>,
    logical_size: impl Into<iced_test::core::Size>,
) -> iced_test::Simulator<'a, Message, Theme, Renderer>
where
    Theme: iced_test::core::theme::Base,
    Renderer: iced_test::core::Renderer + iced_test::core::renderer::Headless,
{
    require_tiny_skia_backend();
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
