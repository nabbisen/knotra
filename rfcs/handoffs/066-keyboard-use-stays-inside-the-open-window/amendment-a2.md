# RFC-066 Handoff — Amendment A2: a key press that is not a shortcut sends nothing

Adds one item to `implementation-handoff.md`, **Stage 2**. Found while reviewing Stage 1
(Review 233 §3). Everything else stands.

## The defect

`app.rs`'s keyboard subscription maps every keyboard event that is not a shortcut to
`Message::Tick`, through `keyboard::listen().map(...)`'s fallback arm. That includes every key
**release**, every modifier change, and any letter pressed outside a text field. `Tick` is
the auto-refresh message, and `handle_tick` starts `refresh_workspace_task`, a status read of
**every project in the workspace**.

So:

- **Wasted work:** releasing Tab after moving focus starts a full re-check. So does tapping
  Shift.
- **A setting that does not hold:** a user who set the refresh interval to 0, to turn
  automatic refresh off, still gets one on every key.
- **Visible churn:** each refresh sets `load_phase = Refreshing`.

## The fix, in Stage 2

- The keyboard subscription uses `Subscription::filter_map` (iced 0.14,
  `iced_futures/src/subscription.rs:341`), returning `Some(Message::Shortcut(_))` for a
  shortcut and **`None` for everything else**. No event becomes `Tick`.
- `Tick` comes only from the `time::every` timer, as its name says.
- Stage 3's `shortcut_for` keeps this shape: the subscription becomes
  `keyboard::listen().filter_map(|e| … shortcut_for(…).map(Message::Shortcut))`.

**Tests:** extract the event-to-message function so it is testable, and assert it returns
`None` for:

- a key release;
- a modifiers change;
- a letter that is not `j`, `k` or `/`.

**Seen to fail first:** against the current fallback, show that a release maps to `Tick`.

**CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Fixed — every key press re-checked all projects

Pressing or releasing a key that is not a shortcut made knotra re-check the status of
every project, even with automatic checking turned off. Only the timer and "Check now"
start a check now.
```

Report it with Stage 2, as its own commit.
