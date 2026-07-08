//! Git-specific read and write operations.
//!
//! All reads delegate to `endringer-backend-async` (`AsyncRepository` →
//! `GitBackend` → `gix`). Write operations (fetch, tag, push, stash) still
//! use the `git` CLI because gix does not expose mutation APIs at this level.
//!
//! Phase split:
//!   - Read : `AsyncRepository::open` → gix (lock-free, `spawn_blocking`)
//!   - Write: `std::process::Command` with `git`

use chrono::Utc;
use std::process::Command;

use endringer_backend_async::AsyncRepository;
use endringer_backend_core::types::{BackendKind, SortOrder};

use crate::model::{
    operation::{ProjectOperationResult, RecoveryHint},
    project::Project,
    status::{
        ConflictStatus, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind,
        WorkingTreeStatus,
    },
};

// ---------------------------------------------------------------------------
// Helpers: raw git CLI (used only for writes)
// ---------------------------------------------------------------------------

fn git_cmd(args: &[&str], cwd: &str) -> std::io::Result<std::process::Output> {
    Command::new("git").args(args).current_dir(cwd).output()
}

fn git_stdout(args: &[&str], cwd: &str) -> Option<String> {
    let out = git_cmd(args, cwd).ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        None
    }
}

async fn run_git(project: &Project, args: &[&str]) -> ProjectOperationResult {
    let cmd_str = format!("git {}", args.join(" "));
    let path = project.path.clone();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || {
        let result = Command::new("git")
            .args(&args_owned)
            .current_dir(&path)
            .output();
        match result {
            Ok(output) => {
                let code = output.status.code().unwrap_or(-1);
                ProjectOperationResult {
                    project_id: project_id.clone(),
                    success: output.status.success(),
                    commands_executed: vec![cmd_str.clone()],
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code: Some(code),
                    error_message: if output.status.success() { None } else {
                        Some(format!("exit code {code}"))
                    },
                }
            }
            Err(e) => ProjectOperationResult {
                project_id,
                success: false,
                commands_executed: vec![cmd_str],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: Some(format!("failed to spawn git: {e}")),
            },
        }
    })
    .await
    .unwrap_or_else(|e| ProjectOperationResult {
        project_id: project.id.clone(),
        success: false,
        commands_executed: vec![],
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        error_message: Some(format!("task join error: {e}")),
    })
}

// ---------------------------------------------------------------------------
// Read — via endringer-backend-async (gix)
// ---------------------------------------------------------------------------

pub async fn read_status(project: &Project) -> ProjectStatus {
    let path = std::path::Path::new(&project.path);

    match AsyncRepository::open(path).await {
        Err(e) => ProjectStatus {
            project_id: project.id.clone(),
            identity: RepositoryIdentity { path: project.path.clone(), vcs_kind: VcsKind::Git },
            context: None,
            remote: Default::default(),
            working_tree: Default::default(),
            conflict: Default::default(),
            refreshed_at: Utc::now(),
            read_error: Some(format!("cannot open repository: {e}")),
        },
        Ok(repo) => {
            // StatusDigest — branch + HEAD commit
            let context = repo.status_digest().await.ok().map(|d| VcsContext {
                label:        d.current_branch.clone(),
                branch:       if d.current_branch.starts_with("(detached") { None }
                              else { Some(d.current_branch.clone()) },
                jj_change_id: None,
                jj_bookmark:  None,
                is_detached:  d.current_branch.starts_with("(detached"),
            });

            // Working tree — dirty / untracked via is_dirty + worktree_status
            let working_tree = repo.worktree_status().await.ok().map(|ws| {
                WorkingTreeStatus {
                    uncommitted_count: (ws.staged.len() + ws.unstaged.len()) as u32,
                    untracked_count:   ws.untracked.len() as u32,
                }
            }).unwrap_or_default();

            // Ahead/Behind via CLI (gix doesn't expose this directly)
            let remote = read_remote_cli(&project.path);

            // Conflict via sentinel files
            let conflict = read_conflict_cli(&project.path);

            ProjectStatus {
                project_id: project.id.clone(),
                identity: RepositoryIdentity {
                    path: project.path.clone(),
                    vcs_kind: VcsKind::Git,
                },
                context,
                remote,
                working_tree,
                conflict,
                refreshed_at: Utc::now(),
                read_error: None,
            }
        }
    }
}

