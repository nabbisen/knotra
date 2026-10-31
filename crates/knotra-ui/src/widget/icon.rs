//! Lucide icon wrappers.
//!
//! Exposes a curated, named subset of the Lucide icon set. Call sites use a
//! semantic function name (`icon::chevron_down()`) rather than reaching into
//! `snora::lucide` directly, so the whole icon vocabulary knotra uses is
//! visible in one place and grows deliberately, one named function at a time.
//!
//! Accessible labels are **not** attached here — this crate has no i18n
//! catalog knowledge. Callers supply a localized label from `state.t(...)`
//! alongside these icons, typically via [`super::button::icon_button_maybe`].

pub use snora::widget::icon::{icon_element, icon_element_sized};

/// Renders `icon` at `size`, in `color` — `icon_element`/`icon_element_sized`
/// return a type-erased `Element` with no colour hook, so anything that
/// needs the glyph in a colour other than the surrounding text (RFC-071's
/// status-badge symbol, the project-name "›" cue in `text_secondary`)
/// reproduces `icon_element_sized`'s own glyph construction here, once,
/// with `.color(...)` added.
///
/// Exhaustive over `Icon`'s two variants this workspace's `snora` feature
/// set can ever produce (`Text`, `Lucide`; `svg-icons` is not enabled) —
/// not a guarded match, so a third variant becoming reachable fails to
/// compile here rather than silently falling through.
pub fn colored_icon_element<'a>(
    icon: &snora::Icon,
    size: f32,
    color: iced::Color,
) -> iced::widget::Text<'a, iced::Theme, iced::Renderer> {
    match icon {
        snora::Icon::Lucide(lucide_const) => {
            let glyph = char::from(*lucide_const).to_string();
            iced::widget::text(glyph)
                .font(iced::Font::with_name("lucide"))
                .size(size)
                .color(color)
        }
        snora::Icon::Text(s) => iced::widget::text(s.clone()).size(size).color(color),
    }
}

/// The raw Lucide font bytes. Must be registered once at application startup
/// (`iced::application(...).font(knotra_ui::widget::icon::FONT_BYTES)`) or
/// `Icon::Lucide` glyphs render as tofu / missing-glyph boxes.
pub const FONT_BYTES: &[u8] = snora::lucide::LUCIDE_FONT_BYTES;

/// Disclosure chevron (workspace switcher, expandable sections).
pub fn chevron_down() -> snora::Icon {
    snora::lucide::ChevronDown.into()
}

/// Disclosure chevron, collapsed state (expandable sections).
pub fn chevron_right() -> snora::Icon {
    snora::lucide::ChevronRight.into()
}

/// Settings destination.
pub fn settings() -> snora::Icon {
    snora::lucide::Settings.into()
}

/// History destination.
pub fn history() -> snora::Icon {
    snora::lucide::History.into()
}

/// Dashboard destination.
pub fn dashboard() -> snora::Icon {
    snora::lucide::LayoutDashboard.into()
}

/// Manual status refresh.
pub fn refresh() -> snora::Icon {
    snora::lucide::RefreshCw.into()
}

/// Command palette.
pub fn command_palette() -> snora::Icon {
    snora::lucide::Command.into()
}

/// Close / dismiss.
pub fn close() -> snora::Icon {
    snora::lucide::X.into()
}

/// Create / add (new workspace, new project).
pub fn add() -> snora::Icon {
    snora::lucide::Plus.into()
}

/// Rename.
pub fn rename() -> snora::Icon {
    snora::lucide::Pencil.into()
}

/// Delete / remove (destructive).
pub fn delete() -> snora::Icon {
    snora::lucide::Trash2.into()
}
