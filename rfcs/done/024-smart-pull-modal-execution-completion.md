# RFC-024 — Smart Pull Modal Execution Completion

| Field | Value |
|---|---|
| Status | Implemented (main: 4362a2e) |
| Priority | High — a visible primary workflow can open or close without completing its promised operation |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state/sync.rs`, `crates/knotra-app/src/persistence.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/model/status.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-vcs/src/vcs/git.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete Smart Pull as a production user workflow. The selection bar exposes
`Get latest safely`, the modal describes a plan/review/progress/result flow,
and `knotra-vcs` already has Git and jj execution adapters. The active UI path,
however, does not reliably connect the visible control to plan generation,
reviewed execution, operation logging, and result persistence.

This RFC makes the Smart Pull contract explicit: a user who starts the workflow
must see a plan, must be able to adjust safe dispositions, must execute exactly
the reviewed plan, and must receive progress, result, recovery, refresh, and
history feedback.

## Background

RFC-0013 defined Smart Pull as one of the five bulk-action modals. RFC-0021
later renamed the first-level UI to `Get latest safely`, with technical terms
reserved for detail views.

Current code already contains useful pieces:

- `selection_bar.rs` shows `Get latest safely` when selected projects have an
  upstream.
- `bulk_modals.rs` can render planning, plan review, running, and result
  states from `SyncPhase`.
- `state/sync.rs` can build a `SmartPullPlan` using current workspace status,
  dirty-state checks, conflicts, user inclusion, and disposition overrides.
- `app.rs` contains a `SmartPullConfirmed(plan)` path that can stream per-project
  execution through `VcsAdapter::fetch` or `VcsAdapter::smart_pull`.
- `knotra-vcs` has Git and jj Smart Pull adapters.

The production gap is in the visible message/task contract:

- `SyncMessage::BulkPullRequested` opens the modal but does not request or build
  a plan.
- `SyncMessage::PlanRequested` opens the modal and returns `Task::none()`.
- `SyncMessage::SmartPullPlanRequested` builds a plan but no visible control
  currently reaches it.
- The modal's primary start button sends `SyncMessage::ExecuteRequested`.
- `SyncMessage::ExecuteRequested` closes the modal and returns `Task::none()`
  instead of executing the reviewed plan.
- The running Smart Pull path builds `SyncPhase::Done`, refreshes status, and
  shows modal results, but does not construct and persist an `OperationLog` for
  the completed Smart Pull.

The result is a broken primary workflow: users can click the advertised action,
but the application can stop at a passive preparing state or close the modal
without running the VCS operation.

## Motivation

Smart Pull is one of knotra's main value propositions: safely update many
repositories without forcing the user to repeat terminal commands. If the
visible action does not execute, users cannot trust other bulk workflows.

Operationally, Smart Pull is a mutating VCS action. It must show a preflight
plan, avoid pulling conflicted projects, treat dirty work conservatively, log
the exact per-project outcomes, and preserve recovery hints when manual action
is needed. A silent close or missing history entry is not acceptable for a
production workflow.

## Requirements

### Functional

R1. Clicking `Get latest safely` from the selection bar opens the Smart Pull
modal and starts plan generation immediately.

R2. The modal must leave the planning state automatically when a plan is ready;
it must not require an invisible or unreachable message.

R3. The review step must show every selected project and its disposition:
`Pull`, `FetchOnly`, `StashAndPull`, or `Excluded`, using first-level
plain-language labels.

R4. Dirty non-conflicted projects must default to `FetchOnly`.

R5. Dirty non-conflicted projects must allow the user to choose
`StashAndPull` explicitly.

R6. Conflicted projects must default to `Excluded` and must not be pulled by
Smart Pull.

R7. Selected projects with no known upstream must default to `Excluded`, must
show a plain-language reason such as `No update source is configured`, and must
not be executable by Smart Pull.

R8. Deselected projects must be excluded from execution.

R9. The primary start button must execute the exact reviewed
`SmartPullPlan`.

R10. The primary start button must be disabled with a plain-language reason when
no project is executable.

R11. During execution, the modal must stay open and show per-project progress.

R12. Completion must produce a result state with per-project succeeded, failed,
and skipped counts, commands behind `Show details`, skipped reasons, and
recovery guidance for failures that provide `RecoveryHint`.

