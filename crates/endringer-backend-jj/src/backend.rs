//! [`JjBackend`] — delegates to [`GitBackend`] opened on jj's git store.
//!
//! Jujutsu stores commit history in a git object database. This backend opens
//! that store directly with gix; no `jj` binary is required.
//!
//! # Repository layout
//!
//! | Mode | Detection | Git store path |
//! |------|-----------|----------------|
//! | Co-located | `.git/` **and** `.jj/` present | project root |
//! | Native jj  | only `.jj/` present | `.jj/repo/store/git/` |
//!
//! # Annotated tags
//!
//! Jujutsu only supports lightweight tags. `create_annotated_tag` creates a
//! lightweight tag and ignores the message, matching jj's own behaviour.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Result, bail};
use endringer_backend_core::backend::VcsBackend;
use endringer_backend_core::types::{BlameEntry, BranchInfo, CommitId, CommitInfo, DiffSummary, SortOrder, StashEntry, StatusDigest, SubmoduleInfo, TagInfo, WorktreeInfo, WorktreeStatus};
use endringer_backend_git::GitBackend;

/// Jujutsu backend backed by the repository's underlying git object store.
pub struct JjBackend {
    git: GitBackend,
    /// Project root (the directory containing `.jj/`).
    root: PathBuf,
}

impl JjBackend {
    /// Opens a Jujutsu repository at `path`.
    ///
    /// Verifies that `path` contains `.jj/`, locates the git store, and opens
    /// it with gix. The `jj` binary is not consulted.
    pub fn open(path: &Path) -> Result<Self> {
        let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        let jj_dir = root.join(".jj");
        if !jj_dir.is_dir() {
            bail!("not a jj repository: no .jj directory at {}", root.display());
        }

        let git_store_path = if root.join(".git").exists() {
            // Co-located: the project root is also the git repository.
            root.clone()
        } else {
            // Native jj: git store lives at .jj/repo/store/git/ (bare repo).
            let native = jj_dir.join("repo").join("store").join("git");
            if !native.is_dir() {
                bail!(
                    "jj repository at {} has no git backend \
                     (looked for {} and {})",
                    root.display(),
                    root.join(".git").display(),
                    native.display()
                );
            }
            native
        };

        let git = GitBackend::open(&git_store_path)?;
        Ok(JjBackend { git, root })
    }
}

impl VcsBackend for JjBackend {
    fn status_digest(&self) -> Result<StatusDigest> {
        let mut digest = self.git.status_digest()?;
        // For native jj repos the git store is at .jj/repo/store/git, whose
        // directory name is "git", not the project name. Override here.
        digest.repo_name = self
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_owned();
        Ok(digest)
    }

    fn local_branches(&self) -> Result<Vec<BranchInfo>> { self.git.local_branches() }
    fn remote_branches(&self) -> Result<Vec<BranchInfo>> { self.git.remote_branches() }
    fn list_commits(&self) -> Result<Vec<CommitInfo>> { self.git.list_commits() }
    fn list_commits_sorted(&self, order: SortOrder) -> Result<Vec<CommitInfo>> { self.git.list_commits_sorted(order) }
    fn log_since(&self, since: SystemTime, until: SystemTime) -> Result<Vec<CommitInfo>> { self.git.log_since(since, until) }
    fn find_commit(&self, id: &CommitId) -> Result<CommitInfo> { self.git.find_commit(id) }
    fn list_tags(&self) -> Result<Vec<TagInfo>> { self.git.list_tags() }
    fn list_tags_sorted(&self, order: SortOrder) -> Result<Vec<TagInfo>> { self.git.list_tags_sorted(order) }
    fn create_tag(&self, name: &str) -> Result<()> { self.git.create_tag(name) }

    /// Always returns an error: Jujutsu does not support annotated tags.
    ///
    /// Use [`create_tag`][Self::create_tag] for a lightweight tag instead.
    fn create_annotated_tag(&self, name: &str, _message: &str) -> Result<()> {
        anyhow::bail!(
            "jj does not support annotated tags;              use create_tag(\"{name}\") for a lightweight tag instead"
        )
    }

    fn delete_tag(&self, name: &str) -> Result<()> { self.git.delete_tag(name) }
    fn diff(&self, from: &CommitId, to: &CommitId) -> Result<DiffSummary> { self.git.diff(from, to) }
    fn remote_url(&self, name: &str) -> Option<String> { self.git.remote_url(name) }
    fn is_dirty(&self) -> Result<bool> { self.git.is_dirty() }
    fn merge_base(&self, a: &CommitId, b: &CommitId) -> Result<Option<CommitId>> { self.git.merge_base(a, b) }
    fn is_ancestor(&self, candidate: &CommitId, descendant: &CommitId) -> Result<bool> { self.git.is_ancestor(candidate, descendant) }
    fn blame(&self, path: &std::path::Path) -> Result<Vec<BlameEntry>> { self.git.blame(path) }
    fn worktree_status(&self) -> Result<WorktreeStatus> { self.git.worktree_status() }
    fn file_at_commit(&self, path: &std::path::Path, commit_id: &CommitId) -> Result<Vec<u8>> { self.git.file_at_commit(path, commit_id) }
    fn submodules(&self) -> Result<Vec<SubmoduleInfo>> { self.git.submodules() }
    fn stash_entries(&self) -> Result<Vec<StashEntry>> { self.git.stash_entries() }
    fn worktrees(&self) -> Result<Vec<WorktreeInfo>> { self.git.worktrees() }
}
