use anyhow::{Context, Result};
use endringer_backend_core::types::StatusDigest;
use gix::Repository;

use crate::util::{gix_id_to_commit_id, seconds_to_systemtime};

pub(crate) fn status_digest(repository: &Repository) -> Result<StatusDigest> {
    let repo_name = repository
        .workdir()
        .and_then(|p| p.canonicalize().ok())
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_owned();

    let mut head = repository.head()?;
    let current_branch = if head.is_detached() {
        "(detached)".to_owned()
    } else {
        head.referent_name()
            .context("failed to resolve HEAD branch name")?
            .shorten()
            .to_string()
    };

    let commit = head.peel_to_commit()?;
    let last_commit_id = gix_id_to_commit_id(commit.id);
    let last_commit_summary = commit
        .message()
        .context("failed to read HEAD commit message")?
        .summary()
        .to_string();
    let last_commit_timestamp = seconds_to_systemtime(
        commit
            .time()
            .context("failed to read HEAD commit timestamp")?
            .seconds,
    );

    Ok(StatusDigest {
        repo_name,
        current_branch,
        last_commit_id,
        last_commit_summary,
        last_commit_timestamp,
    })
}
