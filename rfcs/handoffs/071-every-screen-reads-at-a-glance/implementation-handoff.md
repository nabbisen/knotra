# Developer Handoff — RFC-071: every screen reads at a glance

RFC: `rfcs/accepted/071-every-screen-reads-at-a-glance.md`, **accepted 2026-10-31 (project
owner), with D12's texts.** Read it in full first, including D8a (lines, never panels) and
the corrected D3 (a tone badge, not coloured text). Background: Review 245.

**Start after Task 102 is approved.** Baseline: `origin/main` at that point. Report the
commit and its test count.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard

A user glancing at any screen can tell:

- what matters most;
- which projects need them;
- what they can press.

No meaning is carried by colour alone or by weight alone. Every colour comes from a token,
so RFC-070's high-contrast themes restyle everything with no further change.

**Four stages, four review requests.** Each stage:

- ends with every gate green, its CHANGELOG entry, its evidence and its own review request;
- can ship without the stages after it.

Do not start a stage before the previous one is approved.

**New text:** only D12's. If any step seems to need other wording, **stop and report**.

## Stage 1 — type (D1, D2)

1. **`body_small` back to 14 px.** Delete `with_knotra_typography`'s override
   (`knotra-ui/src/theme.rs:82-85`), and its call sites if nothing else remains in it.
2. **`strong`.**
   - In `knotra-ui`, add `strong(text) -> Text` setting
     `Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }`.
   - Add a guard, the shape of `text_size_guard.rs`: no `Weight::` in `crates/` outside
     that helper.
   - **Seen to fail first:** with one temporary `Weight::` in a view.
3. **D1's table.** Apply it:
   - page titles;
   - group headers (`dashboard/section.rs:90`, `:106`);
   - project names and status lines (`dashboard/row.rs`);
   - history entry titles (`history.rs`);
   - dialog titles (`knotra-ui/src/widget/overlay.rs:136`).

   Every other site keeps its role. If you change one, list it with the reason.
4. **D2's measurement comes first in your report.** Look at the after captures at full
   resolution. Is strong text visibly heavier than regular, in Latin **and** Japanese,
   without smearing?
   - If it is not, **stop and report** before anything else in this stage is filed.
   - The Windows check is the owner's. Dispatch the release workflow on your stage-1 head,
     and give its run ID in the report; I ask the owner to look at one screen.
5. **CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Changed — project names, statuses and headings are easier to read

Project names, their status, group headings and History entries were drawn in the smallest
text on the screen. They now use the normal text size, and names and headings are drawn
stronger. The smallest text is also slightly larger.
```

**Evidence.** The question: is the important text larger and stronger, in both languages?

- Dashboard and History, light, English and Japanese.
- Before and after, from two builds and one script.

Also report how many project rows a 1920×1080 window shows, before and after. Stage 3's D9
holds the final count. Here, report the number only.

## Stage 2 — colour roles (D3, D4, D5, D6)

1. **`status_badge(tone)`** in `knotra-ui`.
   - A small filled shape in the tone's palette colour (`success` / `warning` / `danger` /
     `info`), with the symbol drawn in its `*_text` colour.
   - Size: the line height of the text it precedes.
   - **The symbol for each tone,** from the enabled `lucide-icons` set. List your choices in
     the report.
2. **D3's map.** Find the enum that decides a project's attention group and chip. The tone
   is a function of that enum, exhaustive, with no `_` arm, so a new status cannot go
   without a tone. Place the badge:
   - before each row's status line;
   - in each group header;
   - on each chip.
3. **Contrast test.** For each tone, in light and dark: `*_text` against the tone colour is
   at least 4.5:1, using `snora::design::contrast_ratio`.
4. **Remove `StatusColor`** (`knotra-ui/src/theme.rs:9-48`) and its re-exports. Show
   `git grep StatusColor` empty.
5. **D4, blue means "this acts":**
   - **Project names:** `text_primary`, strong. On hover and on keyboard focus, accent and
     underline. The click target and its message are unchanged.
   - **Group toggles:** `text_secondary`, with the chevron.
   - Report every other blue element you find that does not act. **Change none of them
     without listing it.**
6. **D5, one filled button per area.**
   - History's "Show details" and "Copy log" become `secondary`.
   - The activity strip's "Details" becomes `secondary`.
   - **Chips:** each shows its badge and its count. At count 0 the chip is drawn in
     `text_secondary` and stays in place. The count is a number, not new text.
   - **Enumerate:** list every builder that can put two `style::primary` buttons in one
     row or footer. Fix each one, or report why it is right as it is.
7. **D6, placeholders.**
   - One `text_input` style in `knotra-ui`: placeholder `text_muted`, value `text_primary`,
     the rest from tokens.
   - Apply it to every `text_input` in `crates/`. Show the `git grep` count before and after.
8. **CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Changed — each status has its own colour and symbol

A project that needs help now looks different from one that is all set. Each status has a
coloured symbol beside its words, in the project list, in the group headings and on the
filter buttons, which now also show how many projects each one matches. Blue now marks
only what you can click or press, and on History an entry's buttons are no longer both
highlighted.

### Changed — the hint in an empty search box is easier to read

It was drawn so faintly that it was hard to see. It is now readable, and still lighter than
what you type.

### Removed — `StatusColor` from the `knotra-ui` crate

For developers using `knotra-ui`: this enum held fixed colours that knotra did not use and
that no theme could change. Status colours now come from the theme's palette.
```

