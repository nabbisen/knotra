# RFC-029 - Typed Context Switching and Context Switch Modal Completion

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - context switching is a visible mutating workflow and current target handling can switch the wrong kind of Git ref |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/state/context.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/state/palette.rs`, `crates/knotra-vcs/src/model/status.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-vcs/src/vcs/git.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-vcs/tests/git_integration.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md`, `rfcs/done/027-selection-mode-and-bulk-selection-completion.md`, `rfcs/done/028-command-palette-action-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the context-switch workflow as a typed, reviewable, production-ready
modal. The app currently exposes `Change work area` from selection and command
palette paths, loads a list of branches or jj contexts, and can execute a
switch. However, the modal mostly presents a free-text field, discovered
candidate metadata is flattened into a `String`, and Git switch execution
guesses "remote branch" from whether the target contains `/`.

That heuristic breaks normal local branch names such as `feature/foo`. It also
means the modal cannot faithfully explain what will happen when a user chooses
a local branch, a remote-tracking branch, a jj bookmark, or a jj change.

This RFC defines a typed context target model, a candidate-list modal contract,
Git and jj switching semantics, dirty-state safety, and tests that prove visible
controls reach the intended typed backend path.

## Background

RFC-013 introduced bulk action modals and treated context switching as one of
the guided workflows. RFC-021 established plain-language UI. RFC-027 narrowed
`Change work area` to exactly one selected project until typed context switching
is complete. RFC-028 wired the command palette action to the same one-project
context-switch path.

Current code has useful pieces:

- `ContextCandidate` stores `label`, `target`, `is_current`, and `is_remote`.
- `ContextList` stores project ID, VCS kind, candidates, and a warning.
- `ContextOpsState` stores phase, a free-text `target_context`, and cached
  context lists.
- `handle_context` opens the switch modal, loads contexts, transitions through
  confirmation, runs `VcsAdapter::switch_context`, logs the result, and refreshes
  project status.
- Git listing emits local branch candidates with `is_remote = false` and
  remote-tracking candidates with `is_remote = true`.
- jj listing emits bookmark-like branch candidates and recent commit candidates.

The production gap is the target contract:

- `ContextPhase::ConfirmSwitch` stores `target: String`, losing whether the
  target was local, remote, bookmark, or change.
- `ContextMessage::SwitchTargetChosen(ProjectId, String)` carries only a raw
  string.
- `VcsAdapter::switch_context(project, target: &str)` carries only a raw string.
- Git execution decides remote-vs-local with `target.contains('/')`.
- The existing integration test intentionally avoids slash-containing branch
  names because the implementation treats `/` as remote.
- The switch modal does not render the loaded candidate list as selectable rows;
  it primarily asks the user to type a target.

For a VCS operation that changes the user's current work area, this is not a
safe enough contract.

## Motivation

Users expect `Change work area` to be precise. A local branch named
`feature/foo` must not be treated like a remote-tracking branch just because it
contains a slash. A remote-tracking branch should be shown as a different choice
from an existing local branch, with clear language about creating or tracking a
local branch.

Product readiness requires visible controls to complete the action they
advertise, be disabled with a clear reason, or be hidden. For context switching,
that also means the modal must show the available choices, the current choice,
the confirmation consequence, and the reason switching is blocked when it is
unsafe.

Operationally, context switching mutates repository state. The app must avoid
string heuristics that can select the wrong backend behavior, and it must
preserve enough typed data for tests and operation logs to prove what happened.

## Requirements

### Functional

R1. Context targets must be represented by a typed model from discovery through
confirmation and backend execution.

R2. The typed model must distinguish at least:

- Git local branch.
- Git remote-tracking branch.
- jj bookmark.
- jj change or commit target.
- Manual target, if manual entry remains supported.

R3. `ContextCandidate` or a successor type must preserve enough metadata to
avoid inferring target kind from string shape.

R4. Git local branches containing `/`, such as `feature/foo`, must switch as
local branches.

R5. Git remote-tracking branches must switch through an explicit remote-target
path, not through `target.contains('/')`.

