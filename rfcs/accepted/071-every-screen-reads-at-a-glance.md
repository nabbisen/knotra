# RFC-071 - Every screen reads at a glance

| Field | Value |
|---|---|
| Status | Accepted |
| Priority | High - every user, every screen: the most important text is the smallest, and status has no colour |
| Effort | Medium to large - four stages, each shippable alone: type, colour roles, layout, zoom |
| Target | 0.33.0 (`.git-exclude/reviewed/244-release-plan-0.33.0-scope.md` §7) |
| Related files | `crates/knotra-ui/src/{theme,widget/chip,widget/buttons,widget/layout}.rs`, `crates/knotra-app/src/view/{shell,history,settings}.rs`, `crates/knotra-app/src/view/dashboard/{mod,row,section,toolbar}.rs`, `crates/knotra-app/src/{config,app}.rs`, `crates/knotra-ui/src/i18n.rs` |
| Decided by | project owner, 2026-10-31 - "Accepted", with D12's texts. Row style: the owner asked for one integrated style, chosen against the design philosophy; ruled in D8a. **Architect correction before the handoff (2026-10-31):** D3 drew the tone as coloured text; snora's tone tokens are backgrounds, so D3 now draws a badge. Same meaning: colour plus symbol plus words |
| Amended | Handoff Amendment A1, "kind to older eyes" (D13: readable unavailable controls, no hover-only meaning, one contrast suite), approved by the owner 2026-10-31. Handoff Amendment A2, "strong is bold; the detail panel's labels stack" (Review 248). Handoff Amendment A3, "text you read to act is never small" (D14: 16 px for everything read to act, 14 px only for incidental metadata; D9 revised so text size never gives way to row count), approved by the owner 2026-10-31 |
| Related | `.git-exclude/reviewed/245-visibility-assessment.md` (the findings L1-L5, T1-T3, C1-C5 cited below); RFC-056 (type roles, 12 px floor); RFC-057 (snora palette); RFC-035 R8 (width bands; D9 revises its wide band); RFC-066 (focus orders; D8 changes two); RFC-070 (high contrast, built on D4) |

## Summary

knotra's screens are correct but hard to read at a glance:

- the project names and statuses are the smallest text on the screen;
- nothing is bold;
- a project that needs help looks the same as one that is all set;
- blue marks almost everything;
- on a wide window, the project list floats away from the controls that filter it.

This RFC fixes how the screens **look**, in four stages:

1. **Type:** important text is larger and stronger.
2. **Colour roles:** status gets a colour and a symbol; blue means "this acts".
3. **Layout:** one left edge, rows that read as rows, labelled header controls.
4. **Zoom:** a setting, and Ctrl+Plus / Ctrl+Minus / Ctrl+0, to make all of knotra larger.

No behaviour changes, except that one duplicate "Check now" goes (D8). No text changes,
except the new keys of D12.

## Problem

Measured at `da83175` (tag `0.32.1`) and in the committed Store images
(`packaging/windows/screenshots/en/01`, `02`, `05`, 1920×1080). Review 245 §1 has the full
list. In short:

1. **Type** (T1-T3).
   - `theme.rs:83` lowers snora's `body_small` from 14 px to **13 px**, and `body_small` is
     the role at 145 sites in `knotra-app`, against 81 for `body`. Project names, statuses,
     group headers and history entries are all `body_small`, while the Group and Sort
     controls are larger.
   - No text anywhere has a weight.
   - There is no way to enlarge knotra, except the desktop's own scaling.
2. **Colour** (C1-C5).
   - snora's palette has `success`, `warning`, `danger` and `info`. On the dashboard, none is
     used.
   - `knotra-ui` defines its own `StatusColor` with hard-coded hex values (`theme.rs:11-48`).
     No view uses it, and no high-contrast theme could ever reach it.
   - Blue marks names, toggles, tabs, icons, links and both kinds of button.
   - History shows two filled buttons side by side, on every row.
   - Search placeholders are close to invisible: no text input sets a style, so they take the
     base theme's default.
   - The status chips are identical grey outlines with no count.
3. **Layout** (L1-L5).
   - In the wide band (≥1280 px), `dashboard/mod.rs:247` centres a 1200 px column
     (`WIDE_CONTENT_WIDTH`) while the toolbar above it starts at the left edge: two left
     edges.
   - History stretches each row over the full width, so a result sits 1,600 px from its
     title.
   - Rows have no surface or separator.
   - "Choose resolution" wraps inside a 100 px action track (`row.rs:35`).
   - The VCS label ("Git") is 10 px left of the name above it.
   - The header's refresh, palette and settings controls are 12 px icons with no visible
     label. The refresh icon is a second "Check now" (`shell.rs:211`, `plain.check_now`)
     beside the dashboard's own "Check now" button.

