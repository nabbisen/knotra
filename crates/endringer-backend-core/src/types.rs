use std::time::SystemTime;

/// Opaque commit identifier, stored as raw bytes.
///
/// Supports both SHA-1 (20 bytes / 40 hex chars, used by Git) and SHA-256
/// (32 bytes / 64 hex chars, used by Jujutsu). No VCS library types are
/// exposed.
///
/// # Ordering
///
/// `CommitId` implements `Ord` via byte-level lexicographic comparison.
/// IDs produced by different hash algorithms (SHA-1 vs SHA-256) compare
/// consistently but not meaningfully across algorithms.
///
/// # Example
///
/// ```
/// # use endringer_backend_core::types::CommitId;
/// let id = CommitId::from_hex("0000000000000000000000000000000000000000").unwrap();
/// assert_eq!(id.short().len(), 7);
/// println!("{id}");   // full hex string
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommitId(Vec<u8>);

impl CommitId {
    /// Constructs a `CommitId` from raw bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        CommitId(bytes)
    }

    /// Constructs a `CommitId` by decoding a lowercase hex string.
    ///
    /// Accepts 40-character (SHA-1) or 64-character (SHA-256) hex strings.
    ///
    /// ```
    /// # use endringer_backend_core::types::CommitId;
    /// assert!(CommitId::from_hex("0000000000000000000000000000000000000000").is_ok());
    /// assert!(CommitId::from_hex("not-a-hash").is_err());
    /// assert!(CommitId::from_hex("abc123").is_err());  // too short
    /// ```
    pub fn from_hex(hex: &str) -> Result<Self, CommitIdFromHexError> {
        let len = hex.len();
        if len != 40 && len != 64 {
            return Err(CommitIdFromHexError(hex.to_owned()));
        }
        let mut bytes = Vec::with_capacity(len / 2);
        for chunk in hex.as_bytes().chunks(2) {
            let hi = hex_nibble(chunk[0]).ok_or_else(|| CommitIdFromHexError(hex.to_owned()))?;
            let lo = hex_nibble(chunk[1]).ok_or_else(|| CommitIdFromHexError(hex.to_owned()))?;
            bytes.push((hi << 4) | lo);
        }
        Ok(CommitId(bytes))
    }

    /// Returns the raw bytes of this commit identifier.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the first 7 hex characters — the conventional "short" form.
    pub fn short(&self) -> String {
        let mut out = String::with_capacity(7);
        for &b in self.0.iter().take(4) {
            out.push(nibble_char(b >> 4));
            out.push(nibble_char(b & 0xf));
        }
        out.truncate(7);
        out
    }
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn nibble_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'a' + n - 10) as char,
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in &self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

/// Error returned when [`CommitId::from_hex`] receives an invalid hex string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitIdFromHexError(String);

impl std::fmt::Display for CommitIdFromHexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid commit id {:?}: expected 40 (SHA-1) or 64 (SHA-256) hex chars",
            self.0
        )
    }
}

impl std::error::Error for CommitIdFromHexError {}

/// Information about a branch (local or remote).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchInfo {
    /// Short branch name, e.g. `main`.
    pub name: String,
    /// Full ref name, e.g. `refs/heads/main` or `refs/remotes/origin/main`.
    pub full_name: String,
    /// Commit ID at the tip of the branch.
    pub last_commit_id: CommitId,
    /// First line of the most recent commit message.
    pub last_commit_summary: String,
    /// Author timestamp of the most recent commit.
    pub last_commit_timestamp: SystemTime,
}

/// Lightweight summary of the repository's current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusDigest {
    /// Directory name of the repository's working tree.
    pub repo_name: String,
    /// Name of the currently checked-out branch, or `"(detached)"`.
    pub current_branch: String,
    /// Commit ID of the current HEAD.
    pub last_commit_id: CommitId,
    /// First line of HEAD's commit message.
    pub last_commit_summary: String,
    /// Author timestamp of HEAD.
    pub last_commit_timestamp: SystemTime,
}

/// Information about a single commit.
///
/// **Breaking change (v0.14)**: a `parents` field was added. Code that
/// constructs `CommitInfo` directly (outside this library) must be updated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitInfo {
    /// Full commit identifier.
    pub commit_id: CommitId,
    /// Direct parent commit IDs (empty for the initial commit).
    pub parents: Vec<CommitId>,
    /// Author name.
    pub author: String,
    /// Committer name. Differs from `author` after cherry-pick, rebase, or amend.
    pub committer: String,
    /// First line of the commit message (subject line).
    pub summary: String,
    /// Author timestamp.
    pub timestamp: SystemTime,
    /// Committer timestamp.
    pub committer_timestamp: SystemTime,
}

/// Annotation data for an annotated tag.
///
/// Absent (`TagInfo::annotation` is `None`) for lightweight tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAnnotation {
    /// The annotation message (trimmed).
    pub message: String,
    /// Tagger name, if recorded in the tag object.
    pub tagger_name: Option<String>,
    /// Tagger timestamp, if recorded in the tag object.
    pub tagger_timestamp: Option<SystemTime>,
}

