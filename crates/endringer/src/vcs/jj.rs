//! Jujutsu (jj) status and operation implementations.
//!
//! All operations are performed via the `jj` CLI binary, since jj does not
//! expose a stable library interface. The output is parsed from structured
//! jj formats where available (e.g. `--format json` or template strings).

use chrono::Utc;

use crate::model::{
    operation::ProjectOperationResult,
    project::Project,
    status::{
        ConflictStatus, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind,
        WorkingTreeStatus,
    },
};

/// Read jj repository status via CLI subprocesses.
pub async fn read_status(project: &Project) -> ProjectStatus {
    let path = project.path.clone();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || read_status_blocking(&project_id, &path))
        .await
        .unwrap_or_else(|e| ProjectStatus {
            project_id: project.id.clone(),
            identity: RepositoryIdentity {
                path: project.path.clone(),
                vcs_kind: VcsKind::Jujutsu,
            },
            context: None,
            remote: Default::default(),
            working_tree: Default::default(),
            conflict: Default::default(),
            refreshed_at: Utc::now(),
            read_error: Some(format!("task join error: {e}")),
        })
}

fn read_status_blocking(
    project_id: &crate::model::project::ProjectId,
    path: &str,
) -> ProjectStatus {
    let context = read_jj_context(path);
    let working_tree = read_jj_working_tree(path);
    let conflict = read_jj_conflict(path);

    ProjectStatus {
        project_id: project_id.clone(),
        identity: RepositoryIdentity {
            path: path.to_owned(),
            vcs_kind: VcsKind::Jujutsu,
        },
        context,
        remote: RemoteStatus::default(), // jj remote tracking is complex; placeholder
        working_tree,
        conflict,
        refreshed_at: Utc::now(),
        read_error: None,
    }
}

fn run_jj(args: &[&str], cwd: &str) -> std::io::Result<std::process::Output> {
    std::process::Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
}

fn read_jj_context(path: &str) -> Option<VcsContext> {
    // `jj log -r @ --no-graph -T 'change_id.short() ++ "\n"'`
    let out = run_jj(
        &[
            "log",
            "-r",
            "@",
            "--no-graph",
            "-T",
            r#"change_id.short(8) ++ "|" ++ if(bookmarks, bookmarks.join(","), "") ++ "\n""#,
        ],
        path,
    )
    .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?;
    let mut parts = line.splitn(2, '|');
    let change_id = parts.next()?.trim().to_owned();
    let bookmark_raw = parts.next().unwrap_or("").trim();
    let bookmark = if bookmark_raw.is_empty() {
        None
    } else {
        Some(bookmark_raw.to_owned())
    };

    let label = match &bookmark {
        Some(b) => format!("@ {b} ({change_id})"),
        None => format!("@ {change_id}"),
    };

    Some(VcsContext {
        label,
        branch: None,
        jj_change_id: Some(change_id),
        jj_bookmark: bookmark,
        is_detached: false,
    })
}

fn read_jj_working_tree(path: &str) -> WorkingTreeStatus {
    // `jj diff --stat` counts changed files.
    let out = match run_jj(&["diff", "--stat"], path) {
        Ok(o) => o,
        Err(_) => return WorkingTreeStatus::default(),
    };

    if !out.status.success() {
        return WorkingTreeStatus::default();
    }

    let text = String::from_utf8_lossy(&out.stdout);
    // Last line is a summary like "3 files changed, ..."
    // Count non-summary lines as modified-file count.
    let file_lines = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with(' '))
        .count() as u32;

    WorkingTreeStatus {
        uncommitted_count: file_lines,
        untracked_count: 0,
    }
}

fn read_jj_conflict(path: &str) -> ConflictStatus {
    let out = match run_jj(
        &["log", "-r", "@", "--no-graph", "-T", "conflict\n"],
        path,
    ) {
        Ok(o) => o,
        Err(_) => return ConflictStatus::default(),
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let has_conflict = text.trim() == "true";

    ConflictStatus {
        has_conflict,
        conflict_count: None,
    }
}

/// Execute `jj git fetch` for a project.
pub async fn fetch(project: &Project) -> ProjectOperationResult {
    run_jj_command(project, &["git", "fetch"]).await
}

/// Run a jj CLI command and capture its output.
pub async fn run_jj_command(project: &Project, args: &[&str]) -> ProjectOperationResult {
    use std::process::Command;

    let cmd_str = format!("jj {}", args.join(" "));
    let result = Command::new("jj")
        .args(args)
        .current_dir(&project.path)
        .output();

    match result {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            ProjectOperationResult {
                project_id: project.id.clone(),
                success: output.status.success(),
                commands_executed: vec![cmd_str],
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_code: Some(exit_code),
                error_message: if output.status.success() {
                    None
                } else {
                    Some(format!("exit code {exit_code}"))
                },
            }
        }
        Err(e) => ProjectOperationResult {
            project_id: project.id.clone(),
            success: false,
            commands_executed: vec![cmd_str],
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            error_message: Some(format!("failed to spawn jj: {e}")),
        },
    }
}
