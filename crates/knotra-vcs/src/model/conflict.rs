//! Conflict resolution domain types.

use crate::model::project::ProjectId;
use serde::{Deserialize, Serialize};

/// Conflict state of one file in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictMarker {
    /// Both sides modified the file (`UU` in git status).
    BothModified,
    /// Deleted by one side, modified by the other (`UD` or `DU`).
    DeleteModify,
    /// Both sides added different content (`AA`).
    BothAdded,
    /// Unspecified conflict.
    Other,
}

impl std::fmt::Display for ConflictMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BothModified => write!(f, "both modified"),
            Self::DeleteModify => write!(f, "delete/modify"),
            Self::BothAdded => write!(f, "both added"),
            Self::Other => write!(f, "conflict"),
        }
    }
}

/// A single conflicted file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictedFile {
    /// Repository-relative path.
    pub path: String,
    pub marker: ConflictMarker,
}

/// Conflict state for one project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConflictDetail {
    pub project_id: ProjectId,
    pub project_name: String,
    pub conflicted_files: Vec<ConflictedFile>,
    /// Non-fatal note from the VCS layer (e.g. jj limitation).
    pub note: Option<String>,
    pub read_error: Option<String>,
}

impl ProjectConflictDetail {
    pub fn file_count(&self) -> usize {
        self.conflicted_files.len()
    }
    pub fn is_resolved(&self) -> bool {
        self.conflicted_files.is_empty() && self.read_error.is_none()
    }
}
