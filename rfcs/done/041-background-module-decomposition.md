# RFC-041 - `handle_background` Decomposition

| Field | Value |
|---|---|
| Status | Implemented (main: f3e69aa) |
| Priority | Medium - RFC-040's one declared exception; no external pressure, but it is the last structural debt from that RFC |
| Effort | Small-to-medium - one function, twenty arms, no behaviour change |
| Target | Production Readiness Reset - operational hygiene track |
| Related files | `crates/knotra-app/src/app/background.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/tests.rs` |
| Related RFCs | `rfcs/done/040-app-module-decomposition.md` (R2/D2 defers this here by name) |
| Related audit evidence | `.git-exclude/reviewed/115-release-0.25.0-scope-and-readiness-report.md` §5 (listed as shipping debt) |

## Implementation Record

| Stage | Commits |
|---|---|
| 1 | `4d9103c` `af11f9a` |
| 2 | `54fed21` |
| 3 | `48199bc` |
| 4 | `f3e69aa` |

Accepted 2026-08-08 by the project owner; implemented across four stages, each
independently green on all five gates.

**Outcome against R1.** `background.rs` at 761 ELOC became `background/` at 906
across seven files, every one under the 500-ELOC threshold: `mod.rs` 173,
`smart_pull.rs` 279, `freeze.rs` 164, `fetch.rs` 109, `context_switch.rs` 66,
`status.rs` 65, `conflict.rs` 50. No residual exception.

**Outcome against R2/R5.** Twenty-one items - eighteen arm bodies and three whole
helper functions - moved with byte-identity evidence on each, reproduced
independently at review. One item differed by a rustfmt rewrap only, confirmed
content-identical with whitespace stripped. `crates/knotra-app/src/tests.rs` was
never edited, across all four stages.

**On D2.** Total ELOC rose +145, at the top of the predicted +100-150. The
prediction mattered less than expected: byte-identity measures directly what a size
delta only proxies, which is the mistake RFC-040 R10 made in reverse.

Review artifacts: `.git-exclude/reviewed/125` (Stage 1), `126` (Stage 2), `127`
(Stage 3), `128` (Stage 4 and closure).

**Correction recorded at closure.** This RFC's Motivation claims `background.rs` was
"the only module over" the threshold. That held for `crates/knotra-app/src/app/` and
not for the workspace, where seven other files exceed 500 ELOC - `view/bulk_modals.rs`
at 1337 being nearly twice `background.rs`'s starting size. R1 was scoped to
`app/background/` and is satisfied; the motivating sentence overstated the case.
Detail in `128` §6.

## Summary

`crates/knotra-app/src/app/background.rs` is **761 ELOC**, roughly 1.5x the
project's 500-ELOC threshold, and 678 of those lines are a single function:
`handle_background`, one `match` with twenty arms. This RFC splits it into a
`background/` directory of six domain modules plus a dispatch `mod.rs`, with no
behaviour change.

RFC-040 R2/D2 deferred this deliberately, naming the obstacle: each arm binds
variables out of its own message pattern, so extracting an arm is not a pure move
the way RFC-040's other stages were. That obstacle is real but smaller than it
looked, and §"The fact that makes this cheap and safe" below is the design pass
RFC-040 said this needed.

## Background

### The measured problem

Measured at `5b8c904`, by arm, grouped by domain:

| Group | ELOC | Arms |
|---|---|---|
| smart pull | 264 | `SmartPullRetryStatusReady`, `SmartPullProjectCompleted`, `SmartPullPlanReady` |
| freeze / tag | 149 | `FreezeExecutionDone`, `TagPushCompleted`, `FreezeValidationDone`, `TagsLoaded` |
| fetch | 94 | `ActivityFetchRetryProjectCompleted`, `SingleFetchCompleted`, `BulkFetchCompleted` |
| shared helpers | 80 | `persist_log`, `merge_workspace_status`, `skipped_retry_result` |
| context switch | 55 | `ContextSwitchDone`, `ContextListLoaded` |
| status / misc | 48 | `WorkspaceStatusRefreshed`, `TopologyScanned`, `ChangelogDraftReady`, `MissingProjectsDetected`, `TaskError` |
| conflict | 40 | `ConflictOperationCompleted`, `ConflictFilesLoaded` |
| imports, signature, shared or-arm | 31 | - |
| **total** | **761** | **20** |

The distribution is what makes this worth doing: two arms
(`SmartPullProjectCompleted` at 166 raw lines, `SmartPullRetryStatusReady` at 109)
are larger than seven of the eleven sibling modules RFC-040 produced. A reader
looking for conflict-completion behaviour scrolls past both.

### Why now, specifically

No external pressure. `rfcs/proposed/` is empty, 0.25.0 is released, and every
handoff from 033 through 036 is closed. This is the last item RFC-040 left open,
and it is cheapest to do while nothing is landing on top of it — the same argument
RFC-040 §"Why now" made about `app.rs`, which held.

