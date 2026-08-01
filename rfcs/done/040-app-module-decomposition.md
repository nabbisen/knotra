# RFC-040 - `app.rs` Module Decomposition

| Field | Value |
|---|---|
| Status | Implemented (main: 54e5d5d) |
| Priority | High - blocks RFC-035; cost grows with every RFC that lands first |
| Effort | Medium - mechanical moves, no behaviour change, guarded by an unusually strong test net |
| Target | Production Readiness Reset - operational hygiene track |
| Related files | `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/tests.rs`, `crates/knotra-app/src/lib.rs` |
| Related RFCs | `rfcs/proposed/035-dashboard-and-selection-migration.md` (lands into whatever structure this leaves), `rfcs/done/036-keyboard-navigation-and-focus-traversal.md` (owns the focus block moved here) |
| Related audit evidence | `.git-exclude/reviewed/044-preparation-review-package.md` (finding 2), `.git-exclude/reviewed/081-preparation-review-044-carry-forward-audit.md` |

## Implementation Record

| Stage | Commits |
|---|---|
| 1 | `de1cbfe` |
| 2 | `379ba44` |
| 3 | `05bc358` `f33cb34` `7f7c1ce` `6c5880b` `c5e8b36` `c66dbd0` `193b091` `12ad685` |
| 4 | `d5aca86` |
| 5 | `7e5d1b5` |
| 6 | `54e5d5d` |

Every hash verified against `main` with `git merge-base --is-ancestor <h> main`
before being written here.

**Outcome:** `app.rs` went from **3,255 to 270 ELOC** — a 92% reduction — across
eleven focused modules under `crates/knotra-app/src/app/`. Non-import ELOC moved
+10 across the whole RFC (five decomposition stages, fourteen commits), from a
3,207 pre-RFC baseline to 3,217 at Stage 5's close — effectively a proof that
nothing was rewritten, only moved. `background.rs` (761 ELOC) is the one
accepted exception to R1's 500-ELOC threshold, per R2/D2: `handle_background`'s
20-arm `match` cannot be split by arm without inventing signatures to carry
pattern-bound state across a function boundary, which is not a move. That split
is scheduled as RFC-041, immediately following this RFC's closure.

## Summary

`crates/knotra-app/src/app.rs` is **3,255 ELOC**, against the project's own
500-line strong-split threshold in
`.git-exclude/rules/project-instructions-rust-gui.md`. It is **6.5x** that
threshold and it grows by roughly 90 ELOC per RFC.

This RFC splits it along the seam it already has. `app.rs` is not a tangled
file - it is a dispatcher plus twenty independent message handlers that happen
to share one buffer. The split follows that existing shape rather than imposing
a new one.

**No behaviour changes. No public API changes. No test edits.**

## Background

### The measured problem

`044` (2026-07-18) called this "the largest maintainability risk, trending the
wrong way," at 2,436 ELOC. `081` re-measured on 2026-07-30: **3,255 ELOC, +819
(+34%) in twelve days.** The recommendation in `044` - make it an explicit track
item rather than perennial debt - was not acted on for two releases.

The mechanism is structural, not accidental: **every RFC adds message handlers
to `app.rs`**, and the reset ran nine RFCs through it. RFC-035, RFC-037,
RFC-038, and RFC-039 remain. On the observed rate, `app.rs` reaches roughly
3,600 ELOC before the UI/UX track finishes.

### Why now, specifically

Splitting *before* RFC-035 means the four remaining UI/UX RFCs land into a
structure that can absorb them. Splitting *after* means paying to untangle four
more RFCs' worth of handlers, and reviewing each of those RFCs as a diff inside
a 3,500-line file.

This RFC has a deadline attached in a way most cleanup does not.

### The fact that makes this cheap and safe

**The 3,182-line test suite barely couples to `app.rs` at all.** Measured:

```
$ grep -oE "app::[a-z_]+" crates/knotra-app/src/tests.rs | sort | uniq -c
      4 app::resolve_project_file_path
      1 app::update
```

