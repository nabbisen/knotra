# RFC-047 - Unreadable history is stated, not swallowed

| Field | Value |
|---|---|
| Status | Implemented (main: 95d07a3) |
| Priority | Medium - silent data loss, though bounded to the History screen |
| Effort | Small - one function, one state field, one notice |
| Target | Production Readiness Reset - data correctness |
| Related files | `crates/knotra-app/src/persistence.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/044-...md` (**D3** - the principle this applies), `rfcs/done/046-...md` (noted this in its Security Considerations and deliberately left it) |

## Summary

`load_recent_logs` discards every file it cannot read or parse, silently, and a
discarded file **also costs the user a valid older entry**. Neither the loss nor its
cause reaches any surface.

## Problem

`persistence.rs:105`:

```rust
entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
entries.into_iter()
    .take(limit)                                    // <- before the filter
    .filter_map(|e| {
        let text = std::fs::read_to_string(e.path()).ok()?;   // <- silent
        serde_json::from_str::<OperationLog>(&text).ok()      // <- silent
    })
    .collect()
```

Three distinct failures, none of them visible:

**1. A dropped record is indistinguishable from one that never existed.** A truncated
write, a corrupted file, a hand-edited log — all vanish. `view_body` renders
`history.empty` ("no history yet") when the list comes back empty, so total loss reads
as *"you have not done anything yet."*

**2. An unreadable file costs a valid entry.** `.take(limit)` runs **before**
`filter_map`, so anything unreadable consumes one of the `limit` slots and yields
nothing in return. With `max_log_entries` = 20 and three bad files among the newest 20,
the user sees 17 entries while valid older ones sit unread on disk.

This is not only about corruption. `read_dir` returns **every** directory entry — an
editor swap file, a `.DS_Store`, a half-finished write — and each one takes a slot.

**3. An unreadable directory reads as an empty one.** `Err(_) => return Vec::new()`
(`persistence.rs:109`) makes a permissions failure or a missing mount look exactly like a
first run.

RFC-044 D3 settled the principle for topology data: **absent data must be stated, never
rendered as silence.** The same argument applies here and was noted in RFC-046's Security
Considerations, deliberately left out of that RFC's scope rather than folded in.

## Design

### D1. Filter first, then take

Discard unreadable entries **before** applying `limit`, so a bad file costs the user
nothing but itself. `limit` then means what its name says: the most recent `limit`
*logs*, not the most recent `limit` *directory entries*.

### D2. Return what was skipped, and state it

`load_recent_logs` returns the logs **and a count of entries it could not read**. The
count reaches `AppState` and the History screen states it — both above a populated list
and in place of the empty state, since "12 entries could not be read" is the more
important message when the list is otherwise empty.

Wording is the implementer's; it must name that entries exist and could not be read,
without implying the user did something wrong.

### D3. An unreadable directory is its own case

Distinguish "the history directory could not be read" from "there is no history yet".
Two different sentences; only one of them is good news.

### D4. The notice is not the status bar

This is a persistent condition, discovered at startup and true until the files change.
The status bar is for transient outcomes. It belongs on the screen whose contents are
affected.

## Test Plan

Co-located in `persistence.rs`, using the existing `tempfile` fixtures:

- A corrupt file among valid ones: **the valid ones all load** (D1's slot behaviour) and
  the skipped count is 1.
- A corrupt file when `limit` is smaller than the file count — the case that proves the
  reorder, since it fails under the current code and passes after.
- An unreadable directory reports its own state, distinct from an empty one (D3).

## Security Considerations

A user currently cannot tell the difference between *"this operation was never logged"*
and *"the log for it is gone."* Operation logs are the record of what knotra did to a
user's repositories; losing one silently means an action can become unaccounted for with
no indication. Stating the loss does not prevent it, but it is the difference between
damage and undetected damage.

No new attack surface: this reads the same files with the same parser and adds no
execution path.

## Migration / rollout

None. No format change, no schema change, nothing written differently. Users with
intact history see one difference: a corrupt file no longer costs them a valid entry.
