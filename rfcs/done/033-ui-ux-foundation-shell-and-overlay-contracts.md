# RFC-033 - UI/UX Foundation, Shell, and Overlay Contracts (Umbrella)

| Field | Value |
|---|---|
| Status | Accepted (main: 71b4796) |
| Priority | High - the GUI is not production-ready as an interaction system, and every downstream screen RFC needs these contracts to exist first |
| Effort | Medium (this RFC decides; child RFCs implement) |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-ui/src/theme.rs`, `crates/knotra-ui/src/widget.rs`, `crates/knotra-ui/Cargo.toml`, `crates/knotra-app/src/view.rs`, `crates/knotra-app/src/view/workspace_tabs.rs`, `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/view/selection_bar.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-app/src/view/settings.rs`, `crates/knotra-app/src/view/workspace_manager.rs`, `crates/knotra-app/src/main.rs`, `rfcs/done/022-snora-0.25.0-migration.md`, `rfcs/done/032-dashboard-grouping-sorting-and-tier-density.md` |
| Related audit evidence | `.git-exclude/reviewed/062-current-gui-ui-ux-audit.md`, `.git-exclude/evidence/ui-ux-review/`, `.git-exclude/evidence/rfc-032/` |

## Summary

This is an **umbrella RFC**. It decides the shared UI/UX contracts that every
subsequent screen-level RFC must build on, and it implements none of them.

It settles seven things: where the design foundation comes from, what the
application shell is, how overlays are rendered, what the semantic control
vocabulary is, how the UI responds to width, how state is communicated
accessibly, and what evidence is required before any of it is accepted.

The central decision is **D1**: adopt `snora`'s `design` feature and icon set as
knotra's foundation instead of growing a parallel one inside `knotra-ui`. That
reverses DEC-004, which RFC-022 explicitly left open for exactly this situation.

Deliberately excluded: per-screen mockups, colour palettes chosen by eye,
animation, and any change to domain semantics settled by RFC-023 through
RFC-032.

## Background

`.git-exclude/reviewed/062-current-gui-ui-ux-audit.md` audited the GUI at
`52f9f01` plus the RFC-032 working tree and returned **Needs changes**, with
five High findings:

1. workspace dialogs do not establish a modal layer (content collision);
2. no stable information architecture or persistent navigation model;
3. the component system does not communicate action type or state;
4. dashboard geometry has no bounded scan behaviour at either extreme;
5. busy/disabled guidance is repeated per control until it overwhelms the task.

The audit's structural diagnosis is that items 1, 2, and 4 are composition and
information-architecture problems while items 3 and 5 are component-system
problems, and that **colour changes alone will not solve any of them**.

The audit is explicit that the correct next artifact is one thin umbrella RFC
followed by a small number of coherent implementation RFCs - not one
all-encompassing redesign and not one RFC per screen.

### What already exists

- `knotra-ui` provides `StatusColor` (six VCS-status roles, WCAG AA verified in
  RFC-021 Phase 6), an i18n catalog, a handful of layout constants, and two
  helpers: `guided_button` and `guided_field`
  (`crates/knotra-ui/src/widget.rs:13-47`, `:66-95`).
- `KnotraTheme` wraps `iced::Theme::Light`/`Dark` and adds only status colours
  (`crates/knotra-ui/src/theme.rs:56-83`). It defines no surface or control roles.
- `knotra-app` consumes `snora` as a **layout engine only** - `AppLayout`,
  `Dialog`, `Sheet`, `render`, `app_tab_bar` - per DEC-004.
- `knotra-ui` does **not** depend on `snora` today
  (`crates/knotra-ui/Cargo.toml`).

### What snora 0.25 already ships behind `design`

Verified against `snora-0.25.0`:

| Primitive | Path | Audit need it answers |
|---|---|---|
| Button variants | `snora::design::button::{primary, secondary, ghost, danger}` plus `*_maybe` | Finding 3 (no semantic variants) |
| Tokens | `snora::design::{Tokens, Palette, Spacing, Typography, Radius, Size, Density, Emphasis, Tone, TextRole, FocusTokens}` | Findings 3, 4, and the token vocabulary |
| Notice banner | `snora::design::notice::Notice` (tone, title, body, action, dismiss) | Dashboard load-error notice (RFC-032 R7) |
| Filter chips | `snora::design::chip::{filter, removable}` | Dashboard status filter chips |
| Cards | `snora::design::card::{raised, selected, surface}` | Overlay surface, sticky bars |
| Progress | `snora::design::progress::{row, card}` | Activity strip, bulk modal progress |
| Icons | `snora::lucide` + `snora::widget::icon::{icon_element, icon_element_sized}` | Finding 3 and the Low typography/iconography finding |

The `*_maybe` button variants take `Option<Message>`, which maps directly onto
knotra's existing disabled-with-reason pattern.

### Why DEC-004 is now due for reversal

RFC-022 recorded three conditions for revisiting the deferral
(`rfcs/done/022-snora-0.25.0-migration.md:78-92`). Two are now met:

- *"knotra wants a primitive snora offers that knotra-ui lacks (e.g. the
  `notice` banner or `progress` card) and building it locally would clearly be
  more work than adopting snora's."* - The audit requires notice banners, filter
  chips, four button variants, focus tokens, and an icon set. `knotra-ui` has
  none of them.
- *"knotra-ui's styling accumulates enough maintenance burden that delegating to
  snora's tokens would reduce total complexity."* - The audit found font sizes
  chosen locally from 10px to 20px, ad hoc spacing, and text glyphs standing in
  for icons across every screen.

RFC-022's own framing anticipated this: the migration "would be a deliberate,
scoped piece of work - deleting knotra-ui styling in favour of `snora::design` -
not a dependency bump. This RFC explicitly leaves that door open."

## Motivation

**User trust.** A dialog that paints over the content beneath it is not a
cosmetic problem; it makes layer ownership, click targets, and reading order
genuinely ambiguous. A user cannot tell what is live.

**Product readiness.** The reset's release gate requires that every visible
control works, is disabled with a clear reason, or is hidden. RFC-023 through
RFC-032 made that true *functionally*. The audit shows it is not yet true
*perceptually*: controls that differ in kind and consequence are rendered
identically, so the interface cannot communicate which action is safe, which is
primary, and which is unavailable.

**Cost of deferring.** Every screen RFC written before these contracts exist
will either invent its own patterns or inherit the current ad hoc ones, and then
need migrating. That is the rework this umbrella exists to prevent. It is also
why this RFC precedes the last outstanding drafting-track item (per-project VCS
history), which would otherwise add a new surface built on undefined contracts.

## Decisions

Each decision is numbered `D`, states the chosen option, and records what it
supersedes. Child RFCs implement these; they do not relitigate them.

### D1. Adopt `snora::design` and `snora::lucide` as the foundation

**Decision.** Enable snora's `design` and `lucide-icons` features. Express
knotra's control, surface, typography, spacing, radius, elevation, and focus
roles through snora's tokens and primitives. Delete the ad hoc equivalents in
`knotra-ui` as each is replaced.

**Retained locally.** `StatusColor` stays in `knotra-ui` and keeps its
RFC-021-verified WCAG AA values. Repository status is knotra *domain* semantics,
not a generic control role, and its contrast values are an accepted
non-functional requirement (N-7). Status colour continues to accompany text or
an icon, never carrying meaning alone.

**Crate graph.** `knotra-ui` takes the `snora` dependency and re-exports the
semantic helpers. `knotra-app` view code keeps importing from
`knotra_ui::widget` and must not import `snora::design` directly. This preserves
the DEC-001 layering intent (the app does not reach around its UI foundation)
and gives exactly one place to migrate.

**Supersedes.** DEC-004. RFC-022 remains the historical record of why the
deferral was correct at v0.23.0.

**Alternative rejected.** Build primary/secondary/ghost/danger variants, a
focus-ring system, a token scale, a notice banner, filter chips, and an icon set
inside `knotra-ui`. This is strictly more work than adopting, produces a second
design system in a project that already depends on snora, and is the outcome
RFC-022 called "complicated and messy."

**Risk to measure, not assume.** Build time and binary size increase, and
snora's palette may not preserve the contrast guarantees of knotra's current
theme. Both are acceptance gates in the Test Plan, not assumptions.

### D2. One persistent application shell

**Decision.** A compact top application bar, 48-52px, is the only global
navigation surface.

- **Left:** workspace switcher showing the active workspace and its attention
  count. Its menu owns switch, create, rename, and delete.
- **Centre-left:** persistent Dashboard and History destinations with an
  unambiguous active state.
- **Right:** operation/refresh indicator, refresh command, command-palette
  command, Settings.
- **Below the shell:** a page header owning one title and screen-specific
  actions only.

No screen may invent its own back-navigation control, and no screen repeats the
workspace name that the switcher already supplies. Dashboard search, status
filters, grouping, sorting, and selection mode belong to the dashboard toolbar,
not the shell.

A permanent sidebar is rejected: the supported 800px minimum width is worth more
to VCS status rows than to navigation chrome.

**Supersedes.** The current top strip
(`crates/knotra-app/src/view/workspace_tabs.rs:74-104`) and the per-screen
headers in `history.rs:32-40`, `settings.rs:23-31`, and `dashboard.rs:44-65`.

### D3. One overlay host with a real modal contract

**Decision.** Every modal renders through a single root overlay host. The
current split - `ActiveModal` variants through `snora::Dialog`
(`crates/knotra-app/src/view.rs:94-118`) versus workspace-manager, add-project,
palette, and shortcuts as raw transparent `container(...).center()` layers on an
iced `stack` (`crates/knotra-app/src/view.rs:127-180`) - is removed.

Every overlay must have:

1. a full-window scrim that blocks pointer interaction with content beneath;
2. an opaque bounded surface using width tokens - small ~400px, standard ~520px,
   large ~680px - with 16-24px padding, a maximum height, and a scrollable body;
3. a stable header/body/footer structure: header owns title and close; footer
   owns Cancel plus **one** primary action, right-aligned in locale-aware order;
4. focus entry to the first meaningful control, a focus trap while open, and
   focus return to the opener on close;
5. deterministic stacking order when more than one layer can be open;
6. a phase-aware close policy shared by Escape, scrim click, and the header
   close control.

**Phase-aware close is a contract, not a per-screen choice.** The policies
established by RFC-029 and RFC-031 are authoritative and carry forward
unchanged: cancellable preparation phases release their operation lease on
close; non-cancellable mutating execution disables all three close affordances
and explains ownership once.

Sheets remain reserved for persistent inspection or conflict-resolution work
that benefits from seeing project context beneath. A sheet needs a clear edge,
independent scroll, and an explicit modal/non-modal input policy.

### D4. A semantic control vocabulary

**Decision.** Controls are chosen by role, not by convenience. The vocabulary is
fixed here; child RFCs map their screens onto it.

| Role | Use for | Constraint |
|---|---|---|
| Primary | The single main action of a surface | At most one per dialog or workflow surface |
| Secondary | Ordinary commands | Default choice for most buttons |
| Ghost | Low-emphasis and toolbar commands | No filled background |
| Danger | Destructive confirmation | Only behind an existing confirmation flow |
| Link / disclosure | Opening project details | Not a filled button |
| Icon button | Familiar global commands - refresh, settings, close, overflow | Fixed square target, tooltip and accessible name required |
| Checkbox | Row selection | Real checkbox semantics, not `[ ]` / `[x]` text glyphs |
| Toggle | Binary settings | e.g. filesystem monitoring |
| Segmented control | Short, stable option sets | Selected option is **filled**, never disabled-looking |
| Select menu | Option sets whose width varies by locale | Grouping and sorting selectors |
| Section header | Collapsible group headers | Neutral, with a chevron - never a full-width primary button |

**Explicitly corrected.** The RFC-032 grouping and sorting selectors mark the
active option with a `" *"` suffix and disable it, which renders the selected
value greyed out - the least salient element in the selector
(`crates/knotra-app/src/view/dashboard.rs:196-205`, visible in
`.git-exclude/evidence/rfc-032/dashboard-en-1100x720.png`). Under D4 the
selected option is filled and remains focusable. This is the resolution of
finding 2 of `.git-exclude/reviewed/060-...-implementation-review.md`.

### D5. Three responsive modes

**Decision.** Width behaviour is specified, not emergent.

| Mode | Range | Rules |
|---|---|---|
| Compact | 800-999px | Two-line project rows: identity and row action on line one, status/reason on line two. Toolbar overflow menus. Dialogs use `min(window - 32px, token width)`. Selection actions become a 2x2 grid or an action menu. Settings labels stack above controls. |
| Standard | 1000-1279px | Bounded three-track project rows. Full primary toolbar with low-frequency controls in menus. Dialog widths stay tokenized. |
| Wide | >=1280px | Content centred at ~1180-1240px. Row-track distances do not grow indefinitely. Optional detail sheet uses the remaining width. |

At compact width, composition **changes**; it does not merely wrap. Japanese
allocates controls by intrinsic text width and permits labels to wrap before
controls; menus are preferred over three or more inline text segments. There is
no separate Japanese component structure and no locale-specific hard-coded
width.

### D6. State is communicated once, at the owning level

**Decision.** Operation ownership is represented **once** in the persistent
activity/status region. A disabled reason caused by that ownership is shown
**once per action group**, not beneath every control.

Action-specific reasons - "no upstream", "exactly one project required" - are
preserved and surfaced in one contextual explanation slot or on focus, not
permanently duplicated.

This requires changing the `guided_button` contract, which today always renders
its reason directly beneath the button
(`crates/knotra-ui/src/widget.rs:66-95`), producing the repetition the audit
documents: `Wait for the current operation to finish` rendered five times in one
viewport (`crates/knotra-app/src/view/selection_bar.rs:36-112`,
`crates/knotra-app/src/view/dashboard.rs:427-438`).

**Meaning is preserved; repetition is removed.** No safety information may be
deleted, only relocated.

**Accessibility state contract.** Focus uses a consistent 2px high-contrast ring
with offset. Selected, disabled, busy, error, and keyboard-focused states must
be **separately identifiable**, never conveyed by colour alone. Interactive
targets retain 44px for primary/secondary and touch-critical controls; 36px is
allowed only for dense inline and icon controls.

### D7. Token vocabulary

**Decision.** Values come from tokens, never from per-screen tuning.

- **Spacing:** 4, 8, 12, 16, 24, 32px only, with named roles for inline gap,
  control gap, row padding, section gap, and page gutter.
- **Typography:** 20px page title, 16px section title, 14-15px body/control,
  12-13px metadata; regular and medium weight roles; no viewport-scaled type;
  zero letter spacing.
- **Shape:** 4-6px control radius, at most 8px for dialogs and repeated records.
  No decorative cards, and no cards inside cards.
- **Colour roles:** neutral background, raised surface, subtle border, primary
  text, muted text, focus, accent, destructive, warning, success, disabled - in
  both themes. Saturated accent is reserved for focus, active navigation, and
  the primary action.
- **Elevation:** none for page sections; one subtle level for menus and sticky
  bars; one stronger level plus scrim for modal and sheet surfaces.
- **Layout:** page max width, page gutter, dashboard row tracks, modal widths,
  toolbar height, and the D5 breakpoints are all tokens or reusable helpers.

### D8. Evidence matrix

**Decision.** No child RFC is accepted on English light-theme screenshots alone.
Each must supply captures across:

- **Theme:** Light and Dark;
- **Locale:** English and Japanese;
- **Width:** 800x600, standard, wide;
- **State:** representative empty, error, busy, disabled, and selected states for
  the surfaces it touches;
- **Keyboard:** evidence of focus order, focus trap and return for any overlay it
  introduces or migrates.

The audit could not accept palette parity because no dark-theme captures existed
and could not accept focus behaviour because no keyboard evidence existed. That
gap does not recur.

## Requirements

### Functional

R1. `snora` is enabled with `design` and `lucide-icons`; `knotra-ui` depends on
`snora` and re-exports the semantic helpers (D1).

R2. `knotra-app` view code imports control and token helpers only from
`knotra_ui`, never from `snora::design` directly (D1).

R3. `StatusColor` remains in `knotra-ui` with its RFC-021-verified WCAG AA
values and continues to appear alongside text or an icon (D1, N-7).

R4. One application shell provides workspace switching, top-level destinations,
global commands, and operation status; screens own only a page title and
contextual actions (D2).

R5. Workspace create, rename, and delete are commands in the workspace
switcher's menu, not peer navigation buttons (D2).

R6. Every overlay renders through one root host with scrim, opaque bounded
surface, header/body/footer, focus entry/trap/return, deterministic stacking,
and a phase-aware close policy (D3).

R7. The phase-aware close policies established by RFC-029 and RFC-031 are
preserved exactly: cancellable phases release their lease on every close route;
non-cancellable execution disables Escape, scrim click, and header close (D3).

R8. Every interactive control maps to a role in the D4 vocabulary.

R9. Selected state in segmented controls is filled and focusable, never
communicated by disabling the selected option or by a text suffix (D4).

R10. Row selection uses real checkbox semantics rather than text glyphs (D4).

R11. Compact, standard, and wide modes behave per D5, with composition changes
rather than wrapping at compact width.

R12. Operation ownership renders once in the activity/status region; an
ownership-derived disabled reason renders once per action group (D6).

R13. Action-specific disabled reasons remain available and are not deleted (D6).

R14. Focus, selected, disabled, busy, and error states are separately
identifiable without relying on colour alone (D6).

R15. Spacing, typography, shape, colour roles, elevation, and layout values come
from tokens (D7).

### Non-functional

R16. All user-facing strings introduced or touched by child RFCs are localized
in English and Japanese, and first-level wording passes the existing jargon
guards.

R17. No child RFC changes domain semantics settled by RFC-023 through RFC-032.
Display, layout, and control role may change; classification, ordering,
selection membership, lease behaviour, and VCS execution may not.

R18. Contrast is verified, not assumed: every colour role and every
`StatusColor` on its intended surface meets WCAG AA in both themes after
adopting snora's palette.

R19. Build time and release binary size are measured before and after D1, and
recorded in the child RFC that lands it.

R20. Each child RFC supplies the D8 evidence matrix for the surfaces it touches.

R21. The existing release gates continue to pass: `fmt --all --check`,
`clippy --workspace --all-targets -- -D warnings`, and the three test suites.

## Goals

- Give every downstream screen RFC a foundation it can build on without
  inventing patterns.
- Remove the modal-layer collision, which is a real usability defect.
- Make control kind, consequence, and state legible at a glance.
- Make width behaviour a specification rather than an accident.
- Reduce repeated safety text without reducing safety information.
- Keep the functional semantics of the Production Readiness Reset intact.

## Non-goals

- Per-screen mockups or a full visual redesign in this RFC.
- Animation, transitions, and motion design.
- A palette chosen by eye rather than by contrast verification.
- Any change to grouping, sorting, tiering, selection, retry, lease, or VCS
  semantics from RFC-023 through RFC-032.
- Replacing iced or snora.
- Screen-reader/ARIA support beyond what iced 0.14 exposes; keyboard
  completeness, visible labels, and contrast remain the accessibility contract.
- Per-workspace display preferences, theming customization, or user-defined
  tokens.

## External Design

### Shell

```text
┌────────────────────────────────────────────────────────────────────┐
│ [ work (2) v ]   Dashboard  History          ◷ idle  ⟳  ⌘K  ⚙      │  48-52px
├────────────────────────────────────────────────────────────────────┤
│ Dashboard                                          [ Check now ]    │  page header
├────────────────────────────────────────────────────────────────────┤
│ [chips]  Group: [Needs help v]  Sort: [Needs help first v]  [find] │  screen toolbar
│                                                                     │
│ Needs help (2)                                                      │
│ …                                                                   │
└────────────────────────────────────────────────────────────────────┘
```

The workspace switcher menu owns switch, create, rename, and delete. The page
header does not repeat the workspace name.

### Overlay

```text
┌─ scrim (blocks pointer input) ─────────────────────────────────────┐
│              ┌─ surface (opaque, token width) ─┐                   │
│              │ Title                        ✕  │  header           │
│              ├────────────────────────────────┤                    │
│              │ body (scrolls, max height)     │                    │
│              ├────────────────────────────────┤                    │
│              │              [Cancel] [Primary]│  footer            │
│              └────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────────┘
```

While a non-cancellable operation owns the interlock, `✕`, Escape, and scrim
click are all inert, and the surface explains ownership once.

### Compact row composition

```text
standard (>=1000px)
  api          Git    Changes need your choice        [Resolve]

