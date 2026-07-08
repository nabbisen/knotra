//! `endringer` — VCS abstraction layer for knotra.

pub mod error;
pub mod model;
pub mod vcs;

pub use error::EndringerError;
pub use model::{
    operation::{
        OperationId, OperationLog, OperationPlan, OperationResult, RecoveryHint,
        SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry, SmartPullProgress,
    },
    project::{Project, ProjectId},
    status::{
        ConflictStatus, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind,
        WorkingTreeStatus, WorkspaceStatus,
    },
    workspace::{Workspace, WorkspaceId},
};
pub use vcs::adapter::VcsAdapter;

#[cfg(test)]
mod tests;
