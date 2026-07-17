# RFC-028 - Command Palette Action Completion

| Field | Value |
|---|---|
| Status | Implemented (main: 3699bad) |
| Priority | High - the palette advertises actions that can silently do nothing |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/state/palette.rs`, `crates/knotra-app/src/view/command_palette.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/012-command-palette.md`, `rfcs/done/023-workspace-management-completion.md`, `rfcs/done/024-smart-pull-modal-execution-completion.md`, `rfcs/done/025-freezer-release-point-execution-completion.md`, `rfcs/done/027-selection-mode-and-bulk-selection-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the command palette as an honest production control surface. The
palette currently advertises actions, projects, and workspaces, but several
action keys fall through to `None`, project rows also dispatch `None`, and the
palette closes even when no action is executed. This violates the production
rule that every visible control must work, be disabled with a clear reason, or
be hidden until supported.

This RFC defines a palette action registry with explicit availability,
disabled reasons, dispatch targets, and tests for every advertised row. It also
defines which current actions are in scope now and which must be hidden until
later RFCs complete the underlying workflow.

## Background

RFC-012 introduced the command palette as a discoverability and keyboard-speed
surface. It intended the palette to search actions, projects, and workspaces,
and to execute the selected row on Enter or click.

Current implementation has useful pieces:

- `PaletteState` tracks open/query/results/highlight state.
- `state/palette.rs` builds action, project, and workspace entries.
- `command_palette.rs` renders a centered overlay.
- `handle_palette` handles open, close, query, highlight movement, confirm,
  and row click.
- Workspace rows can dispatch `WorkspaceMessage::WorkspaceSwitched`.
- Some action rows already dispatch real messages, such as Settings, History,
  Refresh, Select visible projects, Clear selection, Add project, Create
  workspace, and Show keyboard shortcuts.

The production gap is the advertised action contract:

- `Fetch all projects` falls through to `None`.
- `Pull selected projects` falls through to `None`.
- `Tag selected projects...` falls through to `None`.
- `Switch branch on selected...` falls through to `None`.
- `Generate changelog for selected...` falls through to `None`.
- `Remove project from workspace` falls through to `None`.
- `Switch to next workspace` falls through to `None`.
- `Toggle dark mode` falls through to `None`.
- Project rows fall through to `None`.
- `handle_palette` closes the palette even when dispatch returns `None`.

After RFC-027, selection and selection-bar workflows are more coherent. The
palette should now mirror those visible contracts rather than carrying its own
parallel, stale action semantics.

## Motivation

Users open a command palette because they expect fast, reliable execution. A
palette row that closes without doing anything is worse than a missing feature:
it creates false confidence and hides failure.

Product readiness requires the palette to be audited like any other visible
control. If a row is visible, it must either dispatch the intended workflow,
stay open with a clear disabled reason, or be omitted until the target workflow
is production-ready.

Operationally, several palette actions launch mutating VCS workflows. They must
reuse the same selection, validation, progress, result, and disabled-state
contracts as the visible dashboard and modal controls.

## Requirements

### Functional

R1. Every palette row has an explicit kind: enabled action, disabled action,
project navigation, workspace switch, or hidden/deferred item.

R2. Confirming or clicking an enabled action dispatches exactly one intended
message path.

R3. Confirming or clicking a disabled action does not close the palette and
shows a plain-language reason.

R4. Confirming or clicking an unavailable row must not silently close the
palette.

R5. The palette action registry must be the single source of truth for action
labels, payload IDs, availability, disabled reasons, and dispatch behavior.

R6. Action labels and disabled reasons must be routed through the i18n catalog.

R7. Action labels must use current plain-language product labels where the UI
already has them, such as `Check for updates`, `Get latest safely`, `Save
release point`, and `Change work area`.

R8. `Check for updates` / fetch-all must be explicit. If the palette keeps a
workspace-wide fetch action, it must be labelled as all-workspace scope, must
initialize sync selection for all fetchable active-workspace projects, and must
not depend on stale modal selection state.

R9. `Get latest safely` from the palette mirrors RFC-027 selection behavior:
it is enabled only when selected projects contain at least one project with an
update source, and it dispatches the Smart Pull selected-project workflow.

R10. `Save release point` from the palette is enabled only with a non-empty
selection and dispatches the Freezer selected-project workflow.

R11. `Change work area` from the palette is enabled only with exactly one
selected project and dispatches the same scoped context workflow as the
selection bar.

R12. `Remove project from workspace` from the palette is enabled only with
exactly one selected project and opens the existing remove confirmation dialog
for that project.

