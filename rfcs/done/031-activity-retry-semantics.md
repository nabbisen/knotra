# RFC-031 - Activity Retry Semantics

| Field | Value |
|---|---|
| Status | Implemented (main: 7b1f689) |
| Priority | High - a visible failure action does not retry and currently navigates somewhere unrelated |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/state/sync.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/view/activity_strip.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/persistence.rs`, `crates/knotra-app/src/tests.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/011-activity-strip.md`, `rfcs/done/021-plain-language-layer.md`, `rfcs/done/024-smart-pull-modal-execution-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Make Activity retry a typed, deliberately limited workflow instead of a
placeholder. The current Activity strip renders `Retry` for every partial or
total failure, but `ActivityMessage::RetryRequested` only navigates to History.
The strip also stores presentation strings rather than the operation identity,
kind, failed project IDs, or parameters needed to decide whether replay is
safe.

This RFC permits direct retry only for failed Fetch projects. A failed Smart
Pull offers `Review retry`, which opens the existing Smart Pull modal with only
the failed projects selected and requires current status, a fresh plan, and
confirmation. Context switches, freezes, freeze rollbacks, and status refreshes
are not replayed from Activity. They show no enabled retry action and explain
that the original workflow must be opened again.

The implementation must derive retry eligibility from structured operation
data, never from display labels or command text. It must also connect completed
operation logs to the Activity strip and make the visible Details action open
the existing History screen, because the currently toggled activity popover is
not rendered.

## Background

RFC-011 designed an Activity strip with running, success, partial-failure, and
total-failure states. It proposed matching a human-readable operation label to
a dispatcher and retrying the failed subset. RFC-021 later established the
plain-language and i18n direction. RFC-024 completed the Smart Pull modal as a
plan, confirmation, execution, and result workflow.

The current repository has useful foundations:

- `OperationLog` records a typed `OperationKind`, an `OperationId`, and
  per-project outcomes.
- Failed project IDs can be obtained from
  `OperationResult::failed_projects()`.
- `VcsAdapter::fetch` and Smart Pull execution support both Git and jj.
- the Sync state can select a project subset and build a new Smart Pull plan;
- operation logs are persisted and loaded for History;
- the Activity view has failure states and a retry message.

The current production behavior does not satisfy those designs:

- both Activity failure states always render an English `Retry` button;
- `ActivityMessage::RetryRequested` navigates to History and does not retry;
- `LatestOpState` stores summary strings and failed names, not an operation ID,
  kind, failed project IDs, or typed retry policy;
- Activity `Started`, `Progress`, and `Completed` messages are not dispatched
  by the current operation paths, so normal completed logs do not reliably
  populate the strip;
- persisted `OperationLog` does not contain Smart Pull dispositions, context
  target metadata, freeze name, or tag annotation, so command replay cannot be
  reconstructed safely;
- the Sync modal's existing `RetryFailedRequested` always dispatches bulk
  Fetch, even when the completed result was Smart Pull;
- the visible Activity `Details` button toggles `popover_open`, but no activity
  popover is composed in the active view.

## Motivation

Retry communicates a strong promise: the app will repeat the failed part of a
known operation. Navigating to History violates that promise. Blindly repeating
commands would be worse because repository state may have changed since the
failure and some operations are not idempotent.

The safe product contract is intentionally asymmetric. Fetch can be repeated
for failed repositories using the same low-risk direct action already exposed
elsewhere. Smart Pull can alter the worktree and must be planned again. Context
switch and release-point operations need parameters and current-state checks
that the generic Activity log does not retain, so Activity must not pretend it
can replay them.

Production readiness also requires the strip to reflect real operation logs,
localize its first-level text, reject stale workspace targets, and prevent
double dispatch while another operation is active.

## Requirements

### Functional

R1. An enabled Activity retry control must perform the action named by its
label. It must not navigate to History, close without dispatch, or return
`Task::none()` for a valid current retry request.

R2. Retry eligibility must be derived from structured fields: source
`OperationId`, `OperationKind`, and failed `ProjectId` values. The
implementation must not parse localized summaries, `commands_executed`,
stdout, stderr, or human-readable operation labels to choose a dispatcher.