Two symbols. Everything else drives the application the way the runtime does -
build an `AppState`, send a `Message` through `update`, assert on the resulting
state. The tests are written against the Elm architecture's public surface, not
against `app.rs`'s internals.

So a decomposition that keeps `app::update` and `app::resolve_project_file_path`
where they are is invisible to **166 tests**. That is not a hope; it is a
mechanical property of the current code, and it is what makes "all tests pass
unedited" a real acceptance gate rather than a wish.

### The shape already present

70 functions, in four clearly separable groups:

| Group | Count | Notes |
|---|---|---|
| Lifecycle | 4 | `init`, `subscription`, `update`, `view` |
| Message handlers | ~20 | `handle_*`, one per `Message` variant group, dispatched 1:1 by `update` |
| Focus model | 16 | Lines 314-528, one contiguous block, all from RFC-036 |
| Shared helpers | ~30 | Operation lease, persistence, project lookup, `start_*` orchestration |

`update` is a 72-line `match` that dispatches each `Message` variant to exactly
one `handle_*`. The variants are already the module boundaries; they simply have
no modules.

## Motivation

- The project's own rules call splitting "strongly recommended" above 500 ELOC.
  `app.rs` is 6.5x that and the rule has been silently suspended for it.
- Every RFC's diff is harder to review inside a 3,255-line file, and reviews
  `065` and `067` both missed defects that a smaller review surface would have
  made obvious.
- Four RFCs are queued behind this, each of which will add handlers.
- `#[allow(unused_imports)]` currently sits on the crate-level import block -
  a suppression that exists because the file is too large to know what it uses.

## Non-goals

- **No behaviour change of any kind.** Not a refactor of logic, only of
  location. If a move requires changing what code does, it is out of scope and
  the RFC needs amending.
- **No `tests.rs` split.** It is 3,182 ELOC and equally over threshold, but
  splitting it is independent work with a different risk profile, and doing both
  at once destroys the test-net property this RFC depends on.
- **No changes to the focus model's behaviour.** RFC-036 closed it. This moves
  the block; it does not touch what it does.
- **No API surface changes for `knotra-ui` or `knotra-vcs`.**

## Decision

### D1. Split by message domain, because that seam already exists

Create `crates/knotra-app/src/app/` with one submodule per message domain,
mirroring the `Message` enum:

```
app.rs                  (parent)           lifecycle + update dispatch + view
app/focus_ops.rs                           the RFC-036 focus block (lines 314-528)
app/shared.rs                              lease, persistence, lookup helpers
app/workspace.rs                           handle_workspace          (426 lines)
app/background.rs                          handle_background         (679 lines)
app/sync.rs                                handle_sync, start_bulk_fetch
app/context.rs                             handle_context
app/conflict_ops.rs                        handle_conflict_ops
app/freezer.rs                             handle_freezer, start_freeze_execution
app/changelog.rs                           handle_changelog
app/activity.rs                            handle_activity, start_activity_*
app/misc.rs                                the short handlers: project, settings,
                                           history, palette, selection, dashboard,
                                           launch, topology, tag_push, shortcut
```

**Rationale.** The alternative - splitting by layer (state mutation vs. task
spawning vs. persistence) - would cut across every handler and require
understanding each one to place its lines. Splitting by domain moves whole
functions without reading their bodies, which is what makes this reviewable and
low-risk.

The short handlers are grouped rather than given a file each; twelve 20-40 line
files would trade one problem for another.

### D2. `handle_background` is split further, or explicitly deferred

At **679 lines** it is above threshold on its own, and it is the largest single
function in the crate. It should be split by the background event it handles.

**But if that split requires understanding its logic rather than moving whole
match arms, defer it and say so.** A 679-line file that is honestly one domain
is a better outcome than a rushed split of the most concurrency-sensitive code
in the application. This is the one place in the RFC where stopping is the
correct result.

### D3. Visibility is widened only as far as a move requires

