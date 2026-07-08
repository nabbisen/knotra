# RFC-009 — Selection Model and Selection Bar

| Field          | Value                                                          |
|----------------|----------------------------------------------------------------|
| Status      | Implemented (v0.12.0) |
| Priority       | **High** — foundational for the UI/UX redesign                 |
| Effort         | Medium — new state field, new widget, integration with handlers |
| Target version | v0.12                                                          |
| Related        | `crates/knotra-app/src/app.rs`, `state/mod.rs`, `view/dashboard.rs` |

## Summary

Introduce a project-selection mechanism: the user can select one or more
projects on the Dashboard, and a context-sensitive action bar appears at the
bottom with bulk operations.  This is the foundational interaction pattern of
the redesigned UI — every bulk operation in later RFCs builds on it.

## Background

The current UI requires the user to navigate to a screen (Sync Center,
Freezer, ContextOps) to perform bulk operations.  Each screen has its own
project list and its own selection state, mostly through checkboxes or "all
projects" behaviour.  This creates a disconnect: the Dashboard shows status,
but actions live elsewhere.

The redesign moves all bulk operations into a single selection-then-act
model on the Dashboard.  This RFC delivers the **mechanic**; later RFCs
connect specific actions to it.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | The user can select projects via mouse (checkbox on card) or keyboard (space) |
| R2  | The user can select a contiguous range via `shift+click` or `shift+space` |
| R3  | The user can toggle individual projects via `ctrl+click` (in addition to the existing single-select) |
| R4  | The user can select all visible projects via `ctrl+a` (limited to the current workspace) |
| R5  | The user can clear the selection via `escape` |
| R6  | The selection bar appears at the bottom when ≥1 project is selected; hidden otherwise |
| R7  | The selection bar shows the selection count and primary action buttons (`Fetch`, `Pull`, `Tag…`, `Switch…`, `⋯`) |
| R8  | Action buttons disable when no selected project supports the action (e.g., `Pull` disables if none have an upstream) |
| R9  | The selection persists across status refreshes within the same workspace |
| R10 | Selection clears when the user switches workspaces |
| R11 | Removing a project from the workspace also removes it from the selection |
| R12 | Bulk action progress and result is reported via the activity strip (RFC-011); not via the selection bar itself |

## External Design

### Visual

#### Selected card

```
┌──────────────────────────────────────────────────────────────────┐
│ ☑  project-gamma            feature-x · ↑2 · 3 dirty             │
└──────────────────────────────────────────────────────────────────┘
       ▲
       └── checkbox visible on hover or when card is in any selection state
```

Cards in any selection state (selected or unselected-but-near-selection)
show their checkbox.  Cards in a workspace with **zero selection** do not
show checkboxes — the UI is the same as today.  Selection mode is implicit:
the first checkbox click activates it.

#### Selection bar

```
┌──────────────────────────────────────────────────────────────────┐
│ ✓ 4 selected   [⤓ Fetch]  [⤒ Pull]  [Tag…]  [Switch…]  [⋯]  [✕] │
└──────────────────────────────────────────────────────────────────┘
                                                                   ▲
                                                                   └── clear selection
```

- Sticky at the bottom of the main view.
- Animates in (slide up + fade) when selection count goes from 0 → ≥1.
- Animates out when selection count goes from ≥1 → 0.
- The `⋯` overflow opens a popover with secondary actions: Generate
  changelog, Open all in terminal, Remove from workspace, Export status JSON.
- Buttons that don't apply to the entire selection are disabled with a
  tooltip explaining why (e.g., "No selected project has an upstream").

### Interaction

| Trigger              | Effect                                            |
|----------------------|---------------------------------------------------|
| Click checkbox       | Toggle selection for that project                 |
| `space` on focused card | Toggle selection                               |
| `shift+space` / `shift+click` | Range-select from the last selected to the focused card |
| `ctrl+click` (or ⌘+click on macOS) | Toggle that one project's selection without affecting others |
| `ctrl+a`             | Select all projects in the current workspace      |
| `escape` (with selection) | Clear selection                              |
| `escape` (without selection, modal open) | Close modal (existing behaviour) |
| Workspace switch     | Selection clears                                  |
| Project removed      | Project removed from selection                    |

### Focus model

Cards become focusable elements.  `tab` cycles through focusable controls
on the page (sidebar, search box, then each project card, then the
selection bar buttons).  `arrow up`/`arrow down` moves focus between cards
within the focused tier.  This is in addition to the existing `j`/`k`
shortcuts proposed in RFC-016.

### Edge case — selecting across tiers

The selection bar shows a unified count regardless of which tier the
selected projects came from.  Range selection (`shift+space`) only works
within a tier; selecting across tiers requires `ctrl+space` per card or
`ctrl+a` for all.

### Edge case — selection includes a path-missing project

The selection bar still shows the project's count, but actions that require
on-disk access show a tooltip: "1 of 4 selected projects has a missing path
and will be skipped."  The action proceeds on the remaining 3.

## Internal Design

### AppState changes