R3. The retry policy is:

| Operation kind | Activity action | Behavior |
|---|---|---|
| `Fetch` | `Retry failed fetches` | Directly fetch eligible failed projects only |
| `SmartPull` | `Review retry` | Open Smart Pull for eligible failed projects, refresh status, build a fresh plan, and require confirmation |
| `ContextSwitch` | unavailable | Explain that the user must open Change work area again |
| `Freeze` | unavailable | Explain that the user must open Record release point and validate again |
| `FreezeRollback` | unavailable | Show recovery/details only; never replay rollback from Activity |
| `StatusRefresh` | unavailable | Use the normal Refresh control rather than an operation replay |

R4. Fetch retry targets only outcomes whose effective outcome is `Failed`.
Successful and skipped projects from the source operation must not run again.

R5. A direct Fetch retry does not require an extra confirmation dialog because
the existing Fetch action is direct and does not update the checked-out
worktree. Its label must say that only failed fetches are being retried.

R6. Smart Pull must never replay an old `SmartPullPlan` or infer old
dispositions from command text. `Review retry` selects only failed projects,
opens the existing Smart Pull modal, refreshes their current status, and keeps
execution behind the normal fresh-plan confirmation.

R7. Smart Pull planning must remain unavailable while the retry status refresh
is running. A localized progress or disabled reason must explain this state.
If status cannot be obtained, the modal must show that project as blocked or
excluded through the existing planning contract.

R8. Unsupported operation kinds must not render an enabled `Retry` button. The
strip or Details/History view must provide a localized explanation of the safe
next action. Hiding the button without an explanation is insufficient for a
failure that otherwise appears retryable.

R9. Before dispatch, every failed project ID must be resolved against the
active workspace. Projects removed from the workspace, belonging only to a
different workspace, missing on disk, or no longer recognized as Git/jj must
not be run. Each exclusion must retain a typed reason.

R10. If some failed projects remain eligible, retry only that subset. The new
retry operation log must include every ineligible source target as a `Skipped`
outcome with a typed, localizable exclusion reason. Any rendered result modal,
Activity summary, and History details must keep attempted, failed, and skipped
counts separate. If none remain eligible, dispatch no VCS task, create no
operation log, and replace the enabled action with a localized unavailable
reason.

R11. This release uses one global handler-level operation interlock for VCS
launches. It covers standalone Fetch, bulk Fetch, Smart Pull preparation and
execution, context switch, Freezer validation/execution, conflict mutation,
tag push, Activity Fetch retry, and Activity Smart Pull retry preparation.
Every covered launch handler must consult the same interlock, including launch
paths outside Activity. This RFC does not introduce concurrent operation
scheduling.

R12. A covered operation that cannot acquire the interlock dispatches no VCS
task and shows a localized busy reason. Starting a retry must acquire the
interlock before returning its task. While retry owns it, ordinary dashboard,
modal, palette, and handler entry points for covered operations must reject new
starts. Releasing an older task must not release an interlock owned by a newer
task.

R13. Starting a retry immediately changes Activity to a running/preparing state
or opens the Smart Pull review flow. A second click cannot start a duplicate
task.

R14. The retry result is a new operation with a new `OperationId` and its own
persisted `OperationLog`. It must not alter or replace the source log.

R15. Fetch retry completion must refresh the affected project status and update
Activity from the new result. Smart Pull completion continues to use the
RFC-024 result, refresh, logging, and recovery behavior.

R16. Normal completed operation logs must update the Activity strip through one
central completion path. Persisting a log and setting the completed Activity
state must not be separately and inconsistently implemented at each call site.

R17. The central completion path must preserve existing operation-log
persistence. A persistence write failure may be reported, but it must not turn
a completed VCS result into an invisible Activity state.

R18. Retry state is session-local. Restarting knotra loads operation History but
does not restore an enabled retry button from an old log. This avoids replaying
against a workspace or repository state that may have changed while the app was
closed.