R6. When switching to a remote-tracking branch, the implementation must define
the local branch name deterministically. The default local name is the branch
portion after the remote name, preserving any remaining slashes.

R7. If the local branch for a remote-tracking target already exists, the
candidate must be treated as local or disabled with a clear reason. It must not
attempt to recreate an existing branch.

R8. jj bookmark and jj change targets must be distinct in the typed model even
if both execute through `jj edit <target>` initially.

R9. The switch modal must render loaded candidates as selectable rows, not only
a free-text field.

R10. Candidate rows must show the current context, VCS target kind, and disabled
state where applicable.

R11. Selecting a current context must be disabled with a plain-language reason.

R12. If listing returns no candidates, the modal must show an empty state and
must not present a misleading enabled switch action.

R13. The modal may keep manual entry as an advanced fallback, but manual entry
must have explicit semantics:

- Git manual entry means local branch name only.
- jj manual entry means a jj revset/change/bookmark expression accepted by
  `jj edit`.
- Remote-tracking Git switches must come from a remote candidate, not from slash
  inference in manual text.

R14. Empty manual entry must keep the action disabled with a localized reason.

R15. When the selected project has unsaved work or conflicts and the backend
will reject switching, the primary confirmation action must be disabled with a
plain-language reason before attempting the VCS operation.

R16. Confirmation must summarize the project, selected target label, target
kind, and the consequence in plain language.

R17. Cancel from confirmation returns to the candidate list with prior search
and typed target state preserved where practical.

R18. Switching starts a progress state that cannot be closed as if work
completed. Close/Escape during switching follows the existing modal-running
policy from Smart Pull and Freezer: keep progress visible or explicitly require
the user to wait.

R19. Completion must show success, failure, recovery hint, and details toggle
consistent with other production modal flows.

R20. Successful or failed context switches must continue to write operation
history through `OperationKind::ContextSwitch`.

R21. After completion, the app must refresh the affected project status and
invalidate stale context-list cache for that project.

R22. Selection-bar and command-palette `Change work area` actions remain
enabled only for exactly one selected project unless a later RFC designs a
multi-project context switch.

R23. If a project is missing, unsupported, or has no switchable contexts, the
visible action or modal state must explain the limitation instead of silently
closing.

### Non-Functional

N1. New user-facing strings must be routed through `knotra-ui` i18n in English
and Japanese.

N2. First-level text uses plain language: "work area", "current", "from shared
source", and "needs saved work" are preferred over raw ref jargon. Technical
ref names may appear as target identifiers where necessary.

N3. Candidate rows and confirmation controls must be keyboard reachable and have
stable focus behavior.

N4. The modal must preserve stable layout dimensions when switching between
loading, empty, browsing, confirmation, progress, and result states.

N5. Tests for Git behavior must be hermetic against global Git signing/editor
configuration.

N6. Backend APIs should avoid ambiguous stringly typed calls at the VCS boundary
for context switching.

## Goals

G1. A user can open `Change work area`, see real candidate rows, choose a
target, review the consequence, and execute the switch.

G2. Git local branch `feature/foo` switches correctly as a local branch.

G3. Git remote-tracking branch `origin/feature/foo` is represented and executed
as a remote-tracking target, creating or tracking local `feature/foo` only when
appropriate.

G4. jj bookmark and jj change targets are visibly distinct and typed even if
the first implementation uses the same `jj edit` command.

G5. Dirty or conflicted repositories are blocked before execution with a clear
reason and recovery guidance.

G6. The context-switch modal follows the same production modal standards as
Smart Pull and Freezer: validation, confirmation, progress, result, error, and
recovery states.

G7. Selection bar and command palette remain consistent entry points into the
same typed switch workflow.

## Non-Goals

- This RFC does not design multi-project context switching.
- This RFC does not add stash-before-switch or auto-save behavior.
- This RFC does not implement branch creation from arbitrary text except as
  needed for explicit remote-tracking branch checkout.
- This RFC does not complete per-project VCS history.
- This RFC does not redesign operation history beyond preserving context-switch
  logging.
- This RFC does not require full jj remote bookmark management.

