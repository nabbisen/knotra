# RFC-074 handoff — Stage 4 (Amendment A1): a tick stays until the user removes it

RFC: `rfcs/accepted/074-one-step-not-three.md`, Amendment A1 (D6). Read it first.

**Start now.** Baseline: `origin/main`. One stage, one review request. No version change, and no
release preparation.

**This handoff is immutable.**

## 1. Ticks are removed only by the user, or with their project

- **Stop trimming the selection to what is visible.** Today `reconcile_selection_with_display`
  (`state.rs`) runs after:
  - a filter (`app.rs`);
  - grouping, sorting or a collapse (`app/misc.rs`);
  - workspace changes (`app/workspace.rs`);
  - every background refresh (`app/background/mod.rs`).
- **Keep only one rule:** a tick whose project no longer exists in the open workspace is removed
  (a project removed; another workspace opened). Name the function for what it does now.
- **"Exit selection", and Escape on the dashboard,** clear every tick, hidden ones included, as
  today.
- **Removing the last existing ticked project** may still end selection mode, as today
  (`app/workspace.rs`). Say whether "last" now counts hidden ticks, and why.

## 2. One function decides what an action uses

`selection_summary().selected_ids` is the set of ticks that are also shown. **Make it the only
source for every action.** These read the raw selection today and must not:

- `app/sync.rs`:
  - Check for updates' `set_selection` (around line 68);
  - Get latest safely (around lines 264-276), including stage 2's "nothing selected" test;
- `app/freezer.rs`: Save release point's pre-selection (around lines 58-63).

Then:

- **In selection mode, with no shown project ticked,** "Get latest safely" (the bar's and the
  palette's) is unavailable, with the existing `plain.disabled.choose_one`. Every project is used
  only outside selection mode.
- **Add a guard:** a test that fails if any file outside the selection model reads
  `selection.selected_ids` directly. Name the files allowed: the model, and the bar's
  hidden-count line. Show it **failing first**.

## 3. The bar says what is not shown

Below the bar's count, only while at least one ticked project is not shown, using the owner's
approved text:

- English: "Selected but not shown: 2 (not included)"
- Japanese: 「選択中で非表示: 2 件（対象外）」

Notes:

- Compose it with new keys, in the catalog's usual way for a number. **The rendered text must match
  these exactly,** in both languages, for any count.
- The bar's own count keeps counting only shown ticks, as today.
- It never wraps inside a word. Add it to the label guard, and to Task 122's description guard if
  it can wrap.

## 4. Tests and captures

Through the real message path:

| Case | Expect |
|---|---|
| Tick three, collapse the group holding two | the bar: "1 selected", then "Selected but not shown: 2 (not included)". Expand: "3 selected", and the line is gone |
| A background refresh moves a ticked project into the collapsed "All set" | still ticked; counted as not shown |
| A filter, then a search, hiding ticked projects | the same |
| Each action with hidden ticks: Check for updates, Get latest safely, Save release point, Change work area, Generate notes, the palette | only the shown ticked projects are used |
| Selection mode, no shown tick, hidden ticks remain | "Get latest safely" unavailable, "Choose at least one project." |
| A ticked project removed from the workspace | its tick is gone |
| Another workspace opened | no tick carries over |
| Escape on the dashboard, and "Exit selection" | every tick is cleared, hidden ones too |

**Captures,** by the owner's GUI rules, with `--scene demo` (a different scene only with a reason
in the report): Task 108's session with a settle delay, the same script before and after, every
XDG folder isolated, every path, name and time read, and the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | Is it clear that hidden ticks are kept and left out? | Japanese, 150 %, Light: three ticked, "作業中" collapsed so two are hidden | the same on `origin/main`, where those two ticks are lost |

## 5. CHANGELOG

In `[Unreleased]`, exactly:

```markdown
### Changed — a tick stays until you remove it

In selection mode, a ticked project now stays ticked until you untick it or leave selection.
Closing a group, filtering, searching, or a refresh no longer removes ticks without a word.
Actions still use only the ticked projects you can see, and the bar says how many ticked projects
are not shown.
```

## 6. Report

- §1's function, and the "last ticked project" answer.
- §2's changes, and the guard's failing output.
- §3's keys, and the rendered text in both languages.
- The tests; the capture pair, with its privacy inspection.
- Gates on `+1.95`, with each crate's printed counts and their sum, **pasted**.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog (pasted) and `git worktree list`.
