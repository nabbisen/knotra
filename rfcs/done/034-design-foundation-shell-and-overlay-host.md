# RFC-034 - Design Foundation, Application Shell, and Overlay Host

| Field | Value |
|---|---|
| Status | Implemented (working tree; pending commit) |
| Priority | High - no overlay in the application has an opaque surface, and the shell decision gates every remaining UI/UX RFC |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `Cargo.toml`, `crates/knotra-ui/Cargo.toml`, `crates/knotra-ui/src/lib.rs`, `crates/knotra-ui/src/theme.rs`, `crates/knotra-ui/src/widget.rs`, `crates/knotra-app/src/view.rs`, `crates/knotra-app/src/view/workspace_manager.rs`, `crates/knotra-app/src/view/workspace_tabs.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-app/src/view/settings.rs`, `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/033-ui-ux-foundation-shell-and-overlay-contracts.md` (contracts), `rfcs/done/022-snora-0.25.0-migration.md` (DEC-004, reversed by D1), `rfcs/done/019-snora-layout-adoption.md`, `rfcs/done/021-plain-language-layer.md` (N-7 contrast), `rfcs/done/023-workspace-management-completion.md` (the dialogs migrated here) |
| Related audit evidence | `.git-exclude/reviewed/062-current-gui-ui-ux-audit.md`, `.git-exclude/reviewed/063-rfc-033-acceptance-and-rfc-034-precondition-review.md` |

## Summary

This is the first implementable child of RFC-033. It builds the foundation the
rest of the UI/UX track consumes, in four sequenced steps:

1. adopt `snora`'s `design` and `lucide-icons` features and re-point the crate
   graph so `knotra-ui` owns the semantic surface;
2. build the token surface in `knotra-ui`, contrast test first;
3. build one overlay host with a real modal contract, and migrate the
   workspace-manager dialogs through it as validation;
4. replace the top strip with the persistent application shell.

It deliberately migrates exactly one overlay family. The dashboard, the mutating
workflow modals, settings, and history stay on their current rendering until
RFC-035 through RFC-037.

**A correction to the audit's diagnosis, established while drafting this RFC:**
the overlay problem is worse and more uniform than reported. The audit found
that workspace dialogs paint over content. In fact **no overlay in knotra has an
opaque surface** - there is not a single `.style(` call across
`bulk_modals.rs`, `workspace_manager.rs`, `add_project_modal.rs`,
`command_palette.rs`, or `shortcuts_overlay.rs`, and no background styling
anywhere under `crates/knotra-app/src/view/`. The `ActiveModal` dialogs only
*appear* correct because `snora::render` paints a 40% dim backdrop beneath them.
Their surfaces are transparent too. Routing alone therefore cannot fix this; the
host must supply both the scrim and the surface.

## Background

### What RFC-033 decided

RFC-033 is accepted (`Accepted (main: 71b4796)`). Its decisions D1 through D8
are settled and are consumed, not relitigated, here. D1 was accepted on measured
evidence recorded in `.git-exclude/reviewed/063-...md`: adopting snora's design
layer grows the dependency graph by exactly two packages, costs roughly 33
seconds of release compile time, and *improves* knotra's ability to hold its
WCAG AA guarantee because `snora-design` exposes its own contrast functions.

### What the code does today

**Crate graph.** `knotra-ui` depends only on `iced`. `knotra-app` depends on
both `knotra-ui` and `snora`, and uses snora purely as a layout engine
(`AppLayout`, `Dialog`, `Sheet`, `render`, `app_tab_bar`) per DEC-004.

**Overlay rendering is split two ways** (`crates/knotra-app/src/view.rs`):

- `ActiveModal::{Pull, Tag, Switch, Changelog}` go through
  `AppLayout::dialog(Dialog::new(el))`, and `Resolve` through
  `AppLayout::sheet(...)`. `snora::render` composes a documented layer stack in
  which layer 4 is a 40%-dim `mouse_area` backdrop dispatching `on_close_modals`
  and layer 5 is the centred dialog. These overlays get input blocking and a
  scrim.
