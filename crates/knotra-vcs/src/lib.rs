//! `knotra-vcs` — VCS facade for knotra: multi-project `VcsAdapter` + domain model over the published `endringer` crates.

pub mod error;
pub mod model;
pub mod vcs;
pub mod watcher;

pub use endringer_core::types::WorktreeStatus as BackendWorktreeStatus;
pub use error::EndringerError;
pub use model::topology::parse_cargo_toml;
pub use model::{
    changelog::{ChangelogDraft, CommitEntry, ProjectCommits, RecentCommits},
    conflict::{ConflictMarker, ConflictedFile, ProjectConflictDetail},
    operation::{
        ContextSwitchResult, FreezeOutcome, FreezeProjectResult, FreezeResult, FreezeValidation,
        FreezeValidationEntry, OperationId, OperationLog, OperationPlan, OperationResult,
        RecoveryHint, SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry, SmartPullProgress,
    },
    project::{Project, ProjectId},
    status::{
        ConflictStatus, ContextCandidate, ContextList, ContextTarget, ProjectStatus, RemoteStatus,
        RepositoryIdentity, VcsContext, VcsKind, WorkingTreeStatus, WorkspaceStatus,
    },
    topology::{DependencyEdge, DependencyGraph, ImpactWarning},
    workspace::{Workspace, WorkspaceId},
};
pub use vcs::adapter::VcsAdapter;
pub use watcher::{FsChangeEvent, FsPoller};

#[cfg(test)]
mod tests;