compact (800-999px)
  api                                                 [Resolve]
  Git · Changes need your choice
```

## Internal Design

### Crate graph

```text
knotra-app  ──uses──►  knotra-ui  ──uses──►  snora (design, lucide-icons)
     │                                            ▲
     └───────────── layout engine only ───────────┘
                    (AppLayout, Sheet, render)
```

`knotra-ui` gains the `snora` dependency and owns the semantic surface.
`knotra-app` continues to use snora's layout engine directly for `AppLayout`
composition, but takes **no** direct dependency on `snora::design`.

### `knotra-ui` module shape

- `theme.rs` - keeps `StatusColor`; `KnotraTheme` gains the snora `Tokens`
  handle and exposes colour roles. Existing `StatusColor` values are retained
  verbatim until R18 re-verification says otherwise.
- `widget.rs` - splits. It is already the home of `guided_button`/`guided_field`
  and, per the project's 300/500-ELOC guideline, should become `widget/` with
  `button.rs`, `field.rs`, `overlay.rs`, `layout.rs`, and `icon.rs`.
- Layout constants (`CARD_RADIUS`, `CARD_GAP`, `CARD_PADDING`, `SIDEBAR_WIDTH`,
  `CARD_MIN_WIDTH`, `FONT_BODY`, `FONT_SMALL`, `BUTTON_HEIGHT`,
  `SMALL_BUTTON_HEIGHT`) are replaced by tokens and removed as each caller
  migrates. `SIDEBAR_WIDTH` and `CARD_MIN_WIDTH` are already dead under D2/D5
  and go first.

### `guided_button` contract change

`guided_button(label, on_press, reason)` currently renders the reason beneath
the button whenever `on_press` is `None`. Under D6 it takes an explicit
disclosure mode so a caller can opt into group-level reporting:

- inline reason - retained for isolated controls;
- group-owned - the button renders disabled with an accessible name carrying the
  reason, while the group renders the reason once.

The call sites are enumerated in the handoff. `guided_field` keeps its inline
error, which is correct for form validation.

### Migration sequence

| RFC | Scope | Depends on |
|---|---|---|
| 033 | This umbrella - decisions only | - |
| 034 | Foundation, shell, overlay host; migrate the workspace dialog and one page header as validation | 033 |
| 035 | Dashboard and selection: toolbar, section disclosure, row tracks, checkbox/action surface, activity ownership, D5 modes | 034 |
| 036 | Mutating workflow overlays: Smart Pull, Freezer, context switch, changelog, conflict resolution | 034 |
| 037 | Settings and History: form grid, record-list pattern, remaining hard-coded English | 034 |
| 038 | Per-project VCS history for Git and jj | 034, and the record-list pattern from 037 |
| - | Polish: palette presentation, detail panel, animation, icon tuning | after 035-037 |

RFC-034 carries the near-term defect correction: it is the RFC that removes the
workspace-dialog collision, and migrating that dialog is what proves the D3
contract before the other overlays follow.

**Note on the drafting track.** `ROADMAP.md` lists *Per-project VCS history for
Git and jj* as the last unstarted drafting-track item. It becomes **RFC-038**
under this sequence. The owner may pull it forward to immediately after RFC-034
to close the drafting track sooner, provided it consumes the record-list pattern
decided here rather than inventing one; RFC-037 would then apply that pattern to
the existing History screen.

## Security Considerations

This RFC changes presentation only. Specifically:

- No change to command construction. All VCS execution remains argument-vector
  based; no display string may be interpolated into a command.
- No change to lease acquisition. Presentation actions - navigation, grouping,
  sorting, filtering, collapse, theme - must never acquire the operation
  interlock or start a VCS task (RFC-032 R22 carries forward).
- Raw adapter error text stays behind an explicit details disclosure and out of
  first-level surfaces (RFC-032 R7 carries forward), which the new notice
  primitive must respect.
- Adopting `snora::design` and `lucide-icons` enlarges the dependency surface.
  Both are already-trusted upstreams from the same maintainer as the layout
  engine knotra already ships, but the child RFC that lands D1 must record the
  added transitive dependencies for supply-chain review.
- Focus trapping must not create an inescapable state: every overlay retains at
  least one keyboard route to close, except where a non-cancellable operation
  deliberately owns the surface and says so.

## Test Plan

This RFC ships no code, so it has no unit tests of its own. It fixes what child
RFCs must prove.

### Contract tests every child RFC inherits

- Every overlay it touches: scrim blocks input beneath, focus enters the first
  meaningful control, focus is trapped while open, focus returns to the opener,
  and Escape/scrim/close obey the phase-aware policy.
- Non-cancellable phases: Escape, scrim click, and header close are all inert,
  and the lease is still held.
- Cancellable phases: every close route releases the matching lease and a late
  completion is ignored - the RFC-031 invariants, retested through the new host.
- Presentation actions acquire no lease and start no task.
- Every control it renders maps to a D4 role.
- Group-level disabled reasons appear once; action-specific reasons remain
  reachable.

### Foundation tests (RFC-034)

- Contrast: every colour role and every `StatusColor` on its intended surface
  meets WCAG AA in both themes, asserted in code rather than by inspection.
- Token usage: no raw font size, spacing, or radius literal remains in migrated
  view code.
- Build cost: compile time and release binary size recorded before and after
  enabling `design` and `lucide-icons`.

### Evidence matrix (D8)

Light/Dark x English/Japanese x 800x600/standard/wide, plus representative
empty, error, busy, disabled, and selected states, plus keyboard focus evidence
for overlays. Pixel captures for each shared primitive.

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
```