R13. Completion must refresh workspace status after the operation finishes.

R14. Completion must persist an `OperationLog` with `OperationKind::SmartPull`
and insert it into in-memory operation history.

R15. Closing or cancelling before execution must not run a VCS operation.

R16. Closing the result modal must not discard the persisted operation history.

### Non-Functional

N1. All first-level user-facing strings added or changed by this RFC must be
routed through the i18n catalog with English and Japanese entries.

N2. Technical terms such as `pull`, `stash`, `ff-only`, branch names, stdout,
stderr, and commands may appear only in detail/recovery sections.

N3. The workflow must preserve the topmost-close behavior established by
RFC-023: one close action closes one topmost visible layer.

N4. The implementation must not shell-interpolate user-controlled repository
paths or refs. Smart Pull should continue to use structured VCS adapter calls.

N5. The implementation must not invent remote targets. It may only act on the
upstream information already known to the VCS layer.

N6. Git and jj behavior must be explicit. If jj cannot perform a Git-like merge
step, the UI and result must describe the jj behavior honestly.

## Goals

- Make the selection-bar Smart Pull path a complete visible-control contract.
- Reuse the existing `SmartPullPlan` and `SmartPullConfirmed(plan)` machinery
where it is correct.
- Remove or consolidate dead/unreachable sync messages if they obscure the
actual workflow.
- Preserve user review as the boundary before any mutating operation.
- Persist Smart Pull history in the same operation-log system used by other
background operations.
- Add tests that prove the visible action reaches plan review and that the
review primary action reaches execution instead of closing silently.

## Non-Goals

- This RFC does not redesign selection mode. Selection semantics are covered by
  a later production-readiness RFC.
- This RFC does not complete command-palette Smart Pull. The command palette
  has its own RFC. If the palette continues to advertise Smart Pull before that
  RFC, it must either dispatch to this completed path or be hidden/disabled by
  that later work.
- This RFC does not implement context switching, release-point creation, or
  conflict resolution.
- This RFC does not add new VCS capabilities beyond the current Git and jj
  Smart Pull adapter boundaries.
- This RFC does not solve global CI or release packaging.

## External Design

### Flow

1. User selects one or more projects.
2. Selection bar shows `Get latest safely` only when at least one selected
   project has an upstream; otherwise it is disabled with a reason.
3. User clicks `Get latest safely`.
4. Modal opens with a short preparing state while the plan is built.
5. Modal shows review rows:
   - clean project: `Get latest`
   - dirty project: `Check only` by default, with `Get anyway` as an explicit
     choice
   - conflicted project: `Skip`, with a note that it needs a choice first
   - project with no upstream: `Skip`, with a note that no update source is
     configured
6. User clicks the primary start button.
7. Modal shows per-project progress.
8. Modal shows result summary and rows.
9. User can open details for commands/output/recovery hints.
10. User closes the result; operation history remains available in History.

### Disabled and Empty States

- No selected projects: the selection bar should not offer Smart Pull.
- Selected projects but none with upstream: disable `Get latest safely` and
  explain that no selected project has updates configured.
- Mixed selection with some projects lacking upstream: enable the workflow if
  at least one selected project is executable, but show every selected project
  in review. Projects without upstream default to `Skip`, show `No update
  source is configured`, and do not count as executable.
- Plan has only excluded projects: keep the modal open, disable start, and
  explain that every selected project needs attention before this workflow can
  run.
- Workspace status missing or stale: build the best available plan, but show a
  plain-language note that status should be refreshed if the user is unsure.

### Keyboard and Close Behavior

- `Esc` or modal close before execution cancels the workflow and runs no VCS
  task.
- `Esc` while execution is running must not imply cancellation unless the
  implementation supports real cancellation. If cancellation is not supported,
  hide/disable close or show a plain-language "working" state until completion.
- Closing the result dismisses only the modal layer.

## Internal Design

### Messages

The implementation should define one obvious path from visible open to plan:

```text
BulkPullRequested
  -> active_modal = Pull
  -> sync.phase = Planning
  -> Task::done/perform SmartPullPlanReady(plan)
```

The implementation should define one obvious path from review to execution:

```text
ExecuteRequested
  -> read current SyncPhase::AwaitingConfirm(plan)
  -> transition to PullRunning
  -> execute SmartPullConfirmed(plan)
```