## Design

### Stage 1 - type

#### D1. Roles by importance, not by habit

| Text | Role today | Role after |
|---|---|---|
| page title ("Dashboard", "What happened", "Settings") | `heading` | `heading`, **strong** (D2) |
| group header ("Needs help (1)") | `body_small` | `title`, **strong** |
| project name | `body_small` | `body`, **strong** |
| a project's status line ("Unsaved files: 1") | `body_small` | `body` |
| branch, VCS kind, timestamps, counts in metadata | `body_small` | `body_small` |
| history entry title | `body_small` | `body`, **strong** |
| dialog title | `title` | `title`, **strong** |

- **`body_small` returns to snora's 14 px.** Delete `with_knotra_typography`'s override
  (`theme.rs:82-85`). 13 px was below snora's own scale, for density that D9's layout now
  provides instead.
- Every other site keeps its role, unless the handoff lists it with a reason.

#### D2. "Strong" is one weight, and never the only signal

- **One helper in `knotra-ui`**, `strong(text)`, applies `Font { weight: Semibold, ..
  Font::DEFAULT }`. No view names a weight itself.
- **A text-scanning guard** (the shape of `text_size_guard.rs`) fails on any `Weight::`
  outside the helper.
- **Weight only reinforces.** Every strong text also differs in role (size) or position
  from its neighbours, so a system font with no semibold face loses only emphasis, never
  meaning. iced falls back to the nearest available face.
- **Measured first.** Stage 1 begins by rendering Latin and Japanese, regular and strong,
  on the dev team's Linux, and on the Windows package of a dispatched release run (owner's
  check, one screen). If either shows no visible difference, or a smeared synthetic bold,
  **stop and report**. Bundling a font is then an owner decision, with sizes: a Japanese
  font adds megabytes to every download.

### Stage 2 - colour roles

#### D3. Each status has a tone and a symbol