- `workspace_manager`, `add_project_modal`, `command_palette`, and
  `shortcuts_overlay` are wrapped in `container(...).center(Length::Fill)` and
  pushed onto an `iced::widget::stack` **above** `render(layout)`. They get no
  scrim, no dim, and no input blocking.

**Neither group has an opaque surface.** `modal_shell`
(`crates/knotra-app/src/view/bulk_modals.rs:41-66`) builds
`container(column![...]).max_width(580.0)` with no `.style(`. The
workspace-manager dialog builder
(`crates/knotra-app/src/view/workspace_manager.rs:103-118`) does the same with
`.width(Length::Fixed(460.0))`. An unstyled `iced` container has no background.

**The top strip mixes concerns.** `view/workspace_tabs.rs:74-104` owns workspace
tabs plus `+ New workspace`, `Rename`, `History`, and `Settings` as peer
controls. `history.rs:32-40` and `settings.rs:23-31` each invent their own back
navigation, and `dashboard.rs:44-65` repeats the workspace name that the tab
strip already shows.

**`knotra-ui` has no semantic layer.** `widget.rs` provides nine layout
constants, `guided_button`, `guided_field`, `guided_field_focused`, focus IDs,
and `focus_input`. `theme.rs` wraps `iced::Theme::Light`/`Dark` and adds
`StatusColor` only - no surface or control roles.

### What snora provides behind `design`

Verified against the versions that actually resolve - `snora 0.25.0`,
`snora-widgets 0.25.0`, `snora-design 0.25.2`, `lucide-icons 1.25.0`:

| Need | API |
|---|---|
| Control variants | `snora::design::button::{primary, secondary, ghost, danger}` and `*_maybe` |
| Disabled-with-`Option` | `primary_maybe(tokens: &Tokens, label: impl Into<String>, on_press: Option<Message>) -> Element` |
| Opaque surfaces | `snora::design::card::{surface, raised, selected}`; `design::style::container::{card_surface, card_raised, card_selected}` |
| Tokens | `Tokens::{light, dark, high_contrast_light, high_contrast_dark}` |
| Palette roles | `background`, `surface`, `surface_raised`, `text_primary`, `text_secondary`, `text_muted`, `border`, `accent`, `accent_text`, `success`, `success_text`, … |
| Contrast | `contrast_ratio`, `relative_luminance`, `composite_over` (with upstream tests) |
| Focus | `FocusTokens` |
| Notices, chips, progress | `notice::Notice`, `chip::{filter, removable}`, `progress::{row, card}` |
| Icons | `snora::lucide`, `snora::widget::icon::{icon_element, icon_element_sized}` |

`primary_maybe`'s `Option<Message>` shape is a direct match for knotra's
existing disabled-with-reason pattern, which keeps the migration mechanical.

## Motivation

**Correctness of the modal boundary.** A transparent dialog over live content is
not a styling preference. Layer ownership, click targets, and reading order are
ambiguous, and the user cannot tell what is live. This is the audit's one
concrete incoherent state.

**Everything else is blocked on this.** RFC-035 through RFC-038 all consume the
tokens, controls, overlay host, and shell decided here. Writing any of them
first means inventing patterns and migrating twice.

**Contrast becomes testable rather than asserted.** N-7 has been held since
RFC-021 Phase 6 by design-time calculation and code review, with no runtime
test. Adopting `snora-design` gives knotra `contrast_ratio` for free, so the
guarantee becomes an assertion that fails a build rather than a claim in a
document.

## Requirements

### Functional

R1. The workspace `snora` dependency enables `design` and `lucide-icons`.

R2. `knotra-ui` depends on `snora` and owns the semantic surface. Application
view code imports controls, tokens, and icons from `knotra_ui`, never from
`snora::design` directly.

