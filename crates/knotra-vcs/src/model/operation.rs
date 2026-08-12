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

/// Explicit per-project outcome within a larger operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectOperationOutcome {
    Succeeded,
    Failed,
    Skipped,
}

/// Stable audit reason for a project excluded from an Activity retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryExclusionReason {
    NotInActiveWorkspace,
    ProjectPathMissing,
    UnsupportedRepository,
    StatusUnavailable,
}

impl RetryExclusionReason {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotInActiveWorkspace => "retry:not_in_active_workspace",
            Self::ProjectPathMissing => "retry:project_path_missing",
            Self::UnsupportedRepository => "retry:unsupported_repository",
            Self::StatusUnavailable => "retry:status_unavailable",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "retry:not_in_active_workspace" => Some(Self::NotInActiveWorkspace),
            "retry:project_path_missing" => Some(Self::ProjectPathMissing),
            "retry:unsupported_repository" => Some(Self::UnsupportedRepository),
            "retry:status_unavailable" => Some(Self::StatusUnavailable),
            _ => None,
        }
    }

    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::NotInActiveWorkspace => "plain.activity.excluded_workspace",
            Self::ProjectPathMissing => "plain.activity.excluded_missing",
            Self::UnsupportedRepository => "plain.activity.excluded_unsupported",
            Self::StatusUnavailable => "plain.activity.excluded_status",
        }
    }
}

impl ProjectOperationOutcome {
    pub fn from_success(success: bool) -> Self {
        if success {
            Self::Succeeded
        } else {
            Self::Failed
        }
    }
}

/// Per-project outcome within a larger operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOperationResult {
    pub project_id: ProjectId,
    #[serde(default = "ProjectOperationResult::default_outcome")]
    pub outcome: ProjectOperationOutcome,
    pub success: bool,
    /// RFC-046 D1: a stable [`RetryExclusionReason`] code (`.code()`),
    /// **never rendered UI text**. This value is serialised to disk and
    /// reloaded at startup, so it outlives the locale — and the knotra
    /// version — that produced it; a rendered sentence baked in here would
    /// be permanent, wrong-language history a user cannot fix by changing
    /// their locale later. Readers map a code back to display text through
    /// the catalog at render time (`view.rs`'s `skip_reason_display`),
    /// falling back to the stored value verbatim for a value that predates
    /// this contract (RFC-046 D4) — deliberate forward/backward
    /// compatibility with logs written before this field's contract was
    /// enforced, not an oversight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    /// The VCS command(s) that were executed, for transparency.
    pub commands_executed: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
}

impl ProjectOperationResult {
    fn default_outcome() -> ProjectOperationOutcome {
        ProjectOperationOutcome::Succeeded
    }

    /// Normalize logs written before explicit outcomes existed.
    pub fn effective_outcome(&self) -> ProjectOperationOutcome {
        if self.outcome == ProjectOperationOutcome::Succeeded && !self.success {
            ProjectOperationOutcome::Failed
        } else {
            self.outcome.clone()
        }
    }

    pub fn is_succeeded(&self) -> bool {
        self.effective_outcome() == ProjectOperationOutcome::Succeeded
    }

    pub fn is_failed(&self) -> bool {
        self.effective_outcome() == ProjectOperationOutcome::Failed
    }

    pub fn is_skipped(&self) -> bool {
        self.effective_outcome() == ProjectOperationOutcome::Skipped
    }
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
        self.per_project.iter().all(|r| r.is_succeeded())
    }

    pub fn any_failed(&self) -> bool {
        self.per_project.iter().any(|r| r.is_failed())
    }

    pub fn failed_projects(&self) -> Vec<&ProjectOperationResult> {
        self.per_project.iter().filter(|r| r.is_failed()).collect()
    }

    pub fn successful_projects(&self) -> Vec<&ProjectOperationResult> {
        self.per_project
            .iter()
            .filter(|r| r.is_succeeded())
            .collect()
    }

    pub fn skipped_projects(&self) -> Vec<&ProjectOperationResult> {
        self.per_project.iter().filter(|r| r.is_skipped()).collect()
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

/// Reason an entry is skipped before Smart Pull execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartPullSkipReason {
    Deselected,
    NoUpstream,
    Conflict,
    MissingStatus,
    ProjectNotFound,
}

impl SmartPullSkipReason {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            SmartPullSkipReason::Deselected => "plain.get_latest.note_not_selected",
            SmartPullSkipReason::NoUpstream => "plain.get_latest.note_no_upstream",
            SmartPullSkipReason::Conflict => "plain.get_latest.note_needs_choice",
            SmartPullSkipReason::MissingStatus => "plain.get_latest.note_status_missing",
            SmartPullSkipReason::ProjectNotFound => "plain.get_latest.note_project_not_found",
        }
    }
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
    pub skip_reason: Option<SmartPullSkipReason>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_exclusion_code_survives_operation_log_json_round_trip() {
        let reason = RetryExclusionReason::ProjectPathMissing;
        let now = Utc::now();
        let log = OperationLog {
            result: OperationResult {
                operation_id: OperationId::new(),
                kind: OperationKind::Fetch,
                started_at: now,
                finished_at: now,
                per_project: vec![ProjectOperationResult {
                    project_id: ProjectId::new(),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some(reason.code().to_owned()),
                    commands_executed: Vec::new(),
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    error_message: None,
                }],
                rollback_attempted: false,
                rollback_succeeded: None,
            },
            recovery_hints: Vec::new(),
        };

        let json = serde_json::to_string(&log).expect("serialize operation log");
        let decoded: OperationLog = serde_json::from_str(&json).expect("deserialize operation log");
        assert_eq!(
            decoded.result.per_project[0]
                .skip_reason
                .as_deref()
                .and_then(RetryExclusionReason::from_code),
            Some(reason)
        );
    }
}