R19. Executable retry intent must not be added to persisted `OperationLog`
merely to serialize UI state. Typed exclusion reasons are audit-result data and
may be added as a backward-compatible optional field on per-project skipped
outcomes; they do not authorize replay. A future RFC may define durable
resumable operations with an explicit schema and migration.

R20. Smart Pull retry preparation must use a request/session ID and source
`WorkspaceId`. Its background completion applies only if request ID, workspace
ID, active Pull modal, and preparation state still match. Closing the modal,
switching workspace, or starting a newer preparation invalidates the request.
Late results are ignored and cannot merge status into another workspace or
advance another retry session.

R21. Smart Pull retry preparation must preserve one status result per eligible
project. Complete collection builds a fresh plan. Partial collection builds a
plan from successful status results and records failed status reads as skipped
retry exclusions. Total collection failure leaves the modal in a retryable
preparation-error state with no executable plan. Every terminal or invalidation
path releases only its matching interlock lease.

R22. The visible Activity Details action must open the existing History screen
and expose the source operation log. It must not toggle unrendered popover state.
The unused popover field/message may be removed or left inaccessible until a
separate popover design is implemented.

R23. The existing Sync-modal `RetryFailedRequested` must become kind-aware or
be replaced by typed actions. A Smart Pull result must never silently retry as
Fetch unless the visible action explicitly says `Fetch only`.

R24. When the source operation has no failed outcomes, Activity must not expose
a retry action even if the log contains skipped outcomes or recovery hints.

R25. Retry-policy derivation must use
`ProjectOperationResult::effective_outcome()` so operation logs written before
the explicit outcome field continue to classify `success == false` as failed.

### Non-Functional

N1. All Activity labels, summaries, failed-project wording, unavailable
reasons, and retry feedback are routed through the English and Japanese i18n
catalogs.

N2. First-level wording describes user intent. Use `Retry failed fetches`,
`Review retry`, `Open Change work area again`, and `Validate the release point
again`; do not expose enum variant names.

N3. Retry and Details controls are keyboard focusable and operable. Disabled
controls expose their reason in visible text or an accessible adjacent label,
not only by color or pointer hover.

N4. The strip must keep stable height and action dimensions as labels and
running states change. Long summaries must truncate or wrap without covering
buttons.

N5. Operation summaries must be derived from typed outcomes and localized at
render time. State must not make an English summary string the source of truth
for retry behavior.

N6. No retry path may execute a shell command assembled from stored command
text. All VCS execution continues through typed adapter methods and structured
process arguments.

N7. Tests must prove visible control to message to handler to task/result
behavior for Fetch retry, Smart Pull review, unavailable operations, and
Details.

### Git And jj Behavior

G1. Fetch retry uses `VcsAdapter::fetch`, preserving the current backend choice:
`git fetch` for Git and `jj git fetch` for jj.

G2. Mixed Git/jj failed subsets are allowed. Each project is resolved and
dispatched independently through its detected backend.

G3. Smart Pull retry follows current RFC-024 Git/jj planning and execution
semantics. It does not add a backend-specific replay shortcut.

G4. A repository whose VCS kind changed after the source failure is treated as
its currently detected kind for a fresh Fetch or Smart Pull plan. If it is no
longer a supported repository, it is excluded with a reason.

G5. Context-switch retry is unavailable for both Git and jj even if the source
log contains command output. Activity does not reconstruct a typed
`ContextTarget` from historical text.

G6. Freeze and rollback retry are unavailable for both Git tags and jj
bookmarks. The user must return to the release-point workflow so name
validation, existing-ref detection, cleanliness checks, and rollback safety run
again.

## Goals

- Make every visible Activity action honest.
- Retry only the failed subset of a Fetch operation.
- Route Smart Pull failures into a fresh, confirmed plan.
- Explain why non-repeatable operations cannot be retried from Activity.
- Connect completed operation logs to typed Activity state.
- Preserve each retry as a new auditable operation.
- Route Details to the working History screen.

## Non-Goals

- This RFC does not add durable retry across application restarts.
- This RFC does not implement generic command replay.
- This RFC does not retry context switches, freezes, rollbacks, conflict
  mutations, tag pushes, or changelog generation.