R3. `KnotraTheme` carries a `snora::design::Tokens` handle and exposes the
colour roles of RFC-033 D7. `KnotraTheme::light()` and `dark()` map to
`Tokens::light()` and `Tokens::dark()`.

R4. `StatusColor` remains in `knotra-ui` with its RFC-021-verified values
unchanged, and continues to appear alongside text or an icon.

R5. A contrast test asserts, using `snora::design`'s own `contrast_ratio`, that
every `StatusColor` meets WCAG AA (>= 4.5:1) against its intended surface in
both themes, and that every text colour role meets AA against every surface role
it is rendered on. This test is written **before** any view migrates.

R6. `knotra-ui` provides a semantic control vocabulary covering, at minimum,
primary, secondary, ghost, danger, and icon-button roles, each with an
`Option<Message>` disabled form.

R7. New controls are **added alongside** `guided_button` and `guided_field`, not
substituted for them. Existing call sites keep working unchanged; each migrates
in its own RFC. The old helpers are deleted when their last caller migrates.

R8. One overlay host renders every modal. It provides, in a single place:

1. a full-window scrim that blocks pointer interaction with content beneath;
2. an **opaque** bounded surface using width tokens - small ~400px, standard
   ~520px, large ~680px - with 16-24px padding, a maximum height, and a
   scrollable body;
3. a stable header/body/footer structure, header owning title and close, footer
   owning Cancel plus at most one primary action;
4. focus entry to the first meaningful control, a focus trap while open, and
   focus return to the opener on close;
5. a deterministic stacking order when more than one layer can be open;
6. a phase-aware close policy shared by Escape, scrim click, and header close.

R9. The workspace-manager create, rename, and delete dialogs render through the
host. They are the validating migration.

R10. `add_project_modal`, `command_palette`, and `shortcuts_overlay` are **not**
migrated by this RFC, but the host must require no new capability to accept
them. Their migration is RFC-036.

R11. Existing close routing is preserved. `close_topmost_layer`
(`crates/knotra-app/src/app.rs`) keeps its branch ordering and its running-phase
predicates; only the layers it hides are re-pointed.

R12. A persistent application shell replaces the top strip, containing: a
workspace switcher showing the active workspace and its attention count, whose
menu owns switch, create, rename and delete; Dashboard and History destinations
with an unambiguous active state; and a right cluster with the operation/refresh
indicator, refresh, command palette, and Settings.

R13. Screens own only a page title and contextual actions. The per-screen back
navigation in `history.rs` and `settings.rs` is removed, as is the
workspace-name repetition in `dashboard.rs`.

R14. One ordinary page header is migrated to the shell's page-header pattern as
validation. Dashboard's header is the chosen one because it is the most
constrained.

R15. Workspace create, rename, and delete remain reachable and behave exactly as
RFC-023 defined; only their entry point and rendering change.

### Non-functional

R16. New or changed user-facing strings are localized in English and Japanese
and pass the first-level wording guards.

R17. No domain semantics from RFC-023 through RFC-032 change. Display, layout,
and control role may change; classification, ordering, selection membership,
lease behaviour, and VCS execution may not.

R18. Release build time and binary size are measured before and after enabling
the two features, and both figures are recorded in the implementation review
package (RFC-033 R19).

R19. No raw font size, spacing, or radius literal remains in the view code this
RFC migrates.

R20. Evidence per RFC-033 D8 for the surfaces touched: Light and Dark, English
and Japanese, 800x600 / standard / wide, plus keyboard focus order, trap, and
return for the migrated dialogs.

R21. Existing gates continue to pass: `fmt --all --check`,
`clippy --workspace --all-targets -- -D warnings`, and the three test suites.

## Goals

- Give knotra one opaque, input-blocking, focus-managing modal contract.
- Replace the ambiguous top strip with one persistent shell.
- Make the contrast guarantee a test rather than a claim.
- Establish the token and control vocabulary the remaining RFCs consume.
- Change nothing about what the application does.

## Non-goals

- Migrating the dashboard toolbar, rows, section headers, or selection bar
  (RFC-035).
