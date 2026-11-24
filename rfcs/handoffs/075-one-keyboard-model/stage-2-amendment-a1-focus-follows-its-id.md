# RFC-075 stage 2 — Amendment A1: focus follows its id

Review: `.git-exclude/reviewed/299-rfc-075-stage-2-review.md`. Read its F1-F4 first.

**Start now.** Baseline: `origin/main` (`279c0ee` or later). One amendment, one review request.

**No release ships until stage 3 is done** (Review 298 §3). There is no CHANGELOG entry.

**This amendment is immutable.**

## 1. Focus follows the control it is on, by id (F1)

**The rule.** When the page is rebuilt, keyboard focus stays on **the control with the same id**,
wherever that control now is.

- If that control is gone, focus goes to the control after it among those still there, else the
  one before it. That is snora's own rule for its toasts (`keyed.rs`, RFC-120 R-1).
- A focus change the reader makes (Tab, an arrow, a click) or the application makes (`focus(id)`
  from `update`) is never undone.
- It holds for every rebuild, whatever caused it: a refresh, a file-change event, a filter, the
  search field, a sort, a group, a collapse, selection mode, a project added or removed.

**The mechanism is one, not one per message.**

- It is knotra's own, in `knotra-ui`, so stage 3's windows can use it too.
- iced's `keyed_column` cannot do this: its diff handles one inserted or removed run, not a move
  (`iced_core-0.14.0/src/widget/tree.rs:115-187`).
- Do not depend on `snora-interaction`: its doc says it is not for applications.
- snora's `Keyed` (`snora-interaction-0.65.0/src/keyed.rs`) shows one workable shape:
  - remember the focused target at each redraw;
  - when the set of targets changes, put focus back by key.

  Here the key is the control's `Id`. Read its stated limit, a Tab in the very frame the set
  changes, and say whether yours shares it.
- Say where it wraps the tree, and why there.
- The two fixes in `1144026` and `279c0ee` either become unnecessary or stay. Say which, and why.
  Their tests stay either way.

**Also measure, and report:** Enter on "Check now" disables it while the refresh runs.

- Where is focus during the refresh, and after it?
- A Tab pressed during the refresh must not start again from the top of the page.

## 2. No `unsafe` in the tests (F2)

- Remove both transmutes in `keyboard_acceptance.rs`.
- **The borrow checker must prove it.** Change the harness so it never holds a view borrowed from
  the state across a call that mutates that state. For example, it could keep only iced's
  `user_interface::Cache` and the renderer between steps, and take the view at each step.
- The design is yours. Neither a cast nor `Box::leak` is allowed.
- `git grep -n unsafe -- 'crates/*.rs'` then finds only `knotra-vcs/src/process.rs`.

## 3. The row action looks as it did (F3)

- "Choose resolution" and "Show details" draw as before stage 2 at rest: the same box, and the
  label centred with the same padding.
- Find the cause and report it.
- Extend stage 1's pixel comparison to the row action, as stage 2 §5 asked:
  - both labels;
  - English and Japanese;
  - the wide and compact layouts;
  - all four themes.

## 4. The height guard sees `view/` (F4)

- Extend `button_height_guard.rs` to `control(` calls in `crates/knotra-app/src/view/`.
- **Seen failing first,** by the very break you used: `shell.rs`'s `nav_control` without its
  height.

## 5. Tests

| Question | Level | How |
|---|---|---|
| Focus stays on its project when rows move | 1 | through `keyboard_acceptance`'s real path. **Seen failing first on `279c0ee`.** Review 299's case is one: two projects in "Needs help", ↓ to Alpha, a refresh moves Alpha to another group, focus is still on Alpha's name. Add Alpha's row action, a filter, a search, a sort, and selection mode, each with the focused control's row moving |
| Focus goes to the neighbour when its control goes | 1 | the focused project removed: focus is on the next project's same control, else the previous project's |
| The reader's and the app's own moves are never undone | 1 | Tab, ↓ and `/` in a frame where rows also change: focus is where the key put it |
| Check now during a refresh | 1 | §1's measurement as a test |
| No `unsafe` | 1 | `git grep` (§2) |
| The row action's look at rest | 2 | §3's comparison |
| The height guard | 1 | §4, failing first |

No captures: nothing a user sees at rest changes, except §3, whose pixel test is the evidence.

## 6. Report

- §1: the mechanism, where it lives and wraps the tree, its limit, and the two earlier fixes'
  fate.
- §1's Check now measurement.
- §2: the harness's new shape.
- §3: the cause, and the comparison.
- The tests, with failing-first output.
- Gates, with the counts summed.
- **CI's result once the run has finished, with its run id.**
- knotra's status and reflog.
