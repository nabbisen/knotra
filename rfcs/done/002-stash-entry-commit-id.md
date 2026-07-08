# RFC-002 — Add `commit_id` to knotra's `StashEntry`

| Field    | Value                                                      |
|----------|------------------------------------------------------------|
| Status      | Implemented (v0.11.0) |
| Priority | Medium — type alignment, no user-visible regression today  |
| Effort   | Small (1 struct field, 1 mapping line)                     |
| Related  | `crates/endringer/src/model/status.rs`, `vcs/git.rs`       |

## Summary

knotra's domain type `model::status::StashEntry` drops the `commit_id` field
that exists in `endringer-backend-core::types::StashEntry`.  Align the two
types so that future operations (stash show, selective stash pop) have access
to the commit OID.

## Problem

```rust
// endringer-backend-core/src/types.rs
pub struct StashEntry {
    pub index:     usize,
    pub commit_id: CommitId,   // ← present
    pub message:   String,
}

// endringer/src/model/status.rs  (knotra domain)
pub struct StashEntry {
    pub index:   usize,
    pub message: String,
    // commit_id dropped at mapping time
}
```

The backend `CommitId` is an opaque byte-vector type.  Exposing it directly in
the knotra domain layer would create a dependency on `endringer-backend-core`
from `knotra-app`, violating the boundary rules.  The solution is to store a
short hex string.

## Design

### Type change

```rust
// endringer/src/model/status.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index:     usize,
    /// Short (8-char) hex hash of the stash commit.
    pub commit_id: String,
    pub message:   String,
}
```

### Mapping change (`vcs/git.rs`)

In `git::stash_entries`, the existing mapping is:

```rust
// current
.map(|e| KnotraStash {
    index:   e.index,
    message: e.message,
})
```

Change to:

```rust
// proposed
.map(|e| KnotraStash {
    index:     e.index,
    commit_id: e.commit_id.short(),   // CommitId::short() → 8-char hex
    message:   e.message,
})
```

`CommitId::short()` is already part of the public API of
`endringer-backend-core::types::CommitId`.

### jj

`JjBackend` does not implement `stash_entries` (jj has no stash concept).
`VcsAdapter::stash_entries` returns `Vec::new()` for jj.  No change needed.

### Consumers

`commit_id` is currently unused in the UI.  Add the field but do not render
it; doing so is left to a future UI enhancement (stash detail view, stash pop
by hash).

## Test Plan

Update the existing unit test in `state::conflict_ops::tests` (or add a
dedicated one) to construct a `StashEntry` with a non-empty `commit_id` and
assert it round-trips through JSON serialization unchanged.

No integration test needed: the backend mapping is covered by the existing
`stash_entries` call inside the endringer integration suite.

## Security Considerations

None.  `commit_id` is a read-only hash string already present in the local
repository.