- This RFC does not add concurrent-operation scheduling or a task queue.
- This RFC does not redesign the full History screen.
- This RFC does not implement the activity popover proposed by RFC-011.
- This RFC does not change the underlying Git or jj Fetch/Smart Pull commands.
- This RFC does not guarantee that a repeated network operation will succeed;
  it guarantees safe targeting, dispatch, progress, result, and recovery.

## External Design

### Fetch Failure

A partial or total Fetch failure shows a specific action:

```text
Fetch: 3 succeeded, 2 failed       [Retry failed fetches] [Details]
```

Selecting `Retry failed fetches` immediately starts Fetch for the two eligible
failed projects. The strip changes to a running state, so the action cannot be
clicked twice. If one project was removed or is missing, the app retries the
remaining project and reports that one could not be retried.

### Smart Pull Failure

A Smart Pull failure does not show a direct execution action:

```text
Get latest: 3 succeeded, 2 failed             [Review retry] [Details]
```

`Review retry` opens the existing Get latest modal with only the failed
projects selected. The modal refreshes current status before enabling plan
review. The user sees current dirty/conflict/upstream decisions, reviews the new
plan, and confirms through the normal RFC-024 flow.

If every status read succeeds, the modal shows the fresh plan. If only some
status reads succeed, the modal shows a plan for the readable projects and
lists the others as excluded with reasons. If every status read fails, the
modal shows `Could not refresh project status` with retry and close actions and
does not expose confirmation.

Closing the modal or switching workspace during preparation cancels the UI
session. The underlying reads may finish, but their late result is ignored and
cannot update the current workspace or reopen the plan. Starting a newer review
also supersedes the older request.

### Non-Repeatable Failure

For a failed context switch or release-point operation, no enabled Retry button
appears. The strip provides concise next-step text, for example:

```text
Release point failed. Validate it again from Record release point. [Details]
```

The explanation may appear inline or in an accessible detail region, but it
must be discoverable without guessing why Retry disappeared.

### Busy And Stale Targets

If another covered operation is running, retry is disabled with `Wait for the
current operation to finish`. The same rule works in reverse: while Activity
retry or retry preparation runs, ordinary Fetch, Get latest, Change work area,
Record release point, conflict action, and tag-push entry points are disabled
and their handlers reject launch. If all failed projects are no longer
available in the active workspace, retry is disabled with `These projects are
no longer available in this workspace`.

### Details

`Details` opens History and makes the source operation discoverable. The first
implementation may navigate to History without auto-expanding the exact entry;
auto-expansion is preferred if it can use the source `OperationId` without a
second navigation contract.

### Keyboard And Palette

Retry and Details participate in normal tab order and activate with Enter or
Space. This RFC adds no command-palette retry command because Activity retry is
contextual to the latest session operation.

## Internal Design

### Typed Activity State

Replace presentation-only completed state with a typed source. One acceptable
shape is:

```rust
pub enum ActivityRetryAction {
    FetchFailed {
        source_operation_id: OperationId,
        project_ids: Vec<ProjectId>,
    },
    ReviewSmartPull {
        source_operation_id: OperationId,
        project_ids: Vec<ProjectId>,
    },
}

pub enum RetryAvailability {
    Available(ActivityRetryAction),
    Unavailable(RetryUnavailableReason),
    NotApplicable,
}

pub struct CompletedActivity {
    pub log: OperationLog,
    pub retry: RetryAvailability,
}
```

Equivalent types are allowed, but operation kind, operation ID, and project IDs
must remain typed. The view derives localized summaries from `OperationLog` and
must not supply executable parameters in the click message.

### Operation Interlock

Add one global interlock to `AppState`. One acceptable shape is:

```rust
pub struct OperationLease {
    pub id: OperationLeaseId,
    pub owner: OperationOwner,
}

pub enum OperationOwner {
    SingleFetch,
    BulkFetch,
    SmartPullPreparation,
    SmartPullExecution,
    ContextSwitch,
    FreezeValidation,
    FreezeExecution,
    ConflictMutation,
    TagPush,
    ActivityFetchRetry,
    ActivitySmartPullPreparation,
}
```