## Acceptance Criteria

This RFC is accepted when the decisions are agreed, not when code lands.

- [ ] D1 is accepted or rejected explicitly, and DEC-004's reversal is recorded.
- [ ] D2's shell composition is agreed, including that workspace CRUD moves into
      the switcher menu.
- [ ] D3's overlay contract is agreed, including that the four ad hoc stack
      layers migrate to the single host.
- [ ] D4's control vocabulary is agreed, including the segmented-control
      selected-state correction.
- [ ] D5's three modes and their rules are agreed.
- [ ] D6's once-per-owner state contract is agreed, including the
      `guided_button` contract change.
- [ ] D7's token vocabulary is agreed.
- [ ] D8's evidence matrix is agreed as a gate for every child RFC.
- [ ] The child-RFC sequence and the RFC-038 renumbering of per-project VCS
      history are agreed.
- [ ] No decision here changes RFC-023 through RFC-032 domain semantics.

## Developer Handoff

This section is guidance for implementers of the child RFCs. It is not itself
implementable work - RFC-033 lands no code.

### H1. RFC-034 - foundation, shell, overlay host

**Order matters.** Do these in sequence; each step keeps the tree green.

**Step 1 - enable the feature and re-point the crate graph.**

- `Cargo.toml` (workspace): change
  `snora = "0.25"` to
  `snora = { version = "0.25", features = ["design", "lucide-icons"] }`.
