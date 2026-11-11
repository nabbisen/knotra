# RFC-073 handoff — Amendment A5: stage 3's loose ends, with stage 4

Issued 2026-11-12. Reason: `.git-exclude/reviewed/274-rfc-073-stage-3-review.md` §2.

**Stage 3 is approved. These items are done in stage 4, as their own commit before stage 4's
own.** §5 waits for the owner's word on its text. If that has not come when you reach it, do
everything else and report §5 as waiting.

**Evidence follows the owner's updated GUI rules and Amendment A4 §1:**

- fixtures in the project, with the git guards;
- Task 108's session script for every real window;
- each property checked at the cheapest level that can prove it.

## 1. Off stays Off, and an interval never becomes Off (Review 274 §2.1)

`load_config`'s snap for `refresh_interval_secs`:

- 0 stays 0 ("Off");
- **a non-zero value snaps among the non-zero choices only** (30, 60, 300, 900), ties going
  downward.

**Test, one table:**

| Saved | Loads as |
|---|---|
| 0 | 0 |
| 1 | 30 |
| 10 | 30 |
| 15 | 30 |
| 45 | 30 |
| 46 | 60 |
| 2000 | 900 |

Correct the `config.rs` comment: values typed in 0.33 and earlier are moved too, not only a
hand-edited file.

## 2. Operation log size takes effect at once (Review 274 §2.2)

- **Lowering:** truncate `operation_logs` at once, in the `MaxLogEntriesChanged` handler.
  - Drop the ids that fall off from `history_expanded`.
  - If the detail panel or anything else refers to an entry by position, report it.
- **Raising:** re-read from disk with `load_recent_logs(&paths, n)`, as a `Task`, off the update
  path.
  - On completion, **merge** with the list in memory by `operation_id`, newest first, truncated
    to `n`. An operation that finishes while the read runs must not be lost.
  - Report unreadable entries the way startup does.
- **Then no setting needs a restart:**
  - remove `settings.needs_restart`, its call site and the restart-note plumbing;
  - correct the `config.rs` comment on `max_log_entries`;
  - in the report, give D13's result as "none, after A5 §2".

**Tests:**

- lower → truncated at once, and the dropped ids are gone from `history_expanded`;
- raise → the merged list holds older entries from disk **and** an entry added during the read,
  with no duplicate (drive the completion message directly).

## 3. Selects stand as tall as the buttons (Review 274 §2.3)

- One helper in `knotra-ui`, `styled_pick_list`, which sets size, line height, padding and
  style together, as `styled_text_input` does.
  - The closed select is `BUTTON_HEIGHT` tall, its text vertically centred.
  - Report the open list's row height. It must be at least `SMALL_BUTTON_HEIGHT`.
- Every `pick_list(` in `view/` and `widget/` goes through it. There are nine today: name each.
- **Extend `text_input_height_guard`** (or add a sibling) to `pick_list(`, seen to fail first.

## 4. Section headings in Bold (Review 274 §2.4)

- `settings.rs` `section_header` uses `strong()` (Bold).
- The same goes for the detail panel's section headings: `detail.section_identity`,
  `detail.section_status`, `detail.section_recent_operations`, and any other `detail.section_*`.
- **Name the typeface of the Japanese headings** in the report (Review 248: Bold, not Semibold).

## 5. File-system monitoring as a checkbox (Review 274 §2.5): **waits for the owner's text**

- The "Disabled" / "Enabled ✓" button becomes a checkbox, through `widget/checkbox.rs`, labelled
  `settings.fs_watch_enable_label`.
- The interval row ("Change detection interval") shows only while the box is ticked.
- **Under Advanced, no subheadings:**
  - remove `settings.section.logs` and `settings.section.fs_watch`;
  - the rows are Simultaneous reads, Operation log size, the checkbox with its hint, and
    (when on) the interval.
- **Remove** `settings.fs_watch_enabled`, `settings.fs_watch_disabled` and
  `plain.activity.fs_watch_disabled`, with the status message it set.
- **Text:** Review 274 §4's table, exactly as the owner approves it, including
  `settings.section.refresh`.

## 6. Layout points (Review 274 §2.6)

- **The Workspaces row:**
  - the reason for an unavailable Remove ("Keep at least one workspace.") goes on its own line
    under the row, aligned with the name;
  - Rename and Remove share one top edge.
- **The Resolve panel:** each "Choose … in Settings first." hint is as wide as its button's
  column and wraps there, with the row's spacing between the columns.
- **The detail panel's recent operations:**
  - each row has the list rows' vertical padding;
  - its badge aligns with the first line;
  - a wrapped time never touches the rule.

## 7. Evidence plan

| Question | Level | How |
|---|---|---|
| §1: Off stays Off; intervals never become Off | 1 | the table test |
| §2: the log size applies at once, both ways, losing nothing | 1 | the two tests |
| §3: every select goes through the helper | 1 | the guard, seen to fail first |
| §3: a select is 44 px, its list rows at least 36 px | 2 | Task 108's harness, if it reads bounds; otherwise level 1 from the helper's arithmetic, as A3 §2 did |
| §4: headings are Bold, in the right typeface | 3 | in the captures below: name the Japanese typeface |
| §5 and §6: Settings, Workspaces, Resolve and the detail panel read cleanly | 3 | the captures below |

**Captures, the worst case plus a baseline.** Use zoom 150 %, the session's fixed size, and the
language whose labels measure wider (name the widths), in Light:

1. Settings with Advanced open and the checkbox ticked, after;
2. its baseline: the same screen on stage 3 (`a34a020`);
3. the Resolve panel with no tools configured, after;
4. the detail panel with three recent operations, after.

No other captures unless a check fails.

## 8. Report

- per item: the diff summary and the tests;
- the nine selects;
- the row height of the open list;
- the captures, with their questions and conditions;
- the privacy inspection;
- the guards, with knotra's status and reflog;
- gates.

**CHANGELOG:**

- §1–§4 and §6 change nothing a stage 3 entry has not already announced. Add no entry for
  them.
- §5, when made, adds exactly:

```markdown
### Changed — file-change refresh is a plain on/off box

The experimental refresh when files change is now a checkbox, and its settings sit together
under "Advanced".
```
