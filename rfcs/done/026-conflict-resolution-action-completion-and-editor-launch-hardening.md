# RFC-026 — Conflict Resolution Action Completion and Editor-Launch Hardening

| Field | Value |
|---|---|
| Status | Implemented (main: 1cde97d) |
| Priority | High — visible conflict-resolution controls can be no-ops and one editor path uses shell interpolation |
| Effort | Large |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state/conflict_ops.rs`, `crates/knotra-app/src/config.rs`, `crates/knotra-vcs/src/model/conflict.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-vcs/src/vcs/git.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the visible conflict-resolution panel contract and harden external
editor launch. The current panel renders `Open in editor`, `Mark done`, and
`Stop this fix attempt`, but the visible buttons do not consistently reach the
backend-backed message paths. The direct editor path also builds a shell command
with user-controlled text and launches it through `sh -c`.

This RFC requires every visible conflict action to either run the supported
backend operation, be disabled with a clear reason, or be hidden. It also
requires editor and merge-tool launch to use structured process arguments, not
shell interpolation.

## Background

RFC-0013 moved conflict resolution into a docked panel. RFC-0021 then made the
first-level UI plain-language and added `Open in editor` as a guided action.

Current code has partial backend support:

- `ConflictOpsMessage::MarkResolvedRequested { project_id, file_path }` exists.
- `ConflictOpsMessage::AbortMergeRequested(project_id)` exists.
- `VcsAdapter::mark_resolved` calls Git `git add <file>`.
- `VcsAdapter::abort_merge` calls Git `git merge --abort`.
- `VcsAdapter::list_conflicted_files` supports Git and jj conflict listing.
- `handle_launch` already has a structured `Command::new(tool).arg(file_path)`
  path for `LaunchMessage`.

The visible panel does not use those safer paths:

- The `Mark done` button sends `ConflictOpsMessage::FileMarkedResolved(path)`.
- `FileMarkedResolved` is handled as `Task::none()`.
- The `Stop this fix attempt` button sends `ConflictOpsMessage::AbortRequested`.
- `AbortRequested` is handled as `Task::none()`.
- `OpenInEditorRequested(path)` bypasses `handle_launch`, builds
  `format!("{} {}", editor, path)`, and executes `sh -c`.

The result is a high-risk UI contract: the user can click actions that look
like they resolve or stop a conflict workflow, but they either do nothing or
launch a shell with interpolated input.

## Motivation

Conflict resolution is already a stressful workflow. Users need knotra to be
predictable: open the exact file, mark only the chosen file as resolved, stop
only supported operations, and refresh conflict state after every action.

Product readiness requires visible controls to work, be disabled with a clear
reason, or be hidden. The current no-op aliases break that rule.

Security-wise, repository paths and conflicted file paths can contain spaces,
quotes, shell metacharacters, or unexpected text. They must never be composed
into `sh -c` strings for editor or merge-tool launch.

## Requirements

### Functional

R1. `Open in editor` must launch the configured editor through a structured
process invocation.

R2. `Open in editor` must pass the selected conflicted file path as an argument,
not through shell interpolation.

R3. If no editor is configured, `Open in editor` must be disabled with a
plain-language reason or hidden.

R4. If the selected file path cannot be resolved safely within the project
root, launch must be refused with a user-visible reason.

R5. `Mark done` must dispatch a message that includes both project ID and file
path.

R6. `Mark done` must call `VcsAdapter::mark_resolved` where supported.

R7. After a mark-resolved attempt, the app must reload conflict detail for that
project and show the updated file list.

R8. Failed mark-resolved attempts must keep the panel open and show a
plain-language failure state with technical details available behind
`Show details`.

R9. `Stop this fix attempt` must dispatch a project-scoped abort request.

R10. Abort must call `VcsAdapter::abort_merge` only where supported.

R11. After an abort attempt, the app must reload conflict detail for that
project and show the updated state.

R12. Unsupported actions must not appear as working buttons. For example, jj
mark-resolved or abort support must be explicit before the UI offers it.