Helpers used by exactly one handler move with it and stay private. Helpers used
by several become `pub(crate)` in `app/shared.rs` - not `pub`.

`app::update` and `app::resolve_project_file_path` keep their current paths and
visibility, because `tests.rs` names them. Any other symbol becoming visible is
a signal the split is in the wrong place.

### D4. `#[allow(unused_imports)]` is removed

It sits on the crate import block today. After the split each module declares
what it uses, so the suppression should not be reinstated. If removing it
produces warnings, those are real and were being hidden.

### D6. Unqualified imports during the split, qualified once at the end

Added 2026-07-31 from `.git-exclude/reviewed/086-rfc-040-stage-1-review.md`,
review focus 1.

Each stage brings its module's cross-boundary functions into `app.rs` with
`use self::<module>::{…}`, leaving existing call sites untouched. That is what
keeps a stage a *provable* move: the diff stays near-symmetric (R10) and a
reviewer does not have to confirm that dozens of mechanical call-site edits
changed nothing. Stage 1 had 11 functions across 30 call sites; qualifying them
would have tripled its diff for no behavioural gain.

**This does not scale to the finished structure.** With roughly ten submodules,
`app.rs` would import ~50 unqualified names and a reader seeing `advance_focus(…)`
could not tell which module owns it - trading a size problem for a provenance
problem.

So it is resolved once, at the end, rather than drifting stage by stage: **Stage
6** qualifies cross-module calls after the module set is complete and stable. A
single mechanical commit, tests unedited, no behaviour change - and by then the
final module boundaries are known, so the churn is paid once rather than being
rewritten as later stages move functions around.

### D7. Dependency direction, not a blanket ban on handler-to-handler calls

Added 2026-07-31 from
`.git-exclude/reviewed/089-rfc-040-stage-3-commits-0-6-and-handler-coupling-ruling.md`.

This RFC's risk table originally justified "handlers must not import each other"
with "does not compile." **That premise is false** - Rust modules within a crate
may reference each other cyclically. The prohibition was written against a
failure mode that does not exist as stated, and it blocked correct work at Stage
3 commit 7, where `handle_shortcut` legitimately calls `handle_context` and
`handle_freezer`.

The real concern is dependency *direction*, not adjacency:

- **`shared.rs` must never depend on a handler module.** This inversion is the
  genuine hazard: `shared.rs` exists to be depended upon.
- **`focus_ops.rs` must not depend on a handler module**, for the same reason one
  level out. Handler → `focus_ops` → `shared` is a clean layering and is allowed.
- **A handler module may call another handler** where the domain genuinely
  requires it, provided the dependency graph stays **acyclic** and the call is
  documented at the import site.

Promoting a domain entry point into `shared.rs` to satisfy the old rule is
**worse** than the import it avoids: `shared.rs` holds helpers with no single
domain owner, and a `handle_*` is by definition its domain's entry point.

**Routers are not handlers.** `handle_shortcut` delegates in every arm and owns
no domain state - it is a routing table from keyboard shortcut to domain action,
structurally what `update` is for messages. It stays in the parent alongside
`update`, which already imports every handler module because dispatch requires
it. Placement follows role, not size.

### D5. The split lands in stages, each independently green

Not one commit. Ordered by increasing risk:

| Stage | Content | Rationale |
|---|---|---|
| 1 | `app/focus_ops.rs` | Contiguous, self-contained, recently reviewed - proves the mechanism |
| 2 | `app/shared.rs` | Establishes what "shared" means before handlers move onto it |
| 3 | `misc.rs`, `changelog`, `freezer`, `activity`, `context`, `conflict_ops`, `sync` | The bulk; each is a whole-function move |
| 4 | `workspace.rs` | 426 lines, more state interaction |
| 5 | `background.rs` (+ D2) | Largest and most concurrency-sensitive; done when the pattern is established |
| 6 | Qualify cross-module calls (D6) | Mechanical; deferred until the module set is final so the churn is paid once |

Every stage: all 166 + 10 + 42 tests pass **unedited**, all five gates green.

## Requirements

