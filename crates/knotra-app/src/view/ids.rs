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
    /// RFC-074 Stage 2 D1/D5: "Get latest safely" with nothing selected —
    /// shown only while at least one project has updates available, and
    /// never in selection mode (the selection bar carries the action
    /// there instead).
    pub fn get_latest() -> Id {
        Id::new("dashboard.toolbar.get_latest")
    }
    /// The Arrange panel's "Group by" select (RFC-075 stage 2 §3).
    pub fn arrange_group() -> Id {
        Id::new("dashboard.toolbar.arrange.group")
    }
    /// The Arrange panel's "Sort by" select.
    pub fn arrange_sort() -> Id {
        Id::new("dashboard.toolbar.arrange.sort")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::SEARCH`
    /// — every text-field id now lives here, with every other control's.
    pub fn search() -> Id {
        Id::new("dashboard-search")
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
    /// RFC-074 Stage 3 D3/D5: a row's own "Get latest safely" — distinct
    /// from [`action`] (the NeedsHelp tier's "Choose resolution"/"Show
    /// details"), since a row never shows two actions and the two never
    /// coexist on the same row, but each still deserves its own id.
    pub fn get_latest(id: &ProjectId) -> Id {
        Id::from(format!("dashboard.row.{id}.get_latest"))
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

/// RFC-075 stage 3 §1: the page's own last two regions.
pub mod detail_panel {
    use super::Id;

    pub fn close() -> Id {
        Id::new("detail_panel.close")
    }
    pub fn refresh() -> Id {
        Id::new("detail_panel.refresh")
    }
    pub fn fetch() -> Id {
        Id::new("detail_panel.fetch")
    }
    pub fn remove() -> Id {
        Id::new("detail_panel.remove")
    }
}

pub mod activity_strip {
    use super::Id;

    pub fn undo() -> Id {
        Id::new("activity_strip.undo")
    }
    pub fn dismiss() -> Id {
        Id::new("activity_strip.dismiss")
    }
    /// Shown only in `LatestOpState::Completed`'s own
    /// `RetryAvailability::Available` arm — not in the handoff's own
    /// Undo/Dismiss/Details table, which names the strip's two
    /// mutually-exclusive phases (the undo snackbar; a completed run)
    /// together without this conditional fourth control. Flagged here
    /// rather than folded silently under `details()`.
    pub fn retry() -> Id {
        Id::new("activity_strip.retry")
    }
    pub fn details() -> Id {
        Id::new("activity_strip.details")
    }
}

/// RFC-075 stage 3 §2: every window. `close()` is shared by every window
/// that draws one through `knotra_ui::widget::overlay::surface` (every
/// window but the workspace switcher menu, which has none) — one id per
/// window, not one per window *and* per phase, since a window's own
/// header close button is the same control throughout its own lifetime.
pub mod smart_pull {
    use super::Id;

    pub fn close() -> Id {
        Id::new("smart_pull.close")
    }
    pub fn review_retry() -> Id {
        Id::new("smart_pull.review_retry")
    }
    pub fn retry_failed_close() -> Id {
        Id::new("smart_pull.retry_failed_close")
    }
    pub fn check_only(index: usize) -> Id {
        Id::from(format!("smart_pull.entry.{index}.check_only"))
    }
    pub fn get_anyway(index: usize) -> Id {
        Id::from(format!("smart_pull.entry.{index}.get_anyway"))
    }
    pub fn start() -> Id {
        Id::new("smart_pull.start")
    }
    pub fn cancel() -> Id {
        Id::new("smart_pull.cancel")
    }
    pub fn show_details() -> Id {
        Id::new("smart_pull.show_details")
    }
    pub fn done_close() -> Id {
        Id::new("smart_pull.done_close")
    }
}

pub mod freezer {
    use super::Id;

    pub fn close() -> Id {
        Id::new("freezer.close")
    }
    pub fn check_readiness() -> Id {
        Id::new("freezer.check_readiness")
    }
    pub fn save() -> Id {
        Id::new("freezer.save")
    }
    pub fn show_details() -> Id {
        Id::new("freezer.show_details")
    }
    pub fn cancel() -> Id {
        Id::new("freezer.cancel")
    }
    pub fn share() -> Id {
        Id::new("freezer.share")
    }
    pub fn not_now() -> Id {
        Id::new("freezer.not_now")
    }
    pub fn done_close() -> Id {
        Id::new("freezer.done_close")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// RELEASE_NAME`.
    pub fn release_name() -> Id {
        Id::new("release-name")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// RELEASE_NOTE` — RFC-066 Stage 2's own optional note field, the
    /// first thing to need a stable id at all.
    pub fn release_note() -> Id {
        Id::new("release-note")
    }
}

pub mod context_switch {
    use super::Id;

    pub fn close() -> Id {
        Id::new("context_switch.close")
    }
    pub fn confirm() -> Id {
        Id::new("context_switch.confirm")
    }
    pub fn cancel() -> Id {
        Id::new("context_switch.cancel")
    }
    pub fn show_details() -> Id {
        Id::new("context_switch.show_details")
    }
    pub fn done_close() -> Id {
        Id::new("context_switch.done_close")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// SWITCH_TARGET`.
    pub fn target() -> Id {
        Id::new("switch-target")
    }
}

pub mod changelog {
    use super::{Id, ProjectId};

    pub fn close() -> Id {
        Id::new("changelog.close")
    }
    pub fn project_toggle(id: &ProjectId) -> Id {
        Id::from(format!("changelog.project.{id}"))
    }
    pub fn generate() -> Id {
        Id::new("changelog.generate")
    }
    pub fn copy() -> Id {
        Id::new("changelog.copy")
    }
    pub fn done_close() -> Id {
        Id::new("changelog.done_close")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// CHANGELOG_SINCE` — RFC-066 Stage 2's own "Since" field.
    pub fn since() -> Id {
        Id::new("changelog-since")
    }
}

pub mod resolve {
    use super::Id;

    pub fn close() -> Id {
        Id::new("resolve.close")
    }
    pub fn keep_fixing() -> Id {
        Id::new("resolve.keep_fixing")
    }
    pub fn stop() -> Id {
        Id::new("resolve.stop")
    }
    pub fn done_close() -> Id {
        Id::new("resolve.done_close")
    }
    pub fn details_toggle() -> Id {
        Id::new("resolve.details_toggle")
    }
    pub fn open_in_editor(index: usize) -> Id {
        Id::from(format!("resolve.file.{index}.editor"))
    }
    pub fn open_in_compare(index: usize) -> Id {
        Id::from(format!("resolve.file.{index}.compare"))
    }
    pub fn mark_done(index: usize) -> Id {
        Id::from(format!("resolve.file.{index}.mark_done"))
    }
    pub fn stop_this_fix() -> Id {
        Id::new("resolve.stop_this_fix")
    }
}

pub mod palette {
    use super::Id;

    pub fn close() -> Id {
        Id::new("palette.close")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// PALETTE_QUERY`.
    pub fn query() -> Id {
        Id::new("palette-query")
    }
}

pub mod add_project {
    use super::Id;

    pub fn close() -> Id {
        Id::new("add_project.close")
    }
    pub fn browse() -> Id {
        Id::new("add_project.browse")
    }
    pub fn next() -> Id {
        Id::new("add_project.next")
    }
    pub fn cancel() -> Id {
        Id::new("add_project.cancel")
    }
    pub fn add() -> Id {
        Id::new("add_project.add")
    }
    pub fn back() -> Id {
        Id::new("add_project.back")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// ADD_PROJECT_PATH`.
    pub fn path() -> Id {
        Id::new("add-project-path")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// ADD_PROJECT_NAME`.
    pub fn name() -> Id {
        Id::new("add-project-name")
    }
}

/// Create and Rename share one dialog shape; Delete is its own.
pub mod workspace_manager {
    use super::Id;

    pub fn close() -> Id {
        Id::new("workspace_manager.close")
    }
    pub fn confirm() -> Id {
        Id::new("workspace_manager.confirm")
    }
    pub fn cancel() -> Id {
        Id::new("workspace_manager.cancel")
    }
    pub fn confirm_delete() -> Id {
        Id::new("workspace_manager.confirm_delete")
    }
    pub fn delete_cancel() -> Id {
        Id::new("workspace_manager.delete_cancel")
    }
    /// RFC-075 stage 5 §1: moved from `knotra_ui::widget::focus_id::
    /// WORKSPACE_NAME`.
    pub fn name() -> Id {
        Id::new("workspace-name")
    }
}

/// The workspace switcher menu — its own dropdown, not `workspace_manager`
/// (`view/shell.rs`, per the stage-0 inventory's own §7e).
pub mod switcher {
    use super::Id;
    use knotra_vcs::model::workspace::WorkspaceId;

    pub fn row(id: &WorkspaceId) -> Id {
        Id::from(format!("switcher.row.{id}"))
    }
    pub fn add_workspace() -> Id {
        Id::new("switcher.add_workspace")
    }
    pub fn rename() -> Id {
        Id::new("switcher.rename")
    }
    pub fn delete() -> Id {
        Id::new("switcher.delete")
    }
}

pub mod remove_project {
    use super::Id;

    pub fn close() -> Id {
        Id::new("remove_project.close")
    }
    pub fn keep() -> Id {
        Id::new("remove_project.keep")
    }
    pub fn remove() -> Id {
        Id::new("remove_project.remove")
    }
}

pub mod shortcuts {
    use super::Id;

    pub fn close() -> Id {
        Id::new("shortcuts.close")
    }
}

/// RFC-075 stage 4 §2.
pub mod settings {
    use super::Id;
    use knotra_vcs::model::workspace::WorkspaceId;

    pub fn locale() -> Id {
        Id::new("settings.locale")
    }
    pub fn theme() -> Id {
        Id::new("settings.theme")
    }
    pub fn zoom() -> Id {
        Id::new("settings.zoom")
    }
    pub fn refresh_auto() -> Id {
        Id::new("settings.refresh_auto")
    }
    pub fn max_concurrent() -> Id {
        Id::new("settings.max_concurrent")
    }
    pub fn max_logs() -> Id {
        Id::new("settings.max_logs")
    }
    pub fn fs_debounce() -> Id {
        Id::new("settings.fs_debounce")
    }
    pub fn editor() -> Id {
        Id::new("settings.editor")
    }
    pub fn merge_tool() -> Id {
        Id::new("settings.merge_tool")
    }
    /// A Settings workspace row's own Rename — distinct from
    /// `switcher::rename()` (the switcher menu's own single trigger): this
    /// is one per workspace, in the list `view_workspaces_section` builds.
    pub fn workspace_rename(id: &WorkspaceId) -> Id {
        Id::from(format!("settings.workspace.{id}.rename"))
    }
    pub fn workspace_remove(id: &WorkspaceId) -> Id {
        Id::from(format!("settings.workspace.{id}.remove"))
    }
    pub fn add_workspace() -> Id {
        Id::new("settings.add_workspace")
    }
    pub fn advanced_toggle() -> Id {
        Id::new("settings.advanced_toggle")
    }
    pub fn fs_watch_checkbox() -> Id {
        Id::new("settings.fs_watch_checkbox")
    }
}

/// RFC-075 stage 4 §2. A History entry's own stable key is its
/// `OperationId` — already what `AppState::history_expanded` is keyed by,
/// so these ids survive a new entry arriving above an already-focused one
/// the same way.
pub mod history {
    use super::Id;
    use knotra_vcs::model::operation::OperationId;

    pub fn search() -> Id {
        Id::new("history.search")
    }
    pub fn toggle(id: &OperationId) -> Id {
        Id::from(format!("history.entry.{id}.toggle"))
    }
    pub fn copy(id: &OperationId) -> Id {
        Id::from(format!("history.entry.{id}.copy"))
    }
}