- `crates/knotra-ui/Cargo.toml`: add `snora = { workspace = true }`.
- Record build time and `target/release/knotra` size before and after (R19).
- Gate: `cargo +1.91 clippy --workspace --all-targets -- -D warnings`.

**Step 2 - build the token surface in `knotra-ui` before touching any view.**

- Split `crates/knotra-ui/src/widget.rs` into `widget/` with `button.rs`,
  `field.rs`, `overlay.rs`, `layout.rs`, `icon.rs`. The file is already at the
  project's split guideline and will grow.
- Add a `Tokens` handle to `KnotraTheme` (`theme.rs:56-83`) and expose colour
  roles. Do **not** touch `StatusColor`'s values in this step - they are
  RFC-021-verified and changing them silently would regress N-7.
- Write the contrast assertion test **first**, covering both themes and every
  `StatusColor` on its intended surface. If snora's palette changes any
  effective background, this test is what tells you before the screenshots do.

**Step 3 - build the overlay host, then migrate exactly one dialog.**

- The defect is at `crates/knotra-app/src/view.rs:127-180`: workspace-manager,
  add-project, palette, and shortcuts are pushed as
  `container(...).center(Length::Fill)` onto an iced `stack`, with no scrim, no
  opaque surface, and no input blocking. `ActiveModal` variants at
  `view.rs:94-118` already go through `snora::Dialog` and are comparatively
  correct.
