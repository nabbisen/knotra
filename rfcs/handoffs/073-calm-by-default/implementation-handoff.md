# Developer Handoff — RFC-073: calm by default

RFC: `rfcs/accepted/073-calm-by-default.md`, **accepted 2026-11-03 (project owner), with D16's
texts.**

- D5: the "Needs a choice" chip goes.
- D6: the labelled command palette stays in the header, with a neutral icon.

Read the RFC in full first, and Reviews 257–260 behind it.

**Start after 0.33.0 is tagged.** 0.34.0's work must not mix into 0.33.0's release. Baseline:
`origin/main` at that point. Report the commit and its test count.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard

An older user opening knotra meets their projects first, then only what they need:

- one word for each action, in both languages;
- one sign for each status;
- targets they can hit;
- settings they understand.

**Nothing is taken away; it moves one step out.**

**Five stages, five review requests.** Each stage ends with every gate green, its CHANGELOG
entries, its evidence and its own review request. Do not start a stage before the previous
one is approved.

**Text:** only D16's, exactly. Any other wording is a **stop and report**.

## Stage 1 — show less (D1–D7)

1. **D1.** With no projects in the active workspace, the dashboard draws neither the toolbar
   (chips, Arrange, Search, Select) nor the selection bar. Only the welcome block remains.
2. **D2. "Arrange"** (`dashboard.arrange`): one control beside Search. It opens a menu holding
   the existing Group and Sort selects, unchanged in behaviour and saving.
   - In standard and wide bands, it replaces the two selects in the toolbar.
   - In the compact band, it takes their place in the existing overflow.
   - RFC-066's dashboard focus order: "Arrange" replaces `GROUP_SELECT` and `SORT_SELECT`.
     Report each focus test's diff.
3. **D3.** The kind label under a name is drawn only when the active workspace holds both Git
   and Jujutsu projects. Unit-test the rule.
4. **D4.**
   - **The header:** with one workspace, its name is plain text; with two or more, today's
     menu.
   - **The count goes** from the name (`shell.rs:140-158`).
   - **Settings gains a "Workspaces" section** (`settings.section.workspaces`). It lists each
     workspace with its existing Rename and Remove, plus the existing "New workspace", opening
     the existing dialogs. The palette entries stay.
   - **Remove keeps its confirmation**, and stays unavailable for the last workspace, with
     its existing reason.
5. **D5.** Remove the "Needs a choice" chip (`StatusFilter::Conflict` from the chip row and the
   filter, and `filter.conflict` from both catalogs).
   - **Before deleting the enum variant,** enumerate every use with `git grep` and open each.
     If it is persisted (a saved filter), **stop and report**: an older settings file must
     still load.
6. **D6.** The palette's header control uses a neutral icon from the existing set, not "⌘".
   Name the icon. The label stays.
7. **D7.** Remove the "Toggle theme" palette entry (`state/palette.rs:142`) and
   `palette.action.toggle_theme`.
8. **CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Changed — the dashboard shows less at once