- Migrating Smart Pull, Freezer, context switch, changelog, or conflict
  resolution overlays, or the remaining three ad hoc layers (RFC-036).
- Migrating Settings or History bodies (RFC-037).
- Per-project VCS history (RFC-038).
- Consolidating repeated busy text. The host and controls make it possible;
  RFC-035 does it for the selection bar.
- Deleting `guided_button` or `guided_field`.
- Animation, motion, or theme customization.
- Screen-reader/ARIA support beyond what iced 0.14 exposes.

## External Design

### Shell

```text
┌────────────────────────────────────────────────────────────────────┐
│ [ work (2) ▾ ]   Dashboard  History        ◷ idle   ⟳   ⌘K   ⚙     │ 48-52px
├────────────────────────────────────────────────────────────────────┤
│ Dashboard                                          [ Check now ]    │ page header
├────────────────────────────────────────────────────────────────────┤
│ (screen content)                                                    │
└────────────────────────────────────────────────────────────────────┘
```

The workspace switcher menu contains the workspace list plus **New workspace**,
**Rename**, and **Delete**. `+`, `Rename`, `History`, and `Settings` no longer
appear as peer buttons. The active destination is unambiguous. The page header
shows the screen name, not the workspace name.

### Overlay

```text
┌─ scrim: blocks pointer input, dispatches the phase-aware close ────┐
│              ┌─ surface: OPAQUE, token width ─┐                    │
│              │ Create workspace            ✕  │ header             │
│              ├────────────────────────────────┤                    │
│              │ Name                           │ body (scrolls)     │
│              │ [__________________________]   │                    │
│              ├────────────────────────────────┤                    │
│              │              [Cancel] [Create] │ footer             │
│              └────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────────┘
```

Focus enters the name field on open, is trapped while open, and returns to the
control that opened the dialog on close. Escape, scrim click, and `✕` share one
policy.

### What the user notices

Workspace dialogs stop overlapping the content beneath them. Workspace commands
move into the switcher menu. History and Settings lose their duplicated back
buttons. Nothing else changes: the dashboard, all bulk workflows, and every
other screen render as they do today.

## Internal Design

### Crate graph

```text
knotra-app ──► knotra-ui ──► snora (design, lucide-icons)
     └──────── layout engine only (AppLayout, Sheet, render) ────────┘
```

`knotra-ui` gains `snora`. `knotra-app` keeps its existing direct `snora`
dependency for `AppLayout` composition but takes no dependency on
`snora::design`.

### `knotra-ui` module shape

`widget.rs` is at the project's split guideline and will grow, so it becomes:

```text
crates/knotra-ui/src/widget/
  mod.rs      re-exports; keeps the existing public paths working
  button.rs   semantic variants + the legacy guided_button
  field.rs    guided_field, guided_field_focused
  overlay.rs  the overlay host surface builder
  layout.rs   spacing/typography/size tokens and layout helpers
  icon.rs     lucide wrappers with accessible names
  focus.rs    focus_id, focus_input
```

`theme.rs` keeps `StatusColor` verbatim and gains a `Tokens` handle plus role
accessors.

### Overlay host

The host is a `knotra-ui` function that wraps body content in an **opaque**
surface with header/body/footer, and an app-side composition point that routes
it through `AppLayout::dialog` so snora's layer 4 scrim and `on_close_modals`
apply.

Two things must both be true, and today neither is for the ad hoc layers and
only the first is for `ActiveModal`:

- the overlay is registered with `AppLayout` (gives scrim + input blocking);
- its surface is styled opaque (gives layer separation).

Stacking: while `AppLayout` accepts one dialog, the host defines the invariant
that at most one modal dialog is open at a time, and `close_topmost_layer`
remains the arbiter of which closes first. Where a second layer is genuinely
concurrent - the command palette over a screen - it stays on its current path
until RFC-036 migrates it under an explicit stacking rule.

