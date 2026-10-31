# RFC-071 handoff — Amendment A4: no badge on group headings

Ruling of Review 250 on Review Request 132 (stop and report: the "In progress" heading's
tone). **Replaces one item of stage 2 step 2** and the stage 2 CHANGELOG's first entry.
Everything else in the handoff, A1, A2 and A3 stands. The ruling is mine, within the RFC's
meaning ("colour plus symbol plus words, never colour alone"); it adds no text and no
behaviour.

## The question

D3 placed a badge in each group heading. "In progress" holds rows of two tones: unsaved
work (`warning`), and updates or local commits (`info`). No single tone fits the heading.

## Ruling: group headings carry no badge

- **A tone describes a status, and a heading is not a status.** It is structure.
  - With "Group by: Project group" or "None" (`DashboardGrouping`, `config.rs:13`), the
    headings are not statuses at all.
  - Under "Attention", every row below a heading already shows its own badge.
- **Any tone on "In progress" would mislead.** `info` hides unsaved work. `warning` paints
  rows that only have updates. A tone that follows the contents would change colour as
  projects change, and a heading's colour should not flicker.
- **One meaning, one place.** Badges sit where a status is: on each row's status line, and on
  each chip. A heading gets its emphasis from D1: the `title` role, bold, and the space above
  it (D8).

## Changes to stage 2

- **Step 2:** the badge goes before each row's status line and on each chip. **Not in group
  headings.**
- **The tone function** stays exhaustive with no `_` arm, over the status types that carry
  a tone:
  - `StatusFilter` (the chips);
  - `DashboardCause` (needs-help rows, always `danger`);
  - `ProgressKind` (`Uncommitted`/`Untracked` → `warning`; `Ahead`/`Behind` → `info`);
  - all-set rows → `success`.

  `DashboardTier` gets no tone function.
- **Untracked files:** your report maps `Untracked` to `warning` with "Unsaved work". That
  is right: an untracked file is work not yet saved to the repository.
- **The CHANGELOG's first stage 2 entry** becomes, exactly:

```markdown
### Changed — each status has its own colour and symbol

A project that needs help now looks different from one that is all set. Each status has a
coloured symbol beside its words, in the project list and on the filter buttons, which now
also show how many projects each one matches. Blue now marks only what you can click or
press, and on History an entry's buttons are no longer both highlighted.
```

Everything else in stage 2 proceeds as written, in one filing.