`ExecuteRequested` must not close the modal before completion. If it is kept as
the view-level message, it must validate that the current phase contains a plan.
If no plan is present, it should keep the modal open and show a recoverable
error or return to planning.

Unreachable or duplicate messages such as `PlanRequested`,
`SmartPullPlanRequested`, and `SmartPullConfirmed(plan)` may be retained if they
serve tests or task boundaries, but the final code should make the visible
workflow unambiguous.

### State

`SyncPhase` should continue to represent the modal lifecycle:

- `Idle`
- `Planning`
- `AwaitingConfirm(SmartPullPlan)`
- `PullRunning { plan, completed }`
- `Done(SyncResult)`

The implementation may add an error phase or error field if plan generation can
fail in a user-visible way.

`SyncCenterState::selected_project_ids` and `project_selection` must be aligned.
When Smart Pull is opened from the selection bar, only selected projects should
be included by default. The plan must not accidentally include every workspace
project.

Plan generation must inspect `RemoteStatus::upstream` from the current
`WorkspaceStatus`. A selected project with `upstream == None` is not a valid
Smart Pull target and must receive `SmartPullDisposition::Excluded` with a
reason distinct from conflict or manual deselection.

If the current `SmartPullPlanEntry` type cannot carry an exclusion reason, the
implementation should extend it rather than infer reasons later from UI state.
At minimum, the plan must preserve these skip causes:

- user deselected the project;
- repository has no known upstream/update source;
- repository has a conflict;
- repository is missing or status is unavailable, if detected during planning.

### Execution

Execution should reuse `VcsAdapter::smart_pull(project, stash_dirty)`:

- `Pull` -> `smart_pull(project, false)`
- `StashAndPull` -> `smart_pull(project, true)`
- `FetchOnly` -> `fetch(project)`
- `Excluded` -> no VCS command, represented as skipped/excluded in the result

Excluded projects should not count as successful VCS operations in user-facing
summary copy. They may remain in the result rows as skipped items so the user
can see why they were not touched.

Projects excluded because they lack an upstream, are conflicted, or were
deselected must run no VCS command. The result row should use a skipped outcome
with the preserved skip reason.

### Result Representation

The current `ProjectOperationResult` shape has only `success: bool`, which
cannot distinguish a skipped row from a successful or failed VCS operation.
This RFC requires an explicit outcome representation before Smart Pull history
is considered production-ready.

The preferred model is to add a serializable outcome field to
`ProjectOperationResult`, for example:

```rust
pub enum ProjectOperationOutcome {
    Succeeded,
    Failed,
    Skipped,
}

pub struct ProjectOperationResult {
    pub project_id: ProjectId,
    pub outcome: ProjectOperationOutcome,
    pub success: bool, // retained only for compatibility with existing logs
    pub skip_reason: Option<String>,
    pub commands_executed: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
}
```

Implementation may choose a different name or equivalent shape, but it must
preserve these semantics:

- executed success rows count as succeeded;
- executed failure rows count as failed;
- skipped rows count as neither succeeded nor failed;
- skipped rows preserve a user-facing reason in modal result and History;
- older logs that contain only `success: bool` remain readable, with missing
  outcome inferred from `success`.

`SyncResult` should use the same outcome semantics as persisted
`OperationLog`. It must expose separate succeeded, failed, and skipped counts
so result summaries do not inflate success or failure totals.

### Operation Logging

When all executable entries finish, the app must construct an `OperationLog`:

- `OperationKind::SmartPull`
- same `operation_id` as the reviewed `SmartPullPlan`
- `started_at` captured when execution begins
- `finished_at` captured when all project results arrive
- per-project `ProjectOperationResult` entries for every executed or skipped
  plan entry
- skipped entries marked with the explicit skipped outcome and preserved
  `skip_reason`
- `rollback_attempted = false`
- `rollback_succeeded = None`
- collected recovery hints

The log must be passed through existing `persist_log`.

### Git Behavior

Git Smart Pull should continue to fetch first. Clean projects may run
fast-forward-only merge from upstream. Dirty projects must not merge unless the
user explicitly chooses the stash-and-pull disposition. If stash pop fails, the
result must surface the recovery hint.

### jj Behavior

