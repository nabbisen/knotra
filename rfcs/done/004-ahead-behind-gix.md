# RFC-004 — Ahead/Behind Counts via gix

| Field    | Value                                                         |
|----------|---------------------------------------------------------------|
| Status      | Implemented (v0.11.0) |
| Priority | Low — current CLI path is functional; gix path is an optimisation |
| Effort   | Spike (0.5 days) + implementation (1 day)                     |
| Related  | `crates/endringer/src/vcs/git.rs` (`read_remote_cli`)         |

## Summary

`read_remote_cli` runs `git rev-list --left-right --count HEAD...@{u}` to
compute the ahead/behind counts relative to the upstream tracking branch.
This is the last remaining `git` CLI call in the read path for Git
repositories.  Replace it with a pure-gix implementation using reference
walking.

## Current implementation

```rust
fn read_remote_cli(path: &str) -> RemoteStatus {
    let out = git_cmd(
        &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
        path
    );
    // parses "2\t1\n" → ahead=2, behind=1
}
```

Known limitations:

- Returns `(0, 0)` silently when the branch has no upstream tracking ref
  (`@{u}` is undefined).
- Returns `(0, 0)` silently in offline environments (no remote refs).
- Spawns a `git` subprocess per repository, adding latency under the
  semaphore cap.

## Requirements

1. No `git` CLI process for ahead/behind calculation.
2. Graceful behaviour when no tracking branch is configured (return
   `RemoteStatus::default()` with `upstream: None`).
3. Graceful behaviour in offline environments (remote refs stale but present
   locally after last `git fetch`).
4. Equivalent accuracy to the CLI path: counts based on local ref state
   (same as what `git fetch` brings down).

## Design

### Approach

gix provides `reference()` to resolve named refs and `Graph` (rev-walk) for
reachability.  The computation is:

```
upstream_ref = repo.find_reference("refs/remotes/<remote>/<branch>")
local_ref    = repo.head_ref()

ahead  = commits reachable from local_ref NOT reachable from upstream_ref
behind = commits reachable from upstream_ref NOT reachable from local_ref
```

This is equivalent to `git rev-list --left-right HEAD...@{u}` with `--count`.

### Spike tasks

Before implementation, confirm:

1. That `gix::Repository::find_reference("HEAD")` resolves through
   `symbolic-ref` chains to the branch ref.
2. That the tracking branch name is accessible via
   `BranchInfo::upstream_name` from `GitBackend::local_branches()` or
   directly via `repo.references()`.
3. That `gix`'s commit graph walk correctly stops at the merge-base without
   requiring `--ancestry-path`.

### Pseudocode

```rust
pub(crate) fn ahead_behind_gix(
    repo: &gix::Repository,
) -> Option<(u32, u32)> {
    let head  = repo.head_commit().ok()?;
    let head_id = head.id;

    // Resolve @{u}: find tracking ref name from HEAD's branch config.
    let tracking = upstream_ref(repo)?;
    let track_id = tracking.peel_to_commit().ok()?.id;

    // Walk both sides from the merge-base.
    let base = repo.merge_base(head_id, track_id).ok()??;

    let ahead  = commit_distance(repo, head_id,  base)? as u32;
    let behind = commit_distance(repo, track_id, base)? as u32;
    Some((ahead, behind))
}

fn commit_distance(
    repo: &gix::Repository,
    from: gix::ObjectId,
    stop: gix::ObjectId,
) -> Option<usize> {
    let mut count = 0usize;
    let mut walk  = repo.rev_walk([from]);
    walk.hide([stop]).ok()?;
    for id in walk {
        let _ = id.ok()?;
        count += 1;
    }
    Some(count)
}
```

### Fallback

If gix returns an error at any step (e.g. no upstream configured, shallow
clone, packed-refs corruption), fall back to `RemoteStatus::default()` with
`upstream: None`.  Do not fall back to the CLI.

## Implementation location

Add `gix_ahead_behind(repo_path: &str) -> RemoteStatus` as a
`pub(crate)` function in `vcs/git.rs`, replacing the call to
`read_remote_cli` inside `read_status`.  Remove `read_remote_cli` once the
gix implementation passes the integration tests.

## Test Plan

Add to `crates/endringer/tests/git_integration.rs`:

1. **`ahead_count_via_gix`** — reuse the `ahead_repo_shows_nonzero_ahead_count`
   fixture; assert `remote.ahead == 1` and `remote.behind == 0`.
2. **`behind_count_via_gix`** — reuse `behind_repo_shows_nonzero_behind_count`.
3. **`no_upstream_returns_zero`** — a repo with no upstream tracking branch
   must return `ahead=0, behind=0, upstream=None` without error.

Remove the corresponding CLI-based assertions once gix passes.

## Security Considerations

None.  All data is read from the local repository object store.
