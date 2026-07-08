//! `VcsAdapter` — the single entry-point used by the GUI layer.
//!
//! All read operations are performed without side effects.
//! All write operations log the commands executed and return structured results.

use std::path::Path;

use crate::{
    error::Result,
    model::{
        project::{Project, ProjectId},
        status::{ProjectStatus, VcsKind, WorkspaceStatus},
        workspace::Workspace,
    },
};

use super::{git, jj};

/// Detect the VCS kind at `path` by inspecting the directory.
///
/// Returns `None` when neither a `.git` entry nor a `.jj` directory is found.
pub async fn detect_vcs_kind(path: &Path) -> Option<VcsKind> {
    // jj check first: it can overlay a .git inside a .jj workspace
    let jj_dir = path.join(".jj");
    if jj_dir.is_dir() {
        return Some(VcsKind::Jujutsu);
    }

    let git_dir = path.join(".git");
    if git_dir.exists() {
        return Some(VcsKind::Git);
    }

    None
}

/// The top-level async adapter that dispatches to Git or jj implementations.
pub struct VcsAdapter;

impl VcsAdapter {
    /// Read the current status of a single registered project.
    ///
    /// On success, returns a `ProjectStatus` snapshot.
    /// On any read failure, still returns a `ProjectStatus` with the error
    /// recorded in `read_error` so the dashboard can show a degraded card.
    pub async fn read_project_status(project: &Project) -> ProjectStatus {
        let path = std::path::Path::new(&project.path);
        let kind = detect_vcs_kind(path).await;

        match kind {
            Some(VcsKind::Git) => git::read_status(project).await,
            Some(VcsKind::Jujutsu) => jj::read_status(project).await,
            None => ProjectStatus {
                project_id: project.id.clone(),
                identity: crate::model::status::RepositoryIdentity {
                    path: project.path.clone(),
                    vcs_kind: VcsKind::Git, // placeholder
                },
                context: None,
                remote: Default::default(),
                working_tree: Default::default(),
                conflict: Default::default(),
                refreshed_at: chrono::Utc::now(),
                read_error: Some(format!(
                    "No Git or jj repository found at {}",
                    project.path
                )),
            },
        }
    }

    /// Refresh all projects in a workspace concurrently, with a parallelism cap.
    pub async fn read_workspace_status(
        workspace: &Workspace,
        max_concurrent: usize,
    ) -> WorkspaceStatus {
        use tokio::sync::Semaphore;
        use std::sync::Arc;

        let sem = Arc::new(Semaphore::new(max_concurrent));
        let mut handles = Vec::with_capacity(workspace.projects.len());

        for project in &workspace.projects {
            let project = project.clone();
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore not closed");
                VcsAdapter::read_project_status(&project).await
            }));
        }

        let mut statuses = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(status) => statuses.push(status),
                Err(e) => tracing::error!("project status task panicked: {e}"),
            }
        }

        WorkspaceStatus {
            projects: statuses,
            last_refresh: Some(chrono::Utc::now()),
        }
    }

    /// Execute `git fetch` / `jj git fetch` on a project.
    ///
    /// Returns the raw stdout / stderr / exit-code for transparency.
    pub async fn fetch(project: &Project) -> crate::model::operation::ProjectOperationResult {
        let path = std::path::Path::new(&project.path);
        let kind = detect_vcs_kind(path).await;

        match kind {
            Some(VcsKind::Git) => git::fetch(project).await,
            Some(VcsKind::Jujutsu) => jj::fetch(project).await,
            None => crate::model::operation::ProjectOperationResult {
                project_id: project.id.clone(),
                success: false,
                commands_executed: vec![],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: Some(format!("No repository at {}", project.path)),
            },
        }
    }
}
