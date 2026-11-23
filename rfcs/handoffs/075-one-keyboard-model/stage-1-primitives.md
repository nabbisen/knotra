# RFC-075 handoff — Stage 1: the primitives, and a keyboard test harness

RFC: `rfcs/accepted/075-one-keyboard-model.md`, with Amendment A1 (2026-11-24). Read both first,
and stage 0's review, `.git-exclude/reviewed/295-rfc-075-stage-0-review.md`.

**Start now.** Baseline: `origin/main`. One stage, one review request.

**No view changes in this stage.** The new primitives sit beside the old ones. Stages 2–4 move the
views onto them, and stage 5 deletes the old ones. Nothing a user sees changes yet.

**This handoff is immutable.**

## 1. The keyboard test harness (RFC-075 A1, stage 1)

A test-only helper that drives the keyboard through **`iced_runtime`'s `UserInterface`**, as
snora's own `tests/modal_keyboard.rs` does (read it in the registry, `snora-0.65.0/tests/`).
`iced_test`'s `Simulator` cannot run `traverse`'s operations (stage 0, answer f).

**What it can do:**

- build any `Element` at a logical size;
- send key presses: Tab, Shift+Tab, ↓, ↑, Enter, Space, Escape and printable characters;
- run the operations `snora::keyboard::traverse` returns, to completion;
- report which control is focused, **by its `.id()`**, or none;
- collect the messages published.

Put it where every crate's tests can use it. Say where, and why. Prove it with three tests on a
plain snora page:

- Tab moves through three controls in order;
- Shift+Tab goes back;
- a disabled control is skipped.

## 2. The primitives, on snora's controls

New functions in `knotra-ui`, **beside** today's. The names are yours, chosen so the old and the
new cannot be confused. Report them.

| Today (iced underneath) | New (snora underneath) |
|---|---|
| the button styles (`primary`, `secondary`, `ghost`, `danger`, with `BUTTON_HEIGHT` and `SMALL_BUTTON_HEIGHT`) | `snora::design::control` with the same `style::button::*` functions and heights, and `.id()` |
| `chip::filter` | `snora::design::control` with `style::chip::{selected, unselected}`, the same count and badge layout |
| `styled_pick_list` | `snora::design::select`, 44 px tall like today, with the open state passed in and `on_open_change` |
| `checkbox` | `snora::design::checkbox` |
| `styled_text_input` | unchanged: `text_input` is already a Tab stop |

**The rules:**

- **The same look at rest:** the same size, padding, colours and label.
- **The focus ring may differ.** snora draws its own ring from `Tokens::focus`, at least 3:1 by
  snora's guarantee. Report the difference.
- Every new primitive takes an id, for stage 3's entries (RFC-075 D3).
- A label never wraps inside its control: `Wrapping::None`, as Task 117 A7 set.

**Their guards:**

- the label guard;
- the button and field height guards;
- the `pick_list` guard, extended to the new select;
- the every-state contrast suite (Task 112).

Each covers the new primitives as well as the old ones.

## 3. Tests

| Question | Level | How |
|---|---|---|
| The harness drives Tab, arrows and Enter | 1 | §1's three tests |
| Each new primitive looks like its old one at rest | 2 | Render old and new side by side, in all four themes, and compare by machine. **Expect identical pixels at rest.** Explain every difference by crop. Report the focused state's difference, the ring, separately |
| Each new primitive is a Tab stop; Enter and Space press it; disabled is skipped | 1 | through the harness |
| The new select opens on Enter, ↓ moves, Enter chooses (the caller closes it), Escape closes and returns focus | 1 | through the harness |
| The guards and the contrast suite cover the new primitives | 1 | each, **seen failing first** on a deliberately broken new primitive, then passing |

There are no captures: nothing a user sees changes in this stage.

## 4. CHANGELOG

None.

## 5. Report

- the harness: where it lives, and its three tests;
- the primitives' names;
- the pixel comparison, with any difference explained and the ring reported;
- the tests, with failing-first output;
- gates, with the counts summed;
- **CI's result once the run has finished, with its run id;**
- knotra's status and reflog.