With no projects yet, the dashboard shows only how to add your first one. "Group" and
"Sort" are now together under "Arrange". A project's kind (Git or Jujutsu) is shown only
when both kinds are in the workspace. With a single workspace, its name is no longer a menu;
workspaces are created, renamed and removed in Settings. The "Needs a choice" filter is gone:
those projects are all under "Needs help", each with its reason.
```

**Evidence.** The question: is there less to read before the first project, and does
everything still work?

- Dashboard: empty, and with the Task 099 A1 demo; with one workspace and with two.
- Settings → Workspaces.
- The Arrange menu open.
- English; one Japanese dashboard.
- Before and after, same script.

## Stage 2 — hands and eyes (D8–D10)

1. **D8.**
   - Every pressable widget built by a `knotra-ui` helper is at least `SMALL_BUTTON_HEIGHT`
     (36) tall, and 44 where the row allows: chips, header controls, group toggles, project
     names, Arrange. The text keeps its size; the hit area grows.
   - **A guard test** asserts a minimum height per helper.
   - **Enumerate** every pressable built outside the helpers (`button(` in `view/`), and bring
     each to the rule or report why not.
2. **D9.** `status_badge` replaces "!", "✓", "✗" and "-" in:
   - the ready check (`freezer.rs`);
   - Smart Pull's plan and results;
   - Change work area's results;
   - History's entry outcome and per-project lines.

   One exhaustive function maps each outcome to tone and symbol (success / check; failure /
   alert; skipped / info with a minus). The words stay.
3. **D10.** The badge symbol is at least 14 px, and the badge at most the text's line height
   plus 4 px. Report the sizes in both text sizes the badge sits beside.
4. **CHANGELOG:**

```markdown
### Changed — everything you can press is easier to hit, and every status looks the same

Filter buttons, header controls, group headings and project names now have a larger area to
click. Every window now shows a project's status with the same coloured symbol as the
dashboard, and the symbol itself is larger.
```

**Evidence.** The question: are the targets larger, and is status shown one way everywhere?

- Dashboard; the ready check; History with an entry open; Smart Pull results.
- English; light and dark; before and after.

## Stage 3 — calm Settings (D11–D13)

1. **D11.**
   - Settings shows Display (Language, Theme, Zoom), Refresh automatically, External tools and
     Workspaces.
   - **"Advanced"** (`settings.section.advanced`), closed by default: Simultaneous reads,
     Operation log size, and File-system monitoring with its interval.
   - Its open or closed state is not saved: it is closed on every visit.
2. **D12.** The four number settings become selects, with D12's choices and D16's labels.
   - Plain numbers ("2", "16", "100", "1000") need no catalog text.
   - The interval's choices show the number with the existing unit `settings.unit_seconds`
     ("2 seconds" / "2 秒"). Report each label you build and how.
   - **Loading:** a saved value that is not a choice loads as the nearest one, ties going
     downward, the same as zoom. Test each setting.
   - **Saving:** at once (A7).
   - The typed-value validation and its error keys go if nothing else uses them. Report
     which.
3. **D13.**
   - **Find, by reading the code,** which settings take effect only after a restart. For
     each, report the reason and the line.
   - Each such setting carries `settings.needs_restart` under it.
   - `settings.restart_hint` goes.
4. **CHANGELOG:**

```markdown
### Changed — Settings shows what most people use

Settings now shows language, theme, zoom, automatic refresh, your tools and your
workspaces. Settings for fine-tuning are under "Advanced", closed until you open it. Numbers
are now chosen from a list instead of typed, and a setting that needs a restart says so.
```

**Evidence.** The question: is Settings shorter, and are the choices readable?

- Settings closed and with Advanced open; one select open.
- English and Japanese.

## Stage 4 — one word per action (D14–D16)

1. Apply **every row of D16's tables**: the changed values, the new key W3
   (`plain.selection.all_in_closed_groups`) and the removed keys.
   - W3 shows only when projects exist and every one is in a closed group.
   - The one corrected reuse: `context_switch.rs:166` uses `detail.loading`.
2. **A test** asserts that no English value in the catalog uses "Check" or "check" outside
   "Check for updates", "Only check for updates" and "Ready check" phrases. Name it in the
   report. It keeps "check" to one meaning.
3. **Report** the catalog diff in both languages, and confirm every D16 value byte for byte.
4. **CHANGELOG:**

```markdown
### Changed — one name for each action

"Check" now always means checking for updates. Re-reading your projects' status is
"Refresh" everywhere, and the project details use the same words as the dashboard
("Unsaved files", "Local commits") instead of version-control terms. In Japanese, a few
actions and states that had two names now have one.
```

**Evidence.** The question: does each action read the same everywhere, in both languages?

- The header; the detail panel; Get latest's plan; the palette (search "check"); the ready
  check.
- English and Japanese, after only. The words are the change, and the catalog diff is the
  proof.

## Stage 5 — your data (D17, then an amendment for D18)

1. **D17, measure.** For tar.gz on Linux, `cargo install` on Linux, and the Microsoft Store
   package on Windows, record where these live on disk:
   - `config.toml`;
   - the workspaces folder;
   - the history folder.

   Record also what remains after uninstalling, and the uninstall method used.
   - **Linux:** you measure.
   - **Windows / Store:** write a checklist of five minutes at most for the owner: install,
     open, add one project, close, look in these places, uninstall, look again. File it with
     the stage's review request. **Do not guess the Store result.**
2. **Stop** after D17 and file the measurements. D18's texts and design come back to you as
   an amendment once the owner has the facts. They cannot be written before.

## Out of scope

- RFC-074's flow changes.
- RFC-072's removal action.
- F6.
- Issue #4.
- Any colour, type or theme rule.
- Any text not in D16.

## Verification, every stage

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <stage baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
```

Requirements:

- Suppression map: five.
- Dead-code probe: one line.
- `advisories ok`.
- Report all test counts, and explain each delta.

**GUI evidence:**

- every capture answers its stage's question;
- before and after come from two builds and one script, at equal sizes;
- isolate all four XDG folders, and run one instance;
- use fictional repositories under `/tmp`;
- every image claim names its file;
- save to `.git-exclude/evidence/rfc-073/stage-N/`, and do not commit them.
