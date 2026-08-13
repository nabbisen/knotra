//! Jujutsu-specific read and write operations.
//!
//! All reads delegate to `endringer-async` (`AsyncRepository` →
//! `JjBackend` → gix, no `jj` binary required).
//! Writes still use the `jj` CLI where gix doesn't expose mutation APIs.

use chrono::Utc;
use std::process::Command;

use endringer_async::AsyncRepository;

use crate::model::{
    operation::{ProjectOperationOutcome, ProjectOperationResult, RecoveryHint},
    project::Project,
    status::{
        ContextTarget, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind,
        WorkingTreeStatus,
    },
};

// ---------------------------------------------------------------------------
// CLI helper
// ---------------------------------------------------------------------------

fn run_jj(args: &[&str], cwd: &str) -> std::io::Result<std::process::Output> {
    Command::new("jj").args(args).current_dir(cwd).output()
}

async fn run_jj_command(project: &Project, args: &[&str]) -> ProjectOperationResult {
    let cmd_str = format!("jj {}", args.join(" "));
    let path = project.path.clone();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let project_id = project.id.clone();

    tokio::task::spawn_blocking(move || {
        match Command::new("jj")
            .args(&args_owned)
            .current_dir(&path)
            .output()
        {
            Ok(output) => {
                let code = output.status.code().unwrap_or(-1);
                ProjectOperationResult {
                    project_id: project_id.clone(),
                    outcome: ProjectOperationOutcome::from_success(output.status.success()),
                    success: output.status.success(),
                    skip_reason: None,
                    commands_executed: vec![cmd_str.clone()],
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code: Some(code),
                    error_message: if output.status.success() {
                        None
                    } else {
                        Some(format!("exit {code}"))
                    },
                }
            }
            Err(e) => ProjectOperationResult {
                project_id,
                outcome: ProjectOperationOutcome::Failed,
                success: false,
                skip_reason: None,
                commands_executed: vec![cmd_str],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: Some(format!("failed to spawn jj: {e}")),
            },
        }
    })
    .await
    .unwrap_or_else(|e| ProjectOperationResult {
        project_id: project.id.clone(),
        outcome: ProjectOperationOutcome::Failed,
        success: false,
        skip_reason: None,
        commands_executed: vec![],
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        error_message: Some(format!("task join error: {e}")),
    })
}

