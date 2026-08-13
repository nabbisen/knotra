# RFC-055 - `log_since` excludes the jj working copy

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - a jj project's changelog leads with a blank entry, and a changelog is a published artifact |
| Effort | Small - one revset, one verification |
| Target | Production Readiness Reset - correctness |
| Related files | `crates/knotra-vcs/src/vcs/jj.rs` |
| Related RFCs | `rfcs/done/039-...md` (whose `recent_commits` established the correct revset), `rfcs/done/003-...md` (jj CLI as a tracked exception) |
| Found by | reviewing Handoff 075 - by running jj, which became possible one handoff earlier |

## Summary

`log_since` builds `{since}..@` for Jujutsu. `@` is jj's working-copy commit, which is
usually empty. **A jj project's changelog leads with a blank entry.**

RFC-039 already solved this for `recent_commits` and verified the fix against a real
binary. Apply the same revset here.

## Problem

### Demonstrated, not inferred

Run against a real repository (jj 0.44.0) with three commits:

```
jj log -r '<oldest>..@'  →  []            ← the working-copy commit, empty description
                            [commit 3]
                            [commit 2]
```

`jj.rs:260` builds exactly that shape. The parse loop at `:304` takes whatever sits
between the delimiters, so an empty description becomes `CommitEntry { subject: "", … }`,
and **nothing downstream filters it** — not the adapter, not `collect_changelog`, not the
changelog overlay.

### Why it survived this long

Every jj invocation in this codebase was written and reviewed without jj installed
(ruling `164` §3). This defect is not visible by reading the code — `{since}..@` is a
perfectly reasonable-looking revset, and it is the shape `recent_commits` would have
shipped too if RFC-039's D7 had not required a real binary.

The install that unblocked RFC-039 is what made this findable. It was predicted in `164`
§4 as the reason to install; this is that prediction landing.

### Why it matters more than the list it was found next to

A changelog is a **published artifact**. `recent_commits` feeds a panel a user glances at;
`log_since` feeds `collect_changelog`, which produces text that goes into release notes.
A blank leading entry is wrong in a place other people read.

## Non-goals

- Git's `log_since`. Git has no working-copy commit; `{since}..{until}` is correct there
  and is not touched.
- `recent_commits` - already correct (RFC-039).
- Filtering empty subjects anywhere downstream. See "Alternatives".
- Any change to what a changelog contains beyond removing the phantom entry.

## Decision

### D1. `{since}..@-`, matching `recent_commits`

The same revset RFC-039 verified. One rule for both jj call sites, rather than two shapes
whose difference nobody can later explain.

### D2. The described-but-uncommitted case is accepted, deliberately

`jj describe` sets a description on `@` without finalising it. Under `..@-` that work is
excluded from the changelog.

**That is correct for this artifact.** A changelog documents what has been committed; the
working copy is by definition still being written. Including it would put unfinished work
into release notes, which is a worse error than omitting it — and the omission is
recoverable by committing, while the inclusion is not recoverable at all once published.

Stated here because it is a real behavioural difference, not an edge case nobody
considered.

### D3. Verified against the binary, not the documentation

jj is installed now. The fix must be run, and the report must show output from a real
repository — including the described-but-uncommitted case from D2, which is the one place
this change removes something a user might expect.

RFC-039's D7 is why this defect was found rather than shipped twice. The same standard
applies to its fix.

## Alternatives considered

**Filter empty-subject entries downstream.** Rejected: it treats a symptom, leaves the
revset wrong for any future consumer, and would silently drop a genuinely empty-subject
git commit — which is possible, if unusual, and is a real commit rather than a phantom.

**Leave `log_since` and document it.** Rejected: the artifact is published.

## Requirements

| # | Requirement |
|---|---|
| R1 | jj's `log_since` uses `{since}..@-`; git's is unchanged |
| R2 | Verified against a real jj repository, with output in the report (D3) |
| R3 | The described-but-uncommitted case is exercised and its behaviour reported (D2) |
| R4 | A test covers it, skipping rather than failing when jj is absent - the precedent already in `git_integration.rs` |
| R5 | `recent_commits` is unchanged |
| R6 | `crates/knotra-app` and `crates/knotra-ui` are not modified |
| R7 | The suppression map stays at five |

## Test Plan

In `crates/knotra-vcs/tests/`, beside RFC-039's: a repository with commits, a `log_since`
range, and an assertion that no returned entry has an empty subject. Plus D2's case,
asserting what it does rather than that it is absent.

Skips when jj is missing, matching the existing pattern.

## Security Considerations

None. One correctness note: the blank entry reaches release notes, so this is a defect in
output other people read rather than only in what the app displays.

## Migration / rollout

No data or config change. jj users' changelogs stop leading with a blank line. Git users
see nothing different.
