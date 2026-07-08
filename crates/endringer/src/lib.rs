//! `endringer` — VCS abstraction layer for knotra.

pub mod error;
pub mod model;
pub mod vcs;

pub use error::EndringerError;
pub use model::{
    changelog::{ChangelogDraft, CommitEntry, ProjectCommits},
    conflict::{ConflictedFile, ConflictMarker, ProjectConflictDetail},
    operation::{
        ContextSwitchResult, FreezeOutcome, FreezeProjectResult, FreezeResult,
        FreezeValidation, FreezeValidationEntry, OperationId, OperationLog, OperationPlan,
        OperationResult, RecoveryHint, SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry,
        SmartPullProgress,
    },
    project::{Project, ProjectId},
    status::{
        ConflictStatus, ContextCandidate, ContextList, ProjectStatus, RemoteStatus,
        RepositoryIdentity, VcsContext, VcsKind, WorkingTreeStatus, WorkspaceStatus,
    },
    topology::{DependencyEdge, DependencyGraph, ImpactWarning, parse_cargo_toml},
    workspace::{Workspace, WorkspaceId},
};
pub use vcs::adapter::VcsAdapter;

#[cfg(test)]
mod tests;
