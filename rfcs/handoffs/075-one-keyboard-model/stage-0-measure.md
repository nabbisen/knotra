# RFC-075 handoff — Stage 0: measure, and compare ↓ ↑

RFC: `rfcs/accepted/075-one-keyboard-model.md` (accepted 2026-11-18). **No product change in
this stage.** Nothing merges into `main` except, at most, a test helper (§4). Each stage after
this one gets its own handoff.

**Start after 0.34.0 is released.** It needs snora 0.65 (Task 112) on `main`. Baseline:
`origin/main` at that point.

**Read first:**

- RFC-075, all of it;
- RFC-066 (`rfcs/done/066-keyboard-use-stays-inside-the-open-window.md`), D1–D7, and its
  handoff's per-window tables;
- snora's guides "Keyboard map" and "Overlays" (`nabbisen.github.io/snora/guides/keyboard.html`,
  `overlays.html`);
- the owner's GUI rules (`.git-exclude/rules/project-instructions-gui-common.md`).

## 1. The inventory

For **every screen and window**:

- the Dashboard, History, Settings and the header;
- every `Layer` (RFC-066 D1), in each phase that shows different controls.

**List every control in reading order,** with:

- its kind: button, chip, link, name, text field, select or checkbox;
- whether it can be disabled, and when;
- its position in knotra's current declared order (`layer_focus`, or `shell_and_dashboard_focus_order`),
  or "none" where knotra has no order today (Settings, History);
- the window's current entry.

**Derive it from the code, and check it with tests where the harness can.** Mark every row
whose reading position and declared position differ.

Save to `.git-exclude/evidence/rfc-075/stage-0/inventory.md`.

## 2. The spike: snora's model in a branch that is never merged

- Make a worktree under `.git-exclude/tmp/rfc-075-spike/`, on a local branch
  `spike/rfc-075-stage-0`.
  - **Do not push it.**
  - Show knotra's `git status --short` and `reflog -3` before and after, and `git worktree list`.
- **In the spike:**
  - route Tab and Shift+Tab to `snora::keyboard::traverse`;
  - rebuild three windows on snora controls, with `.id()` entries (RFC-075 D3):
    - Remove project, a plain dialog;
    - the command palette, a field plus a list;
    - the workspace menu, a list;
  - rebuild one Settings select and the file-change checkbox on `snora::design::select` and
    `snora::design::checkbox`.
- **Measure, and answer each with what you saw** (test output, recorded state, or one
  capture):

| # | Question | For |
|---|---|---|
| a | In a dialog, Tab past the last control: where does focus go, and where does the next Tab go? | RFC-075 D4 a |
| b | A disabled control: is it skipped by Tab? | D4 b |
| c | `operation::focus(id)` on open: is the ring drawn? With no focus set (opened by a click), where does the first Tab land? | D4 c |
| d | With the palette's search field focused, do ↓ and ↑ reach knotra's subscription, or does the field keep them? | D4 d |
| e | The Settings select: does Enter open it, do ↓ ↑ move, does Enter choose, does Escape close it and give focus back, all inside `snora::render`? | D1 |
| f | Can the headless level run `traverse`'s tasks? `iced_test`'s `Emulator` runs a program's tasks, and `Simulator` does not. If yes, show one Tab sequence asserted at level 2 | the acceptance tests |
| g | Escape with Task 111's layer in place: one press still closes, from the field too. **And** with a snora select or menu open inside a window, the first Escape closes only it, and the window waits for the next. Task 111's layer takes Escape before the window's content, so today it would close the window | D4 e |

## 3. D4 d: two builds for the owner to try

The owner decided to compare ↓ ↑ in the list-like windows before choosing. **Build two spike
binaries that differ only in that:**

- **with arrows:** in the palette's results, the workspace menu and "Find work area", ↓ and ↑
  move as Tab and Shift+Tab do, and only while no text field holds focus;
- **without arrows:** ↓ and ↑ do nothing there, except inside snora's own menus and selects,
  which answer them themselves.

**For the owner:**

- A launcher, `.git-exclude/evidence/rfc-075/stage-0/compare-d/try.sh with|without`. It runs
  that binary under its real name, with **all four XDG folders isolated** under
  `.git-exclude/tmp/`, and a copy of Task 108's fixtures as the workspace. The owner's own
  settings and projects are never read or written.
- A five-line `compare-d/README.md` saying what to try, in both builds:
  1. open the command palette;
  2. reach the third result;
  3. open the workspace menu, and reach the second workspace;
  4. in "Find work area", reach the second match;
  5. press Enter.
- No recording is needed. The owner tries them.

## 4. What may merge

Nothing, unless §2 f shows the `Emulator` works. In that case a test helper for level-2 Tab
sequences may merge as its own commit, with a test that uses it on today's knotra.

## 5. Report

- the inventory's path, and its differences counted;
- the answers a–g, each with its evidence;
- the two builds' paths and the launcher, run once by you to show it starts isolated:
  `ls` of the scratch XDG folders, and nothing under the real ones changed (the mtimes of
  `~/.config/knotra` before and after);
- the worktree;
- knotra's status and reflog.

**I will write the amendment** that records the measured answers, and D4 d once the owner has
compared.
