//! `knotra-ui` — application UI foundation for knotra.
//!
//! Provides theme definitions, i18n helpers, and common layout primitives
//! built on top of `iced` 0.14. The GUI crate depends only on this module
//! for styling concerns, keeping display logic separate from business logic.

pub mod i18n;
pub mod theme;
pub mod widget;

pub use theme::{KnotraTheme, ThemeChoice};

/// RFC-075 handoff stage 1 §1: the keyboard test harness. See its own
/// module doc for why it lives here, behind `test-support`, rather than in
/// `knotra-app` or behind a bare `#[cfg(test)]`.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
