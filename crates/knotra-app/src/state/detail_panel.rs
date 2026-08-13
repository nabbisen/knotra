//! Project detail panel state (RFC-0014 identity; RFC-039 D4 recent-commits
//! cache/phase).
//!
//! `RecentCommitsPhase`/`commits_cached` mirror `conflict_ops`'s
//! `ConflictPhase`/`ConflictOpsState::cached` shape exactly (RFC-039 §4):
//! cache checked first, a background task dispatched only on a miss, the
//! cache filled and the phase advanced together on completion.

use std::collections::HashMap;

use knotra_vcs::{ProjectId, RecentCommits};

#[derive(Debug, Clone, Default)]
pub enum RecentCommitsPhase {
    #[default]
    Idle,
    /// Loading the recent-commits list for one project.
    Loading(ProjectId),
    Loaded {
        project_id: ProjectId,
        commits: RecentCommits,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DetailPanelState {
    pub open_project_id: Option<ProjectId>,
    pub commits_phase: RecentCommitsPhase,
    /// Cached per project (RFC-039 D4).
    pub commits_cached: HashMap<ProjectId, RecentCommits>,
}