/// Information about a tag.
///
/// **Breaking change (v0.18)**: an `annotation` field was added.
/// Code that constructs `TagInfo` directly must add `annotation: None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagInfo {
    /// Short tag name, e.g. `v1.0.0`.
    pub name: String,
    /// Full ref name, e.g. `refs/tags/v1.0.0`.
    pub full_name: String,
    /// Commit ID the tag points to (after peeling any tag objects).
    pub commit_id: CommitId,
    /// First line of the tagged commit's message.
    pub commit_summary: String,
    /// Author timestamp of the tagged commit.
    pub commit_timestamp: SystemTime,
    /// Present for annotated tags; `None` for lightweight tags.
    pub annotation: Option<TagAnnotation>,
}

/// Sort order for commit and tag listings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortOrder {
    /// Newest first (descending timestamp).
    NewestFirst,
    /// Oldest first (ascending timestamp).
    OldestFirst,
    /// Alphabetical by tag name or commit summary (ascending).
    ByName,
}

/// Summary of file-level changes between two commits.
///
/// Paths within each category (`added`, `modified`, `deleted`) are sorted
/// in ascending lexicographic order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DiffSummary {
    /// Paths of files added between `from` and `to`.
    pub added: Vec<std::path::PathBuf>,
    /// Paths of files modified between `from` and `to`.
    pub modified: Vec<std::path::PathBuf>,
    /// Paths of files deleted between `from` and `to`.
    pub deleted: Vec<std::path::PathBuf>,
}

/// Which VCS backend a [`Repository`][crate] is backed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    /// Git (via `gix`).
    Git,
    /// Jujutsu (git store read via `gix`).
    Jj,
}

/// One contiguous span of lines in a file, attributed to a single commit.
///
/// Lines are **1-indexed** and inclusive on both ends.
/// `start_line == end_line` means a single-line entry.
///
/// Returned by [`crate::repository::Repository::blame`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameEntry {
    /// Commit that introduced these lines.
    pub commit_id: CommitId,
    /// First line in the blamed file (1-indexed, inclusive).
    pub start_line: u32,
    /// Last line in the blamed file (1-indexed, inclusive).
    pub end_line: u32,
    /// Original file path in the source commit, present only when the file
    /// was renamed between that commit and the blamed file.
    pub original_path: Option<std::path::PathBuf>,
}

// ── Working tree status ───────────────────────────────────────────────────── //

/// The kind of change a [`StatusEntry`] represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// A new file was added (not present in the reference point).
    Added,
    /// An existing file was modified.
    Modified,
    /// A tracked file was deleted.
    Deleted,
}

/// A single file entry in a [`WorktreeStatus`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusEntry {
    /// Path relative to the repository root, using the platform path separator.
    pub path: std::path::PathBuf,
    /// Nature of the change.
    pub kind: ChangeKind,
}

/// Detailed working-tree status, equivalent to the output of `git status`.
///
/// Returned by [`crate::repository::Repository::worktree_status`].
///
/// ## Untracked files
///
/// `untracked` lists every file present in the working tree that the index
/// does not track. **Gitignore rules are not applied in the current
/// implementation** — ignored files will appear here. A future release will
/// honour `.gitignore`.
#[derive(Clone, Debug, Default)]
pub struct WorktreeStatus {
    /// Files whose staged blob OID differs from the HEAD tree
    /// (includes new files added to the index and staged deletions).
    pub staged: Vec<StatusEntry>,
    /// Files whose on-disk content or metadata differs from the index
    /// (modifications and deletions that have not been staged yet).
    pub unstaged: Vec<StatusEntry>,
    /// Files present in the working tree but not tracked by git.
    pub untracked: Vec<std::path::PathBuf>,
}

// ── Submodule information ─────────────────────────────────────────────────── //

/// Information about a single Git submodule as declared in `.gitmodules`.
///
/// Returned by [`crate::repository::Repository::submodules`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmoduleInfo {
    /// Submodule name as declared in `.gitmodules` (typically the same as `path`).
    pub name: String,
    /// Path of the submodule working tree relative to the repository root.
    pub path: std::path::PathBuf,
    /// Remote URL the submodule tracks, if configured.
    pub url: Option<String>,
}

// ── Stash entries ─────────────────────────────────────────────────────────── //

/// A single entry from the stash, corresponding to `stash@{N}`.
///
/// Returned by [`crate::repository::Repository::stash_entries`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashEntry {
    /// Zero-based index (`stash@{0}` = 0, `stash@{1}` = 1, …).
    /// Entries are returned newest-first.
    pub index: usize,
    /// OID of the stash commit.
    pub commit_id: CommitId,
    /// Stash message (e.g. `"WIP on main: abc1234 initial commit"`).
    pub message: String,
}

// ── Linked worktrees ──────────────────────────────────────────────────────── //

/// Information about a linked git worktree.
///
/// Returned by [`crate::repository::Repository::worktrees`]. The main
/// worktree is **not** included; only linked worktrees created via
/// `git worktree add` appear here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// The worktree's identifier (the directory name under `.git/worktrees/`).
    pub id: String,
    /// Absolute path to the worktree's working directory.
    pub path: std::path::PathBuf,
    /// Currently checked-out branch (short name), or `"(detached)"` when
    /// the HEAD is in a detached state.
    pub current_branch: String,
    /// Whether the worktree is locked (`git worktree lock`).
    pub is_locked: bool,
}