fn read_remote_cli(path: &str) -> RemoteStatus {
    let out = match git_cmd(&["rev-list", "--left-right", "--count", "HEAD...@{u}"], path) {
        Ok(o) => o,
        Err(_) => return RemoteStatus::default(),
    };
    if !out.status.success() { return RemoteStatus::default(); }
    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.trim().split_whitespace().collect();
    if parts.len() < 2 { return RemoteStatus::default(); }
    let ahead:  u32 = parts[0].parse().unwrap_or(0);
    let behind: u32 = parts[1].parse().unwrap_or(0);
    let upstream = git_stdout(
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], path);
    RemoteStatus { ahead, behind, upstream }
}

fn read_conflict_cli(path: &str) -> ConflictStatus {
    let git_dir = match git_stdout(&["rev-parse", "--git-dir"], path) {
        Some(d) => {
            let p = std::path::Path::new(d.as_str());
            if p.is_absolute() { p.to_path_buf() }
            else { std::path::Path::new(path).join(p) }
        }
        None => return ConflictStatus::default(),
    };
    let has_conflict = git_dir.join("MERGE_HEAD").exists()
        || git_dir.join("CHERRY_PICK_HEAD").exists()
        || git_dir.join("REBASE_MERGE").is_dir()
        || git_dir.join("REBASE_APPLY").is_dir();
    ConflictStatus { has_conflict, conflict_count: None }
}

// ---------------------------------------------------------------------------
// Context listing — via backend-async
// ---------------------------------------------------------------------------