Focus: the existing `focus_id` constants and `focus_input` task are the
mechanism. The host requires an "initial focus" id from its caller and returns
focus to a recorded opener id on close.

### Shell

A new `view/shell.rs` owns the bar. `view/workspace_tabs.rs` is reduced to the
switcher control or removed, with its create/rename/delete entry points moved
into the switcher menu. `app_view` composes shell → page header → screen.

No new state is required for navigation: `Screen` already carries
Dashboard/History/Settings, and workspace state already exists. The shell reads
existing state and dispatches existing messages.

### Migration strategy for controls

Additive. New semantic helpers land beside `guided_button`. Only the code this
RFC touches - workspace-manager dialogs, shell, dashboard page header - uses
them. Everything else keeps the legacy helper until its own RFC. This keeps the
diff reviewable and means a regression can only appear in migrated surfaces.

## Security Considerations

- Presentation only. No command construction, no lease acquisition, no VCS task
  is added or changed. Shell navigation, menus, and dialogs must not acquire the
  operation interlock.
- Two new dependencies enter the build: `snora-design 0.25.2` and
  `lucide-icons 1.25.0`. Both come from the same upstream as the layout engine
  knotra already ships. The implementation review must record them for
  supply-chain review, along with the measured build-time and size delta.
- Focus trapping must not create an inescapable state. Every migrated dialog
  retains at least one keyboard route to close, except where a non-cancellable
  operation deliberately owns the surface - which does not apply to any dialog
  migrated here.
- Workspace deletion keeps its existing confirmation flow (RFC-023). Moving the
  entry point into a menu must not remove a confirmation step.

## Test Plan

### Foundation tests

- **Contrast (R5), written first.** For both `Tokens::light()` and
  `Tokens::dark()`: every `StatusColor` against its intended surface, and every
  text role against every surface role it renders on, asserted >= 4.5:1 with
  `snora::design`'s `contrast_ratio`. Normal-text AA; document any value relying
  on AA-large with its role, as RFC-021 did for `Unknown` on dark.
- Token usage: no raw font size, spacing, or radius literal in migrated view
  code.
- Build cost: release compile time and binary size before and after, recorded in
  the review package.

### Overlay host contract tests

- The scrim blocks pointer interaction with content beneath.
- The surface is opaque - asserted on the style, not by screenshot.
- Focus enters the first meaningful control on open.
- Focus is trapped while the dialog is open.
- Focus returns to the opener on close.
- Escape, scrim click, and header close share one policy and produce the same
  state transition.
- `close_topmost_layer` ordering is unchanged: its existing tests must pass
  untouched.

### Workspace dialog regression tests

RFC-023's behaviour must be preserved exactly. Existing workspace tests pass
without modification; if one needs editing, the migration changed behaviour it
was not supposed to change.

- Create validates, persists, and updates the active workspace.
- Rename persists.
- Delete retains its confirmation.
- All three reachable from the switcher menu.

### Shell tests

- Dashboard and History destinations set `Screen` and show the correct active
  state.
- Settings remains reachable.
- Workspace create/rename/delete dispatch the same messages as before.
- No shell control acquires the operation interlock.

### i18n tests

- Every new key exists in English and Japanese.
- First-level wording guards pass over new `shell.*` keys.

### Evidence (RFC-033 D8)

Light/Dark x English/Japanese x 800x600/standard/wide for the shell and the
three workspace dialogs, plus keyboard focus order, trap, and return.

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

