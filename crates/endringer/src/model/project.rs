//! Project (repository registration) types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique, stable identifier for a registered project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new() -> Self {
        ProjectId(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A registered repository entry within a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Stable identifier, generated on first registration.
    pub id: ProjectId,
    /// User-visible display name.
    pub name: String,
    /// Absolute path to the repository root on disk.
    pub path: String,
    /// Optional user-assigned tags for filtering / grouping.
    pub tags: Vec<String>,
    /// Optional group assignment (e.g. case/project name).
    pub group: Option<String>,
    /// Whether to include this project in bulk operations by default.
    pub include_in_bulk: bool,
}

impl Project {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Project {
            id: ProjectId::new(),
            name: name.into(),
            path: path.into(),
            tags: Vec::new(),
            group: None,
            include_in_bulk: true,
        }
    }
}
