# RFC-043 - Eliminate `#[allow(dead_code)]`

| Field | Value |
|---|---|
| Status | Accepted (2026-08-10, project owner) - implementation authorised, not yet shipped |
| Priority | High - the suppressions have already cost two completed RFCs real work on unreachable code, and hid it from both |
| Effort | Medium - 39 suppressions, 176 findings, most of the work is triage rather than deletion |
| Target | Production Readiness Reset - operational hygiene track |
| Related files | `crates/knotra-app/src/message.rs` (24 of 39), `crates/knotra-app/src/state*`, `crates/knotra-app/src/view/history.rs` |
| Related RFCs | `rfcs/done/041-background-module-decomposition.md` (moved unreachable arms), `rfcs/done/042-catalog-integrity.md` (localised unreachable strings), `rfcs/accepted/038-settings-and-history.md` (Stage 5's premise depends on this) |

## Summary

Owner direction, 2026-08-10:

> We should avoid `#[allow(dead_code)]` generally, for it brings less readability and
> maintainability, and therefore less security.

`crates/knotra-app` carries **39 suppressions hiding 176 dead-code findings**,
measured with `cargo clippy -- --force-warn dead_code`. Most are applied to **whole
enums**, so every variant of those types is exempt permanently.

This is not a hygiene abstraction. The suppressions have already caused two completed
RFCs to do careful work on code that cannot execute, and hid that fact from both.

## Background

### The measurement

```
cargo +1.91 clippy -p knotra --all-targets -- --force-warn dead_code
```

**176 findings**, read-only, no file modified. Against **39** `#[allow(dead_code)]`
attributes, 24 of them in `message.rs`.

The shape matters: most are not per-item. `#[allow(dead_code)] pub enum
WorkspaceMessage` exempts every variant of that enum, now and forever, including ones
added later.

### What it has already cost

**Six `BackgroundMessage` variants are never constructed in production.** Each has
exactly one reference in the entire tree - its own match arm:

| Variant | References | Production construction sites |
|---|---|---|
| `BulkFetchCompleted` | 1 | 0 |
| `SmartPullCompleted` | 1 | 0 |
| `ContextSwitchCompleted` | 1 | 0 |
| `FreezeCompleted` | 1 | 0 |
| `MissingProjectsDetected` | 1 | 0 |
| `TaskError` | 2 | 0 (one construction, in `tests.rs`) |

Every live variant has 2-6 references. The contrast is unambiguous.

**Consequence 1 - RFC-041 moved unreachable handler arms across six stages.**
`fetch::bulk_fetch_completed`, `status::missing_projects_detected`,
`status::task_error`, and the or-pattern arm serving
`SmartPullCompleted | ContextSwitchCompleted | FreezeCompleted` are all unreachable.
Each was moved with byte-identity evidence and independently verified at review.

RFC-041 **D4 is a design decision about dead code**. It reads:

> Three domains, two lines of body. Splitting it would mean either duplicating it
> three times or inventing a home it does not belong in. It stays where the shared
> helper it calls lives.

That arm serves three variants, none of which is ever constructed.

**Consequence 2 - RFC-042 localised strings that can never render.**
`app/background/fetch.rs:110` and `:118` are inside `bulk_fetch_completed`
(`fetch.rs:104`), reachable only from `BulkFetchCompleted`. Handoff 048 §1b described
them as *"the most visible strings in this handoff - bulk fetch is knotra's core
operation and both branches are hardcoded."* **They are never displayed.** That
assertion was mine and it was wrong; bulk fetch completes through
`SingleFetchCompleted`, which has three references and is live.

**Consequence 3 - `log_to_markdown` is 78 ELOC of dead code** at
`view/history.rs:413`, suppressed at `:412`, with a careful doc comment specifying an
output format nothing produces. RFC-038 Stage 5's central task was to thread a locale
into it. That premise came from RFC-033 H4, which was true when written: the Copy
button has since been rewritten to build its own text inline.

### Why the lint could not help

`dead_code` was working correctly the entire time. It was told not to report.

## Motivation

1. **Work has been done on unreachable code, twice, by two different RFCs**, and
   neither could have known.
2. **176 findings is a survey nobody has run.** Some will be genuine leftovers; some
   may be features wired on the handler side and never triggered - which is a
   different and worse class of defect than unused code.
3. **The clippy gate is called non-negotiable** (N-9/DEC-007) and passes 176 findings
   by declaration.

## Non-goals

- **Not deleting all 176 findings.** Triage first; §D2.
- **Not changing `BackgroundMessage`'s design**, beyond removing what triage confirms
  is dead.
- **Not touching `knotra-vcs`** or `AppConfig`'s schema.
- **No `tests.rs` edits.**

## Decision

### D1. Remove every `#[allow(dead_code)]`, then triage what surfaces

The attribute goes. What the lint then reports is the work list, not the deletion
list.

### D2. Triage into three buckets, and the middle one is the point

| Bucket | Meaning | Action |
|---|---|---|
| **Dead** | Genuinely unused, no intent to use | Delete |
| **Unreached** | A handler exists, nothing triggers it - a feature wired one side only | **Report before acting.** This is a defect, not debt |
| **Deliberate** | Constructed only in tests, or reserved with a stated reason | Keep, with a **per-item** `#[expect(dead_code, reason = "...")]` |

The `BackgroundMessage` six are the reason this bucket exists. A variant with a
working handler and no producer is a feature that silently does nothing - closer to
`settings.saved_ok` rendering as its own name than to unused code.

**Anything landing in Unreached stops and is reported**, not deleted on sight.
Deleting a handler for a feature someone meant to finish is worse than the
suppression was.

### D3. `#[expect]` over `#[allow]`, per item, with a reason

Where a suppression genuinely survives triage, use `#[expect(dead_code, reason = "…")]`
rather than `#[allow]`. `expect` fails if the item stops being dead, so it cannot
outlive its justification - which is precisely how the current 39 survived.

**Never on a whole enum or struct.** Per-item only.

### D4. A guard

Once the count is zero, keep it zero. Shape is the implementer's to propose, but the
bar from RFC-042 R3 applies: **it must be seen to fail before it is trusted.**

## Requirements

| # | Requirement |
|---|---|
| R1 | No `#[allow(dead_code)]` remains in `crates/knotra-app` |
| R2 | `cargo clippy -p knotra --all-targets -- --force-warn dead_code` reports zero findings not covered by a D3 `#[expect]` |
| R3 | Every item in the Unreached bucket is reported to the architect **before** deletion |
| R4 | Surviving suppressions use `#[expect(dead_code, reason = "…")]`, per item, never on a container |
| R5 | A guard prevents reintroduction, proven to fail on a planted violation |
| R6 | `log_to_markdown` is deleted (owner direction; §Background) |
| R7 | `crates/knotra-app/src/tests.rs` is not edited |
| R8 | `knotra-vcs` and `AppConfig`'s schema are unchanged |

## Verification

The five gates, plus:

```
cargo +1.91 clippy -p knotra --all-targets -- --force-warn dead_code
```

Baseline **261 tests**. Deleting unreachable code should not change the count - if a
test fails when an item is removed, that item was not dead and the triage was wrong.
That is the most useful signal available here.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| An Unreached item is deleted rather than reported | A half-finished feature silently removed instead of finished | R3; D2 makes it a stop condition |
| A suppression hides something load-bearing in a way the tests do not cover | Removal breaks behaviour no test observes | The suite has never been edited across five RFCs, so it is an honest net - but it is not complete. Stage the removals |
| 176 findings triaged too quickly to be triaged well | The survey's value lost | Bucket counts reported before deletions begin |
| `#[expect]` used as `#[allow]` with extra words | Same problem, new spelling | R4's per-item rule and the reason string |

## Alternatives considered

**Delete `log_to_markdown` alone.** That is the immediate item and the owner's
direction covers it - but it is one of 39, found by accident while scoping something
else. The other 38 are not less real for being undiscovered.

**Keep the suppressions, add a comment to each.** Comments do not fail builds.
`#[expect]` does.

**Fix `BackgroundMessage` only.** The six variants are the sharpest instance, not the
category. `message.rs` holds 24 of the 39.

**Do nothing; the code compiles.** It does, and two completed RFCs did careful,
verified, reviewed work on parts of it that cannot run. That is the cost already
paid, before counting whatever the remaining 170 findings contain.
