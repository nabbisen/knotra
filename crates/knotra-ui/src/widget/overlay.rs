//! Overlay host surface builder (RFC-034 R8).
//!
//! Builds the opaque, bounded surface every modal dialog renders inside:
//! header (title + close), scrollable body, footer (Cancel + at most one
//! primary action). This module only builds the **surface** — an
//! `Element` with an opaque background, not a transparent container.
//!
//! Registering that surface with `snora::AppLayout::dialog` (so the engine
//! supplies the full-window scrim, input blocking, and `on_close_modals`
//! dispatch) is the application's job, done at the `view.rs` composition
//! point (RFC-034 stage 3). `knotra-ui` has no `Message` type of its own to
//! wire a close dispatch beyond the header close button passed in here, and
//! no `AppState` to read a stacking/focus-return decision from.

use snora::design::Tokens;

use super::icon;
use super::layout::{Element, FONT_BODY, Length};

/// Width tokens for the overlay surface (RFC-034 R8.2: small ~400px,
/// standard ~520px, large ~680px). The exact pixel value is approximate by
/// design — it constrains an inner content wrapper, and the opaque card
/// styling around it adds its own token padding on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayWidth {
    /// Short confirmations, single-field dialogs.
    Small,
    /// The common case: named-field forms (create/rename/delete workspace).
    Standard,
    /// Richer content (plans, previews, multi-field forms).
    Large,
}

impl OverlayWidth {
    /// RFC-051 D2: a fraction of `available` (the window width), clamped to
    /// a per-variant floor and ceiling. The fractions reproduce the pre-RFC-051
    /// fixed constants (400 / 520 / 680) within a fraction of a pixel at
    /// `INITIAL_WINDOW_SIZE` (1100px, R3) — `400/1100`, `520/1100`,
    /// `680/1100`, rounded to four decimal places, so the default window
    /// looks unchanged. Floors keep every variant usable at the
    /// application's 800px minimum window width; ceilings stop `Large`
    /// becoming an unreadable line length on a very wide display. Floors
    /// are all comfortably below 800, so a clamp can never push a result
    /// above the window's actual width (R4) — see the Handoff 070 review
    /// request for the per-variant reasoning, including the conflicted-file
    /// row arithmetic that set `Large`'s floor.
    fn pixels(self, available: f32) -> f32 {
        let (fraction, floor, ceiling) = match self {
            OverlayWidth::Small => (0.3636, 320.0, 460.0),
            OverlayWidth::Standard => (0.4727, 420.0, 600.0),
            OverlayWidth::Large => (0.6182, 640.0, 900.0),
        };
        (fraction * available).clamp(floor, ceiling)
    }
}

/// Cap on the scrollable body's height so a long body never pushes the
/// footer off-screen at the application's 600px minimum window height,
/// leaving room for the header, footer, and surface padding around it.
const BODY_MAX_HEIGHT: f32 = 420.0;

/// Build the opaque overlay surface: header (title + optional close) /
/// scrollable body / footer.
///
/// `on_close` is `None` when the surface has no independent close affordance
/// beyond the caller's own footer buttons — e.g. a non-cancellable
/// in-progress state — in which case no header close button is rendered.
/// Every dialog this RFC migrates passes `Some`; RFC-034 R8.4/security review
/// requires at least one keyboard route to close except where a
/// non-cancellable operation deliberately owns the surface.
///
/// `is_close_focused` draws the RFC-036 focus ring on the close button when
/// true (RFC-036 Stage 5 R8) — ignored when `on_close` is `None`, since
/// there is no close button to ring.
///
/// `available` (RFC-051 D3) is the window's current width in logical
/// pixels, passed by the caller (`state.window_width`) rather than read
/// from anywhere in `knotra-ui`, which has no `AppState` to read it from.
/// `width` stays the enum — `Small`/`Standard`/`Large` remains the
/// vocabulary a call site chooses; only `OverlayWidth::pixels` (private to
/// this module) resolves it against `available`, so a call site can never
/// pass an arbitrary, unenforced pixel value in its place.
#[must_use]
// RFC-051 D3: `available` is the eighth parameter, over clippy's default
// `too_many_arguments` threshold of seven. Keeping `width: OverlayWidth`
// rather than collapsing it with `available` into one caller-resolved
// `f32` (the rejected alternative — see the Handoff 070 review request) is
// what pushes the count past seven; accepted deliberately, not missed.
#[allow(clippy::too_many_arguments)]
pub fn surface<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    width: OverlayWidth,
    available: f32,
    title: impl Into<String>,
    on_close: Option<Message>,
    is_close_focused: bool,
    body: impl Into<Element<'a, Message>>,
    footer: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    use iced::widget::{Space, button, column, container, row, scrollable, text};

    let mut header = row![text(title.into()).size(FONT_BODY + 2.0)]
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(Space::new().width(Length::Fill));

    if let Some(close_msg) = on_close {
        let t = tokens.clone();
        header = header.push(
            button(icon::icon_element(&icon::close()))
                .on_press(close_msg)
                .style(move |_theme, status| {
                    super::buttons::style::with_focus_ring(
                        &t,
                        is_close_focused,
                        super::buttons::style::ghost(&t, status),
                    )
                }),
        );
    }

    let bounded_body =
        container(scrollable(body).height(Length::Shrink)).max_height(BODY_MAX_HEIGHT);

    let content = column![header, bounded_body, footer.into()].spacing(16);

    let sized = container(content).width(Length::Fixed(width.pixels(available)));

    raised_card(tokens, sized)
}

