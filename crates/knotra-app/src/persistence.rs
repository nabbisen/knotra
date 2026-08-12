//! Workspace definition and operation history persistence.

use knotra_vcs::model::{operation::OperationLog, workspace::Workspace};
use serde::{Deserialize, Serialize};

use crate::config::AppPaths;

// ---------------------------------------------------------------------------
// Workspace persistence
// ---------------------------------------------------------------------------

/// On-disk representation of a workspace (TOML).
#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceFile {
    workspace: Workspace,
}

/// Load all workspaces from the workspaces directory.
/// Returns an empty list (not an error) when the directory does not exist.
pub fn load_workspaces(paths: &AppPaths) -> (Vec<Workspace>, Vec<String>) {
    let dir = &paths.workspaces_dir;
    let mut workspaces = Vec::new();
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (workspaces, errors),
        Err(e) => {
            errors.push(format!("cannot read workspaces dir: {e}"));
            return (workspaces, errors);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<WorkspaceFile>(&text) {
                Ok(wf) => workspaces.push(wf.workspace),
                Err(e) => errors.push(format!("{}: parse error: {e}", path.display())),
            },
            Err(e) => errors.push(format!("{}: read error: {e}", path.display())),
        }
    }

    (workspaces, errors)
}

/// Persist a workspace to disk, atomically (Handoff 033 Task A) — overwrites
/// the same `<uuid>.toml` on every edit.
pub fn save_workspace(workspace: &Workspace, paths: &AppPaths) -> Result<(), String> {
    std::fs::create_dir_all(&paths.workspaces_dir)
        .map_err(|e| format!("cannot create workspaces dir: {e}"))?;

    let file_name = format!("{}.toml", workspace.id);
    let path = paths.workspaces_dir.join(file_name);

    let wf = WorkspaceFile {
        workspace: workspace.clone(),
    };
    let text = toml::to_string_pretty(&wf).map_err(|e| format!("serialization error: {e}"))?;
    crate::atomic_write::write(&path, text).map_err(|e| format!("write error: {e}"))
}

/// Remove a persisted workspace file.
///
/// Missing files are treated as already removed so in-memory cleanup can
/// proceed for workspaces loaded before the file disappeared.
pub fn delete_workspace_file(workspace: &Workspace, paths: &AppPaths) -> Result<(), String> {
    let file_name = format!("{}.toml", workspace.id);
    let path = paths.workspaces_dir.join(file_name);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("delete error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Operation history persistence
// ---------------------------------------------------------------------------

/// Persist one operation log entry as a JSON file, atomically (Handoff 033
/// Task A) — lower severity than the config/workspace sites since each call
/// targets a fresh timestamped filename rather than overwriting one, but
/// fixed for uniformity: a torn write would otherwise leave a partial file
/// `load_recent_logs` has to skip on parse failure instead of never
/// producing one.
pub fn save_operation_log(log: &OperationLog, paths: &AppPaths) -> Result<(), String> {
    std::fs::create_dir_all(&paths.history_dir)
        .map_err(|e| format!("cannot create history dir: {e}"))?;

    let ts = log.result.started_at.format("%Y%m%dT%H%M%SZ");
    let file_name = format!("{}_{}.json", ts, log.result.operation_id);
    let path = paths.history_dir.join(file_name);

    let text =
        serde_json::to_string_pretty(log).map_err(|e| format!("serialization error: {e}"))?;
    crate::atomic_write::write(&path, text).map_err(|e| format!("write error: {e}"))
}

/// Load the most recent `limit` operation logs from the history directory.
pub fn load_recent_logs(paths: &AppPaths, limit: usize) -> Vec<OperationLog> {
    let dir = &paths.history_dir;
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return Vec::new(),
    };

    // Sort descending by file name (timestamp prefix).
    entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    entries
        .into_iter()
        .take(limit)
        .filter_map(|e| {
            let text = std::fs::read_to_string(e.path()).ok()?;
            serde_json::from_str::<OperationLog>(&text).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::model::operation::{
        OperationId, OperationKind, OperationResult, ProjectOperationOutcome,
        ProjectOperationResult,
    };

    fn paths_in(tmp: &tempfile::TempDir) -> AppPaths {
        AppPaths {
            config_file: tmp.path().join("config.toml"),
            workspaces_dir: tmp.path().join("workspaces"),
            history_dir: tmp.path().join("history"),
        }
    }

    /// RFC-046 D4/R7: an entry written before D1's contract was enforced
    /// holds rendered prose in `skip_reason`, not a code. D4 chose not to
    /// migrate existing log files — this pins that decision as a test, so a
    /// later change cannot quietly start discarding or mangling pre-fix
    /// records. Covers the full chain R7 names: the record survives
    /// `save_operation_log` -> `load_recent_logs` (no panic, not dropped),
    /// and still renders through the same fallback the on-screen path uses.
    #[test]
    fn a_pre_fix_prose_skip_reason_round_trips_and_still_renders() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = paths_in(&tmp);
        let now = chrono::Utc::now();
        let prose = "このプロジェクトは今は確認できません。";

        let log = OperationLog {
            result: OperationResult {
                operation_id: OperationId::new(),
                kind: OperationKind::Fetch,
                started_at: now,
                finished_at: now,
                per_project: vec![ProjectOperationResult {
                    project_id: knotra_vcs::ProjectId::new(),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some(prose.to_owned()),
                    commands_executed: Vec::new(),
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    error_message: None,
                }],
                rollback_attempted: false,
                rollback_succeeded: None,
            },
            recovery_hints: Vec::new(),
        };

        save_operation_log(&log, &paths).expect("save a pre-fix record");

        let loaded = load_recent_logs(&paths, 10);
        assert_eq!(loaded.len(), 1, "a pre-fix record must not be dropped");
        assert_eq!(
            loaded[0].result.per_project[0].skip_reason.as_deref(),
            Some(prose),
            "a pre-fix prose value must round-trip byte-for-byte"
        );

        let state = crate::state::AppState::new(crate::config::AppConfig::default());
        let rendered = crate::view::skip_reason_display(
            &state,
            loaded[0].result.per_project[0]
                .skip_reason
                .as_deref()
                .unwrap(),
        );
        assert_eq!(
            rendered, prose,
            "an unrecognised value must render verbatim -- no panic, no blank"
        );
    }
}
