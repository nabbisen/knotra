# RFC-051 - Overlay widths scale with the window

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | Medium - proportion and comfort, plus one overlay that is measurably too narrow for its content |
| Effort | Medium - a `knotra-ui` signature change, one new state field, seven call sites |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-ui/src/widget/overlay.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/view/overlays/*.rs`, `crates/knotra-app/src/view/workspace_manager.rs` |
| Related RFCs | `rfcs/done/035-...md` (**R8** - the responsive mechanism this reuses), `rfcs/done/034-...md` (`OverlayWidth`'s introduction) |
| Owner decisions | Both settled 2026-08-13: flexible widths, and the conflict overlay's size |

## Summary

`OverlayWidth` maps to three fixed pixel constants that ignore the window entirely. Make
them scale, and give the conflict overlay the width its content now needs.

## Problem

### Three constants, no window

`surface()` applies `Length::Fixed(width.pixels())` (`overlay.rs:99`), where `pixels()` is
`Small => 400.0`, `Standard => 520.0`, `Large => 680.0`.

The window ranges from a **800×600 minimum** (`main.rs:36`) to whatever the display
allows. At 800px a `Large` overlay leaves 60px of margin per side; at 2560px it is a small
island in a large window. Neither is broken — **this is proportion, not a defect**, and
the RFC should not pretend otherwise.

The mechanism to fix it already exists and overlays simply do not use it: RFC-035 R8's
`Message::WindowResized` → `state.width_mode`, which the dashboard and selection bar
already consult.

### The conflict overlay is measurably too narrow

`overlays/conflict.rs:293` requests `OverlayWidth::Small` — **400px** — and each
conflicted-file row is now:

```
22px icon + Fill(path) + 8px + [Open in editor] + [Open in comparison tool] + [third slot]
```

Three labelled controls need roughly 300px together, leaving the path about **70px**. The
third slot may also hold the Jujutsu hint (`Finish with: \`jj resolve <path>\``), which is
prose, not a button. 400px was chosen when the row had one control; RFC-043's merge-tool
control and RFC-045's hint arrived since.

## Non-goals

- Changing `WidthMode`'s breakpoints or the dashboard's use of them.
- Per-overlay bespoke widths. This RFC changes what `Small`/`Standard`/`Large` *mean*;
  which overlay picks which stays a separate judgement.
- Height. `surface()`'s body cap is untouched.
- Making overlays resizable by the user.

## Design

### D1. `AppState` stores the window width, not only the derived band

`Message::WindowResized` currently computes `WidthMode::from_width(size.width)` and
**discards the width** (`app.rs:240-242`). Store both, set in that one handler, seeded
from `INITIAL_WINDOW_SIZE` (`state.rs:41`) exactly as `width_mode` already is.

`width_mode` stays as it is - the dashboard's contract does not change.

**The two must agree.** A test asserts `WidthMode::from_width(state.window_width) == state.width_mode`
after a resize - the pairing-guard shape RFC-038's `label_en` needed, cheap here because
both are set on the same line.

### D2. `OverlayWidth` becomes a fraction with clamps

`pixels()` takes the available width and returns
`(fraction * available).clamp(min, max)`. Exact numbers are the implementer's to propose.

**The constraint that matters**: at `INITIAL_WINDOW_SIZE` (1100px) the results must land
within a stated tolerance of today's **400 / 520 / 680**, so the default window looks
unchanged. A responsive system whose first visible effect is that everything moved is a
worse outcome than the fixed constants it replaced.

Below the 800px minimum nothing needs handling - the window cannot get there.

### D3. `surface()` takes the available width

`surface(…, width: OverlayWidth)` gains a parameter. Seven call sites pass
`state.window_width`; the compiler finds them all.

**`knotra-ui` is published** (full crates.io metadata, no `publish = false`), so this is a
**breaking change to a published API**. knotra is its only consumer, so the cost is
bookkeeping - but it is a decision, not a side effect, and it belongs in the release notes
for that crate.

### D4. The conflict overlay becomes `Large` (owner-approved)

One line, `conflict.rs:293`. At the 800px minimum window that yields whatever D2's clamp
floor gives - which must leave the file path a usable share after three controls. **Report
the arithmetic at 800px**, not just the constant.

## Requirements

| # | Requirement |
|---|---|
| R1 | `AppState` carries the window width, seeded from `INITIAL_WINDOW_SIZE` and updated by `Message::WindowResized` |
| R2 | A test asserts the stored width and `width_mode` agree after a resize |
| R3 | At 1100px, the three widths are within a stated tolerance of 400 / 520 / 680, and the tolerance is justified |
| R4 | Widths are clamped; no window size produces an overlay wider than the window or narrower than its floor |
| R5 | `conflict.rs` requests `Large`; the file-row arithmetic at an 800px window is reported |
| R6 | `WidthMode`'s breakpoints and the dashboard's behaviour are unchanged |
| R7 | `knotra-ui`'s signature change is noted for that crate's release notes |
| R8 | `crates/knotra-vcs` is not modified |

## Test Plan

Co-located, and all pure arithmetic - no rendering required, which is the point of
computing width from a number rather than measuring a widget:

- R3's tolerance at 1100px, per variant.
- R4's clamps at 800px and at a large width (2560px).
- R2's pairing.

## Security Considerations

None.

## Migration / rollout

No data or config change. At the default window size users should see no meaningful
difference (R3). Narrow windows get proportionally smaller overlays; wide windows get
larger ones. The conflict overlay is visibly wider, which is the point.
