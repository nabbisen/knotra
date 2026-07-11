# RFC-023 — Workspace Management Completion

| Field | Value |
|---|---|
| Status | Implemented (main: 02e1481) |
| Priority | High — visible workspace controls currently do not complete their user contract |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/workspace_tabs.rs`, `crates/knotra-app/src/view.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state/workspace_mgr.rs`, `crates/knotra-app/src/state/palette.rs`, `crates/knotra-app/src/persistence.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/015-workspace-tabs.md`, `rfcs/done/021-plain-language-layer.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md`, `.git-exclude/reviewed/011-rfc-0023-workspace-management-review.md` |

## Summary

Complete workspace management as a production user workflow. The top tab strip
already exposes `+ New workspace`, and the application already has partial
state and handlers for create, rename, and delete, but the active view stack
does not render the workspace-management dialogs. This RFC makes workspace
create, rename, delete, validation, persistence, active-workspace switching,
keyboard close behavior, and tests explicit.

The production rule for this RFC is simple: every visible workspace control
must either complete the action it advertises, be disabled with a plain-language
reason, or be hidden until supported.

## Background

RFC-0015 introduced workspace tabs and required a `[+]` control that opens a
create-workspace dialog. Later redesign work moved the application to a
top-level tab strip rendered by `crates/knotra-app/src/view/workspace_tabs.rs`.

Current code does part of the work:

- `workspace_tabs.rs` renders `+ New workspace` and dispatches
  `WorkspaceMessage::CreateWorkspaceDialogOpened`.
- `app.rs` handles create, rename, delete, and switch messages.
- `state/workspace_mgr.rs` stores `create_dialog`, `rename_dialog`, and
  `confirm_delete`.
- `state/palette.rs` advertises `Create new workspace` but currently falls
  through to no message.
- `persistence.rs` can save and load workspace files.

The production gap is that `view.rs` only layers the add-project modal,
command palette, and shortcuts overlay above the snora layout. No view renders
the workspace create, rename, or delete dialog state. A user can click
`+ New workspace`, mutate hidden state, and see no UI.

The roadmap's Production Readiness Reset records this as the first RFC drafting
item because it is a reported, visible user failure.

## Motivation

Workspace management is a first-run and daily-use workflow. If the tab strip
advertises adding a workspace but nothing appears, users cannot trust the rest
of the dashboard. This is not a cosmetic issue: it breaks the product's promise
that visible controls are intentional and complete.

Completing this workflow also gives the production-readiness series a concrete
pattern for UI contract tests: visible control -> message -> rendered state ->
confirmed task/state transition -> persisted result.

Operationally, workspace management touches local config files, active refresh
state, filesystem-watch pruning, and startup persistence. It must be reliable
before later RFCs add or repair more cross-workspace behavior.

## Requirements

### Functional

R1. Clicking `+ New workspace` opens a visible create-workspace dialog.

R2. The create dialog accepts a workspace name, validates it, and creates a new
empty workspace.

R3. A newly created workspace is persisted to
`~/.config/knotra/workspaces/<uuid>.toml`, appended to the tab list, made
active, and refreshed.

R4. Create validation rejects an empty or whitespace-only name.

R5. Create validation rejects a duplicate name among loaded workspaces, using a
case-insensitive comparison after trimming surrounding whitespace.

R6. Rename is reachable from a visible control and opens a visible rename
dialog for the active workspace.

R7. Rename validates the same name rules as create, persists the workspace, and
updates both `state.workspace` and `state.all_workspaces`.

R8. Delete is reachable from a visible control and opens a visible confirmation
dialog for the active workspace.

R9. Delete removes the active workspace file, removes the tab, switches to the
nearest remaining workspace, clears stale active workspace status, prunes
filesystem watcher snapshots to the new active workspace, and refreshes.

R10. The last remaining workspace cannot be deleted. The delete control is
disabled with a plain-language reason or hidden when only one workspace exists.

R11. Cancel and close actions leave workspaces, persistence, and active
workspace unchanged.

R12. Workspace switching by clicking an existing tab continues to work.

R13. If persistence fails during create, rename, or delete, the UI reports a
plain-language error and must not silently claim success.

R14. The command palette's existing `Create new workspace` action must either
dispatch to the same create-workspace dialog as the tab-strip button or be
hidden until the command-palette completion RFC. It must not remain a visible
action that closes silently.

### Non-Functional

N1. All production user-facing strings introduced by this RFC are routed
through `crates/knotra-ui/src/i18n.rs` in English and Japanese.

N2. The first-level wording uses plain language. Technical terms such as
configuration file, TOML, UUID, or filesystem watcher stay out of primary
dialog copy.

N3. Dialog controls are keyboard reachable. Opening a create or rename dialog
focuses the name input. `Esc` closes the active workspace dialog without
mutation.

N4. Destructive delete uses a safe-first layout: Cancel remains available and
the destructive action is visually and textually explicit.

N5. The implementation keeps the existing local-first model. There is no
network behavior, VCS behavior, Git behavior, or jj behavior in this RFC.

N6. The implementation does not add a second workspace persistence mechanism.
Workspace files remain the source for restart survival.

## Goals

- The tab strip's `+ New workspace` button reliably opens visible UI.
- Create, rename, and delete workflows have clear validation, cancellation,
  error, and success behavior.
- Workspace persistence remains plain-file based and restart-safe.
- Active workspace state remains internally consistent:
  `all_workspaces[active_workspace_idx]`, `workspace`, `workspace_status`, and
  refresh state agree after every operation.
- Existing add-project, command-palette, shortcut, and snora modal layers keep
  working after workspace dialogs are added.
- The existing command-palette `Create new workspace` action no longer silently
  does nothing.
- Tests prove the main visible workspace controls reach intended state changes
  and persistence behavior.

## Non-Goals

- This RFC does not implement Smart Pull, Freezer, conflict resolution, command
  palette parity, or selection-mode repairs.
- This RFC does not implement inactive-workspace background polling or fully
  accurate inactive tab attention counts.
- This RFC does not implement workspace duplication, drag reorder, tab
  scrolling, or right-click context menus unless they are the chosen minimal
  access path for rename/delete.
- This RFC does not change the workspace file format except for backward
  compatible serde defaults if needed.
- This RFC does not edit historical implemented RFCs to mark them incomplete.

## External Design

### Entry Points

The workspace tab strip has a compact workspace actions affordance near the
existing tab controls:

```text
[work (2)] [personal]  [+ New workspace] [Workspace menu] [History] [Settings]
```

The exact visual form can be a small `...`/menu button, text button, or icon
button, but the production behavior must be explicit:

- `+ New workspace` opens create.
- `Rename workspace` opens rename for the active workspace.
- `Delete workspace` opens delete confirmation for the active workspace, or is
  disabled with a reason when only one workspace exists.

If a full menu is too much for the first implementation, rename and delete may
be placed in Settings only if the tab strip exposes a clear route to them. A
hidden handler is not acceptable.

### Create Workspace

```text
Create workspace

