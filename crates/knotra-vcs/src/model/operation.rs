//! Operation planning, execution result, and audit-log types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::project::ProjectId;

/// Unique identifier for a logged operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(pub Uuid);

impl OperationId {
    pub fn new() -> Self {
        OperationId(Uuid::new_v4())
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// High-level kind of operation that can be planned and executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    /// Background or manual status refresh.
    StatusRefresh,
    /// `git fetch` / `jj git fetch` for one or more repositories.
    Fetch,
    /// Safe pull including dirty-state detection and optional stash.
    SmartPull,
    /// Branch / context switch for a single repository.
    ContextSwitch,
    /// Atomic freeze: tag / bookmark creation across repositories.
    Freeze,
    /// Rollback of a partial freeze operation.
    FreezeRollback,
}

impl std::fmt::Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationKind::StatusRefresh => write!(f, "Status Refresh"),
            OperationKind::Fetch => write!(f, "Fetch"),
            OperationKind::SmartPull => write!(f, "Smart Pull"),
            OperationKind::ContextSwitch => write!(f, "Context Switch"),
            OperationKind::Freeze => write!(f, "Freeze"),
            OperationKind::FreezeRollback => write!(f, "Freeze Rollback"),
        }
    }
}

/// Planned operation before execution; shown to the user for confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPlan {
    pub id: OperationId,
    pub kind: OperationKind,
    /// Projects included in this operation, in intended execution order.
    pub target_projects: Vec<ProjectId>,
    /// Human-readable description of what will happen.
    pub description: String,
    /// Potential risks or side effects the user should be aware of.
    pub risks: Vec<String>,
}

/// Per-project outcome within a larger operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOperationResult {
    pub project_id: ProjectId,
    pub success: bool,
    /// The VCS command(s) that were executed, for transparency.
    pub commands_executed: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
}

/// Aggregate result for an entire operation across all target projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation_id: OperationId,
    pub kind: OperationKind,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub per_project: Vec<ProjectOperationResult>,
    pub rollback_attempted: bool,
    pub rollback_succeeded: Option<bool>,
}

impl OperationResult {
    pub fn all_succeeded(&self) -> bool {
        self.per_project.iter().all(|r| r.success)
    }

    pub fn any_failed(&self) -> bool {
        self.per_project.iter().any(|r| !r.success)
    }

    pub fn failed_projects(&self) -> Vec<&ProjectOperationResult> {
        self.per_project.iter().filter(|r| !r.success).collect()
    }

    pub fn successful_projects(&self) -> Vec<&ProjectOperationResult> {
        self.per_project.iter().filter(|r| r.success).collect()
    }
}

/// Hint presented to the user when manual recovery is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHint {
    pub project_id: ProjectId,
    /// Plain-language description of the situation.
    pub situation: String,
    /// One or more shell commands the user can run to recover.
    pub suggested_commands: Vec<String>,
    /// Documentation link or further reading, if available.
    pub see_also: Option<String>,
}

/// Persisted audit record for one completed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub result: OperationResult,
    pub recovery_hints: Vec<RecoveryHint>,
}

// ---------------------------------------------------------------------------
// Smart Pull plan types
// ---------------------------------------------------------------------------

/// Disposition for one project in a Smart Pull plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartPullDisposition {
    /// Clean project: fetch + ff-merge.
    Pull,
    /// Dirty project, user chose to stash, merge, then pop.
    StashAndPull,
    /// Dirty project, user chose to fetch only (merge skipped).
    FetchOnly,
    /// Project excluded from this run entirely.
    Excluded,
}

/// Pre-execution plan for a Smart Pull operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPullPlan {
    pub id: OperationId,
    /// Per-project dispositions, in execution order.
    pub entries: Vec<SmartPullPlanEntry>,
}

/// One entry in a `SmartPullPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPullPlanEntry {
    pub project_id: ProjectId,
    pub project_name: String,
    pub is_dirty: bool,
    pub has_conflict: bool,
    pub disposition: SmartPullDisposition,
}

impl SmartPullPlan {
    /// True when no project will undergo a merge (nothing to do).
    pub fn is_noop(&self) -> bool {
        self.entries.iter().all(|e| {
            matches!(
                e.disposition,
                SmartPullDisposition::FetchOnly | SmartPullDisposition::Excluded
            )
        })
    }