/// A raised, opaque card surface — thin wrapper around
/// `snora::design::card::raised`. Used for [`surface`] above and for
/// free-standing floating content (e.g. a dropdown menu, a tooltip) that
/// needs the same opaque, elevated treatment without the header/body/footer
/// structure.
#[must_use]
pub fn raised_card<'a, Message: 'a>(
    tokens: &Tokens,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    snora::design::card::raised(tokens, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window width `AppState` seeds `window_width` from
    /// (`state.rs::INITIAL_WINDOW_SIZE`) — duplicated here as a literal
    /// rather than imported, since `knotra-ui` does not and should not
    /// depend on `knotra-app`. R3's tolerance is checked against this
    /// value, not derived from it.
    const DEFAULT_WINDOW_WIDTH: f32 = 1100.0;
    /// The application's minimum window width (`main.rs`'s `min_size`).
    const MIN_WINDOW_WIDTH: f32 = 800.0;
    /// A generously wide display, well beyond any realistic laptop/desktop
    /// monitor, to exercise the ceiling.
    const WIDE_WINDOW_WIDTH: f32 = 2560.0;

    /// RFC-051 R3: at the default window width, each variant lands within
    /// 1px of its pre-RFC-051 fixed constant (400 / 520 / 680) — the actual
    /// deviation is under 0.05px (the fractions are `400/520/680 ÷ 1100`,
    /// rounded to four decimal places), so 1px is a deliberately generous
    /// tolerance, not a loose one: it is sub-pixel at any real display scale
    /// factor, chosen to make the assertion legible rather than to just
    /// barely pass.
    #[test]
    fn default_window_width_reproduces_the_pre_rfc_051_constants_within_a_pixel() {
        let cases = [
            (OverlayWidth::Small, 400.0),
            (OverlayWidth::Standard, 520.0),
            (OverlayWidth::Large, 680.0),
        ];
        for (width, expected) in cases {
            let actual = width.pixels(DEFAULT_WINDOW_WIDTH);
            assert!(
                (actual - expected).abs() <= 1.0,
                "{width:?} at {DEFAULT_WINDOW_WIDTH}px: expected within 1px of \
                 {expected}, got {actual}"
            );
        }
    }

    /// RFC-051 R4, the floor half: at the application's minimum window
    /// width, every variant's natural fraction is below its floor (by
    /// construction — the floors were chosen well under 800px precisely so
    /// this holds), so the floor governs and each result is comfortably
    /// less than `MIN_WINDOW_WIDTH` itself — an overlay can never be wider
    /// than the window that contains it, at the narrowest window the
    /// application allows.
    #[test]
    fn minimum_window_width_clamps_to_each_variants_floor_and_never_exceeds_the_window() {
        let cases = [
            (OverlayWidth::Small, 320.0),
            (OverlayWidth::Standard, 420.0),
            (OverlayWidth::Large, 640.0),
        ];
        for (width, floor) in cases {
            let actual = width.pixels(MIN_WINDOW_WIDTH);
            assert_eq!(actual, floor, "{width:?} at {MIN_WINDOW_WIDTH}px");
            assert!(
                actual < MIN_WINDOW_WIDTH,
                "{width:?}'s floor ({actual}) must stay below the window's own \
                 minimum width ({MIN_WINDOW_WIDTH})"
            );
        }
    }

    /// RFC-051 R4, the ceiling half: on a very wide display, every
    /// variant's natural fraction exceeds its ceiling, so the ceiling
    /// governs rather than letting `Large` in particular grow into an
    /// unreadable line length.
    #[test]
    fn wide_window_width_clamps_to_each_variants_ceiling() {
        let cases = [
            (OverlayWidth::Small, 460.0),
            (OverlayWidth::Standard, 600.0),
            (OverlayWidth::Large, 900.0),
        ];
        for (width, ceiling) in cases {
            let actual = width.pixels(WIDE_WINDOW_WIDTH);
            assert_eq!(actual, ceiling, "{width:?} at {WIDE_WINDOW_WIDTH}px");
        }
    }
}