Name
[ Work projects                         ]

[Cancel] [Create workspace]
```

Validation examples:

- Empty: "Enter a workspace name."
- Duplicate: "That workspace already exists."
- Persistence failure: "We could not save this workspace."

On success, the dialog closes, the new workspace tab appears, and it becomes
the active workspace. The dashboard shows the empty-workspace state.

### Rename Workspace

```text
Rename workspace

Name
[ Work projects                         ]

[Cancel] [Rename workspace]
```

On success, the active tab label updates immediately and the workspace file is
saved.

### Delete Workspace

```text
Remove workspace?

This removes "lab" from knotra. Project folders on this computer stay where
they are.

[Cancel] [Remove workspace]
```

When only one workspace exists:

```text
Remove workspace
Keep at least one workspace.
```

The user should not have to understand workspace file paths or UUIDs to decide.

### Keyboard and Close Behavior

- `Esc` closes the topmost workspace dialog with no mutation.
- `Enter` confirms create/rename when the input is valid.
- Tab order reaches input, cancel, and confirm controls.
- Existing global close behavior must not discard unsaved typed input without
  following the same cancellation path as the visible Cancel button.

### Command Palette

The command palette has its own production-readiness RFC. For this RFC, only
one workspace-management action is in scope because it is already visible:
`Create new workspace`.

Implementation must choose one of two acceptable outcomes:

- Wire `Create new workspace` to `WorkspaceMessage::CreateWorkspaceDialogOpened`
  and the same create dialog as the tab-strip button.
- Hide `Create new workspace` from palette results until the later
  command-palette completion RFC.

Leaving the action visible while dispatching `None` is not acceptable. Full
palette parity for rename, delete, switch-next, and other actions remains
deferred to the command-palette completion RFC.

## Internal Design

### State

Keep `WorkspaceMgrState` as the owner of workspace dialog state, but make the
state expressive enough for production UI:

```rust
pub struct WorkspaceMgrState {
    pub create_dialog: Option<CreateWorkspaceDialog>,
    pub rename_dialog: Option<RenameWorkspaceDialog>,
    pub confirm_delete: Option<DeleteWorkspaceDialog>,
}
```

`confirm_delete: bool` may remain for a narrow implementation, but an explicit
dialog struct is preferred because the view needs the workspace name, project
count, and optional error.

Create and rename dialog state should carry:

- current input text;
- validation error;
- optional persistence error;
- an input focus id.

Validation should live in a pure helper, for example:

```rust
validate_workspace_name(candidate, existing, current_id) -> Result<String, WorkspaceNameError>
```

The helper trims the accepted value and can be unit-tested without iced.

### Messages

The current message set is mostly usable:

- `CreateWorkspaceDialogOpened`
- `CreateWorkspaceNameChanged(String)`
- `CreateWorkspaceConfirmed`
- `CreateWorkspaceCancelled`
- `RenameWorkspaceDialogOpened`
- `RenameWorkspaceNameChanged(String)`
- `RenameWorkspaceConfirmed`
- `RenameWorkspaceCancelled`
- `DeleteWorkspaceRequested`
- `DeleteWorkspaceConfirmed`
- `DeleteWorkspaceCancelled`
- `WorkspaceSwitched(WorkspaceId)`

Implementation may add more specific messages if the view needs them, such as
`WorkspaceDialogClosed`, `WorkspaceDeleteRequested(WorkspaceId)`, or
`WorkspaceMenuOpened`. Avoid overloading `ShortcutMessage::Close` in a way that
closes unrelated overlays before the active workspace dialog.

### View Composition

Add a workspace-management view module, for example:

```text
crates/knotra-app/src/view/workspace_manager.rs
```

It should expose one function that returns an optional overlay:

```rust
pub fn view(state: &AppState) -> Option<Element<'_, Message>>
```

`view.rs` then layers this overlay in the same stack as add-project,
palette, and shortcuts. The ordering should prevent workspace dialogs from
being visually hidden behind the palette or another modal. The first
implementation should define a simple topmost rule, such as:

1. shortcuts overlay;
2. command palette;
3. add-project dialog;
4. workspace-management dialog;
5. snora layout.

If multiple modal states are open because of existing bugs, `ShortcutMessage::Close`
must close only the visible topmost modal or use a deterministic close order.

### Handler Behavior

Workspace-management handler logic must be testable without touching the
developer's real application directories. The current implementation resolves
paths inside handlers via `AppPaths::resolve()` and inside `persist_workspace`.
This RFC requires extracting pure or path-parameterized helpers so tests can
pass temporary `AppPaths`.

Create:

1. Open dialog and focus name input.
2. On confirm, validate name.
3. Create `Workspace::new(trimmed_name)`.
4. Save via `save_workspace`.
5. Only after successful save, push to `all_workspaces`, update
   `active_workspace_idx`, update `workspace`, clear `workspace_status`, set
   refreshing state, close dialog, and refresh.
6. On save failure, keep the dialog open and show the error.

Rename:

1. Open dialog with active workspace name.
2. On confirm, validate name while allowing the active workspace's current name.
3. Update the active workspace object and matching `all_workspaces` entry.
4. Save via `save_workspace`.
5. On save failure, roll back in-memory name or avoid mutating until save
   succeeds.

Delete:

1. Do not allow deleting the last workspace.
2. On confirmation, remove the workspace file first or perform a recoverable
   sequence that does not leave the UI claiming deletion when the file remains.
3. Remove the workspace from `all_workspaces`.
4. Choose the next active index:
   - previous workspace if deleting the last tab;
   - otherwise the workspace now occupying the deleted index.
5. Update `workspace`, clear `workspace_status`, prune `fs_poller` to the new
   active workspace project ids, set refreshing state, and refresh.

Palette:

1. If this RFC wires the palette action, `state/palette.rs` must dispatch
   `action.workspace_create` to
   `Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened)`.
2. If this RFC hides the palette action, `build_entries` must omit
   `action.workspace_create` until the command-palette RFC.
3. In either case, selecting a visible `Create new workspace` palette entry
   must not return `None`.

### Persistence

Continue using `save_workspace` and the existing workspace file shape. The
implementation must introduce a testable path boundary. Acceptable approaches:

- helper functions that accept `&AppPaths` and are called by the iced handlers;
- an application path/provider field in state used by handlers;
- narrower operation helpers that take explicit workspace paths in tests and
  are wrapped by production handlers.

Do not write handler tests that depend on the developer's real
`~/.config/knotra`. Add a small delete helper if needed:

```rust
delete_workspace_file(workspace_id, paths) -> Result<(), String>
```

Direct `std::fs::remove_file` inside `app.rs` should be replaced or wrapped so
delete errors can be tested and shown to the user. Existing logging-only helper
paths such as `persist_workspace(ws)` should not be the only route used by
create/rename tests, because they hide persistence errors and resolve real
paths internally.

### i18n

Add all production strings to English and Japanese catalogs. Suggested keys:

- `workspace.create.title`
- `workspace.create.name_label`
- `workspace.create.confirm`
- `workspace.rename.title`
- `workspace.rename.confirm`
- `workspace.delete.title`
- `workspace.delete.body`
- `workspace.delete.confirm`
- `workspace.delete.disabled_last`
- `workspace.error.empty_name`
- `workspace.error.duplicate_name`
- `workspace.error.save_failed`
- `workspace.error.delete_failed`

Existing `plain.add_workspace` can remain for the tab-strip button label.

## Security Considerations

Workspace names are local UI data and TOML content, not shell commands. They
must never be interpolated into shell strings. The implementation must continue
to use UUID-derived file names, not user-provided names, for workspace files.

Delete must only remove the file for the selected workspace id under
`AppPaths::workspaces_dir`. It must not remove repository directories listed
inside the workspace.

Create and rename must preserve local-first behavior. No network, VCS process,
Git command, or jj command is introduced by this RFC.

Data loss prevention:

- Deleting a workspace removes only knotra's workspace list entry.
- Dialog copy must explicitly say project folders remain on disk.
- The last workspace is protected.
- Persistence failures remain visible and do not silently drop state.

## Test Plan

### Unit Tests

Add pure state/validation tests, preferably in `state/workspace_mgr.rs` or
`crates/knotra-app/src/tests.rs`:

1. `workspace_name_rejects_empty`
2. `workspace_name_rejects_duplicate_case_insensitive`
3. `workspace_name_allows_current_name_for_rename`
4. `create_dialog_defaults_empty`
5. `delete_last_workspace_is_not_allowed`
6. `delete_active_workspace_selects_nearest_remaining_workspace`

### UI Contract / Smoke Tests

Add tests or smoke coverage for visible control paths:

1. `new_workspace_button_opens_dialog`: the tab-strip button dispatches
   `CreateWorkspaceDialogOpened`, and the rendered app contains the create
   dialog state.
2. `create_workspace_confirm_persists_and_switches`: confirming a valid name
   saves a workspace, appends a tab, makes it active, and closes the dialog.
3. `rename_workspace_confirm_persists_and_updates_tab`: rename changes the
   active tab label and saved file.
4. `delete_workspace_confirm_removes_tab_and_file`: delete removes the active
   workspace from state and disk while preserving project folders.
5. `cancel_paths_do_not_mutate_workspace_state`.
6. `palette_create_workspace_is_wired_or_hidden`: the palette either does not
   show `Create new workspace`, or selecting it opens the create dialog.

If iced view introspection is not practical, create focused view-model helpers
that expose dialog visibility and enabled/disabled state, and test those
helpers plus update handlers.

### Persistence Tests

Use temporary config directories through the path boundary required by this
RFC. Do not read or write the developer's real `~/.config/knotra`.

1. Save then load a created workspace.
2. Rename then load the same workspace id with the new name.
3. Delete removes only the workspace file.
4. Create, rename, and delete handler-level tests pass explicit temporary
   paths or a test path provider.

### i18n Tests

Extend catalog parity coverage so all `workspace.*` keys exist in English and
Japanese. Ensure first-level workspace dialog labels avoid forbidden jargon.

### Commands

For implementation review, run and observe:

```sh
cargo +1.91 fmt --check
cargo +1.91 clippy --workspace --all-targets
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
```

If `knotra-vcs` tests remain environment-sensitive before the separate
hermeticity fix, document the isolated Git environment used and do not claim
the normal gate is production-ready.

## Acceptance Criteria

- [ ] `+ New workspace` opens a visible create-workspace dialog.
- [ ] Palette `Create new workspace` is either wired to the same dialog or
      hidden; it does not silently close with no action.
- [ ] Create, rename, and delete workspace flows are reachable from visible UI.
- [ ] Create and rename reject empty and duplicate names with plain-language
      errors.
- [ ] Create persists the new workspace, appends a tab, activates it, and
      refreshes its dashboard.
- [ ] Rename persists and updates both active workspace state and tab label.
- [ ] Delete removes only the workspace record, never repository folders.
- [ ] Delete is disabled or blocked with a clear reason for the last remaining
      workspace.
- [ ] Cancel and close paths do not mutate workspace state or disk.
- [ ] Persistence failures remain visible and do not silently claim success.
- [ ] Handler and persistence tests use temporary paths through an explicit
      path boundary, not real application directories.
- [ ] All new production strings are in the English and Japanese i18n catalog.
- [ ] Tests prove visible workspace controls reach their intended handler,
      state, persistence, and rendered result.
- [ ] No placeholder workspace-management control remains visible.
- [ ] No debug output is rendered in production workspace UI.
- [ ] Current gate evidence is recorded before moving this RFC to `done/`.

## Review Questions

1. Should rename/delete be exposed in the tab strip, Settings, or both for the
   first production-ready implementation?
2. Should duplicate-name validation be case-insensitive as proposed, or should
   workspaces allow names that differ only by case?
3. Should delete remove the workspace file before mutating in-memory state, or
   should the implementation mutate then roll back on delete failure?
4. For the first implementation, should palette `Create new workspace` be wired
   immediately or hidden until the command-palette RFC?