**Evidence.** The question: can you tell each status apart without reading it, and is blue
only on things that act?

- Dashboard: light and dark, English.
- History: light, English.
- Before and after.

## Stage 3 — layout (D7, D8, D8a, D9, D10)

1. **D7, one left edge.**
   - In `dashboard/mod.rs:247`, replace the wide band's `center_x` with left alignment at
     the toolbar's edge, keeping the 1200 px maximum (`WIDE_CONTENT_WIDTH`).
   - Give History and Settings the same maximum, through one shared constant.
   - Update RFC-035 R8's tests for the wide band, and only those. Report each change.
2. **D8a, lines everywhere.**
   - Build one `knotra-ui` list helper. It draws a 1 px `border` line between items, never
     above the first or below the last.
   - Use it for:
     - the dashboard rows;
     - the history entries;
     - the ready check, the "Get latest safely" plan and the results (`freezer.rs`,
       `smart_pull.rs`);
     - the command palette's results. The line falls between commands, never between a
       command and its reason.
   - **Enumerate** every other list of items with an instrument: `git grep` for `column(`
     built from a `.map(`/`.collect()` over items in `view/`, then each match opened. List
     what you found, and what you converted.
   - **No list item on a panel.** Show that `raised_card` and `overlay::surface` are called
     only for windows, menus and tooltips.
   - **Remove** `CARD_RADIUS`, `CARD_GAP`, `CARD_PADDING`, `CARD_MIN_WIDTH`, `SIDEBAR_WIDTH`
     and their re-exports (`knotra-ui/src/widget/{layout,mod}.rs`). Update `row.rs:25`'s
     comment, which names `SIDEBAR_WIDTH`.
3. **D8, the rest.**
   - **Group headers:** a larger gap above them than between rows, from `tokens.spacing`.
   - **The VCS label** lines up with the name.
   - **`ACTION_TRACK_WIDTH`** (`row.rs:35`): the longest action label in both languages,
     at its role, plus the button padding. Show how you measured it, as in Task 098.
   - **Remove the dashboard's own "Check now" button.** RFC-066's dashboard order test
     loses exactly that entry. Report the diff of that test, and confirm no other focus test
     changed.
4. **D9, density.**
   - Use one status line ("main · 1 unsaved file") in the standard and wide bands.
     - Join the existing catalog texts with ` · `; add no new words.
     - If a status cannot be joined without new wording, **stop and report**.
   - In the compact band, the line wraps.
   - Report the project rows shown at 1920×1080, against stage 1's baseline count. A loss
     of more than one row is a **stop and report**.
5. **D10, header controls.**
   - Refresh, command palette and Settings show their label beside the icon, from
     `plain.check_now`, `palette.title` and `nav.settings`.
   - The icon is at least 16 px; the control is at least `SMALL_BUTTON_HEIGHT` high.
   - In the compact band: icon only, with today's tooltip.
6. **CHANGELOG.** Add to `[Unreleased]`, exactly:

