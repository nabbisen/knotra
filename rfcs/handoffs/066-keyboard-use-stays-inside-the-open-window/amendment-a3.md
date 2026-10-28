# RFC-066 Handoff — Amendment A3: the R2 guard covers every layer and phase

Adds two items to `implementation-handoff.md`, **Stage 3**. From Review 234. Everything else
stands.

## 1. R2 enumerates, not samples

Stage 2's `every_risky_entry_never_starts_a_change_r2` checks the phases where an entry
*could* land on a change. The others are argued safe "by construction": a text field, or a
bare Close, Cancel or Keep. That is true of today's code. The test exists to fail when
tomorrow's code changes, for example when someone reorders a footer or adds a control to a
phase. An argument cannot do that.

Change it to enumerate:

- **One table** in the test: every `Layer`, and for each, every phase the test can
  construct. Use one state builder per row, reusing Stage 2's builders.
- **For each row:** compute `layer_focus` and take `entry`. If it is `Some`, find its
  activation message in `order` and assert it is not one of the change-starting messages
  (the one list Stage 2 already keeps).
- **Coverage check:** for each `Layer` variant, assert at least one row exists, through an
  exhaustive `match` on `Layer` in the test. A new layer then cannot be added without a row.
- Keep the instruction to add any new change-starting message to the list.

## 2. Remove project's buttons: confirm the look

Stage 2 moved "Keep" and "Remove from list" off snora's `ghost_maybe` and `danger_maybe` and
onto a local `styled_button`. The sizes were not compared. Stage 3 redraws this confirmation as
a dialog (D6) anyway. In its evidence, the D6 capture is the check: report whether the two
buttons match the other dialogs' buttons in height and padding.
