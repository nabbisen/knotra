# RFC-044 - Release Impact Warnings

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | Medium-high - the most knotra-specific capability in the codebase, built to the last step and never shown |
| Effort | Small-to-medium - the data layer exists and is tested; the work is when to compute, and where to show |
| Target | Production Readiness Reset - UI/UX foundation track |
| Related files | `crates/knotra-app/src/state/topology.rs`, `crates/knotra-app/src/app/background/status.rs`, `crates/knotra-app/src/view/overlays/freezer.rs`, `crates/knotra-app/src/view/settings.rs` |
| Related RFCs | `rfcs/accepted/043-eliminate-dead-code-suppressions.md` (this closes one of its two held items), `rfcs/done/025-freezer-release-point-execution-completion.md` (the Freezer this feeds) |

## Summary

knotra parses every registered project's `Cargo.toml`, builds a dependency graph
between **projects the user actually manages**, and turns it into:

```rust
/// A warning generated from the topology for the Freezer screen.
pub struct ImpactWarning {
    pub frozen_project_name: String,
    pub dependent_projects: Vec<String>,
    pub is_transitive: bool,
}
```

**Before you cut a release point on a library, this tells you which of your other
projects depend on it.** That is the single most knotra-specific capability in the
codebase - a thing only a multi-repository tool can offer.

It is computed, stored, and tested. **Nothing displays it.**

This RFC finishes it, and closes one of RFC-043's two held items.

## Background

### What exists, verified at `478cc3d`

| Piece | Location | State |
|---|---|---|
| `Cargo.toml` parsing, crate-name → project mapping | `knotra-vcs/src/vcs/adapter.rs:570` | Works; keeps only edges between registered projects |
| `DependencyGraph`, `direct_dependents` | `knotra-vcs/src/model/topology.rs` | Works |
| `ImpactWarning`, `description()` | same | Works |
| `TopologyState::compute_warnings` | `knotra-app/src/state/topology.rs:23` | Works, **tested** (`:74`, `:90-92`) |
| `state.topology.impact_warnings` | populated at `app/background/status.rs:43` | Populated |
| **A consumer** | — | **None** |

`TopologyPhase::Ready(DependencyGraph)`'s payload is one of RFC-043's two remaining
`#[allow(dead_code)]` entries precisely because of that last row.

### Three problems the current wiring has

**1. `compute_warnings` is called with the wrong argument.** Its doc comment says "for
a set of projects **about to be frozen**", and its parameter is `freezing: &[String]`.
`status.rs:43` passes **every project in the workspace**:

```rust
let names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();
state.topology.impact_warnings = state.topology.compute_warnings(&graph, &names);
```

So what is stored is not freeze-specific warnings. It is a general "who depends on
what" map for the whole workspace, computed at scan time.

**2. Data exists only if the user manually scanned.** The only trigger is the Settings
"Scan" button (`view/settings.rs:334`). A user who has never opened Settings has no
topology data, so a warning would silently not appear - the worst failure mode for a
safety feature, because its absence is indistinguishable from "no dependents".

**3. The data goes stale silently.** Nothing invalidates the graph when a project is
added, removed, or its manifest changes.

### Why this is worth finishing rather than deleting

RFC-043 held this item back for an owner decision, and the first recommendation was to
delete it - reasoning from "Settings renders only 'Scan complete.'" without reading
what the scan feeds. That was wrong, and the correction matters: this is not a graph
viewer awaiting a graph-rendering design. It is release-impact analysis awaiting a
list of project names in a screen that already exists.

## Motivation

1. **Freezing a library without knowing its dependents is the mistake this prevents.**
   In a single-repo tool the question does not arise; in knotra it is the point.
2. **The expensive parts are done and tested.** Manifest parsing, graph construction,
   dependent resolution, warning generation.
3. **It closes half of RFC-043's remaining distance.**

## Non-goals

- **No graph visualisation.** Rendering a dependency graph is a different feature with
  a different design problem. This RFC shows *names*, in a list.
- **No transitive analysis.** `ImpactWarning.is_transitive` exists and is always set
  `false` today. Direct dependents only; transitive is a later question.
- **No non-Cargo ecosystems.** `parse_cargo_toml` is Rust-specific. Extending it is out
  of scope.
- **No `knotra-vcs` changes.** The data layer is correct as built.
- **No `tests.rs` edits.**

