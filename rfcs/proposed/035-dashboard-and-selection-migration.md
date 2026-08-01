# RFC-035 - Dashboard and Selection Migration

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - the dashboard is the primary repeated-use surface and carries four of the audit's five High findings |
| Effort | Medium-Large |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view/activity_strip.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-ui/src/widget/`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/036-...md` (focus model and Tab traversal R22/R23 depend on; **implemented, `main: d20c7be`**), `rfcs/proposed/040-...md` (`app.rs` decomposition; lands first, so this RFC's handler edits land in `app/misc.rs` rather than `app.rs`), `rfcs/done/033-...md` (D4/D5/D6/D7/D8), `rfcs/done/034-...md` (foundation this consumes), `rfcs/done/032-...md` (display semantics this must preserve), `rfcs/done/031-...md` (activity/lease semantics), `rfcs/done/027-...md` (selection semantics) |
| Related audit evidence | `.git-exclude/reviewed/062-current-gui-ui-ux-audit.md` findings 3, 4, 5; `.git-exclude/reviewed/060`, `066`, `068` |

## Summary

Migrate the dashboard and its selection surface onto the RFC-034 foundation.
This covers the audit's remaining High findings for this screen: control
semantics (finding 3), scan geometry and responsive behaviour (finding 4), and
repeated busy/disabled guidance (finding 5).

**Nothing about what the dashboard computes changes.** RFC-032's
`DashboardDisplay` pipeline - classification, filtering, ordering,
`ordered_selectable_ids`, and selection reconciliation - is settled and
authoritative. This RFC changes only how that result is rendered and how the
user acts on it.

Two decisions are made here that supersede earlier guidance, both discovered by
reading the available primitives rather than assuming them:

- **Status filter chips use `chip::filter`**, which carries `selected` natively
  and renders filled - so D4's "selected is filled, never disabled-looking" is
  satisfied by the primitive, not by a workaround.
- **Grouping and sorting become select menus**, per D4's explicit assignment,
  which also collapses today's four-row toolbar. This means **no `selected_or`
  helper is needed**, superseding review `068` H2.

## Background

### What the audit found on this screen

- **Finding 3 (High)** - filters, selected choices, project names, section
  expanders, row actions, and primary workflow actions all share the same
  saturated blue button treatment. Section headers look like full-width primary
  buttons. Selected state is communicated by appending `*`; selection uses `[ ]`
  and `[x]` text glyphs.
- **Finding 4 (High)** - at 1500x900 controls and row columns stretch across the
  window leaving large empty regions; at 800x600 long names wrap unpredictably
  and the toolbar consumes disproportionate vertical space. Rows use
  `FillPortion(4)`/`FillPortion(5)` with no bounded tracks or breakpoint
  behaviour.
- **Finding 5 (High)** - `Wait for the current operation to finish` renders
  beneath each of four selection actions and again under row actions; five
  copies in one viewport, worse in Japanese at 800x600.

### What later reviews added

- `060` finding 2 - the grouping/sort selectors mark the active option with
  `" *"` and **disable** it, so the selected value renders greyed: the least
  salient element in the selector.
- `066` observation 1 - the Select button's disabled reason reads
  *"No projects match this view."* while the body reads *"Welcome to knotra -
  Add your first project folder."* Both are individually correct and together
  contradictory: the truth is that no projects are registered, not that a filter
  matched nothing.
- `066` observations 2-3 - the toolbar's right column (`Select` plus its reason)
  is marooned ~1030px out, disconnected from the search field; the dashboard
  empty state sits in a large vertical void.
- `060` finding 4 - the ready-state summary always prints all four counts
  including zeros ("... 0 could not be checked" on a fully successful run).

### What RFC-034 already provides

`knotra-ui` now owns the semantic surface: `primary`/`secondary`/`ghost`/`danger`
with `_maybe` forms, a `style` module of `(&Tokens, Status) -> Style` wrappers,
`current_or`, `icon_button_maybe`, `overlay::surface`/`raised_card`, `Tokens`,
and lucide icons. `KnotraTheme` carries tokens and D7 colour roles, with a
contrast test asserting WCAG AA in both themes. The ambient iced theme is
applied at the application root.

### What is still missing from `knotra-ui`

Verified against `snora-widgets 0.25.0` and `iced_widget 0.14.2`:

| Need | Upstream | Status |
|---|---|---|
| Filter chip | `snora::design::chip::filter(tokens, label, selected, on_toggle)` - solid accent when selected, documented >=6.7:1 | exists; **no `knotra-ui` wrapper yet** |
| Notice banner | `snora::design::notice::Notice` builder (tone/title/body/action/dismiss) | exists; **no wrapper yet** |
| Progress row | `snora::design::progress::{row, card}` | exists; **no wrapper yet** |
| Select menu | `iced_widget::pick_list::PickList` | exists in iced; **snora provides no token styling** - `knotra-ui` must write one |
| Checkbox | `iced::widget::checkbox` | exists; needs token styling |

R2 forbids view code reaching into `snora::design`, so each of these needs a
`knotra-ui` wrapper before the dashboard can use it - the same pattern RFC-034
established for buttons.

## Motivation

**This is the screen users look at most.** Every other surface is entered from
it. Its scan geometry and control legibility set the perceived quality of the
whole application.

**Three of the audit's five High findings live here**, and the two most
concrete user-facing defects - the greyed-selected selectors and the contradictory
empty-state wording - are both visible in a single current screenshot.

**The foundation is ready and idle.** RFC-034 built tokens, semantic controls,
and an overlay host, but migrated only the shell and one dialog. The dashboard
is where that investment either pays off or does not.

## Requirements

### Functional - control semantics (D4)

R1. Status filter chips render through `chip::filter` via a `knotra-ui` wrapper.
Selected chips are filled; unselected are not; both remain pressable so a filter
can be toggled off.

R2. Grouping and sorting render as **select menus**, not segmented buttons.
Each shows its current value and, when opened, the complete option set. The
`" *"` suffix is removed.

R3. No control communicates selected state by being disabled. Specifically, the
`choice_button` helper and its `on_press_maybe((!active)...)` pattern are
removed from `view/dashboard.rs`.

R4. Section headers are neutral with a chevron indicating expanded/collapsed.
They are not full-width filled buttons. Needs-help remains non-collapsible and
its header carries no chevron.

R5. Row selection uses a real checkbox control, not `[ ]` / `[x]` text glyphs.

R6. Row primary actions use `secondary` (needs-help rows, which carry one safe
action) or `ghost`. Project names open the detail panel via a link or disclosure
treatment, not a filled button.

R7. The load-error notice renders through the `Notice` primitive via a
`knotra-ui` wrapper, preserving RFC-032 R7 exactly: generic first-level copy,
Retry, and raw adapter text only behind Show details.

### Functional - geometry and responsiveness (D5)

R8. Three width modes, with composition changes rather than wrapping:

| Mode | Range | Behaviour |
|---|---|---|
| Compact | 800-999px | Two-line rows: identity and row action on line one, status/reason on line two. Toolbar collapses low-frequency controls. Selection actions become a 2x2 grid or an action menu. |
| Standard | 1000-1279px | Bounded three-track rows. Full toolbar. |
| Wide | >=1280px | Content centred at ~1180-1240px; row tracks do not grow indefinitely. |

R9. Project rows use bounded tracks, not `FillPortion`. Identity, status, and
action columns have stable widths so the eye scans consistent columns down the
list.

R10. The toolbar is one coherent region. `Select` and its reason are no longer
separated from the search field by a fill spacer. Select menus replacing
segmented buttons must reduce toolbar height at 800px.

R11. The dashboard empty state renders near the content origin, not centred in
the full vertical space.

### Functional - state communication (D6)

R12. Operation ownership is represented **once**, in the activity/status region.

R13. When an operation owns the interlock, the selection action group shows
**one** shared disabled reason, not one per action. Row-level mutation actions
do not repeat it.

R14. Action-specific reasons - "no upstream", "exactly one project required" -
remain available and are surfaced in one contextual slot or on focus, not
permanently duplicated beneath every control.

R15. Disabled-reason wording must be true to the cause. When no projects are
registered, the Select control must not say a filter matched nothing. This
requires a distinct key from the no-filter-match case.

R16. The ready-state summary omits zero-valued segments rather than printing
"0 could not be checked" on a fully successful run.

### Non-functional

R17. **RFC-032 display semantics are unchanged.** `crates/knotra-app/src/state/dashboard.rs`
is not modified. Classification, the R6 fact-filter truth table, ordering,
`ordered_selectable_ids`, collapse behaviour, and selection reconciliation all
behave exactly as today.

R18. RFC-027 selection semantics and RFC-031 lease/activity semantics are
unchanged. No presentation action acquires the operation interlock.

R19. `grep -rn 'snora::design' crates/knotra-app/src/` returns **zero**. New
primitives are wrapped in `knotra-ui` first.

R20. All new or changed strings are localized in English and Japanese and pass
the first-level wording guards.

R21. Contrast: any new colour pairing introduced by chips, select menus,
checkboxes, or notices is covered by the existing contrast test, extended as
needed. WCAG AA in both themes.

R22. Keyboard: every control the dashboard renders is reachable and operable
without a pointer, including select menus, chips, checkboxes, and section
disclosures. Section disclosure keyboard activation uses the same message path
as pointer activation (D3/R20 of RFC-033).

**RFC-036 is implemented (`main: d20c7be`), so this requirement is now partly
satisfied. Measured against the tree on 2026-07-31
(`.git-exclude/reviewed/088-rfc-035-staleness-audit.md`):**

| Element | State | This RFC's obligation |
|---|---|---|
| Dashboard controls reachable by Tab | **done** — `dashboard::focus_order` covers section disclosures, row checkboxes, row names, and NeedsHelp actions | preserve |
| `Enter` opens the focused card's detail panel | **done** — the row-name target carries `DetailPanelMessage::Opened`, which `activate_focused` fires | **preserve, do not rebuild** |
| Visible focus ring on dashboard controls | **missing** — `with_focus_ring` is used only in `shell.rs` and `workspace_manager.rs` | **build** |
| `↑`/`↓`/`j`/`k` between cards | **missing** — no `Named::Up`/`Named::Down` bindings exist | **build** |

**The ring is the urgent half.** Tab now moves focus across the dashboard and
nothing renders it, so the primary screen currently shows no focus indication at
all — a worse state than before RFC-036, when Tab did nothing and no user could
form a false expectation. This shipped in 0.24.0 as a documented known issue and
this RFC is what closes it.

Card arrow-navigation is the dashboard-specific interaction RFC-036 explicitly
excluded from its own scope, and remains this RFC's to build.

R23. Evidence per RFC-033 D8 for every surface touched, including keyboard
focus order. The tooling and the focus model both exist now, so this evidence is
producible; capture per
`.git-exclude/reference/002-keyboard-evidence-runbook.md`.

R24. Existing gates pass: `fmt --all --check`,
`clippy --workspace --all-targets -- -D warnings`, and the three test suites.

## Goals

- Make control kind, consequence, and state legible at a glance on the busiest screen.
- Give the dashboard stable scan columns and specified width behaviour.
- Say each disabled reason once, in the right place, with true wording.
- Consume the RFC-034 foundation rather than extending ad hoc styling.

## Non-goals

- Any change to `state/dashboard.rs` or the display pipeline.
- Migrating Smart Pull, Freezer, context switch, changelog, or conflict overlays
  (RFC-037), or the remaining ad hoc overlay layers.
- Migrating Settings or History bodies (RFC-038).
- Per-project VCS history (RFC-039).
- Deleting `guided_button`/`guided_field` outright - this RFC migrates the
  dashboard's call sites; the helpers die when their last caller anywhere does.
- The project detail panel's internal layout.
- Animation, motion, or theme customization.

## External Design

### Toolbar

```text
standard / wide
┌──────────────────────────────────────────────────────────────────────┐
│ (Needs help) (Unsaved work) (Updates available) (Local commits) …     │  chips
│ Group [ Needs help  ▾ ]   Sort [ Needs help first ▾ ]   [ Search… ]  [Select] │
└──────────────────────────────────────────────────────────────────────┘

compact (800-999)
┌────────────────────────────────────────────┐
│ (Needs help) (Unsaved work) (Updates…)  ⋯  │  chips + overflow
│ [ Search…                    ]  ▾  [Select]│  selectors behind ▾
└────────────────────────────────────────────┘
```

Selected chips are filled. Selectors show their current value and open to the
full option set.

### Rows

```text
standard (>=1000px) — bounded tracks
  api            Git    Changes need your choice          [ Resolve ]
  worker         feature/jobs                  4 changed
  docs           main

compact (800-999) — two lines
  api                                          [ Resolve ]
  Git · Changes need your choice
```

Tier density from RFC-032 R10 is unchanged: needs-help carries identity,
problem, and one action; in-progress carries work area and one count; all-set
carries identity and work area only.

### Selection and busy state

```text
before (today)                        after
  [Check for updates]                   [Check for updates] [Get latest safely]
  Wait for the current operation…       [Save release point] [Change work area]
  [Get latest safely]                   Waiting for the current operation to finish.
  Wait for the current operation…
  [Save release point]                  ← one reason for the group,
  Wait for the current operation…          ownership shown once in the
  [Change work area]                       activity strip
  Wait for the current operation…
```

### Empty states

- No projects registered: guided add-project state near the content origin. The
  Select control's reason says *no projects are registered*, not that a filter
  matched nothing.
- Filters match nothing: no-match state with a working Clear filters action.
- Load error with workspace: `Notice` with Retry and Show details, per RFC-032 R7.

## Internal Design

### `knotra-ui` additions

Wrappers, added before any view consumes them (R19):

- `widget::chip::filter(tokens, label, selected, is_focused, on_toggle)` ->
  `Element`. **Corrected 2026-08-01** (`094` Finding 1): the original signature
  omitted `is_focused` and described this as a pass-through to
  `snora::design::chip::filter`. It cannot be one - that function returns an
  `Element`, not a `Style`, and snora's `chip_style_selected` /
  `chip_style_unselected` are private, so there is no seam to compose a ring
  onto. A chip with no `is_focused` cannot satisfy **R22** (chips reachable and
  operable without a pointer) together with RFC-033 **D7** (the focused control
  renders a visible ring). Build it from `KnotraTheme` like `select` and
  `checkbox`. A chip is a button, so its style type *is* `button::Style` and
  `with_focus_ring` applies directly.
- `widget::notice::…` - a thin builder pass-through preserving `Notice`'s
  tone/title/body/action/dismiss shape
- `widget::progress::{row, card}` - for the activity region
- `widget::select::pick_list(...)` plus a token-derived style function.
  **snora provides no select styling**, so this is written from
  `KnotraTheme`'s D7 roles: surface, border, text, accent for the open/selected
  row, and the standard 2px focus ring.
- `widget::checkbox(...)` with token styling and a 44px-compliant target.

`current_or` is **not** used by this RFC. It remains nav-specific; chips and
select menus carry selected state natively.

### View decomposition

`view/dashboard.rs` is 495 ELOC across 17 functions and will grow. Split into
`view/dashboard/` with `mod.rs`, `toolbar.rs`, `section.rs`, `row.rs`, and
`empty.rs`, matching the RFC-034 precedent for `widget/`.

`view_project_card`, `choice_button`, and `filter_button` are removed once their
replacements land.

### Responsive strategy

Iced has no media queries; width must be observed. **Use
`iced::widget::responsive`.** Derive a mode enum (`Compact` / `Standard` /
`Wide`) from the `Size` its closure receives, and select composition from it. The
mode is **presentation-derived, not persisted state** - it must not enter
`AppState` or `AppConfig`, and must not trigger a message on resize.

**This was Open Question 2 and is now decided** (2026-08-01), because the
mechanism chosen here is inherited by RFC-037 and RFC-038, and a cross-RFC
pattern left to per-stage judgement drifts.

Three reasons, in order of weight:

1. **Window width is the wrong input.** `responsive`'s closure receives the
   maximum space available *to that widget* (`iced_widget-0.14.2/src/responsive.rs`).
   The shell has chrome, so window width and dashboard content width are not the
   same number, and breakpoints keyed on the former would silently mis-fire
   whenever chrome changes.
2. **The alternative does not exist.** An earlier draft of this section offered
   "the existing window-size subscription" as a choice; there is no such
   subscription — `main.rs` sets `window::Settings` at startup and nothing
   observes resize. Choosing it would have meant building a subscription, a
   `Message` variant, and an `AppState` field, then keeping them in sync — which
   also contradicts this section's own rule that the mode is not persisted state.
3. **It keeps composition in `view`**, where R8's "composition changes rather
   than wrapping" belongs, instead of routing layout decisions through `update`.

**Constraint that comes with it:** `responsive` re-invokes its closure during
layout, so the closure must stay cheap — build elements in it, do not compute or
allocate anything derivable outside it.

### Disabled-reason ownership

`guided_button` currently renders its reason beneath the button whenever
`on_press` is `None`. For the selection group, callers switch to the plain
semantic variants (`primary_maybe` etc.) and the group renders one reason
element beneath the whole action row. Action-specific reasons attach to their
control's accessible name and surface in a single contextual slot.

`guided_button` itself is not modified - RFC-034 R7 keeps it stable for
unmigrated callers.

## Security Considerations

Presentation only. No command construction, no lease acquisition, no VCS task is
added or changed; RFC-032 R22 and RFC-031's interlock semantics carry forward
unchanged. Raw adapter error text stays behind Show details (RFC-032 R7); the
`Notice` primitive must not surface it at first level. Project and group names
are rendered as text and never interpolated into a command.

## Test Plan

### Preserved-behaviour tests (must pass unmodified)

The RFC-032 and RFC-027 suites are the regression gate. **If any existing test
in `crates/knotra-app/src/tests.rs` requires editing, the migration changed
behaviour it was not supposed to change** - stop and report rather than adjust
the test.

### New contract tests

- Filter chips dispatch `FilterMessage::StatusFilterToggled` and reflect
  selected state.
- Select menus dispatch `DashboardMessage::GroupingChanged` / `SortChanged` for
  every option, and the current value is displayed.
- No dashboard control is disabled solely to indicate selection.
- Section disclosure activates identically by pointer and keyboard; needs-help
  is non-collapsible.
- Row checkboxes toggle the same `ProjectId` set as before, in all three tiers.
- When the interlock is held, exactly one disabled reason renders for the
  selection action group.
- With zero projects registered, the Select reason is the no-projects string,
  not the no-filter-match string.
- The ready summary omits zero-valued segments.

### Visual and layout

- Compact/standard/wide composition at 800, 1100, 1500px, English and Japanese.
- No overlap, clipping, or unstable row heights at 800x600 in Japanese.

### Contrast and i18n

- Extend the contrast test to any new pairing (chip selected/unselected, select
  menu open row, checkbox checked, notice tones).
- Both catalogs carry every new key; wording guards pass.

### Evidence (D8)

Light/Dark x English/Japanese x 800x600/standard/wide for: toolbar, all three
row tiers, an expanded and a collapsed section, selection mode with and without
a held interlock, the no-projects empty state, the no-match empty state, and the
load-error notice. Plus keyboard focus order for the toolbar and one row, and
card-to-card arrow-key navigation with Enter-to-open.

**RFC-036 is implemented (`main: d20c7be`), so this evidence is now
producible.** The keyboard-evidence tooling spike
(`.git-exclude/tasks/developer/004-keyboard-evidence-tooling-spike.md`)
established the capture method, and RFC-036 built the Tab traversal, focus
trap/return, and visible ring the method had nothing to capture at the time.

Two consequences for this RFC's evidence, per R22's table:

- **Dashboard focus-order captures will show no ring until this RFC draws one.**
  That absence is the defect being fixed, so capture it before and after rather
  than treating a ringless "before" as a broken capture.
- **Enter-to-open already works** via RFC-036's generic activation. Evidence
  should demonstrate it still works after the migration - a regression check,
  not a new-feature capture.

### Commands

```sh
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
env TMPDIR="$PWD/.git-exclude/tmp" \
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
  GIT_EDITOR=true VISUAL=true EDITOR=true \
  cargo +1.91 test -p knotra-vcs
git diff --check
grep -rn 'snora::design' crates/knotra-app/src/    # must return nothing
```

## Acceptance Criteria

- [ ] Filter chips use `chip::filter` through a `knotra-ui` wrapper; selected chips are filled and remain toggleable.
- [ ] Grouping and sorting are select menus showing their current value; the `" *"` suffix is gone.
- [ ] No dashboard control indicates selection by being disabled; `choice_button` is removed.
- [ ] Section headers are neutral with a chevron; needs-help is non-collapsible.
- [ ] Row selection uses a real checkbox.
- [ ] The load-error notice uses the `Notice` primitive and keeps raw text behind Show details.
- [ ] Compact, standard, and wide modes behave per R8, with composition changes at compact.
- [ ] Rows use bounded tracks; no `FillPortion` row geometry remains.
- [ ] The toolbar is one region; `Select` is no longer marooned.
- [ ] The empty state renders near the content origin.
- [ ] Operation ownership renders once; the selection group shows one shared reason.
- [ ] The Select reason is true to its cause when no projects are registered.
- [ ] The ready summary omits zero-valued segments.
- [ ] `state/dashboard.rs` is unmodified and all RFC-032/027 tests pass **unedited**.
- [ ] `grep -rn 'snora::design' crates/knotra-app/src/` returns zero.
- [ ] New strings localized in both catalogs; wording guards pass.
- [ ] Contrast test extended to new pairings and passing in both themes.
- [ ] Every dashboard control is keyboard-operable.
- [ ] D8 evidence supplied, including keyboard focus order.
- [ ] All gates pass with observed output.

## Developer Handoff

Five stages, each leaving a green tree. Stage 1 is the enabler; do not start
Stage 2 before it lands.

### Stage 1 - `knotra-ui` primitives

Add wrappers for **chip, select, and checkbox**, plus the token-derived select
style function. Extend the contrast test to cover the new pairings **in this
stage**, before any view uses them - the same discipline that worked in RFC-034
Stage 2.

Nothing in `knotra-app` changes here.

**`notice` and `progress` are deliberately not in this stage** (corrected
2026-08-01). Their only consumer is Stage 5's activity region, and Open Question
3 requires deciding whether `progress::row` is needed *at all* before adding it -
"do not add the primitive speculatively." Building them here would either
pre-empt that decision or leave dead code standing through three stages. They are
added in Stage 5, under the same rule: wrapper and contrast coverage land before
the view consuming them.

The split matters for scoping this stage. Of the five primitives:

| Primitive | snora provides it? | Work |
|---|---|---|
| `chip::filter` | function exists, but **not composable** | built from `KnotraTheme` — corrected 2026-08-01, see above |
| `notice` | **yes** — `design::notice::Notice` | thin pass-through (Stage 5) |
| `progress::{row, card}` | **yes** — both | thin pass-through (Stage 5) |
| `select::pick_list` | **no** | written from `KnotraTheme` roles |
| `checkbox` | **no** | written from `KnotraTheme` roles |

So this stage is one pass-through and two primitives built from scratch, and
essentially all of its risk and all of its contrast-test burden sit in the
latter two.

### Stage 2 - toolbar

Split `view/dashboard.rs` into `view/dashboard/`. Replace `filter_button` with
the chip wrapper and `choice_button` with select menus. Rebuild the toolbar as
one region and remove the fill spacer marooning `Select`.

Expect toolbar height at 800px to drop; that is the point.

### Stage 3 - sections and rows

Neutral section headers with chevrons. Bounded row tracks replacing
`FillPortion`. Real checkboxes. Row actions on semantic variants.

`ordered_selectable_ids` consumption is unchanged - read the display result
exactly as today.

### Stage 4 - responsive modes

Derive the width mode in the view layer and implement compact two-line rows,
toolbar collapse, and the wide centred column. Do not put the mode in `AppState`.

### Stage 5 - state communication

Consolidate busy text to one group-level reason, move ownership to the activity
region, fix the Select reason wording (new i18n key), and omit zero-valued
summary segments.

Also adds the `notice` and `progress` wrappers deferred from Stage 1, since this
is where they are first consumed - and resolves Open Question 3 (whether
`progress::row` is needed at all) *before* adding it, per that question's own
"do not add the primitive speculatively." Contrast coverage lands with them, same
rule as Stage 1.

### Guardrails

1. **Never modify `crates/knotra-vcs`.**
2. **Never modify `crates/knotra-app/src/state/dashboard.rs`.** If a rendering
   need seems to require it, that is a signal the change belongs elsewhere -
   stop and escalate.
3. **Existing tests must pass unedited.** Editing one to make it pass is
   evidence of an unintended behaviour change.
4. **Do not use `current_or`** for chips or select menus. It is nav-specific;
   both of these carry selected state natively. This supersedes review `068` H2.
5. **Do not modify `guided_button`/`guided_field` signatures.** Migrate call
   sites; leave the helpers alone.
6. **Do not migrate outside the dashboard, selection bar, and activity strip.**
7. **`grep -rn 'snora::design' crates/knotra-app/src/` must return zero** at
   every stage boundary, not only at the end.
8. **Every new string in both catalogs**, same commit.

### Leave alone

- `state/dashboard.rs`, `state.rs`'s selection logic, and `app.rs`'s handlers.
- The `ActiveModal` overlays and the remaining ad hoc layers (RFC-037).
- Settings and History bodies (RFC-038).
- The detail panel's internals.
- `current_or` and the shell.

## Open Questions

1. **Select menu vs. segmented control for Group/Sort.** This RFC follows D4,
   which assigns select menus to "option sets whose width varies by locale" and
   names grouping and sorting explicitly. The counter-argument is discoverability:
   segmented controls show all options at rest. If Stage 2 finds the select menu
   materially worse to use, report it with captures rather than silently
   reverting - D4 would then need amending, which is an architect decision.

2. ~~**How is the width mode observed?**~~ **Resolved 2026-08-01 — see
   Internal Design / Responsive strategy.** `iced::widget::responsive`. Settled in
   the RFC rather than delegated, because RFC-037 and RFC-038 inherit the
   mechanism, and because the question as originally posed offered a false choice:
   there is no existing window-size subscription in the codebase.

3. **Does the activity strip need `progress::row`?** R12 puts ownership there,
   but the strip may already communicate it adequately. Decide during Stage 5 and
   record it; do not add the primitive speculatively.

## Deferred Follow-ups

- RFC-037: mutating workflow overlays and the remaining ad hoc layers.
- RFC-038: settings form grid and history record-list pattern.
- RFC-039: per-project VCS history.
- Deletion of `guided_button`/`guided_field` when their last callers migrate.
- Animation, icon tuning, and command-palette presentation.
