//! RFC-035 R8: the dashboard's width-derived presentation mode.
//!
//! Presentation-derived only, per RFC-035's own constraint (Internal Design
//! §Responsive strategy) — this type is never stored in `AppState`/
//! `AppConfig` and never drives a message on resize. It exists purely as the
//! result of `iced::widget::responsive`'s per-layout closure, recomputed
//! fresh on every layout pass from the space actually available to the
//! composition, not the window size.
//!
//! **Revised by Handoff 027 Ruling 6.2**: `responsive` originally wrapped
//! only the dashboard body (`dashboard/mod.rs`'s own `view`). It now wraps
//! `view.rs`'s whole body composition (screen content + selection bar +
//! activity strip), computed once and passed to both `dashboard::view` and
//! `selection_bar::view` — two independent `responsive` wrappers would
//! measure different regions (the dashboard's own padding vs. not) and
//! could classify differently at a band edge, a live risk once a
//! precise-1000px window was found to measure `999.9983` (see
//! `from_width`'s doc).
//!
//! The type itself stays defined here rather than moving to `view/` root
//! (Handoff 027 §5's original reasoning still holds even though the
//! *computation* moved up a level): the dashboard is still the primary
//! consumer and the type's own semantics (R8's breakpoints) are the
//! dashboard's. `pub(crate)` (widened from `pub(super)` by Ruling 6.2) so
//! `view.rs` and `view/selection_bar.rs` can name it.

/// The three width bands RFC-035 R8 defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WidthMode {
    /// 800-999px: two-line rows, collapsed toolbar.
    Compact,
    /// 1000-1279px: bounded three-track rows, full toolbar.
    Standard,
    /// >=1280px: content centred, tracks do not grow.
    Wide,
}

impl WidthMode {
    /// R8's breakpoints: `<1000` compact, `1000..1280` standard, `>=1280`
    /// wide.
    ///
    /// `width` is rounded to the nearest pixel first. `responsive`'s
    /// measured `Size` is not exactly the requested window width — an
    /// exactly-1000px window measured `999.9983` in practice (window
    /// chrome/border loss through the layout tree, not a knotra bug), which
    /// landed just on the wrong side of a strict `<1000.0` comparison and
    /// misclassified R8's own first standard-width pixel as compact. Found
    /// empirically with a temporary `eprintln!` probe (since reverted)
    /// while capturing Handoff 027's required evidence, the same
    /// instrument-and-revert discipline as Handoff 024.
    pub(crate) fn from_width(width: f32) -> Self {
        let width = width.round();
        if width < 1000.0 {
            WidthMode::Compact
        } else if width < 1280.0 {
            WidthMode::Standard
        } else {
            WidthMode::Wide
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WidthMode;

    #[test]
    fn breakpoints_match_r8() {
        assert_eq!(WidthMode::from_width(800.0), WidthMode::Compact);
        assert_eq!(WidthMode::from_width(999.0), WidthMode::Compact);
        assert_eq!(WidthMode::from_width(1000.0), WidthMode::Standard);
        assert_eq!(WidthMode::from_width(1279.0), WidthMode::Standard);
        assert_eq!(WidthMode::from_width(1280.0), WidthMode::Wide);
    }

    /// The exact regression found in the live app: `responsive` measured
    /// `999.9983` for what was, at the OS/window-manager level, a precise
    /// 1000px window.
    #[test]
    fn sub_pixel_measurement_loss_does_not_cross_a_boundary() {
        assert_eq!(WidthMode::from_width(999.9983), WidthMode::Standard);
        assert_eq!(WidthMode::from_width(1279.498), WidthMode::Standard);
        assert_eq!(WidthMode::from_width(1280.0017), WidthMode::Wide);
    }
}
