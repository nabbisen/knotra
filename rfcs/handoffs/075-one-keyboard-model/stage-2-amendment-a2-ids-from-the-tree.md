# RFC-075 stage 2 — Amendment A2: the ids come from the tree

Review: `.git-exclude/reviewed/300-rfc-075-stage-2-amendment-a1-review.md`. Read its F1-F3 first.

**Start now.** Baseline: `origin/main` (`c913ffa` or later). One amendment, one review request.

No CHANGELOG entry, and no release until stage 3 (Review 298 §3).

**This amendment is immutable.**

## 1. Focus follows the id it is on, read from the tree (F1)

`FollowsId` keeps its rule from A1 §1:

- focus stays on the same control;
- if that control is gone, focus goes to the next survivor, else the previous one;
- a move the reader or the application makes is never undone.

**What changes is where the keys come from:**

- **Each pass reads the targets' ids from the tree,** in order, through the `id` that
  `Operation::focusable` receives.
- **Focus is given by id,** not by position.
- **Delete** `dashboard_content_focus_keys`, `hash_id` and the position-based `Refocus`.
  `focus_by_key` then takes only its content.
- **A focus target without an id** cannot be followed. Say what the widget does with one. Then
  add a test: in every page layout §4 lists, every focus target has an id. A control added later
  without one then fails a test rather than silently escaping.

## 2. One wrapper, at the page's root (F2)

- Wrap the page once: the header, the toolbar with its Arrange panel, the body and the selection
  bar. Windows wait for stage 3, which will use the same widget.
- `1144026`'s and `279c0ee`'s `focus(id)` fixes: remove each one the wrapper now covers. Keep
  their tests.
- Say which were removed, and show their tests passing without them.

## 3. Check now keeps its place during a refresh (F3)

When Check now holds focus and becomes disabled for a refresh, whether by Enter or by an
automatic refresh:

1. **no other control takes focus,** and no ring appears elsewhere;
2. **Tab goes to the control after it** (Command palette), and **Shift+Tab to the one before it**
   (History). Neither starts from the top of the page;
3. **when the refresh ends,** if the reader has not moved, focus is on Check now again, with its
   ring.

The design is yours.

- §1's neighbour rule must not hand Check now's focus away: a control that is disabled has not
  gone.
- Say how the widget tells the two apart, or why it need not.
- Replace `check_now_disabling_mid_refresh_is_measured_not_fixed`, which asserts today's
  behaviour, with tests of 1-3.
- Apply the same rule to any other page control that disables itself when pressed, and list each
  one.

## 4. Tests

| Question | Level | How |
|---|---|---|
| The ids come from the tree | 1 | `git grep` finds no `dashboard_content_focus_keys` and no `hash_id`. `keyed.rs`'s unit tests are rewritten for ids |
| A reorder with an equal count still follows the id | 1 | New: two "Needs help" projects swap places (by sort or by name), with focus on the first one's name. Focus must stay on its project. (It passes on `c913ffa` too, because the hand-made list happens to be right today. It guards the new mechanism) |
| Every focus target has an id | 1 | wide and compact; selection mode on and off; an empty workspace; no matches; the Arrange panel open; two workspaces |
| The page's root is covered | 1 | focus on "Select" when a chip press adds "Clear filters"; on a chip when the Arrange panel opens; on the last header control when the switcher appears |
| Check now during a refresh | 1 | §3's 1-3, by Enter and by an automatic refresh |
| A1's tests | 1 | all still pass, on the new mechanism |

No captures: nothing a user sees at rest changes.

## 5. Report

- §1: the widget's new shape, and what it does with a target that has no id.
- §2: where it wraps, and which earlier fixes went.
- §3: the design, the controls it covers, and its tests.
- The tests, with failing-first output.
- Gates, with the counts summed **and the sum checked**.
- **CI's result once the run has finished, with its run id.**
- knotra's status and reflog.
