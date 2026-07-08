//! Git-specific read and write implementations.
//!
//! Read operations use the `git` CLI (gix upgrade planned for Phase 2+).
//! Write operations always record the commands executed for transparency.

use chrono::Utc;
use std::process::Command;

use crate::model::{
    operation::{ProjectOperationResult, RecoveryHint},
    project::Project,
    status::{
        ConflictStatus, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind,
        WorkingTreeStatus,
    },
};

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

pub async fn read_status(project: &Project) -> ProjectStatus {
    let path = project.path.clone();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || read_blocking(&project_id, &path))
        .await
        .unwrap_or_else(|e| ProjectStatus {
            project_id: project.id.clone(),
            identity: RepositoryIdentity { path: project.path.clone(), vcs_kind: VcsKind::Git },
            context: None,
            remote: Default::default(),
            working_tree: Default::default(),
            conflict: Default::default(),
            refreshed_at: Utc::now(),
            read_error: Some(format!("task join error: {e}")),
        })
}

fn read_blocking(project_id: &crate::model::project::ProjectId, path: &str) -> ProjectStatus {
    let err_status = |msg: String| ProjectStatus {
        project_id: project_id.clone(),
        identity: RepositoryIdentity { path: path.to_owned(), vcs_kind: VcsKind::Git },
        context: None,
        remote: Default::default(),
        working_tree: Default::default(),
        conflict: Default::default(),
        refreshed_at: Utc::now(),
        read_error: Some(msg),
    };

    let rev = git_cmd(&["rev-parse", "--git-dir"], path);
    if rev.map(|o| !o.status.success()).unwrap_or(true) {
        return err_status(format!("not a git repository: {path}"));
    }

    ProjectStatus {
        project_id: project_id.clone(),
        identity: RepositoryIdentity { path: path.to_owned(), vcs_kind: VcsKind::Git },
        context: read_context(path),
        remote: read_remote(path),
        working_tree: read_working_tree(path),
        conflict: read_conflict(path),
        refreshed_at: Utc::now(),
        read_error: None,
    }
}

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

fn read_context(path: &str) -> Option<VcsContext> {
    match git_stdout(&["symbolic-ref", "--short", "HEAD"], path) {
        Some(b) => Some(VcsContext {
            label: b.clone(),
            branch: Some(b),
            jj_change_id: None,
            jj_bookmark: None,
            is_detached: false,
        }),
        None => {
            let hash = git_stdout(&["rev-parse", "--short", "HEAD"], path)
                .unwrap_or_else(|| "(unknown)".to_owned());
            Some(VcsContext {
                label: format!("(detached: {hash})"),
                branch: None,
                jj_change_id: None,
                jj_bookmark: None,
                is_detached: true,
            })
        }
    }
}

fn read_remote(path: &str) -> RemoteStatus {
    let out = match git_cmd(&["rev-list", "--left-right", "--count", "HEAD...@{u}"], path) {
        Ok(o) => o,
        Err(_) => return RemoteStatus::default(),
    };
    if !out.status.success() {
        return RemoteStatus::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return RemoteStatus::default();
    }
    let ahead: u32  = parts[0].parse().unwrap_or(0);
    let behind: u32 = parts[1].parse().unwrap_or(0);
    let upstream = git_stdout(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], path);
    RemoteStatus { ahead, behind, upstream }
}

