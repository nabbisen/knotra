//! RFC-075 handoff stage 2 §3: every control's real (`iced::widget::Id`)
//! Tab target, **in one place** — the same discipline `shell.rs`'s/
//! `toolbar.rs`'s own `focus_target` modules held knotra's `FocusTarget`
//! keys to, now for the ids `.id(...)` actually takes. The string each
//! function builds is the same key the old `FocusTarget` used, so nothing
//! downstream (a capture, a saved report) needs to learn a new name.
//!
//! A row's ids are built from its project's id (`row::checkbox`/`name`/
//! `action`), matching `dashboard/row.rs`'s own retired `checkbox_key`/
//! `name_key`/`action_key` one-to-one.

use iced::widget::Id;
use knotra_vcs::ProjectId;

use crate::state::dashboard::DashboardTier;

pub mod shell {
    use super::Id;

    pub fn workspace_switcher() -> Id {
        Id::new("shell.workspace_switcher")
    }
    pub fn nav_dashboard() -> Id {
        Id::new("shell.nav.dashboard")
    }
    pub fn nav_history() -> Id {
        Id::new("shell.nav.history")
    }
    pub fn refresh() -> Id {
        Id::new("shell.refresh")
    }
    pub fn palette() -> Id {
        Id::new("shell.palette")
    }
    pub fn settings() -> Id {
        Id::new("shell.settings")
    }
}

pub mod toolbar {
    use super::Id;

    pub fn filter_needs_help() -> Id {
        Id::new("dashboard.toolbar.filter.needs_help")
    }
    pub fn filter_dirty() -> Id {
        Id::new("dashboard.toolbar.filter.dirty")
    }
    pub fn filter_behind() -> Id {
        Id::new("dashboard.toolbar.filter.behind")
    }
    pub fn filter_ahead() -> Id {
        Id::new("dashboard.toolbar.filter.ahead")
    }
    pub fn filter_all_set() -> Id {
        Id::new("dashboard.toolbar.filter.all_set")
    }
    pub fn arrange() -> Id {
        Id::new("dashboard.toolbar.arrange")
    }
    pub fn clear_filters() -> Id {
        Id::new("dashboard.toolbar.clear_filters")
    }
    pub fn select_mode() -> Id {
        Id::new("dashboard.toolbar.select_mode")
    }
    /// The Arrange panel's "Group by" select (RFC-075 stage 2 §3).
    pub fn arrange_group() -> Id {
        Id::new("dashboard.toolbar.arrange.group")
    }
    /// The Arrange panel's "Sort by" select.
    pub fn arrange_sort() -> Id {
        Id::new("dashboard.toolbar.arrange.sort")
    }
}

pub mod section {
    use super::{DashboardTier, Id};

    pub fn toggle(tier: DashboardTier) -> Id {
        Id::from(format!("dashboard.section.{tier:?}"))
    }
}

pub mod row {
    use super::{Id, ProjectId};

    pub fn checkbox(id: &ProjectId) -> Id {
        Id::from(format!("dashboard.row.{id}.checkbox"))
    }
    pub fn name(id: &ProjectId) -> Id {
        Id::from(format!("dashboard.row.{id}.name"))
    }
    pub fn action(id: &ProjectId) -> Id {
        Id::from(format!("dashboard.row.{id}.action"))
    }
}

pub mod empty {
    use super::Id;

    pub fn add_first_project() -> Id {
        Id::new("dashboard.empty.add_first_project")
    }
    /// Distinct from [`super::toolbar::clear_filters`] — the no-matches
    /// empty state's own "Clear filters", drawn in the content area rather
    /// than the toolbar (RFC-075 stage 0 inventory §2d).
    pub fn clear_filters() -> Id {
        Id::new("dashboard.empty.clear_filters")
    }
}

pub mod selection_bar {
    use super::Id;

    pub fn fetch() -> Id {
        Id::new("dashboard.selection_bar.fetch")
    }
    pub fn pull() -> Id {
        Id::new("dashboard.selection_bar.pull")
    }
    pub fn tag() -> Id {
        Id::new("dashboard.selection_bar.tag")
    }
    pub fn switch() -> Id {
        Id::new("dashboard.selection_bar.switch")
    }
    pub fn exit() -> Id {
        Id::new("dashboard.selection_bar.exit")
    }
}
