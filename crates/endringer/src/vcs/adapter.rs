//! `VcsAdapter` — the single entry-point used by the GUI layer.

use std::path::Path;

use crate::{
    model::{
        operation::{ProjectOperationResult, RecoveryHint},
        project::Project,
        status::{ProjectStatus, VcsKind, WorkspaceStatus},
        workspace::Workspace,
    },
};

use super::{git, jj};

// ---------------------------------------------------------------------------
// VCS detection
// ---------------------------------------------------------------------------

pub async fn detect_vcs_kind(path: &Path) -> Option<VcsKind> {
    if path.join(".jj").is_dir()  { return Some(VcsKind::Jujutsu); }
    if path.join(".git").exists() { return Some(VcsKind::Git); }
    None
}

// ---------------------------------------------------------------------------
// VcsAdapter
// ---------------------------------------------------------------------------

pub struct VcsAdapter;

impl VcsAdapter {
    // --- Read ---

    pub async fn read_project_status(project: &Project) -> ProjectStatus {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::read_status(project).await,
            Some(VcsKind::Jujutsu) => jj::read_status(project).await,
            None => ProjectStatus {
                project_id: project.id.clone(),
                identity: crate::model::status::RepositoryIdentity {
                    path: project.path.clone(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: Default::default(),
                working_tree: Default::default(),
                conflict: Default::default(),
                refreshed_at: chrono::Utc::now(),
                read_error: Some(format!("no Git or jj repository at {}", project.path)),
            },
        }
    }

    pub async fn read_workspace_status(workspace: &Workspace, max_concurrent: usize) -> WorkspaceStatus {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let sem = Arc::new(Semaphore::new(max_concurrent));
        let mut handles = Vec::with_capacity(workspace.projects.len());

        for project in &workspace.projects {
            let project = project.clone();
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore open");
                VcsAdapter::read_project_status(&project).await
            }));
        }

        let mut statuses = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(s)  => statuses.push(s),
                Err(e) => tracing::error!("project status task panicked: {e}"),
            }
        }

        WorkspaceStatus { projects: statuses, last_refresh: Some(chrono::Utc::now()) }
    }

    // --- Write: fetch ---

    pub async fn fetch(project: &Project) -> ProjectOperationResult {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::fetch(project).await,
            Some(VcsKind::Jujutsu) => jj::fetch(project).await,
            None => ProjectOperationResult {
                project_id: project.id.clone(),
                success: false,
                commands_executed: vec![],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: Some(format!("no repository at {}", project.path)),
            },
        }
    }

    // --- Write: smart pull ---

    /// Execute a Smart Pull for one project.
    ///
    /// Returns the operation result and an optional recovery hint.
    pub async fn smart_pull(
        project: &Project,
        stash_dirty: bool,
    ) -> (ProjectOperationResult, Option<RecoveryHint>) {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::smart_pull(project, stash_dirty).await,
            Some(VcsKind::Jujutsu) => jj::smart_pull(project, stash_dirty).await,
            None => (
                ProjectOperationResult {
                    project_id: project.id.clone(),
                    success: false,
                    commands_executed: vec![],
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    error_message: Some(format!("no repository at {}", project.path)),
                },
                None,
            ),
        }
    }
}

impl VcsAdapter {
    // --- Context listing ---

    /// List all switchable context candidates for a project (branches / change-sets).
    pub async fn list_contexts(project: &Project) -> crate::model::status::ContextList {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::list_contexts(project).await,
            Some(VcsKind::Jujutsu) => jj::list_contexts(project).await,
            None => crate::model::status::ContextList {
                project_id: project.id.clone(),
                vcs_kind: VcsKind::Git,
                candidates: Vec::new(),
                warning: Some(format!("no repository at {}", project.path)),
            },
        }
    }

    // --- Context switch ---

    /// Switch the working context of a repository.
    pub async fn switch_context(
        project: &Project,
        target: &str,
    ) -> (
        crate::model::operation::ProjectOperationResult,
        Option<crate::model::operation::RecoveryHint>,
    ) {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::switch_context(project, target).await,
            Some(VcsKind::Jujutsu) => jj::switch_context(project, target).await,
            None => (
                crate::model::operation::ProjectOperationResult {
                    project_id: project.id.clone(),
                    success: false,
                    commands_executed: vec![],
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    error_message: Some(format!("no repository at {}", project.path)),
                },
                None,
            ),
        }
    }
}
