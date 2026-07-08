# RFC-020 — Migrate to endringer 0.33.1

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| Status         | Implemented (v0.18.0)                                                 |
| Priority       | Low — no code changes; pure version bump                              |
| Effort         | Minimal                                                               |
| Target version | v0.18.0                                                               |
| Related        | RFC-018 (endringer migration to 0.19.2)                              |

## Summary

endringer 0.33.1 is the project's declared stable version (8/9
stabilisation gates passed; only "maintainer v1.0 approval" remains).
This RFC updates `knotra-vcs` from `endringer-* 0.19.2` to `0.33.1`.
No knotra source code changes are required.

## What changed in endringer 0.19.2 → 0.33.1

Fourteen releases added 229 tests (88 → 317), expanded the API
significantly, and contained three breaking changes in the range:

### Breaking changes — impact on knotra-vcs

**RFC-006 (v0.23.0) — typed errors: `anyhow::Result<T>` → `endringer::Result<T>`**

Every public async and sync method now returns
`Result<T, endringer::Error>` instead of `anyhow::Result<T>`.
knotra-vcs impact: **none in practice.**
- All `AsyncRepository` calls in knotra-vcs use `.await.ok()` or
  `.await.ok().map(…)` — `.ok()` discards the error type entirely.
- The two direct `VcsBackend` calls (`create_tag`, `create_annotated_tag`)
  match on `Err(e)` and call `e.to_string()`.
  `endringer::Error` implements `Display`, so `.to_string()` is unchanged.
- `GitBackend::open` still returns `anyhow::Result<Self>` (internal detail
  of `endringer-git`; not the public API boundary). Unchanged.

**RFC-022 (v0.28.0) — `TagAnnotation` gains `tagger_email: Option<String>`**

Breaking only for code that constructs `TagAnnotation` literals directly.
knotra-vcs never constructs `TagAnnotation`; it reads `TagInfo` via
`list_tags_sorted()` and maps to its own model. **No impact.**

**RFC-009 / RFC-005 (v0.25.0) — `VcsBackend` gains required methods
(`repository_info`, `ahead_behind`)**

Breaking only for crates that *implement* `VcsBackend` (custom backends).
knotra-vcs *calls* backend methods but does not implement the trait.
`GitBackend` and `JjBackend` supply these; knotra-vcs is unaffected.
**No impact.**

### New API surface of interest for future knotra work

The following methods are now available in `endringer 0.33.1` and are
relevant to known knotra roadmap items:

| Method | Relevance |
|---|---|
| `operation_state()` | Replace jj CLI conflict detection (C-2) |
| `conflict_summary()` | Typed per-path conflict info |
| `branch_ahead_behind(branch)` | Native ahead/behind; removes knotra's manual tracking |
| `ahead_behind(local, upstream)` | Symmetric divergence metric |
| `snapshot(SnapshotRequest)` | Batch read for dashboard refresh (perf) |
| `repository_info()` / `HeadState` | Typed detached-HEAD detection |
| `rich_worktree_status(options)` | Per-file staged/unstaged/conflict detail |
| `query_commits(CommitQuery)` | Bounded, pageable history |
| `diff_entries(from, to, options)` | Rename-aware diff |

These are additive; none requires immediate adoption. Each is a candidate
for a future RFC when the corresponding knotra feature reaches design.

## Decision

**Migrate.** The version bump is risk-free (zero code changes, all 36
knotra-vcs tests pass), and the stability signal is strong: 317 tests,
gix 0.84, typed errors, and a public contract fully audited against the
implementation (0.33.1 stabilisation gate 8/9).

## Implementation

One change: `crates/knotra-vcs/Cargo.toml`:

```toml
# before
endringer-core  = "0.19.2"
endringer-git   = "0.19.2"
endringer-jj    = "0.19.2"
endringer-async = "0.19.2"

# after
endringer-core  = "0.33.1"
endringer-git   = "0.33.1"
endringer-jj    = "0.33.1"
endringer-async = "0.33.1"
```

No source code changes. Verified: `cargo +1.91 check --workspace --all-targets`
0/0; `cargo +1.91 clippy --workspace --all-targets` 0/0;
knotra-vcs 36 tests pass (17 unit + 19 integration) against 0.33.1.

## Open questions

None.
