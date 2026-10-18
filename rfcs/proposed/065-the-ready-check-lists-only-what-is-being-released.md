# RFC-065 - The ready check lists only what is being released, and says why exactly

| Field | Value |
|---|---|
| Status | Proposed 2026-10-19 |
| Priority | High - the Freezer's ready check misleads users about what is being released |
| Effort | Medium - one model reshaped in `knotra-vcs`, its two producers, one view, catalog keys |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/vcs/{adapter,git,jj}.rs`, `crates/knotra-app/src/view/overlays/freezer.rs`, `crates/knotra-app/src/state/topology.rs`, `crates/knotra-ui/src/i18n.rs`, `crates/knotra-vcs/tests/git_integration.rs` |
| Related | `rfcs/done/044-...md` (impact warnings in the same panel), `.git-exclude/reviewed/212-...md` (the Store screenshot that shows the defect) |
| Decided by | project owner, 2026-10-19 - "Agreed. Draft it." |

## Summary

"Save release point" runs a ready check, then lists the result. Today that list holds **every
project in the workspace**. The ones not selected are marked "Not included", or, if they have
a problem, "!" with that problem, exactly as a selected project that blocks the release would
be. The list also explains each problem by **substring-matching an English sentence**, which
already gives Jujutsu users the wrong message.

After this RFC, the ready check lists **only the projects being released**. Each one says
**every** reason it cannot be released, from a typed reason rather than a parsed sentence.
And no selected project can drop out of the check unseen.

## Problem

All measured in the code at `2a31434`.

**1. Unselected projects are checked and listed.** `VcsAdapter::validate_freeze` validates
every workspace project, marking each `included` or not; the view lists them all. The Store
screenshot `04-freezer` shows it: two projects selected, six listed, and two of the four
unselected shown as "!" with "Has unsaved work." and "Needs your choice — resolve it first."
— read naturally, *the release is blocked by projects I did not choose.* It is not: Save
counts only selected projects. Nothing uses the unselected entries (the RFC-044 impact check
filters to `included`; dependents come from the topology graph), so they also cost a
repository open per unselected project, on every check.

**2. Reasons are parsed from English sentences.** `knotra-vcs` records blockers as strings
(`"unresolved conflict"`, `"3 uncommitted change(s)"`, `"tag 'v1' already exists"`), and
`plain_blocker` in the view maps them back by substring:

| Producer string | Mapped to |
|---|---|
| Git `"tag 'v1' already exists"` | "This release name is already in use…" |
| **Jujutsu `"bookmark 'v1' already exists"`** | **"Needs help"** — no `tag` in it, so it falls through |
| `"cannot open repository: …"`, `"no repository at …"`, `"task join error: …"` | "Needs help" |

The Jujutsu case is a live bug: the user is told something is wrong, not that the name is
taken. And any future rewording in `knotra-vcs` silently changes what users see.

**3. Only the first reason is shown.** A project with a conflict *and* a taken name shows
one. The user fixes it, checks again, and only then learns of the second.

**4. A selected project can vanish from the check.** `validate_freeze` collects results with
`if let Ok(e) = h.await`. If a project's validation task panics, its entry is dropped
without trace. Execution then freezes only the entries it has, and **reports success for a
release that silently left that project out.**

**5. Two fields duplicate the blockers, and one is computed and never shown.**
`is_clean` and `tag_exists` restate what `blockers` already says; nothing outside tests reads
them. `notes` ("3 untracked file(s)") is computed for Git and never displayed — yet it is
exactly what a user needs to know: **new files not yet added to the project are not part of
the release point.**

## Non-goals

- What Freezer *does*: tag creation, rollback, push offer, impact warnings — unchanged.
- The dashboard's selection model, and how the Freezer is opened.
- Jujutsu untracked-file notes: Jujutsu snapshots new files automatically, so it has none.

## Decision

### D1. The ready check covers exactly the selection

`validate_freeze` validates **only selected projects**, in workspace order. `included` is
removed from `FreezeValidationEntry`: every entry is part of the release by construction, so
the field could only ever say `true`. `ready()` becomes `!is_blocked()`; `all_ready`,
`blocked_count`, `ready_count` lose their `included` filters, and `topology::warnings_for`
uses every entry. `plain.release.row_excluded` ("Not included" / "対象外") is deleted from
both catalogs.

### D2. Typed reasons, owned by `knotra-vcs`

```rust
pub enum FreezeBlocker {
    UnresolvedConflict,
    UncommittedChanges,
    NameInUse,          // a Git tag or a Jujutsu bookmark with this name exists
    Unreadable { reason: String }, // repository missing, unopenable, or its check failed
}

