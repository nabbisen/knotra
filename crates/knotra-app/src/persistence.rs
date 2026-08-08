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
