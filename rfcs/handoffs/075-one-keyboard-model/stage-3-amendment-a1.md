# RFC-075 stage 3 — Amendment A1: no release yet, and how a window was opened

Review: `.git-exclude/reviewed/302-rfc-075-stage-3-review.md`. Read its F1-F4 first.

**Start now.** Baseline: `origin/main` (`fb0282f` or later). One amendment, one review request.

**This amendment is immutable.**

## 1. Revert the release preparation (F1)

- **The version goes back to 0.34.0:**
  - `Cargo.toml` (the workspace version, and the `knotra-ui` and `knotra-vcs` dependency
    versions);
  - `Cargo.lock`;
  - `packaging/windows/AppxManifest.xml`.
- **The metainfo:**
  - remove the `0.35.0` `<release>`;
  - point the screenshot URLs back at `0.34.0`.
- **The CHANGELOG:**
  - remove `## [0.35.0] — 2026-11-25` and its bold headline;
  - put the "Changed — The keyboard reaches everything…" entry, exactly as written, and Task
    118's "Fixed" entry, under `## [Unreleased]`.
- Show `git diff 74b3a21 -- Cargo.toml Cargo.lock packaging/ CHANGELOG.md` afterwards. It must
  contain only the CHANGELOG entry.

**A release is prepared only in its own release task, after the owner authorises it.**

## 2. Each window remembers how it was opened (F2)

A window is in **keyboard mode** from the moment it opens if it was opened from the keyboard
(stage 3 §3's definition: a shortcut, a page control holding keyboard focus, or another window in
keyboard mode). Otherwise it is in **mouse mode**.

- A window in mouse mode switches to keyboard mode when the reader first presses Tab, Shift+Tab,
  ↓ or ↑ in it.
- It stays in keyboard mode until it closes.
- **Entry on open:** only in keyboard mode, as today.
- **Phase change:** focus goes to the new phase's entry **only in keyboard mode**. In mouse mode,
  nothing is focused, and no ring appears.
- **One window replacing another:** the new one inherits the old one's mode.
- **Keep it in one place:**
  - the mode lives beside the top layer in `AppState`;
  - `settle_layer_focus` reads it;
  - no open site sets focus.

## 3. Return on close, tested (F3)

**The rule** (stage 3 §3) stands:

- in keyboard mode, closing returns focus to the page control that held it when the first window
  of the chain opened;
- if that control is gone, focus goes to its neighbour;
- in mouse mode, nothing is focused.

Whether `FollowsId`'s memory carries this or an explicit `focus(id)` does, **test it through the
real key path:**

| Case | Expect |
|---|---|
| Add workspace, opened by Enter from the workspace menu, closed by Escape; by Close; by Cancel | focus on the switcher |
| Remove project, opened by Enter on the detail panel's Remove, closed by "Keep" | focus on the panel's Remove |
| Remove project, confirmed: the project and its row are gone | focus on the neighbour, by A1's rule |
| The palette → a command that opens Get latest safely → Cancel | focus on the page control that opened the palette |
| Any window opened by a click, then closed | nothing focused |

## 4. Mouse mode and phases, tested (F2)

| Case | Expect |
|---|---|
| Get latest safely opened by a click; Planning → the plan | nothing focused; no ring |
| The same, then Tab | the window's first control; the next phase change focuses its entry |
| Get latest safely opened by Enter; Planning → the plan | the plan's entry (inventory §6a) |
| Save release point by Ctrl+T; Check readiness → ready | the ready phase's entry (§6b) |
| Every window, opened by keyboard | its entry, by the real key path, not by calling `entry_id` |

## 5. The late key, and housekeeping (F4, Review 302 §3)

**F4.** In Task 108's session:

- build both `origin/main` before stage 3 (`74b3a21`) and this amendment;
- send the same burst to each: ten Tabs with no delay between them, from the same start;
- take one screenshot after it.

Report both final positions, against the position ten Tabs should reach.

- If only the new build falls short, find the cause (the chained `then` tasks are the first
  suspect), fix it, and add a test that sends the burst in one batch of events.
- If both fall short equally, say so: it is the session.

**Housekeeping:**

- keep only the six final captures in `.git-exclude/evidence/rfc-075/stage-3/`;
- remove the `stage3-baseline` worktree (`git worktree remove`) and its target folder;
- **no `git stash` in the shared checkout.** Use a worktree.

## 6. Report

- §1's diff.
- §2's design, in one place.
- §3's and §4's tests, each seen failing first on `fb0282f` where it can fail.
- §5's two positions and the conclusion.
- Gates, with **each crate's printed counts** and their sum.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog and `git worktree list`.
