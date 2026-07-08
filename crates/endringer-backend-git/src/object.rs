//! Object-database helpers: file content retrieval and tree traversal.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use endringer_backend_core::types::CommitId;
use gix::Repository;

// ── Public API ───────────────────────────────────────────────────────────── //

/// Returns the raw bytes of `path` (relative to the repo root, forward
/// slashes) as it exists in the tree of `commit_id`.
pub(crate) fn file_at_commit(
    repo: &Repository,
    path: &Path,
    commit_id: &CommitId,
) -> Result<Vec<u8>> {
    let oid = gix::ObjectId::from_hex(commit_id.to_string().as_bytes())
        .map_err(|_| anyhow::anyhow!("invalid commit id '{}'", commit_id))?;

    let commit = repo
        .find_object(oid)
        .with_context(|| format!("commit '{}' not found", commit_id.short()))?
        .try_into_commit()
        .map_err(|_| anyhow::anyhow!("object '{}' is not a commit", commit_id.short()))?;

    let tree = commit.tree().context("failed to read commit tree")?;
    let root_tree_id = tree.id;

    // Normalise separators and split into components.
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("non-UTF-8 path: {}", path.display()))?;
    let normalised = path_str.replace('\\', "/");
    let components: Vec<&[u8]> = normalised.split('/').map(str::as_bytes).collect();

    let blob_oid =
        find_in_tree(repo, root_tree_id, &components).with_context(|| {
            format!("'{}' not found in commit '{}'", path.display(), commit_id.short())
        })?;

    let blob = repo
        .find_object(blob_oid)
        .context("blob object missing")?;
    Ok(blob.data.to_vec())
}

/// Builds a flat map of `b"src/foo.rs"` → `ObjectId` for every blob in
/// `tree_id`'s subtree.  Used by `status.rs` to detect staged changes.
pub(crate) fn collect_blob_oids(
    repo: &Repository,
    tree_id: gix::ObjectId,
) -> Result<HashMap<Vec<u8>, gix::ObjectId>> {
    let mut map = HashMap::new();
    collect_blobs_recursive(repo, tree_id, &[], &mut map)?;
    Ok(map)
}

// ── Internals ────────────────────────────────────────────────────────────── //

fn collect_blobs_recursive(
    repo: &Repository,
    tree_id: gix::ObjectId,
    prefix: &[u8],
    map: &mut HashMap<Vec<u8>, gix::ObjectId>,
) -> Result<()> {
    let tree_obj = repo
        .find_object(tree_id)
        .context("failed to find tree object")?;
    let tree = tree_obj
        .try_into_tree()
        .map_err(|_| anyhow::anyhow!("object is not a tree"))?;

    for entry_result in tree.iter() {
        let te = entry_result.context("tree entry decode error")?;
        let filename = te.filename();

        let full_path: Vec<u8> = if prefix.is_empty() {
            filename.to_vec()
        } else {
            let mut p = prefix.to_vec();
            p.push(b'/');
            p.extend_from_slice(filename);
            p
        };

        if te.mode().is_tree() {
            collect_blobs_recursive(repo, te.object_id(), &full_path, map)?;
        } else {
            // Blob (regular file, executable, symlink).
            map.insert(full_path, te.object_id());
        }
    }
    Ok(())
}

/// Walks `tree_id` following `path_components` and returns the final entry's
/// ObjectId.  Works iteratively to avoid borrow-checker issues with nested
/// `Tree<'_>` lifetimes.
fn find_in_tree(
    repo: &Repository,
    root_tree_id: gix::ObjectId,
    path_components: &[&[u8]],
) -> Result<gix::ObjectId> {
    let mut current_id = root_tree_id;

    for (i, component) in path_components.iter().enumerate() {
        if component.is_empty() {
            continue; // skip empty segments (leading or trailing slash)
        }
        let tree_obj = repo
            .find_object(current_id)
            .context("tree object missing during path walk")?;
        let tree = tree_obj
            .try_into_tree()
            .map_err(|_| anyhow::anyhow!("expected a tree at path component {i}"))?;

        let mut found = None;
        for entry_result in tree.iter() {
            let te = entry_result?;
            if te.filename() == *component {
                found = Some(te.object_id());
                break;
            }
        }

        current_id = found.ok_or_else(|| {
            anyhow::anyhow!(
                "path component '{}' not found",
                String::from_utf8_lossy(component)
            )
        })?;
    }
    Ok(current_id)
}
