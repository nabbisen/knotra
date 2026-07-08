//! Repository status types: the primary data surfaced on the dashboard.

use serde::{Deserialize, Serialize};

use super::project::ProjectId;

/// Which VCS backs this repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsKind {
    Git,
    Jujutsu,
}

impl std::fmt::Display for VcsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VcsKind::Git => write!(f, "Git"),
            VcsKind::Jujutsu => write!(f, "jj"),
        }
    }
}

/// Stable identity of a repository on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    /// Resolved absolute path to the repository root.
    pub path: String,
    /// VCS kind detected at that path.
    pub vcs_kind: VcsKind,
}

/// The user's current "where am I working" position.
///
/// For Git this is a branch name (or a detached-HEAD description).
/// For jj this is a change-id short hash plus an optional bookmark name.
/// The GUI displays the `label` string, which is always populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsContext {
    /// Human-readable label shown in the dashboard card.
    pub label: String,
    /// Git branch name, if applicable.
    pub branch: Option<String>,
    /// jj change-id (short), if applicable.
    pub jj_change_id: Option<String>,
    /// jj bookmark name, if applicable.
    pub jj_bookmark: Option<String>,
    /// True when HEAD is detached (Git) or the workcopy has no description (jj).
    pub is_detached: bool,
}

/// Ahead / Behind relative to the upstream remote.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteStatus {
    /// Commits local has that the remote does not.
    pub ahead: u32,
    /// Commits the remote has that local does not.
    pub behind: u32,
    /// Name of the tracked remote, e.g. `origin/main`.
    pub upstream: Option<String>,
}

/// Whether the working tree has uncommitted or untracked content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkingTreeStatus {
    /// Number of modified / staged / deleted tracked files.
    pub uncommitted_count: u32,
    /// Number of untracked files (not git-ignored).
    pub untracked_count: u32,
}

impl WorkingTreeStatus {
    pub fn is_dirty(&self) -> bool {
        self.uncommitted_count > 0 || self.untracked_count > 0
    }
}

/// Merge / rebase conflict state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictStatus {
    /// True when the repository is in a conflicted state.
    pub has_conflict: bool,
    /// Approximate number of conflicted files, if determinable.
    pub conflict_count: Option<u32>,
}

/// Aggregate health of one repository, as displayed in a dashboard card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub project_id: ProjectId,
    pub identity: RepositoryIdentity,
    pub context: Option<VcsContext>,
    pub remote: RemoteStatus,
    pub working_tree: WorkingTreeStatus,
    pub conflict: ConflictStatus,
    /// Wall-clock time this status snapshot was produced.
    pub refreshed_at: chrono::DateTime<chrono::Utc>,
    /// Short description of the last read error, if the repository could not
    /// be read successfully.
    pub read_error: Option<String>,
}

impl ProjectStatus {
    /// Convenience: is the repository in any kind of unhealthy state?
    pub fn is_healthy(&self) -> bool {
        self.read_error.is_none()
            && !self.conflict.has_conflict
            && !self.working_tree.is_dirty()
            && self.remote.behind == 0
    }

    /// True when there is any local-only content not yet pushed.
    pub fn is_ahead(&self) -> bool {
        self.remote.ahead > 0
    }

    /// True when the remote has commits not yet merged locally.
    pub fn is_behind(&self) -> bool {
        self.remote.behind > 0
    }
}

/// Composite state across an entire workspace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// Ordered list of per-project statuses.
    pub projects: Vec<ProjectStatus>,
    /// Timestamp of the most recent workspace-wide refresh.
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// Context listing (branches / change-sets available to switch to)
// ---------------------------------------------------------------------------

/// One switchable context candidate for a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCandidate {
    /// Short human-readable label (branch name, jj change-id + description).
    pub label: String,
    /// Full ref string used as the switch target (e.g. `refs/heads/main`).
    pub target: String,
    /// True when this is the currently active context.
    pub is_current: bool,
    /// True when the candidate is a remote-tracking ref (not locally checked out).
    pub is_remote: bool,
}

/// All context candidates for one repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextList {
    pub project_id: crate::model::project::ProjectId,
    pub vcs_kind: VcsKind,
    pub candidates: Vec<ContextCandidate>,
    /// Non-fatal warning produced during listing (e.g. detached HEAD).
    pub warning: Option<String>,
}

impl Default for ContextList {
    fn default() -> Self {
        ContextList {
            project_id: crate::model::project::ProjectId::new(),
            vcs_kind: VcsKind::Git,
            candidates: Vec::new(),
            warning: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Stash and worktree detail types (new in Phase migration)
// ---------------------------------------------------------------------------

/// A single stash entry (maps to endringer-backend StashEntry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index:   usize,
    pub message: String,
}
