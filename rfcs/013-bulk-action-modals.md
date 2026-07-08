# RFC-013 — Bulk Action Modals

| Field          | Value                                                                |
|----------------|----------------------------------------------------------------------|
| Status      | **Implemented** (v0.12.0)         |
| Priority       | **High** — the structural change that removes most screens           |
| Effort         | Large — five modals, plan/confirm/execute flows, result rendering    |
| Target version | v0.14                                                                |
| Related        | RFC-009 (selection), RFC-010 (tiers), RFC-011 (activity strip)    |

## Summary

Replace the Sync Center, Freezer, Context Operations, Conflict Resolution,
and Changelog screens with **modal dialogs** opened from the selection bar
(RFC-009).  Each modal handles one workflow (Pull / Tag / Switch /
Resolve / Generate Changelog) from plan to confirmation to execution to
result.  When the modal closes, the user is back on the Dashboard with
updated state visible.

## Background

The current UI has five screens that each implement the same logical
pattern:

```
plan → confirm → execute → result → return to dashboard
```

Each screen replicates a project list, navigation buttons, status display,
and result rendering.  The pattern is right; the **placement** is wrong.
Modals over the Dashboard preserve the user's context, eliminate the
"navigate back" step, and give the result visibility immediately on the
same screen the user was working on.

This RFC delivers all five modals.  It is the **largest structural change**
in the redesign and is a single RFC because:

- The five modals share a common shell (overlay, headers, footer, action
  buttons).
- The plan/confirm/execute pattern is the same.
- They unblock removing five sidebar entries.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | Each modal opens over the Dashboard; clicking outside it does NOT close it (prevents accidental dismissal during destructive operations) |
| R2  | Each modal has a close affordance (`✕` top-right and `Esc`) |
| R3  | Each modal shows the projects involved at the top, with per-project status icons |
| R4  | Plan modals show what will happen before execution |
| R5  | Result modals show what happened, including per-project success / failure and any recovery hints |
| R6  | Execution runs asynchronously; UI shows progress without blocking |
| R7  | All five modals are reachable from the selection bar (RFC-009) AND from the command palette (RFC-012) |
| R8  | All five modals are reachable from the inline buttons on Needs Attention cards (RFC-010) for relevant actions |
| R9  | Modal state survives accidental window unfocus and refocus |
| R10 | Closing a result modal does NOT undo the operation |

## External Design

### Shared modal shell

Every bulk modal uses the same shell:

```
┌─────────────────────────────────────────────────────────────────┐
│ Modal title                                              [✕]    │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│  [phase-specific content]                                        │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│                              [Cancel]  [Primary action]          │
└─────────────────────────────────────────────────────────────────┘
```

- Width: 720 px (centered, semi-overlay above dashboard with dim backdrop).
- Vertical: 15% from top.
- Backdrop: 50% opacity dark layer; blocks clicks on dashboard.
- `Esc` triggers Cancel.

### Modal A — Smart Pull

#### Plan phase

```
┌─────────────────────────────────────────────────────────────────┐
│ Smart pull 4 projects                                     [✕]   │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│   Project          Branch    Behind  Plan                        │
│   ──────────────   ───────   ──────  ────────────────────        │
│   ☑ alpha          main         3    Fast-forward                │
│   ☑ beta           main         1    Fast-forward                │
│   ☑ gamma          feature-x    2    Stash → pull → pop          │
│   ☑ delta          main         0    No change needed (skip)     │
│                                                                  │
│   ⚠ delta is up to date but uncommitted changes will not be     │
│     touched.                                                     │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│                              [Cancel]   [Execute 4 pulls]        │
└─────────────────────────────────────────────────────────────────┘
```

- Each row is checkable; user can opt out per project.
- The plan column shows the disposition: `Fast-forward` / `Stash → pull → pop`
  / `Skip (no upstream)` / `Skip (conflict pending)`.
- Warnings appear inline below the table.
- Primary action label reflects checked count: "Execute N pulls."

#### Executing phase

```
│  Project          Plan                       Result                │
│  ──────────────   ─────────────────────────  ─────────────────────│
│  alpha            Fast-forward               ⟳ Pulling…           │
│  beta             Fast-forward               ✓ +1 commit           │
│  gamma            Stash → pull → pop         ⟳ Stashing…           │
│  delta            Skip                       — skipped              │
```

Rows update live as each project completes.

#### Result phase

