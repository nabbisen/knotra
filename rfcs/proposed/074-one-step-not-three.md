# RFC-074 - One step, not three

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - the most common wish, "bring everything up to date", takes three steps |
| Effort | Medium - two entry points, one summary line, one row action, and where one bar is drawn |
| Target | 0.36.0 |
| Related files | `crates/knotra-app/src/app/sync.rs`, `crates/knotra-app/src/state/palette.rs`, `crates/knotra-app/src/view/dashboard/{toolbar,row}.rs`, `crates/knotra-app/src/view/overlays/smart_pull.rs`, `crates/knotra-app/src/view.rs` |
| Related | Review 257 F1-F3 (the older-users assessment); Review 258 H2; Review 259 §4 (F1's safety condition); RFC-073 ("calm by default"); RFC-075 (one keyboard model) |

## Summary

Getting the latest for every project takes three steps today:

1. enter selection;
2. tick each project;
3. choose "Get latest safely" from the bar.

This RFC makes it **one step**, without making it less safe. "Get latest safely" can be started with
nothing selected, and then covers every project. The plan window, which already asks before
anything changes, also states **how many projects, and which**.

A row whose next step is clear offers that one step. The selection bar stays on the dashboard,
where its actions belong.

## Problem

- **F1. Three steps for the common case.** The plan already covers every project when nothing is
  selected (`app/sync.rs`, `SmartPullPlanRequested`). But nothing starts it that way:
  - the command palette's "Get latest" needs a selection ("Choose at least one project");
  - the selection bar exists only in selection mode.
- **F2. No next step on the row.** Only rows that need help carry a button. A project with updates
  available, whose next step is obvious, has none.
- **F3. The selection bar follows the user.** It is drawn on every screen while selection is on
  (`view.rs`), so it sits under History and Settings, offering dashboard actions there.

## Design

### D1. "Get latest safely" with nothing selected covers every project

- **The command palette's "Get latest safely"** (`plain.get_latest`) is available with no
  selection, unless an operation is running.
  - With a selection, it acts on the selection, as today.
  - With none, it acts on every project. Its existing message already does this.
- **The dashboard toolbar gains "Get latest safely"** with nothing selected. It appears only while
  at least one project shows "Updates available", so a calm dashboard stays calm (RFC-073).
  - It opens the same plan, for every project.
  - In selection mode it is not drawn, because the selection bar carries the same action for the
    selection.
- **No new words:** both use the action's existing name, "Get latest safely".

### D2. The plan says how many, and which, before anything starts

Review 259 §4's condition. Above the plan's rows, one summary line counts each disposition, using
**each disposition's existing name** (`plain.get_latest.action_get`, `.action_check`,
`.skipped_row`). For example:

> Get latest: 3 · Only check for updates: 1 · Skipped: 2

- Every project is listed below it, as today, each with what will happen and, if skipped, why.
- A count of zero is left out of the line.
- If nothing would change (only skips), **"Start getting latest" is unavailable,** and its reason
  is printed beside it (RFC-073).
- Changing a row's choice ("Only check for updates" or "Get anyway") updates the line at once.

This holds however the plan was opened: from the selection, the toolbar, the palette, or a row
(D3).

### D3. A row offers its one clear next step

- A row shows **"Get latest safely"** when its project shows "Updates available" **and** its plan
  would get the latest:
  - a clean work area;
  - a shared source;
  - nothing needing a choice.
- It opens the plan **for that one project**. It never pulls directly: the same plan, the same
  "Start".
- A row with unsaved work, or anything that would make its plan only check, shows no button.
  Offering it there would promise what the plan will not do.
- **The button carries the action's own name,** "Get latest safely", not "Get latest". A shorter
  name would read as a different, less safe action, against 0.34.0's rule of one name for each
  action.
- Rows that need help keep their own action ("Choose resolution", "Show details"). No row shows
  two actions.

### D4. The selection bar belongs to the dashboard

- The bar is drawn **only on the dashboard**.
- Leaving the dashboard keeps the selection. Returning shows the bar again, with the same
  selection.
- History and Settings never show dashboard actions.

### D5. The keyboard (RFC-075)

- The toolbar button and each row's "Get latest safely" have ids in `view/ids.rs`, built from the
  project's id for a row.
- They follow focus by id, as every control does.
- A control that disables itself while an operation runs joins the page's `reserved` list while
  it is drawn.
- The plan window's entry and return follow RFC-075 stage 3:
  - opened from the keyboard, the plan's entry is focused;
  - on close, focus returns to the control that opened it: the toolbar button, the row's button,
    or the selection bar's.

## Requirements

- R1. D1:
  - the palette's "Get latest safely" with no selection opens the plan for every project;
  - the toolbar button appears only while a project shows "Updates available", and never in
    selection mode.
- R2. D2:
  - the summary line counts match the rows, in every way of opening the plan;
  - zero counts are left out;
  - "Start getting latest" is unavailable, with its reason, when nothing would change;
  - the line updates when a row's choice changes.
- R3. D3:
  - the row's "Get latest safely" appears only on rows that meet every condition, and opens a
    plan for that project only;
  - a row never shows two actions.
- R4. D4: the bar is drawn only on the dashboard; the selection survives leaving and returning.
- R5. D5: ids; focus by id; entry and return, through the real key path.
- R6. **No new catalog text.** If a design step needs new words, it stops and asks the owner.

## Acceptance

- From a dashboard with updates available, one press of "Get latest safely" opens a plan that
  names every project and counts what will happen. One more press starts it.
- No path starts getting the latest without the plan.
- History and Settings never show the selection bar.

## Stages

1. **D2 and D4,** which change no flow: the summary line, and the bar only on the dashboard.
2. **D1:** the toolbar button and the palette without a selection.
3. **D3:** the row's "Get latest safely".

Each stage has its own handoff and review. Captures follow the owner's GUI rules: the plan with
its summary line, the toolbar button, and a row's button, each in English and Japanese.

## Out of scope

- Adding a folder of repositories (issue #4): its own RFC, in 0.37.0.
- Any action other than "Get latest safely" without a selection. Save release point and Change
  work area keep needing a choice of projects, because their effect is not the same for every
  project.

## Decided

Awaiting the owner.

- D1's toolbar button is shown only while updates are available. That is my recommendation, for
  RFC-073's calm.
- D4 keeps the selection across screens.
