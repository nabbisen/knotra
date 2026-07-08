//! Sync Center UI state.

use endringer::{
    model::operation::{
        RecoveryHint, SmartPullDisposition, SmartPullPlan,
        SmartPullPlanEntry, SmartPullProgress,
    },
    ProjectId, WorkspaceStatus,
};



// ---------------------------------------------------------------------------
// Sync Center phase
// ---------------------------------------------------------------------------

/// The current phase of the Sync Center workflow.
#[derive(Debug, Clone, Default)]
pub enum SyncPhase {
    /// Idle — waiting for the user to select an operation.
    #[default]
    Idle,
    /// A bulk fetch is running.
    FetchRunning { total: usize, done: usize },
    /// Computing the Smart Pull plan (checking dirty states).
    Planning,
    /// Plan ready — awaiting user confirmation.
    AwaitingConfirm(SmartPullPlan),
    /// Smart Pull in progress — collecting streaming results.
    PullRunning {
        plan: SmartPullPlan,
        completed: Vec<SmartPullProgress>,
    },
    /// Operation finished — show results.
    Done(SyncResult),
}

/// Aggregate result shown after an operation completes.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub kind: SyncKind,
    pub per_project: Vec<ProjectOutcome>,
    pub recovery_hints: Vec<RecoveryHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncKind {
    Fetch,
    SmartPull,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectOutcome {
    pub project_id: ProjectId,
    pub project_name: String,
    pub success: bool,
    pub commands_executed: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub log_expanded: bool,
}

impl SyncResult {
    pub fn success_count(&self) -> usize { self.per_project.iter().filter(|p| p.success).count() }
    pub fn fail_count(&self)    -> usize { self.per_project.iter().filter(|p| !p.success).count() }
    pub fn all_succeeded(&self) -> bool  { self.fail_count() == 0 }
}

// ---------------------------------------------------------------------------
// Sync Center state
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct SyncCenterState {
    /// Per-project inclusion toggle (true = included).
    pub project_selection: std::collections::HashMap<ProjectId, bool>,
    /// Disposition overrides set by the user in the plan view.
    pub disposition_overrides: std::collections::HashMap<ProjectId, SmartPullDisposition>,
    pub phase: SyncPhase,
}

impl SyncCenterState {
    /// Initialise selection from workspace projects (all included by default).
    pub fn init_selection(&mut self, projects: &[endringer::Project]) {
        for p in projects {
            self.project_selection.entry(p.id.clone()).or_insert(true);
        }
        // Remove stale entries.
        let ids: std::collections::HashSet<_> = projects.iter().map(|p| &p.id).collect();
        self.project_selection.retain(|id, _| ids.contains(id));
    }

    pub fn is_selected(&self, id: &ProjectId) -> bool {
        *self.project_selection.get(id).unwrap_or(&true)
    }

    pub fn selected_ids(&self) -> Vec<ProjectId> {
        self.project_selection
            .iter()
            .filter(|(_, v)| **v)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Build a Smart Pull plan from the current workspace status + user selections.
    pub fn build_plan(
        &self,
        projects: &[endringer::Project],
        workspace_status: Option<&WorkspaceStatus>,
    ) -> SmartPullPlan {
        use endringer::OperationId;

        let statuses = workspace_status
            .map(|ws| ws.projects.as_slice())
            .unwrap_or(&[]);

        let entries: Vec<SmartPullPlanEntry> = projects
            .iter()
            .map(|p| {
                let selected = self.is_selected(&p.id);
                let status = statuses.iter().find(|s| s.project_id == p.id);
                let is_dirty    = status.map(|s| s.working_tree.is_dirty()).unwrap_or(false);
                let has_conflict= status.map(|s| s.conflict.has_conflict).unwrap_or(false);

                // Default disposition.
                let disposition = if !selected {
                    SmartPullDisposition::Excluded
                } else if let Some(d) = self.disposition_overrides.get(&p.id) {
                    d.clone()
                } else if has_conflict {
                    SmartPullDisposition::Excluded // never auto-pull conflicted repos
                } else if is_dirty {
                    SmartPullDisposition::FetchOnly // conservative default for dirty
                } else {
                    SmartPullDisposition::Pull
                };

                SmartPullPlanEntry {
                    project_id: p.id.clone(),
                    project_name: p.name.clone(),
                    is_dirty,
                    has_conflict,
                    disposition,
                }
            })
            .collect();

        SmartPullPlan { id: OperationId::new(), entries }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use endringer::{
        model::status::{
            ConflictStatus, RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus,
        },
        ProjectId, WorkspaceStatus,
    };
    use chrono::Utc;

    fn make_project(name: &str) -> endringer::Project {
        endringer::Project::new(name, "/tmp")
    }

    fn make_status(
        project_id: ProjectId,
        uncommitted: u32,
        conflict: bool,
    ) -> endringer::ProjectStatus {
        endringer::ProjectStatus {
            project_id,
            identity: RepositoryIdentity { path: "/tmp".into(), vcs_kind: VcsKind::Git },
            context: None,
            remote: RemoteStatus::default(),
            working_tree: WorkingTreeStatus { uncommitted_count: uncommitted, untracked_count: 0 },
            conflict: ConflictStatus { has_conflict: conflict, conflict_count: None },
            refreshed_at: Utc::now(),
            read_error: None,
        }
    }

    #[test]
    fn clean_project_gets_pull_disposition() {
        let p = make_project("svc");
        let status = make_status(p.id.clone(), 0, false);
        let ws = WorkspaceStatus { projects: vec![status], last_refresh: None };

        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p.clone()]);
        let plan = sc.build_plan(&[p.clone()], Some(&ws));

        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::Pull);
    }

    #[test]
    fn dirty_project_defaults_to_fetch_only() {
        let p = make_project("svc");
        let status = make_status(p.id.clone(), 3, false);
        let ws = WorkspaceStatus { projects: vec![status], last_refresh: None };

        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p.clone()]);
        let plan = sc.build_plan(&[p.clone()], Some(&ws));

        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::FetchOnly);
    }

    #[test]
    fn conflicted_project_is_excluded() {
        let p = make_project("svc");
        let status = make_status(p.id.clone(), 0, true);
        let ws = WorkspaceStatus { projects: vec![status], last_refresh: None };

        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p.clone()]);
        let plan = sc.build_plan(&[p.clone()], Some(&ws));

        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::Excluded);
    }

    #[test]
    fn deselected_project_is_excluded() {
        let p = make_project("svc");
        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p.clone()]);
        sc.project_selection.insert(p.id.clone(), false);

        let plan = sc.build_plan(&[p.clone()], None);
        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::Excluded);
    }

    #[test]
    fn user_override_stash_and_pull() {
        let p = make_project("svc");
        let status = make_status(p.id.clone(), 2, false);
        let ws = WorkspaceStatus { projects: vec![status], last_refresh: None };

        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p.clone()]);
        sc.disposition_overrides
            .insert(p.id.clone(), SmartPullDisposition::StashAndPull);

        let plan = sc.build_plan(&[p.clone()], Some(&ws));
        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::StashAndPull);
    }

    #[test]
    fn plan_pull_count() {
        let p1 = make_project("a");
        let p2 = make_project("b");
        let p3 = make_project("c");

        let s1 = make_status(p1.id.clone(), 0, false);
        let s2 = make_status(p2.id.clone(), 1, false); // dirty → FetchOnly
        let s3 = make_status(p3.id.clone(), 0, true);  // conflict → Excluded
        let ws = WorkspaceStatus { projects: vec![s1, s2, s3], last_refresh: None };

        let mut sc = SyncCenterState::default();
        sc.init_selection(&[p1.clone(), p2.clone(), p3.clone()]);
        let plan = sc.build_plan(&[p1, p2, p3], Some(&ws));

        assert_eq!(plan.pull_count(),     1);
        assert_eq!(plan.excluded_count(), 1);
    }
}
