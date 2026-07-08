//! Conflict resolution UI state.

use endringer::{ProjectConflictDetail, ProjectId};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub enum ConflictPhase {
    #[default]
    Idle,
    /// Loading conflict file list for one project.
    Loading(ProjectId),
    /// Showing file list + actions for one project.
    Browsing {
        project_id: ProjectId,
        detail: ProjectConflictDetail,
    },
    /// A mark-resolved or abort operation is in progress.
    Operating { project_id: ProjectId, action: String },
    /// Show the operation result.
    Done { project_id: ProjectId, success: bool, message: String },
}

#[derive(Debug, Default)]
pub struct ConflictOpsState {
    pub phase: ConflictPhase,
    /// Cached detail per project (invalidated on re-check).
    pub cached: HashMap<ProjectId, ProjectConflictDetail>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use endringer::{ConflictedFile, ConflictMarker, ProjectConflictDetail, ProjectId};

    #[test]
    fn conflict_detail_resolved_when_no_files() {
        let detail = ProjectConflictDetail {
            project_id: ProjectId::new(),
            project_name: "svc".to_owned(),
            conflicted_files: vec![],
            note: None,
            read_error: None,
        };
        assert!(detail.is_resolved());
    }

    #[test]
    fn conflict_detail_not_resolved_with_files() {
        let detail = ProjectConflictDetail {
            project_id: ProjectId::new(),
            project_name: "svc".to_owned(),
            conflicted_files: vec![ConflictedFile {
                path: "src/lib.rs".to_owned(),
                marker: ConflictMarker::BothModified,
            }],
            note: None,
            read_error: None,
        };
        assert!(!detail.is_resolved());
        assert_eq!(detail.file_count(), 1);
    }
}
