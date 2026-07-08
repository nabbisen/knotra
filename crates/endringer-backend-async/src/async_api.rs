//! [`AsyncRepository`] — async wrapper around [`endringer::repository::Repository`].

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Result;
use crate::repository::{Repository, jj_repository, repository};
use endringer_backend_core::types::{BlameEntry, BranchInfo, CommitId, CommitInfo, DiffSummary, SortOrder, StashEntry, StatusDigest, SubmoduleInfo, TagInfo, WorktreeInfo, WorktreeStatus};

/// Async wrapper around [`Repository`].
///
/// Cloneable. Each method clones the inner `Arc<Repository>` and dispatches
/// to `tokio::task::spawn_blocking` so that blocking VCS I/O does not stall
/// the async executor.
#[derive(Clone)]
pub struct AsyncRepository {
    inner: Arc<Repository>,
}

impl AsyncRepository {
    /// Opens a Git repository at `path`.
    pub async fn open(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let inner = tokio::task::spawn_blocking(move || repository(&path)).await??;
        Ok(AsyncRepository { inner: Arc::new(inner) })
    }

    /// Opens a Jujutsu repository at `path`.
    pub async fn open_jj(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let inner = tokio::task::spawn_blocking(move || jj_repository(&path)).await??;
        Ok(AsyncRepository { inner: Arc::new(inner) })
    }

    // ── Status ─────────────────────────────────────────────────────────── //

    pub async fn status_digest(&self) -> Result<StatusDigest> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.status_digest()).await?
    }

    // ── Branches ───────────────────────────────────────────────────────── //

    pub async fn local_branches(&self) -> Result<Vec<BranchInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.local_branches()).await?
    }

    pub async fn remote_branches(&self) -> Result<Vec<BranchInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.remote_branches()).await?
    }

    // ── Commits ────────────────────────────────────────────────────────── //

    pub async fn list_commits(&self) -> Result<Vec<CommitInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.list_commits()).await?
    }

    pub async fn list_commits_sorted(&self, order: SortOrder) -> Result<Vec<CommitInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.list_commits_sorted(order)).await?
    }

    pub async fn log_since(&self, since: SystemTime, until: SystemTime) -> Result<Vec<CommitInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.log_since(since, until)).await?
    }

    pub async fn find_commit(&self, id: CommitId) -> Result<CommitInfo> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.find_commit(&id)).await?
    }

    // ── Tags ───────────────────────────────────────────────────────────── //

    pub async fn list_tags(&self) -> Result<Vec<TagInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.list_tags()).await?
    }

    pub async fn list_tags_sorted(&self, order: SortOrder) -> Result<Vec<TagInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.list_tags_sorted(order)).await?
    }

    pub async fn create_tag(&self, name: String) -> Result<()> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.create_tag(&name)).await?
    }

    pub async fn create_annotated_tag(&self, name: String, message: String) -> Result<()> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.create_annotated_tag(&name, &message)).await?
    }

    pub async fn delete_tag(&self, name: String) -> Result<()> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.delete_tag(&name)).await?
    }

    // ── Diff ───────────────────────────────────────────────────────────── //

    pub async fn diff(&self, from: CommitId, to: CommitId) -> Result<DiffSummary> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.diff(&from, &to)).await?
    }

    // ── Remotes ────────────────────────────────────────────────────────── //

    pub async fn remote_url(&self, name: String) -> Option<String> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.remote_url(&name))
            .await
            .ok()
            .flatten()
    }

    // ── Working tree ───────────────────────────────────────────────────── //

    pub async fn is_dirty(&self) -> Result<bool> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.is_dirty()).await?
    }

    // ── Commit graph ───────────────────────────────────────────────────── //

    pub async fn merge_base(&self, a: CommitId, b: CommitId) -> Result<Option<CommitId>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.merge_base(&a, &b)).await?
    }

    pub async fn is_ancestor(&self, candidate: CommitId, descendant: CommitId) -> Result<bool> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.is_ancestor(&candidate, &descendant)).await?
    }

    // ── Blame ──────────────────────────────────────────────────────────── //

    pub async fn blame(&self, path: std::path::PathBuf) -> Result<Vec<BlameEntry>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.blame(&path)).await?
    }

    pub async fn worktree_status(&self) -> Result<WorktreeStatus> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.worktree_status()).await?
    }

    pub async fn file_at_commit(
        &self,
        path: std::path::PathBuf,
        commit_id: CommitId,
    ) -> Result<Vec<u8>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.file_at_commit(&path, &commit_id)).await?
    }

    pub async fn submodules(&self) -> Result<Vec<SubmoduleInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.submodules()).await?
    }

    pub async fn stash_entries(&self) -> Result<Vec<StashEntry>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.stash_entries()).await?
    }

    pub async fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let r = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || r.worktrees()).await?
    }
}
