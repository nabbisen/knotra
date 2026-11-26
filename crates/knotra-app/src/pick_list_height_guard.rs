//! RFC-075 stage 5 §3: this guard's own original job — catching a bare
//! `pick_list(...)` built with no `BUTTON_HEIGHT` anywhere in its own
//! builder chain — is now vacuous: `styled_pick_list`, the one place that
//! ever called `pick_list` directly, retired with the rest of the old
//! focus model (handoff §1), and nothing else in `view/`/`widget/` calls
//! `pick_list` at all (`select_control`, the sole surviving select
//! primitive, is built on `snora::design::select` instead — confirmed,
//! `grep -rn 'pick_list(' crates/knotra-app/src/view crates/knotra-ui/src/
//! widget` finds nothing). The scanner itself (`rust_files_under`/
//! `check_source`/`Violation` and the two tests proving its own matching
//! rules) is gone with it, rather than left to pass on an empty scan
//! forever and tell no one why.
//!
//! What remains answers the same underlying question a different way —
//! not a text scan, since `select_control` never calls `pick_list(` for
//! one to find, and `Select` exposes no `.height(...)` builder to scan for
//! either: a rendered measurement, reading the drawn control's own height
//! back from pixels.

#[cfg(test)]
mod tests {
    /// RFC-075 handoff stage 1 §2: this guard's own rule — a select reaches
    /// `BUTTON_HEIGHT` — now asked of `select_control` (`snora::design::
    /// select`-based), the only select primitive left. **Not a text
    /// scan**: `select_control` never calls `pick_list(`, and `Select`
    /// exposes no `.height(...)` builder at all to scan for (confirmed by
    /// reading its full builder surface, `knotra-ui/src/widget/select.rs`'s
    /// own `select_control` doc comment) — a rendered measurement is the
    /// only way to ask this. Renders at a fixed logical size and reads the
    /// drawn box's own height back from pixels (the same technique
    /// RFC-075 stage 0 used for the focus ring, where `Operation`-based
    /// introspection also could not reach).
    ///
    /// **Does not reach `BUTTON_HEIGHT`.** `select_control`'s drawn box
    /// measures 30px (Task 123: re-measured once every machine renders
    /// headless tests on tiny-skia, never wgpu — stage 2 §1's own 28px
    /// and stage 1's 26px were each read on whichever renderer the
    /// un-pinned headless renderer of that day happened to start, wgpu
    /// locally and tiny-skia on CI, not two real states of the control;
    /// Review 310 F2 traced the gap to exactly that), not 44 — `Select`'s
    /// own trigger row sizes to its content (`snora_widgets::form::
    /// Skin::designed`'s own fixed, non-configurable padding, confirmed by
    /// reading it in the registry), with no seam this crate can use to
    /// make it larger. This test asserts the **found** height, not a
    /// target one, so it still catches a regression (the gap closing or
    /// widening without this guard, or the handoff report, being updated)
    /// rather than silently passing or being deleted — see the handoff
    /// report itself for how the owner wants this carried forward.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and reads back pixels — the pinned face (Noto Sans) is installed \
                  only on the Linux CI runner (Task 108 A1 §4, this crate's own \
                  label_overflow_guard.rs precedent); the height this measures would be \
                  some other font's on macOS/Windows"
    )]
    fn select_control_height_is_measured_not_scanned_and_falls_short_of_button_height() {
        use knotra_ui::test_support::Harness;
        use knotra_ui::theme::KnotraTheme;

        type Msg = ();
        let tokens = KnotraTheme::light().tokens;
        let size = iced_test::core::Size {
            width: 220.0,
            height: 60.0,
        };
        let select: iced::Element<'static, Msg> = knotra_ui::widget::select::select_control(
            &tokens,
            vec![("a", "Alpha".to_owned())],
            Some("a"),
            false,
            |_| (),
            |_| (),
            iced::widget::Id::new("select"),
        );
        let theme = KnotraTheme::light();
        let mut h = Harness::new(select, size, Some(knotra_ui::widget::icon::FONT_BYTES));
        let (rgba, w, h_px) = h.screenshot(&theme.base);

        let mut min_y = u32::MAX;
        let mut max_y = 0u32;
        for y in 0..h_px {
            for x in 0..w.min(110) {
                let i = (((y * w) + x) * 4) as usize;
                if rgba[i..i + 3] != [255, 255, 255] {
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        let measured_height = max_y - min_y + 1;

        // Task 123: `select_control`'s height is derived from glyph metrics
        // (`Select`'s own trigger row sizes to its content), so it still
        // moves a little by font/platform — but every machine measuring
        // this now renders through the same renderer (tiny-skia,
        // `Harness::new`'s own `Some("tiny-skia")`), so the old 22-34px
        // band (wide enough to cover wgpu's own 26-28px *and* tiny-skia's)
        // is no longer needed. 30px, tightened to a 1px tolerance either
        // side for whatever residual font-hinting variance remains between
        // two Linux machines' own fontconfig.
        assert!(
            (29..=31).contains(&measured_height),
            "select_control's measured height ({measured_height}px) moved outside the 29-31px \
             Task 123 re-measured range (every machine now renders through tiny-skia, so this \
             should no longer vary by local-vs-CI renderer) -- if this now reads \
             knotra_ui::widget::BUTTON_HEIGHT (44), update this assertion to match and \
             remove the \"does not reach\" framing above and in the handoff report; if it \
             changed to some other value, the root cause in select.rs's own doc comment \
             needs re-tracing before this assertion is simply updated to follow it"
        );
    }
}
