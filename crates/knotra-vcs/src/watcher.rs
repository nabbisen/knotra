//! File-system change detection for registered repositories.
//!
//! This module provides a **polling-based** watcher that checks a set of
//! Git/jj sentinel files for modification time changes. It is deliberately
//! simple and does not rely on OS-level inotify/kqueue/ReadDirectoryChanges
//! to keep cross-platform behaviour predictable and the dependency footprint
//! small.
//!
//! The watcher is optional and disabled by default. It can be enabled in
//! application settings and runs as part of the iced `Subscription`.
//!
//! # Design
//!
//! For each repository we watch a small set of sentinel files whose mtime
//! change whenever the user performs a relevant VCS action:
//!
//! | File | Indicates |
//! |---|---|
//! | `.git/HEAD` | Branch switch, commit, reset |
//! | `.git/index` | Stage/unstage |
//! | `.git/refs/` (dir mtime) | Branch creation/deletion, tag |
//! | `.jj/working_copy/` (dir mtime) | jj working-copy change |
//! | `.jj/op_heads/` (dir mtime) | jj operation-log change |
//!
//! The poller runs at a configurable interval (default 2 s) and emits a
//! `FsChangeEvent` for each repository where any sentinel changed.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::model::project::ProjectId;

/// A file-system change detected in one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsChangeEvent {
    pub project_id: ProjectId,
    pub project_path: String,
}

/// Snapshot of sentinel file mtimes for one repository.
#[derive(Debug, Clone)]
struct RepoSnapshot {
    /// Per-sentinel path → last-seen mtime.
    sentinels: HashMap<PathBuf, Option<SystemTime>>,
}

impl RepoSnapshot {
    fn for_repo(repo_path: &Path) -> Self {
        let candidates = sentinel_paths(repo_path);
        let sentinels = candidates
            .into_iter()
            .map(|p| {
                let mtime = std::fs::metadata(&p).ok().and_then(|m| m.modified().ok());
                (p, mtime)
            })
            .collect();
        RepoSnapshot { sentinels }
    }

    /// Returns true if any sentinel mtime differs from `other`.
    fn changed_vs(&self, other: &RepoSnapshot) -> bool {
        for (path, mtime) in &self.sentinels {
            if other.sentinels.get(path) != Some(mtime) {
                return true;
            }
        }
        false
    }
}

fn sentinel_paths(repo_path: &Path) -> Vec<PathBuf> {
    let git_dir = repo_path.join(".git");
    let jj_dir = repo_path.join(".jj");

    if jj_dir.is_dir() {
        // jj repository.
        vec![jj_dir.join("working_copy"), jj_dir.join("op_heads")]
    } else if git_dir.is_dir() || git_dir.is_file() {
        // Git repository (git_dir can be a file for worktrees).
        let real_git = if git_dir.is_file() {
            // Worktree: read the gitdir file to find the real .git location.
            std::fs::read_to_string(&git_dir)
                .ok()
                .map(|s| PathBuf::from(s.trim().trim_start_matches("gitdir: ")))
                .unwrap_or_else(|| git_dir.clone())
        } else {
            git_dir
        };
        vec![
            real_git.join("HEAD"),
            real_git.join("index"),
            real_git.join("refs"),
        ]
    } else {
        vec![]
    }
}

/// Stateful poller: holds the last-seen snapshots per project.
#[derive(Debug, Default)]
pub struct FsPoller {
    snapshots: HashMap<ProjectId, RepoSnapshot>,
}

impl FsPoller {
    /// Poll all given projects. Returns one `FsChangeEvent` per changed repo.
    ///
    /// The first call always returns empty (establishes the baseline).
    pub fn poll(
        &mut self,
        projects: &[(ProjectId, String)], // (id, path)
    ) -> Vec<FsChangeEvent> {
        let mut changed = Vec::new();

        for (id, path) in projects {
            let current = RepoSnapshot::for_repo(Path::new(path));
            if let Some(prev) = self.snapshots.get(id)
                && current.changed_vs(prev)
            {
                changed.push(FsChangeEvent {
                    project_id: id.clone(),
                    project_path: path.clone(),
                });
            }
            // Update snapshot (first call establishes baseline, no event).
            self.snapshots.insert(id.clone(), current);
        }

        changed
    }

    /// Remove stale entries for projects that are no longer registered.
    pub fn prune(&mut self, active_ids: &[ProjectId]) {
        self.snapshots.retain(|id, _| active_ids.contains(id));
    }

    /// Force-invalidate the snapshot for one project (call after write ops).
    pub fn invalidate(&mut self, id: &ProjectId) {
        self.snapshots.remove(id);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::ProjectId;
    use std::io::Write;

    fn make_temp_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        // Create HEAD sentinel.
        let mut head = std::fs::File::create(git.join("HEAD")).unwrap();
        writeln!(head, "ref: refs/heads/main").unwrap();
        // Create index sentinel.
        std::fs::File::create(git.join("index")).unwrap();
        // Create refs sentinel dir.
        std::fs::create_dir(git.join("refs")).unwrap();
        dir
    }

    #[test]
    fn first_poll_establishes_baseline_no_events() {
        let dir = make_temp_git_repo();
        let id = ProjectId::new();
        let mut poller = FsPoller::default();
        let projects = vec![(id, dir.path().to_string_lossy().to_string())];
        let events = poller.poll(&projects);
        assert!(events.is_empty(), "first poll must not emit events");
    }

    #[test]
    fn second_poll_no_change_no_events() {
        let dir = make_temp_git_repo();
        let id = ProjectId::new();
        let path = dir.path().to_string_lossy().to_string();
        let mut poller = FsPoller::default();
        let projects = vec![(id.clone(), path.clone())];
        poller.poll(&projects); // baseline
        let events = poller.poll(&projects);
        assert!(events.is_empty(), "no change → no events");
    }

    #[test]
    fn modified_sentinel_triggers_event() {
        let dir = make_temp_git_repo();
        let id = ProjectId::new();
        let path = dir.path().to_string_lossy().to_string();
        let mut poller = FsPoller::default();
        let projects = vec![(id.clone(), path.clone())];
        poller.poll(&projects); // baseline

        // Touch HEAD (simulate a commit or branch switch).
        // Sleep briefly to ensure mtime changes on filesystems with 1-second resolution.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let head_path = dir.path().join(".git").join("HEAD");
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&head_path)
                .unwrap();
            writeln!(f, "ref: refs/heads/feature/x").unwrap();
        }
        // Force mtime update via touch.
        filetime::set_file_mtime(&head_path, filetime::FileTime::now()).unwrap();

        let events = poller.poll(&projects);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].project_id, id);
    }

    #[test]
    fn prune_removes_stale_snapshots() {
        let dir = make_temp_git_repo();
        let id = ProjectId::new();
        let path = dir.path().to_string_lossy().to_string();
        let mut poller = FsPoller::default();
        let projects = vec![(id.clone(), path)];
        poller.poll(&projects); // establishes snapshot
        assert!(poller.snapshots.contains_key(&id));

        poller.prune(&[]);
        assert!(poller.snapshots.is_empty());
    }
}
