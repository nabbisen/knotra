# RFC-075 handoff — Stage 3: every window, and the page's last two regions

RFC: `rfcs/accepted/075-one-keyboard-model.md`, with Amendment A1. Read first:

- stage 0's inventory, `.git-exclude/evidence/rfc-075/stage-0/inventory.md` §6-§7: every window,
  phase and entry;
- RFC-066 D3 (`rfcs/done/066-keyboard-use-stays-inside-the-open-window.md`);
- Reviews 299-301.

**Start now,** once the owner has approved §6's text (Review 301 §4). Baseline: `origin/main`.
One stage, one review request.

**This stage ends the "no release" period** (Review 298 §3). Its CHANGELOG entry covers stages 2
and 3.

**This handoff is immutable.**

## 1. The page's last two regions (Review 301 §3)

The detail panel and the activity strip become snora controls, with ids in `view/ids.rs`, inside
stage 2's wrap.

| Region | Controls |
|---|---|
| Detail panel (`view/detail_panel.rs`) | Close, Refresh, Check for updates, Remove from workspace |
| Activity strip (`view/activity_strip.rs`) | Undo, Dismiss, Details |

- **The same look at rest.** Compare by pixels, as stages 1 and 2 did.
- **Opening the panel from the keyboard** (Enter on a project's name) moves focus to the panel's
  Close. Close is safe, and the panel sits at the end of the page, many Tabs away.
- **Opened by a click,** focus does not move.
- **Closing the panel** returns focus to the name that opened it. If that row is gone, focus goes
  to its neighbour, by A1's rule.
- Any of these controls that disables itself when pressed joins `reserved`. List each one.

## 2. Every window on snora's controls

The windows are listed in inventory §6-§7:

- Get latest safely, Save release point, Change work area, Generate notes, Resolve;
- the command palette, Add project;
- the workspace's create, rename and delete;
- the workspace menu;
- Remove project;
- the shortcuts sheet.

**The rules:**

- **Every control** is a snora control from stage 1's primitives, or an iced `text_input`, and
  has an id in `view/ids.rs`. The look at rest is unchanged, compared by pixels. §4 has the one
  exception.
- **Tab stays inside the window,** and wraps at both ends (A1).
  - snora's dialog and sheet slots make the page beneath inert.
  - **The palette, Add project and the shortcuts sheet are drawn in knotra's own outer stack**
    (`view.rs`), not in snora's slots. Say how Tab is kept out of the page beneath them, or move
    them into snora's slots.
- **Each window is wrapped in stage 2's `focus_by_id`,** so a moving row inside it (Get latest
  safely's plan, the work-area list) keeps focus on its own control. Each window's controls that
  disable themselves when pressed join its `reserved`. List them.
- **Escape is unchanged** (Task 095, Task 111). No window holds a select or a menu today
  (`git grep` finds none), so the rule "a select inside a window closes on the first Escape" has no
  case yet. Say so in the report.

## 3. Entry and return, in one place (RFC-075 D3, D4 c)

One function, run after every update, compares the window on top (and its phase) before and after,
as RFC-066 D3 does today. It now returns `operation::focus(id)`.

- **Opened from the keyboard** means one of these:
  - by a shortcut;
  - while a page control holds keyboard focus;
  - from another window that holds it (a palette command, for example).

  A click presses without focusing, so a window opened by a click finds no focus.
- **A window opened from the keyboard:** focus goes to its entry, as inventory §6-§7 lists it, for
  each phase. A control that starts a change is never the entry.
- **The phase changes inside a window that holds keyboard focus:** focus goes to the new phase's
  entry. A phase with no controls leaves focus nowhere until the next phase has one.
- **Opened by a click:** nothing is focused. The first Tab reaches the window's first control.
- **The window closes:** focus returns to the page control that held it when the window opened.
  - If that control is gone, focus goes to its neighbour (A1's rule).
  - If the window was opened by a click, nothing is focused.
  - With snora, the window's own `focus(id)` takes focus from the page, so the return must be
    made explicitly. RFC-066's "return by construction" no longer holds.

## 4. Lists: ↓ ↑ (A1, D4 d)

**A field with its list:** the command palette (its search box and results), and Change work area
(its "Find work area" field and its candidates).

- **Focus stays in the field.** ↓ and ↑ move a **highlight** through the list.
- The highlight is drawn with snora's focus-ring tokens, so it looks exactly like focus. Prove the
  likeness by pixels against a focused snora control's ring.
- **Every typed character, Space included, goes to the field,** as typing always does. Backspace
  and an input method (IME) work as in any field.

  So nothing typed can ever run a result. This is A1's rule, by construction.
- **Enter runs the highlighted result.** With nothing highlighted, Enter does nothing, as today.
- **The highlight:**
  - ↓ from the field highlights the first enabled item;
  - ↓ on the last wraps to the first;
  - ↑ on the first returns to the field, with nothing highlighted;
  - disabled items are skipped. Their reason is printed beside them (RFC-073).
- **A change to the query** removes the highlight.
- **A click** on an item runs it, as today.
- **The items are not Tab stops.** Tab moves between the window's controls (Close, the field,
  Cancel), and the arrows move through the list.
  - The items are therefore plain iced buttons with a click handler, not snora controls. This is
    the one exception to §2's rule, and it is stated here.
  - Name the item widget, and give it the label and height guards.

**A list on its own:** the workspace menu (`Layer::WorkspaceMenu`).

- Its rows are snora controls and Tab stops.
- ↓ and ↑ move focus between the rows, wrapping at both ends. Disabled rows are skipped.

**Everywhere else in a window,** ↓ and ↑ do nothing. knotra's old arrow routing in windows
(`ShortcutMessage::ArrowDown`/`ArrowUp` → `advance_focus`) goes.

## 5. What retires

- knotra's window focus: `overlay_focus_order`, `layer_focus`, `advance_focus`,
  `activate_focused` and every window's own `focus_order`.
- `ring()`, and every use of `with_focus_ring(&t, false, …)`, once nothing calls them.
- **Remove whatever nothing uses any more,** and list it. Stage 5 removes the rest of the model
  and replaces RFC-066's tests.
- The suppression map stays at 7.

## 6. The shortcuts sheet (text approved by the owner, Review 301 §4)

The arrow row changes, exactly:

| | Keys | Where | What |
|---|---|---|---|
| English | ↑ / ↓ | In a list | Move to the next or previous item |
| Japanese | ↑ / ↓ | 一覧の中 | 次または前の項目へ移動 |

- Use new keys, for example `shortcut.where_list` and `shortcut.action_move_in_list`.
- If `shortcut.where_window` and `shortcut.action_focus_move` are left unused, remove them from both
  catalogs.
- The sheet's test (RFC-066 D7) must still say what the keys do.

## 7. Tests and captures

| Question | Level | How |
|---|---|---|
| Every window's Tab sequence, per phase, by id, wrapping both ways | 1 | the real key path, as in stage 2. The sequences are inventory §6-§7's, except §4's lists, whose items are no longer Tab stops |
| Tab never reaches the page while a window is open | 1 | every window, including the three in knotra's outer stack |
| Entry by keyboard, none by click, per phase; return on close, including to a neighbour | 1 | each window and phase in inventory §6-§7, and the detail panel |
| A change is never the entry | 1 | Start getting latest, Save release point, Change work area, Stop and discard edits, Remove from list, Confirm delete |
| The palette: typing, including Space, goes to the field with a result highlighted; Enter runs it; ↓ ↑ wrap; ↑ from the first returns to the field; disabled results are skipped | 1 | the real key path |
| The same for "Find work area"; the workspace menu's arrows | 1 | the real key path |
| The highlight looks like snora's ring | 2 | pixels |
| The look at rest of every converted control | 2 | pixels, all four themes |
| Undo and the detail panel are reachable | 1 | the real key path |
| Focus follows its id inside a window | 1 | Get latest safely's plan, with a row moving under focus |
| The guards cover the new controls | 1 | each **seen failing first** |

**Captures**, by the owner's GUI rules (`.git-exclude/rules/project-instructions-gui-common.md`):

- Task 108's session (`--scene demo`);
- the same script before and after, the before built from `origin/main`;
- every XDG folder isolated;
- every path, name and time on screen read, and the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | Is the palette's highlight clear, and does typing still read right? | Japanese, 150 %, Light: the palette opened by its shortcut, a short query typed, ↓ twice | the same keys on `origin/main` |
| C2 | Is a window's entry clear when it opens from the keyboard? | Dark, English, 100 %: Get latest safely's plan, opened from the selection bar by Enter, its entry ringed | the same keys on `origin/main` |
| C3 | Is the detail panel's entry clear? | Light, English, 100 %: Enter on a project's name, the panel's Close ringed | the same keys on `origin/main` |

## 8. CHANGELOG

Exactly, under 0.35.0:

```markdown
### Changed — The keyboard reaches everything on the dashboard and in every window

- Tab now reaches every control on the dashboard and in every window, including the Group and
  Sort choices, the buttons shown when there are no projects or no matches, the details panel,
  and Undo.
- After the last control, Tab goes back to the first, and Shift+Tab the other way, so the
  highlight never disappears.
- ↑ and ↓ move through lists: the command palette's results, the workspace menu, and "Find work
  area". In the command palette, everything you type goes into the search box, and Enter runs the
  highlighted command.
- The highlight around the control you are on is easier to see.
- When the list of projects changes, the highlight stays on the project you were on.
```

## 9. Report

- §1-§4, each with its design choices and the lists asked for (`reserved`, retired items).
- How Tab is kept out of the page beneath the three windows in knotra's outer stack.
- The tests, with failing-first output for the guards.
- The three capture pairs, with the privacy inspection.
- Gates, with the counts summed and checked.
- **CI's result once the run has finished, with its run id.**
- knotra's status and reflog.