| Status (the dashboard's attention groups and chips) | Tone | Symbol |
|---|---|---|
| Needs help, Needs a choice, unreadable | `danger` | alert |
| Unsaved work | `warning` | pencil / dot |
| Updates available | `info` | arrow down |
| Local commits | `info` | arrow up |
| All set | `success` | check |

- The symbols come from the `lucide-icons` set already enabled. The handoff names each one.
- **How the tone is drawn: a badge, not coloured text.** snora's `success`, `warning`,
  `danger` and `info` are **background** colours, each with a `*_text` colour for what sits
  on them (`snora-design` `palette.rs`). Used as text on the page background they can fail
  contrast; `warning` on white is the obvious case. So:
  - the tone is a small badge: a filled shape in the tone colour, with the symbol drawn in
    its `*_text` colour;
  - the status **words** stay in `text_primary` (or `text_secondary` for metadata).
  - One `knotra-ui` helper, `status_badge(tone)`, draws it. No view colours a status itself.
- **Where it shows:**
  - before the row's status line;
  - in each group header;
  - on each status chip (D5).
- **The colours come from snora's palette tokens**, never a literal, so RFC-070's
  high-contrast presets restyle the badges with no further change.
- **A test** checks each tone pair: the symbol colour (`*_text`) against its badge colour,
  at least 4.5:1, in both themes. The theme tests then cover RFC-070's presets the same
  way.
- **Never colour alone** (WCAG 1.4.1): the symbol and the words carry the meaning too.
- **`StatusColor` is removed** from `knotra-ui`. It is unused, it holds literals, and it
  would stay outside every theme.
  - It is public API on crates.io. 0.33.0 is a minor release, so the removal is allowed.
  - The CHANGELOG says so under "Removed".

#### D4. Blue means "this acts"

- **Accent (blue) is kept for:** buttons, links, the selected tab, and controls that change
  something.
- **Project names** become `text_primary`, strong (D1).
  - They still open the project's details on click.
  - On hover and on keyboard focus they show the accent and an underline, so what is
    clickable is visible when it matters.
- **Group toggles** become `text_secondary`, with their chevron. The chevron, not the
  colour, says "this opens and closes".
- **Header icons** follow D10.

#### D5. One primary action per area, and chips that inform

- **The rule:** within one area (a row, a dialog footer, a page header) at most one button
  is filled (`primary`). The others are `secondary` or `ghost`.
  - **Known breaks:**
    - History's "Show details" + "Copy log", on every row: both become `secondary`,
      since an entry's actions are not the page's main act;
    - the activity strip's "Details": becomes `secondary`.
  - The handoff lists every other break it finds.
  - A test asserts, per view, at most one `style::primary` per row or footer builder where
    that is checkable.
- **Status chips** show their count and their tone's symbol: "⚠ Needs help 1".
  - A chip with count 0 stays visible, in `text_secondary`, so the row of chips does not
    jump.
  - The selected chip keeps today's selected style.

#### D6. Placeholders are readable

- **Every `text_input`** in knotra gets one style from `knotra-ui`.
- **Placeholder colour:** `text_muted`, which the theme's AA test already covers
  (`theme.rs:260-286`).
- **Typed text:** `text_primary`.
- The placeholder stays visibly lighter than typed text, and readable.

### Stage 3 - layout

#### D7. One left edge, and a readable width (revises RFC-035 R8's wide band)

- **In every band, the page content starts at the same left edge as the toolbar above it.**
  The page content is the toolbar, the chips and the project list.
- **The wide band keeps its 1200 px maximum width.** Content then ends at 1200 px, and the
  rest of the window is empty on the **right**, not split around a centred column.
  RFC-035 R8's "content centred" is replaced by "content left-aligned, at most 1200 px".
- **History uses the same maximum width.** An entry's result and actions then sit within
  1200 px of its title.
- **Settings uses it too.**

#### D8. Rows read as rows; one "Check now"

- **Rows are separated by lines, by one rule everywhere (D8a).**
  - A group header has more space above it than between rows, so it reads as a heading.
- **The VCS kind** ("Git", "jj") lines up with the project name.
- **The action track fits its labels.** `ACTION_TRACK_WIDTH` grows to fit the longest
  action label in both languages, at `body_small`, plus the button padding. This is
  measured, like Task 098's columns, so "Choose resolution" / 「解決方法を選ぶ」 sit on one
  line.
- **One "Check now".** The header's refresh control (D10) becomes the only "Check now".
  - The dashboard's own "Check now" button is removed.
  - The header control is visible on every screen; the dashboard button was a second
    route to the same message.
  - **RFC-066's dashboard focus order loses one entry.** Its test changes by exactly that.

#### D8a. One list style for all of knotra: lines, never panels

**The rule:** every list of items in knotra separates its items with a 1 px line in the
`border` token. No list item sits on a panel of its own. A raised panel (`surface`,
`raised_card`) keeps exactly one meaning: **something floating above the screen**. Today
that means the windows (RFC-033 D3), the workspace menu (`shell.rs:347`) and tooltips
(`buttons.rs:321`, `checkbox.rs:62`).

The lists the rule covers:

- the dashboard's project rows and history entries;
- the lists inside windows: the ready check, the "Get latest safely" plan, the results,
  and the command palette's results;
- any other list the handoff finds.

**Why lines and not panels**, against the owner's design philosophy:

- **Never confusing.** A panel already means "a window, a menu or a tooltip". Panels on
  every row would give one visual sign two meanings. Inside a window they would put panels
  on a panel, and the user could no longer tell what floats above what.
- **Robust.** In high-contrast themes (RFC-070), a panel's only edge is a slight surface
  tint, and that tint can vanish against the background, taking the separation with it. A
  line is drawn in `border`, which a high-contrast theme strengthens.
- **Clean.** A line costs 1 px. Panels cost padding and gaps on every row, which works
  against D9's limit on lost rows.
- **Sophisticated.** One device with one meaning, used the same way on every screen and in
  every window, rather than a style chosen screen by screen.
- **Working with the other states.** A selected or hovered row is shown by a full-width
  tint, and the focus outline is drawn on top. On a line-separated row that is one clear
  signal. On a panel it would be a third frame inside the panel's own edge and the outline.

**The details:**

- **Where the lines go:** only between items, not above the first or below the last.
  - A group header already separates groups, by D1 and D3.
  - The command palette's line falls between commands, never between a command and its
    reason. The reason stays visibly part of its command.
- **One helper:** a `knotra-ui` list builder draws the separators. No view draws its own
  line.
- **Legacy constants removed:** `CARD_RADIUS`, `CARD_GAP`, `CARD_PADDING`,
  `CARD_MIN_WIDTH` and `SIDEBAR_WIDTH` (`knotra-ui/src/widget/layout.rs`). They are left
  over from the card grid that RFC-035 replaced, are used nowhere, and invite the panels
  this rule rules out. They are public API, so the removal goes in the CHANGELOG under
  "Removed", with D3's `StatusColor`.

#### D9. The density moves from type to layout

D1 makes rows taller. To keep a full HD window showing about as many projects as today:

- **Within a row:** spacing comes from `tokens.spacing`, with no extra padding.
- **Status in one line:** a row's status is one line ("main · 1 unsaved file") where it
  fits the middle track. It wraps only in the compact band.

The handoff reports how many rows a 1920×1080 window shows before and after. A loss of more
than one row is a stop-and-report.

#### D10. Header controls say what they do

- **Visible labels.** Refresh ("Check now"), the command palette and Settings show their
  label beside the icon, using the existing keys `plain.check_now`, `palette.title` and
  `nav.settings`.
- **Size.** The icon is at least 16 px, and the control is at least `SMALL_BUTTON_HEIGHT`.
- **Compact band:** icons only, with the label as a tooltip, as today.

### Stage 4 - zoom

#### D11. Zoom, saved with the settings

- **The control.** A Settings choice, **"Zoom"**: 100 %, 115 %, 130 %, 150 %.
- **How it works.** It sets iced's application `scale_factor`, so text, controls and
  spacing grow together. A text-only size would break every fixed track width.
- **Saved.** `AppConfig.zoom_percent: u16`, default 100. An unknown value loads as the
  nearest allowed one, and an older settings file loads as 100. Tests for both.
- **The width bands follow.** At 150 % on a 1280 px window, the logical width is 853 px,
  which is compact. That is correct, and needs no code: `WidthMode` already reads the
  logical width.
- **Shortcuts.**
  - **Ctrl+Plus** (and Ctrl+=) steps up, **Ctrl+Minus** steps down, **Ctrl+0** returns to
    100 %.
  - They are saved like the Settings choice.
  - They work only when no window is open, under RFC-066's rule.
  - The shortcuts sheet lists them, so it stays truthful (RFC-066 R2).

#### D12. Texts (owner approval needed)

| Key | English | Japanese |
|---|---|---|
| `settings.zoom_label` | Zoom | 表示倍率 |
| `settings.zoom_help` | Makes everything in knotra larger or smaller, text included. | 文字を含め、knotraの表示全体を拡大・縮小します。 |
| `shortcut.action_zoom_in` | Zoom in | 拡大 |
| `shortcut.action_zoom_out` | Zoom out | 縮小 |
| `shortcut.action_zoom_reset` | Reset zoom | 倍率を元に戻す |

Percent labels ("115 %") are numbers, not catalog text. No other text changes.

## Requirements

- R1. D1's table holds, and `body_small` is 14 px. `text_size_guard` passes.
- R2. Weight appears only through `strong`, enforced by the new guard. D2's measurement is
  reported before any other stage 1 change.
- R3. Every status shows tone badge, symbol and words, through `status_badge` only. The
  contrast test covers every tone pair, in both themes.
- R4. `StatusColor` is gone, recorded in the CHANGELOG under "Removed".
- R5. D5's rule holds on History and the activity strip. Every other break found is listed.
- R6. All text inputs use the one style, and the placeholder is `text_muted`.
- R7. One left edge in every band, and a 1200 px maximum on the Dashboard, History and
  Settings. RFC-035 R8's tests are updated to the new wide band, and only there.
- R8. One "Check now". RFC-066's dashboard order test changes by that one entry, and no
  other focus test changes.
- R8a. D8a: one list builder draws every separator; no list item sits on a panel; the
  `CARD_*` and `SIDEBAR_WIDTH` constants are gone, and recorded under "Removed".
- R9. Rows shown at 1920×1080: before and after reported, with a loss of at most one.
- R10. Zoom: the four steps, saved, older and unknown values handled, the shortcuts gated
  by RFC-066 and listed in the sheet.
- R11. Each stage is its own commit set, with its own CHANGELOG entry and its own evidence.
  It can ship without the stages after it.
- R12. RFC-070 starts after stage 2 is approved.

## Evidence (per stage, under the GUI evidence rules)

Every stage has one question, before and after images from two builds and one script,
isolated XDG folders, and one instance.

| Stage | The question | Captures |
|---|---|---|
| 1 | Is the important text larger and stronger, in both languages? | Dashboard, History; light; en, ja |
| 2 | Can you tell each status apart without reading it, and is blue only on things that act? | Dashboard light and dark, en; History light, en |
| 3 | Is there one left edge, and do rows read as rows? | Dashboard at 1920 and 1000 px wide; History at 1920; light, en |
| 4 | Does zoom enlarge everything evenly? | Dashboard at 100 % and 150 %; en, ja |

**After stages 2 and 3, the ten Store images no longer match.** 0.33.0's release prep
recaptures all ten, and I audit them as before.

## Out of scope

- High contrast (RFC-070, after stage 2).
- F6 (RFC-069, 0.34.0).
- Any change to wording beyond D12.
- New features on any screen.
- The detail panel's own layout, unless a D-rule above names it.
- A bundled font: only if D2's measurement fails, by the owner's decision.

## Owner inputs

1. **Accept**, with D12's texts.
2. **D2's Windows check:** one screen, from a dispatched build's package, when stage 1 asks
   for it.
3. ~~D8's row style~~: ruled in D8a. The owner asked for one integrated style, chosen
   against the design philosophy.