## Decision

### D1. Compute at validation time, from the freeze selection

Move the `compute_warnings` call out of `topology_scanned` and into the Freezer's
validation step, passing **the projects actually being frozen**.

That is what the function's signature and doc comment already describe, and it makes
the result meaningful rather than a general map the view would have to filter. The
graph is available - `TopologyPhase::Ready(graph)` holds it.

`state.topology.impact_warnings` as a stored field may then not be needed at all;
prefer computing into the validation result over keeping a second copy of derived
state. **Propose which, with reasoning.**

### D2. Show warnings in `ValidationReady`, beside the blockers

`FreezerPhase::ValidationReady(FreezeValidation)` is where the user decides whether to
proceed, and where per-project blockers already render. An impact warning is the same
kind of information - "here is something about this project you should know before
continuing" - and belongs in the same place.

**It is a warning, not a blocker.** It must not prevent saving. `validation.all_ready()`
governs `can_save` and is not affected by this.

### D3. Absent topology data must be visible, not silent

The failure mode that matters: no scan has run, so no warnings appear, and the user
reads that as "nothing depends on this."

**When topology data is absent or stale, the Freezer must say so** rather than
rendering nothing. Wording is the implementer's to propose; the requirement is that
"we have not checked" and "we checked and found nothing" are distinguishable.

### D4. Scan automatically, and drop the Settings button

A safety feature that only works if the user first visits Settings and presses a
button is not a safety feature.

**Scan on workspace load and when the project set changes.** The scan is local
`Cargo.toml` reads with no network, bounded by project count.

That makes Settings' Scan button and its "Scan complete." status redundant - **remove
them**, along with the topology section of Settings. This also resolves the phrasing
problem where Settings reported a scan whose result was never shown.

**If automatic scanning proves too slow on large workspaces, report it** rather than
reinstating a manual button; the answer would be caching, not asking the user.

## Requirements

| # | Requirement |
|---|---|
| R1 | Impact warnings render in `FreezerPhase::ValidationReady` for the projects being frozen |
| R2 | A warning never blocks saving; `can_save` is unchanged |
| R3 | Absent or stale topology data is stated in the Freezer, distinguishable from "no dependents found" |
| R4 | Topology is scanned without user action, on workspace load and project-set change |
| R5 | Settings' topology section, its Scan button, and their catalog keys are removed |
| R6 | `TopologyPhase::Ready`'s payload has a real consumer; its `#[allow(dead_code)]` is deleted |
| R7 | `crates/knotra-vcs` is unchanged |
| R8 | `crates/knotra-app/src/tests.rs` is not edited |
| R9 | New logic carries co-located tests; `compute_warnings`' existing tests still pass unmodified |

## Verification

The five gates, gate five in the range form, plus:

```
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

That must lose the `TopologyPhase::Ready` line, leaving only `tag_exists`
(out of scope) and `OpenInMergeTool` (the other held item, until its own work lands).

Baseline **259 tests**. A rise is expected - this adds behaviour.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Automatic scanning is slow on large workspaces | Startup lag | D4; report rather than reinstating a manual trigger |
| A warning reads as a blocker | User abandons a legitimate release | D2/R2; wording and placement must distinguish them |
| Silent absence returns in a different form | The exact failure this RFC exists to prevent | R3 makes it explicit, and it is the requirement to check hardest |
| Removing Settings' topology section loses a diagnostic | A user cannot tell whether scanning works | D4 folds that into the Freezer, where it is actionable rather than informational |

## Alternatives considered

**Filter the existing whole-workspace map in the view.** Keeps `status.rs:43` as-is and
selects the relevant entries at render time. Rejected: it leaves `compute_warnings`
called with an argument its own doc comment contradicts, and keeps derived state stored
where it can go stale.

**Keep the manual Scan button and show warnings only when data exists.** Smaller change,
and it preserves the failure mode where absence reads as safety. Rejected on D3's
reasoning.

**Delete the feature** - RFC-043's original recommendation, and mine. Rejected once the
data layer was actually read: the remaining work is a list of names in an existing
screen, not the graph-rendering problem that recommendation assumed.

**Show transitive dependents too.** `is_transitive` exists and is always `false`.
Deferred - direct dependents answer the question the Freezer asks, and transitive
closure raises presentation questions this RFC does not need to settle.
