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

impl VcsAdapter {
    // --- Freezer: validate ---

    /// Validate all selected projects before a freeze.
    ///
    /// Runs all validations concurrently with the standard semaphore cap.
    pub async fn validate_freeze(
        projects: &[crate::model::project::Project],
        selection: &std::collections::HashSet<crate::model::project::ProjectId>,
        freeze_name: &str,
        max_concurrent: usize,
    ) -> crate::model::operation::FreezeValidation {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let sem  = Arc::new(Semaphore::new(max_concurrent));
        let name = freeze_name.to_owned();
        let mut handles = Vec::new();

        for project in projects {
            let project  = project.clone();
            let sem      = Arc::clone(&sem);
            let name     = name.clone();
            let included = selection.contains(&project.id);

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("open");
                let kind = detect_vcs_kind(std::path::Path::new(&project.path)).await;
                match kind {
                    Some(VcsKind::Git)      => git::validate_for_freeze(&project, &name, included).await,
                    Some(VcsKind::Jujutsu) => jj::validate_for_freeze(&project, &name, included).await,
                    None => crate::model::operation::FreezeValidationEntry {
                        project_id:   project.id.clone(),
                        project_name: project.name.clone(),
                        included,
                        is_clean: false,
                        tag_exists: false,
                        notes: Vec::new(),
                        blockers: vec![format!("no repository at {}", project.path)],
                    },
                }
            }));
        }

        let mut entries = Vec::with_capacity(handles.len());
        for h in handles {
            if let Ok(e) = h.await { entries.push(e); }
        }

        crate::model::operation::FreezeValidation {
            freeze_name: freeze_name.to_owned(),
            entries,
        }
    }

    // --- Freezer: execute (with rollback) ---

    /// Execute the freeze: tag/bookmark creation with automatic rollback on failure.
    ///
    /// Projects are processed in order. If any fails, all previously tagged
    /// projects are rolled back. Rollback failures are recorded but do not
    /// prevent subsequent rollback attempts.
    pub async fn execute_freeze(
        projects: &[crate::model::project::Project],
        validation: &crate::model::operation::FreezeValidation,
    ) -> crate::model::operation::FreezeResult {
        use crate::model::operation::{
            FreezeOutcome, FreezeProjectResult, FreezeResult, RecoveryHint,
        };

        let freeze_name = &validation.freeze_name;

        // Only process projects that are included and not blocked.
        let ready_entries: Vec<_> = validation
            .entries
            .iter()
            .filter(|e| e.ready())
            .collect();

        if ready_entries.is_empty() {
            return FreezeResult {
                freeze_name: freeze_name.clone(),
                project_results: Vec::new(),
                outcome: FreezeOutcome::NothingDone,
            };
        }

        let project_map: std::collections::HashMap<_, _> = projects
            .iter()
            .map(|p| (p.id.clone(), p))
            .collect();

        let mut completed: Vec<FreezeProjectResult> = Vec::new();
        let mut failed    = false;

        // --- Execute in order ---
        for entry in &ready_entries {
            let project = match project_map.get(&entry.project_id) {
                Some(p) => *p,
                None    => {
                    failed = true;
                    completed.push(FreezeProjectResult {
                        project_id:         entry.project_id.clone(),
                        project_name:       entry.project_name.clone(),
                        success:            false,
                        commands_executed:  vec![],
                        stdout:             String::new(),
                        stderr:             String::new(),
                        rollback_attempted: false,
                        rollback_succeeded: None,
                        recovery_hint:      None,
                    });
                    break;
                }
            };

            let kind = detect_vcs_kind(std::path::Path::new(&project.path)).await;
            let result = match kind {
                Some(VcsKind::Git) =>
                    git::tag_create(project, freeze_name).await,
                Some(VcsKind::Jujutsu) =>
                    jj::bookmark_create(project, freeze_name).await,
                None => crate::model::operation::ProjectOperationResult {
                    project_id:        project.id.clone(),
                    success:           false,
                    commands_executed: vec![],
                    stdout:            String::new(),
                    stderr:            String::new(),
                    exit_code:         None,
                    error_message:     Some(format!("no repository at {}", project.path)),
                },
            };

            let success = result.success;
            completed.push(FreezeProjectResult {
                project_id:         project.id.clone(),
                project_name:       entry.project_name.clone(),
                success,
                commands_executed:  result.commands_executed,
                stdout:             result.stdout,
                stderr:             result.stderr,
                rollback_attempted: false,
                rollback_succeeded: None,
                recovery_hint:      None,
            });

            if !success {
                failed = true;
                break;
            }
        }

        if !failed {
            return FreezeResult {
                freeze_name: freeze_name.clone(),
                project_results: completed,
                outcome: FreezeOutcome::Success,
            };
        }

        // --- Rollback: delete tags/bookmarks already created ---
        let succeeded_ids: Vec<_> = completed
            .iter()
            .filter(|r| r.success)
            .map(|r| r.project_id.clone())
            .collect();

        let mut any_rollback_failed = false;

        for res in completed.iter_mut() {
            if !succeeded_ids.contains(&res.project_id) { continue; }

            let project = match project_map.get(&res.project_id) {
                Some(p) => *p,
                None    => { res.rollback_attempted = true; res.rollback_succeeded = Some(false); any_rollback_failed = true; continue; }
            };

            let kind = detect_vcs_kind(std::path::Path::new(&project.path)).await;
            let rb = match kind {
                Some(VcsKind::Git) =>
                    git::tag_delete(project, freeze_name).await,
                Some(VcsKind::Jujutsu) =>
                    jj::bookmark_delete(project, freeze_name).await,
                None => crate::model::operation::ProjectOperationResult {
                    project_id:        project.id.clone(),
                    success:           false,
                    commands_executed: vec![],
                    stdout:            String::new(),
                    stderr:            String::new(),
                    exit_code:         None,
                    error_message:     Some("no repository".to_owned()),
                },
            };

            res.rollback_attempted = true;
            res.rollback_succeeded = Some(rb.success);

            if !rb.success {
                any_rollback_failed = true;
                let cmd = match kind {
                    Some(VcsKind::Jujutsu) => format!("jj bookmark delete {freeze_name}"),
                    _                       => format!("git tag -d {freeze_name}"),
                };
                res.recovery_hint = Some(RecoveryHint {
                    project_id: project.id.clone(),
                    situation:  format!(
                        "rollback failed — tag/bookmark '{}' may still exist on this project",
                        freeze_name
                    ),
                    suggested_commands: vec![
                        format!("cd {:?} && {}", project.path, cmd),
                    ],
                    see_also: None,
                });
            }
        }

        FreezeResult {
            freeze_name: freeze_name.clone(),
            project_results: completed,
            outcome: if any_rollback_failed {
                FreezeOutcome::RollbackFailed
            } else {
                FreezeOutcome::RolledBack
            },
        }
    }
}