## External Design

### Entry Points

The workflow is available from:

- selection bar `Change work area`, enabled for exactly one selected project;
- command palette `Change work area`, with the same availability rules;
- any existing project-detail or card shortcut that opens context switching for
  one project.

If zero or multiple projects are selected, the action is disabled with the
existing selection reason. If the selected project is unavailable, the modal
opens into an explanatory unavailable state or the entry point is disabled with
a localized reason.

### Loading State

The modal title remains `Change work area`. While loading, it shows:

- a checking status;
- the selected project name if known;
- no enabled switch button.

### Browsing State

After candidates load, the modal shows:

- search/filter input;
- optional manual target input if retained;
- a list of candidates;
- current context marked as current and disabled;
- target kind labels, for example:
  - current work area;
  - local work area;
  - from shared source;
  - jj bookmark;
  - jj change.

Candidate row text should use plain wording first and show technical target text
as secondary detail when useful.

### Confirmation State

After choosing a candidate or valid manual target, the modal shows:

- project name;
- selected target label;
- target kind;
- consequence copy:
  - local Git branch: change to existing local work area;
  - remote Git branch: create or track a local work area from the shared source;
  - jj bookmark/change: change the working copy to the selected jj target;
- disabled reason if unsaved work or conflict blocks the switch;
- `Change work area` and `Cancel`.

### Progress State

While switching, the modal shows `Changing work area...` and does not allow a
normal close that implies completion.

### Result State

On success, show:

- `Work area changed.`
- target label;
- close action.

On failure, show:

- `We could not change the work area.`
- plain-language failure hint;
- recovery hint if available;
- details toggle for commands/stdout/stderr.

## Internal Design

### Typed Target Model

Introduce a typed context target model in `knotra-vcs`, for example:

```rust
pub enum ContextTarget {
    GitLocalBranch { name: String },
    GitRemoteBranch { remote: String, branch: String, full_name: String },
    JjBookmark { name: String },
    JjChange { id: String },
    Manual { vcs_kind: VcsKind, input: String },
}
```

The exact shape may differ, but it must preserve kind and backend-relevant
fields without requiring slash heuristics.

`ContextCandidate` should either contain this target enum or be replaced by a
candidate type that does. It should still carry:

- user-visible label;
- current flag;
- disabled reason key, if known;
- optional secondary detail.

### Messages and State

Update context messages to carry typed targets:

- replace or supplement `SwitchTargetChosen(ProjectId, String)` with a typed
  target message;
- store typed target in `ContextPhase::ConfirmSwitch`;
- store typed target in `ContextPhase::Switching`;
- keep a separate string buffer for manual input and search text.

The state should not need to rediscover target kind during confirmation or
execution.

### Modal Rendering

`switch_modal` must render candidate rows from `ContextPhase::BrowsingList`.
`ContextOpsState::filtered_candidates()` can remain the filtering helper, but
the active view must use it.

Candidate row click/keyboard activation sends the typed target choice. Disabled
rows render a reason and do not execute.

### VCS Adapter

Change the adapter boundary from:

```rust
switch_context(project: &Project, target: &str)
```

to an API that accepts the typed target. The adapter may reject mismatched VCS
target kinds before calling backend-specific code.

Git backend behavior:

- `GitLocalBranch { name }` runs `git switch <name>`.
- `GitRemoteBranch { remote, branch, full_name }` runs a tracking checkout only
  when the local branch does not already exist.
- local branch existence is checked explicitly, not inferred from strings.
- failure hints use the exact command shape that was attempted.

jj backend behavior:

- `JjBookmark { name }` runs `jj edit <name>` unless a better jj API is adopted.
- `JjChange { id }` runs `jj edit <id>`.
- mismatched Git target kinds fail before invoking jj.

### Dirty and Conflict Safety

Before confirmation or before execution, detect whether switching is blocked by:

- uncommitted tracked changes;
- untracked changes if backend policy blocks them;
- active merge/conflict state.

If backend behavior remains "fail when dirty", the UI must mirror that policy
by disabling confirmation and showing a reason. The UI must not tell the user
the switch will proceed if the backend will reject it.

