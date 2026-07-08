//! Linked worktree listing.

use std::path::PathBuf;

use anyhow::Result;
use gix::bstr::ByteSlice;
use endringer_backend_core::types::WorktreeInfo;
use gix::Repository;

/// Returns all linked worktrees for this repository.
///
/// The main worktree is not included; only worktrees created with
/// `git worktree add` appear here. Results are sorted by worktree id.
pub(crate) fn worktrees(repo: &Repository) -> Result<Vec<WorktreeInfo>> {
    let proxies = repo.worktrees()?;
    let mut out = Vec::with_capacity(proxies.len());

    for proxy in proxies {
        let id = proxy.id().to_str_lossy().into_owned();
        let is_locked = proxy.is_locked();

        // The working directory path.
        let path: PathBuf = proxy.base().unwrap_or_default();

        // Read the HEAD file from the worktree's private git dir to determine
        // the current branch without opening a second repository.
        let head_file = proxy.git_dir().join("HEAD");
        let current_branch = match std::fs::read_to_string(&head_file) {
            Ok(content) => parse_head(&content),
            Err(_) => "(inaccessible)".to_owned(),
        };

        out.push(WorktreeInfo { id, path, current_branch, is_locked });
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Converts the raw content of a `HEAD` file into a branch name or
/// `"(detached)"`.
fn parse_head(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(rest) = trimmed.strip_prefix("ref: refs/heads/") {
        rest.to_owned()
    } else if let Some(rest) = trimmed.strip_prefix("ref: ") {
        rest.to_owned()
    } else {
        // Detached HEAD — content is a raw OID.
        "(detached)".to_owned()
    }
}