- Build the host so **both** groups render through it. Do not fix the four ad
  hoc layers by wrapping them in `snora::Dialog` one at a time; that reproduces
  the split this decision removes.
- Migrate **workspace-manager first**. It is the one with photographic evidence
  of the collision (`.git-exclude/evidence/ui-ux-review/workspace-create-en-1100x720.png`),
  so it is the clearest before/after proof.
- Preserve `on_close_modals` routing into `ShortcutMessage::Close`, and preserve
  `close_topmost_layer`'s ordering semantics
  (`crates/knotra-app/src/app.rs`, `fn close_topmost_layer`). That function
  already encodes the phase-aware policy correctly for Smart Pull retry
  preparation, Freezer validation, context switching, and tag push - **do not
  rewrite it**, only re-point what it hides.

**Step 4 - the shell.**

- `crates/knotra-app/src/view/workspace_tabs.rs:74-104` becomes the shell's
  workspace switcher; move create/rename/delete into its menu.
- Delete the per-screen back-navigation headers at
  `crates/knotra-app/src/view/history.rs:32-40` and
  `crates/knotra-app/src/view/settings.rs:23-31`, and the workspace-name
  repetition at `crates/knotra-app/src/view/dashboard.rs:44-65`.
- Screens keep only their page title and contextual actions.