- [ ] `snora` is enabled with `design` and `lucide-icons`; `knotra-ui` depends on `snora`.
- [ ] No application view code imports `snora::design` directly.
- [ ] `KnotraTheme` carries `Tokens` and exposes the D7 colour roles.
- [ ] `StatusColor` values are unchanged.
- [ ] The contrast test exists, uses `snora::design`'s `contrast_ratio`, covers both themes, and passes.
- [ ] Semantic control variants exist in `knotra-ui` with `Option<Message>` disabled forms.
- [ ] `guided_button` and `guided_field` still exist and their unmigrated call sites are untouched.
- [ ] One overlay host provides scrim, opaque surface, header/body/footer, focus entry/trap/return, stacking order, and phase-aware close.
- [ ] Workspace create, rename, and delete render through the host and no longer overlap underlying content.
- [ ] The remaining three ad hoc layers are unchanged and need no new host capability.
- [ ] `close_topmost_layer` branch ordering and running-phase predicates are unchanged, and its tests pass untouched.
- [ ] The shell exists with workspace switcher, Dashboard/History, and the right cluster.
- [ ] Workspace create/rename/delete are commands in the switcher menu.
- [ ] Per-screen back navigation and the dashboard workspace-name repetition are removed.
- [ ] RFC-023 workspace behaviour is preserved; its tests pass unmodified.
- [ ] No RFC-023..032 domain semantics changed.
- [ ] New strings are localized in both catalogs and pass the wording guards.
- [ ] Build time and binary size deltas are recorded.
- [ ] D8 evidence supplied for the shell and the three dialogs.
- [ ] All gates pass with observed output.

## Developer Handoff

Four stages. Each ends with a green tree and is separately reviewable. Do not
reorder them - stage 2's test is what protects stages 3 and 4.

### Stage 1 - features and crate graph

1. `Cargo.toml`: `snora = "0.25"` becomes
   `snora = { version = "0.25", features = ["design", "lucide-icons"] }`.
2. `crates/knotra-ui/Cargo.toml`: add `snora = { workspace = true }`.
3. Record release build time and `target/release/knotra` size **before** step 1
   and again after stage 4. Expect roughly +33s of compile and two added
   packages (`snora-design 0.25.2`, `lucide-icons 1.25.0`); report actuals.
4. Gate: `cargo +1.91 clippy --workspace --all-targets -- -D warnings`.

Nothing else changes in this stage. `Cargo.lock` will update; commit it.

### Stage 2 - token surface and the contrast test

**Write the contrast test before migrating any view.** If snora's palette
changes an effective background, this test tells you before the screenshots do,
and before a migrated view bakes in a wrong assumption.

1. Split `crates/knotra-ui/src/widget.rs` into the `widget/` layout in the
   Internal Design. `mod.rs` re-exports everything the app already imports, so
   no `knotra-app` file changes in this stage.
2. Add the `Tokens` handle to `KnotraTheme` (`theme.rs:56-83`) and expose the
   D7 colour roles. **Do not touch `StatusColor`'s values.**
3. Write the R5 contrast test using `snora::design`'s `contrast_ratio`.
4. Add the semantic control variants, wrapping
   `snora::design::button::{primary_maybe, secondary_maybe, ghost_maybe,
   danger_maybe}`. Their `Option<Message>` shape matches knotra's existing
   pattern, so the wrappers are thin.
5. Add the overlay surface builder in `widget/overlay.rs`, using
   `snora::design::card::surface` or
   `design::style::container::card_surface` for the opaque background.

If the contrast test fails on a `StatusColor`, **stop and report the computed
ratio** rather than adjusting the value. Those values are an accepted
non-functional requirement (N-7) and moving one is a decision, not a fix.

### Stage 3 - overlay host and the validating migration

1. Build the host so both the `AppLayout::dialog` group and the ad hoc group can
   render through it. Do **not** wrap the four ad hoc layers in
   `snora::Dialog` one at a time - that reproduces the split RFC-033 D3 removes.
2. Migrate **workspace-manager only** - create, rename, delete
   (`view/workspace_manager.rs`). It has photographic evidence of the collision
   in `.git-exclude/evidence/ui-ux-review/workspace-create-en-1100x720.png`, so
   the before/after is the clearest proof the contract works.
3. Route it through `AppLayout::dialog` in `view.rs` so snora's layer-4 scrim
   applies, and give it the opaque surface from stage 2. **Both are required.**
   Routing alone leaves a transparent card; styling alone leaves the content
   beneath clickable.