```
│  Project          Plan                       Result                │
│  ──────────────   ─────────────────────────  ─────────────────────│
│  alpha            Fast-forward               ✓ +3 commits          │
│  beta             Fast-forward               ✓ +1 commit           │
│  gamma            Stash → pull → pop         ✓ +2 commits, popped  │
│  delta            Skip                       — skipped              │
│                                                                    │
│  All 3 pulls succeeded.                                            │
│                                                                    │
│ ─────────────────────────────────────────────────────────────── │
│                                                  [Close]          │
```

On failure:

```
│  alpha            Stash → pull → pop         ✗ stash pop failed    │
│                                              Stash entry remains.  │
│                                              [Show recovery]       │
```

`[Show recovery]` expands an inline section with the suggested commands and
links to documentation.

### Modal B — Tag (replaces Freezer screen)

#### Plan phase

```
┌─────────────────────────────────────────────────────────────────┐
│ Tag 4 projects                                            [✕]   │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│  Tag name:       [ v1.2.3                                ]      │
│  Message:        [                                       ]      │
│                  (optional — leave empty for lightweight tag)    │
│                                                                  │
│  Projects:                                                       │
│   ☑ alpha       main · clean                                    │
│   ☑ beta        main · clean                                    │
│   ☑ gamma       feature-x · ⚠ 1 dirty file                      │
│   ☑ delta       main · clean                                    │
│                                                                  │
│  Topology warnings:                                              │
│   ⚠ gamma depends on alpha. Tag alpha before gamma if order      │
│     matters.                                                      │
│                                                                  │
│  Blockers:                                                       │
│   ✗ gamma — uncommitted changes                                  │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│                              [Cancel]   [Tag 3 projects]         │
└─────────────────────────────────────────────────────────────────┘
```

- The name field validates inline (no `..`, no spaces, no leading `-`).
- The message field is optional; non-empty → annotated tag (RFC-005).
- Per-project blockers disable the corresponding row and reduce the count
  in the primary button.
- Topology warnings (RFC-007) appear inline.

#### Execution → result phase

Atomic per project (existing behaviour preserved); rollback on failure.
Result phase mirrors the Smart Pull result with recovery hints.

After success, an inline banner appears:

```
│  All 3 tags created.                                              │
│                                                                   │
│  [ Push tags to remote ]   ← optional follow-up action           │
```

### Modal C — Switch Branch / Changeset

```
┌─────────────────────────────────────────────────────────────────┐
│ Switch 4 projects                                         [✕]   │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│  Target:  [ feature-x                          ▼ ]              │
│                                                                  │
│  Branches available in all 4 projects:                           │
│   - main                                                          │
│   - develop                                                       │
│   - feature-x                                                     │
│                                                                  │
│  Projects:                                                       │
│   ☑ alpha       main → feature-x   (clean — ok)                 │
│   ☑ beta        main → feature-x   (clean — ok)                 │
│   ✗ gamma       main → feature-x   ⚠ uncommitted changes        │
│   ☑ delta       main → feature-x   (clean — ok)                 │
│                                                                  │
│  Blocked: 1 project has uncommitted changes.                     │
│   [Stash and switch all]   [Skip gamma]                          │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│                              [Cancel]   [Switch 3 projects]      │
└─────────────────────────────────────────────────────────────────┘
```

- The dropdown lists only branches present in **all** selected projects.
  An "Other…" entry opens a text input for free-form names.
- Blocked rows show the reason; user picks resolution strategy.

For jj projects in the selection, the target is a changeset (revset
identifier) and the dropdown shows recent bookmarks plus `@-` etc.

### Modal D — Resolve Conflict

Unlike the others, this modal opens for **one project at a time** and is
triggered from a Needs Attention card's `[Resolve…]` button.  It is more
of a side panel than a centered modal: it docks to the right of the
window.

```
┌─── alpha · Conflict on main ──────────────────[✕]──┐
│                                                      │
│ 3 conflicted files                                   │
│ ──────────────────────────────────────────────────  │
│ src/main.rs                                          │
│   UU  Both modified                                  │
│   [Open in editor]  [Open merge tool]  [Mark resolved]│
│                                                      │
│ src/lib.rs                                           │
│   UU  Both modified                                  │
│   [Open in editor]  [Open merge tool]  [Mark resolved]│
│                                                      │
│ tests/conflict_test.rs                               │
│   AA  Both added                                     │
│   [Open in editor]  [Open merge tool]  [Mark resolved]│
│                                                      │
│ ──────────────────────────────────────────────────  │
│  [Re-check]                          [Abort merge]   │
└──────────────────────────────────────────────────── ┘
```