```rust
// state/mod.rs
pub struct AppState {
    // ... existing fields ...

    /// Currently selected project IDs within the active workspace.
    /// Cleared on workspace switch (see RFC-008 prune symmetry).
    pub selection: std::collections::HashSet<ProjectId>,

    /// Last project ID toggled in the selection — used as the anchor
    /// for range-select operations. None at startup or after a clear.
    pub selection_anchor: Option<ProjectId>,

    /// Currently focused project (for keyboard navigation).
    /// Independent of selection.
    pub focused_project: Option<ProjectId>,
}
```

### Messages

```rust
// message.rs
pub enum SelectionMessage {
    /// Toggle a single project's selection state.
    Toggled(ProjectId),
    /// Toggle without affecting other selections (ctrl+click semantics).
    /// Functionally identical to Toggled but kept distinct for the anchor.
    ToggledOne(ProjectId),
    /// Range-select from the anchor to the given project.
    RangeTo(ProjectId),
    /// Select every project in the active workspace.
    SelectAll,
    /// Clear the selection.
    Clear,
    /// Focus moved to a different project (no selection change).
    Focused(ProjectId),
    /// Focus moved by relative offset (e.g., arrow key).
    FocusMoved(FocusDirection),
}

pub enum FocusDirection {
    Up,
    Down,
    PageUp,    // future
    PageDown,  // future
}
```

`SelectionMessage` becomes a new variant of the top-level `Message`.

### Update handler

```rust
fn handle_selection(state: &mut AppState, msg: SelectionMessage) -> Task<Message> {
    match msg {
        SelectionMessage::Toggled(id) => {
            if !state.selection.remove(&id) {
                state.selection.insert(id.clone());
                state.selection_anchor = Some(id);
            }
            Task::none()
        }
        SelectionMessage::RangeTo(id) => {
            let Some(anchor) = state.selection_anchor.clone() else {
                // No anchor: behave as Toggled.
                state.selection.insert(id.clone());
                state.selection_anchor = Some(id);
                return Task::none();
            };
            let visible_ids = visible_project_ids_in_tier_order(state);
            select_range(&mut state.selection, &visible_ids, &anchor, &id);
            Task::none()
        }
        SelectionMessage::SelectAll => {
            if let Some(ws) = &state.workspace {
                state.selection.extend(ws.projects.iter().map(|p| p.id.clone()));
            }
            Task::none()
        }
        SelectionMessage::Clear => {
            state.selection.clear();
            state.selection_anchor = None;
            Task::none()
        }
        SelectionMessage::Focused(id) => {
            state.focused_project = Some(id);
            Task::none()
        }
        SelectionMessage::FocusMoved(dir) => {
            move_focus(state, dir);
            Task::none()
        }
        SelectionMessage::ToggledOne(id) => {
            if !state.selection.remove(&id) {
                state.selection.insert(id.clone());
                state.selection_anchor = Some(id);
            } else if state.selection_anchor == Some(id.clone()) {
                state.selection_anchor = state.selection.iter().next().cloned();
            }
            Task::none()
        }
    }
}
```

### View — selection bar

A new view function `view::selection_bar(state) -> Option<Element<Message>>`:

```rust
pub fn selection_bar(state: &AppState) -> Option<Element<'_, Message>> {
    if state.selection.is_empty() {
        return None;
    }

    let count = state.selection.len();
    let any_has_upstream = selection_summary(state).any_has_upstream;
    let all_clean        = selection_summary(state).all_clean;

    let fetch_btn   = button(text(state.t("action.fetch"))).on_press(
        Message::Selection(SelectionMessage::ApplyAction(BulkAction::Fetch))
    );
    let pull_btn    = button(text(state.t("action.pull"))).on_press_maybe(
        if any_has_upstream { Some(Message::Selection(
            SelectionMessage::ApplyAction(BulkAction::Pull))) }
        else { None }
    );
    let tag_btn     = button(text(state.t("action.tag"))).on_press_maybe(
        if all_clean { Some(Message::Selection(
            SelectionMessage::ApplyAction(BulkAction::Tag))) }
        else { None }
    );
    let switch_btn  = button(text(state.t("action.switch")));
    let overflow    = button(text("⋯"));
    let clear       = button(text("✕")).on_press(
        Message::Selection(SelectionMessage::Clear));

    Some(row![
        text(state.t_count("selection.count", count)),
        fetch_btn, pull_btn, tag_btn, switch_btn, overflow, clear,
    ].spacing(8).padding(12).into())
}
```

### View — checkbox on card

The existing project card gains a leading slot for the checkbox.  When
`state.selection` is empty and the card is not focused, the checkbox is
hidden (or shown faded on hover, depending on density mode).

```rust
fn card_checkbox<'a>(state: &AppState, project_id: &ProjectId) -> Element<'a, Message> {
    let is_selected = state.selection.contains(project_id);
    let is_focused  = state.focused_project.as_ref() == Some(project_id);
    let visible     = !state.selection.is_empty() || is_focused;

    if visible {
        checkbox("", is_selected)
            .on_toggle({
                let id = project_id.clone();
                move |_| Message::Selection(SelectionMessage::Toggled(id.clone()))
            })
            .into()
    } else {
        Space::with_width(20).into()  // reserved space so hover doesn't reflow
    }
}
```