R13. The panel must show loading, operating, done, empty, unsupported, and error
states without closing silently.

R14. Closing the panel during a running operation must not imply cancellation
unless true cancellation is implemented. If cancellation is not implemented,
close must be disabled or treated as no-op while operating.

### Non-Functional

N1. New first-level user-facing strings must be routed through the i18n catalog
with English and Japanese entries.

N2. First-level copy must remain plain-language. Technical commands such as
`git add`, `git merge --abort`, stdout, stderr, and adapter error text belong
behind details.

N3. The implementation must not introduce `sh -c`, shell string construction,
or platform shell fallback for editor/merge-tool launch.

N4. Repository paths and conflict file paths must be treated as structured
paths. Relative conflict paths must be joined against the project root and
checked before launch.

N5. Git and jj behavior must be explicit in the UI and tests.

## Goals

- Wire visible `Mark done` to backend-backed mark-resolved behavior.
- Wire visible `Stop this fix attempt` to backend-backed abort behavior where
  supported.
- Replace the direct conflict editor launch path with the existing structured
  launch pattern or an equivalent safe launcher.
- Reload conflict state after mark-resolved and abort operations.
- Surface unsupported VCS behavior as disabled or hidden controls, not failed
  surprise operations.
- Add tests proving visible control -> message -> handler -> task/result for
  conflict actions.

## Non-Goals

- This RFC does not implement a full merge editor inside knotra.
- This RFC does not add real jj mark-resolved or abort behavior unless the
  implementation explicitly adds and tests adapter support.
- This RFC does not redesign the entire project detail panel.
- This RFC does not complete command-palette conflict actions. Command-palette
  parity is covered by a later RFC.
- This RFC does not implement retry semantics for failed conflict actions.

## External Design

### Flow

1. User opens a project with conflicts.
2. Right-side conflict panel loads the conflicted file list.
3. For each conflicted file, the panel shows the file path and supported
   actions.
4. User clicks `Open in editor`.
5. If an editor is configured and the file path is safe, the editor launches.
   Otherwise the panel or status area shows a clear reason.
6. User resolves a file externally.
7. User clicks `Mark done`.
8. Panel enters an operating state for that file.
9. Backend marks the file resolved where supported.
10. Panel reloads conflict detail and updates the list.
11. User may click `Stop this fix attempt` when a supported abort operation is
    available.
12. Abort enters an operating state, calls the backend, reloads conflict state,
    and shows the result.

### Disabled and Empty States

- No configured editor: `Open in editor` is disabled with a reason.
- Unsupported VCS action: `Mark done` or `Stop this fix attempt` is hidden or
  disabled with a reason.
- No conflicted files after reload: show a resolved/empty state and allow close.
- Project not found: show an error state; do not attempt editor launch or VCS
  commands.
- Running operation: keep progress visible; close does not imply cancellation.

### Git Behavior

For Git projects:

- `Mark done` should run the equivalent of `git add <path>` through
  `VcsAdapter::mark_resolved`.
- `Stop this fix attempt` should run the equivalent of `git merge --abort`
  through `VcsAdapter::abort_merge`.
- If a Git repository is in a rebase/cherry-pick state where `merge --abort`
  is not the right command, the UI must not overclaim. Either support the
  correct operation explicitly or present a clear unsupported reason.

### jj Behavior

For jj projects:

- Conflict listing may remain supported.
- `Mark done` and abort controls must be disabled or hidden unless real jj
  adapter support exists.
- The panel may explain in details that this action is currently available for
  Git projects only.

## Internal Design

### Messages

The visible file-row action should dispatch the project-scoped message:

```text
Mark done
  -> ConflictOpsMessage::MarkResolvedRequested { project_id, file_path }
```

The visible abort action should dispatch:

```text
Stop this fix attempt
  -> ConflictOpsMessage::AbortMergeRequested(project_id)
```

The legacy aliases `FileMarkedResolved(String)` and `AbortRequested` should be
removed, hidden from the view, or retained only as compatibility shims that
route to the real project-scoped messages when enough context exists. They must
not remain visible no-op paths.

