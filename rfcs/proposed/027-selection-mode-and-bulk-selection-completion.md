# RFC-027 — Selection Mode and Bulk-Selection Completion

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High — bulk workflows depend on a coherent selection contract |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/state/palette.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/009-selection-model.md`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md`, `rfcs/done/024-smart-pull-modal-execution-completion.md`, `rfcs/done/025-freezer-release-point-execution-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the dashboard selection-mode contract that drives bulk workflows.
The application already has `SelectionState`, `selection_mode`, selection
messages, a selection bar, dashboard card checkboxes, keyboard shortcut text,
and command-palette selection actions. These pieces are not yet presented as
one coherent user function in the active UI.

This RFC defines when selection mode starts and ends, which card affordances are
visible, how the selection bar behaves with zero or many selected projects, and
how keyboard and command-palette selection actions interact with the dashboard.
Bulk actions launched from the selection bar must either open the real workflow,
be disabled with a plain-language reason, or be hidden.

## Background

RFC-009 introduced the original selection model and selection bar as a
foundation for bulk workflows. RFC-013 moved those workflows into modals, and
RFC-021 required plain-language first-level UI. RFC-024 and RFC-025 completed
the Smart Pull and release-point execution paths that the selection bar can
launch.

Current code has partial implementation:

- `SelectionState` stores `selected_ids` and `anchor_id`.
- `AppState` stores explicit `selection_mode`.
- `SelectionMessage` includes toggle, range, select-all, clear, mode-enter,
  and mode-exit variants.
- `handle_selection` toggles IDs, selects all workspace projects, and clears on
  exit.
- `selection_bar::view` renders only when `selection_mode` is true.
- The active generic dashboard card renders a checkbox unconditionally.
- Dead tier-specific card helpers render checkboxes only when
  `selection_mode` is true.
- The command palette advertises `Select all projects` and `Clear selection`.

The production gap is the contract between those pieces:

- Users can see card selection checkboxes even when the selection bar is hidden.
- `selection_mode` can be true with no selected projects, producing a bulk bar
  that says `0 selected` with some actions still clickable.
- Deselecting the last project leaves mode semantics unclear.
- Range selection exists in messages/state but has no visible or tested active
  UI contract.
- Bulk action availability is not consistently tied to selection count,
  workspace state, VCS support, or missing path state.
- Selection strings such as count labels are still hardcoded in production UI.

## Motivation

Selection is the entry point for high-value workflows: get latest, save a
release point, switch work area, and later changelog generation. If the
selection surface is inconsistent, users cannot predict which repositories will
be mutated by a bulk action.

Product readiness requires visible controls to either work, be disabled with a
clear reason, or be hidden. Selection mode must therefore make its state and
action availability obvious before any mutating workflow opens.

Operationally, selection itself is non-mutating, but it scopes later mutating
VCS operations. It must not accidentally include hidden or stale projects, and
it must clear when the active workspace changes.

## Requirements

### Functional

R1. The dashboard must expose a visible way to enter selection mode.

R2. Selection mode starts when the user enters selection mode, selects all
projects, or toggles a project through a visible selection affordance.

R3. Selection mode ends only through `Exit selection`, `Clear selection`,
workspace switch, project removal of all selected projects, or an accepted
keyboard close action. The behavior must be explicit and tested.

R4. The active dashboard card render path must show project checkboxes only
while selection mode is active.

R5. Hidden or dead card render paths must either be removed or use the same
selection-affordance helper as the active render path.

R6. Selection mode with zero selected projects must show a clear empty-selection
state, not a misleading set of enabled bulk actions.

R7. With zero selected projects, all bulk action buttons are disabled with the
plain-language reason `Choose at least one project` or equivalent.

R8. With one or more selected projects, the selection bar shows a localized
count and the supported bulk actions for the current selection.

R9. `Get latest` is enabled only when at least one selected project has a known
upstream and the workflow can build a plan. Otherwise it is disabled with a
plain-language reason.

R10. `Save release point` is enabled only when at least one selected project is
eligible for the release-point workflow. If eligibility cannot be determined at
the bar level, the modal must open into its validation state and present
per-project blockers.

R11. `Change work area` is enabled only when exactly one selected project is
supported by the context-switch workflow, unless the context-switch RFC later
defines a real multi-project switch. Until then, multi-selection must be
disabled with a reason.

R12. `Check for updates` from the selection bar targets selected projects only.
It must initialize the fetch workflow from `state.selection.selected_ids` and
must not read stale modal-internal selection state or silently expand an empty
selection to all workspace projects.

R13. `Check for updates` is disabled when no project is selected. If selected
projects exist but none are fetchable because they are missing, unsupported, or
unavailable, the button is disabled with a plain-language reason. If the
selection is mixed, the workflow may run for fetchable selected projects, but
missing/unsupported/unavailable selected projects must appear as skipped
per-project results or equivalent first-level feedback.

R14. The selection action label is `Select visible projects`. It selects the
active dashboard's visible project set after current search/filter/tier
visibility rules are applied. If visible IDs cannot be derived in the first
implementation, the action must be renamed to `Select all workspace projects`
and select the active workspace project set consistently in view, palette,
tests, and i18n.

R15. Range selection must operate over the same ordered project list that the
active dashboard renders.

R16. Selection is pruned when projects are removed and cleared when the active
workspace changes.

R17. Selection must not persist to disk or survive application restart.

R18. Command-palette selection actions must match the visible selection
contract: `Select visible projects` enters selection mode and selects the
intended set; `Clear selection` exits selection mode. They must not silently
no-op.

R19. Command-palette bulk actions that depend on selection are deferred to the
command-palette completion RFC, except that this RFC must not make selection
state inconsistent with those future actions.

### Non-Functional

N1. New user-facing selection strings are routed through the i18n catalog in
English and Japanese.

N2. First-level wording uses plain language. Technical terms such as upstream,
ref, dirty, staged, or VCS stay out of primary selection-bar copy unless behind
details in the launched workflow.

N3. Selection controls preserve keyboard accessibility and visible focus.

N4. Selection targets must have stable layout dimensions so checkboxes and bar
buttons do not shift rows unexpectedly when mode changes.

N5. Selection state must be deterministic and testable without depending on
global Git or jj configuration.

### Git and jj Behavior

G1. Selection itself is VCS-neutral. Git, jj, missing-path, and unsupported
states affect only action availability and downstream modal validation.

G2. `Get latest` follows the Smart Pull RFC behavior for Git and jj after the
selection is handed to the sync state.

G3. `Save release point` follows the Freezer RFC behavior for Git tags and jj
bookmarks after the selection is handed to freezer state.

G4. `Change work area` follows the context-switch RFC. Until typed context
switching is complete, the selection bar must avoid promising unsupported
multi-project switching.

## Goals

- Make selection mode discoverable from the dashboard.
- Align card checkboxes, selection mode state, selection bar visibility,
  keyboard shortcuts, and palette actions.
- Prevent empty-selection bulk actions from dispatching mutating workflow
  messages.
- Preserve selected project IDs only while they belong to the active workspace.
- Provide a reusable selection summary for action enablement and UI tests.
- Add tests proving visible selection controls reach the intended message and
  state transitions.

## Non-Goals

- This RFC does not implement command-palette parity for every advertised bulk
  action. That is covered by the next RFC.
- This RFC does not complete typed context switching; it only prevents the
  selection bar from overpromising unsupported multi-project switching.
- This RFC does not redesign dashboard grouping, sorting, or tier-density.
- This RFC does not add persistent saved project sets.
- This RFC does not add new VCS backend operations.

## External Design

### Dashboard Entry

The dashboard toolbar exposes a `Select` control when a workspace has projects.
Activating it enters selection mode and reveals a fixed checkbox slot on every
visible project row.

When no workspace project is visible because of search/filter state, the
control is disabled with a plain-language reason such as `No projects match
this view`.

### Card Rows

Normal mode:

```text
project-alpha              Ready
project-beta               Needs attention
```

Selection mode:

```text
☐  project-alpha           Ready
☑  project-beta            Needs attention
```

The checkbox column has reserved width so entering or exiting selection mode is
predictable. Clicking a project name still opens the detail panel. Clicking the
checkbox toggles only selection.

### Selection Bar

Empty selection:

```text
No projects selected        [Check for updates] [Get latest] [Save release point] [Change work area] [Exit selection]
```

All bulk action buttons are disabled with the reason `Choose at least one
project`. `Exit selection` remains enabled.

Non-empty selection:

```text
3 selected                  [Check for updates] [Get latest] [Save release point] [Change work area] [Exit selection]
```

Disabled buttons use the same guided disabled-button behavior as the bulk
modals, including a visible reason. For example:

- `Get latest`: `No selected project has an update source`.
- `Check for updates`: `Choose at least one project`.
- `Change work area`: `Choose one project to change work area`.
- `Save release point`: `Choose at least one project`.

`Check for updates` in this bar is intentionally selected-projects-only. A
separate all-workspace refresh or fetch control may exist elsewhere, but it must
not be conflated with the selected-projects bulk action.

### Keyboard

- `Space` on a focused checkbox toggles that project.
- `Shift+Space` range-selects to the focused project.
- `Ctrl+A` / `Cmd+A` selects the same project set as the visible
  `Select visible projects` action when dashboard content has focus.
- `Esc` exits selection mode when no modal, dialog, palette, or sheet is
  currently handling close.

Modal and overlay close behavior remains topmost-first. Selection clear must
not consume `Esc` while another topmost layer should close.

### Command Palette

Selection actions must be honest:

- `Select visible projects` enters selection mode and selects the active
  dashboard's visible project set.
- `Clear selection` exits selection mode and clears selected IDs.

Palette actions that launch bulk workflows are completed by the command-palette
RFC. Until then, this RFC must not add new silently closing palette actions.

## Internal Design

### State

Keep `SelectionState` and `selection_mode`, but define their invariant:

- `selection_mode == false` means no selection UI is shown and
  `selected_ids` must be empty.
- `selection_mode == true` means dashboard selection affordances and the
  selection bar are visible, even when `selected_ids` is empty.
- `anchor_id` must be `None` when `selected_ids` is empty.
- Deselecting the last selected project leaves `selection_mode == true`, clears
  `anchor_id`, and shows the empty-selection bar.

Add a helper such as:

```rust
pub struct SelectionSummary {
    pub mode_active: bool,
    pub selected_count: usize,
    pub visible_count: usize,
    pub selected_ids: Vec<ProjectId>,
    pub has_upstream: bool,
    pub has_missing_path: bool,
}
```

The exact shape can differ, but the implementation must avoid duplicating
selection eligibility logic across the dashboard, selection bar, and tests.

### Messages

Existing messages can be retained if they meet the contract:

- `ModeEntered`
- `ModeExited`
- `Toggled(ProjectId)`
- `RangeTo(ProjectId)`
- `SelectAll`
- `Clear`
- `FocusMoved(ProjectId)`

Implementation may add more explicit messages, such as `SelectVisible`, if
needed to avoid ambiguity between all workspace projects and visible projects.

### Update Behavior

The selection handler must enforce invariants:

- `ModeEntered`: `selection_mode = true`; selection remains unchanged.
- `ModeExited` / `Clear`: clear `selected_ids`, clear `anchor_id`, and set
  `selection_mode = false`.
- `Toggled`: valid only for active workspace project IDs; enters selection mode
  if needed; toggles the ID; updates or clears the anchor deterministically.
- `RangeTo`: enters selection mode; selects the inclusive range over active
  rendered order.
- `SelectAll`: enters selection mode and selects visible project IDs for
  `Select visible projects`, or consistently selects workspace project IDs if
  the implementation adopts the fallback label.
- Workspace switch: clear selection and exit selection mode.
- Project removal: prune removed IDs; if no selected IDs remain, either keep
  explicit empty selection mode only when the user is still on the dashboard, or
  exit mode. The chosen behavior must be tested and documented in code.

### View Composition

Create one dashboard selection-control helper and use it from the active card
path. Dead tier-specific card paths must not carry divergent checkbox logic.
They should be removed, left unreachable with no selection code, or routed
through the same helper.

`selection_bar::view` should render when `selection_mode` is true. Its actions
must use `on_press_maybe` or the existing guided disabled-button helper so an
empty or unsupported selection cannot dispatch a misleading action.

### Bulk Workflow Handoff

When a bulk action is enabled:

- Check for updates receives exactly `state.selection.selected_ids` and runs as
  a selected-projects-only workflow. It must initialize or bypass any
  modal-internal fetch selection so stale `state.sync.project_selection` cannot
  define the target set.
- Smart Pull receives exactly `state.selection.selected_ids`.
- Freezer receives exactly `state.selection.selected_ids`.
- Context switch receives exactly one selected project until multi-project
  context switching is deliberately designed.
- Empty selected-project sets cannot dispatch Check for updates, Smart Pull,
  Freezer, or Context Switch workflow messages.

## Security Considerations

Selection stores project IDs only and does not execute shell commands or VCS
operations by itself.

The security risk is indirect: a stale or hidden selection could scope later
mutating VCS operations incorrectly. The implementation must validate selected
IDs against the active workspace before launching any bulk workflow and must
clear or prune selection when the workspace/project set changes.

No repository path, branch, tag, or command text should be parsed or executed
as part of this RFC.

## Test Plan

### Unit and State Tests

- `selection_mode_enter_shows_empty_selection_state`: entering selection mode
  sets `selection_mode` and leaves selection empty.
- `selection_clear_exits_mode_and_clears_anchor`: clear restores the invariant.
- `selection_toggle_enters_mode_and_toggles_project`: toggling a visible project
  selects and deselects it deterministically.
- `selection_last_deselect_keeps_empty_mode`: deselecting the last selected
  project leaves `selection_mode == true`, clears `anchor_id`, and renders
  disabled bulk actions.
- `selection_toggle_rejects_project_outside_active_workspace`: stale IDs cannot
  be selected.
- `selection_select_all_uses_defined_project_set`: select-all matches the RFC's
  active workspace or visible-project decision.
- `selection_range_uses_rendered_order`: range selection follows dashboard
  ordering.
- `workspace_switch_clears_selection_mode`: switching workspaces clears
  selected IDs and exits selection mode.
- `project_removal_prunes_selection`: removing a project removes its ID from
  selection.

### UI Contract / Smoke Tests

- Dashboard `Select` control dispatches `SelectionMessage::ModeEntered`.
- In selection mode, a card checkbox dispatches
  `SelectionMessage::Toggled(project_id)`.
- In normal mode, the active card path does not render a checkbox affordance.
- Empty selection bar renders disabled bulk actions and enabled exit action.
- Selection-bar `Check for updates` dispatches only when selected IDs exist and
  loads exactly those IDs into the fetch workflow.
- Selection-bar `Check for updates` is disabled when all selected projects are
  missing, unsupported, or unavailable.
- Non-empty selection bar dispatches Smart Pull and Freezer messages with the
  exact selected IDs loaded into their workflow state.
- Context switch action is disabled for zero or multiple selected projects.
- Palette `Select visible projects` and `Clear selection` dispatch and preserve
  the same state invariants as visible controls.

### i18n Tests

- English and Japanese catalog entries exist for new selection labels,
  counts, and disabled reasons.
- Existing wording guard continues to pass with no first-level technical terms.

### Commands

Required before implementation acceptance:

- `cargo +1.91 fmt --check`
- `cargo +1.91 test -p knotra`
- `cargo +1.91 test -p knotra-ui`
- `env GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null VISUAL=true EDITOR=true cargo +1.91 test -p knotra-vcs`
- `cargo +1.91 clippy --workspace --all-targets`

## Acceptance Criteria

- [ ] Dashboard has a visible way to enter selection mode.
- [ ] Active dashboard cards show selection checkboxes only in selection mode.
- [ ] Selection mode with zero selected projects has clear empty-state copy.
- [ ] Empty-selection bulk actions are disabled and cannot dispatch workflow
      messages.
- [ ] `Check for updates` from the selection bar is selected-projects-only and
      cannot read stale modal-internal selection state.
- [ ] Missing, unsupported, or unavailable selected projects are disabled at
      bar level when none can be fetched, or shown as skipped per-project
      results when mixed with fetchable projects.
- [ ] Non-empty selection bar actions use the selected project set exactly.
- [ ] `Change work area` does not promise unsupported multi-project switching.
- [ ] Selection is cleared on workspace switch and pruned on project removal.
- [ ] Palette select-all and clear-selection actions match visible behavior.
- [ ] New user-facing strings are localized in English and Japanese.
- [ ] Tests prove visible control -> message -> handler -> state/result paths.
- [ ] No placeholder visible selection controls remain.
- [ ] Current gate evidence is recorded before moving this RFC to `done/`.
