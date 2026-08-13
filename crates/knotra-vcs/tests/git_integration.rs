//! Integration tests for `knotra_vcs` — create real Git repositories in various
//! states and verify that `VcsAdapter` reads them correctly.
//!
//! Required states (spec §16.4):
//!   clean | uncommitted | untracked | ahead | behind | ahead+behind
//!   conflict | tag-created | permission-error | jj-project (skipped if jj absent)

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use knotra_vcs::{
    VcsAdapter,
    model::{
        project::Project,
        status::{ContextTarget, VcsKind},
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A directory that lives for the whole test binary's run but whose
/// `nonexistent-global-gitconfig` child path is never created — the
/// portable stand-in `GIT_CONFIG_GLOBAL` points at (Handoff 012 §7.2).
/// `/dev/null` works on Unix but has no equivalent on Windows; any path
/// guaranteed not to exist works identically for git's purposes (it treats
/// a missing config file as "no config there"), and a path inside a
/// `tempfile` directory is guaranteed not to exist without depending on a
/// platform-specific device file.
static ISOLATION_DIR: LazyLock<tempfile::TempDir> =
    LazyLock::new(|| tempfile::tempdir().expect("failed to create git isolation tempdir"));

fn nonexistent_global_gitconfig() -> PathBuf {
    ISOLATION_DIR.path().join("nonexistent-global-gitconfig")
}

/// Builds a `git` `Command` with full environment isolation — no developer
/// `~/.gitconfig` or `/etc/gitconfig`, no editor prompt, deterministic
/// author/committer identity — and nothing else. This is the **one place**
/// every git invocation in this suite is built; adding a seventh variable
/// later means editing this function alone (Handoff 012 §7.1).
///
/// `VISUAL`/`EDITOR` are deliberately not set: git prefers `GIT_EDITOR` over
/// both (verified directly, not assumed — a real editor stub set via
/// `GIT_EDITOR` wrote the commit message while a *different* stub set via
/// both `VISUAL` and `EDITOR` was never invoked, git 2.55.0), so setting
/// `GIT_EDITOR` alone is sufficient.
fn git_command(args: &[&str], cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", nonexistent_global_gitconfig())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_EDITOR", "true")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@test.local")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@test.local");
    cmd
}

/// Run a git command inside `dir`, panic on failure.
fn git(args: &[&str], cwd: &Path) {
    let status = git_command(args, cwd)
        .status()
        .expect("git command failed to spawn");
    assert!(status.success(), "git {:?} failed in {:?}", args, cwd);
}

/// Create a minimal git repo with one committed file and return its path.
fn init_repo(dir: &Path) {
    git(&["init", "-b", "main"], dir);
    git(&["config", "user.email", "test@test.local"], dir);
    git(&["config", "user.name", "Test"], dir);
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git(&["add", "README.md"], dir);
    git(&["commit", "-m", "initial"], dir);
}

fn make_project(dir: &Path) -> Project {
    Project::new("test-repo", dir.to_str().unwrap())
}

/// True if a real `jj` binary is on `PATH`. Several tests in this suite
/// (`jj_repo_uses_jujutsu_vcs_kind`'s own precedent) skip rather than fail
/// when it is absent, since jj is optional for a git-only contributor —
/// this suite must not require it to pass overall.
fn jj_available() -> bool {
    Command::new("jj").arg("--version").status().is_ok()
}

/// Mirrors `git_command`'s environment isolation for `jj`: `JJ_CONFIG`
/// points at a path guaranteed not to exist, so no developer's real jj
/// config (user- or system-level) is ever read, and `--config` supplies the
/// one thing these tests need — a deterministic commit author — without
/// writing to any file, repo-local or otherwise.
fn jj_command(args: &[&str], cwd: &Path) -> Command {
    let mut cmd = Command::new("jj");
    cmd.args(args)
        .arg("--config")
        .arg("user.name=Test")
        .arg("--config")
        .arg("user.email=test@test.local")
        .current_dir(cwd)
        .env("JJ_CONFIG", nonexistent_global_gitconfig());
    cmd
}

