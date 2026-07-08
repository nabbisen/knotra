# RFC-011 — Activity Strip

| Field          | Value                                                              |
|----------------|--------------------------------------------------------------------|
| Status      | **Implemented** (v0.12.0)         |
| Priority       | Medium — improves outcome visibility                               |
| Effort         | Small–Medium — new bottom widget, minor state additions            |
| Target version | v0.12                                                              |
| Related        | RFC-009 (selection bar), `state/mod.rs`, `view/mod.rs`            |

## Summary

Add a single-line strip at the bottom of the main window showing the most
recent operation's outcome.  Clicking it expands a popover with the last
20 operations.  Replaces the History sidebar item for the common case of
"what just happened" and keeps the operation log discoverable.

## Background

Operation logs are currently stored, persisted, and rendered on the
History screen — a separate navigation destination.  After a bulk
operation, the user wants to immediately see "did it work?" — but the
result lives in a screen they have to navigate to.

The activity strip surfaces the latest operation's summary inline at the
bottom of the dashboard.  History becomes the strip's expanded popover.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | A single-line strip occupies the bottom edge of the main window |
| R2  | The strip shows the most recent operation's kind + result summary |
| R3  | During an in-progress operation, the strip shows a progress bar across its width |
| R4  | Clicking the strip opens a popover with the last 20 operations |
| R5  | Failed operations colour the strip red; partial successes yellow |
| R6  | A "Retry failed" inline button appears when the latest operation had any failed projects |
| R7  | The strip auto-collapses (fades to muted color) after 30 seconds |
| R8  | The strip is hidden entirely when no operations have ever run in the current session |
| R9  | The popover supports the same actions as today's History screen: copy log to clipboard (RFC-001), expand entry, see commands executed |
| R10 | The History screen remains accessible via the command palette for users who prefer a full-screen view |

## External Design

### Visual states

#### Idle (no operations yet this session)

Strip is hidden.  Main view extends to the bottom edge.

#### In progress

```
┌────────────────────────────────────────────────────────────────────┐
│ ⟳ Fetching… 12 of 28 projects                  ▓▓▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒ │
└────────────────────────────────────────────────────────────────────┘
```

- Indeterminate or determinate progress bar based on whether the operation
  has a known total (`12 of 28`).
- Live counter updates as projects complete.

#### Success

```
┌────────────────────────────────────────────────────────────────────┐
│ ⓘ Last: Fetched 28 projects · 28 ok                  2s ago [›]   │
└────────────────────────────────────────────────────────────────────┘
```

- Subtle accent background; fades to muted gray after 30 s.

#### Partial failure

```
┌────────────────────────────────────────────────────────────────────┐
│ ⚠ Last: Fetched 28 projects · 27 ok, 1 failed (alpha) [Retry] [›] │
└────────────────────────────────────────────────────────────────────┘
```

- Yellow accent background.
- `[Retry]` button re-runs the operation on only the failed projects.
- Persists in attention-colour until the next operation or until clicked.

#### Total failure

```
┌────────────────────────────────────────────────────────────────────┐
│ ✗ Last: Tag 'v1.2.3' failed · rolled back · [Show details] [Retry]│
└────────────────────────────────────────────────────────────────────┘
```

- Red accent background.
- Persists until next operation; does not auto-fade.

### Popover (expanded view)

Triggered by clicking `[›]` or the strip itself:

```
┌────────────────────────────────────────────────────────────────────┐
│ Recent activity                                              [✕]   │
│ ───────────────────────────────────────────────────────────────── │
│ ⚠ 2026-05-22 14:32 · Fetched 28 projects · 27 ok, 1 failed   [▶] │
│   alpha                                                            │
│ ✓ 2026-05-22 14:18 · Smart pull · 4 projects ok               [▶]│
│ ✓ 2026-05-22 13:55 · Tag 'v1.2.3' · 8 projects ok             [▶]│
│ ⓘ 2026-05-22 13:42 · Switch to feature-x · 4 projects ok      [▶]│
│ ...                                                                │
│                                                                    │
│ [Open full history]                                                │
└────────────────────────────────────────────────────────────────────┘
```