R13. `Select visible projects` dispatches `SelectionMessage::SelectAll`.

R14. `Clear selection` dispatches `SelectionMessage::Clear`. It is disabled
when selection mode is inactive and no project is selected.

R15. `Add project to workspace` opens the Add Project dialog.

R16. `Create new workspace` opens the Create Workspace dialog.

R17. `Switch to next workspace` is enabled only when at least two workspaces
are loaded and switches to the next workspace in tab order.

R18. `Open Settings`, `Open History`, `Refresh workspace`, and `Show keyboard
shortcuts` continue to dispatch their existing message paths.

R19. `Toggle dark mode` must either dispatch a real theme toggle with behavior
matching Settings or be hidden until settings persistence semantics are defined.
It must not remain visible as a no-op action.

R20. `Generate changelog for selected...` must be hidden until the changelog
modal completion RFC lands, unless implementation also guarantees the palette
path opens a non-debug, production-ready changelog workflow. The default
decision for this RFC is to hide it.

R21. Project rows must not close silently. They must either open the project
detail panel for the selected project, focus the project card if a focus/scroll
contract exists, or be hidden. The default decision for this RFC is to open the
project detail panel.

R22. Workspace rows continue to switch workspaces, but confirmation must leave
the palette only after dispatching `WorkspaceMessage::WorkspaceSwitched`.

### Non-Functional

N1. First-level palette labels and disabled reasons are plain-language and
localized in English and Japanese.

N2. The palette must remain keyboard-accessible: open, query, move, confirm,
and close behavior must continue to work.

N3. Disabled rows must be visually distinguishable from enabled rows and must
preserve stable row height.

N4. The implementation must avoid new shell/process execution paths.

N5. Palette dispatch must reuse existing workflow messages rather than adding
parallel backend calls.

### Git and jj Behavior

G1. The palette does not add new VCS semantics. It only routes to existing
Git/jj-aware workflows.

G2. Smart Pull behavior follows RFC-024 and RFC-027.

G3. Release-point behavior follows RFC-025 and RFC-027.

G4. Context switching behavior follows RFC-027 for one selected project and is
further constrained by the later typed context-switching RFC.

G5. Changelog behavior remains deferred to the changelog completion RFC.

## Goals

- Remove silent no-op palette rows.
- Keep the palette open when the highlighted row cannot be executed.
- Provide disabled reasons for unavailable actions.
- Mirror selection-bar and workspace controls rather than inventing alternate
  command paths.
- Open project detail from project rows or hide project rows if detail opening
  is not viable.
- Add tests proving every visible palette row dispatches, stays disabled with a
  reason, or is hidden.

## Non-Goals

- This RFC does not implement a fuzzy matching library or recent-items
  persistence from RFC-012.
- This RFC does not complete typed context switching.
- This RFC does not complete changelog rendering.
- This RFC does not add per-project VCS history.
- This RFC does not redesign palette visuals beyond disabled states and
  first-level copy needed for correctness.
- This RFC does not add new VCS backend operations.

## External Design

### Rows

Enabled action row:

```text
Check all projects for updates
```

Disabled action row:

```text
Get latest safely
Choose at least one project.
```

Project row:

```text
Project: api-server
```

Workspace row:

```text
Workspace: Release Train
```

### Confirm Behavior

- Enabled action: close palette and dispatch the action.
- Disabled action: keep palette open and show the disabled reason.
- Project row: close palette and open the project detail panel.
- Workspace row: close palette and switch workspace.
- No result row: no action.

### Current Action Decisions

| Current row | RFC-028 decision |
|---|---|
| Fetch all projects | Keep, relabel as all-workspace check/fetch, wire to all active workspace projects |
| Pull selected projects | Keep, relabel as `Get latest safely`, disabled unless selection supports it |
| Tag selected projects... | Keep, relabel as `Save release point`, disabled unless selection is non-empty |
| Switch branch on selected... | Keep, relabel as `Change work area`, disabled unless exactly one project is selected |
| Generate changelog for selected... | Hide until changelog completion RFC |
| Add project to workspace | Keep and wire |
| Remove project from workspace | Keep, disabled unless exactly one project is selected |
| Create new workspace | Keep and wire |
| Switch to next workspace | Keep, disabled unless at least two workspaces exist |
| Select visible projects | Keep and wire |
| Clear selection | Keep, disabled unless selection is active or non-empty |
| Open Settings | Keep and wire |
| Open History | Keep and wire |
| Toggle dark mode | Hide unless implementation wires a real settings-compatible toggle |
| Refresh workspace | Keep and wire |
| Show keyboard shortcuts | Keep and wire |

