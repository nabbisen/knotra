//! `snora` 0.9 — application foundation for knotra.
//!
//! Provides theme definitions, i18n helpers, and common layout primitives
//! built on top of `iced` 0.14. The GUI crate depends only on this module
//! for styling concerns, keeping display logic separate from business logic.

pub mod i18n;
pub mod theme;
pub mod nav_menu;
pub mod widget;

pub use theme::{KnotraTheme, StatusColor};
pub use nav_menu::{nav_bar, NavItem, NAV_BAR_HEIGHT};