4. Leave `add_project_modal`, `command_palette`, and `shortcuts_overlay` exactly
   as they are. Confirm the host needs no new capability to take them later; if
   it does, say so in the review - that is a finding about the contract, not a
   reason to migrate them here.
5. `close_topmost_layer` in `app.rs`: re-point which layer it hides, and change
   nothing else. Its branch ordering encodes the phase-aware policy from RFC-029
   and RFC-031 correctly. Its existing tests must pass **unmodified**; if one
   needs editing, you changed behaviour.

### Stage 4 - the shell

1. New `view/shell.rs`. Move the workspace switcher out of
   `view/workspace_tabs.rs:74-104` and put create/rename/delete in its menu.
2. Remove per-screen back navigation: `view/history.rs:32-40`,
   `view/settings.rs:23-31`. Remove the workspace-name repetition in
   `view/dashboard.rs:44-65`, keeping `Check now` as a page-header action.
3. Compose `app_view` as shell → page header → screen.
4. Migrate the dashboard page header only. Its toolbar, rows, and section
   headers are RFC-035; do not touch them.

### Guardrails

1. **Never modify `crates/knotra-vcs`.** This is presentation work. If it seems
   necessary, that is a scope error - stop and escalate.
2. **Do not rewrite `close_topmost_layer`.** Re-point what it hides; leave its
   branch ordering and running-phase predicates alone.
3. **Do not change `StatusColor` values.** Report a failing ratio instead.
4. **Do not delete `guided_button` or `guided_field`**, and do not change their
   signatures. New controls are additive; unmigrated call sites must keep
   compiling untouched.
5. **Do not migrate anything outside the four stages.** The dashboard body, the
   bulk modals, settings, and history bodies belong to later RFCs.
6. **Every new string goes into both catalogs** in the same commit and must pass
   the first-level wording guards.
7. **Do not change domain semantics.** If an existing test needs editing to pass,
   treat it as evidence of an unintended behaviour change.

### Leave alone

- The `ActiveModal` enum and its variants. Their overlays migrate in RFC-036.
- `snora::render`'s layer stack. It already provides the scrim; consume it.
- `Screen`. Dashboard/History/Settings is sufficient for the shell.
- RFC-023's workspace validation, persistence, and delete confirmation. Only the
  entry point and rendering change.
- The `62`-numbered artifact collision and `ROADMAP.md`. Both are separate
  bookkeeping items.

### Suggested commit shape

One commit per stage, in order, each green:

```
Enable snora design tokens and re-point the knotra-ui crate graph
Add knotra-ui token surface, semantic controls, and contrast test
Add overlay host and migrate workspace dialogs through it
Replace the top strip with the application shell
```

## Open Questions

1. **Does the workspace switcher menu need a new overlay primitive?** snora
   ships `menu.rs` in `snora-widgets`. If it fits, use it; if a menu needs the
   overlay host's stacking rules, say so in the implementation review - that is
   a genuine contract gap and RFC-036 should know before it migrates the
   palette.

2. **Should the attention count live in the switcher or the page header?**
   RFC-033 D2 puts it in the switcher. If 800x600 English plus Japanese makes
   that unreadable, report it with a capture rather than silently relocating it.

3. **`app_tab_bar` retirement.** RFC-019 adopted snora's `app_tab_bar` for
   workspace tabs. If the shell's switcher replaces it entirely, note that
   RFC-019's adoption is superseded in that respect so the decision record stays
   accurate.

## Deferred Follow-ups

- RFC-035: dashboard and selection migration, including busy-text consolidation
  and the segmented-control selected-state correction.
- RFC-036: mutating workflow overlays plus the remaining three ad hoc layers,
  under an explicit stacking rule.
- RFC-037: settings form grid and history record-list pattern.
- RFC-038: per-project VCS history.
- Deletion of `guided_button` and `guided_field` once their last callers
  migrate.
- Animation, motion, and icon tuning.