pub async fn list_contexts(project: &Project) -> crate::model::status::ContextList {
    use crate::model::status::{ContextCandidate, ContextList};

    let path = std::path::Path::new(&project.path);
    match AsyncRepository::open(path).await {
        Err(e) => ContextList {
            project_id: project.id.clone(),
            vcs_kind: VcsKind::Git,
            candidates: vec![],
            warning: Some(format!("cannot open: {e}")),
        },
        Ok(repo) => {
            let current = repo.status_digest().await.ok()
                .map(|d| d.current_branch)
                .unwrap_or_default();

            let mut candidates: Vec<ContextCandidate> = Vec::new();
            let mut seen = std::collections::HashSet::new();

            if let Ok(branches) = repo.local_branches().await {
                for b in branches {
                    let is_current = b.name == current;
                    seen.insert(b.name.clone());
                    candidates.push(ContextCandidate {
                        label: b.name.clone(),
                        target: b.name,
                        is_current,
                        is_remote: false,
                    });
                }
            }
            if let Ok(remotes) = repo.remote_branches().await {
                for b in remotes {
                    let local_name = b.name.split_once('/').map(|(_, n)| n.to_owned())
                        .unwrap_or(b.name.clone());
                    if seen.contains(&local_name) { continue; }
                    candidates.push(ContextCandidate {
                        label:      local_name,
                        target:     b.name,
                        is_current: false,
                        is_remote:  true,
                    });
                }
            }

            candidates.sort_by(|a, b|
                b.is_current.cmp(&a.is_current)
                    .then(a.is_remote.cmp(&b.is_remote))
                    .then(a.label.cmp(&b.label))
            );

            ContextList {
                project_id: project.id.clone(),
                vcs_kind: VcsKind::Git,
                candidates,
                warning: None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Changelog — via backend-async
// ---------------------------------------------------------------------------

pub async fn log_since(
    project: &Project,
    since_ref: &str,
    until_ref: Option<&str>,
) -> crate::model::changelog::ProjectCommits {
    use crate::model::changelog::{CommitEntry, ProjectCommits};

    let path  = project.path.clone();
    let since = since_ref.to_owned();
    let until = until_ref.unwrap_or("HEAD").to_owned();
    let project_id   = project.id.clone();
    let project_name = project.name.clone();

    tokio::task::spawn_blocking(move || {
        // Use `git log <since>..<until>` — ref-based, no timestamp ambiguity.
        let range  = format!("{since}..{until}");
        let fmt    = "%H|%s|%an|%aI";
        let output = Command::new("git")
            .args(["log", &range, &format!("--format={fmt}"), "--no-merges"])
            .current_dir(&path)
            .output();

        match output {
            Err(e) => ProjectCommits {
                project_id, project_name, since_ref: since, entries: vec![],
                error: Some(e.to_string()),
            },
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_owned();
                ProjectCommits {
                    project_id, project_name, since_ref: since, entries: vec![],
                    error: Some(stderr),
                }
            },
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let entries = text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| {
                        let mut p = l.splitn(4, '|');
                        let hash    = p.next()?.to_owned();
                        let subject = p.next()?.to_owned();
                        let author  = p.next()?.to_owned();
                        let date    = p.next()?.trim().parse::<chrono::DateTime<chrono::Utc>>().ok()?;
                        Some(CommitEntry { hash, subject, author, date })
                    })
                    .collect();
                ProjectCommits { project_id, project_name, since_ref: since, entries, error: None }
            }
        }
    }).await.unwrap_or_else(|e| crate::model::changelog::ProjectCommits {
        project_id: project.id.clone(), project_name: project.name.clone(),
        since_ref: since_ref.to_owned(), entries: vec![],
        error: Some(format!("task join error: {e}")),
    })
}

// ---------------------------------------------------------------------------
// Tag operations — via backend-async
// ---------------------------------------------------------------------------

pub async fn list_tags_blocking(path: &str) -> Vec<String> {
    let p = std::path::Path::new(path).to_path_buf();
    tokio::task::spawn_blocking(move || {
        let repo = endringer_backend_git::GitBackend::open(&p).ok()?;
        use endringer_backend_core::backend::VcsBackend;
        repo.list_tags_sorted(SortOrder::NewestFirst).ok()
            .map(|tags| tags.into_iter().map(|t| t.name).collect())
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}

pub async fn tag_create(project: &Project, tag_name: &str) -> ProjectOperationResult {
    let path = std::path::Path::new(&project.path).to_path_buf();
    let name = tag_name.to_owned();
    let project_id = project.id.clone();
    let project_id2 = project.id.clone();

    tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        match endringer_backend_git::GitBackend::open(&path) {
            Err(e) => ProjectOperationResult {
                project_id: project_id.clone(),
                success: false,
                commands_executed: vec![format!("git tag {name}")],
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: None,
                error_message: Some(e.to_string()),
            },
            Ok(backend) => match backend.create_tag(&name) {
                Ok(()) => ProjectOperationResult {
                    project_id: project_id.clone(),
                    success: true,
                    commands_executed: vec![format!("git tag {name}")],
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    error_message: None,
                },
                Err(e) => ProjectOperationResult {
                    project_id,
                    success: false,
                    commands_executed: vec![format!("git tag {name}")],
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: Some(1),
                    error_message: Some(e.to_string()),
                },
            },
        }
    })
    .await
    .unwrap_or_else(|e| ProjectOperationResult {
        project_id: project_id2,
        success: false,
        commands_executed: vec![],
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        error_message: Some(format!("task join error: {e}")),
    })
}

pub async fn tag_delete(project: &Project, tag_name: &str) -> ProjectOperationResult {
    // Use CLI for delete (simpler, avoids reflog complexities)
    run_git(project, &["tag", "-d", tag_name]).await
}

pub async fn tag_exists(project: &Project, tag_name: &str) -> bool {
    let path = std::path::Path::new(&project.path).to_path_buf();
    let name = tag_name.to_owned();
    tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        endringer_backend_git::GitBackend::open(&path).ok()
            .and_then(|b| b.list_tags().ok())
            .map(|tags| tags.iter().any(|t| t.name == name))
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Stash entries — NEW via backend
// ---------------------------------------------------------------------------

pub async fn stash_entries(project: &Project) -> Vec<crate::model::status::StashEntry> {
    use crate::model::status::StashEntry as KnotraStash;
    let path = std::path::Path::new(&project.path).to_path_buf();
    tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        endringer_backend_git::GitBackend::open(&path).ok()
            .and_then(|b| b.stash_entries().ok())
            .map(|entries| entries.into_iter().map(|e| KnotraStash {
                index:   e.index,
                message: e.message,
            }).collect())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Write: fetch, smart pull, conflict ops, push
// ---------------------------------------------------------------------------

pub async fn fetch(project: &Project) -> ProjectOperationResult {
    run_git(project, &["fetch", "--prune"]).await
}

pub async fn smart_pull(
    project: &Project,
    stash_dirty: bool,
) -> (ProjectOperationResult, Option<RecoveryHint>) {
    // Read dirty state via gix
    let path = std::path::Path::new(&project.path).to_path_buf();
    let is_dirty = tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        endringer_backend_git::GitBackend::open(&path).ok()
            .and_then(|b| b.is_dirty().ok())
            .unwrap_or(false)
    }).await.unwrap_or(false);

    let fetch_res = run_git(project, &["fetch", "--prune"]).await;
    if !fetch_res.success {
        let hint = RecoveryHint {
            project_id: project.id.clone(),
            situation: "fetch failed".to_owned(),
            suggested_commands: vec![format!("cd {:?} && git fetch --prune", project.path)],
            see_also: None,
        };
        return (fetch_res, Some(hint));
    }

    if is_dirty && !stash_dirty {
        return (ProjectOperationResult {
            project_id: project.id.clone(),
            success: true,
            commands_executed: fetch_res.commands_executed,
            stdout: fetch_res.stdout,
            stderr: format!("{}[knotra] dirty — merge skipped", fetch_res.stderr),
            exit_code: Some(0),
            error_message: None,
        }, None);
    }

    let stash_applied = if is_dirty && stash_dirty {
        let r = run_git(project, &["stash", "push", "-m", "knotra-smart-pull"]).await;
        if !r.success { return (r, None); }
        true
    } else { false };

    let merge = run_git(project, &["merge", "--ff-only", "@{u}"]).await;

    if stash_applied {
        let pop = run_git(project, &["stash", "pop"]).await;
        if !pop.success {
            let hint = RecoveryHint {
                project_id: project.id.clone(),
                situation: "stash pop failed — changes remain in stash".to_owned(),
                suggested_commands: vec![
                    format!("cd {:?} && git stash pop", project.path),
                ],
                see_also: Some("https://git-scm.com/docs/git-stash".to_owned()),
            };
            return (merge, Some(hint));
        }
    }

    (merge, None)
}

pub async fn switch_context(
    project: &Project,
    target: &str,
) -> (ProjectOperationResult, Option<RecoveryHint>) {
    // Check dirty via gix
    let path = std::path::Path::new(&project.path).to_path_buf();
    let wt_status = tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        endringer_backend_git::GitBackend::open(&path).ok()
            .and_then(|b| b.worktree_status().ok())
    }).await.ok().flatten();

    let uncommitted = wt_status.as_ref()
        .map(|s| s.staged.len() + s.unstaged.len()).unwrap_or(0);
    let untracked = wt_status.as_ref()
        .map(|s| s.untracked.len()).unwrap_or(0);

    if uncommitted > 0 {
        let hint = RecoveryHint {
            project_id: project.id.clone(),
            situation: format!(
                "working tree is dirty ({uncommitted} uncommitted, {untracked} untracked)"
            ),
            suggested_commands: vec![
                format!("cd {:?} && git stash push -m before-switch", project.path),
            ],
            see_also: None,
        };
        return (ProjectOperationResult {
            project_id: project.id.clone(),
            success: false,
            commands_executed: vec![],
            stdout: String::new(),
            stderr: "[knotra] blocked: dirty working tree".to_owned(),
            exit_code: Some(1),
            error_message: Some("dirty working tree".to_owned()),
        }, Some(hint));
    }

    let is_remote = target.contains('/');
    let result = if is_remote {
        let local = target.split_once('/').map(|(_, b)| b).unwrap_or(target);
        run_git(project, &["switch", "-c", local, "--track", target]).await
    } else {
        run_git(project, &["switch", target]).await
    };

    let hint = if !result.success {
        Some(RecoveryHint {
            project_id: project.id.clone(),
            situation: format!("switch to '{}' failed", target),
            suggested_commands: vec![format!("cd {:?} && git switch {}", project.path, target)],
            see_also: None,
        })
    } else { None };

    (result, hint)
}

