# RFC-050 - Wording accuracy: the FS-watch hint, and one disclosure vocabulary

| Field | Value |
|---|---|
| Status | Implemented (main: ee763a6) |
| Priority | Medium - one user-facing sentence is inaccurate for Jujutsu users; one control uses a word that means something else |
| Effort | Small - two catalog edits, one call site, one doc table |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-ui/src/i18n.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-vcs/src/watcher.rs` |
| Related RFCs | `rfcs/done/021-...md` (the plain-language vocabulary D3 unifies onto), `rfcs/done/048-...md`, `rfcs/done/049-...md` (the two preceding wording RFCs) |
| Owner decisions | Both settled 2026-08-13: the latency framing (D1) and the vocabulary unification (D3) |

## Summary

`settings.fs_watch_hint` describes a feature by naming Git internals, and the
description is **wrong for Jujutsu users**. Separately, the History screen's disclosure
control uses its own two-word vocabulary where the rest of the application shares one -
and its Japanese wording says "Close" for a control that closes nothing.

## Problem

### The FS-watch hint is inaccurate, not merely technical

> "When enabled, knotra watches `.git/HEAD` and index for changes and refreshes
> automatically."

`sentinel_paths()` (`knotra-vcs/src/watcher.rs:73`) dispatches per VCS:

| Repository | Watched |
|---|---|
| Git | `.git/HEAD`, `.git/index`, `.git/refs` (worktree-aware) |
| Jujutsu | `.jj/working_copy`, `.jj/op_heads` |

So the sentence omits `.git/refs`, and omits Jujutsu entirely. **A jj user reads it and
reasonably concludes the feature does not apply to them.** It does. Both locales carry
the same error.

### No sentence naming coverage can be true of both

The two systems are asymmetric **by their own design**, not by knotra's:

- Git keeps working-tree state *outside* `.git`. An unstaged edit changes no sentinel, so
  it is **not** detected.
- jj records every operation through `op_heads` and snapshots the working copy, so
  essentially any jj activity is.

Any wording implying uniform coverage is therefore false for Git. The previous draft of
this fix - *"notices changes you make outside knotra"* - would have replaced one
inaccuracy with a broader one.

**And nothing links the catalog string to `sentinel_paths()`.** No guard can tell us when
a path-naming sentence goes stale, which is the actual maintenance cost here.

### `watcher.rs`'s own doc table is also incomplete

The module table (`:19-22`) lists `.jj/working_copy/` and omits `.jj/op_heads`, which
`sentinel_paths` watches. The source the string should agree with does not agree with
itself.

### Two vocabularies for one control

| Key | English | Japanese | Call sites |
|---|---|---|---|
| `plain.show_details` / `plain.hide_details` | "Show details" / "Hide details" | 詳細を表示 / 詳細を隠す | **5** - dashboard row, context switch, smart pull, conflict, freezer |
| `history.expand` / `history.collapse` | "Details" / "Hide" | 詳細 / **閉じる** | **1** - `history.rs:185`/`:187` |

The Japanese is worse than inconsistent: 閉じる means *close*, and the control does not
close anything - it collapses a detail that stays on screen. `action.close` already uses
閉じる for actual closing.

## Non-goals

- Changing what the FS watcher watches, or when it polls. This is a description fix.
- The `.git/refs` and `op_heads` behaviour itself.
- Any other `settings.*` wording.
- A guard tying descriptions to implementation - see D1's note on why one is not
  proposed.

## Decision

### D1. The hint describes latency, not coverage (owner-approved)

English, as approved:

> "When enabled, knotra refreshes sooner after activity in your projects, instead of
> waiting for the next scheduled check."

Japanese is the implementer's, matching that sense.

This is durable for three reasons, and durability is the point:

1. It makes **no claim about which changes**, so the Git/jj asymmetry cannot falsify it.
2. It survives any change to `sentinel_paths()` - and since nothing will warn us when a
   path-naming sentence goes stale, wording that does not depend on the path list is the
   only real protection.
3. It is **what the feature actually is**. The periodic refresh still catches everything
   eventually; FS-watch improves latency, not completeness. The current sentence implies
   otherwise, which is its deeper error.

### D2. `watcher.rs`'s doc table lists `.jj/op_heads`

Correct the module table to match `sentinel_paths`. **This is the one `knotra-vcs`
change in this RFC, and it is a comment.**

### D3. History adopts the shared disclosure vocabulary (owner-approved)

`history.rs:185`/`:187` use `plain.hide_details` / `plain.show_details`. Remove
`history.expand` and `history.collapse` from both catalogs.

One call site moves to the vocabulary five others already share; four catalog entries go.

## Requirements

| # | Requirement |
|---|---|
| R1 | The hint names no file path and makes no claim about which changes are detected, in either locale |
| R2 | The English hint is the sentence in D1, verbatim |
| R3 | `watcher.rs`'s doc table matches `sentinel_paths`; **no non-comment line in `knotra-vcs` changes** |
| R4 | `history.rs`'s disclosure control uses `plain.show_details`/`plain.hide_details` |
| R5 | `history.expand` and `history.collapse` are removed from **both** catalogs, after confirming `history.rs` was their only referent |
| R6 | All catalog guards stay green, including RFC-048's and RFC-049's |
| R7 | `crates/knotra-app/src/tests.rs` is not edited |

## Test Plan

No new tests. Nothing changes behaviour: one sentence, one doc table, one call site
retargeted, four keys removed.

`every_literal_t_call_names_an_existing_key` is the guard that matters here - it will
fail if `history.expand`/`collapse` are removed while still referenced, which is exactly
the mistake this could make.

## Security Considerations

None. One accuracy note: a user deciding whether to enable a feature is entitled to a
true description of it, and a Jujutsu user currently has a false one.

## Migration / rollout

No data or config change. Users see a corrected hint and, on the History screen, the same
disclosure wording the rest of the application uses. Japanese users stop being told
"Close" by a control that collapses.