- Width: half the window, max 600 px, anchored to right.
- Maximum 20 entries; oldest dropped.
- `[▶]` per entry expands an inline detail view (kind/project list/per-
  project status), same content as today's History screen entries.
- `[Open full history]` opens the legacy History screen (kept available).

### Interaction details

| Trigger                                | Effect                                                   |
|----------------------------------------|----------------------------------------------------------|
| Click strip (any state)                | Open popover                                             |
| Click `[Retry]`                        | Re-run the operation on only the failed projects         |
| Click `[Show details]`                 | Open popover and auto-expand the latest entry            |
| Click `[›]`                            | Open popover                                             |
| Click `[✕]` in popover                 | Close popover                                            |
| Click outside popover                  | Close popover                                            |
| `Esc` while popover open               | Close popover                                            |
| `h` (no modifier, with focus on body)  | Toggle popover (see RFC-016)                            |

### Progress display details

The progress bar's segmentation reflects per-project completion when
applicable:

```
▓▓▓▓▓▓▓▒▒▒▒▒▒▒
^^^^^^^^^^^^^^
each block = one project
filled = completed, empty = pending, light = in flight
```

For operations without a per-project breakdown (e.g., a single git fetch
on one project), an indeterminate spinner is shown instead.

### Animation

- Slide up from below when first operation appears (no animation on
  subsequent updates).
- Color transition 250 ms for accent→muted fade.
- No animation on retry click; immediate visual feedback in the strip.

## Internal Design

### New types

```rust
// state/mod.rs
pub struct AppState {
    // ... existing fields ...

    /// The most recently completed or in-progress operation.
    /// None means no operations have run in this session.
    pub latest_operation: Option<LatestOp>,

    /// Whether the activity popover is open.
    pub activity_popover_open: bool,
}

#[derive(Clone, Debug)]
pub struct LatestOp {
    pub id:          OperationId,
    pub kind:        String,        // human label: "Fetched", "Smart pull", "Tag", ...
    pub started_at:  chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub state:       LatestOpState,
}

#[derive(Clone, Debug)]
pub enum LatestOpState {
    /// Currently running.  Optional totals for progress bar.
    Running { done: u32, total: Option<u32> },
    /// All projects succeeded.
    Success { count: u32 },
    /// Some projects failed.
    PartialFailure { ok: u32, failed: Vec<ProjectId> },
    /// The entire operation failed (e.g., tag rollback).
    TotalFailure { reason: String },
}

pub type OperationId = String;
```

### Messages

```rust
pub enum ActivityMessage {
    /// Operation started — populate latest_operation.
    Started { id: OperationId, kind: String },
    /// Per-project progress update during a bulk operation.
    Progress { id: OperationId, done: u32, total: Option<u32> },
    /// Operation completed — write final state.
    Completed { id: OperationId, state: LatestOpState },
    /// Click on the strip / `[›]` — toggle popover.
    PopoverToggled,
    /// Click "Retry" — re-run on failed projects only.
    RetryRequested,
    /// Click "Show details" — open popover + auto-expand latest.
    ShowDetailsRequested,
}
```

### Operation dispatchers (refactor)

Existing operation handlers (`handle_sync`, `handle_freezer`, etc.) currently
push to `state.operation_logs` directly.  Refactor: every handler that runs
an async VCS operation also sends:

```rust
ActivityMessage::Started { id, kind: "Fetched" }
// ... progress events ...
ActivityMessage::Completed { id, state: LatestOpState::Success { count: 28 } }
```

This couples activity strip updates to operation lifecycle automatically.

### View — strip

```rust
// view/activity.rs
pub fn strip<'a>(state: &AppState) -> Option<Element<'a, Message>> {
    let op = state.latest_operation.as_ref()?;
    let body: Element<'a, Message> = match &op.state {
        LatestOpState::Running { done, total } => running_view(state, op, *done, *total),
        LatestOpState::Success { count }       => success_view(state, op, *count),
        LatestOpState::PartialFailure { ok, failed } => partial_view(state, op, *ok, failed),
        LatestOpState::TotalFailure { reason } => total_failure_view(state, op, reason),
    };
    Some(container(body)
        .style(activity_style(&op.state))
        .padding([6, 12])
        .into())
}
```

