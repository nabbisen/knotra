# RFC-0003 — jj Conflict Detection: gix Path or Documented Exception

| Field    | Value                                                            |
|----------|------------------------------------------------------------------|
| Status      | Implemented (v0.11.0) |
| Priority | Medium — correctness claim vs. implementation reality            |
| Effort   | Investigation spike (1–2 days) + implementation or doc change    |
| Related  | `crates/endringer/src/vcs/jj.rs`, `docs/src/contributing/architecture.md` |

## Summary

knotra declares that "jj read operations do not require the `jj` binary."
This is true for all read paths **except** conflict detection, which calls
`jj log -r @ --no-graph -T conflict\n` via the CLI.  Either remove this
exception by reading the conflict flag from disk with gix, or formally document
it as an intentional exception.

## Background

After the v0.10 migration, the jj read path is:

```
AsyncRepository::open_jj(path)
  → JjBackend::open(path)
      → reads .jj/repo/store/git/ via gix
      → is_dirty(), worktree_status(), list_commits(), …  — all gix-based
```

One exception remains in `vcs/jj.rs`:

```rust
let has_conflict = run_jj(
    &["jj", "log", "-r", "@", "--no-graph", "-T", "conflict\n"],
    &project.path,
).map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
 .unwrap_or(false);
```

This call is made during `read_status` and `validate_for_freeze`.

## Requirements

1. The conflict-detection result must be **reliable**: false negatives are
   worse than false positives (a conflict that is not shown blocks the
   user silently).
2. Behaviour when `jj` is absent must be **defined**: currently the call
   silently returns `false` (`unwrap_or(false)`), which is a false negative.
3. The implementation must be consistent with the stated architecture.

## Option A — Read from disk (gix path)

jj stores the conflict flag for the working copy in a protobuf-encoded file at:

```
.jj/working_copy/checkout       (jj ≤ 0.17)
.jj/working_copy/tree_state     (jj ≥ 0.18)
```

The `tree_state` file is a protobuf message.  The `has_conflict` boolean is
field 3 of the top-level message (`TreeState.has_conflict`).

### Spike task

1. Confirm the path and field number on jj ≥ 0.18 by inspecting a repository
   that has a known conflict.
2. Parse the protobuf manually (the file is small; use the
   [`prost`](https://crates.io/crates/prost) crate or raw varint decoding).
3. Return `has_conflict: true` when field 3 is present and set.

### Pros / Cons

| | Option A |
|---|---|
| ✓ | Eliminates the `jj` binary dependency for reads entirely |
| ✓ | Works offline and in environments without `jj` installed |
| ✗ | Requires protobuf parsing (adds complexity and a dependency) |
| ✗ | Tight coupling to jj's on-disk format; must track jj releases |
| ✗ | Spike needed to confirm the format is stable |

## Option B — Documented exception

Keep the CLI call but make the behaviour on `jj`-absent explicit:

```rust
// vcs/jj.rs — proposed change
let has_conflict = match run_jj(
    &["log", "-r", "@", "--no-graph", "-T", "conflict\n"],
    &project.path,
) {
    Err(_) => {
        // jj binary not found; conflict detection unavailable.
        // Return None so the UI shows "Unknown" rather than "No conflict".
        return ConflictStatus { has_conflict: false, conflict_count: None };
    }
    Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "true",
};
```

Update the architecture documentation to state:

> **jj conflict detection requires the `jj` binary.**  
> All other jj read operations use gix directly.  
> If `jj` is not installed, conflict status is reported as Unknown.

### Pros / Cons

| | Option B |
|---|---|
| ✓ | No new dependencies or format-coupling risk |
| ✓ | Unambiguous behaviour when `jj` is absent |
| ✗ | The "no binary needed" claim becomes partially false |
| ✗ | Requires `jj` even for read-only status display |

## Recommendation

**Implement Option B now; revisit Option A if jj adoption grows.**

The protobuf format is not part of jj's public API and may change without
notice between jj releases.  The risk of silent false negatives (a conflict
that looks like "clean") is higher than the inconvenience of requiring `jj` for
one read operation.

The `unwrap_or(false)` fallback must be changed to make the absence of `jj`
observable in the UI (Unknown status, not Synced).

## Design (Option B, detailed)

### `ConflictStatus` change

Add a third state to `ConflictStatus`:

```rust
// endringer/src/model/status.rs
pub struct ConflictStatus {
    pub has_conflict:    bool,
    pub conflict_count:  Option<u32>,
    /// True when the conflict check could not be run (e.g. jj absent).
    pub detection_unavailable: bool,
}
```

`knotra-app` maps `detection_unavailable == true` to the `Unknown` status
badge instead of `Synced`.

### `jj.rs` change

```rust
fn detect_conflict(path: &str) -> ConflictStatus {
    let output = std::process::Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "conflict\n"])
        .current_dir(path)
        .output();

    match output {
        Err(_) => ConflictStatus {
            has_conflict: false,
            conflict_count: None,
            detection_unavailable: true,
        },
        Ok(o) => {
            let flag = String::from_utf8_lossy(&o.stdout).trim() == "true";
            ConflictStatus {
                has_conflict: flag,
                conflict_count: None,
                detection_unavailable: false,
            }
        }
    }
}
```

### Documentation change

Update `docs/src/contributing/architecture.md` to list the exception
explicitly under "Read / Write split."

## Test Plan

1. **Unit test** — `detect_conflict` on a path where `jj` is guaranteed absent
   (mock the PATH or call with a non-repo path) returns
   `detection_unavailable: true`.
2. **Integration test** — existing `jj_repo_uses_jujutsu_vcs_kind` test
   already skips when `jj` is not installed; no change required.
3. **Manual test** — create a jj repository with a conflict, run knotra, confirm
   the dashboard card shows `Conflict` rather than `Synced`.

## Security Considerations

None beyond the general CLI-invocation considerations that already apply to
write operations.
