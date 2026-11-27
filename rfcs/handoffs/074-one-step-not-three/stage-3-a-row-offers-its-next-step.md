# RFC-074 handoff — Stage 3: a row offers its one clear next step

RFC: `rfcs/accepted/074-one-step-not-three.md` (D3, D5). Stages 1 and 2 are done.

**Start now.** Baseline: `origin/main` (`506f59a` or later). One stage, one review request.

No version change, and no release preparation.

**This handoff is immutable.**

## 0. First: the summary uses the rows' own word

Stage 1's summary line says "Skipped" (`plain.get_latest.skipped_row`). The rows say "Skip"
(`plain.get_latest.action_skip`). Japanese is 「スキップ」 for both.

- Use `plain.get_latest.action_skip` in the summary, so it reads "Get latest: 7 · Skip: 13".
- `skipped_row` stays for the result view, which says what was done.
- Update stage 1's tests.

## 1. D3: the row's "Get latest safely"

**When a row shows it.** All of these must hold:

- its project shows "Updates available";
- its work area is clean;
- it has a shared source;
- nothing needs a choice;
- it is not missing;
- **not in selection mode.** There the rows carry checkboxes, and the bar acts on the selection.

**Make "would its plan get the latest" one function,** shared with the plan itself, so the button
and the plan can never disagree. Say where it lives.

**What it does:**

- it opens the plan **for that one project**, never a direct pull;
- use a message that sets the plan's project list **and** its selection map to exactly that
  project (stage 2's lesson, Review 317);
- the plan's row must read "Get latest", and its summary "Get latest: 1".

**How it looks:**

- `secondary_control`, labelled with the action's own name, `plain.get_latest` ("Get latest
  safely" / 「安全に最新を取得」);
- in the row's action track, where "Choose resolution" sits on a row that needs help;
- **a row never shows two actions;**
- the label never cut or squeezed, in either band, at any size the label guard checks. Extend the
  guard to it, and show it **failing first**.

**The keyboard (D5):**

- an id from the project's id (for example `ids::row::get_latest(&id)`);
- focus follows it;
- while an operation runs, it is unavailable, and joins `reserved` while it is drawn;
- opened from the keyboard, the plan's entry is focused;
- on close, focus returns to the row's button, or, if that row no longer qualifies, to its
  neighbour.

## 2. Tests and captures

| Question | Level | How |
|---|---|---|
| Shown exactly when every condition holds | 1 | one row per failing condition (unsaved work, no shared source, a needed choice, missing, selection mode) and one that passes |
| The button and the plan agree | 1 | for each case above, the button's presence equals "the plan would get the latest" |
| One project's plan | 1 | the real message path: one row, "Get latest", summary "Get latest: 1" |
| Never two actions on a row | 1 | a row that needs help keeps only its own action |
| The label never cut | 2 | the label guard, **seen failing first** |
| Entry and return by keyboard | 1 | the real key path, including the row ceasing to qualify after the plan runs |
| §0's word | 1 | the summary reads "Skip: n" in English |

**Captures,** by the owner's GUI rules, **with `--scene demo`** (the handoff's scene; if another is
needed, say why in the report): Task 108's session with a settle delay, the same script before and
after, every XDG folder isolated, every path, name and time read, and the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | Is the row's button clear, and only where it belongs? | Japanese, 150 %, Light: the demo dashboard, web-frontend's row (updates available) beside billing-service's (unsaved work, no button) | the same on `origin/main` |
| C2 | Does it open a plan for that project alone? | English, 100 %, Light: the plan from web-frontend's button | none (new) |

## 3. CHANGELOG

In `[Unreleased]`, exactly:

```markdown
### Changed — a project with updates available offers "Get latest safely" on its row

Its button opens the plan for that one project, which says what will happen before anything
starts. The button appears only when the project would really get the latest: no unsaved work, a
source to get updates from, and nothing that needs your choice.
```

## 4. Report

- The shared function, and where it lives.
- The message that opens a one-project plan, and its selection map.
- The tests, with the guard's failing output.
- The capture pair, with the privacy inspection.
- Gates, with each crate's printed counts and their sum, **pasted**.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog (pasted) and `git worktree list`.