## Internal Design

### Palette Entry State

Replace the action-only `Option<Message>` dispatch model with an explicit entry
availability model. The exact names can differ, but the implementation must
represent:

```rust
pub enum PaletteAvailability {
    Enabled,
    Disabled { reason_key: &'static str },
    Hidden,
}
```

Palette rows should carry enough data for the view and handler to know whether
the row can be executed. Disabled rows must remain visible only when useful;
hidden rows must not appear in search results.

### Action Registry

Create a registry of palette actions with:

- stable action ID;
- i18n label key or label builder;
- availability function;
- dispatch function;
- optional search aliases.

The registry should replace the current static `ACTIONS` tuple plus
`match payload` split so labels and dispatch cannot drift independently.

### Dispatch

`dispatch_entry` should return a richer outcome, such as:

```rust
pub enum PaletteDispatch {
    Dispatched(Message),
    Disabled(&'static str),
    Noop,
}
```

`handle_palette` must close the palette only for `Dispatched`. Disabled or
noop outcomes keep the palette open and set a visible palette status/notice.

### Selection Summary

Palette actions that depend on selection must consume
`AppState::selection_summary()` from RFC-027. They must not inspect selection
in multiple divergent ways.

### Project Rows

Project rows dispatch `DetailPanelMessage::Opened(project_id)`. If future
scroll/focus support is added, it can be layered after the detail panel opens,
but this RFC does not require it.

### Workspace Rows

Workspace rows dispatch `WorkspaceMessage::WorkspaceSwitched(workspace_id)`.
The active workspace row can be disabled with a reason such as `Already open`
or hidden from results. The implementation must choose and test one behavior.

## Security Considerations

The palette must not add shell execution, process spawning, or string command
construction. It dispatches existing app messages only.

Mutating VCS workflows launched from the palette must reuse their existing
validation and confirmation flows. The palette must not bypass Smart Pull,
Freezer, or context-switch confirmation states.

Project and workspace IDs come from loaded application state. The palette must
not parse user-entered query text as a repository path, branch, tag, or command.

## Test Plan

### Unit and State Tests

- Palette registry contains no visible action whose dispatch is missing.
- Hidden actions do not appear in `update_results`.
- Disabled actions appear with disabled reasons when the design says they
  should be visible.
- `dispatch_entry` returns Disabled and does not close for disabled rows.
- `dispatch_entry` returns Dispatched for each enabled action.

### UI Contract / Smoke Tests

- `Get latest safely` palette action dispatches Smart Pull selected-project
  flow only when selection supports it.
- `Save release point` palette action dispatches Freezer selected-project flow
  only with non-empty selection.
- `Change work area` palette action dispatches one-selected-project context
  flow only with exactly one selected project.
- `Remove project from workspace` opens confirmation only with exactly one
  selected project.
- `Check all projects for updates` initializes all active workspace projects
  for fetch and does not use stale modal selection.
- `Switch to next workspace` moves to the next workspace when more than one
  exists.
- Project row opens the project detail panel.
- Workspace row switches workspace or is disabled when already active,
  depending on the chosen behavior.
- Confirming an unavailable row keeps the palette open and shows the reason.

### i18n Tests

- English and Japanese catalog entries exist for new palette labels, disabled
  reasons, empty states, and status notices.
- Existing first-level wording guard continues to pass.

### Commands

Required before implementation acceptance:

- `cargo +1.91 fmt --check`
- `cargo +1.91 test -p knotra`
- `cargo +1.91 test -p knotra-ui`
- `env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs`
- `cargo +1.91 clippy --workspace --all-targets`

## Acceptance Criteria

- [ ] No visible palette action falls through to silent `None`.
- [ ] Confirming a disabled row keeps the palette open and shows a reason.
- [ ] Palette action labels and dispatch targets come from one registry.
- [ ] Hidden/deferred actions do not appear in search results.
- [ ] Project rows open the project detail panel or are intentionally hidden.
- [ ] Workspace rows switch workspace or are intentionally disabled for the
      active workspace.
- [ ] Selection-dependent actions consume the RFC-027 selection summary.
- [ ] Palette actions mirror visible UI controls for Smart Pull, Freezer,
      context switch, workspace management, selection, settings/history, and
      shortcuts.
- [ ] New user-facing strings are localized in English and Japanese.
- [ ] Tests prove visible palette row -> dispatch/disabled outcome for every
      visible row kind.
- [ ] Current gate evidence is recorded before moving this RFC to `done/`.
