# RFC-074 handoff — Stage 1: the plan counts, and the bar stays on the dashboard

RFC: `rfcs/accepted/074-one-step-not-three.md` (D2, D4, and "Decided"). Read it first.

**Start now.** Baseline: `origin/main`. One stage, one review request.

No version change, and no release preparation.

**This handoff is immutable.**

## 1. D2: the plan says how many, and which

**Where:** Get latest safely's plan (`view/overlays/smart_pull.rs`, the `AwaitingConfirm` phase),
between the subtitle and the first row.

**The line** counts each disposition by its **existing** name:

| Dispositions | Name (English / Japanese) | Key |
|---|---|---|
| `Pull`, `StashAndPull` | Get latest / 最新を取得 | `plain.get_latest.action_get` |
| `FetchOnly` | Only check for updates / 更新の確認のみ | `plain.get_latest.action_check` |
| `Excluded` | Skipped / スキップ | `plain.get_latest.skipped_row` |

- **Form:** `<name>: <n>`, joined by ` · ` (the separator the dashboard's rows already use), in
  this order. For example, "Get latest: 3 · Only check for updates: 1 · Skipped: 2".
- **A count of zero is left out.**
- **Changing a row's choice** ("Only check for updates" / "Get anyway") updates the line at once.
- **The counts always equal the rows.** Build both from the same `plan.entries`, in one place.

**"Start getting latest" when every project is skipped:**

- it stays unavailable, as today;
- **its reason becomes the owner's new text** (RFC-074 "Decided"), under a new key, for example
  `plain.get_latest.all_skipped`:
  - English: "Every project is skipped, so nothing would change."
  - Japanese: 「すべてのプロジェクトがスキップされるため、変更はありません。」
- copy both character for character;
- the busy reason (`plain.activity.busy`) is unchanged;
- if `plain.disabled.choose_one` is then unused here, leave it: other places use it. Say which.

**The line never wraps inside a name,** and it must not leave a stranded last line in Japanese.
Add it to Task 122's description guard (all its cases), and to the label guard.

## 2. D4: the selection bar belongs to the dashboard

- **Draw the bar only on the dashboard.** Today `view.rs` draws it on every screen while
  selection mode is on.
- **Leaving the dashboard keeps the selection,** with the same projects ticked. Returning shows the
  bar again, unchanged.
- **Escape on History or Settings must not clear the selection.**
  - Today, with no window open, Escape ends selection mode on any screen
    (`app/focus_ops.rs`, `close_topmost_layer`, the `None` arm).
  - With the bar hidden, that would discard a choice the user cannot see.
  - On the dashboard, Escape behaves as today.
- **The page's `reserved` list** (`view.rs`) already includes the bar's "Check for updates" only
  while the bar is drawn. Confirm that it follows.
- **Focus (RFC-075):** a control in the bar cannot hold focus on a screen where the bar is not
  drawn. Leaving and returning must leave focus where A1's rules put it. Test it.

## 3. Tests and captures

| Question | Level | How |
|---|---|---|
| The counts equal the rows | 1 | plans with each mix: all kinds; only Get latest; only skipped; a row's choice changed. The line's text, in both languages |
| Zero counts are left out | 1 | the same plans |
| All skipped: Start unavailable, with the new reason, in both languages | 1 | the real key path |
| No stranded line, no broken name | 2 | the description guard and the label guard, at all their sizes. **Seen failing first** on a deliberately long fake count |
| The bar only on the dashboard; the selection kept | 1 | selection on, go to History, then Settings, then back to the dashboard: the bar is absent, absent, then present with the same ticks |
| Escape on History keeps the selection | 1 | the real key path; and on the dashboard it still clears |
| Focus across the screen change | 1 | the real key path, with focus on the bar's "Exit selection" when leaving |

**Captures,** by the owner's GUI rules: Task 108's session (`--scene demo`, with a settle delay),
the same script before and after, every XDG folder isolated, every path, name and time read, and
the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | Is the count clear before Start? | Japanese, 150 %, Light: the plan for the four usual projects (Task 105 §4), billing-service at 「更新の確認のみ」 | the same on `origin/main` |
| C2 | Is History free of the bar? | English, 100 %, Light: selection on with two ticked, then History | the same on `origin/main` |

## 4. CHANGELOG

In `[Unreleased]`, exactly:

```markdown
### Changed — Get latest safely counts what will happen before you start

The plan now says, above the list, how many projects will get the latest, how many will only
check for updates, and how many are skipped. If every project is skipped, it says so, instead of
asking you to choose a project.

### Changed — the selection bar stays on the dashboard

The bar for acting on selected projects is shown only on the dashboard. Your selection is kept
while you visit History or Settings, and Escape there no longer clears it.
```

## 5. Report

- §1's composition in one place, and the new key.
- §2's changes, including Escape.
- The tests, with the guard's failing output.
- The two capture pairs, with the privacy inspection.
- Gates, with each crate's printed counts and their sum, **pasted**.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog (pasted) and `git worktree list`.