**Tests to add:** scrim blocks input beneath; focus enters, traps, and returns;
Escape/scrim/close obey phase-aware policy; the RFC-031 lease-release
invariants still hold through the new host; contrast assertions.

**Leave alone:** `close_topmost_layer`'s branch ordering and its running-phase
predicates; `StatusColor` values; anything in `knotra-vcs`.

### H2. RFC-035 - dashboard and selection

- Consume `DashboardDisplay` exactly as RFC-032 defines it. Row density,
  ordering, `ordered_selectable_ids`, and selection reconciliation are settled -
  this RFC changes only how those results are *rendered*.
- Replace `filter_button` and `choice_button`
  (`crates/knotra-app/src/view/dashboard.rs:183-205`) with `chip::filter` and a
  real segmented control or select menu. Removing `choice_button` resolves the
  greyed-selected-option defect (D4) and its unused `_state` parameter.
- Replace the section-header full-width button
  (`dashboard.rs:336-377`) with a neutral header plus chevron. Keep
  `DashboardMessage::TierToggled` and keep Needs-help non-collapsible.
- Replace `FillPortion(4)`/`FillPortion(5)` row geometry
  (`dashboard.rs:450-461`) with bounded tracks and the D5 compact two-line form.
- Consolidate busy text: `crates/knotra-app/src/view/selection_bar.rs:36-112`
  passes the same reason to four `guided_button`s, and
  `dashboard.rs:427-438` repeats it per row. Move ownership to the activity
  region and render the shared reason once per group. Keep "no upstream" and
  "exactly one project required" reachable.
