//! Git-specific read and write implementations.
//!
//! Phase 1: All reads use the `git` CLI for simplicity and correctness.
//! Phase 2 goal: replace hot-path reads with `gix` (gitoxide) for speed.

use chrono::Utc;
use std::process::Command;

use crate::model::{
    operation::ProjectOperationResult,
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

    // Verify this is actually a git repo.
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
    // Try symbolic branch name first.
    let branch = git_stdout(&["symbolic-ref", "--short", "HEAD"], path);
    match branch {
        Some(b) => Some(VcsContext {
            label: b.clone(),
            branch: Some(b),
            jj_change_id: None,
            jj_bookmark: None,
            is_detached: false,
        }),
        None => {
            // Detached HEAD — show short commit hash.
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
    // `git rev-list --left-right --count HEAD...@{u}` gives ahead behind.
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
    let ahead: u32 = parts[0].parse().unwrap_or(0);
    let behind: u32 = parts[1].parse().unwrap_or(0);

    // Get upstream name for display.
    let upstream = git_stdout(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], path);

    RemoteStatus { ahead, behind, upstream }
}

fn read_working_tree(path: &str) -> WorkingTreeStatus {
    // `git status --porcelain` — one line per changed/untracked file.
    let out = match git_cmd(&["status", "--porcelain"], path) {
        Ok(o) => o,
        Err(_) => return WorkingTreeStatus::default(),
    };
    if !out.status.success() {
        return WorkingTreeStatus::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut uncommitted = 0u32;
    let mut untracked = 0u32;
    for line in text.lines() {
        if line.len() < 2 { continue; }
        if &line[..2] == "??" {
            untracked += 1;
        } else {
            uncommitted += 1;
        }
    }
    WorkingTreeStatus { uncommitted_count: uncommitted, untracked_count: untracked }
}

fn read_conflict(path: &str) -> ConflictStatus {
    // Conflict markers: MERGE_HEAD, CHERRY_PICK_HEAD, REBASE_MERGE, REBASE_APPLY.
    let git_dir = match git_stdout(&["rev-parse", "--git-dir"], path) {
        Some(d) => std::path::PathBuf::from(path).join(d),
        None => return ConflictStatus::default(),
    };
    let git_dir = if git_dir.is_absolute() {
        git_dir
    } else {
        std::path::PathBuf::from(path).join(&git_dir)
    };

    let has_conflict = git_dir.join("MERGE_HEAD").exists()
        || git_dir.join("CHERRY_PICK_HEAD").exists()
        || git_dir.join("REBASE_MERGE").is_dir()
        || git_dir.join("REBASE_APPLY").is_dir();

    ConflictStatus { has_conflict, conflict_count: None }
}

// ---------------------------------------------------------------------------
// Write (CLI)
// ---------------------------------------------------------------------------

pub async fn fetch(project: &Project) -> ProjectOperationResult {
    run_git_command(project, &["fetch", "--prune"]).await
}

pub async fn run_git_command(project: &Project, args: &[&str]) -> ProjectOperationResult {
    let cmd_str = format!("git {}", args.join(" "));
    let path = project.path.clone();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || {
        let result = Command::new("git").args(&args_owned).current_dir(&path).output();
        match result {
            Ok(output) => {
                let exit_code = output.status.code().unwrap_or(-1);
                ProjectOperationResult {
                    project_id: project_id.clone(),
                    success: output.status.success(),
                    commands_executed: vec![cmd_str.clone()],
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code: Some(exit_code),
                    error_message: if output.status.success() { None } else {
                        Some(format!("exit code {exit_code}"))
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
