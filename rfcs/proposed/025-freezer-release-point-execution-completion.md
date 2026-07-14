# RFC-025 — Freezer / Release Point Execution Completion

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High — a visible release workflow validates but cannot reach execution from the modal |
| Effort | Large |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state/freezer.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/persistence.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-vcs/src/vcs/git.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/005-annotated-tag-freezer.md`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md`, `rfcs/done/024-smart-pull-modal-execution-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the `Save release point` workflow as a production user contract. The
modal already renders input, validation, execution, and result phases, and
`knotra-vcs` already contains validation, tag/bookmark creation, rollback, and
Git tag push primitives. The active app handler does not connect the visible
primary action to execution.

This RFC wires the modal through validation, execution, rollback result
presentation, operation logging, and a post-success push offer where supported.
It also requires explicit Git and jj behavior so the UI does not promise more
than the VCS adapter can safely perform.

## Background

Earlier roadmap entries and RFCs describe Freezer as an atomic cross-repository
static-point workflow. RFC-0005 added annotated Git tag support. RFC-0013 moved
Freezer into a bulk modal. RFC-0021 renamed the first-level UI to
`Save release point` and introduced guided modal states.

Current code has useful pieces:

- `selection_bar.rs` exposes `Save release point` for selected projects.
- `bulk_modals.rs` renders `FreezerPhase::Idle`, `Validating`,
  `ValidationReady`, `Executing`, and `Done`.
- `state/freezer.rs` stores the freeze name, optional tag message, selected
  projects, and phase.
- `VcsAdapter::validate_freeze` validates Git tags and jj bookmarks.
- `VcsAdapter::execute_freeze` creates Git tags or jj bookmarks in order and
  rolls back earlier successes when a later project fails.
- `VcsAdapter::push_tag` can push Git tags.

The production gap is the visible message/task path:

- `FreezerMessage::ExecuteConfirmed` returns
  `Task::done(Message::Freezer(FreezerMessage::ExecuteRequested))`.
- `FreezerMessage::ExecuteRequested` returns
  `Task::done(Message::Freezer(FreezerMessage::ExecuteConfirmed))`.
- The modal primary button sends `ExecuteConfirmed`, so the execution backend
  is never reached from the visible modal path.
- `BackgroundMessage::FreezeExecutionDone` constructs
  `Task::done(Message::TagPush(TagPushMessage::OfferShown { ... }))`, assigns it
  to `_`, and returns `Task::none()`, so the post-success push offer is
  discarded.
- The push adapter is Git-only; jj bookmark push is not implemented by
  `push_tag`.

This makes a high-trust release workflow look complete while failing to perform
its promised operation.

## Motivation

Release points are mutating VCS actions with high user-trust impact. Users need
to know which repositories will receive a tag or bookmark, why any repository is
blocked, what happened during rollback, and whether the local release point was
pushed to its remote.

Product readiness requires every visible control to work, be disabled with a
clear reason, or be hidden. `Save release point` currently passes validation but
loops before execution. That violates the core production-readiness rule and
also hides the loss of the push offer after a successful freeze.

Operationally, partial release-point creation can leave repositories in a mixed
state. The implementation must preserve the validation boundary, avoid
overwriting existing refs, show rollback outcomes, log commands and recovery
hints, and keep unsupported push behavior honest.

## Requirements

### Functional

R1. Opening `Save release point` from the selection bar must initialize the
modal with exactly the selected projects included by default.

R2. Opening the Freezer through any existing shortcut or non-selection entry
point must initialize selection from the active workspace without stale project
entries.

R3. The release-point name field must validate before readiness checks run.
Invalid names must keep the readiness action disabled with plain-language copy.

R4. The readiness action must call `VcsAdapter::validate_freeze` and transition
through `FreezerPhase::Validating` to `ValidationReady`.

R5. Validation rows must show every relevant project, whether it is included,
ready, excluded, or blocked.

R6. The primary save action must be enabled only when every included project is
ready.

R7. The primary save action must call `VcsAdapter::execute_freeze` using the
reviewed `FreezeValidation`; it must not bounce between app messages.

R8. Execution must transition to `FreezerPhase::Executing` and keep the modal
open until a result arrives.

R9. Closing or pressing Esc while execution is running must not imply
cancellation unless real cancellation is implemented. If cancellation is not
implemented, close/Esc must be disabled or treated as no-op during execution.

R10. Completion must transition to `FreezerPhase::Done(FreezeResult)` and show
success, rolled-back, rollback-failed, and nothing-done outcomes.

R11. Result rows must show per-project saved, undone, failed, and recovery
states using first-level plain-language labels.

R12. Commands, stdout/stderr, and recovery commands must remain behind
`Show details`.

R13. Completion must persist an `OperationLog` with `OperationKind::Freeze` and
insert it into in-memory operation history.

R14. Operation history must record rollback attempted/succeeded state and
recovery hints.

R15. A fully successful Git freeze must offer to push the created tag to the
remote instead of discarding the offer task.

R16. The push offer must include only projects where the successful release
point can be pushed by the current adapter. Git tags are supported. jj bookmark
push is not supported by the existing `push_tag` adapter and must be hidden,
disabled with a reason, or split into a follow-up RFC by explicit decision.

R17. Declining a push offer must clear pending push state without changing the
saved release point.

R18. Confirming a push offer must call the push adapter once per offered
project, show pushing state, and show a completion summary.

### Non-Functional

N1. All new or changed first-level user-facing strings must be routed through
the i18n catalog with English and Japanese entries.

N2. Technical terms such as tag, bookmark, rollback, stdout, stderr, and command
may appear only in detail or recovery sections unless the target audience needs
the exact VCS term to make a safe decision.

N3. The workflow must preserve the modal close contract established by RFC-023
and RFC-024: one close action closes one topmost visible layer, except while
execution is running and no real cancellation exists.

N4. Release-point names must be passed to VCS adapters as structured arguments.
New implementation must not introduce shell interpolation.

N5. The workflow must never overwrite an existing Git tag or jj bookmark.

N6. Git and jj behavior must be explicit in code, tests, and user-facing
details.

## Goals

- Make `Save release point` a complete visible-control contract.
- Keep validation as the boundary before any mutating VCS command.
- Reuse `FreezeValidation` and `VcsAdapter::execute_freeze` where they already
  encode correct backend behavior.
- Remove the `ExecuteConfirmed` / `ExecuteRequested` loop.
- Preserve modal progress and result feedback for long-running freeze actions.
- Persist freeze history with rollback and recovery metadata.
- Surface post-success Git tag push as an actual user choice.
- State the jj bookmark push limitation plainly until a real jj push path is
  designed and implemented.

## Non-Goals

- This RFC does not redesign selection mode. Selection semantics are covered by
  a later production-readiness RFC.
- This RFC does not implement general command-palette parity. If the command
  palette advertises release-point actions, the command-palette RFC must wire,
  disable, or hide them.
- This RFC does not add signed tags.
- This RFC does not change release naming policy beyond the validation needed
  to safely call the current Git and jj adapters.
- This RFC does not implement jj bookmark push unless the implementation review
  explicitly accepts adding that adapter capability in this RFC.
- This RFC does not solve global CI or release packaging.

## External Design

### Flow

1. User selects one or more projects.
2. Selection bar shows `Save release point`.
3. User opens the modal.
4. Modal focuses the release-point name field.
5. User enters a name and optional note.
6. User runs the readiness check.
7. Modal shows per-project readiness rows.
8. If any included project is blocked, the save action is disabled with a
   plain-language reason.
9. If all included projects are ready, user clicks `Save release point`.
10. Modal shows a saving state until execution completes.
11. Modal shows result rows and summary.
12. If all successful projects are Git projects, or if the implementation can
    filter to Git projects, the modal or follow-up prompt offers `Push tag`.
13. User may decline or confirm the push.
14. User closes the result; persisted history remains available.

### Disabled and Empty States

- No selected projects: the selection bar should not offer the bulk release
  action, or it must be disabled with a clear reason.
- Empty workspace: the modal entry point must be disabled or show an empty
  state; it must not run validation over an implicit empty set without telling
  the user.
- Invalid name: readiness check is disabled and the field shows a plain reason.
- Validation has blockers: save action is disabled and each blocker is mapped
  to plain-language copy.
- Validation has no included ready projects: save action is disabled and the
  modal explains that no selected project can be saved.
- Running execution: close/Esc does not dismiss the modal unless a real
  cancellation mechanism exists.
- Git push unsupported or unavailable: do not show a working-looking push
  control. Show a disabled reason only if the user needs to understand why a
  successful local release point was not offered for push.

### Result Copy

First-level result labels should remain goal-oriented:

- `Saved` for successful project rows.
- `Undone` for projects rolled back after a later failure.
- `We could not undo everything` for rollback failure summaries.
- `Nothing was saved` for no-op results.

Details may use exact VCS terms:

- Git tag name
- jj bookmark name
- commands executed
- rollback commands
- push commands

## Internal Design

### Messages

The implementation should define one obvious review-to-execute path:

```text
ExecuteConfirmed
  -> require FreezerPhase::ValidationReady(validation)
  -> state.freezer.phase = Executing
  -> Task::perform(VcsAdapter::execute_freeze(..., validation), FreezeExecutionDone)