### Subscription — keyboard

The existing keyboard subscription gains new bindings (full keyboard layout
detailed in RFC-016):

```rust
keyboard::on_key_press(|key, modifiers| match (key, modifiers) {
    (Key::Named(Named::Space), Modifiers::NONE) =>
        focused().map(|id| Message::Selection(SelectionMessage::Toggled(id))),
    (Key::Named(Named::Space), Modifiers::SHIFT) =>
        focused().map(|id| Message::Selection(SelectionMessage::RangeTo(id))),
    (Key::Character(c), m) if c == "a" && m == Modifiers::CTRL =>
        Some(Message::Selection(SelectionMessage::SelectAll)),
    (Key::Named(Named::Escape), Modifiers::NONE) if !selection_empty() =>
        Some(Message::Selection(SelectionMessage::Clear)),
    (Key::Named(Named::ArrowUp), Modifiers::NONE) =>
        Some(Message::Selection(SelectionMessage::FocusMoved(FocusDirection::Up))),
    (Key::Named(Named::ArrowDown), Modifiers::NONE) =>
        Some(Message::Selection(SelectionMessage::FocusMoved(FocusDirection::Down))),
    _ => None,
})
```

### BulkAction enum (placeholder)

```rust
pub enum BulkAction {
    Fetch,
    Pull,
    Tag,
    Switch,
    GenerateChangelog,
    OpenInTerminal,
}
```

In v0.12, the action handler routes each action to the **existing screen**
with the selection pre-filled.  This keeps the change minimal while
delivering the new interaction pattern.

```rust
SelectionMessage::ApplyAction(BulkAction::Tag) => {
    // Phase 1 (v0.12): route to existing Freezer screen with selection.
    state.freezer.project_selection = state.selection.iter()
        .map(|id| (id.clone(), true)).collect();
    state.screen = Screen::Freezer;
    Task::none()
}
```

In v0.14 (RFC-013), the action opens a modal instead of switching screens.

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.12   | Selection state, checkboxes on cards, selection bar UI, action routing to existing screens |
| 2     | v0.13   | Tier-aware range select (RFC-010 prerequisite) |
| 3     | v0.14   | Actions open modals instead of routing to screens (RFC-013) |

## Test Plan

### Unit tests (`crates/knotra-app/src/tests.rs`)

1. **`selection_toggle_adds_and_removes`** — toggle one ID, assert present;
   toggle again, assert absent.
2. **`selection_range_inclusive`** — given visible order A B C D E and
   anchor B, range-to D selects {B, C, D}.
3. **`selection_range_without_anchor_acts_as_toggle`** — clear anchor, range-to X
   adds X and sets anchor to X.
4. **`select_all_includes_only_active_workspace`** — workspace has projects
   A B; switching to another workspace with C D and select-all yields {C, D}.
5. **`workspace_switch_clears_selection`** — assert selection empty after
   `WorkspaceSwitched`.
6. **`project_removal_drops_from_selection`** — remove a selected project;
   assert selection no longer contains it.
7. **`focus_arrow_down_skips_collapsed_tier`** — when Clean tier is
   collapsed, arrow-down past last Active card focuses first Clean card
   only if expanded (otherwise stays at last Active).
8. **`pull_button_disabled_when_no_upstream`** — selection contains 2
   projects, neither with upstream; `selection_summary().any_has_upstream`
   is false.

### Integration / interaction tests

Not feasible without a UI harness; reserved for future Wasm-based test
infrastructure.

### Manual test plan

1. Click 3 checkboxes → bar shows "3 selected" with all action buttons
   enabled.
2. Switch workspaces → bar disappears, selection cleared.
3. `Ctrl+A` → all projects selected; bar count matches workspace project
   count.
4. `Shift+click` on card 5, then `Shift+click` on card 2 → all cards 2–5
   selected.
5. Focus a card with arrow keys, press space → that card is selected.

## Open Questions

### Q1 — Modal/dialog interaction with selection

When a modal is open (e.g., Settings), should keyboard shortcuts that affect
selection (`ctrl+a`, `escape`) still trigger?  **Tentative answer**: no.
The modal absorbs all keyboard events until closed.

### Q2 — Persisting selection to disk?

Should `selection` be saved to disk so a knotra restart preserves the user's
selection?  **Tentative answer**: no.  Selection is ephemeral; a restart
resets to nothing selected.  Users who want a persistent set can use the
existing workspace concept.

### Q3 — Visual density of checkboxes

Checkboxes always-visible vs. visible-on-hover.  **Tentative answer**:
always visible once selection is non-empty; hidden otherwise (a single
selection state is the trigger).  Settings option to override: "Always show
checkboxes."

## Security Considerations

None.  All operations are local-only.  Selection is in-memory.