### Auto-fade

A `Subscription` ticks every 5 seconds; the `style` of the strip is
computed from `(now - finished_at)`:

```rust
fn activity_style(state: &LatestOpState) -> impl iced::widget::container::Catalog {
    match state {
        LatestOpState::TotalFailure { .. } => style_red(),
        LatestOpState::PartialFailure { .. } => style_yellow(),
        LatestOpState::Success { .. } if seconds_since_finish() > 30 => style_muted(),
        LatestOpState::Success { .. } => style_accent(),
        LatestOpState::Running { .. } => style_active(),
    }
}
```

The 5-second tick exists only to drive the fade animation; it is cheap and
only registered when `state.latest_operation` is `Some`.

### History integration

The existing `operation_logs: Vec<OperationLog>` continues to be populated.
The popover renders the last 20 entries from this same list.  The History
screen renders the full list with search.

A new helper `view::activity::popover(state)` reads `state.operation_logs`
directly; no separate data structure.

### Retry mechanics

Each handler stores enough information to retry on the failed subset.  When
a fetch fails on alpha, `LatestOpState::PartialFailure { ok: 27, failed:
[alpha] }` carries the project IDs.  The retry handler:

```rust
ActivityMessage::RetryRequested => {
    let Some(op) = &state.latest_operation else { return Task::none(); };
    let LatestOpState::PartialFailure { failed, .. } = &op.state else { return Task::none(); };
    let ids = failed.clone();
    let kind = op.kind.clone();

    match kind.as_str() {
        "Fetched" => start_bulk_fetch(state, ids),
        "Smart pull" => start_smart_pull(state, ids),
        // ... etc
        _ => Task::none(),
    }
}
```

Each operation kind has a known re-dispatcher.  Operations that cannot be
retried (e.g., a rollback that failed) hide the Retry button.

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.12   | Strip widget; running / success / failure states.  Popover with simple list.  No retry button. |
| 2     | v0.13   | Retry button + per-operation re-dispatchers |
| 3     | v0.14   | Replace History sidebar with command-palette-only access |

History screen is **not** removed.  It is reachable via:
- The activity popover's `[Open full history]` link.
- Command palette: "Open history."
- Direct shortcut: `g h` (RFC-016).

## Test Plan

### Unit tests

1. **`activity_strip_hidden_when_no_ops`** — `latest_operation = None` →
   `strip(state) = None`.
2. **`activity_strip_shows_running_with_total`** — Running { 12, Some(28) }
   → strip contains "12 of 28."
3. **`activity_strip_shows_success_count`** — Success { 28 } → strip
   contains "28 ok."
4. **`activity_strip_shows_partial_failure_with_first_failed_id`** —
   PartialFailure { 27, [alpha, beta] } → strip contains "27 ok, 2 failed."
5. **`retry_dispatches_only_failed_ids`** — given PartialFailure { failed:
   [alpha] }, RetryRequested fires a fetch task with `ids = [alpha]`.
6. **`popover_renders_at_most_20_entries`** — operation_logs has 50
   entries; popover shows 20.

### Manual test plan

1. Run a bulk fetch → strip shows running progress, then success.
2. Disconnect network; run fetch → strip shows partial failure with retry
   button.
3. Click retry → strip resumes running.
4. Wait 35 s after success → strip fades to muted color.
5. Click strip → popover opens.

## Open Questions

### Q1 — Multiple concurrent operations

If a user starts a bulk fetch and then a bulk pull on a different selection
before the first completes, what does the strip show?  **Tentative
answer**: the most recent of any in-progress operation.  Long-term: a
stacked indicator showing all active operations.  For v0.12, only one
operation at a time (UI disables relevant buttons while one is in flight —
already the current behaviour).

### Q2 — Persistence

Should `latest_operation` survive a knotra restart?  **Tentative answer**:
no.  Restarts start with an empty strip.  Operation logs remain in
`history/` on disk.

### Q3 — Light theme readability

Red/yellow accent backgrounds need careful palette work for light theme.
Resolution: defer detailed palette to the design tokens; for v0.12, use
text-color cues + small left-border accent instead of full background fill.

## Security Considerations

None.  Activity strip reads only in-memory state.