```

`ExecuteRequested` should either be removed, made an alias that calls the same
execution helper directly, or retained only for tests. It must not dispatch back
to `ExecuteConfirmed`.

If execution is requested without a ready validation, the handler must keep the
modal open and return to a recoverable state. It must not close silently or run
with stale validation.

### State

`FreezerPhase` should continue to represent the modal lifecycle:

- `Idle`
- `Validating`
- `ValidationReady(FreezeValidation)`
- `Executing`
- `Done(FreezeResult)`

Implementation may add an error phase or a status field if needed, but it must
not hide backend errors by returning to `Idle`.

`FreezerState::project_selection` must remain scoped to current workspace or
current bulk selection. `init_selection` already prunes stale entries; the bulk
open path must also avoid stale hidden selections.

Post-success push state currently lives in `AppState::pending_tag_push`.
Implementation may keep that shape, but the push offer must be reachable from
`FreezeExecutionDone` and visible in the active UI.

### Execution

Execution should call:

```rust
VcsAdapter::execute_freeze(&projects, &validation).await
```

The projects vector must come from the active workspace at execution time, and
the validation must be the reviewed `FreezeValidation` shown to the user.

The handler must capture enough timing metadata to write a useful operation
log. If `FreezeResult` does not include per-project exit codes or all command
outputs needed by History, the implementation should extend the model or make a
documented limitation visible in the operation log.

### Operation Logging

On completion, the app must construct and persist an `OperationLog`:

- `OperationKind::Freeze`
- a new `OperationId`
- `started_at` captured when execution begins
- `finished_at` captured when all project results arrive
- one per-project operation result for each attempted project result
- explicit failed outcome for failed execution rows
- rollback attempted/succeeded aggregate fields
- recovery hints copied from `FreezeProjectResult`

If the operation has `FreezeOutcome::NothingDone`, the log may contain zero
per-project rows, but History must still show that the user attempted the
workflow if execution was requested.

### Post-Success Push

After `FreezeOutcome::Success`, the app must compute push-eligible projects from
the successful project results and current workspace project metadata.

Current adapter behavior:

- Git: `VcsAdapter::push_tag` calls `git::push_tags`.
- jj: `VcsAdapter::push_tag` returns failure with
  `push_tag only supported for Git`.

Therefore the accepted implementation must choose one of these designs:

1. Offer push only for successful Git projects and state in details that jj
   bookmark push is not supported yet.
2. Add a real jj bookmark push adapter and include successful jj projects in
   the offer.
3. Explicitly split jj bookmark push into a follow-up RFC and keep this RFC's
   push offer Git-only.

The implementation must not offer a generic push button that knowingly sends jj
projects to a Git-only adapter and reports avoidable failures.

The existing discarded task bug must be fixed by returning or batching the push
offer task with the state update. A constructed `Task` assigned to `_` is not an
acceptable implementation.

### Git Behavior

For Git projects, validation must reject dirty/conflicted repositories and
existing tag names. Execution must create either:

- a lightweight tag when the note is empty; or
- an annotated tag when the note is present, if the adapter supports the note.

If the current `execute_freeze` path ignores `FreezerState::tag_message`, the
implementation must either wire it into Git tag creation or hide/disable the
note field until annotated execution is actually supported.

Rollback must delete created tags for earlier successful Git projects when a
later project fails. Rollback failures must surface recovery commands.

### jj Behavior

For jj projects, validation must reject dirty/conflicted repositories and
existing bookmark names. Execution must create bookmarks and rollback must
delete bookmarks.

The optional note field does not have an obvious jj bookmark equivalent in the
current adapter. The UI must not imply that jj will store the note unless the
implementation adds real support. Details may state that the note is used for
Git annotated tags only.

Bookmark push is not supported by the current `push_tag` adapter. The UI must
not present jj push as supported unless this RFC implementation adds and tests a
real jj push path.

## Security Considerations

Release-point creation mutates repositories and may publish refs when the push
offer is confirmed. The implementation must keep validation and explicit
confirmation before mutation.

Release names are user input. They must be passed as structured command
arguments through VCS adapter APIs. New code must not use `sh -c`, string-built
shell commands, or shell interpolation for release names or repository paths.

The implementation must not overwrite existing tags or bookmarks. Validation
and execution should both defend against duplicates because repository state may
change between validation and execution.

Recovery hints may contain local paths and commands. They belong behind
`Show details` and in History, not in first-level modal copy.

## Test Plan

### Unit and App Contract Tests

- Opening from selection bar initializes only selected projects.
- Opening from workspace entry initializes current workspace projects and prunes
  stale selections.
- Invalid release names keep validation disabled.
- `ValidateRequested` sets `FreezerPhase::Validating` and returns a task that
  resolves to `FreezeValidationDone`.
- `FreezeValidationDone` shows `ValidationReady`.
- Blocked validation disables execution with a plain-language reason.
- `ExecuteConfirmed` from `ValidationReady` sets `Executing` and returns a task
  that resolves to `FreezeExecutionDone`.
- `ExecuteRequested` no longer loops with `ExecuteConfirmed`.
- Close/Esc while `Executing` does not dismiss the modal unless cancellation is
  implemented.
- `FreezeExecutionDone` stores `Done(result)`, persists `OperationKind::Freeze`,
  and preserves rollback metadata.
- Successful Git freeze creates reachable pending push state or a visible push
  offer.
- The push offer excludes unsupported jj projects unless jj push support is
  implemented.
- Declining push clears pending push state.
- Confirming push sets pushing state and dispatches adapter work.

### VCS Tests

- Git validation rejects existing tag names.
- Git execution creates the requested tag.
- Git annotated-tag execution uses the optional note when present, or the note
  field is hidden/disabled if unsupported.
- Git rollback deletes tags created before a later failure.
- Git push confirmation invokes the Git push path with the requested tag name.
- jj validation rejects existing bookmark names.
- jj execution creates the requested bookmark.
- jj rollback deletes bookmarks created before a later failure.
- jj push behavior is covered by the accepted design: either unsupported and
  not offered, or implemented and tested.

### i18n and UI Coverage

- New first-level labels and disabled reasons exist in English and Japanese.
- No new production UI strings bypass the i18n catalog.
- UI contract tests prove visible button -> message -> handler -> task/result
  for validation, execution, close while running, and push offer.

### Commands

Implementation review must include current evidence from:

```sh
cargo +1.91 fmt --check
cargo +1.91 clippy --workspace --all-targets
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs
```

## Acceptance Criteria

- `Save release point` no longer has a message loop between
  `ExecuteConfirmed` and `ExecuteRequested`.
- The visible modal path reaches `VcsAdapter::execute_freeze`.
- Execution progress keeps the modal open until completion.
- Result state shows success, rollback, rollback failure, and no-op outcomes.
- Operation history records completed freeze attempts with rollback metadata and
  recovery hints.
- Fully successful Git freeze offers push through reachable UI state.
- Unsupported jj bookmark push is not presented as a working control unless a
  real jj push adapter is implemented.
- Optional note behavior is truthful for Git and jj.
- All changed first-level strings are localized.
- Tests prove visible control -> message -> handler -> task/result contracts.
- No placeholder visible controls remain in this workflow.
- Required implementation gates pass with current command output before the RFC
  is moved to `done/`.