### The fact that makes this cheap and safe

RFC-040 D2 recorded the blocker, and the module's own doc comment repeats it:

> Splitting it by arm is not a move: each arm would need an invented signature to
> carry those bindings across a function boundary.

**Half of that is right and half is not, and the difference decides this RFC.**

**The signature is derived, not invented.** `SmartPullProjectCompleted { lease_id,
project_id, result }` yields exactly one sensible signature — `fn(state, lease_id,
project_id, result) -> Task<Message>`. There is no design freedom to get wrong, and
no two people would write it differently. That is mechanical, not creative.

**The eighteen early `return`s are semantically neutral.** This is the part that
looked dangerous and is not. `handle_background`'s body *is* the match — there is no
code after it. So for any arm:

```rust
// today
Msg::A { x } => { if guard { return Task::none(); } do_work(state, x) }

// after
Msg::A { x } => handle_a(state, x),
fn handle_a(state: &mut AppState, x: X) -> Task<Message> {
    if guard { return Task::none(); }
    do_work(state, x)
}
```

`return` currently exits `handle_background`, whose value is the match's value,
which is the arm's value. After extraction it exits `handle_a`, whose value becomes
the arm's value, which becomes `handle_background`'s value. **Identical**, and the
same holds for the tail expression of every arm without an early return.

This is the concrete answer to RFC-040's stated risk that "the `handle_background`
split changes concurrency behaviour." Nothing about message ordering, lease
handling, or `Task` scheduling depends on which function a body is written in. The
work stays synchronous inside `update()`, in the same order, returning the same
`Task`.

**R4 requires this be verified rather than believed.** It is my analysis, not a
measurement.

### The shape already present

The domains are not invented for this RFC. `app/` already contains `sync.rs`,
`freezer.rs`, `context.rs`, `conflict_ops.rs`, `changelog.rs`, and `activity.rs` —
the user-initiated halves of the same features whose completions this module holds.
The seam is proven; RFC-040 cut along it eleven times.

## Motivation

1. **Close RFC-040's last open item.** It was deferred with a named successor, on
   record in the module's own doc comment. Leaving it indefinitely turns a scheduled
   deferral into a permanent exception.
2. **The threshold means something or it does not.** `background.rs` is the only
   module over it. One standing exception is a precedent; zero is a rule.
3. **Concentrated risk.** The most concurrency-sensitive code in the application is
   also its least navigable. Those should not be the same file.

## Non-goals

- **No behaviour change of any kind.** Not a refactor-and-improve; a move.
- **No change to `BackgroundMessage`.** Splitting it into per-domain sub-enums
  would be cleaner in the abstract and is explicitly out of scope: it is a real API
  change touching `message.rs` and every emitter, it would make the diff impossible
  to verify by the byte-identity technique R5 relies on, and it buys nothing this
  RFC needs. Revisit separately if ever.
- **No async or `Task` restructuring.**
- **No `tests.rs` edits.** It has survived RFC-040, RFC-035, and Handoffs 033-036
  untouched. That property is what makes every passing count meaningful.
- **No merging of the completion halves into their existing domain modules.** See
  Alternatives.

## Decision

### D1. A `background/` directory, split by message domain

`app/background.rs` becomes `app/background/`:

| File | Projected ELOC |
|---|---|
| `mod.rs` - dispatch, shared helpers, shared or-arm | ~140 |
| `smart_pull.rs` | ~290 |
| `freeze.rs` | ~170 |
| `fetch.rs` | ~110 |
| `context_switch.rs` | ~70 |
| `status.rs` | ~60 |
| `conflict.rs` | ~55 |

Projections add a signature, a closing brace, and a `use` block per module to the
measured arm totals. All are under threshold; the largest is `smart_pull.rs`.

### D2. Total ELOC will rise, and that is not evidence of rewriting

Expect roughly +100 to +150 across the directory, essentially all `use` blocks and
function signatures. RFC-040 R10 initially treated a size increase as a sign that
logic had been rewritten; that was wrong there (+125 of +140 was imports) and it
would be wrong here. Predicted in advance so it is not relitigated in review.

### D3. Three helpers are shared; two are domain-local

Measured by call site, not by name:

| Helper | Callers | Home |
|---|---|---|
| `persist_log` | 7 arms across 5 domains | `mod.rs` |
| `merge_workspace_status` | `WorkspaceStatusRefreshed`, `SmartPullRetryStatusReady` | `mod.rs` |
| `skipped_retry_result` | `ActivityFetchRetryProjectCompleted`, `SmartPullProjectCompleted` | `mod.rs` |
| `find_project_name` | smart pull only | `smart_pull.rs` |
| `git_push_offer_for_freeze` | `FreezeExecutionDone` only | `freeze.rs` |
| `project_is_git_for_push` | `git_push_offer_for_freeze` only | `freeze.rs` |

