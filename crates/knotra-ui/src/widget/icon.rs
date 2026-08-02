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