`try_acquire(owner)` succeeds only when no lease exists and returns a monotonic
lease ID. Every covered async completion carries that ID and calls
`release_if_matches(id)`. Closing or cancelling a preparation also releases by
matching ID. A late completion from an older task cannot clear a newer lease.

The interlock is enforced in handlers, not only by disabled buttons. All
standalone Fetch, bulk Fetch, Smart Pull preparation/execution, context switch,
Freezer validation/execution, conflict mutation, tag push, and Activity retry
launch paths use it. Validation that can fail synchronously happens before
acquisition where possible; after acquisition, every early-return, task error,
completion, cancellation, and invalidation path must release the matching
lease.

### Messages

The retry click should carry the source operation identity, for example:

```rust
ActivityMessage::RetryRequested {
    source_operation_id: OperationId,
}
ActivityMessage::DetailsRequested {
    operation_id: OperationId,
}
```

The handler re-resolves the action from current state and rejects a stale
message whose operation ID is no longer the latest completed operation. This
prevents a queued click from retrying a replacement result.

Smart Pull retry preparation uses a dedicated completion message rather than
the uncorrelated generic workspace refresh message:

```rust
BackgroundMessage::SmartPullRetryStatusReady {
    request_id: RetryPreparationId,
    workspace_id: WorkspaceId,
    lease_id: OperationLeaseId,
    statuses: Vec<ProjectStatus>,
}
```

The task returns one `ProjectStatus` for each eligible target, including a
status carrying `read_error` when inspection fails. A task-level failure uses a
matching correlated error message rather than generic `TaskError`.

### Completion Integration

Introduce one helper that:

1. attempts to persist the completed log;
2. inserts it into in-memory History;
3. derives retry availability from typed kind and effective outcomes;
4. updates the Activity completed state.

All completed operation paths that already create `OperationLog` use this
helper. Fetch paths that currently finish without creating a log must create
one so retry and History have the same source of truth.

Running updates may continue through Activity messages or a helper, but each
retry must set running state before returning its task. Completion from the new
operation replaces the running state only for that operation's lifecycle.

The helper remains narrow: persistence attempt, in-memory History insertion,
retry-policy derivation, Activity update, and persistence-error reporting.
Workflow phase transitions, refresh dispatch, and recovery remain in their
owning handlers.

### Excluded-Target Accounting

Add a typed audit reason for retry exclusions, for example:

```rust
pub enum RetryExclusionReason {
    NotInActiveWorkspace,
    ProjectPathMissing,
    UnsupportedRepository,
    StatusUnavailable,
}
```

`ProjectOperationResult` may gain a serde-defaulted optional
`retry_exclusion_reason` field while retaining the existing `skip_reason` field
for old logs and other workflow reasons. The enum supplies an i18n key for UI
rendering and a stable serialized value for History. This is result accounting,
not executable retry intent.

For a partially eligible retry, construct the new operation log from both the
attempted adapter results and synthetic `Skipped` results for every excluded
source target. Synthetic results execute no command and have empty
`commands_executed`, stdout, and stderr. Their typed exclusion reason remains
visible after Activity leaves running state and in persisted History. If no
target is eligible, no operation starts and no synthetic operation log is
created.

### Fetch Retry Dispatcher

The Activity handler validates the source ID, busy state, active workspace,
project membership, path availability, and supported repository kind. It then
calls the existing bulk Fetch dispatcher with only eligible failed IDs. The
dispatcher acquires the interlock before returning work, creates a new operation
ID/log, includes excluded source targets as skipped audit outcomes, and
refreshes affected statuses on completion. Its completion releases only the
matching lease.

### Smart Pull Review Dispatcher

The Activity handler validates the source ID and failed IDs, acquires an
interlock lease, allocates a monotonic retry-preparation ID, captures the active
`WorkspaceId`, opens `ActiveModal::Pull`, resets stale Sync overrides, and
stores eligible IDs plus typed exclusions in retry-preparation state. It then
reads current status for the eligible subset.

