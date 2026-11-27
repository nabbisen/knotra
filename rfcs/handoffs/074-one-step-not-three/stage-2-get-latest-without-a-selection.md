# RFC-074 handoff — Stage 2: "Get latest safely" without a selection

RFC: `rfcs/accepted/074-one-step-not-three.md` (D1, D5, "Decided"). Stage 1 is done: the plan
counts what will happen before Start.

**Start now.** Baseline: `origin/main` (`fde8a80` or later). One stage, one review request.

No version change, and no release preparation.

**This handoff is immutable.**

## 1. The command palette

Today, "Get latest safely" (`action.pull_selected`, `plain.get_latest`) is unavailable without a
selection (`state/palette.rs`, `selection_has_upstream_unless_busy`).

- **With a selection,** it acts on the selection, as today.
- **With none, it is available whenever any project in the workspace has a shared source,** unless
  an operation is running. It opens the plan for every project.
  - `SmartPullPlanRequested` already covers every project when its list is empty (`app/sync.rs`).
  - Make sure the empty list really reaches it from this path, and test it.
- **The label is unchanged.** The plan says what will happen.
- Each disabled case keeps an accurate reason: busy, or no shared source anywhere. Check the
  existing keys, and say which is used. **No new text.** If an accurate reason needs new words,
  stop and report.

## 2. The toolbar button

- **"Get latest safely"** (`plain.get_latest`), a `secondary_control`, with an id in
  `view/ids.rs` (for example `ids::toolbar::get_latest()`).
- **Shown only while at least one project shows "Updates available"** (the same condition as the
  "Updates available" chip's count above zero), and **never in selection mode,** where the bar
  carries the action for the selection.
- **Pressing it opens the plan for every project,** with the same message as the palette's path
  with no selection.
- **Placement:** in the toolbar's own row, in reading order after "Select". In the compact band,
  it may wrap to its own line; it must never be cut or squeezed. Extend the label guard to it, at
  every size the guard checks.
- **While an operation runs,** it is unavailable, and the page's `reserved` list includes it while
  it is drawn (RFC-075 stage 3 A2).

## 3. The keyboard (RFC-075)

- The new button is a Tab stop in reading order, and focus follows it by id.
- **Opened from the keyboard:** the plan's entry is focused, as RFC-075 stage 3 defines.
- **On close,** focus returns to the button that opened it. If the button is gone because no
  project still has updates, focus goes to its neighbour, by A1's rule.

## 4. Tests and captures

| Question | Level | How |
|---|---|---|
| The palette with no selection opens a plan for every project | 1 | the real message path; the plan's rows equal the workspace's projects |
| The palette with a selection acts on the selection | 1 | unchanged, still passing |
| The button's visibility | 1 | shown with one project behind; hidden with none; hidden in selection mode |
| The button opens the same plan as the palette | 1 | the same rows and the same summary line |
| Unavailable while busy, and `reserved` | 1 | the real path |
| Entry and return by keyboard, including to a neighbour | 1 | the real key path |
| The label never cut, in both bands | 2 | the label guard, **seen failing first** on a deliberately narrow track |

**Captures,** by the owner's GUI rules: Task 108's session (`--scene demo`, with a settle delay),
the same script before and after, every XDG folder isolated, every path, name and time read, and
the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | Is the button clear, and calm, on the dashboard? | Japanese, 150 %, Light: the demo dashboard (web-frontend has updates) | the same on `origin/main`, without the button |
| C2 | Does one press give a plan for everything, counted? | English, 100 %, Light: the plan opened by the button | none (new) |

## 5. CHANGELOG

In `[Unreleased]`, exactly:

```markdown
### Changed — Get latest safely works without choosing projects first

When any project has updates available, the dashboard shows "Get latest safely". It opens the plan
for every project, which says what will happen before anything starts. The command palette's
"Get latest safely" works the same way when nothing is selected.
```

## 6. Report

- §1's availability rule, and the reasons used.
- §2's placement, and the visibility condition in one place.
- The tests, with the guard's failing output.
- The capture pairs, with the privacy inspection.
- Gates, with each crate's printed counts and their sum, **pasted**.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog (pasted) and `git worktree list`.