```markdown
### Changed — the dashboard and History line up, and rows are easier to tell apart

On a wide window, the project list now starts at the same left edge as the controls above
it instead of floating in the middle, and History's results sit close to what they
describe. Items in a list, on these screens and inside windows, are separated by thin
lines. The header's "Check now", command palette and Settings now show their names, and
the dashboard's second "Check now" button is gone: the one in the header does the same.
```

   Then extend stage 2's "Removed" entry. Its heading becomes `### Removed — `StatusColor`
   and unused layout constants from the `knotra-ui` crate`. Append to its body, exactly:
   `The constants CARD_RADIUS, CARD_GAP, CARD_PADDING, CARD_MIN_WIDTH and SIDEBAR_WIDTH,
   left over from an earlier layout, are removed as well.` (with each name in backticks).

**Evidence.** The question: is there one left edge, and do rows read as rows?

- Dashboard at 1920 and at 1000 px wide.
- History at 1920 px.
- One window with a list: the ready check.
- Light theme, English. Before and after.

## Stage 4 — zoom (D11, D12)

1. **`AppConfig.zoom_percent: u16`**, default 100. Allowed values: 100, 115, 130, 150.
   - An unknown value loads as the nearest allowed one.
   - An older file without the field loads as 100.
   - Tests for both.
2. **Scaling.** iced's application `scale_factor` reads `zoom_percent / 100`. `WidthMode`
   needs no change. Show a test or a capture where 150 % on a 1280 px window is compact.
3. **Settings.** A "Zoom" choice with D12's label and help, built with knotra's own select
   (`knotra-ui/src/widget/select.rs`), and saved like every other setting.
4. **Shortcuts.**
   - Ctrl+Plus and Ctrl+= step up; Ctrl+Minus steps down; Ctrl+0 returns to 100.
   - Add them in `shortcut_for` (`app.rs`).
   - Gate them by RFC-066's layer rule, so they do nothing while a window is open.
   - Save each change.
   - **The shortcuts sheet lists them** with D12's three keys. RFC-066's sheet test must
     still pass, extended to the new keys.
5. **Catalog:** D12's five keys, English and Japanese exactly. The unused-key test passes.
6. **CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Added — Zoom

In Settings, "Zoom" makes everything in knotra larger or smaller: 100 %, 115 %, 130 % or
150 %. Ctrl+Plus, Ctrl+Minus and Ctrl+0 do the same when no window is open.
```

**Evidence.** The question: does zoom enlarge everything evenly?

- Dashboard at 100 % and at 150 %, English and Japanese, from one build and one script.

## Out of scope

- High contrast (RFC-070).
- F6 (RFC-069).
- Any text beyond D12 and the CHANGELOG.
- Any behaviour beyond D8's removed button and D11.
- The detail panel's own layout.
- Store screenshots: release prep retakes them.

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

- Suppression map: **five**.
- Dead-code probe: **one** line.
- `advisories ok`.
- Report all test counts, `knotra-vcs` as 20 + 36, and explain each stage's delta.

**GUI evidence:**

- every capture answers its stage's question;
- before and after come from two builds and one script;
- isolate all four XDG folders, and run one instance;
- use fictional repositories under `/tmp`;
- save to `.git-exclude/evidence/rfc-071/stage-N/{before,after}/`, and do not commit them;
- inspect every image for privacy.

## What to report back, per stage

- Stage 1:
  - **D2's measurement first**;
  - the dispatched run ID;
  - the guard seen to fail;
  - every role change outside D1's table;
  - the row count at 1920×1080.
- Stage 2:
  - the symbols chosen;
  - the tone function;
  - the contrast test;
  - `StatusColor` gone;
  - every non-acting blue element, and what you did with it;
  - the two-primary enumeration;
  - the `text_input` counts.
- Stage 3:
  - the alignment change and RFC-035's test diffs;
  - the list enumeration, with what was converted;
  - the panel-call check;
  - the action-track measurement;
  - RFC-066's test diff;
  - the row counts;
  - the removed constants.
- Stage 4:
  - the config tests;
  - the compact-at-150 % proof;
  - the shortcut gating;
  - the sheet test;
  - the catalog diff.
- Every stage:
  - the CHANGELOG as committed;
  - the evidence paths, sizes, backend and privacy inspection;
  - gate output, with the `diff --check` range.