| # | Requirement |
|---|---|
| R1 | Submodules live in `crates/knotra-app/src/app/`; no file there exceeds 500 ELOC, except as permitted by R2. The parent may stay `app.rs` alongside the directory (Rust 2018+ style, smaller diff) or become `app/mod.rs` — either satisfies this |
| R1a | The new focus submodule must **not** be named `focus`. `app.rs` already has `use crate::state::{… focus …}`, so `mod focus;` in the same scope is a "name defined multiple times" error. Rename the new module, not the long-established `state::focus` import that other modules also use |
| R2 | `handle_background` is split by event, **or** the review request states why it could not be split without changing behaviour |
| R3 | `app::update` and `app::resolve_project_file_path` keep their current paths and visibility |
| R4 | `crates/knotra-app/src/tests.rs` is **not edited** - not one line, including imports |
| R5 | No function body changes. Moves, plus `use` statements and visibility only |
| R6 | Symbols become `pub(crate)` only where a move requires it; nothing becomes `pub` |
| R7 | `#[allow(unused_imports)]` is removed and not reinstated |
| R8 | `close_topmost_layer`, `advance_focus`, `activate_focused`, and `overlay_focus_order` move **byte-identical** - RFC-036 R5/R7 depend on them |
| R9 | Each stage is independently committable with all five gates green |
| R10 | **Non-import** ELOC across `app.rs` + `app/` stays within +5% of 3,255. Import growth is excluded: measured at Stage 3, **+125 of the +140 total was `use` statements**, ten modules each declaring what they use. Crossing 5% on import growth alone is **not** a breach. (The original wording said a larger increase "means logic was rewritten"; that was false, and the real rewriting-detectors are R10a and the byte-identity check.) |
| R10a | Per-commit `git diff --stat` is near-symmetric — insertions ≈ deletions. This, with per-function byte-identity, is what actually detects rewriting; ELOC only ever proxied for it |

## Verification

The test suite is the verification, and it is unusually strong here precisely
because it does not know `app.rs`'s internals exist.

- **166 `knotra` tests unedited and passing** at every stage. R4 makes this
  meaningful: a passing suite that had to be edited proves nothing.
- `git diff --stat` per stage should show near-symmetric insertions and
  deletions. A stage that adds substantially more than it removes has rewritten
  something.
- For R8, extract each named function at the stage's parent and at the stage
  commit and `diff` them - byte-identical, the same check `084` used for
  `guided_button`.
- ELOC per file recorded in the final review request, so R1 is measured rather
  than asserted.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| A move quietly changes behaviour | Regression in a shipped path | R5 forbids body edits; R10 catches rewriting by size; 166 tests unedited |
| Inverted dependency — `shared.rs` or `focus_ops.rs` importing a handler | The layering the split exists to create is lost; `shared.rs` stops being safe to depend on | `shared.rs` lands before the handlers (D5 stage 2); **D7** states the direction rule. Note the original entry here claimed circular imports "does not compile" — that was **false**, and D7 replaces it |
| `handle_background` split changes concurrency behaviour | Hard-to-reproduce defects in the most sensitive code | D2 permits deferral; it is scheduled last |
| Visibility widens to make it compile | Encapsulation quietly lost | R6; anything becoming `pub` fails review |
| The split stalls half-done | Two structures instead of one | D5 stages are each independently green and shippable |

## Alternatives considered

**Do nothing, split later.** Rejected: `081` measured +34% in twelve days, and
four RFCs are queued. "Later" has been the answer since 0.22.0 and the file has
only grown.

**Split by layer rather than domain.** Rejected: cuts across every handler,
requires reading each body to place its lines, and destroys the whole-function
property that makes this reviewable.

**Split `tests.rs` at the same time.** Rejected: `tests.rs` is the safety net
for this change. Moving both at once means neither guards the other.

**Move handlers into `state/`.** Rejected: `state` holds data; handlers spawn
`Task`s and orchestrate the VCS layer. Merging them would confuse a boundary
that currently works.
