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

/// `load_recent_logs`'s result (RFC-047 D2/D3): the logs themselves plus
/// what could not be produced, so a caller can state the loss instead of
/// rendering it as silence.
pub struct LoadedLogs {
    pub logs: Vec<OperationLog>,
    /// Directory entries that could not be read or parsed as an
    /// `OperationLog`, within the most-recent-`limit` window actually
    /// requested (RFC-047 D1) — not a count of every bad file that may sit
    /// further back in the directory, unrequested.
    pub unreadable: usize,
    /// The history directory itself could not be read (e.g. a permissions
    /// failure or a missing mount) — distinct from the directory never
    /// having been created, which is genuinely "no history yet" (RFC-047
    /// D3): `save_operation_log` creates the directory on first write, so a
    /// `NotFound` here is a first run, not a loss.
    pub directory_unreadable: bool,
}

/// Load the most recent `limit` operation logs from the history directory.
///
/// RFC-047 D1: filters before taking. `read_dir` yields every directory
/// entry — a corrupt file, a stray `.DS_Store`, a half-written file — and
/// the previous `.take(limit)` ran before parsing, so any one of those
/// consumed a slot a valid older entry could have filled. This walks
/// entries newest-first and keeps going until `limit` *valid* logs are
/// collected (or the directory is exhausted), so `limit` means what its
/// name says.
pub fn load_recent_logs(paths: &AppPaths, limit: usize) -> LoadedLogs {
    let dir = &paths.history_dir;
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return LoadedLogs {
                logs: Vec::new(),
                unreadable: 0,
                directory_unreadable: false,
            };
        }
        Err(_) => {
            return LoadedLogs {
                logs: Vec::new(),
                unreadable: 0,
                directory_unreadable: true,
            };
        }
    };

    // Sort descending by file name (timestamp prefix).
    entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    let mut logs = Vec::new();
    let mut unreadable = 0;
    for entry in entries {
        if logs.len() >= limit {
            break;
        }
        let parsed = std::fs::read_to_string(entry.path())
            .ok()
            .and_then(|text| serde_json::from_str::<OperationLog>(&text).ok());
        match parsed {
            Some(log) => logs.push(log),
            None => unreadable += 1,
        }
    }

    LoadedLogs {
        logs,
        unreadable,
        directory_unreadable: false,
    }
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

    /// A minimal, valid log at an explicit, controllable timestamp — RFC-047's
    /// tests need deterministic newest-first ordering, which `Utc::now()`
    /// calls a few microseconds apart cannot reliably guarantee at
    /// second-resolution filenames.
    fn log_at(seconds: i64) -> OperationLog {
        let ts = chrono::DateTime::from_timestamp(seconds, 0).unwrap();
        OperationLog {
            result: OperationResult {
                operation_id: OperationId::new(),
                kind: OperationKind::Fetch,
                started_at: ts,
                finished_at: ts,
                per_project: Vec::new(),
                rollback_attempted: false,
                rollback_succeeded: None,
            },
            recovery_hints: Vec::new(),
        }
    }

    /// A file `load_recent_logs` will encounter and fail to parse, named to
    /// sort at the given position among real log files (same
    /// `{timestamp}_{suffix}.json` shape `save_operation_log` uses).
    fn write_corrupt_file(paths: &AppPaths, seconds: i64) {
        std::fs::create_dir_all(&paths.history_dir).expect("create history dir");
        let ts = chrono::DateTime::from_timestamp(seconds, 0)
            .unwrap()
            .format("%Y%m%dT%H%M%SZ");
        let path = paths.history_dir.join(format!("{ts}_corrupt.json"));
        std::fs::write(path, "not valid json").expect("write corrupt file");
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

        let loaded = load_recent_logs(&paths, 10).logs;
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

    /// RFC-047 D1, the reordering fix itself. Five valid logs plus one
    /// corrupt file *newer than all of them*, `limit` smaller than the
    /// total file count. At `9db1296` (before this handoff)
    /// `.take(limit)` ran before parsing, so the corrupt file — sorted
    /// first — consumed one of the 3 requested slots and this assertion
    /// failed with `left: 2, right: 3`. Confirmed by running this exact
    /// scenario against the unmodified baseline before writing the fix,
    /// reported verbatim in the review request per the handoff's request —
    /// a reordering fix whose test passes before the change was not
    /// testing the reorder.
    #[test]
    fn load_recent_logs_fills_the_limit_with_valid_logs_despite_a_newer_corrupt_file() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = paths_in(&tmp);

        for s in [100, 101, 102, 103, 104] {
            save_operation_log(&log_at(s), &paths).expect("save a valid log");
        }
        write_corrupt_file(&paths, 105); // newer than every valid log

        let loaded = load_recent_logs(&paths, 3);
        assert_eq!(
            loaded.logs.len(),
            3,
            "expected the 3 most recent VALID logs, not limit - 1"
        );
        assert_eq!(loaded.unreadable, 1);
        assert!(!loaded.directory_unreadable);
    }

    /// RFC-047 D2: a corrupt file among valid ones is reported, not just
    /// silently absorbed — the valid ones all still load (D1's slot
    /// behaviour), and the skipped count says one entry could not be read.
    #[test]
    fn load_recent_logs_reports_the_unreadable_count() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = paths_in(&tmp);

        for s in [100, 101, 102] {
            save_operation_log(&log_at(s), &paths).expect("save a valid log");
        }
        write_corrupt_file(&paths, 103);

        let loaded = load_recent_logs(&paths, 10);
        assert_eq!(loaded.logs.len(), 3, "all three valid logs must load");
        assert_eq!(loaded.unreadable, 1);
        assert!(!loaded.directory_unreadable);
    }

    /// RFC-047 D3: an unreadable directory is its own, distinct state, not
    /// indistinguishable from "no history yet". A directory that was never
    /// created (no prior `save_operation_log` call) is the first-run case
    /// and must NOT report `directory_unreadable` — only a directory that
    /// exists but genuinely cannot be read should.
    #[test]
    #[cfg(unix)]
    fn load_recent_logs_reports_an_unreadable_directory_distinctly() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = paths_in(&tmp);
        std::fs::create_dir_all(&paths.history_dir).expect("create history dir");
        std::fs::set_permissions(&paths.history_dir, std::fs::Permissions::from_mode(0o000))
            .expect("remove read permission");

        let loaded = load_recent_logs(&paths, 10);

        // Restore permissions before the tempdir is dropped, or cleanup fails.
        std::fs::set_permissions(&paths.history_dir, std::fs::Permissions::from_mode(0o755))
            .expect("restore permission for cleanup");

        assert!(loaded.logs.is_empty());
        assert_eq!(loaded.unreadable, 0);
        assert!(
            loaded.directory_unreadable,
            "a permissions failure must be reported, not read as an empty directory"
        );
    }

    /// The other half of D3: a directory that has simply never been
    /// created is genuinely "no history yet", not an error.
    #[test]
    fn load_recent_logs_treats_a_missing_directory_as_no_history_not_an_error() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = paths_in(&tmp); // history_dir never created

        let loaded = load_recent_logs(&paths, 10);

        assert!(loaded.logs.is_empty());
        assert_eq!(loaded.unreadable, 0);
        assert!(!loaded.directory_unreadable);
    }
}
