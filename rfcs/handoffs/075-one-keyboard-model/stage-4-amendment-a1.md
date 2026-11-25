# RFC-075 stage 4 — Amendment A1: focus that a slot hands across is not the reader's

Review: `.git-exclude/reviewed/305-rfc-075-stage-4-review.md`. Read its F1 and F2 first.

**Start now.** Baseline: `origin/main` (`43c6646` or later). One amendment, one review request.

No CHANGELOG change, and no version change.

**This amendment is immutable.**

## 1. The rule, in `FollowsId` (F1)

After a rebuild, the real focused control is **the reader's** only if one of these holds:

- it is the remembered id; or
- an operation explains the change (Tab, an arrow, a `focus(id)` task: `resync` already sees
  these); or
- the reader focused a text field directly (a click), and that field is still the one focused.

Otherwise, when the set of ids has changed shape, the focus was handed across by a reused tree
slot. **Put it back on the remembered id,** or on that id's neighbour if it is gone (stage 2's
rule).

**The design is yours. Keep it in `keyed.rs`, in one place.**

- Say how the widget tells a click into a text field apart from a handed-across slot. One
  workable test: a slot that hands focus across puts it on the control now at the remembered
  control's old position; a click does not.
- The keyed columns for History and Settings stay. They are good, but the rule above must hold
  without them: **test it on the dashboard's plain columns.**

## 2. Tests

Through the real key path. Each is **seen failing first on `43c6646`** where it can fail.

| Case | Expect |
|---|---|
| Review 305's probe: Beta and Gamma, focus on Gamma, Alpha added and sorted first | focus on Gamma's name |
| A project inserted into a section above the focused row by a refresh | focus stays on its project |
| The project above the focused one removed | focus stays on its project |
| **A real reorder:** first assert that the order did change, then that focus stays | focus stays on its project. Rewrite `a_same_count_reorder_by_sort_still_follows_the_id` this way, or replace it |
| Typing in the dashboard's search box, so the rows change with every key | focus never leaves the field |
| A click into the search field (focus without an operation), then a refresh changes the rows | focus stays in the field |
| Tab in the same step as an insertion | focus is where Tab put it |

## 3. The shared checkout (F2)

- In the report, say which command wrote `reset: moving to HEAD` at 18:42, and what it touched.
- From now on, work that needs a clean tree goes in a worktree. **No `git stash` and no
  `git reset` in the shared checkout.**

## 4. Report

- §1's rule, where it lives, and how it tells a click from a handed-across slot.
- The tests, with failing-first output.
- §3's answer.
- Gates, with each crate's printed counts and their sum.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog and `git worktree list`.