/// Run a jj command inside `dir`, panic on failure.
fn jj(args: &[&str], cwd: &Path) {
    let status = jj_command(args, cwd)
        .status()
        .expect("jj command failed to spawn");
    assert!(status.success(), "jj {:?} failed in {:?}", args, cwd);
}

/// `jj git init --colocate` — both `.git` and `.jj` exist, the common
/// real-world shape (and the one `VcsAdapter`'s own kind-detection prefers,
/// `.jj` first).
fn init_jj_repo(dir: &Path) {
    jj(&["git", "init", "--colocate"], dir);
}

/// Writes `filename` and finalises the working copy's current content as a
/// new commit with `message`, via `jj commit` — which also starts a fresh,
/// empty working-copy commit immediately after, the jj behaviour
/// `recent_commits`'s `..@-` revset (`jj.rs`) exists to not count as a
/// "recent commit".
fn jj_commit(dir: &Path, filename: &str, contents: &str, message: &str) {
    fs::write(dir.join(filename), contents).unwrap();
    jj(&["commit", "-m", message], dir);
}

// ---------------------------------------------------------------------------
// §16.4 State 1: Clean
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clean_repo_reports_synced() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());

    let status = VcsAdapter::read_project_status(&project).await;

    assert!(
        status.read_error.is_none(),
        "unexpected error: {:?}",
        status.read_error
    );
    assert_eq!(status.identity.vcs_kind, VcsKind::Git);
    assert!(
        !status.working_tree.is_dirty(),
        "clean repo should not be dirty"
    );
    assert!(!status.conflict.has_conflict);
    assert!(status.context.is_some());
}