fn read_working_tree(path: &str) -> WorkingTreeStatus {
    let out = match git_cmd(&["status", "--porcelain"], path) {
        Ok(o) => o,
        Err(_) => return WorkingTreeStatus::default(),
    };
    if !out.status.success() {
        return WorkingTreeStatus::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut uncommitted = 0u32;
    let mut untracked   = 0u32;
    for line in text.lines() {
        if line.len() < 2 { continue; }
        if &line[..2] == "??" { untracked  += 1; }
        else                  { uncommitted += 1; }
    }
    WorkingTreeStatus { uncommitted_count: uncommitted, untracked_count: untracked }
}

fn read_conflict(path: &str) -> ConflictStatus {
    let git_dir = match git_stdout(&["rev-parse", "--git-dir"], path) {
        Some(d) => {
            let p = std::path::Path::new(d.as_str());
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                std::path::Path::new(path).join(p)
            }
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
// Write — fetch
// ---------------------------------------------------------------------------

pub async fn fetch(project: &Project) -> ProjectOperationResult {
    run_git(project, &["fetch", "--prune"]).await
}

// ---------------------------------------------------------------------------
// Write — Smart Pull helpers
// ---------------------------------------------------------------------------

/// Stash working-tree changes (`git stash push -m knotra-smart-pull`).
pub async fn stash_push(project: &Project) -> ProjectOperationResult {
    run_git(project, &["stash", "push", "-m", "knotra-smart-pull"]).await
}

/// Pop the topmost stash.
pub async fn stash_pop(project: &Project) -> ProjectOperationResult {
    run_git(project, &["stash", "pop"]).await
}

/// Fast-forward merge after a clean fetch (`git merge --ff-only @{u}`).
pub async fn merge_ff(project: &Project) -> ProjectOperationResult {
    run_git(project, &["merge", "--ff-only", "@{u}"]).await
}

/// Full Smart Pull sequence for one Git project.
///
/// Steps:
/// 1. Fetch
/// 2. If dirty and `stash_dirty` → stash, pull ff-only, pop
/// 3. If dirty and not `stash_dirty` → fetch only, mark excluded
/// 4. If clean  → fetch + ff-only merge
///
/// Returns the aggregate result and a recovery hint if stash-pop failed.
pub async fn smart_pull(
    project: &Project,
    stash_dirty: bool,
) -> (ProjectOperationResult, Option<RecoveryHint>) {
    let mut commands_executed: Vec<String> = Vec::new();
    let mut stdout_acc = String::new();
    let mut stderr_acc = String::new();

    // --- Step 1: Read dirty state (non-destructive) ---
    let wt = tokio::task::spawn_blocking({
        let path = project.path.clone();
        move || read_working_tree(&path)
    })
    .await
    .unwrap_or_default();

    // --- Step 2: Fetch ---
    let fetch_res = run_git(project, &["fetch", "--prune"]).await;
    commands_executed.extend(fetch_res.commands_executed.clone());
    stdout_acc.push_str(&fetch_res.stdout);
    stderr_acc.push_str(&fetch_res.stderr);

    if !fetch_res.success {
        let hint = RecoveryHint {
            project_id: project.id.clone(),
            situation: "fetch failed".to_owned(),
            suggested_commands: vec![
                format!("cd {:?} && git fetch --prune", project.path),
            ],
            see_also: None,
        };
        return (
            ProjectOperationResult {
                project_id: project.id.clone(),
                success: false,
                commands_executed,
                stdout: stdout_acc,
                stderr: stderr_acc,
                exit_code: fetch_res.exit_code,
                error_message: Some("fetch failed".to_owned()),
            },
            Some(hint),
        );
    }

    // --- Step 3: Decide merge strategy based on dirty state ---
    if wt.is_dirty() && !stash_dirty {
        // Excluded: fetch done, merge skipped.
        return (
            ProjectOperationResult {
                project_id: project.id.clone(),
                success: true,
                commands_executed,
                stdout: stdout_acc,
                stderr: format!("{stderr_acc}[knotra] dirty — merge skipped (project excluded)"),
                exit_code: Some(0),
                error_message: None,
            },
            None,
        );
    }

    let stash_applied = if wt.is_dirty() && stash_dirty {
        // Stash before merging.
        let stash_res = run_git(project, &["stash", "push", "-m", "knotra-smart-pull"]).await;
        commands_executed.extend(stash_res.commands_executed.clone());
        stdout_acc.push_str(&stash_res.stdout);
        stderr_acc.push_str(&stash_res.stderr);
        if !stash_res.success {
            return (
                ProjectOperationResult {
                    project_id: project.id.clone(),
                    success: false,
                    commands_executed,
                    stdout: stdout_acc,
                    stderr: stderr_acc,
                    exit_code: stash_res.exit_code,
                    error_message: Some("stash failed before merge".to_owned()),
                },
                None,
            );
        }
        true
    } else {
        false
    };

    // --- Step 4: ff-only merge ---
    let merge_res = run_git(project, &["merge", "--ff-only", "@{u}"]).await;
    commands_executed.extend(merge_res.commands_executed.clone());
    stdout_acc.push_str(&merge_res.stdout);
    stderr_acc.push_str(&merge_res.stderr);

    // --- Step 5: Stash pop (if we stashed) ---
    let mut recovery_hint: Option<RecoveryHint> = None;
    if stash_applied {
        let pop_res = run_git(project, &["stash", "pop"]).await;
        commands_executed.extend(pop_res.commands_executed.clone());
        stdout_acc.push_str(&pop_res.stdout);
        stderr_acc.push_str(&pop_res.stderr);
        if !pop_res.success {
            recovery_hint = Some(RecoveryHint {
                project_id: project.id.clone(),
                situation: "stash pop failed after merge — your changes are in the stash".to_owned(),
                suggested_commands: vec![
                    format!("cd {:?} && git stash pop", project.path),
                    format!("cd {:?} && git stash list", project.path),
                ],
                see_also: Some("https://git-scm.com/docs/git-stash".to_owned()),
            });
        }
    }

    let success = merge_res.success && recovery_hint.is_none();
    (
        ProjectOperationResult {
            project_id: project.id.clone(),
            success,
            commands_executed,
            stdout: stdout_acc,
            stderr: stderr_acc,
            exit_code: merge_res.exit_code,
            error_message: if success { None } else {
                Some(merge_res.error_message.unwrap_or_else(|| "merge failed".to_owned()))
            },
        },
        recovery_hint,
    )
}

// ---------------------------------------------------------------------------
// Internal CLI runner
// ---------------------------------------------------------------------------

async fn run_git(project: &Project, args: &[&str]) -> ProjectOperationResult {
    let cmd_str = format!("git {}", args.join(" "));
    let path = project.path.clone();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || {
        let result = Command::new("git").args(&args_owned).current_dir(&path).output();
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
// Context listing
// ---------------------------------------------------------------------------

/// List local and remote branches for a Git repository.
pub async fn list_contexts(project: &Project) -> crate::model::status::ContextList {
    let path = project.path.clone();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || list_contexts_blocking(&project_id, &path))
        .await
        .unwrap_or_else(|e| crate::model::status::ContextList {
            project_id: project.id.clone(),
            vcs_kind: crate::model::status::VcsKind::Git,
            candidates: Vec::new(),
            warning: Some(format!("task join error: {e}")),
        })
}

fn list_contexts_blocking(
    project_id: &crate::model::project::ProjectId,
    path: &str,
) -> crate::model::status::ContextList {
    use crate::model::status::{ContextCandidate, ContextList, VcsKind};

    let current = git_stdout(&["symbolic-ref", "--short", "HEAD"], path)
        .unwrap_or_default();

    // Local branches.
    let local_out = git_stdout(
        &["branch", "--format=%(refname:short)"],
        path,
    )
    .unwrap_or_default();

    // Remote branches (stripped of "origin/" prefix for display).
    let remote_out = git_stdout(
        &["branch", "-r", "--format=%(refname:short)"],
        path,
    )
    .unwrap_or_default();

    let mut candidates: Vec<ContextCandidate> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in local_out.lines() {
        let label = line.trim().to_owned();
        if label.is_empty() { continue; }
        let is_current = label == current;
        seen.insert(label.clone());
        candidates.push(ContextCandidate {
            label: label.clone(),
            target: label,
            is_current,
            is_remote: false,
        });
    }

    for line in remote_out.lines() {
        let full = line.trim();
        // Skip HEAD symbolic refs.
        if full.contains("HEAD") { continue; }
        // Strip remote prefix for the label (e.g. "origin/main" → "main").
        let label = full
            .split_once('/')
            .map(|(_, b)| b.to_owned())
            .unwrap_or_else(|| full.to_owned());
        // Skip if a local branch with the same name already listed.
        if seen.contains(&label) { continue; }
        candidates.push(ContextCandidate {
            label: label.clone(),
            target: full.to_owned(),
            is_current: false,
            is_remote: true,
        });
    }

    // Sort: current first, then local, then remote.
    candidates.sort_by(|a, b| {
        b.is_current.cmp(&a.is_current)
            .then(a.is_remote.cmp(&b.is_remote))
            .then(a.label.cmp(&b.label))
    });

    ContextList {
        project_id: project_id.clone(),
        vcs_kind: VcsKind::Git,
        candidates,
        warning: if current.is_empty() {
            Some("Detached HEAD — no current branch".to_owned())
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// Context switch
// ---------------------------------------------------------------------------

/// Switch to a branch (local or remote-tracking).
///
/// For remote targets, creates a local tracking branch if needed
/// (`git checkout -b <branch> --track <remote>/<branch>`).
pub async fn switch_context(
    project: &Project,
    target: &str,
) -> (
    crate::model::operation::ProjectOperationResult,
    Option<crate::model::operation::RecoveryHint>,
) {
    use crate::model::operation::RecoveryHint;

    // Dirty-state check before switching.
    let wt = tokio::task::spawn_blocking({
        let path = project.path.clone();
        move || read_working_tree(&path)
    })
    .await
    .unwrap_or_default();

    if wt.is_dirty() {
        let hint = RecoveryHint {
            project_id: project.id.clone(),
            situation: format!(
                "working tree is dirty ({} uncommitted, {} untracked) — \
                 stash or commit changes before switching",
                wt.uncommitted_count, wt.untracked_count
            ),
            suggested_commands: vec![
                format!("cd {:?} && git stash push -m before-switch", project.path),
                format!("cd {:?} && git switch {}", project.path, target),
                format!("cd {:?} && git stash pop", project.path),
            ],
            see_also: None,
        };
        let result = run_git(
            project,
            &["switch", "--dry-run", target],
        )
        .await;
        return (
            crate::model::operation::ProjectOperationResult {
                project_id: project.id.clone(),
                success: false,
                commands_executed: result.commands_executed,
                stdout: result.stdout,
                stderr: format!(
                    "[knotra] switch blocked: dirty working tree\n{}",
                    result.stderr
                ),
                exit_code: Some(1),
                error_message: Some(
                    "dirty working tree — stash or commit before switching".to_owned(),
                ),
            },
            Some(hint),
        );
    }

    // Determine whether the target is a remote ref.
    let is_remote = target.contains('/');
    let args: Vec<&str> = if is_remote {
        // `git switch -c <local-branch> --track <remote>/<branch>`
        let local = target.split_once('/').map(|(_, b)| b).unwrap_or(target);
        vec!["switch", "-c", local, "--track", target]
    } else {
        vec!["switch", target]
    };

    let result = run_git(project, &args).await;

    let hint = if !result.success {
        Some(RecoveryHint {
            project_id: project.id.clone(),
            situation: format!("switch to '{}' failed", target),
            suggested_commands: vec![
                format!("cd {:?} && git switch {}", project.path, target),
                format!("cd {:?} && git status", project.path),
            ],
            see_also: None,
        })
    } else {
        None
    };

    (result, hint)
}