pub async fn list_conflicted_files(
    project: &Project,
) -> crate::model::conflict::ProjectConflictDetail {
    use crate::model::conflict::{ConflictMarker, ConflictedFile, ProjectConflictDetail};

    // worktree_status gives us staged/unstaged, but for conflicts we need
    // the raw porcelain (UU/AA/UD markers). Keep CLI for this.
    let path = project.path.clone();
    let project_id = project.id.clone();
    let project_name = project.name.clone();

    tokio::task::spawn_blocking(move || {
        let out = Command::new("git")
            .args(["diff", "--name-status", "--diff-filter=U"])
            .current_dir(&path)
            .output();
        match out {
            Err(e) => ProjectConflictDetail {
                project_id, project_name, conflicted_files: vec![], note: None,
                read_error: Some(e.to_string()),
            },
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let files = text.lines().filter(|l| !l.trim().is_empty()).map(|l| {
                    let mut parts = l.splitn(2, '\t');
                    let status = parts.next().unwrap_or("").trim();
                    let path   = parts.next().unwrap_or("").trim().to_owned();
                    let marker = match status {
                        "UU" => ConflictMarker::BothModified,
                        "AA" => ConflictMarker::BothAdded,
                        "UD" | "DU" => ConflictMarker::DeleteModify,
                        _ => ConflictMarker::Other,
                    };
                    ConflictedFile { path, marker }
                }).collect();
                ProjectConflictDetail {
                    project_id, project_name, conflicted_files: files, note: None, read_error: None,
                }
            }
        }
    }).await.unwrap_or_else(|e| crate::model::conflict::ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: vec![], note: None,
        read_error: Some(format!("task join error: {e}")),
    })
}

