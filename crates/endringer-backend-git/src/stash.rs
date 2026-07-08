//! Stash entry listing via the `refs/stash` reflog.

use anyhow::Result;
use endringer_backend_core::types::{CommitId, StashEntry};
use gix::Repository;

/// Returns all stash entries, newest first (`stash@{0}` first).
///
/// Reads the `logs/refs/stash` reflog file directly, which is the same
/// data source `git stash list` uses.
pub(crate) fn stash_entries(repo: &Repository) -> Result<Vec<StashEntry>> {
    let reflog_path = repo.git_dir().join("logs").join("refs").join("stash");
    let content = match std::fs::read_to_string(&reflog_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };

    // Reflog lines are oldest-first; reverse so stash@{0} comes first.
    let raw_lines: Vec<&str> = content.lines().collect();
    let entries: Vec<StashEntry> = raw_lines
        .iter()
        .rev()
        .enumerate()
        .filter_map(|(index, line)| parse_reflog_line(line, index))
        .collect();

    Ok(entries)
}

/// Parses one reflog line into a [`StashEntry`].
///
/// Format: `old_oid new_oid Name <email> timestamp +tz\tmessage`
fn parse_reflog_line(line: &str, index: usize) -> Option<StashEntry> {
    // Fields are separated by whitespace, the message follows a literal tab.
    let tab_pos = line.find('\t')?;
    let message = line[tab_pos + 1..].trim().to_owned();

    // The second token (index 1) is the "new" OID.
    let new_oid_hex = line.split_whitespace().nth(1)?;
    let commit_id = CommitId::from_hex(new_oid_hex).ok()?;

    Some(StashEntry { index, commit_id, message })
}