pub enum FreezeNote {
    UntrackedFiles(usize),
}
```

`blockers: Vec<FreezeBlocker>` and `notes: Vec<FreezeNote>` replace the string vectors, with
the same `Serialize`/`Deserialize` derives as the rest of the model. The diagnostic text the
strings carried (the open error, the path, the join error) travels **inside the variant it
explains**, `Unreadable { reason }`, so it can never describe a different problem from the
one shown. The view shows `reason` only under "Show details", as the result screen already
does for recovery commands. The ready check has no such toggle today: it gains the result
screen's "Show details" button, sharing the same `show_op_details` state, **only when some
entry is `Unreadable`** — otherwise there is nothing to show and no button. `is_clean` and
`tag_exists` are removed.

The view maps each variant with an **exhaustive `match`** — no wildcard arm — so adding a
variant without a message fails to compile. `plain_blocker` is deleted.

### D3. Every reason is shown

A blocked row lists each blocker on its own line, once each, in the enum's declaration
order. A ready row says "Ready", followed by any notes.

### D4. Nothing drops out

A project whose validation task fails to join yields an entry with
`FreezeBlocker::Unreadable { reason }` carrying the join error. **`validate_freeze` returns
exactly one entry per selected project.** Factor the collection step so a test can hand it a
task that panics, and prove the entry survives.

### D5. The untracked-files note

`FreezeNote::UntrackedFiles(n)` renders as a "label: count" line, following the catalog's
established convention (RFC-038 Stage 4 §1b), which needs no plural forms:

| Key | English | Japanese |
|---|---|---|
| `plain.release.note_untracked` | `New files not in this release point:` | `このリリースポイントに含まれない新しいファイル:` |
| `plain.release.blocker_unreadable` | `Couldn't read this project. Check that its folder still exists.` | `このプロジェクトを読み取れませんでした。フォルダーがあるか確認してください。` |

The three existing blocker keys keep their text. **Japanese copy is a draft for the owner's
review.**

## Requirements

| # | Requirement |
|---|---|
| R1 | The ready check lists exactly the selected projects; no unselected project is opened or shown |
| R2 | `FreezeValidationEntry` has no `included`, `is_clean` or `tag_exists`; `blockers`/`notes` are typed (D2) |
| R3 | The view maps blockers and notes with exhaustive `match`es; `plain_blocker` and every substring test on blocker text are gone |
| R4 | A Jujutsu project whose bookmark name is taken shows the "name already in use" message — a test asserts the Jujutsu producer emits `NameInUse` |
| R5 | A project with several blockers shows all of them |
| R6 | `validate_freeze` returns one entry per selected project even when a task fails; a test plants the failure |
| R7 | Git's untracked count appears as D5's note on a ready row |
| R8 | `plain.release.row_excluded` is removed from both catalogs; D5's keys are added to both |
| R9 | `Unreadable`'s `reason` is shown only under "Show details" |
| R10 | Save's enabled/disabled logic and its reasons are unchanged in behaviour |
| R11 | Confirm before changing that `FreezeValidation` is not persisted anywhere (history, config); if it is, stop and report |
| R12 | CHANGELOG `[Unreleased]`: one `### Fixed` entry in user terms |

## Test Plan

- `knotra-vcs` integration tests: the existing freeze tests move to typed assertions
  (`blockers.contains(&FreezeBlocker::NameInUse)`); a new test validates a two-project
  workspace with one selected and asserts **one** entry (R1).
- A Jujutsu producer test for `NameInUse` (R4), if the suite can create a jj repository;
  otherwise a unit test at the mapping point, and the report says which.
- The planted task failure (R6), seen to fail on the old `if let Ok` code first.
- Gates; test count reported with the delta explained.
- **Screenshots, English and Japanese, light:** the ready check with the Task 087 demo
  environment and the same two-project selection. These replace the Store set's
  `04-freezer` before the next Store update; Task 087's privacy rules apply in full.

## Security Considerations

None new. `Unreadable`'s `reason` can contain a filesystem path. It was already shown to the
user (as the "no repository at …" blocker); it now sits behind "Show details", and is not
written anywhere new.

## Migration / rollout

`FreezeValidation`, `FreezeValidationEntry` and the new enums are public in `knotra-vcs`, a
published crate: this is a breaking change, carried by the next minor release. No stored data
changes, subject to R11.
