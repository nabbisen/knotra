//! Workspace definition types.

use serde::{Deserialize, Serialize};

use super::project::{Project, ProjectId};

/// Unique identifier for a named workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub uuid::Uuid);

impl WorkspaceId {
    pub fn new() -> Self { WorkspaceId(uuid::Uuid::new_v4()) }
}
impl Default for WorkspaceId { fn default() -> Self { Self::new() } }
impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A named collection of related repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub projects: Vec<Project>,
    pub description: Option<String>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Workspace { id: WorkspaceId::new(), name: name.into(), projects: Vec::new(), description: None }
    }
    pub fn add_project(&mut self, project: Project) { self.projects.push(project); }
    pub fn remove_project(&mut self, id: &ProjectId) { self.projects.retain(|p| &p.id != id); }
}
