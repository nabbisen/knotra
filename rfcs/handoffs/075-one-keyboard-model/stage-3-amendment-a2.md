# RFC-075 stage 3 — Amendment A2: a neighbour is a control that is there

Review: `.git-exclude/reviewed/303-rfc-075-stage-3-amendment-a1-review.md`. Read its F1-F3 first.

**Start now.** Baseline: `origin/main` (`e534683` or later). One amendment, one review request.

No CHANGELOG change, and no version change.

**This amendment is immutable.**

## 1. Focus lands on a control that is really there (F1)

- **The neighbour search** (`belongs_on`) considers only the ids the tree reports now. A reserved
  id that the survey did not report is never chosen as a neighbour.
- **Reinstatement** (`merge_reserved`) keeps only **the remembered focused id itself** in place,
  so that a control which disables itself does not hand focus away. It never makes an absent id
  a candidate for anyone else.
- **`reserved` names only controls the view draws now.** `view.rs` builds the list from the
  state:
  - Check now: always;
  - the selection bar's Check for updates: only while the selection bar is drawn;
  - the detail panel's Check for updates: only while the panel is open.

  Do the same for each window's list.
- **Replace** `remove_project_confirmed_lands_on_nothing_a_found_bug_not_the_neighbour` with a
  test asserting the neighbour.

## 2. A late answer never overrides the keyboard (F2)

- `WindowFocusModeResolved(Mouse)` never replaces `Keyboard`.
- Tab, Shift+Tab, ↓ or ↑ in a window whose mode is still unknown (`None`) sets `Keyboard`, as it
  does from `Mouse`.

## 3. The anchor is asked of a named wrapper (F3)

- Each `focus_by_id` wrapper carries a scope: the page, or the open window.
- `tab_anchor` names the scope it asks.
  - A window opening asks the page.
  - Tab inside a window asks the window.
- Remove the "last non-empty wins" rule.

## 4. Tests

Through the real key path:

| Case | Expect |
|---|---|
| The panel open for Beta; Remove → Remove from list | focus on Alpha's row, by A1's rule. **Seen failing first on `e534683`** |
| Selection mode; Check for updates running (disabled, reserved); focus on Get latest safely; that control then goes (selection cleared by another path) | focus on a control the tree reports, never nowhere |
| Get latest safely opened by a click; Tab **before** the mode is resolved; then the late answer | `Keyboard`; the next phase change focuses its entry. **Seen failing first** |
| A window opens while the page has focus; Tab inside the window | each query is answered by the wrapper it names |

## 5. Report

- §1-§3's changes, each in one place.
- The tests, failing first where marked.
- Gates, with each crate's printed counts and their sum.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog and `git worktree list`.
