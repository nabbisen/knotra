//! The [`VcsBackend`] trait that all VCS backends implement.
//!
//! This trait is `pub` so that downstream crates can implement custom
//! backends and inject them via [`endringer::repository::Repository::with_backend`].
//! The trait signature may change before v1.0.

use std::time::SystemTime;

use anyhow::Result;

use crate::types::{BlameEntry, BranchInfo, CommitId, CommitInfo, DiffSummary, SortOrder, StashEntry, StatusDigest, SubmoduleInfo, TagInfo, WorktreeInfo, WorktreeStatus};

/// Common interface implemented by every VCS backend.
///
/// All methods take `&self` and must be safe to call from multiple threads
/// (`Send + Sync` bound).
pub trait VcsBackend: Send + Sync {
    // ── Status ─────────────────────────────────────────────────────────── //
    fn status_digest(&self) -> Result<StatusDigest>;

    // ── Branches ───────────────────────────────────────────────────────── //
    fn local_branches(&self) -> Result<Vec<BranchInfo>>;
    fn remote_branches(&self) -> Result<Vec<BranchInfo>>;

    // ── Commits ────────────────────────────────────────────────────────── //
    fn list_commits(&self) -> Result<Vec<CommitInfo>>;
    fn list_commits_sorted(&self, order: SortOrder) -> Result<Vec<CommitInfo>>;
    fn log_since(&self, since: SystemTime, until: SystemTime) -> Result<Vec<CommitInfo>>;
    fn find_commit(&self, id: &CommitId) -> Result<CommitInfo>;

    // ── Tags ───────────────────────────────────────────────────────────── //
    fn list_tags(&self) -> Result<Vec<TagInfo>>;
    fn list_tags_sorted(&self, order: SortOrder) -> Result<Vec<TagInfo>>;
    fn create_tag(&self, name: &str) -> Result<()>;
    fn create_annotated_tag(&self, name: &str, message: &str) -> Result<()>;
    fn delete_tag(&self, name: &str) -> Result<()>;

    // ── Diff ───────────────────────────────────────────────────────────── //
    fn diff(&self, from: &CommitId, to: &CommitId) -> Result<DiffSummary>;

    // ── Remotes ────────────────────────────────────────────────────────── //
    fn remote_url(&self, name: &str) -> Option<String>;

    // ── Working tree ───────────────────────────────────────────────────── //

    /// Returns `true` if the working tree has any uncommitted changes
    /// (staged or unstaged).
    ///
    /// Bare repositories always return `false` (no working tree).
    fn is_dirty(&self) -> Result<bool>;

    // ── Commit graph ───────────────────────────────────────────────────── //

    /// Returns the best common ancestor of commits `a` and `b`, or `None`
    /// if there is no shared history.
    fn merge_base(&self, a: &CommitId, b: &CommitId) -> Result<Option<CommitId>>;

    /// Returns `true` if `candidate` is a direct or transitive ancestor of
    /// `descendant` (a commit is considered its own ancestor).
    fn is_ancestor(&self, candidate: &CommitId, descendant: &CommitId) -> Result<bool>;

    // ── Blame ──────────────────────────────────────────────────────────── //

    /// Returns per-line commit attribution for `path` at HEAD.
    ///
    /// `path` must be relative to the repository root.
    /// Returns [`BlameEntry`] items in line order (ascending `start_line`).
    fn blame(&self, path: &std::path::Path) -> Result<Vec<BlameEntry>>;

    // ── Working tree status ────────────────────────────────────────────── //

    /// Returns per-file working-tree status (staged changes, unstaged changes,
    /// and untracked files).
    fn worktree_status(&self) -> Result<WorktreeStatus>;

    // ── File content ───────────────────────────────────────────────────── //

    /// Returns the raw content of `path` (relative to the repository root)
    /// as it exists in the tree of `commit_id`.
    ///
    /// Returns an error if the path does not exist in that commit's tree.
    fn file_at_commit(&self, path: &std::path::Path, commit_id: &CommitId) -> Result<Vec<u8>>;

    // ── Submodules ─────────────────────────────────────────────────────── //

    /// Returns metadata for all submodules declared in `.gitmodules`.
    /// Returns an empty `Vec` when no submodules are configured.
    fn submodules(&self) -> Result<Vec<SubmoduleInfo>>;

    // ── Stash ──────────────────────────────────────────────────────────── //

    /// Returns all stash entries (newest first), or an empty `Vec` if the
    /// stash is empty.
    fn stash_entries(&self) -> Result<Vec<StashEntry>>;

    // ── Linked worktrees ───────────────────────────────────────────────── //

    /// Returns all linked worktrees. The main worktree is **not** included.
    /// Returns an empty `Vec` for repositories with no linked worktrees.
    fn worktrees(&self) -> Result<Vec<WorktreeInfo>>;
}