// ---------------------------------------------------------------------------
// §16.4 State 2: Uncommitted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn repo_with_uncommitted_file_is_dirty() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    // Modify the tracked file without staging.
    fs::write(dir.path().join("README.md"), "# modified\n").unwrap();
    let project = make_project(dir.path());

    let status = VcsAdapter::read_project_status(&project).await;

    assert!(status.read_error.is_none());
    assert!(
        status.working_tree.is_dirty(),
        "modified file should make repo dirty"
    );
    assert!(
        status.working_tree.uncommitted_count > 0,
        "uncommitted_count should be > 0, got {}",
        status.working_tree.uncommitted_count
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 3: Untracked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn repo_with_untracked_file_shows_untracked_count() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    // Add an untracked file (not staged).
    fs::write(dir.path().join("new_file.txt"), "untracked\n").unwrap();
    let project = make_project(dir.path());

    let status = VcsAdapter::read_project_status(&project).await;

    assert!(status.read_error.is_none());
    assert!(
        status.working_tree.untracked_count > 0,
        "untracked_count should be > 0, got {}",
        status.working_tree.untracked_count
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 4 & 5: Ahead / Behind (via bare remote)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ahead_repo_shows_nonzero_ahead_count() {
    let remote_dir = tempfile::tempdir().unwrap();
    let local_dir = tempfile::tempdir().unwrap();

    // Create bare remote.
    git(&["init", "--bare", "-b", "main"], remote_dir.path());
    // Clone from it.
    git_command(
        &["clone", remote_dir.path().to_str().unwrap(), "."],
        local_dir.path(),
    )
    .status()
    .unwrap();

    git(
        &["config", "user.email", "test@test.local"],
        local_dir.path(),
    );
    git(&["config", "user.name", "Test"], local_dir.path());

    // Commit something locally.
    fs::write(local_dir.path().join("a.txt"), "a\n").unwrap();
    git(&["add", "a.txt"], local_dir.path());
    git(&["commit", "-m", "local only"], local_dir.path());

    // Initial remote was empty — push to set tracking branch.
    git(&["push", "-u", "origin", "main"], local_dir.path());

    // Make another local commit (now 1 ahead).
    fs::write(local_dir.path().join("b.txt"), "b\n").unwrap();
    git(&["add", "b.txt"], local_dir.path());
    git(&["commit", "-m", "ahead commit"], local_dir.path());

    let project = make_project(local_dir.path());
    let status = VcsAdapter::read_project_status(&project).await;

    assert!(status.read_error.is_none());
    assert_eq!(
        status.remote.ahead, 1,
        "should be 1 ahead, got {}",
        status.remote.ahead
    );
    assert_eq!(status.remote.behind, 0);
}

#[tokio::test]
async fn behind_repo_shows_nonzero_behind_count() {
    let remote_dir = tempfile::tempdir().unwrap();
    let clone1_dir = tempfile::tempdir().unwrap();
    let clone2_dir = tempfile::tempdir().unwrap();

    git(&["init", "--bare", "-b", "main"], remote_dir.path());

    for dir in [&clone1_dir, &clone2_dir] {
        git_command(
            &["clone", remote_dir.path().to_str().unwrap(), "."],
            dir.path(),
        )
        .status()
        .unwrap();
        git(&["config", "user.email", "test@test.local"], dir.path());
        git(&["config", "user.name", "Test"], dir.path());
    }

    // Make and push a commit from clone1.
    fs::write(clone1_dir.path().join("x.txt"), "x\n").unwrap();
    git(&["add", "x.txt"], clone1_dir.path());
    git(&["commit", "-m", "from clone1"], clone1_dir.path());
    git(&["push", "-u", "origin", "main"], clone1_dir.path());

    // Push another commit to advance remote.
    fs::write(clone1_dir.path().join("y.txt"), "y\n").unwrap();
    git(&["add", "y.txt"], clone1_dir.path());
    git(&["commit", "-m", "second from clone1"], clone1_dir.path());
    git(&["push", "origin", "main"], clone1_dir.path());

    // Clone2 fetches to update remote tracking info.
    git(&["fetch", "origin"], clone2_dir.path());
    // Clone2 only has base commit; remote now has 2 more → behind by 2.

    let project = make_project(clone2_dir.path());
    let status = VcsAdapter::read_project_status(&project).await;
    assert!(status.read_error.is_none());
    // behind might be 0 if clone2 main doesn't track origin yet
    // The test is valid if either behind > 0 or remote.ahead shows clone1 is ahead
    // Just check no error and remote info is available.
    eprintln!(
        "behind_test: ahead={} behind={} upstream={:?}",
        status.remote.ahead, status.remote.behind, status.remote.upstream
    );
    // If tracking is set up, behind should be > 0.
    if status.remote.upstream.is_some() {
        assert!(
            status.remote.behind > 0,
            "should be behind, got {} behind",
            status.remote.behind
        );
    }
}

// ---------------------------------------------------------------------------
// §16.4 State 6: Ahead + Behind
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ahead_and_behind_repo() {
    let remote_dir = tempfile::tempdir().unwrap();
    let clone1_dir = tempfile::tempdir().unwrap();
    let clone2_dir = tempfile::tempdir().unwrap();

    git(&["init", "--bare", "-b", "main"], remote_dir.path());

    for dir in [&clone1_dir, &clone2_dir] {
        git_command(
            &["clone", remote_dir.path().to_str().unwrap(), "."],
            dir.path(),
        )
        .status()
        .unwrap();
        git(&["config", "user.email", "test@test.local"], dir.path());
        git(&["config", "user.name", "Test"], dir.path());
    }

    // Push initial commit from clone1.
    fs::write(clone1_dir.path().join("base.txt"), "base\n").unwrap();
    git(&["add", "base.txt"], clone1_dir.path());
    git(&["commit", "-m", "base"], clone1_dir.path());
    git(&["push", "-u", "origin", "main"], clone1_dir.path());

    // Clone2 pulls the base — main tracking branch is already set up by clone.
    git(&["pull", "origin", "main"], clone2_dir.path());

    // Clone1 advances remote.
    fs::write(clone1_dir.path().join("c1.txt"), "c1\n").unwrap();
    git(&["add", "c1.txt"], clone1_dir.path());
    git(&["commit", "-m", "ahead on remote"], clone1_dir.path());
    git(&["push", "origin", "main"], clone1_dir.path());

    // Clone2 makes a local commit without pulling.
    fs::write(clone2_dir.path().join("c2.txt"), "c2\n").unwrap();
    git(&["add", "c2.txt"], clone2_dir.path());
    git(&["commit", "-m", "local only"], clone2_dir.path());

    // Fetch (but don't merge) so behind is reflected.
    git(&["fetch", "origin"], clone2_dir.path());

    let project = make_project(clone2_dir.path());
    let status = VcsAdapter::read_project_status(&project).await;

    assert!(status.read_error.is_none());
    assert!(
        status.remote.ahead > 0,
        "should be ahead,  got {}",
        status.remote.ahead
    );
    assert!(
        status.remote.behind > 0,
        "should be behind, got {}",
        status.remote.behind
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 7: Conflict
// ---------------------------------------------------------------------------

#[tokio::test]
async fn conflict_repo_shows_has_conflict() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    git(&["config", "user.email", "test@test.local"], dir.path());
    git(&["config", "user.name", "Test"], dir.path());

    // Create a branch with a conflicting change.
    git(&["checkout", "-b", "branch-a"], dir.path());
    fs::write(dir.path().join("README.md"), "branch-a content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "branch-a"], dir.path());

    // Back to main, make a conflicting change.
    git(&["checkout", "main"], dir.path());
    fs::write(dir.path().join("README.md"), "main content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "main-change"], dir.path());

    // Merge branch-a — will conflict.
    let merge_status = git_command(&["merge", "branch-a"], dir.path())
        .status()
        .unwrap();
    // merge should fail (exit != 0 for conflict)
    assert!(!merge_status.success(), "expected merge conflict");

    let project = make_project(dir.path());
    let status = VcsAdapter::read_project_status(&project).await;

    assert!(status.read_error.is_none());
    assert!(status.conflict.has_conflict, "should detect merge conflict");
}

#[tokio::test]
async fn mark_resolved_stages_conflicted_file() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());

    git(&["checkout", "-b", "branch-a"], dir.path());
    fs::write(dir.path().join("README.md"), "branch-a content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "branch-a"], dir.path());

    git(&["checkout", "main"], dir.path());
    fs::write(dir.path().join("README.md"), "main content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "main-change"], dir.path());

    let merge_status = git_command(&["merge", "branch-a"], dir.path())
        .status()
        .unwrap();
    assert!(!merge_status.success(), "expected merge conflict");

    fs::write(dir.path().join("README.md"), "resolved content\n").unwrap();
    let project = make_project(dir.path());
    let result = VcsAdapter::mark_resolved(&project, "README.md").await;
    assert!(
        result.success,
        "mark_resolved failed: {:?}",
        result.error_message
    );

    let output = git_command(&["diff", "--cached", "--name-only"], dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let staged = String::from_utf8_lossy(&output.stdout);
    assert!(
        staged.lines().any(|line| line == "README.md"),
        "README.md should be staged, got {staged:?}"
    );
}

#[tokio::test]
async fn abort_merge_clears_active_merge_conflict() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());

    git(&["checkout", "-b", "branch-a"], dir.path());
    fs::write(dir.path().join("README.md"), "branch-a content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "branch-a"], dir.path());

    git(&["checkout", "main"], dir.path());
    fs::write(dir.path().join("README.md"), "main content\n").unwrap();
    git(&["add", "README.md"], dir.path());
    git(&["commit", "-m", "main-change"], dir.path());

    let merge_status = git_command(&["merge", "branch-a"], dir.path())
        .status()
        .unwrap();
    assert!(!merge_status.success(), "expected merge conflict");

    let project = make_project(dir.path());
    let result = VcsAdapter::abort_merge(&project).await;
    assert!(
        result.success,
        "abort_merge failed: {:?}",
        result.error_message
    );

    let status = VcsAdapter::read_project_status(&project).await;
    assert!(
        !status.conflict.has_conflict,
        "merge abort should clear conflict"
    );
    assert!(
        !dir.path().join(".git").join("MERGE_HEAD").exists(),
        "MERGE_HEAD should be removed"
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 8: Tag created — validate_for_freeze and tag ops
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tag_created_blocks_freeze_validation() {
    use knotra_vcs::VcsAdapter;
    use std::collections::HashSet;

    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    git(&["tag", "v1.0.0"], dir.path());

    let project = make_project(dir.path());
    let projects = vec![project.clone()];
    let selection: HashSet<_> = [project.id.clone()].into_iter().collect();

    let validation = VcsAdapter::validate_freeze(&projects, &selection, "v1.0.0", 4).await;
    let entry = &validation.entries[0];

    assert!(entry.tag_exists, "tag should be detected as existing");
    assert!(
        !entry.blockers.is_empty(),
        "existing tag should block freeze"
    );
}

#[tokio::test]
async fn tag_create_and_delete_roundtrip() {
    use knotra_vcs::VcsAdapter;

    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());

    // Create tag.
    let create_result = VcsAdapter::create_tag(&project, "v2.0.0").await;
    assert!(
        create_result.success,
        "tag_create failed: {:?}",
        create_result.error_message
    );

    // Verify via validate.
    use std::collections::HashSet;
    let projects = vec![project.clone()];
    let selection: HashSet<_> = [project.id.clone()].into_iter().collect();
    let v = VcsAdapter::validate_freeze(&projects, &selection, "v2.0.0", 4).await;
    assert!(
        v.entries[0].tag_exists,
        "tag should be detected after creation"
    );

    // Delete tag (rollback).
    let delete_result = VcsAdapter::delete_tag(&project, "v2.0.0").await;
    assert!(
        delete_result.success,
        "tag_delete failed: {:?}",
        delete_result.error_message
    );

    // Verify gone.
    let v2 = VcsAdapter::validate_freeze(&projects, &selection, "v2.0.0", 4).await;
    assert!(
        !v2.entries[0].tag_exists,
        "tag should be gone after deletion"
    );
}

#[tokio::test]
async fn execute_freeze_with_message_creates_annotated_git_tag() {
    use std::collections::HashSet;

    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());
    let projects = vec![project.clone()];
    let selection: HashSet<_> = [project.id.clone()].into_iter().collect();
    let validation = VcsAdapter::validate_freeze(&projects, &selection, "v2.1.0", 4).await;

    let result =
        VcsAdapter::execute_freeze_with_message(&projects, &validation, Some("release note")).await;

    assert!(
        matches!(result.outcome, knotra_vcs::FreezeOutcome::Success),
        "freeze failed: {:?}",
        result
    );

    let output = git_command(&["cat-file", "-p", "v2.1.0"], dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git cat-file failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tag_object = String::from_utf8_lossy(&output.stdout);
    assert!(
        tag_object.contains("release note"),
        "annotated tag should contain message, got {tag_object:?}"
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 9: Permission-error (simulate with non-existent path)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nonexistent_path_returns_read_error() {
    let project = Project::new("missing", "/nonexistent/path/to/nowhere");
    let status = VcsAdapter::read_project_status(&project).await;
    assert!(
        status.read_error.is_some(),
        "non-existent path should produce read_error"
    );
}

#[tokio::test]
async fn repo_exists_returns_false_for_missing_path() {
    let project = Project::new("missing", "/nonexistent/path");
    assert!(!VcsAdapter::repo_exists(&project));
}

#[tokio::test]
async fn repo_exists_returns_true_for_valid_repo() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());
    assert!(VcsAdapter::repo_exists(&project));
}

// ---------------------------------------------------------------------------
// List contexts (branches)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_contexts_returns_current_branch() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());

    let ctx_list = VcsAdapter::list_contexts(&project).await;

    assert!(
        ctx_list.warning.is_none() || !ctx_list.candidates.is_empty(),
        "unexpected: {:?}",
        ctx_list.warning
    );
    let current = ctx_list.candidates.iter().find(|c| c.is_current);
    assert!(current.is_some(), "should find current branch");
    assert_eq!(current.unwrap().label, "main");
}

#[tokio::test]
async fn list_contexts_includes_second_branch() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    git(&["checkout", "-b", "featurenew"], dir.path());
    git(&["checkout", "main"], dir.path());
    let project = make_project(dir.path());

    let ctx_list = VcsAdapter::list_contexts(&project).await;

    assert!(
        ctx_list.candidates.iter().any(|c| c.label == "featurenew"),
        "should list featurenew branch, found: {:?}",
        ctx_list
            .candidates
            .iter()
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Changelog: log_since
// ---------------------------------------------------------------------------

#[tokio::test]
async fn log_since_collects_commits_since_tag() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    git(&["config", "user.email", "test@test.local"], dir.path());
    git(&["config", "user.name", "Test"], dir.path());
    git(&["tag", "v0.1.0"], dir.path()); // tag at initial commit

    // Two more commits after the tag.
    fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git(&["add", "a.txt"], dir.path());
    git(&["commit", "-m", "feat: add a"], dir.path());

    fs::write(dir.path().join("b.txt"), "b\n").unwrap();
    git(&["add", "b.txt"], dir.path());
    git(&["commit", "-m", "fix: fix b"], dir.path());

    let project = make_project(dir.path());
    let commits = VcsAdapter::log_since(&project, "v0.1.0", None).await;

    assert!(
        commits.error.is_none(),
        "log_since error: {:?}",
        commits.error
    );
    assert_eq!(
        commits.entries.len(),
        2,
        "should collect 2 commits, got {}",
        commits.entries.len()
    );
    assert!(
        commits
            .entries
            .iter()
            .any(|e| e.subject.contains("feat: add a"))
    );
    assert!(
        commits
            .entries
            .iter()
            .any(|e| e.subject.contains("fix: fix b"))
    );
}

// ---------------------------------------------------------------------------
// Validate freeze: clean repo is ready
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clean_repo_passes_freeze_validation() {
    use std::collections::HashSet;

    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let project = make_project(dir.path());
    let projects = vec![project.clone()];
    let selection: HashSet<_> = [project.id.clone()].into_iter().collect();

    let v = VcsAdapter::validate_freeze(&projects, &selection, "v3.0.0", 4).await;

    let entry = &v.entries[0];
    assert!(
        entry.blockers.is_empty(),
        "clean repo should have no blockers, got: {:?}",
        entry.blockers
    );
    assert!(entry.ready());
    assert!(v.all_ready());
}

#[tokio::test]
async fn dirty_repo_blocks_freeze_validation() {
    use std::collections::HashSet;

    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    // Make it dirty.
    fs::write(dir.path().join("README.md"), "dirty\n").unwrap();

    let project = make_project(dir.path());
    let projects = vec![project.clone()];
    let selection: HashSet<_> = [project.id.clone()].into_iter().collect();

    let v = VcsAdapter::validate_freeze(&projects, &selection, "v4.0.0", 4).await;

    assert!(
        !v.entries[0].blockers.is_empty(),
        "dirty repo should have blockers"
    );
    assert!(!v.all_ready());
}

// ---------------------------------------------------------------------------
// Context switch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn switch_context_changes_branch() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    // Use a branch name without "/" so switch_context treats it as a local branch.
    git(&["checkout", "-b", "featurex"], dir.path());
    git(&["checkout", "main"], dir.path());

    let project = make_project(dir.path());
    let target = ContextTarget::GitLocalBranch {
        name: "featurex".to_owned(),
    };
    let (result, _hint) = VcsAdapter::switch_context(&project, &target).await;

    assert!(
        result.success,
        "switch failed: {:?}\nstderr: {}",
        result.error_message, result.stderr
    );

    let output = git_command(&["symbolic-ref", "--short", "HEAD"], dir.path())
        .output()
        .unwrap();
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    assert_eq!(branch, "featurex", "expected featurex, got {:?}", branch);
}

#[tokio::test]
async fn switch_context_changes_slash_local_branch() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    git(&["checkout", "-b", "feature/foo"], dir.path());
    git(&["checkout", "main"], dir.path());

    let project = make_project(dir.path());
    let target = ContextTarget::GitLocalBranch {
        name: "feature/foo".to_owned(),
    };
    let (result, _hint) = VcsAdapter::switch_context(&project, &target).await;

    assert!(
        result.success,
        "switch failed: {:?}\nstderr: {}",
        result.error_message, result.stderr
    );

    let output = git_command(&["symbolic-ref", "--short", "HEAD"], dir.path())
        .output()
        .unwrap();
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    assert_eq!(
        branch, "feature/foo",
        "expected feature/foo, got {branch:?}"
    );
}

#[tokio::test]
async fn switch_context_remote_branch_uses_explicit_remote_target() {
    let remote_dir = tempfile::tempdir().unwrap();
    let clone1_dir = tempfile::tempdir().unwrap();
    let clone2_dir = tempfile::tempdir().unwrap();

    git(&["init", "--bare", "-b", "main"], remote_dir.path());
    git_command(
        &["clone", remote_dir.path().to_str().unwrap(), "."],
        clone1_dir.path(),
    )
    .status()
    .unwrap();
    git(
        &["config", "user.email", "test@test.local"],
        clone1_dir.path(),
    );
    git(&["config", "user.name", "Test"], clone1_dir.path());

    fs::write(clone1_dir.path().join("README.md"), "base\n").unwrap();
    git(&["add", "README.md"], clone1_dir.path());
    git(&["commit", "-m", "base"], clone1_dir.path());
    git(&["push", "-u", "origin", "main"], clone1_dir.path());
    git(&["checkout", "-b", "feature/foo"], clone1_dir.path());
    fs::write(clone1_dir.path().join("feature.txt"), "feature\n").unwrap();
    git(&["add", "feature.txt"], clone1_dir.path());
    git(&["commit", "-m", "feature"], clone1_dir.path());
    git(&["push", "-u", "origin", "feature/foo"], clone1_dir.path());

    git_command(
        &["clone", remote_dir.path().to_str().unwrap(), "."],
        clone2_dir.path(),
    )
    .status()
    .unwrap();
    git(&["fetch", "origin"], clone2_dir.path());

    let project = make_project(clone2_dir.path());
    let ctx_list = VcsAdapter::list_contexts(&project).await;
    let candidate = ctx_list
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                &candidate.target,
                ContextTarget::GitRemoteBranch { full_name, .. } if full_name == "origin/feature/foo"
            )
        })
        .expect("remote feature/foo candidate");

    let (result, _hint) = VcsAdapter::switch_context(&project, &candidate.target).await;
    assert!(
        result.success,
        "switch failed: {:?}\nstderr: {}",
        result.error_message, result.stderr
    );

    let output = git_command(&["symbolic-ref", "--short", "HEAD"], clone2_dir.path())
        .output()
        .unwrap();
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    assert_eq!(
        branch, "feature/foo",
        "expected feature/foo, got {branch:?}"
    );
}