Completion applies only when request ID, workspace ID, lease ID, active modal,
and preparation state all match. It never passes through
`WorkspaceStatusRefreshed`, whose message lacks correlation. On complete or
partial collection, matching statuses may merge into the captured current
workspace, a fresh plan is built, exclusions are displayed, and the preparation
lease is released. On total failure, the modal enters preparation error and the
lease is released without producing a confirmable plan.

Closing the modal, switching workspace, or starting a newer preparation clears
the old preparation state and releases its matching lease. Late completion is
ignored. The new plan has a new operation ID and derives dispositions from
current status. Old `commands_executed` and old plan decisions are never replay
inputs.

When the user confirms Smart Pull, its normal execution acquires a new
interlock lease. Retry exclusions carried by Sync state are appended as skipped
outcomes to the resulting operation log, while only included plan entries reach
the VCS adapter.

### Existing Sync Retry

`SyncMessage::RetryFailedRequested` must not use one untyped branch that always
starts Fetch. It may be split into explicit `RetryFetchFailedRequested` and
`ReviewSmartPullFailedRequested` messages, or dispatch through the same typed
retry-policy helper as Activity.

### Persistence

No one-time operation-log migration is required. Retry action state is derived
for the current session when an operation completes and is not reconstructed by
`load_recent_logs`. If the optional typed exclusion field is added, serde
defaults keep existing JSON logs readable and new skipped audit outcomes remain
readable by future versions.

### Details Routing

Replace `PopoverToggled` as the visible Details route with navigation to
History, optionally setting `history_expanded` for the source operation ID.
Do not leave a visible button connected only to unused `popover_open` state.

## Security Considerations

Retry uses project IDs only to resolve current `Project` values from the active
workspace. It must not trust a stored path or execute a path from display text.

No persisted command string is executable input. Fetch and Smart Pull continue
through `VcsAdapter` and structured process arguments, with no shell
interpolation.

Smart Pull can stash, merge, and modify the worktree. For that reason Activity
only opens a fresh review flow; it never directly replays the old operation.

Freeze, rollback, and context switch can have non-idempotent or state-dependent
effects. Their retry controls remain unavailable until a future design captures
typed parameters and revalidation rules adequate for safe replay.

Operation stderr and recovery hints may contain repository paths or technical
details. Keep them in History/details rather than first-level Activity copy.

## Test Plan

### Unit And State Tests

- retry policy maps failed Fetch to `FetchFailed` with failed IDs only;
- retry policy maps failed Smart Pull to `ReviewSmartPull` with failed IDs only;
- skipped and successful outcomes never enter the retry target set;
- context switch, freeze, rollback, and status refresh map to localized
  unavailable reasons;
- an all-success log has no retry action;
- a legacy per-project result with default `Succeeded` outcome and
  `success == false` is treated as failed through `effective_outcome()`;
- a stale source operation ID is rejected;
- removed, missing, and wrong-workspace projects are excluded;
- typed retry exclusion reasons survive operation-log JSON round-trip and are
  absent/defaulted when loading old JSON;
- session startup with loaded History does not restore retry state;
- completed-log recording updates History and Activity even when persistence
  reports an error through an injected/test persistence boundary.

### UI Contract Tests

- Fetch failure renders `Retry failed fetches` and dispatches a typed Activity
  retry message;
- Smart Pull failure renders `Review retry`, not a direct generic Retry;
- freeze/context failure renders no enabled retry and shows a reason;
- busy state disables retry with a reason;
- representative covered dashboard, modal, and palette operation controls are
  disabled or show the localized busy reason while Activity retry owns the
  interlock;
- Details dispatches History navigation for the source operation;
- no Activity control dispatches only `PopoverToggled` to an unrendered layer;
- summaries and reasons come from i18n keys, not hardcoded English.

### Handler And Workflow Tests

- Fetch retry dispatches only eligible failed project IDs and immediately
  enters running state;
- a duplicate click while running dispatches no second task;
- Activity retry is rejected while each covered ordinary operation owner holds
  the interlock;