`skipped_retry_result` is passed as a bare function reference
(`.map(skipped_retry_result)`), not called with parentheses. A grep for
`skipped_retry_result(` misses both call sites — noted because it misled me during
this RFC's own investigation.

### D4. The or-pattern arm stays in `mod.rs`

```rust
BackgroundMessage::SmartPullCompleted(log)
| BackgroundMessage::ContextSwitchCompleted(log)
| BackgroundMessage::FreezeCompleted(log) => { persist_log(&log, state); Task::none() }
```

Three domains, two lines of body. Splitting it would mean either duplicating it
three times or inventing a home it does not belong in. It stays where the shared
helper it calls lives.

### D5. Names avoid the collisions RFC-040 hit

`app/context.rs` and `app/shared.rs` already exist. Therefore
**`context_switch.rs`**, not `context.rs`, and the shared helpers stay in `mod.rs`
rather than becoming a second `shared.rs`. RFC-040 R1a lost a review round to a
module name collision; this pre-empts the same class.

### D6. Stages, smallest first, most sensitive last

| Stage | Content | Rationale |
|---|---|---|
| 1 | `background/` scaffold, `mod.rs`, `status.rs`, `conflict.rs` | Smallest arms establish the pattern under review before it is applied at scale |
| 2 | `fetch.rs`, `context_switch.rs` | Mid-size, independent domains |
| 3 | `freeze.rs` | Brings the two domain-local helpers with it |
| 4 | `smart_pull.rs` | Largest and most concurrency-sensitive; done once the pattern is settled |

Each stage independently green on all five gates. This mirrors RFC-040 D5, which
worked.

## Requirements

| # | Requirement |
|---|---|
| R1 | Every module in `app/background/` is under 500 ELOC, `mod.rs` included |
| R2 | No behaviour change. Arm bodies move verbatim apart from the destructuring line and indentation |
| R3 | Each extracted function's parameters are exactly the bindings its message pattern produces - no reordering, no bundling into new structs, no added arguments |
| R4 | The review request states, for each arm containing an early `return`, that the extracted form preserves it - checked, not assumed from this RFC's §"The fact that makes this cheap and safe" |
| R5 | Byte-identity evidence per moved arm, per RFC-040's technique: extract the arm body at the parent commit and the function body at the child, diff, report "identical modulo signature and rustfmt" or explain |
| R6 | `crates/knotra-app/src/tests.rs` is not edited. Zero lines, across all stages |
| R7 | `BackgroundMessage` is not modified |
| R8 | Visibility widened only as far as a move requires (RFC-040 D3) |
| R9 | The module doc comment's RFC-040 R2/D2 exception note is removed, since the exception no longer applies, and replaced with a pointer to this RFC |

## Verification

Per stage:

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check
```

Baseline at `5b8c904`: **255 tests**, clippy clean. The count should not change —
this RFC adds no behaviour to test. A changed count is a signal to stop and explain,
not to update the number.

CI runs all five gates plus the MSRV check on push.

Final: report per-module ELOC against R1, and `git diff --numstat` on `tests.rs`
showing zero.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| An extracted arm changes behaviour | Defects in the most concurrency-sensitive code | R2/R5 byte-identity; R4's explicit early-`return` check; 255 unmodified tests; stages smallest-first |
| An early `return` is mis-transcribed | Silent control-flow change | R4 requires per-arm confirmation rather than a blanket claim |
| A borrow that compiled inline fails across a function boundary | Stage stalls | Expected in the largest arms; report rather than work around it by restructuring the body - that would breach R2 |
| Module name collision | Lost review round, as in RFC-040 R1a | D5 |
| Total ELOC rises and reads as rewriting | Relitigation in review | D2 predicts it |

## Alternatives considered

**Move each completion handler into its existing domain module** — put the smart
pull completions in `sync.rs`, freeze completions in `freezer.rs`, and so on, so
each feature's initiation and completion live together. Conceptually the most
attractive option, and **rejected on measurement**: `sync.rs` is 374 ELOC and the
smart pull completions are 264, giving 638 — a new over-threshold module, trading
one violation for another. `freezer.rs` (154) + 149 = 303 would have been fine, but
a rule that applies to some domains and not others is worse than a consistent
directory.

**Split by size rather than domain** — `background_a.rs`, `background_b.rs`, or a
split at the midpoint. Meets R1 and nothing else. No reader would predict which file
holds a given arm.

**Split `BackgroundMessage` into per-domain sub-enums**, letting each submodule own
a total match. Structurally cleanest end state, and out of scope: it is an API change
across `message.rs` and every emitter, and it destroys the byte-identity verification
R5 depends on. If it is ever wanted, it is a separate RFC that should follow this one,
not replace it.

**Leave it.** The status quo. Rejected because the exception was granted with a named
successor and a scheduled time, and that time is now — but it is a real option, and
the cost of taking it is one permanently over-threshold module rather than anything
breaking.
