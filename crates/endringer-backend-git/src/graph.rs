//! Commit-graph operations: merge base and ancestry checks.

use anyhow::Result;
use endringer_backend_core::types::CommitId;
use gix::Repository;

use crate::util::gix_id_to_commit_id;

fn parse_oid(id: &CommitId) -> Result<gix::ObjectId> {
    gix::ObjectId::from_hex(id.to_string().as_bytes())
        .map_err(|_| anyhow::anyhow!("invalid commit id '{}'", id))
}

/// Returns the best common ancestor of `a` and `b`, or `None` if there is
/// none (unrelated histories).
///
/// Uses gix's `Repository::merge_base` which applies the same algorithm as
/// `git merge-base a b`.
pub(crate) fn merge_base(
    repo: &Repository,
    a: &CommitId,
    b: &CommitId,
) -> Result<Option<CommitId>> {
    let a_oid = parse_oid(a)?;
    let b_oid = parse_oid(b)?;

    match repo.merge_base(a_oid, b_oid) {
        Ok(id) => Ok(Some(gix_id_to_commit_id(id.detach()))),
        Err(e) => {
            // NotFound means no common ancestor — return None rather than Err.
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("NotFound") {
                Ok(None)
            } else {
                Err(anyhow::anyhow!("merge_base failed: {e}"))
            }
        }
    }
}

/// Returns `true` if `candidate` is a direct or transitive ancestor of
/// `descendant`.
///
/// A commit is considered its own ancestor.
///
/// Implementation: the merge base of (candidate, descendant) equals
/// candidate if and only if candidate is an ancestor.
pub(crate) fn is_ancestor(
    repo: &Repository,
    candidate: &CommitId,
    descendant: &CommitId,
) -> Result<bool> {
    if candidate == descendant {
        return Ok(true);
    }

    match merge_base(repo, candidate, descendant)? {
        Some(base) => Ok(base == *candidate),
        None => Ok(false),
    }
}