/// Detect jj conflict status for the working copy.
///
/// Requires the `jj` binary.  When absent, returns
/// `detection_unavailable: true` so the UI shows "Unknown"
/// rather than a false "No conflict."
fn detect_jj_conflict(path: &str) -> crate::model::status::ConflictStatus {
    match std::process::Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "conflict\n"])
        .current_dir(path)
        .output()
    {
        Err(_) => crate::model::status::ConflictStatus {
            has_conflict: false,
            conflict_count: None,
            detection_unavailable: true,
        },
        Ok(o) => {
            let flag = String::from_utf8_lossy(&o.stdout).trim() == "true";
            crate::model::status::ConflictStatus {
                has_conflict: flag,
                conflict_count: None,
                detection_unavailable: false,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Read — via endringer-async (JjBackend → gix, no jj binary)
// ---------------------------------------------------------------------------

pub async fn read_status(project: &Project) -> ProjectStatus {
    let path = std::path::Path::new(&project.path);

    match AsyncRepository::open_jj(path).await {
        Err(e) => ProjectStatus {
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
            read_error: Some(format!("cannot open jj repository: {e}")),
        },
        Ok(repo) => {
            let context = repo.status_digest().await.ok().map(|d| VcsContext {
                label: d.current_branch.clone(),
                branch: None,
                jj_change_id: Some(d.last_commit_id.short()),
                jj_bookmark: if d.current_branch.starts_with("(detached") {
                    None
                } else {
                    Some(d.current_branch.clone())
                },
                is_detached: false,
            });

            let working_tree = repo
                .worktree_status()
                .await
                .ok()
                .map(|ws| WorkingTreeStatus {
                    uncommitted_count: (ws.staged.len() + ws.unstaged.len()) as u32,
                    untracked_count: ws.untracked.len() as u32,
                })
                .unwrap_or_default();

            let conflict = detect_jj_conflict(&project.path);

            ProjectStatus {
                project_id: project.id.clone(),
                identity: RepositoryIdentity {
                    path: project.path.clone(),
                    vcs_kind: VcsKind::Jujutsu,
                },
                context,
                remote: RemoteStatus::default(),
                working_tree,
                conflict,
                refreshed_at: Utc::now(),
                read_error: None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Context listing — via backend-async
// ---------------------------------------------------------------------------

pub async fn list_contexts(project: &Project) -> crate::model::status::ContextList {
    use crate::model::status::{ContextCandidate, ContextList, ContextTarget};

    let path = std::path::Path::new(&project.path);
    match AsyncRepository::open_jj(path).await {
        Err(e) => ContextList {
            project_id: project.id.clone(),
            vcs_kind: VcsKind::Jujutsu,
            candidates: vec![],
            warning: Some(e.to_string()),
        },
        Ok(repo) => {
            let current = repo
                .status_digest()
                .await
                .ok()
                .map(|d| d.last_commit_id.short())
                .unwrap_or_default();

            let mut candidates = Vec::new();

            // Bookmarks via local branches
            if let Ok(branches) = repo.local_branches().await {
                for b in branches {
                    candidates.push(ContextCandidate {
                        label: b.name.clone(),
                        target: ContextTarget::JjBookmark { name: b.name },
                        is_current: false,
                    });
                }
            }

            // Recent commits
            if let Ok(commits) = repo.list_commits().await {
                for c in commits.into_iter().take(20) {
                    let short = c.commit_id.short();
                    let label = if c.summary.is_empty() {
                        short.clone()
                    } else {
                        format!("{} {}", short, c.summary)
                    };
                    candidates.push(ContextCandidate {
                        label,
                        target: ContextTarget::JjChange { id: short.clone() },
                        is_current: short == current,
                    });
                }
            }

            candidates.sort_by(|a, b| b.is_current.cmp(&a.is_current).then(a.label.cmp(&b.label)));
            candidates.dedup_by(|a, b| a.target.display_target() == b.target.display_target());

            ContextList {
                project_id: project.id.clone(),
                vcs_kind: VcsKind::Jujutsu,
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
    _until_ref: Option<&str>,
) -> crate::model::changelog::ProjectCommits {
    use crate::model::changelog::{CommitEntry, ProjectCommits};

    let path = project.path.clone();
    let since = since_ref.to_owned();
    let pid = project.id.clone();
    let pname = project.name.clone();

    tokio::task::spawn_blocking(move || {
        // Use `jj log -r <bookmark>..@` — ref-based, no timestamp ambiguity.
        let rev = format!("{since}..@");
        let tmpl = concat!(
            "change_id.short(8)",
            r#" ++ "|" ++ description.first_line()"#,
            r#" ++ "|" ++ author.name()"#,
            r#" ++ "|" ++ committer.timestamp().format("%Y-%m-%dT%H:%M:%S+00:00")"#,
            r#" ++ "
""#,
        );
        let out = std::process::Command::new("jj")
            .args(["log", "-r", &rev, "--no-graph", "-T", tmpl])
            .current_dir(&path)
            .output();

        match out {
            Err(e) => ProjectCommits {
                project_id: pid,
                project_name: pname,
                since_ref: since,
                entries: vec![],
                error: Some(format!("jj not available: {e}")),
            },
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_owned();
                ProjectCommits {
                    project_id: pid,
                    project_name: pname,
                    since_ref: since,
                    entries: vec![],
                    error: Some(if stderr.is_empty() {
                        format!("jj log exited with code {:?}", o.status.code())
                    } else {
                        stderr
                    }),
                }
            }
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let entries = text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| {
                        let mut p = l.splitn(4, '|');
                        let hash = p.next()?.to_owned();
                        let subject = p.next()?.to_owned();
                        let author = p.next()?.to_owned();
                        let date = p
                            .next()?
                            .trim()
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .ok()?;
                        Some(CommitEntry {
                            hash,
                            subject,
                            author,
                            date,
                        })
                    })
                    .collect();
                ProjectCommits {
                    project_id: pid,
                    project_name: pname,
                    since_ref: since,
                    entries,
                    error: None,
                }
            }
        }
    })
    .await
    .unwrap_or_else(|e| ProjectCommits {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        since_ref: since_ref.to_owned(),
        entries: vec![],
        error: Some(format!("task join: {e}")),
    })
}

/// RFC-039 D1/D7: the most recent `limit` commits, no since-ref.
///
/// **Verified against a real `jj 0.44.0` binary and real repositories**
/// (jj was not installed in this project's environment before this
/// handoff; see Review Request 074 and its ruling) — not inferred from
/// documentation alone, per D7's explicit requirement.
///
/// The revset is `..@-`, **not** `..@` — this is a deliberate, stated
/// difference from `log_since`'s `{since}..@` shape, not an oversight.
/// `@` is jj's working-copy commit, which always exists and is frequently
/// empty and description-less (jj auto-creates a new one after every
/// `jj commit`/`jj describe`). Confirmed directly: `jj log -r ..@ -n <N>`
/// on a real repo returns the empty working-copy commit as its *first*
/// entry, ahead of the real commits beneath it, and on a **brand-new
/// repository with zero real commits**, `..@` returns one spurious
/// empty/authorless entry while `..@-` (excluding the working copy,
/// starting from its parent) correctly returns nothing — the right input
/// for D5's "no commits yet" state. `git log -n <limit>` has no equivalent
/// of "the commit currently being written" to begin with, so `..@-` is the
/// revset that actually answers the same question `-n <limit>` answers for
/// git; `..@` would not.
pub async fn recent_commits(
    project: &Project,
    limit: usize,
) -> crate::model::changelog::RecentCommits {
    use crate::model::changelog::{CommitEntry, RecentCommits};

    let path = project.path.clone();
    let pid = project.id.clone();
    let limit = limit.to_string();

    tokio::task::spawn_blocking(move || {
        let tmpl = concat!(
            "change_id.short(8)",
            r#" ++ "|" ++ description.first_line()"#,
            r#" ++ "|" ++ author.name()"#,
            r#" ++ "|" ++ committer.timestamp().format("%Y-%m-%dT%H:%M:%S+00:00")"#,
            r#" ++ "
""#,
        );
        let out = std::process::Command::new("jj")
            .args(["log", "-r", "..@-", "--no-graph", "-T", tmpl, "-n", &limit])
            .current_dir(&path)
            .output();

        match out {
            Err(e) => RecentCommits {
                project_id: pid,
                entries: vec![],
                error: Some(format!("jj not available: {e}")),
            },
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_owned();
                RecentCommits {
                    project_id: pid,
                    entries: vec![],
                    error: Some(if stderr.is_empty() {
                        format!("jj log exited with code {:?}", o.status.code())
                    } else {
                        stderr
                    }),
                }
            }
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let entries = text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| {
                        let mut p = l.splitn(4, '|');
                        let hash = p.next()?.to_owned();
                        let subject = p.next()?.to_owned();
                        let author = p.next()?.to_owned();
                        let date = p
                            .next()?
                            .trim()
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .ok()?;
                        Some(CommitEntry {
                            hash,
                            subject,
                            author,
                            date,
                        })
                    })
                    .collect();
                RecentCommits {
                    project_id: pid,
                    entries,
                    error: None,
                }
            }
        }
    })
    .await
    .unwrap_or_else(|e| RecentCommits {
        project_id: project.id.clone(),
        entries: vec![],
        error: Some(format!("task join: {e}")),
    })
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

pub async fn fetch(project: &Project) -> ProjectOperationResult {
    run_jj_command(project, &["git", "fetch"]).await
}

pub async fn smart_pull(
    project: &Project,
    _stash_dirty: bool,
) -> (ProjectOperationResult, Option<RecoveryHint>) {
    let fetch_res = fetch(project).await;
    if !fetch_res.success {
        let hint = RecoveryHint {
            project_id: project.id.clone(),
            situation: "jj git fetch failed".to_owned(),
            suggested_commands: vec![format!("cd {:?} && jj git fetch", project.path)],
            see_also: Some("https://jj-vcs.github.io/jj/latest/git-compatibility/".to_owned()),
        };
        return (fetch_res, Some(hint));
    }

    let conflict_st = detect_jj_conflict(&project.path);
    let has_conflict = conflict_st.has_conflict;

    let hint = if has_conflict {
        Some(RecoveryHint {
            project_id: project.id.clone(),
            situation: "jj detected conflict after fetch".to_owned(),
            suggested_commands: vec![
                format!("cd {:?} && jj status", project.path),
                format!("cd {:?} && jj resolve", project.path),
            ],
            see_also: Some("https://jj-vcs.github.io/jj/latest/conflicts/".to_owned()),
        })
    } else {
        None
    };

    (fetch_res, hint)
}

pub async fn switch_context(
    project: &Project,
    target: &ContextTarget,
) -> (ProjectOperationResult, Option<RecoveryHint>) {
    let Some(target_text) = (match target {
        ContextTarget::JjBookmark { name } | ContextTarget::JjChange { id: name } => {
            Some(name.clone())
        }
        ContextTarget::Manual { vcs_kind, input } if *vcs_kind == VcsKind::Jujutsu => {
            Some(input.clone())
        }
        _ => None,
    }) else {
        return (
            ProjectOperationResult {
                project_id: project.id.clone(),
                outcome: ProjectOperationOutcome::Failed,
                success: false,
                skip_reason: None,
                commands_executed: vec![],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: Some("target is not a jj work area".to_owned()),
            },
            None,
        );
    };
    let result = run_jj_command(project, &["edit", &target_text]).await;
    let hint = if !result.success {
        Some(RecoveryHint {
            project_id: project.id.clone(),
            situation: format!("jj edit {} failed", target_text),
            suggested_commands: vec![format!("cd {:?} && jj edit {}", project.path, target_text)],
            see_also: Some("https://jj-vcs.github.io/jj/latest/working-copy/".to_owned()),
        })
    } else {
        None
    };
    (result, hint)
}

pub async fn bookmark_create(project: &Project, name: &str) -> ProjectOperationResult {
    run_jj_command(project, &["bookmark", "create", name, "-r", "@"]).await
}

pub async fn bookmark_delete(project: &Project, name: &str) -> ProjectOperationResult {
    run_jj_command(project, &["bookmark", "delete", name]).await
}

pub async fn list_conflicted_files(
    project: &Project,
) -> crate::model::conflict::ProjectConflictDetail {
    use crate::model::conflict::{ConflictMarker, ConflictedFile, ProjectConflictDetail};

    let path = project.path.clone();
    let project_id = project.id.clone();
    let project_name = project.name.clone();

    tokio::task::spawn_blocking(move || {
        match Command::new("jj")
            .args(["resolve", "--list"])
            .current_dir(&path)
            .output()
        {
            Err(e) => ProjectConflictDetail {
                project_id,
                project_name,
                conflicted_files: vec![],
                read_error: Some(e.to_string()),
            },
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let files = text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| ConflictedFile {
                        path: l.trim().to_owned(),
                        marker: ConflictMarker::BothModified,
                    })
                    .collect();
                ProjectConflictDetail {
                    project_id,
                    project_name,
                    conflicted_files: files,
                    read_error: None,
                }
            }
        }
    })
    .await
    .unwrap_or_else(|e| crate::model::conflict::ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: vec![],
        read_error: Some(format!("task join error: {e}")),
    })
}

pub async fn validate_for_freeze(
    project: &crate::model::project::Project,
    freeze_name: &str,
    included: bool,
) -> crate::model::operation::FreezeValidationEntry {
    use crate::model::operation::FreezeValidationEntry;

    let path = std::path::Path::new(&project.path).to_path_buf();
    let name = freeze_name.to_owned();
    let project_id_err = project.id.clone();
    let project_name_err = project.name.clone();
    let project = project.clone();

    tokio::task::spawn_blocking(move || {
        use endringer_core::backend::VcsBackend;

        // Open jj backend to check dirty state
        let is_dirty = endringer_jj::JjBackend::open(&path)
            .ok()
            .and_then(|b| b.is_dirty().ok())
            .unwrap_or(false);

        let conflict_status = detect_jj_conflict(path.to_str().unwrap_or(""));
        let conflict = conflict_status.has_conflict || conflict_status.detection_unavailable;

        let bm_exists = run_jj(&["bookmark", "list"], path.to_str().unwrap_or(""))
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|l| l.trim().starts_with(name.as_str()))
            })
            .unwrap_or(false);

        let is_clean = !is_dirty && !conflict;
        let mut blockers = vec![];
        if conflict {
            blockers.push("unresolved conflict".to_owned());
        }
        if is_dirty {
            blockers.push("uncommitted diff".to_owned());
        }
        if bm_exists {
            blockers.push(format!("bookmark '{}' already exists", name));
        }

        FreezeValidationEntry {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            included,
            is_clean,
            tag_exists: bm_exists,
            notes: vec![],
            blockers,
        }
    })
    .await
    .unwrap_or_else(|e| crate::model::operation::FreezeValidationEntry {
        project_id: project_id_err,
        project_name: project_name_err,
        included,
        is_clean: false,
        tag_exists: false,
        notes: vec![],
        blockers: vec![format!("task join error: {e}")],
    })
}
