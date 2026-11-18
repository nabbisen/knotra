# RFC-073 handoff — Amendment A6: the narrow layout hides nothing, and "no commits yet"

Issued 2026-11-18. Reasons:

- `.git-exclude/reviewed/280-task-108-a1-and-task-107-review.md` §3;
- `.git-exclude/reviewed/281-task-110-review.md` §3;
- `.git-exclude/reviewed/283-stage-4-additions-and-open-text-decisions.md`.

**Done in stage 4, with A5, as their own commits before stage 4's own.** **The owner approved
both on 2026-11-18:** §1's layout change and §2's text, `dashboard.no_commits_yet` = "No commits
yet" / 「まだコミットがありません」.

Evidence follows the owner's GUI rules, Amendment A4 §1 and Task 108's session script.

## 1. The Compact toolbar shows every filter, and names Arrange (Review 283 §1)

Since Task 110 the Compact filter row wraps. So the reason for hiding two filters behind "⋯"
has gone, and so has the reason for a bare "▾" in place of "Arrange".

- **Compact shows all five filters,** in the same wrapping row Standard uses.
- **The "⋯" disclosure goes:**
  - `dashboard_toolbar_overflow_open`;
  - `ToolbarOverflowToggled`;
  - `COMPACT_VISIBLE_CHIP_COUNT`;
  - its focus target;
  - its tests.

  Report each removal.
- **Compact's Arrange is Standard's:** the same builder, with its label (`dashboard.arrange`)
  and its chevron. The bare "▾" goes.
- **The label guard (Task 110)** checks all five chips and Arrange in Compact. It is seen to fail
  first, on the "▾" (it has no label) or on the missing chips.

## 2. A project with no commits yet (Review 283 §2)

A repository with no commits (a fresh `git init`) has a branch and no commit.

- endringer's `status_digest` fails at `peel_to_commit`, so knotra has no context.
- The row then reads "Work area unavailable", beside whatever status its files give,
  **✓ All set** included (`row.rs:134`).

**The fix:**

- **knotra-vcs reads `repository_info().head`** (endringer 0.33.4). For `HeadState::Unborn`, it
  builds the context with the branch name and a marker that there is no commit yet.
- **The row reads** `<branch> · <dashboard.no_commits_yet>`, for example "main · No commits
  yet". The tier is unchanged: it follows the files, as for any project.
- **Any other missing context** (HEAD cannot be read for a reason other than "unborn") is
  `NeedsHelp` with the existing `ReadUnavailable` cause. "Work area unavailable" then never
  stands beside ✓.
- **Tests:**
  - an unborn repository, empty → All set, row "main · No commits yet";
  - an unborn repository with a new file → its tier, and the same row text;
  - a repository whose HEAD file is corrupt → Needs help, ReadUnavailable.

  Each runs through knotra-vcs's integration fixtures, with the A4 §1 guards.
- **Jujutsu:** check what endringer's jj backend reports for a new jj repository, and report it.
  Change nothing for jj unless it shows the same contradiction.

## 3. Evidence plan

| Question | Level | How |
|---|---|---|
| Compact hides no filter and names Arrange | 2 | the label guard, failing first |
| The unborn and corrupt cases read truthfully | 1 | §2's tests |
| The Compact toolbar reads well | 3 | **one** capture: zoom 150 %, 800 × 720, the wider language, after; one baseline under the same conditions |

## 4. CHANGELOG (the stage's `[Unreleased]`)

§1, exactly:

```markdown
### Changed — a small window shows every status filter

In a small window or at a large zoom, all five status filters are shown, continuing on a
second line, and "Arrange" is shown by name. Nothing is hidden behind "⋯" any more.
```

§2, exactly:

```markdown
### Fixed — a project with no commits says so

A project with no commits yet showed "Work area unavailable", sometimes beside "All set". It
now shows its branch and "No commits yet".
```