- standalone Fetch, bulk Fetch, Smart Pull preparation/execution, context
  switch, Freezer validation/execution, conflict mutation, and tag push are
  each rejected while Activity retry holds the interlock;
- a completion with an old lease ID cannot release a newer lease;
- a correlated task error releases its matching lease;
- modal cancellation, workspace-switch invalidation, and retry-preparation
  supersession each release their matching lease;
- Fetch retry completion creates a new operation ID, persists a new log, and
  refreshes project status;
- partial Fetch eligibility executes only eligible projects and records every
  excluded source target as skipped with separate attempted, failed, and
  skipped counts;
- Smart Pull review opens the Pull modal with failed projects selected;
- Smart Pull review clears stale dispositions, refreshes status, and does not
  execute before a fresh confirmation;
- complete Smart Pull status preparation creates a fresh plan only for the
  matching request and workspace;
- partial Smart Pull status preparation creates a plan for readable projects
  and retains status failures as skipped exclusions;
- total Smart Pull status failure exposes no confirmable plan;
- closing the modal, switching workspace, and superseding preparation each
  invalidate the request and ignore its late completion;
- an old preparation completion cannot merge status into a newer session or a
  different workspace;
- a Smart Pull failure cannot route through the Fetch retry branch accidentally;
- zero eligible projects produce localized feedback and no VCS task.

### VCS Integration Tests

- failed Git Fetch retry invokes Fetch only for the selected Git fixture;
- failed jj Fetch retry invokes Fetch only for the selected jj fixture where jj
  is available in the documented environment;
- mixed Git/jj failed IDs preserve one result per attempted project;
- Smart Pull retry continues to use the existing RFC-024 Git/jj integration
  coverage because Activity opens that reviewed flow instead of replaying VCS
  commands directly.

### i18n Tests

- English and Japanese catalogs contain all new Activity action, summary,
  progress, unavailable-reason, busy, and excluded-project keys;
- the production Activity view has no new hardcoded first-level English text.

### Commands

Run at least:

```sh
cargo fmt --all --check
cargo test -p knotra-vcs
cargo test -p knotra-ui
cargo test -p knotra
```

VCS integration tests must use the repository's documented hermetic Git
environment so global signing, editor, and hook configuration cannot alter
results. Record exact current command output in the implementation review
package.

## Acceptance Criteria

- The proposed RFC is reviewed and moved to `rfcs/done/` before implementation
  begins.
- Activity Retry no longer navigates to History.
- Retry policy uses typed operation kind, source operation ID, and failed
  project IDs; it does not parse labels or command output.
- Failed Fetch retries only eligible failed projects.
- Failed Smart Pull opens a fresh status, plan, and confirmation flow.
- Context switch, freeze, rollback, and status refresh expose no enabled generic
  retry and explain the safe next action.
- One handler-level interlock covers standalone Fetch, bulk Fetch, Smart Pull,
  context switch, Freezer, conflict mutation, tag push, and Activity retry in
  both launch directions.
- Retry is disabled during another covered operation, ordinary operations are
  rejected while retry runs, and stale completions cannot release newer work.
- Smart Pull preparation completion is correlated by request ID, workspace ID,
  lease ID, modal, and phase; close, workspace switch, and supersession make
  late results inert.
- Removed, missing, and wrong-workspace projects are not executed.
- Partially eligible retries retain excluded targets as typed skipped audit
  outcomes with separate attempted, failed, and skipped counts.
- A retry with zero eligible targets starts no task and creates no operation
  log.
- Every started retry operation creates a new operation log and preserves the
  source log.
- Normal completed operation logs update Activity through the central recording
  path.
- Details opens the working History screen rather than toggling an unrendered
  popover.
- Existing operation-log JSON remains backward compatible without a one-time
  migration; optional typed exclusion audit data does not persist executable
  retry intent.
- All new first-level strings are localized in English and Japanese.
- Git and jj Fetch retry use the typed VCS adapter without shell interpolation.
- Tests prove visible control to message to handler to task/result behavior.
- Current format, app, UI, and VCS gate evidence is recorded before the RFC is
  marked implemented.
