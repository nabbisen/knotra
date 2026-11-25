# RFC-075 handoff — Stage 4: Settings and History

RFC: `rfcs/accepted/075-one-keyboard-model.md` (D5, and Amendment A1). Read first:

- stage 0's inventory, `.git-exclude/evidence/rfc-075/stage-0/inventory.md` §4-§5;
- Reviews 298-304.

**Start now.** Baseline: `origin/main` (`c97883b` or later). One stage, one review request.

No version change, and no release preparation (Review 302 F1).

**This handoff is immutable.**

## 1. The problem

Neither screen has a single Tab stop. Tab reaches only the header. These are RFC-075's first two
problems.

## 2. Every control becomes a snora control

Use stage 1's primitives, inside the page's `focus_by_id_page` wrap, with ids in `view/ids.rs`.

| Screen | Controls (inventory §4-§5) |
|---|---|
| Settings (`view/settings.rs`) | the selects: Language, Theme, Zoom, Auto-refresh, Simultaneous reads, Operation log size, File-change interval; the text fields: Editor, Merge tool; per workspace, Rename and Remove; Add workspace; the Advanced toggle; the File-change checkbox |
| History (`view/history.rs`) | the search field; per entry, Show/Hide details and Copy log |

**The rules:**

- **The same look at rest,** compared by pixels, with two exceptions:
  - the selects (Review 298 F3: the shorter snora outline, 16 px text);
  - the checkbox (Review 298 F4: snora's box).
- **Ids:**
  - a per-workspace id is built from the workspace's id;
  - a per-entry id is built from the History entry's own stable key.

  Name that key, and show that it survives a new entry arriving above it.
- **The File-change checkbox draws its label** (`settings.fs_watch_enable_label`: "Refresh when
  files change (experimental)" / 「ファイルが変わったら再読み込み（実験的機能）」). Remove the
  separate text beside it, so the words are part of the target. Check that the label wraps
  nowhere, at 150 % in Japanese.
- **Each select's open state lives in `AppState`:**
  - at most one is open on the screen;
  - choosing closes it;
  - leaving the screen closes it;
  - Escape closes only the select.
- **Choosing a language, theme or zoom** rebuilds the whole screen. Focus must stay on that same
  select (A1 of stage 2: by id).
- **Any control that disables itself when pressed** joins `reserved` while it is drawn
  (Amendment A2 of stage 3). List each one, or say there is none. Check Copy log, and Remove
  while the interlock is busy.
- **Rename and Remove open windows.** Stage 3's rules apply: entry when opened from the keyboard,
  and return on close.
  - Removing a workspace takes away its row. Focus then goes to its neighbour.
- **No entry is set on a screen change** (D5). With focus on the header's Settings or History,
  the next Tab reaches the screen's first control.

## 3. What retires

Whatever nothing uses any more in `knotra-app` (clippy will say). Stage 5 removes the old
primitives from `knotra-ui`, so leave those.

## 4. CHANGELOG

In `[Unreleased]`, replace the existing entry's heading and first bullet, exactly:

```markdown
### Changed — The keyboard reaches everything, on every screen and in every window

- Tab now reaches every control on every screen and in every window, including Settings and
  History, the Group and Sort choices, the buttons shown when there are no projects or no
  matches, the details panel, and Undo.
```

The other bullets stay as they are.

## 5. Tests and captures

| Question | Level | How |
|---|---|---|
| Settings' Tab sequence, by id, wrapping both ways | 1 | the real key path. Advanced closed; Advanced open with the checkbox off; and with it on (the interval select appears). One workspace (its Remove disabled and skipped) and two |
| History's Tab sequence | 1 | the real key path, with two entries, one expanded |
| Each select opens, ↓ moves, Enter chooses, and focus stays on it | 1 | Language, Theme and Zoom, each rebuilding the screen |
| A new History entry above the focused one | 1 | focus stays on its entry |
| Rename and Remove from the keyboard: entry, return, and the neighbour after a removal | 1 | the real key path |
| Escape with a select open | 1 | closes only the select |
| The look at rest | 2 | pixels, all four themes, except the two named exceptions |
| The guards cover the new controls | 1 | each **seen failing first** |

**Captures**, by the owner's GUI rules (`.git-exclude/rules/project-instructions-gui-common.md`):

- Task 108's session (`--scene demo`), with a settle delay after launch (Request 175 §5);
- the same script before and after, the before built from `origin/main`;
- every XDG folder isolated;
- every path, name and time on screen read, and the PNG chunks audited.

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | **Do Settings' selects sit well beside the text fields?** (the owner's open decision, Review 299 §3) | Japanese, 150 %, Light: Settings scrolled so that selects and the Editor field show together, Theme focused by Tab | the same view on `origin/main`, with no focus |
| C2 | Does the checkbox read well with its label drawn? | English, 100 %, Dark: Advanced open, the checkbox ticked and focused, the interval select below it | the same view on `origin/main` |
| C3 | Is History's focus clear? | English, 100 %, Light: one entry's details shown by Enter, Copy log focused | the same view on `origin/main` |

Some baselines need a click on `origin/main`, because nothing there is reachable by keyboard. If
the session's click does not register, say so, and give the after capture alone.

## 6. Report

- §2's ids, the stable History key, `reserved`, and the checkbox's label.
- §3's retirements.
- The tests, with failing-first output for the guards.
- The three capture pairs, with the privacy inspection.
- Gates, with each crate's printed counts and their sum.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog and `git worktree list`.
