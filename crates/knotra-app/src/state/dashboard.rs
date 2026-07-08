//! Dashboard-specific state helpers: filtering, grouping, and status colour.

use knotra_vcs::{
    model::{project::Project, status::ProjectStatus},
    WorkspaceStatus,
};
use knotra_ui::theme::StatusColor;

use crate::{message::StatusFilter, state::FilterState};

/// Compute the semantic `StatusColor` for a project's status snapshot.
pub fn project_status_color(status: &ProjectStatus) -> StatusColor {
    if status.read_error.is_some()  { return StatusColor::Unknown; }
    if status.conflict.has_conflict { return StatusColor::Conflict; }
    if status.working_tree.is_dirty(){ return StatusColor::Dirty; }
    if status.remote.behind > 0     { return StatusColor::Behind;  }
    if status.remote.ahead  > 0     { return StatusColor::Ahead;   }
    StatusColor::Healthy
}

/// Return true when a project passes all active filters.
pub fn project_matches_filter(
    project: &Project,
    status: Option<&ProjectStatus>,
    filter: &FilterState,
) -> bool {
    // Text search on name (case-insensitive).
    if !filter.search_text.is_empty()
        && !project.name.to_lowercase().contains(&filter.search_text.to_lowercase()) {
            return false;
        }

    // Group filter.
    if let Some(ref grp) = filter.active_group
        && project.group.as_deref() != Some(grp.as_str()) {
            return false;
        }

    // Status filter chips — if any active, at least one must match.
    if !filter.status_filters.is_empty() {
        let color = status.map(project_status_color).unwrap_or(StatusColor::Unknown);
        let passes = filter.status_filters.iter().any(|sf| match sf {
            StatusFilter::Healthy  => color == StatusColor::Healthy,
            StatusFilter::Behind   => color == StatusColor::Behind,
            StatusFilter::Ahead    => color == StatusColor::Ahead,
            StatusFilter::Dirty    => color == StatusColor::Dirty,
            StatusFilter::Conflict => color == StatusColor::Conflict,
            StatusFilter::Error    => status.is_some_and(|s| s.read_error.is_some()),
        });
        if !passes { return false; }
    }

    true
}

/// A group section used by the dashboard card grid.
pub struct ProjectGroup<'a> {
    /// None means "ungrouped".
    pub name: Option<&'a str>,
    pub entries: Vec<GroupEntry<'a>>,
}

pub struct GroupEntry<'a> {
    pub project: &'a Project,
    pub status: Option<&'a ProjectStatus>,
}

/// Partition the workspace into display groups respecting the active filter.
/// Groups are stable-sorted: named groups alphabetically, ungrouped last.
pub fn build_display_groups<'a>(
    workspace_projects: &'a [Project],
    workspace_status: Option<&'a WorkspaceStatus>,
    filter: &FilterState,
) -> Vec<ProjectGroup<'a>> {
    let statuses = workspace_status.map(|ws| ws.projects.as_slice()).unwrap_or(&[]);

    // Collect (group_key, project, status) tuples that pass the filter.
    let mut named: std::collections::BTreeMap<&str, Vec<GroupEntry<'a>>> =
        std::collections::BTreeMap::new();
    let mut ungrouped: Vec<GroupEntry<'a>> = Vec::new();

    for project in workspace_projects {
        let status = statuses.iter().find(|s| s.project_id == project.id);
        if !project_matches_filter(project, status, filter) { continue; }

        let entry = GroupEntry { project, status };
        match project.group.as_deref() {
            Some(g) => named.entry(g).or_default().push(entry),
            None    => ungrouped.push(entry),
        }
    }

    let mut groups: Vec<ProjectGroup<'a>> = named
        .into_iter()
        .map(|(name, entries)| ProjectGroup { name: Some(name), entries })
        .collect();

    if !ungrouped.is_empty() {
        groups.push(ProjectGroup { name: None, entries: ungrouped });
    }

    groups
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::model::status::{
        ConflictStatus, RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus,
    };
    use knotra_vcs::ProjectId;
    use chrono::Utc;

    fn make_status(ahead: u32, behind: u32, uncommitted: u32, conflict: bool) -> ProjectStatus {
        ProjectStatus {
            project_id: ProjectId::new(),
            identity: RepositoryIdentity { path: "/tmp".into(), vcs_kind: VcsKind::Git },
            context: None,
            remote: RemoteStatus { ahead, behind, upstream: None },
            working_tree: WorkingTreeStatus { uncommitted_count: uncommitted, untracked_count: 0 },
            conflict: ConflictStatus {
                has_conflict: conflict,
                conflict_count: None,
                detection_unavailable: false,
            },
            refreshed_at: Utc::now(),
            read_error: None,
        }
    }

    #[test]
    fn status_color_priority() {
        // Conflict beats everything.
        let s = make_status(1, 1, 1, true);
        assert_eq!(project_status_color(&s), StatusColor::Conflict);

        // Dirty beats behind/ahead.
        let s = make_status(0, 1, 2, false);
        assert_eq!(project_status_color(&s), StatusColor::Dirty);

        // Behind beats ahead.
        let s = make_status(2, 1, 0, false);
        assert_eq!(project_status_color(&s), StatusColor::Behind);

        // Ahead when only ahead.
        let s = make_status(1, 0, 0, false);
        assert_eq!(project_status_color(&s), StatusColor::Ahead);

        // Healthy when everything clear.
        let s = make_status(0, 0, 0, false);
        assert_eq!(project_status_color(&s), StatusColor::Healthy);
    }

    #[test]
    fn filter_text_search() {
        let p = Project::new("api-server", "/tmp");
        let filter_match = FilterState { search_text: "api".into(), ..Default::default() };
        let filter_miss  = FilterState { search_text: "xyz".into(), ..Default::default() };
        assert!( project_matches_filter(&p, None, &filter_match));
        assert!(!project_matches_filter(&p, None, &filter_miss));
    }

    #[test]
    fn filter_status_chip() {
        let s_behind = make_status(0, 1, 0, false);
        let p = Project::new("svc", "/tmp");

        let behind_filter = FilterState {
            status_filters: vec![StatusFilter::Behind],
            ..Default::default()
        };
        assert!( project_matches_filter(&p, Some(&s_behind), &behind_filter));

        let ahead_filter = FilterState {
            status_filters: vec![StatusFilter::Ahead],
            ..Default::default()
        };
        assert!(!project_matches_filter(&p, Some(&s_behind), &ahead_filter));
    }

    #[test]
    fn grouping_separates_named_and_ungrouped() {
        let mut p1 = Project::new("svc-a", "/tmp/a");
        p1.group = Some("backend".into());
        let mut p2 = Project::new("svc-b", "/tmp/b");
        p2.group = Some("backend".into());
        let p3 = Project::new("web", "/tmp/c"); // ungrouped

        let projects = vec![p1, p2, p3];
        let groups = build_display_groups(&projects, None, &FilterState::default());

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, Some("backend"));
        assert_eq!(groups[0].entries.len(), 2);
        assert_eq!(groups[1].name, None); // ungrouped
        assert_eq!(groups[1].entries.len(), 1);
    }
}