### Cache and History

On switch attempt:

- invalidate cached context list for the project when execution starts;
- persist `OperationKind::ContextSwitch` result as today;
- include enough result target text to explain what was attempted;
- refresh project status after completion.

If the operation result model needs typed target metadata for history, add it in
a backward-compatible way or store the user-visible target label in
`ContextSwitchResult`.

## Security Considerations

Context targets are untrusted repository-derived strings or user input. They
must never be passed through a shell. Git and jj commands must use structured
argument vectors.

Manual input must be validated or constrained before execution. For Git, manual
input must not become a remote-tracking operation by syntax accident. For jj,
manual revset input should be passed as one argument to `jj edit`, not split by
spaces or shell rules.

Failure hints may show copyable commands, but command construction for actual
execution must remain argument-vector based.

The workflow must block or clearly reject dirty/conflicted repository switches
according to backend policy to avoid data loss or hidden working-copy movement.

## Test Plan

### State and UI Contract Tests

- Opening from selection bar with one selected project enters loading state.
- Opening with zero or multiple selected projects is disabled or no-op according
  to RFC-027.
- Loaded candidate list renders selectable rows in browsing state.
- Current target row is disabled with a reason.
- Selecting a candidate transitions to confirmation with typed target preserved.
- Cancel from confirmation returns to browsing.
- Dirty/conflicted project disables confirmation with a reason.
- Confirm dispatches the typed target to the handler and enters switching state.
- Completion persists operation history and refreshes project status.
- Command palette action opens the same typed workflow.

### VCS Unit and Integration Tests

- Git local branch `feature/foo` lists as local and switches with
  `git switch feature/foo`.
- Git remote branch `origin/feature/foo` lists as remote and switches through an
  explicit remote target path.
- Remote switch preserves local branch name `feature/foo`.
- Remote switch does not attempt to recreate an existing local branch.
- Dirty Git repository blocks before switch and provides recovery guidance.
- jj bookmark candidate maps to typed `JjBookmark`.
- jj change candidate maps to typed `JjChange`.
- jj switch uses structured `jj edit <target>` execution.
- Mismatched target kind and repository kind fails safely.

### i18n Tests

- New `plain.switch.*` or `palette.*` keys exist in English and Japanese.
- First-level English copy avoids developer jargon in the plain-language layer.

### Commands

Before marking implemented, observe current-thread evidence for:

- `cargo +1.91 fmt --check`
- `cargo +1.91 test -p knotra`
- `cargo +1.91 test -p knotra-ui`
- `env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs`
- `cargo +1.91 clippy --workspace --all-targets`
- `git diff --check`

## Acceptance Criteria

- [ ] Context target kind is preserved from listing/manual selection through
      confirmation and backend execution.
- [ ] Git local branch names containing `/` switch correctly as local branches.
- [ ] Git remote-tracking branches use explicit remote target metadata.
- [ ] jj bookmark and change candidates are typed distinctly.
- [ ] The context-switch modal renders loaded candidate rows in the active UI.
- [ ] Current context rows are disabled with clear reasons.
- [ ] Empty, loading, dirty/conflicted, progress, success, failure, and recovery
      states are user-facing and localized.
- [ ] No visible context-switch control silently closes without action or
      explanation.
- [ ] Selection bar and command palette use the same one-project availability
      contract.
- [ ] Backend switch execution uses structured command arguments, not shell
      interpolation.
- [ ] Operation history still records context-switch attempts.
- [ ] Project status refreshes after completion.
- [ ] Tests prove visible control -> message -> handler -> typed task/result
      behavior.
- [ ] Required gates are run and observed in the implementation review thread.

## Open Questions

1. Should manual Git target entry remain visible by default, or should it move
   behind an advanced affordance after candidate-list rendering lands?

2. Should dirty Git repositories be strictly blocked in the UI, or should a
   later RFC add an explicit stash-before-switch option?

3. For jj, should bookmark and change targets share the same confirmation copy
   initially, or should they have distinct copy from the first implementation?