- **Do not** change `state/dashboard.rs`. If a rendering need seems to require
  changing classification, filtering, or ordering, that is a signal the change
  belongs in a different RFC.

### H3. RFC-036 - mutating workflow overlays

- Migrate Smart Pull, Freezer, context switch, changelog, and conflict
  resolution to the D3 primitives **without touching their state machines**.
- The safety-critical invariants to preserve, with their sources:
  - RFC-029: close and Escape are inert while `ContextPhase::Switching`;
  - RFC-031: cancellable preparation/validation releases its exact lease on
    every close route and ignores late completion; non-cancellable execution and
    tag push disable every close affordance;
  - RFC-030: the changelog request-id guard and the `Collecting`-phase field
    policy.
- Recommended proof: run the existing app suite before and after each screen's
  migration and require zero test changes. If a test needs editing, the
  migration changed behaviour it was not supposed to change.
- `conflict resolution` stays a sheet, not a dialog - it benefits from seeing
  project context (D3).

### H4. RFC-037 - settings and history

- Settings: bounded two-column form grid at standard/wide, stacked at compact.
  Language and theme become segmented controls or select menus; filesystem
  monitoring becomes a toggle; numeric values become validated numeric fields
  with units and persistent errors
  (`crates/knotra-app/src/view/settings.rs:41-87`, `:153-216`, `:223-230`).
