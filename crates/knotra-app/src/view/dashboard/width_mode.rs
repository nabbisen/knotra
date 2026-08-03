//! RFC-035 R8: the dashboard's width-derived presentation mode.
//!
//! Presentation-derived only, per RFC-035's own constraint (Internal Design
//! §Responsive strategy) — this type is never stored in `AppState`/
//! `AppConfig` and never drives a message on resize. It exists purely as the
//! result of `iced::widget::responsive`'s per-layout closure, recomputed
//! fresh on every layout pass from the space actually available to the
//! dashboard body, not the window size (`mod.rs`'s `view` is where that
//! distinction is made — `responsive` is wrapped around the body region,
//! not the whole window).
//!
//! Scoped to `dashboard/` rather than `view/` root (Handoff 027 §5): the
//! dashboard is the only current consumer, and RFC-037/038 - candidates for
//! reuse - are not specified enough yet to design a shared home around.
//! Moving this module up a level later is a mechanical change, not a
//! redesign, if that need becomes concrete.

/// The three width bands RFC-035 R8 defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WidthMode {
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
    pub(super) fn from_width(width: f32) -> Self {
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
}
