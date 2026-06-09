//! Dashboard-specific state helpers: filtering and grouping logic.

use endringer::model::status::{ProjectStatus, WorkingTreeStatus};
use snora::theme::StatusColor;

use crate::{message::StatusFilter, state::FilterState};

/// Compute the semantic `StatusColor` for a project status.
pub fn project_status_color(status: &ProjectStatus) -> StatusColor {
    if status.read_error.is_some() {
        return StatusColor::Unknown;
    }
    if status.conflict.has_conflict {
        return StatusColor::Conflict;
    }
    if status.working_tree.is_dirty() {
        return StatusColor::Dirty;
    }
    if status.remote.behind > 0 {
        return StatusColor::Behind;
    }
    if status.remote.ahead > 0 {
        return StatusColor::Ahead;
    }
    StatusColor::Healthy
}

/// Return true when a project's status matches the active filter set.
///
/// An empty filter set means "show all".
pub fn matches_filter(status: &ProjectStatus, filter: &FilterState) -> bool {
    // Text search: project name would require the Project struct, so callers
    // that have the name should pre-filter. Here we skip text if no name.
    if !filter.status_filters.is_empty() {
        let color = project_status_color(status);
        let matches_any = filter.status_filters.iter().any(|sf| match sf {
            StatusFilter::Healthy => color == StatusColor::Healthy,
            StatusFilter::Behind => color == StatusColor::Behind,
            StatusFilter::Ahead => color == StatusColor::Ahead,
            StatusFilter::Dirty => color == StatusColor::Dirty,
            StatusFilter::Conflict => color == StatusColor::Conflict,
            StatusFilter::Error => status.read_error.is_some(),
        });
        if !matches_any {
            return false;
        }
    }
    true
}
