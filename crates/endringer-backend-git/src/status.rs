//! Working-tree status: `is_dirty` and full `WorktreeStatus`.
//!
//! ## Dirty-check heuristic
//!
//! For each index entry the check proceeds in two stages:
//!
//! 1. **Fast path** — compare on-disk mtime (seconds) and file size against
//!    the index stat cache.  If both match, assume the file is clean (this is
//!    what `git update-index --refresh` does).
//! 2. **Content-hash fallback** — when mtime and size are identical, compute
//!    the SHA-1 of the file as a git blob and compare it with the OID stored
//!    in the index.  This catches modifications that happen within the same
//!    clock second without changing the file size.
//!
//! ## Gitignore
//!
//! Untracked files are filtered through the repository's active ignore rules
//! (`.gitignore`, `$GIT_DIR/info/exclude`, global excludes) via gix's
//! exclude-stack.  Files that match an ignore rule do not appear in
//! `WorktreeStatus::untracked`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use gix::Repository;

use endringer_backend_core::types::{ChangeKind, StatusEntry, WorktreeStatus};

use crate::object::collect_blob_oids;

// ── Public API ───────────────────────────────────────────────────────────── //

/// Quick dirty check. Returns `true` if the working tree has any uncommitted
/// changes (staged or unstaged). Bare repos always return `false`.
pub(crate) fn is_dirty(repo: &Repository) -> Result<bool> {
    if repo.workdir().is_none() {
        return Ok(false);
    }
    let status = worktree_status(repo)?;
    Ok(!status.staged.is_empty() || !status.unstaged.is_empty())
}

/// Full working-tree status (staged, unstaged, untracked).
pub(crate) fn worktree_status(repo: &Repository) -> Result<WorktreeStatus> {
    if repo.workdir().is_none() {
        return Ok(WorktreeStatus::default());
    }

    let index = repo.open_index().context("open git index")?;
    let workdir = repo.workdir().expect("checked above");

    // ── Build HEAD blob map ─────────────────────────────────────────────── //
    let head_blobs: HashMap<Vec<u8>, gix::ObjectId> =
        match repo.head().ok().and_then(|mut h| h.peel_to_commit().ok()) {
            Some(commit) => {
                let tree = commit.tree().context("HEAD tree")?;
                collect_blob_oids(repo, tree.id)?
            }
            None => HashMap::new(),
        };

    // ── Build index blob map ────────────────────────────────────────────── //
    let mut index_blobs: HashMap<Vec<u8>, gix::ObjectId> = HashMap::new();
    for entry in index.entries() {
        index_blobs.insert(entry.path(&index).to_vec(), entry.id);
    }
    let index_path_set: HashSet<Vec<u8>> = index_blobs.keys().cloned().collect();

    // ── Staged changes (index vs HEAD) ──────────────────────────────────── //
    let mut staged = Vec::new();
    for (path, &index_oid) in &index_blobs {
        match head_blobs.get(path.as_slice()) {
            None => staged.push(make_entry(path, ChangeKind::Added)),
            Some(&head_oid) if head_oid != index_oid => {
                staged.push(make_entry(path, ChangeKind::Modified))
            }
            _ => {}
        }
    }
    for path in head_blobs.keys() {
        if !index_blobs.contains_key(path.as_slice()) {
            staged.push(make_entry(path, ChangeKind::Deleted));
        }
    }
    staged.sort_by(|a, b| a.path.cmp(&b.path));

    // ── Unstaged changes (working tree vs index) ────────────────────────── //
    let mut unstaged = Vec::new();
    for entry in index.entries() {
        let path_bytes = entry.path(&index);
        let abs = workdir.join(Path::new(
            path_bytes.to_os_str_lossy().as_ref() as &std::ffi::OsStr,
        ));

        match std::fs::metadata(&abs) {
            Err(_) => unstaged.push(make_entry(path_bytes, ChangeKind::Deleted)),
            Ok(meta) => {
                let cached_secs = entry.stat.mtime.secs;
                let cached_size = entry.stat.size;
                let disk_secs = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as u32);
                let disk_size = meta.len() as u32;

                let mtime_ok = disk_secs == Some(cached_secs);
                let size_ok = disk_size == cached_size;

                if !mtime_ok || !size_ok {
                    // Fast path: obvious mismatch.
                    unstaged.push(make_entry(path_bytes, ChangeKind::Modified));
                } else {
                    // Content-hash fallback: mtime and size match — verify
                    // blob SHA-1 to catch same-second, same-size edits.
                    let disk_oid = blob_sha1_of_file(&abs);
                    if disk_oid.map_or(true, |oid| oid != entry.id) {
                        unstaged.push(make_entry(path_bytes, ChangeKind::Modified));
                    }
                }
            }
        }
    }
    unstaged.sort_by(|a, b| a.path.cmp(&b.path));

    // ── Untracked files (with gitignore) ────────────────────────────────── //
    let mut untracked = Vec::new();
    let mut exclude_stack = repo
        .excludes(
            &*index,
            None,
            gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )
        .ok(); // If building the exclude stack fails, we fall back to no filtering.

    collect_untracked(
        workdir,
        workdir,
        &index_path_set,
        &mut exclude_stack,
        &mut untracked,
    )?;
    untracked.sort();

    Ok(WorktreeStatus { staged, unstaged, untracked })
}

// ── Helpers ──────────────────────────────────────────────────────────────── //

fn make_entry(path_bytes: &[u8], kind: ChangeKind) -> StatusEntry {
    let s = String::from_utf8_lossy(path_bytes);
    StatusEntry {
        path: PathBuf::from(s.replace('/', std::path::MAIN_SEPARATOR_STR).as_str()),
        kind,
    }
}

/// Computes the git blob SHA-1 of a file on disk:
/// `sha1("blob " + size + "\0" + content)`.
///
/// Returns `None` on I/O error.
fn blob_sha1_of_file(path: &Path) -> Option<gix::ObjectId> {
    let content = std::fs::read(path).ok()?;
    let header = format!("blob {}\0", content.len());
    let mut hasher = gix::hash::hasher(gix::hash::Kind::Sha1);
    hasher.update(header.as_bytes());
    hasher.update(&content);
    hasher.try_finalize().ok()
}

fn collect_untracked(
    workdir: &Path,
    dir: &Path,
    index_paths: &HashSet<Vec<u8>>,
    exclude_stack: &mut Option<gix::AttributeStack<'_>>,
    result: &mut Vec<PathBuf>,
) -> Result<()> {
    for e in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let e = e?;
        let abs = e.path();
        let rel = abs.strip_prefix(workdir).unwrap_or(&abs);

        // Skip .git directory.
        if rel.components().next().map_or(false, |c| c.as_os_str() == ".git") {
            continue;
        }

        let ft = e.file_type()?;
        if ft.is_dir() {
            collect_untracked(workdir, &abs, index_paths, exclude_stack, result)?;
        } else if ft.is_file() {
            let rel_key = rel.to_str().unwrap_or("").replace('\\', "/").into_bytes();
            if !index_paths.contains(&rel_key) {
                // Check gitignore rules if the exclude stack is available.
                let ignored = exclude_stack
                    .as_mut()
                    .and_then(|stack| stack.at_path(rel, None).ok())
                    .map_or(false, |platform| platform.is_excluded());
                if !ignored {
                    result.push(rel.to_path_buf());
                }
            }
        }
    }
    Ok(())
}