- Localize the hard-coded English at `settings.rs:79-87` and `:177-208`, and the
  hard-coded back label at `history.rs:32-40`. Add them to both catalogs and
  keep the jargon guards green.
- History: bounded search/filter toolbar, fixed metadata hierarchy, explicit
  disclosure control, empty state near the content origin
  (`history.rs:66-73`, `:109-177`).
- Establish the **record-list pattern** here in reusable form - RFC-038 depends
  on it.
- Opportunity: `log_to_markdown` (`history.rs:308`) is a free function with no
  `AppState`, which is why copied History Markdown still emits raw reason codes
  while the visible path localizes them (disclosed in
  `.git-exclude/reviewed/055-...`). If this RFC threads a locale through, that
  long-standing gap closes with it.

### H5. Cross-cutting rules for every child RFC

- **Never** change `knotra-vcs`. If a UI RFC needs a VCS change, stop and raise
  it - that is a scope error.
- Every new or touched string goes into both catalogs in the same commit, and
  first-level wording passes the jargon guards.
- Run the full gate list from the Test Plan, including the hermetic VCS
  invocation, before requesting review.
- Supply the D8 evidence matrix for the surfaces touched. A child RFC without
  dark-theme and Japanese captures is not reviewable.
- Keep each commit to one coherent migration. The reset's precedent - draft,
  review, implement, review, marker - applies unchanged.
- A developer handoff is a prescription for *what* to change; it is not approval
  of the diff that results. Route implementations through the normal pre-commit
  review.

## Open Questions

1. **D1 is accepted.** See
   `.git-exclude/reviewed/063-rfc-033-acceptance-and-rfc-034-precondition-review.md`
   (Decision 1) for the full analysis. The dependency surface grows by exactly
   two packages (676 to 678: `snora-design`, `lucide-icons`); both added crates
   compile in about 33 seconds in release, a figure that already includes a
   release `iced` compile the app pays for regardless; and `snora-design`
   publicly exposes `contrast_ratio`, `relative_luminance`, and
   `composite_over`, so knotra's WCAG AA assertion test can consume snora's own
   contrast function rather than reimplementing luminance maths. D1 reverses
   DEC-004, so the owner retains override. If the owner later overrides this
   acceptance, D4, D6, and D7 must be implemented inside `knotra-ui` instead,
   and RFC-034's effort rises from Medium to Large. Everything else is
   unaffected.
2. **Where per-project VCS history lands.** RFC-038 after RFC-037 (reuses the
   record-list pattern) versus immediately after RFC-034 (closes the drafting
   track sooner). Recommended: after RFC-037, unless closing the track is worth
   more than pattern reuse.
3. **Whether the ROADMAP reset tracks are reconciled now.** Three verification
   items are provably passing on current evidence while two are genuinely open.
   Recommended: reconcile as a separate bookkeeping commit, not inside a child
   RFC. `ROADMAP.md` also needs a UI/UX track added and its drafting-track entry
   for per-project VCS history renumbered to RFC-038.

4. **How a decisions-only RFC terminates.** The project has no precedent for
   this. Every RFC in `rfcs/done/` carries `Implemented (main: <hash>)` because
   code landed; RFC-033 lands none, and its Acceptance Criteria are satisfied by
   agreement rather than by implementation. It cannot stay in `rfcs/proposed/`,
   because the lifecycle policy forbids starting dependent work while an RFC is
   proposed - which would block RFC-034 indefinitely.

   Recommended: on acceptance, move it to `rfcs/done/` with Status
   `Accepted (main: <hash>)`, where the hash is the acceptance commit itself,
   and use the same value in the `rfcs/README.md` Shipped column. This needs
   deciding before RFC-034 is drafted, because RFC-034's metadata table cites
   this document's path, and because acceptance is also what satisfies
   precondition 1 of
   `.git-exclude/tasks/developer/001-rfc-034-foundation-shell-and-overlay-host.md`.

   Note that the owner's acceptance is itself the design review for this RFC:
   it was authored by the reviewer, so an independent architect review of it
   would carry little value. Reviews resume normally at RFC-034.

## Deferred Follow-ups

- Animation and motion design.
- Command-palette presentation and detail-panel refinement.
- Screen-reader/ARIA support, which iced 0.14 does not expose.
- Automated pixel-diff regression testing; the D8 matrix is manual inspection.
- Locale-aware collation, still deferred from RFC-032.
- User-configurable theming or token overrides.