    pub fn pull_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| {
                matches!(
                    e.disposition,
                    SmartPullDisposition::Pull | SmartPullDisposition::StashAndPull
                )
            })
            .count()
    }

    pub fn excluded_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.disposition == SmartPullDisposition::Excluded)
            .count()
    }
}

/// Progress event emitted during Smart Pull execution (one per project).
#[derive(Debug, Clone)]
pub struct SmartPullProgress {
    pub project_id: ProjectId,
    pub project_name: String,
    pub result: ProjectOperationResult,
    pub recovery_hint: Option<RecoveryHint>,
}

// ---------------------------------------------------------------------------
// Context switch result
// ---------------------------------------------------------------------------

/// Result of a single context-switch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSwitchResult {
    pub project_id: ProjectId,
    pub project_name: String,
    /// The target context the switch was attempted to.
    pub target: String,
    pub operation_result: ProjectOperationResult,
    pub recovery_hint: Option<RecoveryHint>,
}

// ---------------------------------------------------------------------------
// Freezer (static-point creation) types
// ---------------------------------------------------------------------------

/// Per-project pre-execution validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeValidationEntry {
    pub project_id: ProjectId,
    pub project_name: String,
    /// True when the project will be included in the freeze.
    pub included: bool,
    /// True when the working tree is clean and there are no conflicts.
    pub is_clean: bool,
    /// True when a tag/bookmark with the freeze name already exists.
    pub tag_exists: bool,
    /// Non-fatal diagnostic notes for the user (e.g. "Ahead by 2").
    pub notes: Vec<String>,
    /// Reasons the project cannot be frozen (blocks execution).
    pub blockers: Vec<String>,
}

impl FreezeValidationEntry {
    /// True when this project blocks execution.
    pub fn is_blocked(&self) -> bool {
        !self.blockers.is_empty()
    }

    /// True when this project can be frozen.
    pub fn ready(&self) -> bool {
        self.included && !self.is_blocked()
    }
}

/// Pre-execution validation across all selected projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeValidation {
    pub freeze_name: String,
    pub entries: Vec<FreezeValidationEntry>,
}

impl FreezeValidation {
    /// True when every included project is ready.
    pub fn all_ready(&self) -> bool {
        self.entries
            .iter()
            .filter(|e| e.included)
            .all(|e| !e.is_blocked())
    }

    pub fn blocked_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.included && e.is_blocked())
            .count()
    }

    pub fn ready_count(&self) -> usize {
        self.entries.iter().filter(|e| e.ready()).count()
    }
}

/// Outcome of freezing one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeProjectResult {
    pub project_id: ProjectId,
    pub project_name: String,
    pub success: bool,
    pub commands_executed: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    /// True when a rollback was attempted for this project.
    pub rollback_attempted: bool,
    /// True when the rollback succeeded (only meaningful when `rollback_attempted`).
    pub rollback_succeeded: Option<bool>,
    pub recovery_hint: Option<RecoveryHint>,
}

/// Aggregate result for a complete freeze operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeResult {
    pub freeze_name: String,
    pub project_results: Vec<FreezeProjectResult>,
    /// Overall outcome.
    pub outcome: FreezeOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreezeOutcome {
    /// All projects tagged successfully.
    Success,
    /// Some projects failed; rollback succeeded for all that had been tagged.
    RolledBack,
    /// Some projects failed and rollback itself partially or fully failed.
    RollbackFailed,
    /// All projects excluded or nothing to do.
    NothingDone,
}

impl FreezeResult {
    pub fn success_count(&self) -> usize {
        self.project_results.iter().filter(|r| r.success).count()
    }
    pub fn failed_count(&self) -> usize {
        self.project_results.iter().filter(|r| !r.success).count()
    }
    pub fn rollback_partial_failure(&self) -> bool {
        self.project_results
            .iter()
            .any(|r| r.rollback_attempted && r.rollback_succeeded == Some(false))
    }
    pub fn recovery_hints(&self) -> Vec<&RecoveryHint> {
        self.project_results
            .iter()
            .filter_map(|r| r.recovery_hint.as_ref())
            .collect()
    }
}
