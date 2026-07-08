use std::time::SystemTime;

use anyhow::{Context, Result};
use endringer_backend_core::types::{BranchInfo, CommitId, CommitInfo, SortOrder};
use gix::Repository;

use crate::util::{gix_id_to_commit_id, seconds_to_systemtime};

mod util;

pub(crate) fn local_branches(repository: &Repository) -> Result<Vec<BranchInfo>> {
    util::branches(repository, "refs/heads/")
}

pub(crate) fn remote_branches(repository: &Repository) -> Result<Vec<BranchInfo>> {
    util::branches(repository, "refs/remotes/")
}

pub(crate) fn list_commits(repository: &Repository) -> Result<Vec<CommitInfo>> {
    collect_commits(repository, |_| true)
}

pub(crate) fn list_commits_sorted(
    repository: &Repository,
    order: SortOrder,
) -> Result<Vec<CommitInfo>> {
    let mut commits = list_commits(repository)?;
    apply_commit_sort(&mut commits, order);
    Ok(commits)
}

pub(crate) fn log_since(
    repository: &Repository,
    since: SystemTime,
    until: SystemTime,
) -> Result<Vec<CommitInfo>> {
    collect_commits(repository, |ts| ts >= since && ts <= until)
}

fn collect_commits(
    repository: &Repository,
    predicate: impl Fn(SystemTime) -> bool,
) -> Result<Vec<CommitInfo>> {
    let head = repository.head()?;
    let head_id = head
        .id()
        .ok_or_else(|| anyhow::anyhow!("HEAD is not pointing to a commit"))?;

    let mut history = Vec::new();

    for info in head_id.ancestors().all()? {
        let info = info?;
        let commit = info.object()?;

        let message = commit.message()?;
        let author = commit.author()?;
        let committer = commit.committer()?;
        let author_time = author.time().context("failed to read author timestamp")?;
        let committer_time = committer.time().context("failed to read committer timestamp")?;

        let timestamp = seconds_to_systemtime(author_time.seconds);

        if !predicate(timestamp) {
            continue;
        }

        // Collect parent commit IDs from the ancestry graph entry.
        let parents: Vec<CommitId> = info
            .parent_ids
            .iter()
            .copied()
            .map(gix_id_to_commit_id)
            .collect();

        history.push(CommitInfo {
            commit_id: gix_id_to_commit_id(info.id),
            parents,
            summary: message.summary().to_string(),
            author: author.name.to_string(),
            committer: committer.name.to_string(),
            timestamp,
            committer_timestamp: seconds_to_systemtime(committer_time.seconds),
        });
    }

    Ok(history)
}

fn apply_commit_sort(commits: &mut Vec<CommitInfo>, order: SortOrder) {
    match order {
        SortOrder::NewestFirst => commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
        SortOrder::OldestFirst => commits.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)),
        SortOrder::ByName => commits.sort_by(|a, b| a.summary.cmp(&b.summary)),
    }
}

/// O(1) commit lookup by [`CommitId`].
pub(crate) fn find_commit(repository: &Repository, id: &CommitId) -> Result<CommitInfo> {
    let oid = gix::ObjectId::from_hex(id.to_string().as_bytes())
        .map_err(|_| anyhow::anyhow!("invalid commit id '{}'", id))?;

    let obj = repository
        .find_object(oid)
        .map_err(|e| anyhow::anyhow!("commit '{}' not found: {e}", id.short()))?;
    let commit = obj
        .try_into_commit()
        .map_err(|_| anyhow::anyhow!("object '{}' is not a commit", id.short()))?;

    let author = commit.author()?;
    let committer = commit.committer()?;
    let author_time = author.time().context("author timestamp")?;
    let committer_time = committer.time().context("committer timestamp")?;
    let message = commit.message()?;

    let parents: Vec<CommitId> = commit
        .parent_ids()
        .map(|id| gix_id_to_commit_id(id.detach()))
        .collect();

    Ok(CommitInfo {
        commit_id: id.clone(),
        parents,
        author: author.name.to_string(),
        committer: committer.name.to_string(),
        summary: message.summary().to_string(),
        timestamp: seconds_to_systemtime(author_time.seconds),
        committer_timestamp: seconds_to_systemtime(committer_time.seconds),
    })
}