- Width: 480 px, full window height.
- Replaces the current ConflictResolution screen.
- The dashboard remains visible to the left; other cards remain
  interactive.
- The panel auto-closes when the last file is resolved.

### Modal E — Generate Changelog

```
┌─────────────────────────────────────────────────────────────────┐
│ Generate changelog for 4 projects                         [✕]   │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│  Since reference:  [ v1.2.0                  ▼ ]                │
│                    (or type any ref: branch / tag / commit)      │
│                                                                  │
│  Format:           ( ) Markdown                                  │
│                    ( ) Plain text                                │
│                    (•) HTML                                      │
│                                                                  │
│  Projects:                                                       │
│   ☑ alpha       3 commits since v1.2.0                          │
│   ☑ beta        1 commit  since v1.2.0                          │
│   ☑ gamma       0 commits since v1.2.0                          │
│   ☑ delta       8 commits since v1.2.0                          │
│                                                                  │
│  Preview:                                                        │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │ # Changelog for 4 projects since v1.2.0                   │  │
│   │ ## alpha (3 commits)                                       │  │
│   │ - abc123 Fix race condition in pool init                  │  │
│   │ - def456 Update CI matrix to 1.91                         │  │
│   │ ... (50 line preview)                                      │  │
│   └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│  [Cancel]   [Copy to clipboard]   [Save as file…]                │
└─────────────────────────────────────────────────────────────────┘
```

- The since-ref dropdown is populated from the union of tags + bookmarks
  across selected projects; free-form input also accepted.
- The preview updates live as the reference changes.
- The format selector affects rendering of the preview and the export.

## Internal Design

### Modal type system

```rust
// state/modal.rs
pub enum Modal {
    None,
    Pull(PullModal),
    Tag(TagModal),
    Switch(SwitchModal),
    Resolve(ResolveModal),    // docked right; technically a panel
    Changelog(ChangelogModal),
}

pub struct PullModal {
    pub phase: ModalPhase,
    pub plan:  SmartPullPlan,           // existing type
    pub result: Option<SmartPullResult>,
}

pub enum ModalPhase {
    Planning,     // gathering info, user can change parameters
    Executing,    // in flight; progress visible
    Done,         // complete; show result
}
```

`AppState` gets a `pub modal: Modal` field.  When `Modal != None`, the
dashboard renders the modal as an overlay on top.

### Single open modal invariant

Only one modal is open at a time, except: the Resolve panel (docked right)
may co-exist with a centered modal because they don't overlap visually.

The Resolve panel uses a separate field:

```rust
pub struct AppState {
    pub modal: Modal,          // centered modals
    pub resolve_panel: Option<ResolveModal>,  // docked panel
}
```

### Messages

```rust
pub enum ModalMessage {
    Closed,

    // Pull modal
    PullPlanGenerated(SmartPullPlan),
    PullProjectToggled(ProjectId, bool),
    PullExecuteRequested,
    PullProgress(ProjectId, ProjectOperationResult),
    PullCompleted(SmartPullResult),

    // Tag modal (mirrors Freezer)
    TagNameChanged(String),
    TagMessageChanged(String),
    TagProjectToggled(ProjectId, bool),
    TagValidationDone(FreezeValidation),
    TagExecuteRequested,
    TagCompleted(FreezeResult),

    // Switch modal
    SwitchTargetChanged(String),
    SwitchProjectToggled(ProjectId, bool),
    SwitchStashAndExecuteRequested,
    SwitchExecuteRequested,
    SwitchCompleted(Vec<ProjectOperationResult>),

    // Resolve panel
    ResolveOpened(ProjectId),
    ResolveFileClicked(String, ResolveAction),
    ResolveRecheckRequested,
    ResolveAbortRequested,
    ResolveCompleted(ProjectConflictDetail),

    // Changelog modal
    ChangelogSinceChanged(String),
    ChangelogFormatChanged(ChangelogFormat),
    ChangelogProjectToggled(ProjectId, bool),
    ChangelogCopyRequested,
    ChangelogSaveRequested,
    ChangelogPreviewReady(String),
}

pub enum ResolveAction {
    OpenInEditor,
    OpenMergeTool,
    MarkResolved,
}
```

### View dispatcher