impl VcsAdapter {
    // --- Conflict resolution ---

    pub async fn list_conflicted_files(
        project: &Project,
    ) -> crate::model::conflict::ProjectConflictDetail {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git)      => git::list_conflicted_files(project).await,
            Some(VcsKind::Jujutsu) => jj::list_conflicted_files(project).await,
            None => crate::model::conflict::ProjectConflictDetail {
                project_id:       project.id.clone(),
                project_name:     project.name.clone(),
                conflicted_files: vec![],
                note:             None,
                read_error:       Some(format!("no repository at {}", project.path)),
            },
        }
    }

    pub async fn mark_resolved(project: &Project, file_path: &str) -> crate::model::operation::ProjectOperationResult {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git) => git::mark_resolved(project, file_path).await,
            _ => crate::model::operation::ProjectOperationResult {
                project_id:        project.id.clone(),
                success:           false,
                commands_executed: vec![],
                stdout:            String::new(),
                stderr:            String::new(),
                exit_code:         None,
                error_message:     Some("mark-resolved only supported for Git".to_owned()),
            },
        }
    }

    pub async fn abort_merge(project: &Project) -> crate::model::operation::ProjectOperationResult {
        let kind = detect_vcs_kind(Path::new(&project.path)).await;
        match kind {
            Some(VcsKind::Git) => git::abort_merge(project).await,
            _ => crate::model::operation::ProjectOperationResult {
                project_id:        project.id.clone(),
                success:           false,
                commands_executed: vec![],
                stdout:            String::new(),
                stderr:            String::new(),
                exit_code:         None,
                error_message:     Some("abort-merge only supported for Git".to_owned()),
            },
        }
    }

    // --- Changelog ---

    pub async fn collect_changelog(
        projects: &[Project],
        since_ref: &str,
        max_concurrent: usize,
    ) -> crate::model::changelog::ChangelogDraft {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let sem   = Arc::new(Semaphore::new(max_concurrent));
        let since = since_ref.to_owned();
        let mut handles = Vec::new();

        for project in projects {
            let project = project.clone();
            let sem     = Arc::clone(&sem);
            let since   = since.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("open");
                let kind = detect_vcs_kind(Path::new(&project.path)).await;
                match kind {
                    Some(VcsKind::Git) =>
                        git::log_since(&project, &since, None).await,
                    Some(VcsKind::Jujutsu) =>
                        jj::log_since(&project, &since, None).await,
                    None => crate::model::changelog::ProjectCommits {
                        project_id:   project.id.clone(),
                        project_name: project.name.clone(),
                        since_ref:    since,
                        entries:      vec![],
                        error:        Some(format!("no repository at {}", project.path)),
                    },
                }
            }));
        }

        let mut project_commits = Vec::with_capacity(handles.len());
        for h in handles {
            if let Ok(c) = h.await { project_commits.push(c); }
        }

        crate::model::changelog::ChangelogDraft {
            release_name:  since_ref.to_owned(),
            generated_at:  chrono::Utc::now(),
            projects:      project_commits,
        }
    }

    // --- Topology ---

    /// Scan registered projects for Cargo.toml manifests and build a dependency graph.
    pub async fn scan_topology(projects: &[Project]) -> crate::model::topology::DependencyGraph {
        use crate::model::topology::{DependencyEdge, DependencyGraph, parse_cargo_toml};
        use std::collections::HashMap;

        // Build crate-name → ProjectId map first.
        let mut name_to_id: HashMap<String, crate::model::project::ProjectId> = HashMap::new();
        let mut edges = Vec::new();

        for project in projects {
            // Try Cargo.toml in the project root.
            let cargo_path = format!("{}/Cargo.toml", project.path);
            let Ok(manifest) = parse_cargo_toml(&cargo_path) else { continue; };

            if let Some(ref pkg_name) = manifest.package_name {
                name_to_id.insert(pkg_name.clone(), project.id.clone());
            }

            for dep in &manifest.dependencies {
                edges.push(DependencyEdge {
                    from_project_id:   project.id.clone(),
                    from_project_name: project.name.clone(),
                    to_project_name:   dep.name.clone(),
                    version_req:       dep.version_req.clone(),
                    is_path_dep:       dep.is_path,
                });
            }
        }

        // Only keep edges where `to_project_name` is a known registered project.
        let known_names: std::collections::HashSet<String> = projects
            .iter()
            .map(|p| p.name.clone())
            .chain(name_to_id.keys().cloned())
            .collect();

        edges.retain(|e| known_names.contains(&e.to_project_name));
        DependencyGraph { edges }
    }

    /// List all tags in a project (for the changelog "since" selector).
    pub async fn list_tags(project: &Project) -> Vec<String> {
        let path = project.path.clone();
        tokio::task::spawn_blocking(move || {
            git::list_tags_blocking(&path)
        })
        .await
        .unwrap_or_default()
    }
}