pub async fn mark_resolved(project: &Project, file_path: &str) -> ProjectOperationResult {
    run_git(project, &["add", file_path]).await
}

pub async fn abort_merge(project: &Project) -> ProjectOperationResult {
    run_git(project, &["merge", "--abort"]).await
}

pub async fn push_tags(project: &Project, tag_name: &str) -> ProjectOperationResult {
    run_git(project, &["push", "origin", tag_name]).await
}

// ---------------------------------------------------------------------------
// Freeze validation — via gix (is_dirty) + tag check
// ---------------------------------------------------------------------------

pub async fn validate_for_freeze(
    project: &crate::model::project::Project,
    freeze_name: &str,
    included: bool,
) -> crate::model::operation::FreezeValidationEntry {
    use crate::model::operation::FreezeValidationEntry;

    let path = std::path::Path::new(&project.path).to_path_buf();
    let name = freeze_name.to_owned();
    let err_id   = project.id.clone();
    let err_name = project.name.clone();
    let project  = project.clone();

    tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;

        let backend = match endringer_backend_git::GitBackend::open(&path) {
            Ok(b)  => b,
            Err(e) => return FreezeValidationEntry {
                project_id: project.id.clone(), project_name: project.name.clone(),
                included, is_clean: false, tag_exists: false, notes: vec![],
                blockers: vec![format!("cannot open repository: {e}")],
            },
        };

        let wt_status = backend.worktree_status().ok();
        let uncommitted = wt_status.as_ref()
            .map(|s| s.staged.len() + s.unstaged.len()).unwrap_or(0);
        let untracked = wt_status.as_ref()
            .map(|s| s.untracked.len()).unwrap_or(0);
        let conflict   = read_conflict_cli(path.to_str().unwrap_or(""));
        let tag_exists = backend.list_tags().ok()
            .map(|tags| tags.iter().any(|t| t.name == name))
            .unwrap_or(false);

        let is_clean   = uncommitted == 0 && !conflict.has_conflict;
        let mut blockers = vec![];
        let mut notes    = vec![];

        if conflict.has_conflict { blockers.push("unresolved conflict".to_owned()); }
        if uncommitted > 0 { blockers.push(format!("{uncommitted} uncommitted change(s)")); }
        if untracked   > 0 { notes.push(format!("{untracked} untracked file(s)")); }
        if tag_exists      { blockers.push(format!("tag '{}' already exists", name)); }

        FreezeValidationEntry {
            project_id: project.id.clone(), project_name: project.name.clone(),
            included, is_clean, tag_exists, notes, blockers,
        }
    }).await.unwrap_or_else(|e| crate::model::operation::FreezeValidationEntry {
        project_id: err_id, project_name: err_name,
        included, is_clean: false, tag_exists: false, notes: vec![],
        blockers: vec![format!("task join error: {e}")],
    })
}