```rust
// view/mod.rs
pub fn view(state: &AppState) -> Element<'_, Message> {
    let main = dashboard::view(state);

    let mut layers: Vec<Element<Message>> = vec![main];

    // Right-docked resolve panel.
    if let Some(panel) = &state.resolve_panel {
        layers.push(resolve_panel_view(state, panel));
    }

    // Centered modal.
    if !matches!(state.modal, Modal::None) {
        layers.push(modal_backdrop());
        layers.push(modal_view(state));
    }

    // Palette is on top.
    if state.palette.open {
        layers.push(palette::overlay(state).unwrap());
    }

    stack(layers).into()
}
```

`iced::widget::stack` (available in iced 0.14) is used to layer overlays.

### Reusing existing logic

These modals are **not** rewrites of the screen logic; they wrap the
existing `state/sync.rs`, `state/freezer.rs`, `state/context.rs`,
`state/conflict_ops.rs`, and `state/changelog.rs` modules.  The data
structures and operations remain unchanged.  Only the rendering changes.

For example, `state/freezer.rs::FreezerState` becomes
`state::modal::TagModal::inner: FreezerState`.  The view code is mostly
copied from `view/freezer.rs` with the layout adjusted to a modal.

### Modal-to-screen routing (deprecated path)

During the v0.14 migration period, the existing screens remain reachable
via the command palette ("Open Sync Center", etc.) for users who prefer
them.  In v0.16 (RFC-017), they are removed.

### Card inline action wiring

Needs Attention cards have buttons that previously navigated to screens.
They now open modals directly:

```rust
// In Needs Attention card view
button(text(state.t("attention.action.resolve")))
    .on_press(Message::Modal(ModalMessage::ResolveOpened(project_id.clone())))
```

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.14   | All five modals implemented; selection bar routes to them; old screens still reachable via palette / sidebar |
| 2     | v0.15   | Sidebar entries removed (RFC-017 dependency); palette is the only way to reach the old screens |
| 3     | v0.16   | Old screens removed entirely; modals are the only path |

## Test Plan

### Unit tests

For each modal:

1. **`modal_opens_with_correct_selection`** — selection has 4 projects;
   `Tag` action opens TagModal with those 4 projects.
2. **`modal_dismisses_on_escape`** — open Pull modal; send Esc message;
   modal becomes None.
3. **`modal_blocks_outside_clicks`** — clicks on dashboard area while
   modal is open are absorbed by the backdrop.
4. **`modal_progress_updates_live`** — three async pull operations; status
   per project updates as each completes.
5. **`modal_recovery_shows_when_failed`** — single pull fails; result phase
   includes the recovery section.

### Integration

1. **`pull_modal_end_to_end`** — open, plan, execute on a test fixture
   workspace with 3 ahead/1 dirty, verify per-project results.
2. **`tag_modal_rollback_on_failure`** — simulate a tag failure on project
   2 of 3; verify rollback of projects 1 and 3 (already in existing
   integration suite for Freezer).
3. **`resolve_panel_closes_on_all_resolved`** — open with 3 conflicts;
   mark all 3 resolved; panel auto-closes; project moves out of Needs
   Attention tier.

### Manual

For each modal: open from selection bar, from palette, from card inline
button (where applicable).  All three entry points open the same modal in
the same state.

## Open Questions

### Q1 — Modal state during workspace switch

If a Pull modal is open and the user switches workspaces, what happens?
**Tentative answer**: the modal closes (operation in progress is allowed
to complete; result is visible via History after closing).  An in-flight
operation must complete before any UI dismissal commits.

### Q2 — Single-project actions

A card's `[Open in editor]` button is not a "modal" — it's a single
synchronous action.  How does that fit the model?  **Tentative answer**:
single-action buttons remain as direct messages; only multi-step workflows
become modals.

### Q3 — Resolve panel and modal co-existence

If a user opens the Resolve panel for project alpha, then opens the Pull
modal (via selection bar), what happens?  **Tentative answer**: both
visible.  The centered modal takes input focus; the Resolve panel is
visible but inert until the modal closes.  This is fine because each
controls different projects.

### Q4 — Closing during execute

Can the user close a modal while execution is in flight?  **Tentative
answer**: no for destructive operations (Tag, Switch, Resolve).  Yes for
read-only operations (Changelog generation can be cancelled).  The close
button is disabled during destructive execution.

## Security Considerations

Modals do not introduce new attack surface; they only re-package existing
operations.  Same execution paths, same logging.