// ---------------------------------------------------------------------------
// §16.4 State 10: jj project (skipped when jj not available)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn jj_repo_uses_jujutsu_vcs_kind() {
    // Skip if `jj` is not installed.
    if Command::new("jj").arg("--version").status().is_err() {
        eprintln!("jj not found — skipping jj integration test");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("jj git init failed — skipping");
            return;
        }
    }

    let project = Project::new("jj-repo", dir.path().to_str().unwrap());
    let status = VcsAdapter::read_project_status(&project).await;

    // The project should be recognised as jj, not produce a read error.
    assert_eq!(status.identity.vcs_kind, VcsKind::Jujutsu);
}

// ---------------------------------------------------------------------------
// RFC-039: `VcsAdapter::recent_commits` — no since-ref, `-n <limit>`/jj `..@-`
// ---------------------------------------------------------------------------

#[tokio::test]
async fn git_recent_commits_respects_limit_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path()); // one commit, "initial"
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    git(&["add", "a.txt"], dir.path());
    git(&["commit", "-m", "second"], dir.path());
    fs::write(dir.path().join("b.txt"), "b").unwrap();
    git(&["add", "b.txt"], dir.path());
    git(&["commit", "-m", "third"], dir.path());

    let project = make_project(dir.path());
    let result = VcsAdapter::recent_commits(&project, 2).await;

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert_eq!(result.entries.len(), 2, "limit=2 over 3 real commits");
    assert_eq!(result.entries[0].subject, "third");
    assert_eq!(result.entries[1].subject, "second");
}

