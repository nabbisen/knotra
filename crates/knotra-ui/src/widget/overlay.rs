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
    fn pixels(self) -> f32 {
        match self {
            OverlayWidth::Small => 400.0,
            OverlayWidth::Standard => 520.0,
            OverlayWidth::Large => 680.0,
        }
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
#[must_use]
pub fn surface<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    width: OverlayWidth,
    title: impl Into<String>,
    on_close: Option<Message>,
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
                .style(move |_theme, status| super::buttons::style::ghost(&t, status)),
        );
    }

    let bounded_body =
        container(scrollable(body).height(Length::Shrink)).max_height(BODY_MAX_HEIGHT);

    let content = column![header, bounded_body, footer.into()].spacing(16);

    let sized = container(content).width(Length::Fixed(width.pixels()));

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
