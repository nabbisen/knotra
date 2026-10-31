//! RFC-071 D3: the status-badge vocabulary — a small filled shape in a
//! tone's palette colour, with its symbol drawn in the tone's own `*_text`
//! colour. Replaces `theme.rs`'s old `StatusColor` (hard-coded hex, no
//! symbol, no theme) — every colour here comes from `tokens.palette`, so
//! RFC-070's high-contrast presets restyle every badge with no further
//! change.
//!
//! `Tone` names *where a colour comes from* (one of snora's four palette
//! pairs); it carries no symbol of its own. A badge's symbol is a property
//! of the concrete status it represents (RFC-071 D3's table gives "Updates
//! available" and "Local commits" the same `info` tone but different
//! arrows), so every call site passes its own icon alongside the tone it
//! picked — `status_badge` composes the two, it does not choose either.

use iced::widget::container;
use iced::{Alignment, Border, Color, Element, Length};
use snora::design::Tokens;

use super::icon::colored_icon_element;

/// One of snora's four semantic tone pairs (RFC-071 D3). Never a literal
/// colour itself — [`Tone::colors`] reads the pair from `tokens.palette`,
/// so a tone always renders through the active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Success,
    Warning,
    Danger,
    Info,
}

fn to_iced(color: snora::design::Color) -> Color {
    snora::design::style::color::to_iced_color(color)
}

impl Tone {
    /// `(badge background, symbol colour)` — `tokens.palette`'s background/
    /// `*_text` pair for this tone.
    #[must_use]
    pub fn colors(self, tokens: &Tokens) -> (Color, Color) {
        match self {
            Tone::Success => (
                to_iced(tokens.palette.success),
                to_iced(tokens.palette.success_text),
            ),
            Tone::Warning => (
                to_iced(tokens.palette.warning),
                to_iced(tokens.palette.warning_text),
            ),
            Tone::Danger => (
                to_iced(tokens.palette.danger),
                to_iced(tokens.palette.danger_text),
            ),
            Tone::Info => (
                to_iced(tokens.palette.info),
                to_iced(tokens.palette.info_text),
            ),
        }
    }
}

/// A status badge: a filled circle in `tone`'s palette colour, `glyph`
/// centred inside it in the tone's own `*_text` colour. `line_height` is
/// the badge's own diameter — the line height of the text it precedes, so
/// it sits on the same baseline rhythm as its neighbour rather than an
/// independently-chosen pixel size.
#[must_use]
pub fn status_badge<'a, Message: 'a>(
    tokens: &Tokens,
    tone: Tone,
    glyph: &snora::Icon,
    line_height: f32,
) -> Element<'a, Message> {
    let (bg, fg) = tone.colors(tokens);
    let diameter = line_height;
    let glyph_size = diameter * 0.65;

    container(colored_icon_element(glyph, glyph_size, fg))
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_theme| container::Style {
            background: Some(bg.into()),
            border: Border {
                radius: (diameter / 2.0).into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// Lucide symbol chosen per RFC-071 D3 row, named here once rather than
/// repeated at each call site — see the handoff report for the full
/// per-status reasoning.
pub mod symbol {
    /// "Needs help", "Needs a choice", "unreadable".
    #[must_use]
    pub fn alert() -> snora::Icon {
        snora::lucide::AlertCircle.into()
    }

    /// "Unsaved work".
    #[must_use]
    pub fn unsaved() -> snora::Icon {
        snora::lucide::CircleDot.into()
    }

    /// "Updates available".
    #[must_use]
    pub fn updates_available() -> snora::Icon {
        snora::lucide::ArrowDown.into()
    }

    /// "Local commits".
    #[must_use]
    pub fn local_commits() -> snora::Icon {
        snora::lucide::ArrowUp.into()
    }

    /// "All set".
    #[must_use]
    pub fn all_set() -> snora::Icon {
        snora::lucide::Check.into()
    }
}