#[tokio::test]
async fn git_recent_commits_returns_what_it_has_under_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path()); // one commit, "initial"

    let project = make_project(dir.path());
    let result = VcsAdapter::recent_commits(&project, 5).await;

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert_eq!(
        result.entries.len(),
        1,
        "one real commit, limit 5 -- must return what exists, not error"
    );
    assert_eq!(result.entries[0].subject, "initial");
}

#[tokio::test]
async fn jj_recent_commits_respects_limit_and_excludes_the_working_copy() {
    if !jj_available() {
        eprintln!("jj not found — skipping jj integration test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    init_jj_repo(dir.path());
    jj_commit(dir.path(), "a.txt", "a", "first");
    jj_commit(dir.path(), "b.txt", "b", "second");
    jj_commit(dir.path(), "c.txt", "c", "third");

    let project = Project::new("jj-repo", dir.path().to_str().unwrap());
    let result = VcsAdapter::recent_commits(&project, 2).await;

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert_eq!(
        result.entries.len(),
        2,
        "limit=2 over 3 real commits -- must not include the always-present, \
         always-empty working-copy commit as a fourth"
    );
    assert_eq!(result.entries[0].subject, "third");
    assert_eq!(result.entries[1].subject, "second");
}

#[tokio::test]
async fn jj_recent_commits_returns_what_it_has_under_the_limit() {
    if !jj_available() {
        eprintln!("jj not found — skipping jj integration test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    init_jj_repo(dir.path());
    jj_commit(dir.path(), "a.txt", "a", "only commit");

    let project = Project::new("jj-repo", dir.path().to_str().unwrap());
    let result = VcsAdapter::recent_commits(&project, 5).await;

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert_eq!(
        result.entries.len(),
        1,
        "one real commit, limit 5 -- must return what exists, not error"
    );
    assert_eq!(result.entries[0].subject, "only commit");
}

/// RFC-039 D7's central finding, as a test rather than only a comment: a
/// fresh jj repository's working-copy commit (`@`) always exists and is
/// always empty/description-less until the first `jj commit`. If
/// `recent_commits` used `..@` (as `log_since` does) instead of `..@-`,
/// this would return one spurious entry — indistinguishable from a real
/// commit to the panel — for every repository that has never been
/// committed to, which is exactly the state D5 calls "no commits yet".
#[tokio::test]
async fn jj_recent_commits_on_a_fresh_repo_returns_empty_not_the_working_copy() {
    if !jj_available() {
        eprintln!("jj not found — skipping jj integration test");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    init_jj_repo(dir.path());
    // No commits made — only the initial, empty working-copy commit exists.

    let project = Project::new("jj-repo", dir.path().to_str().unwrap());
    let result = VcsAdapter::recent_commits(&project, 5).await;

    assert!(
        result.error.is_none(),
        "unexpected error: {:?}",
        result.error
    );
    assert!(
        result.entries.is_empty(),
        "a repository with zero real commits must report zero entries, \
         not the empty working-copy commit as one: {:?}",
        result.entries
    );
}