Current jj Smart Pull performs `jj git fetch` and reports conflicts if detected
after fetch. Because this is not identical to Git fast-forward merge behavior,
the review/result copy must not claim a Git-style merge for jj projects unless
the adapter actually performs one. If this behavior is intentionally fetch-only
for jj, that limitation should be visible in details and documented in tests.

## Security Considerations

Smart Pull mutates repositories. The implementation must preserve the review
boundary and must not run mutating commands from modal open or plan generation.

Repository paths come from workspace configuration. They must be passed to VCS
adapter functions as structured data. New implementation must not introduce
`sh -c`, string-built shell commands, or shell-interpolated paths.

The workflow must not stash or merge dirty work without explicit user consent.
Conflicted repositories must remain excluded from Smart Pull.

## Test Plan

### Unit and Handler Tests

- `BulkPullRequested` from a state with selected projects opens the Pull modal
  and transitions to `Planning` or directly to `AwaitingConfirm`.
- The generated plan includes selected projects and excludes unselected
  projects.
- Dirty projects default to `FetchOnly`.
- Conflicted projects default to `Excluded`.
- Selected projects with no upstream default to `Excluded` with a no-upstream
  reason.
- Mixed selections with one upstream project and one no-upstream project keep
  the workflow executable while showing the no-upstream project as skipped.
- All-no-upstream selections keep the selection-bar action disabled or produce
  a non-executable plan with a disabled start button and reason.
- `DispositionChanged` updates the reviewed plan or affects the next generated
  plan in a visible way.
- `ExecuteRequested` from `AwaitingConfirm(plan)` transitions to
  `PullRunning` and returns a non-empty execution task.
- `ExecuteRequested` outside `AwaitingConfirm` does not close the modal
  silently.
- Completion from all project progress events transitions to `Done`, sets
  refresh state, and persists/inserts a Smart Pull `OperationLog`.
- Skipped rows do not increase success or failure counts in `SyncResult`.
- Skipped rows are persisted in `OperationLog` with their skip reason and are
  readable through operation-history loading.
- Closing before execution runs no VCS operation.

### UI Contract Tests

- Selection bar `Get latest safely` button dispatches the Smart Pull open path.
- The visible review primary button dispatches the execution path for the
  current reviewed plan.
- The no-executable-project state renders a disabled primary button with a
  reason.
- Mixed executable/skipped review rows show both the executable action and the
  skipped no-upstream reason.
- Result rows expose command output only behind `Show details`.
- Result summaries include skipped counts separately from succeeded and failed
  counts.

### VCS Integration Tests

- Git clean repository with upstream: plan uses `Pull`, execution succeeds, and
  the operation log records executed commands.
- Git dirty repository: plan defaults to `FetchOnly`.
- Git dirty repository with explicit stash disposition: execution attempts
  stash, fast-forward merge, and pop; stash-pop failure surfaces a recovery
  hint.
- Git conflicted repository: plan excludes it.
- Git repository with no upstream: plan excludes it with `No update source is
  configured` and no VCS command is executed.
- jj repository: execution behavior matches the documented jj semantics and
  recovery hints are surfaced on fetch/conflict problems.

### i18n Tests

- New `plain.get_latest.*` and disabled-reason keys exist in English and
  Japanese catalogs.
- First-level Smart Pull copy does not introduce raw technical terms outside
  detail/recovery sections.

### Commands

Before marking this RFC implemented, run and record current output for:

```text
cargo +1.91 fmt --check
cargo +1.91 clippy --workspace --all-targets
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs
```

## Acceptance Criteria

- `Get latest safely` from the selection bar reaches visible plan review.
- The review primary action executes the exact reviewed plan.
- The modal does not silently close instead of executing.
- Dirty, conflicted, no-upstream, and deselected projects follow the
  conservative defaults described in this RFC.
- Execution progress and result states are visible.
- Smart Pull completion refreshes workspace status.
- Smart Pull completion persists and displays operation history.
- Recovery hints are preserved and displayed for failures.
- Skipped rows are represented explicitly and do not count as successful or
  failed VCS operations.
- All new first-level strings are i18n-backed.
- Tests cover visible control -> message -> handler -> task/result for the
  Smart Pull workflow.
- Current gate evidence is recorded before moving this RFC to `done/`.