Editor launch should dispatch either:

```text
Open in editor
  -> Message::Launch(LaunchMessage::OpenInEditor(resolved_file_path))
```

or an equivalent conflict-scoped message that reaches the same structured launch
helper. It must not call `Command::new("sh").args(["-c", ...])`.

### State

`ConflictPhase` already has useful states:

- `Idle`
- `Loading(ProjectId)`
- `Browsing { project_id, detail }`
- `Operating { project_id, action }`
- `Done { project_id, success, message }`

Implementation may extend `Operating` or `Done` to carry operation kind, file
path, `ProjectOperationResult`, and recovery/detail text. If it keeps the
existing shape, the view must still show enough information for failures and
reloads.

The cached conflict detail must be invalidated before mutating operations and
reloaded after the operation finishes.

### Path Handling

Conflict file paths should be treated as repository-relative paths unless the
backend explicitly returns absolute paths. Before launching an editor, the app
should:

- find the owning project;
- join the conflict path to the project root when it is relative;
- normalize enough to reject paths that escape the project root;
- pass the final path as a single structured argument to the configured tool.

The implementation should prefer standard path APIs. It must not use ad hoc
quoting or shell escaping as the safety boundary.

### Operation Results

Mark-resolved and abort operations should preserve:

- commands executed;
- success/failure;
- stdout/stderr;
- error message;
- recovery hints if available later.

The user-facing panel should show plain-language status first and technical
details only when requested.

## Security Considerations

This RFC directly addresses command injection risk. The implementation must
remove the conflict-panel `sh -c` path and avoid replacement shell fallbacks.

Repository paths and file paths are not secrets, but they can contain
attacker-controlled characters in shared workspaces. Treat them as structured
arguments.

Abort and mark-resolved mutate repository state. The UI must keep the project
scope visible and avoid running these operations without explicit user action.

## Test Plan

### App Contract Tests

- `Mark done` file-row message carries the active project ID and file path.
- `FileMarkedResolved` is no longer a visible no-op path, or it routes to the
  real message when context exists.
- `MarkResolvedRequested` sets `ConflictPhase::Operating` and returns a backend
  task for supported Git projects.
- Mark-resolved completion reloads conflict detail.
- Failed mark-resolved keeps the panel open and surfaces failure.
- `Stop this fix attempt` dispatches a project-scoped abort request.
- `AbortMergeRequested` sets `ConflictPhase::Operating` and returns a backend
  task for supported Git projects.
- Abort completion reloads conflict detail.
- Running conflict operation close/Esc does not dismiss the panel unless real
  cancellation exists.
- `Open in editor` dispatches structured launch and never calls the shell path.
- No-editor state disables or hides editor launch with an i18n reason.

### VCS Tests

- Git `mark_resolved` stages the selected file.
- Git `abort_merge` aborts an active merge and clears conflict state.
- Unsupported jj mark-resolved and abort behavior is either not offered in app
  tests or covered by adapter tests with explicit failure/unsupported results.

### Security Tests

- A conflict file path containing spaces is passed as one argument.
- A conflict file path containing shell metacharacters is not interpreted by a
  shell.
- A path that escapes the project root is rejected before launch.

### i18n and Gate Commands

- New first-level conflict strings exist in English and Japanese.
- Existing first-level jargon guard passes.
- Implementation review must include current evidence from:

```sh
cargo +1.91 fmt --check
cargo +1.91 clippy --workspace --all-targets
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs
```

## Acceptance Criteria

- Visible `Mark done` no longer dispatches a no-op message.
- Visible `Stop this fix attempt` no longer dispatches a no-op message.
- Editor launch from the conflict panel no longer uses `sh -c`.
- Conflict actions are project-scoped and file-scoped where needed.
- Unsupported jj actions are not presented as working controls.
- Conflict detail reloads after mark-resolved and abort attempts.
- Running operations keep progress visible and are not silently dismissed.
- New user-facing strings are localized.
- Tests prove visible control -> message -> handler -> task/result contracts.
- Required implementation gates pass with current command output before the RFC
  is moved to `done/`.
