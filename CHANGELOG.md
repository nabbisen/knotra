# Changelog

All notable changes to knotra are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.27.0] — 2026-08-10

**The overlay release.** Every modal in knotra — Smart Pull, Freezer, context
switch, conflict resolution, changelog — now renders through the same design
system the dashboard moved to in 0.25.0. Two of them had bugs that only showed
up once someone looked closely enough to migrate them.

### Fixed — the option you had chosen was the hardest one to see

In Smart Pull's plan view you pick a disposition per project: fetch only, or
stash → pull → pop. The **selected** option was drawn faded, because it was
implemented by removing its press handler and iced dims any button that cannot
be pressed. So the choice you had made looked unavailable, and the ones you had
not made looked active.

This is the same defect 0.25.0 fixed for the dashboard's Group and Sort menus.
It survived here because that work stopped at the dashboard's edge.

### Fixed — "Remove" and "No" looked equally serious

The confirmation for removing a project from a workspace styled its **Remove**
button exactly like its **No** button. Removing a project is not reversible from
the dialog, and it now looks like it.

### Changed — the modals

Migrating the five overlays onto shared components changed how they look, in the
same direction for all of them:

- **Completing actions stay put.** Save, Cancel, Close and their siblings now sit
  in a fixed footer beneath the scrolling area rather than after the content, so
  a long list of projects no longer pushes them out of reach.
- **Every button shows focus.** Keyboard focus is now visible on the controls
  inside modals, matching the rest of the application.
- **Modals are wider** — 680px rather than 580px — which gives plan tables and
  the changelog preview more room before wrapping.
- **The conflict panel stops repeating itself.** Its result banner printed the
  same sentence twice, once as a heading and once as the message. It says it once.

Nothing about what these workflows *do* changed: the same steps, the same
confirmations, and the same rules about when a modal refuses to close mid-operation.

### Changed — the minimum Rust version is now actually declared

0.26.0 corrected `rust-version` to 1.88 in the workspace manifest. That value was
never inherited by the three published crates, so it reached neither `cargo` nor
crates.io and had no effect on anything. The crates now inherit it, so building
on an older toolchain fails naming knotra rather than only its dependencies, and
the registry shows the real figure for the first time.

### Internal — RFC-037

`view/bulk_modals.rs`, a 1,337-line file holding all five overlays behind a
hand-rolled modal shell, became a `view/overlays/` directory of six files, each
under the project's 500-line threshold. Both legacy helpers it depended on —
`modal_shell` and `guided_button` — are deleted, the latter after its replacement
gained the one piece it was missing.

Six stages, twelve commits. `tests.rs` was never edited, so the existing suite
passed unmodified at every step; the single test added covers the new component's
own logic.

### Known limitations

Unchanged from 0.26.0: the Group and Sort menus still cannot be opened by
keyboard, and there is no screen-reader support. `guided_field` remains in place —
the design system never grew a text-field replacement for it, so there is nothing
to migrate those call sites to yet.

### Compatibility

No config migration. Requires Rust 1.88 or newer to build from source, which was
already true in practice and is now stated where tools can see it.

---

## [0.26.0] — 2026-08-10

**The durability release.** Your settings file can no longer be destroyed by a
crash, a full disk, or a symlink. Most of the diff is an internal decomposition
you will never see; the part you might notice is that saving got safer.

### Fixed — `config.toml` could be truncated by a crash mid-save

knotra wrote its config with a plain truncate-then-write. If the machine died,
the disk filled, or the process was killed in the window between those two
steps, the file was left empty or half-written and the next start fell back to
defaults — losing your editor path, merge tool, locale, and intervals.

The window was wider than "when you press Save Settings" suggests: `config.toml`
is rewritten whenever you change the dashboard's **Group** or **Sort**, or
collapse a section. Those are one-click actions people perform constantly.

All three writes — config, workspace definitions, and operation history — are
now atomic. The new contents go to a temporary file beside the target, are
flushed to disk, and are renamed into place, so a crash leaves either the old
file or the new one and never a broken one. An existing file's permissions are
preserved, so a mode you set deliberately is not reset by a save.

### Fixed — a symlinked `config.toml` was replaced instead of written through

Keeping `config.toml` in a dotfiles repository is a normal thing to do:

```sh
ln -s ~/dotfiles/knotra/config.toml ~/.config/knotra/config.toml
```

Under the atomic-write change above, the rename would have landed on the *link*
and replaced it with a regular file — silently, on your first Group change or
section collapse — leaving the dotfiles repo holding stale content and no longer
linked. knotra now writes through the link and leaves it in place. It also
creates the target when the target's directory exists, so you can make the link
first and let knotra populate it.

If the link cannot be followed anywhere writable — its target's directory is
missing, or it points at another broken link — knotra **refuses to save** rather
than replacing your link. Your change still applies for the rest of the session;
it is simply not written. Repair the link and change the setting again.

One caveat, stated rather than hidden: the full reason names the link and the
missing directory when you save from the Settings screen, but a Group, Sort, or
collapse failure shows only a short notice and logs the detail. Narrowing that
gap is scheduled.

### Fixed — an unresolvable config directory failed silently

If knotra could not work out where `~/.config` lives, it fell back to whatever
directory it was started from and said nothing, so settings appeared to come and
go depending on how you launched it. It still falls back — knotra does not
refuse to start over this — but now it tells you.

### Changed — minimum supported Rust is 1.88

`Cargo.toml` claimed 1.87, which was never buildable: `iced 0.14`, `image`, and
`wgpu 27` have all required 1.88 for some time, so Cargo refused before
compiling anything. Nothing that previously worked stops working; the manifest
now says what was already true, and CI checks it against the manifest on every
push so it cannot drift again.

### Internal — `handle_background` decomposition (RFC-041)

`background.rs` was 761 lines, of which 678 were a single function: one `match`
with twenty arms. It is now a `background/` directory of seven files — dispatch
plus six domain modules — each under the project's 500-line threshold. That
closes the one exception RFC-040 declared.

No behaviour change. Every one of the twenty-one moved items was verified
byte-identical to its original, and `tests.rs` was never edited, so all 255
tests passed unmodified at each of the four stages.

### Documentation

`docs/src/reference/config.md` now covers how saving works, how symlinked config
files behave, and what happens when the config directory cannot be found.

### Known limitations

Unchanged from 0.25.0: the Group and Sort menus still cannot be opened by
keyboard, there is no screen-reader support, and seven modules outside
`app/background/` remain above the 500-line threshold.

### Compatibility

No config migration. `AppConfig` gained no fields; existing `config.toml`,
workspace, and history files load unchanged. Requires Rust 1.88 or newer to
build from source.

---

## [0.25.0] — 2026-08-04

**The dashboard and decomposition release.** Two RFCs: the dashboard migrated
onto the design system 0.24.0 introduced, and `app.rs` broken apart.

### Fixed — the dashboard was keyboard-navigable but invisibly so (RFC-035)

0.24.0 shipped Tab traversal that reached the dashboard's controls and drew
nothing. Pressing Tab moved focus across the main screen with no indication of
where it had gone. That is fixed, along with the rest of the dashboard's
migration:

- **Focus is visible everywhere.** Toolbar chips and selects, section headers,
  row checkboxes, row names, row actions, and the selection bar's five controls
  each render a focus ring, verified against WCAG AA by test in both themes.
- **`↑` / `↓` / `j` / `k` move between project cards**, and `Enter` opens the
  focused card's detail panel — specified back in `rfcs/done/0016-keyboard-shortcuts.md`
  and never built until now.
- **Real controls instead of improvised ones.** Status filters are chips that
  fill when selected, rather than buttons with a `*` appended. Grouping and
  sorting are select menus showing their current value, rather than five buttons
  where the active one was greyed out — it was *disabled* to look selected, so
  the current choice was the least legible thing in the row. Row selection uses a
  real checkbox, not `[ ]` / `[x]` text.
- **Rows line up.** Bounded column tracks replace proportional fills, so
  identity, status, and action columns sit at the same offsets down the list.
- **Three width modes**, switching composition rather than wrapping: two-line
  rows and a collapsed toolbar below 1000px, bounded three-track rows to 1279px,
  centred content above that.
- **Disabled reasons are said once.** "Wait for the current operation to finish"
  appeared beneath each of four selection actions and again under row actions —
  five copies in one viewport. It now appears once for the group, with
  action-specific reasons in a single labelled slot.
- **The Select button no longer contradicts the screen.** With no projects added,
  it said "No projects match this view" beside a body reading "Add your first
  project folder." Both were true; together they were nonsense.
- **A completed run no longer reports "0 failed."**

### Changed — `app.rs` split into eleven modules (RFC-040)

`crates/knotra-app/src/app.rs` went from **3,255 to 296 ELOC**. No behaviour
changed: every function moved verbatim, and the same 166 tests passed at every
one of the sixteen commits without being edited.

The file had grown ~90 ELOC per RFC because every RFC added message handlers to
it. It is now a dispatcher plus lifecycle, with one module per message domain.

**One exception, declared rather than hidden:** `app/background.rs` is 761 ELOC.
Splitting it means extracting `match` arms, which is redesign rather than
relocation — it has its own RFC coming.

### Added — continuous integration

`.github/workflows/ci.yaml` runs formatting, lints, and all three test suites on
every push and pull request. The project previously had no gate CI; the release
workflow added in 0.24.0 covers artifacts only.

### Fixed — the Git test suite is self-isolating

`cargo test -p knotra-vcs` previously hung on an inherited editor unless six
environment variables were supplied. It now works unadorned, verified against a
hostile ambient Git configuration.

### Known limitations in this release

- **Group and Sort cannot be opened by keyboard.** Tab reaches them and the focus
  ring renders, but iced 0.14's `pick_list` does not respond to key presses, so a
  keyboard-only user cannot change grouping or sorting. This is the one reason
  the roadmap's keyboard-navigation item stays open; closing it needs a
  purpose-built select control.
- **`app/background.rs` is 761 ELOC**, above the project's own threshold, as
  described above.
- **A focused *disabled* filled button's ring measures ~3:1** — acceptable under
  WCAG 1.4.11 in dark theme, marginal in light. No colour choice improves it.
- **`guided_button` still serves 11 call sites** in the add-project modal, bulk
  modals, and dashboard empty states. Retiring it is scheduled work.
- **No screen-reader support**, and none planned while knotra targets iced 0.14,
  which exposes no accessibility API.
- **User-supplied names are passed to `git` without a `--` separator** — a name
  beginning with `-` would be read as an option. Local and self-inflicted only;
  no remote-controlled value reaches an argument position. Unchanged from 0.24.0.

### Compatibility

No user action required. No configuration fields were added or changed; existing
`config.toml`, workspace, and history files load unchanged.

---

## [0.24.0] — 2026-07-30

**The Production Readiness Reset release.** Twelve RFCs, closing the gap between
what knotra's interface advertised and what it actually did. The rule the reset
enforced: every visible control must complete its action, be disabled with a
stated reason, or be hidden.

### Fixed — every visible control now completes its action (RFC-023 – RFC-032)

Before this release many controls rendered but did not reach a working code path.
Each item below was a visible button, menu entry, or modal that did not do what
it said.

- **Workspace management** (RFC-023, `main: 02e1481`) — `+ New workspace` opened
  nothing, because the create/rename/delete dialogs were never rendered by the
  active view stack. Create, rename, delete, name validation, persistence,
  active-workspace switching, and keyboard close now all work.
- **Smart Pull** (RFC-024, `main: 4362a2e`) — `Get latest safely` did not
  reliably connect to plan generation, reviewed execution, operation logging, or
  result persistence. The full plan → review → progress → result flow now runs.
- **Save release point** (RFC-025, `main: d9f687a`) — the modal's primary action
  was not wired to execution. Validation, tag/bookmark creation, rollback on
  failure, and Git tag push are now reachable from the button.
- **Conflict resolution** (RFC-026, `main: 1cde97d`) — `Open in editor`,
  `Mark done`, and `Stop this fix attempt` did not consistently reach their
  backend paths. **Security fix in the same change:** the direct editor path
  built a shell command from user-controlled text and ran it through `sh -c`.
  That path is removed; editors are launched without a shell.
- **Selection mode** (RFC-027, `main: 0fd1e22`) — selection state, card
  checkboxes, the selection bar, and the bulk actions existed as disconnected
  pieces. Selection mode now has defined entry and exit and drives the bulk
  workflows coherently.
- **Command palette** (RFC-028, `main: 3699bad`) — several action keys and all
  project rows dispatched nothing, and the palette closed anyway, so it appeared
  to have acted. Every advertised entry now executes or is absent.
- **Context switching** (RFC-029, `main: 9821bef`) — `Change work area` offered
  a mostly free-text field, flattened candidate metadata into a `String`, and
  guessed "remote branch" from whether the target contained a `/`. Candidates are
  now typed and reviewable, and remote branches are identified rather than
  inferred.
- **Changelog modal** (RFC-030, `main: fb551ae`) — the ready state rendered
  `format!("{:?}", draft)` and the copy button copied that Rust debug string to
  the clipboard. It now renders and copies the intended Markdown.
- **Activity retry** (RFC-031, `main: 0936345`) — `Retry` appeared on every
  failure but only navigated to History. Retry now carries the operation's
  identity, kind, failed project IDs, and parameters, and is offered only where
  replay is safe.
- **Dashboard grouping and sorting** (RFC-032, `main: 43bace5`) — `Group by` was
  inert and two render paths competed. Grouping (attention / project group /
  none) and sorting (urgency / name) are now independent, visible, and persisted
  in your configuration.

### Added — UI/UX foundation (RFC-033, RFC-034)

- **Design foundation, application shell, and overlay host** (RFC-034,
  `main: ce05a44`), under the RFC-033 umbrella (`main: bf07f1c`): a persistent
  application shell, a single overlay host, a semantic control vocabulary, three
  responsive layout modes, and a shared token set. Minimum supported window size
  is now **800×600**.

### Added — keyboard navigation and focus traversal (RFC-036)

`main: d20c7be`. Before this, Tab did nothing anywhere in knotra and there was no
focus indication at all — iced 0.14 implements focus only for text inputs, so
knotra now owns its own focus model.

- Tab / Shift-Tab traversal across the shell and dialogs.
- Overlay focus entry, trap, and return-to-opener.
- A 2px focus ring, verified against WCAG AA by test in both themes, on the shell
  and all three workspace-manager dialogs. Destructive dialogs enter with focus
  on Cancel, never on the destructive action.
- Bare `/` focuses search. `Ctrl+/`, previously the only way to do this, still
  works.

See Known limitations below for what this does **not** yet cover.

### Changed — adopted snora's design system (reverses a prior deferral)

knotra now enables `snora`'s `design` and `lucide-icons` features. Release 0.23.0
evaluated and explicitly deferred this, on the grounds that `knotra-ui` already had
an equivalent layer; measurement during RFC-033 put the cost at two additional
packages and the earlier decision was reversed. `snora`'s presets are consumed
unmodified.

The migration is **incomplete by design**: the previous `guided_button` /
`guided_field` helpers are still in use alongside the new semantic controls, so
two control systems coexist in this release. Retiring the older one is scheduled
work, not an oversight — see Known limitations.

### Fixed — dark mode did not reach every widget

`config.dark_theme` was never applied as iced's ambient theme, so widgets using
default styling stayed light regardless of the setting. The theme is now applied
at the application root.

### Known limitations in this release

Documented rather than omitted:

- **Dashboard rows and toolbar carry no focus ring.** They are in the Tab order,
  so keyboard focus moves through the main screen with no visible indication.
  Their styling belongs to the next release's dashboard migration.
- **Two control systems coexist.** The `guided_button` / `guided_field` helpers
  remain alongside the new semantic controls; the dashboard has not yet been
  migrated off them.
- **No dashboard card-to-card arrow movement or Enter-to-open**, despite being
  specified in `rfcs/done/0016-keyboard-shortcuts.md`.
- **A focused *disabled* filled button's ring measures ~3.0–3.3:1** — it meets
  WCAG 1.4.11 non-text contrast (3:1) in dark theme but not quite in light. No
  colour choice improves this; it needs a second visual channel.
- **No screen-reader or ARIA support**, and none is planned while knotra targets
  iced 0.14, which exposes no accessibility API.
- **A delete-dialog sentence renders a stray period**, and its Japanese
  translation inherits a Latin full stop.
- **User-supplied names are passed to `git` without a `--` separator.** A tag
  name, branch name, or manually typed switch target beginning with `-` would be
  read by `git` as an option rather than a name. Local and self-inflicted only —
  no remote-controlled value reaches an argument position, and `fetch` uses fixed
  arguments — but it is unguarded. Pre-existing; hardening is scheduled.

### Compatibility

No user action required. Four dashboard preference fields were added to the
configuration with defaults, so existing `config.toml` files load unchanged.
Workspace and history persistence formats are untouched.

---

## [0.23.0] — 2026-06-11

### Changed — Migrate to snora 0.25.0 (RFC-0022)

`snora` dependency bumped from `0.18.1` to `0.25.0`. No knotra source changes.

snora 0.25.0 introduces the opt-in Snora Design System (`design` feature:
iced-free token crate, semantic palette, automated contrast tests, button/
card/notice/chip/progress helpers). The two breaking changes in the
0.18 → 0.25 range — `Palette::roles()` removed (v0.24) and the chip
selected-state visual change (v0.24) — affect only snora's `design` surface,
which knotra does not use. knotra consumes snora purely as a layout engine
(`AppLayout`, `Dialog`, `Sheet`, `render`, `app_tab_bar`), all unchanged.
iced stays at 0.14.

**The `design` feature is evaluated and deferred, not adopted.** knotra
already has a complete, WCAG-AA-verified design layer in `knotra-ui`
(`KnotraTheme`, `StatusColor`, `guided_button`/`guided_field`, focus tokens).
Enabling snora's `design` would duplicate it; the lean choice is to stay on
knotra-ui's own layer. RFC-0022 records when this decision should be
revisited.

---

## [0.22.0] — 2026-06-11

### Changed — RFC-0021 Phase 6: accessibility hardening

**Contrast fix (WCAG AA):** Light-theme `Behind` and `Dirty` status colours
corrected from `#E65100` (3.5:1, failed AA for normal text) to `#BF4600`
(4.71:1, passes AA). All six `StatusColor` values now meet WCAG AA on both
dark and light backgrounds. `Unknown` on dark (3.7:1) meets AA-large as
before; it is used only as a secondary label alongside an icon.

**Programmatic focus (keyboard UX):** Six `widget::Id` constants added in
`knotra_ui::widget::focus_id`. `focus_input()` helper wraps
`iced::widget::operation::focus`. Auto-focus applied to:
- Add Project dialog → path field on open; name field on advance to step 2
  (both manual Next and Browse auto-advance)
- Command palette → query field on open
- Save release point modal → release name field on open
- Change work area modal → target field on open

**`guided_field_focused`** variant added: identical to `guided_field` but
assigns a `widget::Id` to the underlying input.

**Accessible labels on icon-only controls:**
- Dashboard refresh: `⟳` → `⟳  Check now` / `⟳  Checking…`
- Workspace tabs: `⊟` → `⊟  History`, `⚙` → `⚙  Settings`,
  `+` → `+ New workspace`
- Activity strip details: `›` → `› Details`

**Confirm remove dialog:** Upgraded to `guided_button`, 44px controls,
plain wording ("Remove this project?" / "This only removes it from knotra.
Your project folder stays on this computer." / "Remove from list"), and
safe-action-first button order (Cancel on left, Remove on right).

**Modal width:** Fixed 600px modal width changed to `Fill` with
`max_width(580)` — prevents overflow at the 800px minimum window width.

**Shortcuts overlay:** Selection-mode binding descriptions updated to
plain language, e.g. "Fetch selected projects" → "Check for updates (fetch)".

**Pre-existing catalog gaps closed:** 33 missing i18n keys added to both
English and Japanese (`history.*`, `settings.*`, `topology.scan`,
`plain.remove.*`, `plain.add_workspace`). Views for History and Settings
were already using these keys but falling back to the raw key string.

---

## [0.21.0] — 2026-06-11

### Changed — RFC-0021 Phase 5: guided setup, empty states, undo for removal

**2-step Add Project flow** (`view/add_project_modal.rs`):
- Step 1: "Choose the folder that contains your project." — folder field,
  Browse button, Next (disabled with reason when empty).
- Step 2: "Give this project a name." — shows chosen folder as read-only
  confirmation, name field, Add project (disabled with reason when empty),
  Back to return to Step 1.
- Browse auto-advances to Step 2 and auto-fills the project name from the
  folder name. `AddProjectStep` enum and `AddProjectNextStep` message added.

**Empty states** (`view/dashboard.rs`):
- No-projects welcome: "Welcome to knotra" with a plain description and an
  "Add project folder" primary button (44px, prominent).
- All-clean: "🎉 All set — Every project is up to date." shown when all tiers
  are empty and no filter is active.
- No-filter-match: "No projects match the current filter." shown when tiers
  are empty because of an active filter.

**Undo for project removal** (`view/activity_strip.rs`, `app.rs`):
- `RemoveProjectConfirmed` now captures a snapshot of the removed project and
  its last status before deletion, stored as `AppState::recent_removal`.
- The activity strip displays an "Undo" snackbar when `recent_removal` is set,
  with "Undo" (restores project + status) and "Dismiss" buttons.
- `WorkspaceMessage::UndoRemoval` and `DismissUndoSnackbar` handle the two
  outcomes. Adding a new project clears any pending undo.

**30 new i18n keys (EN + JA)**, all passing the wording guard.

---

## [0.20.0] — 2026-06-11

### Changed — RFC-0021 Phases 2–4: guided modal flows and safe components

**Phase 2 — safe component helpers** (`knotra-ui::widget`):
- `guided_button(label, on_press, reason)` — renders a reason beneath a
  disabled button so users always know why they cannot proceed.
- `guided_field(label, placeholder, value, on_change, error)` — labelled
  input that keeps the label visible on focus and shows inline errors.

**Phase 3 — guided "Get latest safely"** (Smart Pull modal):
- Four named views: preparing → plan review → in-progress → result.
- Plan table shows plain-language dispositions: "Get latest", "Check only",
  "Get latest anyway" (stash), "Skip". Developer terms (fetch, pull, stash,
  fast-forward) shown only under "Show details".
- Dirty-project rows show inline toggle between "Check only" / "Get anyway".
- Result view: plain summary ("3 done. 1 needs help.") with per-project rows;
  technical commands shown under "Show details" toggle.
- `show_op_details: bool` field added to `AppState`; `Message::ToggleOpDetails`
  toggles it and is used by all three modal result views.

**Phase 4 — guided "Save release point"** (Freezer / Tag modal):
- Five views: name + note input → validation (ready check) → executing → result.
- Validation table uses plain labels: "Ready", "Not included", blocker messages
  in plain language ("This release name is already in use", "Needs your
  choice — resolve it first", "Has unsaved work").
- Primary button disabled with a reason: "Fix 1 item before saving." or
  "Fix highlighted items before saving." (never just greyed out silently).
- Result views: "Release point saved.", "We stopped and undid all changes.",
  "We could not undo everything." Technical rollback output behind "Show details".

**Conflict resolve panel** (also Phase 3):
- "Open in editor" button added per file; launches `external_editor` via shell.
- `ConflictOpsMessage::OpenInEditorRequested(path)` added and handled.
- Button labels: "Mark done", "Stop this fix attempt" (was "Mark resolved",
  "Abort merge").

**All 72 new i18n keys added in English and Japanese.** The wording guard test
(`first_level_wording_has_no_developer_jargon`) caught two violations during
implementation ("conflict" leaked into two note strings) and forced corrections
before the tests could pass — validating the Phase 1 regression guard works.

---

## [0.19.0] — 2026-06-11

### Changed — Plain-language layer for non-technical users (RFC-0021, Phase 1)

First-level interface wording now uses goal-oriented plain language; expert
terms remain available behind "Show details".

- Dashboard tiers: "Needs attention / Active / Clean" → **Needs help /
  In progress / All set**
- Card status labels: Conflict → **Needs your choice**, Uncommitted →
  **Unsaved work**, Behind → **Updates available**, Ahead → **Unshared
  changes**, Unknown → **Not sure yet**
- Selection-bar actions: Fetch → **Check for updates**, Pull → **Get latest
  safely**, Tag → **Save release point**, Switch → **Change work area**
- Tier and selection-bar labels are now routed through the i18n catalog
  (previously hardcoded English); new `plain.*` / `tier.*` keys added in
  both English and Japanese
- Accessibility: 44px minimum touch target (`widget::BUTTON_HEIGHT`) applied
  to selection-bar actions; `FONT_BODY` (15px) token added

### Added

- `knotra-ui::i18n` tests: `first_level_wording_has_no_developer_jargon`
  (guards against jargon leaking into first-level wording) and
  `plain_keys_are_localised_in_both_catalogs` (guards Japanese coverage)

---

## [0.18.0] — 2026-06-11

### Changed — Migrate to endringer 0.33.1 (RFC-0020)

`knotra-vcs` dependency bumped from `endringer-* 0.19.2` to `0.33.1`
(the project's declared stable version: 8/9 stabilisation gates complete,
317 tests, gix 0.84). No knotra source code changes.

The breaking changes in the 0.19.2 → 0.33.1 range (typed error return,
`TagAnnotation` field addition, new required `VcsBackend` methods) are all
transparent to knotra-vcs: every endringer call site uses `.ok()` or
`.to_string()`, no `TagAnnotation` literals are constructed, and knotra-vcs
does not implement `VcsBackend`.

Notable new endringer API available for future RFCs: `operation_state()`,
`conflict_summary()`, `branch_ahead_behind()`, `snapshot()`,
`rich_worktree_status()`, `query_commits()`.

---

## [0.17.0] — 2026-06-11

### Changed — Remove legacy full-screen views (RFC-0017)

The five screens replaced by modals in RFC-0013 are now deleted.
`Screen` is trimmed from eight variants to three: `Dashboard`, `History`,
`Settings`. All navigation that previously went to a legacy screen now
opens the corresponding modal or panel instead.

- `SyncMessage::OpenRequested` → `ActiveModal::Pull`
- `FreezerMessage::OpenRequested` → `ActiveModal::Tag`
- `ContextMessage::OpenRequested` → `ActiveModal::Switch`
- `ConflictOpsMessage::OpenRequested(id)` → `ActiveModal::Resolve(id)`
- `ChangelogMessage::OpenRequested` → `ActiveModal::Changelog`

Removed view modules (1,262 lines): `view/sync_center.rs`,
`view/freezer.rs`, `view/context_ops.rs`, `view/conflict_ops.rs`,
`view/changelog_view.rs`. State modules for all five features are
retained — the modals and panels consume the same state.

---

## [0.16.0] — 2026-06-10

### Changed — Adopt snora 0.18 layout framework (RFC-0019)

- **`snora = "0.18"` added to `knotra-app`.** The snora engine (`render`,
  `AppLayout`, overlay vocabulary) replaces the hand-rolled `stack!`
  layer-composition in `view/mod.rs::app_view`.

- **Modal overlays re-layered onto snora's `AppLayout`.**  
  `ActiveModal::{Pull, Tag, Switch, Changelog}` → `AppLayout::dialog(Dialog::new(el))`.  
  `ActiveModal::Resolve` → `AppLayout::sheet(Sheet::new(el).at(SheetEdge::End).with_size(SheetSize::Half))`.  
  The modal dim backdrop, click-outside close sink, and z-order are now
  managed by snora. `on_close_modals` dispatches
  `Message::Shortcut(ShortcutMessage::Close)`, which now also clears
  `active_modal` (previously it did not).  
  Command palette, shortcuts overlay, and add-project modal retain their
  own stack layers above `render(layout)` (they have independent state
  channels).

- **Workspace tab strip migrated to `snora::widget::app_tab_bar`.**
  `view/workspace_tabs.rs` replaced: the workspace list is now a
  `TabBar<WorkspaceId>` rendered by `app_tab_bar`, which is direction-aware.
  Attention-count badges are embedded in the `Tab::label`. The fixed
  action buttons (`+`, history, settings) remain as a row alongside the
  tab strip.

- **`knotra-ui::nav_menu` removed.** The dead `nav_bar` / `NavItem` /
  `NAV_BAR_HEIGHT` module (unused since v0.15.0) is deleted. Its role is
  superseded by snora's `app_header` / `render_menu` / `app_side_bar`.

- **`knotra-ui` unchanged.** `KnotraTheme`, `StatusColor`, the i18n
  catalog, and card layout tokens are knotra-specific and are kept.

---

## [0.15.0] — 2026-06-10

### Changed — Migrate onto the published `endringer` crates (RFC-0018)

The in-tree VCS backend crates were the published `endringer` crates
vendored at 0.14; knotra now consumes them from crates.io instead.

- Removed the in-tree `endringer-backend-core`/`-git`/`-jj`/`-async`
  crates; the workspace now depends on published
  `endringer-core`/`-git`/`-jj`/`-async` **0.19.2**. Read types and the
  `VcsBackend` trait are identical across the two; the facade's tag writes
  and async reads needed no signature changes.
- Renamed the in-tree facade crate `endringer` → **`knotra-vcs`**
  (`VcsAdapter` + domain model + `FsPoller`; writes stay on the VCS CLI
  per constraint C-1), now layered over the published reads.
- Renamed the in-tree `snora` foundation → **`knotra-ui`**
  (`KnotraTheme`, `StatusColor`, the i18n catalog, layout tokens),
  resolving the name collision with the published `snora` crate. No
  dependency on published `snora` is added: knotra consumes none of its
  surface today, and adopting its layout framework (prefab widgets,
  `render()`/`AppLayout`, ABDD RTL) is deferred to a future UI RFC.
- `knotra-app` imports updated (`endringer::` → `knotra_vcs::`,
  `snora::` → `knotra_ui::`); message/state/view logic unchanged.

### Fixed

- Two test fixtures (`state/sync.rs`, `state/dashboard.rs`) built
  `ConflictStatus` without the `detection_unavailable` field added in
  RFC-0003; they now compile under the test profile.
- Cleared pre-existing clippy lints across the workspace to restore the
  0-warning baseline (`collapsible_if`, `needless_return`,
  `field_reassign_with_default`, `module_inception` in `tests.rs`,
  `sort_by_key`, `slice::from_ref`).

---

## [0.14.0] — 2026-05-24

### Changed — Less is more: UI simplification

Applied the "less is more" design principle across the entire UI.  No features
were removed; information was moved to the right level of the information
hierarchy.

#### Cards: 15 data points → 2–3

Cards now use tier-based density rather than showing everything on every card.

| Tier | Before | After |
|------|--------|-------|
| Needs Attention | name · VCS badge · status label · branch · ↑↓●? counters · timestamp · Fetch · Remove | name — problem · **one action** |
| Active | same 15 items | name · branch |
| Clean | same 15 items | name only |

`vcs_label`, `status_label` (beyond problem text), `stat_cell` counters, the
`last_updated` timestamp, and the per-card **Fetch** / **Remove** buttons are
no longer on cards.  All this information remains available in the detail panel
(click the project name).

#### Dashboard header: 5 items → 2

Before: workspace name · last-updated time · Add project · Refresh · Bulk sync  
After: workspace name · ⟳ refresh icon

Add project lives in the empty state only.  Bulk sync is accessible via the
selection bar.  Last-updated moves to the detail panel.

#### Selection: always-on checkboxes → explicit mode

Checkboxes are now hidden by default.  Selecting any project (keyboard or
click inside selection mode) enters **selection mode**, which shows checkboxes
and the selection bar.  Exiting via "Exit selection" hides them again.

This keeps clean cards truly clean.  A new user never sees a checkbox until
they need to bulk-select.

#### Navigation: 2 rows → 1

The nav bar added in v0.13.0 (Dashboard / History / Settings row) has been
**removed** — it was adding chrome without removing friction.

Navigation is now:
- **Workspace tab strip** (top): workspace tabs + `+` new workspace + `⊟` History + `⚙` Settings
- **Command palette** (⌘K): every action by name

#### Empty state: clear call to action

When a workspace has no projects, the dashboard shows a centred prompt:

```
No projects yet.
Add a project folder to get started.

    [ + Add project ]
```

The add button is prominent when it matters and invisible otherwise.

#### snora: nav_menu module added

`snora::nav_menu` provides `nav_bar()` and `NavItem` for horizontal
navigation menus.  Not used in the main layout (which no longer has a nav
bar) but available for future use in settings screens or onboarding flows.

### Tests

- endringer unit: 17 pass.
- endringer integration: 19 pass.
- knotra-app: `cargo check` clean — 0 errors, 0 warnings.


## [0.13.0] — 2026-05-24

### Added — Navigation menu, Add Project modal, archive structure fix

#### Navigation menu (`view/nav_bar.rs`, `snora::nav_menu`)

A horizontal navigation bar is now rendered below the workspace tabs on every
screen.  It replaces the old sidebar navigation with a leaner, always-visible
strip:

```
[Dashboard]  [History]  [Settings]          [+ Add project]
```

- **Left side**: tab-style nav items from `snora::nav_menu::nav_bar()`.
  The active screen is indicated by a `•` prefix.  Clicking a non-active
  item navigates immediately.
- **Right side**: **Add project** button — moved here from the bottom-left
  dashboard toolbar where it was easy to miss.

New in `snora`: `nav_menu` module with `NavItem`, `nav_bar()`, and the
`NAV_BAR_HEIGHT` constant.  Consumers call `nav_bar(items)` passing a
`Vec<NavItem>` — labels, active flag, and dispatch message — and receive
a full-width `Element`.

#### Add Project modal (`view/add_project_modal.rs`)

The dialog is now a **centred stack overlay** (same layer as the bulk
action modals) instead of being appended below the dashboard content.

The path field gains a **Browse…** button that opens a native OS folder
picker via `rfd::AsyncFileDialog`:

- Folder picker opens without blocking the UI thread (async task).
- On selection, the path field is populated.
- If the project name field is still empty, it is auto-filled from the
  folder's last path component.
- Dismissing the picker (Cancel) leaves both fields unchanged.

`WorkspaceMessage::BrowsePathRequested` and
`WorkspaceMessage::BrowsePathSelected(Option<String>)` are the two new
message variants.

#### Archive structure fix

Release archives now contain `knotra-vX.Y.Z/(files)` directly, without
an intermediate `knotra/` subdirectory.  The packaging command uses
`tar --transform` to rename the root directory at archive time.

Before: `knotra-v0.12.1.tar.gz` → `knotra/Cargo.toml`  
After:  `knotra-v0.13.0.tar.gz` → `knotra-v0.13.0/Cargo.toml`

### Changed

- `snora`: new public `nav_menu` module; `nav_bar`, `NavItem`,
  `NAV_BAR_HEIGHT` exported from `snora` root.
- `view/mod.rs`: `app_view` now includes the nav bar in every layout;
  `add_project_modal` overlay inserted into the stack layers.
- `view/dashboard.rs`: removed the old inline dialog append; removed the
  `add_btn` from the dashboard toolbar row (it lives in the nav bar now).
- `view/add_project_dialog` (old private fn): replaced by
  `view/add_project_modal.rs` (public module, centered overlay).
- `i18n`: new key `dialog.add_project.browse` (en + ja).

### Tests

- endringer unit: 17 pass.
- endringer integration: 19 pass.
- knotra-app: `cargo check` clean — 0 errors, 0 warnings.


## [0.12.1] — 2026-05-23

### Changed — RFC directory restructured to follow lifecycle policy

Documentation-only release.  No code changes.

`rfcs/` has been reorganised from a flat directory into a four-folder
layout following [RFC 000 — RFC lifecycle policy](rfcs/done/000-rfc-lifecycle-policy.md).

#### New structure

```
rfcs/
  README.md           ← state-grouped index (rebuilt)
  proposed/           ← RFCs open for review (do not implement yet)
  done/               ← implemented RFCs; historical record
  archive/            ← withdrawn or superseded RFCs
```

#### Moves

| File | From | To | Reason |
|------|------|----|--------|
| `000-rfc-lifecycle-policy.md` | (project docs) | `done/` | Policy is in effect; self-placed per §Self-application |
| `0001`–`0016` | flat `rfcs/` | `done/` | All implemented (v0.11.0 / v0.12.0) |
| `0017-screen-removal.md` | flat `rfcs/` | `proposed/` | Not yet shipped; targeted at v0.16 |

#### Status field updates

All RFC files in `done/` now carry `Implemented (vX.Y.Z)` in their
Status field, matching their folder.  `0017` retains `Proposed`.


## [0.12.0] — 2026-05-23

### Added — UI/UX Redesign: RFC 0009–0016

All eight code RFCs from the UI/UX redesign are implemented.  No screens are
removed in this release (that is RFC-0017, targeted at v0.16); all existing
navigation continues to work alongside the new panels and modals.

#### RFC-0009 — Selection model

`SelectionState` added to `AppState`.  Every dashboard card now has a
checkbox.  Clicking the checkbox (or pressing `Space` on a focused card)
toggles it.  A sticky selection bar slides up from the bottom of the window
whenever ≥ 1 project is selected, showing the count and primary actions:
**Fetch**, **Pull…**, **Tag…**, **Switch…**.  Keyboard: `Space` toggle,
`Ctrl+A` / `⌘A` select all, `Esc` clear.

New state: `state::SelectionState` with `toggle`, `select_range`,
`select_all`, `clear`.  
New message enum: `SelectionMessage` (Toggled, RangeTo, SelectAll, Clear,
FocusMoved).  
New view: `view/selection_bar.rs`.

---

#### RFC-0010 — Three-tier attention grouping

`state/tier.rs` introduces `compute_tier(status, path_exists) →
(AttentionTier, Option<AttentionCause>)`, which maps any `ProjectStatus` to
one of:

- 🔴 **NeedsAttention** — conflict, detection-unavailable, detached HEAD,
  path missing, or a read error.
- 🟡 **Active** — uncommitted changes, ahead/behind upstream, or
  non-default branch.
- ⚪ **Clean** — synced, default branch, working tree clean.

Dashboard uses `view_tier_grid` when `AppState::grouping_mode ==
GroupingMode::Auto` (default).  Each tier has a collapsible header with
a count badge.  The Clean tier is collapsed by default.  Legacy filter-chip
grouping is still available by switching `GroupingMode::Legacy` via the tier
message handler.

New state: `AttentionTier`, `GroupingMode`, `TierCollapseState`,
`AttentionCause` (in `state/tier.rs`).  
New message enum: `TierMessage` (Toggled, GroupingModeChanged).

---

#### RFC-0011 — Activity strip

A single-line status bar at the very bottom of the window.  Hidden when idle.
Transitions through four states:

- **Running** — `⟳ Fetching… 3/12` with done/total counter.
- **Success** — `ⓘ Fetched 12 projects`.
- **PartialFailure** — `⚠ … failed: project-alpha` with a **Retry** button.
- **TotalFailure** — `✗ …` with a **Retry** button.

`›` button opens the history popover.

New state: `LatestOpState`, `ActivityStripState` on `AppState`.  
New message enum: `ActivityMessage` (Started, Progress, Completed,
PopoverToggled, RetryRequested, Tick).  
New view: `view/activity_strip.rs`.

---

#### RFC-0012 — Command palette

`⌘K` / `Ctrl+K` opens a centered floating input.  Typing runs
case-insensitive substring search across:

- **Actions** — 16 built-in entries (Fetch all, Pull selected, Tag selected,
  Switch branch, Generate changelog, Add project, Create workspace, Select
  all, Clear selection, Open Settings, Open History, Refresh, Show shortcuts…).
- **Projects** — all projects in the current workspace.
- **Workspaces** — all workspaces.

Selecting an entry dispatches its corresponding `Message`.  `Esc` closes.
Up/Down arrows navigate; `Enter` confirms.

New state: `PaletteState`, `PaletteEntry`, `PaletteEntryKind` on `AppState`.  
New logic: `state/palette.rs` with `update_results` and `dispatch_entry`.  
New message enum: `PaletteMessage`.  
New view: `view/command_palette.rs`.

---

#### RFC-0013 — Bulk action modals

Five workflow modals rendered over the dashboard via `iced::widget::stack`:

| Modal | Trigger | State path |
|-------|---------|-----------|
| **Pull** | Selection bar `Pull…` | `state::sync` |
| **Tag** | Selection bar `Tag…` | `state::freezer` |
| **Switch branch** | Selection bar `Switch…` | `state::context_ops` |
| **Resolve** | Card `Resolve…` button | `state::conflict_ops` |
| **Changelog** | Command palette | `state::changelog` |

Each modal is pre-populated from the dashboard selection.  The existing
state machines (`SyncPhase`, `FreezerPhase`, `ChangelogPhase`, etc.) are
reused unchanged; only the view layer changed.

New state: `ActiveModal` enum on `AppState`.  
New view: `view/bulk_modals.rs` (pull_modal, tag_modal, switch_modal,
resolve_panel, changelog_modal).  
New `app_view` in `view/mod.rs` stacks modals over the base layout.

---

#### RFC-0014 — Project detail side panel

Clicking a project **name** (not the checkbox) opens a right-docked 300 px
panel showing:

- **Identity** — VCS, path, remote upstream.
- **Status** — branch, ahead, behind, dirty count, untracked, conflict flag.
- **Recent operations** — last 5 ops touching this project (icon + kind +
  timestamp).
- **Actions** — Refresh, Fetch, Remove from workspace.

Panel is semi-modal: the main view remains interactive while it is open.

New state: `DetailPanelState` on `AppState`.  
New message enum: `DetailPanelMessage` (Opened, Closed).  
New view: `view/detail_panel.rs`.

---

#### RFC-0015 — Workspace tabs

A horizontal tab strip replaces the sidebar workspace list at the top of
every screen.  Each tab shows the workspace name and a parenthetical count
of **Needs Attention** projects (e.g. `work (3)`).  The active workspace
tab has its button's `on_press` disabled.

`⌘1` – `⌘9` / `Ctrl+1` – `Ctrl+9` switches to workspace by index.
A `+` button at the end opens the create-workspace dialog.

New view: `view/workspace_tabs.rs`.  
Keyboard handling updated in the `subscription` function.

---

#### RFC-0016 — Keyboard shortcuts and cheat sheet

`?` toggles a centred overlay listing all 17 documented key bindings across
five contexts (Global, Dashboard, Selection, Palette, Modal).

Leader-key state machine: pressing `g` sets `LeaderKeyState::G`; a
subsequent `h` navigates to History, `s` to Settings.

New state: `KeyboardState` (cheat_sheet_open, leader: `LeaderKeyState`).  
New message enum: `KeyboardMessage` (CheatSheetToggled, LeaderGPressed,
LeaderCancelled).  
New view: `view/shortcuts_overlay.rs`.

---

#### RFC-0017 — Screen removal

**Deferred to v0.16.**  All five screens (Sync Center, Freezer, ContextOps,
ConflictResolution, Changelog) remain accessible via sidebar.  The modals
introduced in RFC-0013 provide the preferred workflow path for v0.12–v0.15.

---

### Changed

- `AppState` gains 8 new fields: `selection`, `activity`, `palette`,
  `grouping_mode`, `tier_collapse`, `keyboard`, `detail_panel`,
  `active_modal`.
- Dashboard view switches between legacy grid and tier grid based on
  `grouping_mode` (default: Auto / tier grid).
- `app_view` now renders workspace tabs at top, selection bar and activity
  strip at bottom, and modal/palette/cheat-sheet overlays via
  `iced::widget::stack`.
- Clicking a project name now opens the detail panel rather than doing nothing.
- `SyncCenterState` gains `selected_project_ids: HashSet<ProjectId>`.
- `ContextOpsState` gains `target_context: String`.
- `ChangelogState` gains `since_ref: String`.

### Tests

- **endringer unit:** 17 pass.
- **endringer integration:** 19 pass.
- **knotra-app:** `cargo check` clean, 0 errors, 0 warnings.


## [0.11.1] — 2026-05-23

### Added — UI/UX Redesign RFCs (0009 – 0017)

This is a **documentation-only release**.  No code changes; nine detailed
RFCs for the v0.12 → v0.16 UI/UX redesign are added under `rfcs/`.  Together
they describe the migration from screen-based navigation to a single-view,
selection-driven dashboard.

| RFC  | Title                                       | Target | Effort       |
|------|---------------------------------------------|--------|--------------|
| [0009](rfcs/0009-selection-model.md)        | Selection model and selection bar          | v0.12 | Medium        |
| [0010](rfcs/0010-attention-tiers.md)        | Three-tier attention grouping (Needs Attention / Active / Clean) | v0.13 | Medium |
| [0011](rfcs/0011-activity-strip.md)         | Activity strip at bottom of window         | v0.12 | Small–Medium  |
| [0012](rfcs/0012-command-palette.md)        | Command palette (⌘K) with fuzzy search     | v0.12 stub / v0.13 full | Medium |
| [0013](rfcs/0013-bulk-action-modals.md)     | Bulk action modals replacing 5 screens     | v0.14 | **Large**     |
| [0014](rfcs/0014-project-detail-panel.md)   | Right-docked project detail side panel     | v0.15 | Medium        |
| [0015](rfcs/0015-workspace-tabs.md)         | Workspace tabs at top + ⌘1/⌘2 shortcuts    | v0.15 | Small–Medium  |
| [0016](rfcs/0016-keyboard-shortcuts.md)     | Keyboard shortcuts table + `?` cheat sheet | v0.13 | Medium        |
| [0017](rfcs/0017-screen-removal.md)         | Removal of Sync Center / Freezer / ContextOps / Conflict Resolution / Changelog screens | v0.16 | Small–Medium |

### Design Source

The redesign rationale and IA decisions are documented in
[`docs/src/contributing/ui-ux-redesign.md`](docs/src/contributing/ui-ux-redesign.md).
Each RFC carries forward the relevant design decisions with implementation-level
detail — state shape, message variants, file boundaries, test plans.

### Status of 0001 – 0008

The technical RFCs from v0.11.0 are marked **Implemented** in their status
tables.  Code references in those RFCs reflect the shipped v0.11.0 surface.


## [0.11.0] — 2026-05-22

### Added / Fixed — RFC 0001–0008 Implementation

All eight RFCs introduced in v0.10.1 are implemented in this release.

---

#### RFC-0001 — Complete `HistoryMessage::LogCopyRequested`

`log_to_markdown(log: &OperationLog) -> String` added to `view/history.rs`.
Renders a full Markdown document covering: operation kind, timestamps, status
badge, per-project success/failure, commands executed, stdout/stderr excerpts,
and recovery hints.

`LogCopyRequested(id)` handler in `app.rs` now looks up the log entry, generates
the Markdown, and dispatches `Message::CopyToClipboard(text)` — wiring to
`iced::clipboard::write` that was already present but previously unreached.

New i18n keys: `history.copy_ok_prefix`, `history.copy_ok_suffix`,
`history.copy_miss` (en + ja).

---

#### RFC-0002 — `StashEntry.commit_id`

`crate::model::status::StashEntry` gains `commit_id: String` (8-char hex).
Mapped from `endringer_backend_core::types::StashEntry::commit_id` via
`CommitId::short()`.  jj stash entries continue to return an empty Vec
(jj has no stash concept).

---

#### RFC-0003 — jj Conflict Detection (Option B)

`ConflictStatus` gains a new field:

```rust
/// True when the detection mechanism was unavailable (e.g. `jj` absent).
/// UI should show "Unknown" rather than "No conflict."
#[serde(default)]
pub detection_unavailable: bool,
```

New `detect_jj_conflict(path: &str) -> ConflictStatus` helper in `vcs/jj.rs`:
returns `detection_unavailable: true` when the `jj` binary is absent rather
than silently returning `has_conflict: false` (a false negative).

Dashboard card shows "? Conflict detection unavailable" when this flag is set.

Architecture docs updated to list the jj CLI exception explicitly.

---

#### RFC-0004 — Ahead/Behind Counts via gix

`gix_ahead_behind(repo_path: &str) -> RemoteStatus` added to `vcs/git.rs`.
Replaces `read_remote_cli` (which spawned `git rev-list --left-right --count`).

Implementation:
1. Opens the repository with `endringer_backend_git::GitBackend` (gix).
2. Calls `status_digest()` to get the current branch name — no CLI.
3. Resolves the upstream tracking ref name via `git rev-parse --abbrev-ref
   --symbolic-full-name @{u}` — **one remaining CLI call**, only invoked when
   gix confirms a non-detached HEAD.
4. Counts commits with `git rev-list --count <from> ^<exclude>` (lightweight,
   no worktree traversal).

The "no upstream" path (most common for local-only repos) returns early from
the gix step without spawning any process.  `read_remote_cli` removed.

---

#### RFC-0005 — Annotated Tag Support in the Freezer

- `git::tag_create_annotated(project, name, message)` added to `vcs/git.rs`
  using `GitBackend::create_annotated_tag`.
- `VcsAdapter::create_tag_with_message(project, tag_name, message: Option<&str>)`
  added to `vcs/adapter.rs`: routes to `tag_create_annotated` when message is
  non-empty, `tag_create` (lightweight) when empty, jj `bookmark_create`
  regardless.
- `FreezerState.tag_message: String` added (empty = lightweight tag).
- `FreezerMessage::TagMessageChanged(String)` added.
- Freezer view gains a "Tag message" text input below the name field.
- `execute_freeze` dispatches `create_tag_with_message` instead of `create_tag`.
- New i18n keys: `freezer.tag_message_label`, `freezer.tag_message_hint`,
  `freezer.tag_message_jj_note` (en + ja).

---

#### RFC-0006 — jj `log_since` Accurate Range

`jj::log_since` rewritten to use `jj log -r <bookmark>..@` instead of
calling `list_commits()` and discarding the `since_ref`.  Consistent with
the Git implementation which uses `git log <ref>..HEAD`.  Returns an error
entry when the `jj` binary is absent.

---

#### RFC-0007 — Topology Scan Scope Documented (Option A)

Added a note to `docs/src/guide/freezer.md`:

> Dependency scanning reads `Cargo.toml` files only.
> Node.js, Python, Go, and other ecosystems are not scanned.

Architecture docs updated with the full jj CLI exception explanation.

---

#### RFC-0008 — `FsPoller::prune` on Workspace Switch

`FsPoller::prune(active_ids)` now called in:
- `WorkspaceMessage::WorkspaceSwitched` — prunes snapshots for the previous
  workspace's projects before switching.
- `WorkspaceMessage::DeleteWorkspaceConfirmed` — prunes before the workspace
  is removed from the list.

Prevents unbounded growth of the `FsPoller::snapshots` HashMap across
workspace switches.

---

### Changed

- `ConflictStatus` has a new `#[serde(default)]` field `detection_unavailable`.
  Existing serialised values deserialise cleanly (field defaults to `false`).
- `StashEntry` has a new `commit_id: String` field.
  Existing serialised history entries must be re-read; `commit_id` defaults
  to empty string for old entries (no `#[serde(default)]` annotation needed
  as `String` implements `Default`).


## [0.10.1] — 2026-05-05

### Added — `rfcs/` directory

Added a top-level `rfcs/` directory containing implementation specifications
for all open design questions identified in the v0.10.0 design-note review.

| RFC  | Title | Priority |
|------|-------|----------|
| [0001](rfcs/0001-history-log-copy.md) | Complete `HistoryMessage::LogCopyRequested` — generate Markdown from `OperationLog` and write to clipboard via `Message::CopyToClipboard` | **High** |
| [0002](rfcs/0002-stash-entry-commit-id.md) | Add `commit_id: String` to knotra's `StashEntry` domain type to align with `endringer-backend-core` | Medium |
| [0003](rfcs/0003-jj-conflict-detection.md) | jj conflict detection: choose between gix-based disk read vs. documented CLI exception; includes `ConflictStatus::detection_unavailable` design | Medium |
| [0004](rfcs/0004-ahead-behind-gix.md) | Replace `read_remote_cli` with a gix reference-walk; includes spike tasks and `merge_base` pseudocode | Low |
| [0005](rfcs/0005-annotated-tag-freezer.md) | Annotated tag support in the Freezer: `VcsAdapter::create_tag_with_message`, optional message field in Freezer UI | Medium |
| [0006](rfcs/0006-jj-log-since-range.md) | Fix `jj::log_since` to use `jj log -r <bookmark>..@` instead of returning all commits | Medium |
| [0007](rfcs/0007-topology-multi-manifest.md) | Topology scan multi-manifest: document Rust-only scope or add `package.json` / `pyproject.toml` parsers | Low |
| [0008](rfcs/0008-fspoller-prune-on-switch.md) | Call `FsPoller::prune` in `WorkspaceSwitched` handler to release stale snapshots | Low |

Each RFC follows a lightweight template (Summary / Problem / Design / Test Plan /
Security Considerations) and is extended with Requirements, External/Internal
Design, and Alternatives sections where scope warrants it.


## [0.10.0] — 2026-05-04

### Changed — endringer 0.19.2 migration

**Background:** endringer 0.19.2 was published to crates.io with gix updated
from 0.77 to 0.83 — the same version already used by knotra.  
This release replaces the hand-written knotra-internal VCS implementation layer
with the upstream endringer 0.19.2 backends, eliminating all gix-version
compatibility workarounds.

#### New workspace crates

Four upstream crates are now vendored in `crates/` and built as part of the
knotra workspace:

| Crate | Role |
|---|---|
| `endringer-backend-core` | `VcsBackend` trait + all public types |
| `endringer-backend-git` | `GitBackend` — gix-powered, `ThreadSafeRepository` |
| `endringer-backend-jj` | `JjBackend` — gix direct read, no `jj` binary required |
| `endringer-backend-async` | `AsyncRepository` — `spawn_blocking` async façade |

Sources are copied verbatim from endringer 0.19.2 with only crate-name
identifiers substituted (`endringer_core` → `endringer_backend_core`, etc.).

#### Internal endringer VCS layer rewritten

`crates/endringer/src/vcs/git.rs` and `vcs/jj.rs` now delegate **all reads**
to `AsyncRepository` (→ `endringer-backend-async` → gix):

| Operation | Before | After |
|---|---|---|
| Branch / HEAD | `git symbolic-ref` CLI | `AsyncRepository::status_digest` → gix |
| Working tree dirty/staged/untracked | `git status --porcelain` CLI | `AsyncRepository::worktree_status` → gix |
| Branch list | `git branch -a` CLI | `AsyncRepository::local_branches` + `remote_branches` → gix |
| Tag list | `git tag --sort` CLI | `endringer_backend_git::GitBackend::list_tags_sorted` → gix |
| Tag create | `git tag` CLI | `GitBackend::create_tag` → gix |
| Context switch dirty check | `git status` CLI | `AsyncRepository::worktree_status` → gix |
| Freeze validation dirty check | `git status` CLI | `GitBackend::worktree_status` → gix |
| jj status / branch / commits | `jj` CLI | `AsyncRepository::open_jj` → `JjBackend` → gix (no `jj` binary) |

Write operations (fetch, merge, stash, push, abort-merge) continue to use the
`git` / `jj` CLI because gix does not expose write APIs at this level.

#### New public VcsAdapter operations

- `VcsAdapter::stash_entries(project)` → `Vec<StashEntry>` (gix, no CLI)
- `VcsAdapter::worktree_status(project)` → `Option<BackendWorktreeStatus>` — full
  per-file staged/unstaged/untracked detail, gitignore-aware

#### jj binary dependency eliminated

`JjBackend::open` reads jj's underlying git object store directly with gix.
The `jj` binary is no longer required for read operations on jj repositories
(conflict detection still uses `jj log` CLI).

#### gix features updated

`Cargo.toml` workspace gix entry now includes:
- `blame` and `attributes` (required by endringer-backend-git)
- `parallel` — **required** for `ThreadSafeRepository` to implement `Send + Sync`.  
  Without `parallel`, gix 0.84 uses `Rc`-based internal pools which are not
  thread-safe. With `parallel`, pools switch to `Arc`, making
  `ThreadSafeRepository: Send + Sync`.

#### Removed

- `vcs/git.rs`: `gix_read_head`, `gix_read_working_tree` (Phase 9 hot-path
  stubs) — superseded by the full `endringer-backend-git` implementation.
- Direct `gix` dependency in `crates/endringer/Cargo.toml`.
- Mutex/unsafe-Send workarounds that were required during the aborted
  0.19.1 migration attempt.

### Fixed

- `log_since` uses `git log <ref>..HEAD` CLI range (not timestamp-based).
  The previous timestamp approach was unreliable when commits are created
  within the same second (e.g. in tests and CI).


## [0.9.1] — 2025-xx-xx

### Changed — Boundary Enforcement (endringer / knotra-app separation)

**Background:** The Phase 9 review identified three places where the VCS implementation details of `endringer` were leaking through its public surface.

**1. `gix_read_head` / `gix_read_working_tree` restricted to `pub(crate)`**

These functions are internal hot-path optimisations. Nothing outside `endringer` should call them directly. Callers always go through `VcsAdapter::read_project_status`.

**2. `vcs::git` and `vcs::jj` modules restricted to `pub(crate)`**

Previously `pub`, meaning any downstream crate could call `endringer::vcs::git::tag_create(...)` directly. Now only `VcsAdapter` (within the same crate) can access these modules.

**3. `VcsAdapter::create_tag`, `VcsAdapter::delete_tag`, `VcsAdapter::log_since` added**

These operations existed in `vcs::git` but had no `VcsAdapter` entry-point, forcing callers (including integration tests) to bypass the public API. They are now first-class operations on `VcsAdapter`, dispatching to Git or jj as appropriate.

| New method | Git | jj |
|---|---|---|
| `VcsAdapter::create_tag(project, name)` | `git tag <name>` | `jj bookmark create <name> -r @` |
| `VcsAdapter::delete_tag(project, name)` | `git tag -d <name>` | `jj bookmark delete <name>` |
| `VcsAdapter::log_since(project, since, until)` | `git log <since>..<until>` | `jj log -r <since>..@` |

**4. Integration tests updated to use `VcsAdapter` exclusively**

The three `endringer::vcs::git::*` direct calls in `git_integration.rs` are replaced with the new `VcsAdapter` methods. The integration test file now has zero imports of `endringer::vcs`.

**Result:** The public surface of `endringer` is now exactly: `VcsAdapter`, the `model::*` types, `FsPoller`, and `EndringerError`. No VCS-specific internals (`git`, `jj`, `gix`) are reachable from outside the crate.


## [0.9.0] — 2025-xx-xx

### Added
- Phase 9: Code Quality, Integration Tests & gix Hot-path.

**Integration test suite (`crates/endringer/tests/git_integration.rs`) — all §16.4 states:**

| State | Test | What is verified |
|---|---|---|
| Clean | `clean_repo_reports_synced` | No error, no dirty, no conflict, context present |
| Uncommitted | `repo_with_uncommitted_file_is_dirty` | `uncommitted_count > 0` |
| Untracked | `repo_with_untracked_file_shows_untracked_count` | `untracked_count > 0` |
| Ahead | `ahead_repo_shows_nonzero_ahead_count` | `remote.ahead == 1` |
| Behind | `behind_repo_shows_nonzero_behind_count` | `remote.behind > 0` |
| Ahead + Behind | `ahead_and_behind_repo` | Both ahead and behind > 0 |
| Conflict | `conflict_repo_shows_has_conflict` | `conflict.has_conflict == true` |
| Tag created | `tag_created_blocks_freeze_validation` + `tag_create_and_delete_roundtrip` | Tag existence blocking, create/delete cycle |
| Permission-error | `nonexistent_path_returns_read_error` | `read_error` set for missing path |
| jj project | `jj_repo_uses_jujutsu_vcs_kind` | VcsKind::Jujutsu detected (skipped if jj absent) |
| Repo exists | `repo_exists_returns_true/false` | `VcsAdapter::repo_exists` works correctly |
| List contexts | `list_contexts_returns_current_branch`, `includes_second_branch` | Branch list and current detection |
| Changelog | `log_since_collects_commits_since_tag` | 2 commits collected, subjects match |
| Freeze: clean | `clean_repo_passes_freeze_validation` | `all_ready()` true, no blockers |
| Freeze: dirty | `dirty_repo_blocks_freeze_validation` | Blockers non-empty |
| Context switch | `switch_context_changes_branch` | Branch changed, confirmed via `symbolic-ref` |
| Missing path | `repo_exists_returns_false_for_missing_path` | Returns false |
| (+ 2 more) | `ahead_and_behind_repo`, `tag_create_and_delete_roundtrip` | — |

19 integration tests. All use real `git` processes in tempdir sandboxes.

**Compiler warning elimination (0 warnings):**
- Added `#[allow(dead_code)]` to all message enums, state FSM enums, and impl blocks with intentionally forward-facing variants.
- Removed 12 unused import statements across 10 files.
- Prefixed 8 unused variable bindings with `_`.
- Removed duplicate `WorkspaceSwitched` match arm (unreachable pattern).
- Removed dead `view_validation_entry` function (superseded by `view_validation_entry_owned`).
- Removed dead `FsChangeMessage` warn by annotating with `#[allow(dead_code)]`.
- Fixed `mut` / `unused_mut` in freezer view.

**gix hot-path reads (Phase 1 deferred work):**
- `endringer/vcs/git.rs`: `gix_read_head(repo_path)` — opens repository with `gix::open`, reads `head.kind` for branch name and detached-HEAD state. Zero process spawns.
- `endringer/vcs/git.rs`: `gix_read_working_tree(repo_path)` — uses `gix` status iterator to count `Modification` (uncommitted) and `Untracked` entries. Zero process spawns.
- `read_blocking` now tries gix first for both reads, falls back to CLI on any gix error.
- Net effect: status reads for Git repositories now avoid two `std::process::Command` spawns for the most common path (clean or lightly dirty repositories).


## [0.8.0] — 2025-xx-xx

### Added
- Phase 8: File-system Event Monitoring + Clipboard Integration.

**File-system Event Monitoring (`endringer::watcher`):**
- `endringer::watcher::FsPoller` — polling-based sentinel-file watcher.
  - Watches `.git/HEAD`, `.git/index`, `.git/refs/` for Git; `.jj/working_copy/`, `.jj/op_heads/` for jj.
  - `poll(projects)` → `Vec<FsChangeEvent>`: first call establishes baseline (no events); subsequent calls emit one `FsChangeEvent` per changed repository.
  - `prune(active_ids)` removes snapshots for deregistered projects.
  - Handles worktree repos (`gitdir:` file pointer).
- `AppState.fs_poller: FsPoller` — persists across ticks in application state.
- `Message::FsWatchTick` — emitted by `fs_watch_subscription` at the configured interval.
- `fs_watcher::fs_watch_subscription(state)` — returns `Subscription::none()` when disabled, otherwise `time::every(debounce_secs)`.
- `handle_fs_watch_tick` — on change: refreshes affected projects individually (≤3 changes) or triggers a full workspace refresh (>3 changes).
- Config fields: `fs_watch_enabled: bool` (default `false`), `fs_debounce_secs: u32` (default 2).
- Settings screen: toggle button and debounce-interval input under new "File-system Monitoring" section.
- 4 new unit tests: baseline-no-event, no-change-no-event, modified-sentinel-triggers-event, prune.

**Clipboard Integration:**
- `Message::CopyToClipboard(String)` — routes directly to `iced::clipboard::write`, providing true system clipboard access.
- History screen: **Copy** button now builds a formatted text block (kind, timestamp, status, commands, stderr) and writes it to the clipboard.
- Changelog screen: **Copy Markdown** button writes the full generated Markdown to the clipboard.
- `ChangelogMessage::CopyRequested` handler returns `clipboard::write(md)` as a `Task`.
- Settings: **Topology Scan** button added for convenience (same as the Freezer button).

**Settings screen additions:**
- File-system Monitoring section: enable/disable toggle + debounce interval input.
- Dependency Topology section: Scan Dependencies button + scan-phase status label.
- `SettingsMessage::FsWatchEnabledChanged(bool)`, `SettingsMessage::FsDebounceSecs(u32)`.
- `SettingsEdit.fs_debounce_secs` field.

**i18n additions (minimal):** FS watch section labels are inline in the settings view (English only; i18n pass in a future phase).

### Changed
- `AppConfig` extended with `fs_watch_enabled` and `fs_debounce_secs`.
- `app::subscription` now batches tick, keyboard, and FS-watch subscriptions.
- History `LogCopyRequested` now emits `Message::CopyToClipboard` with full formatted log text.
- Changelog copy now uses real clipboard write, not a status-bar placeholder.


## [0.14.0] — 2026-05-24

### Changed — Less is more: UI simplification

Applied the "less is more" design principle across the entire UI.  No features
were removed; information was moved to the right level of the information
hierarchy.

#### Cards: 15 data points → 2–3

Cards now use tier-based density rather than showing everything on every card.

| Tier | Before | After |
|------|--------|-------|
| Needs Attention | name · VCS badge · status label · branch · ↑↓●? counters · timestamp · Fetch · Remove | name — problem · **one action** |
| Active | same 15 items | name · branch |
| Clean | same 15 items | name only |

`vcs_label`, `status_label` (beyond problem text), `stat_cell` counters, the
`last_updated` timestamp, and the per-card **Fetch** / **Remove** buttons are
no longer on cards.  All this information remains available in the detail panel
(click the project name).

#### Dashboard header: 5 items → 2

Before: workspace name · last-updated time · Add project · Refresh · Bulk sync  
After: workspace name · ⟳ refresh icon

Add project lives in the empty state only.  Bulk sync is accessible via the
selection bar.  Last-updated moves to the detail panel.

#### Selection: always-on checkboxes → explicit mode

Checkboxes are now hidden by default.  Selecting any project (keyboard or
click inside selection mode) enters **selection mode**, which shows checkboxes
and the selection bar.  Exiting via "Exit selection" hides them again.

This keeps clean cards truly clean.  A new user never sees a checkbox until
they need to bulk-select.

#### Navigation: 2 rows → 1

The nav bar added in v0.13.0 (Dashboard / History / Settings row) has been
**removed** — it was adding chrome without removing friction.

Navigation is now:
- **Workspace tab strip** (top): workspace tabs + `+` new workspace + `⊟` History + `⚙` Settings
- **Command palette** (⌘K): every action by name

#### Empty state: clear call to action

When a workspace has no projects, the dashboard shows a centred prompt:

```
No projects yet.
Add a project folder to get started.

    [ + Add project ]
```

The add button is prominent when it matters and invisible otherwise.

#### snora: nav_menu module added

`snora::nav_menu` provides `nav_bar()` and `NavItem` for horizontal
navigation menus.  Not used in the main layout (which no longer has a nav
bar) but available for future use in settings screens or onboarding flows.

### Tests

- endringer unit: 17 pass.
- endringer integration: 19 pass.
- knotra-app: `cargo check` clean — 0 errors, 0 warnings.


## [0.13.0] — 2026-05-24

### Added — Navigation menu, Add Project modal, archive structure fix

#### Navigation menu (`view/nav_bar.rs`, `snora::nav_menu`)

A horizontal navigation bar is now rendered below the workspace tabs on every
screen.  It replaces the old sidebar navigation with a leaner, always-visible
strip:

```
[Dashboard]  [History]  [Settings]          [+ Add project]
```

- **Left side**: tab-style nav items from `snora::nav_menu::nav_bar()`.
  The active screen is indicated by a `•` prefix.  Clicking a non-active
  item navigates immediately.
- **Right side**: **Add project** button — moved here from the bottom-left
  dashboard toolbar where it was easy to miss.

New in `snora`: `nav_menu` module with `NavItem`, `nav_bar()`, and the
`NAV_BAR_HEIGHT` constant.  Consumers call `nav_bar(items)` passing a
`Vec<NavItem>` — labels, active flag, and dispatch message — and receive
a full-width `Element`.

#### Add Project modal (`view/add_project_modal.rs`)

The dialog is now a **centred stack overlay** (same layer as the bulk
action modals) instead of being appended below the dashboard content.

The path field gains a **Browse…** button that opens a native OS folder
picker via `rfd::AsyncFileDialog`:

- Folder picker opens without blocking the UI thread (async task).
- On selection, the path field is populated.
- If the project name field is still empty, it is auto-filled from the
  folder's last path component.
- Dismissing the picker (Cancel) leaves both fields unchanged.

`WorkspaceMessage::BrowsePathRequested` and
`WorkspaceMessage::BrowsePathSelected(Option<String>)` are the two new
message variants.

#### Archive structure fix

Release archives now contain `knotra-vX.Y.Z/(files)` directly, without
an intermediate `knotra/` subdirectory.  The packaging command uses
`tar --transform` to rename the root directory at archive time.

Before: `knotra-v0.12.1.tar.gz` → `knotra/Cargo.toml`  
After:  `knotra-v0.13.0.tar.gz` → `knotra-v0.13.0/Cargo.toml`

### Changed

- `snora`: new public `nav_menu` module; `nav_bar`, `NavItem`,
  `NAV_BAR_HEIGHT` exported from `snora` root.
- `view/mod.rs`: `app_view` now includes the nav bar in every layout;
  `add_project_modal` overlay inserted into the stack layers.
- `view/dashboard.rs`: removed the old inline dialog append; removed the
  `add_btn` from the dashboard toolbar row (it lives in the nav bar now).
- `view/add_project_dialog` (old private fn): replaced by
  `view/add_project_modal.rs` (public module, centered overlay).
- `i18n`: new key `dialog.add_project.browse` (en + ja).

### Tests

- endringer unit: 17 pass.
- endringer integration: 19 pass.
- knotra-app: `cargo check` clean — 0 errors, 0 warnings.


## [0.12.1] — 2026-05-23

### Changed — RFC directory restructured to follow lifecycle policy

Documentation-only release.  No code changes.

`rfcs/` has been reorganised from a flat directory into a four-folder
layout following [RFC 000 — RFC lifecycle policy](rfcs/done/000-rfc-lifecycle-policy.md).

#### New structure

```
rfcs/
  README.md           ← state-grouped index (rebuilt)
  proposed/           ← RFCs open for review (do not implement yet)
  done/               ← implemented RFCs; historical record
  archive/            ← withdrawn or superseded RFCs
```

#### Moves

| File | From | To | Reason |
|------|------|----|--------|
| `000-rfc-lifecycle-policy.md` | (project docs) | `done/` | Policy is in effect; self-placed per §Self-application |
| `0001`–`0016` | flat `rfcs/` | `done/` | All implemented (v0.11.0 / v0.12.0) |
| `0017-screen-removal.md` | flat `rfcs/` | `proposed/` | Not yet shipped; targeted at v0.16 |

#### Status field updates

All RFC files in `done/` now carry `Implemented (vX.Y.Z)` in their
Status field, matching their folder.  `0017` retains `Proposed`.


## [0.12.0] — 2026-05-23

### Added — UI/UX Redesign: RFC 0009–0016

All eight code RFCs from the UI/UX redesign are implemented.  No screens are
removed in this release (that is RFC-0017, targeted at v0.16); all existing
navigation continues to work alongside the new panels and modals.

#### RFC-0009 — Selection model

`SelectionState` added to `AppState`.  Every dashboard card now has a
checkbox.  Clicking the checkbox (or pressing `Space` on a focused card)
toggles it.  A sticky selection bar slides up from the bottom of the window
whenever ≥ 1 project is selected, showing the count and primary actions:
**Fetch**, **Pull…**, **Tag…**, **Switch…**.  Keyboard: `Space` toggle,
`Ctrl+A` / `⌘A` select all, `Esc` clear.

New state: `state::SelectionState` with `toggle`, `select_range`,
`select_all`, `clear`.  
New message enum: `SelectionMessage` (Toggled, RangeTo, SelectAll, Clear,
FocusMoved).  
New view: `view/selection_bar.rs`.

---

#### RFC-0010 — Three-tier attention grouping

`state/tier.rs` introduces `compute_tier(status, path_exists) →
(AttentionTier, Option<AttentionCause>)`, which maps any `ProjectStatus` to
one of:

- 🔴 **NeedsAttention** — conflict, detection-unavailable, detached HEAD,
  path missing, or a read error.
- 🟡 **Active** — uncommitted changes, ahead/behind upstream, or
  non-default branch.
- ⚪ **Clean** — synced, default branch, working tree clean.

Dashboard uses `view_tier_grid` when `AppState::grouping_mode ==
GroupingMode::Auto` (default).  Each tier has a collapsible header with
a count badge.  The Clean tier is collapsed by default.  Legacy filter-chip
grouping is still available by switching `GroupingMode::Legacy` via the tier
message handler.

New state: `AttentionTier`, `GroupingMode`, `TierCollapseState`,
`AttentionCause` (in `state/tier.rs`).  
New message enum: `TierMessage` (Toggled, GroupingModeChanged).

---

#### RFC-0011 — Activity strip

A single-line status bar at the very bottom of the window.  Hidden when idle.
Transitions through four states:

- **Running** — `⟳ Fetching… 3/12` with done/total counter.
- **Success** — `ⓘ Fetched 12 projects`.
- **PartialFailure** — `⚠ … failed: project-alpha` with a **Retry** button.
- **TotalFailure** — `✗ …` with a **Retry** button.

`›` button opens the history popover.

New state: `LatestOpState`, `ActivityStripState` on `AppState`.  
New message enum: `ActivityMessage` (Started, Progress, Completed,
PopoverToggled, RetryRequested, Tick).  
New view: `view/activity_strip.rs`.

---

#### RFC-0012 — Command palette

`⌘K` / `Ctrl+K` opens a centered floating input.  Typing runs
case-insensitive substring search across:

- **Actions** — 16 built-in entries (Fetch all, Pull selected, Tag selected,
  Switch branch, Generate changelog, Add project, Create workspace, Select
  all, Clear selection, Open Settings, Open History, Refresh, Show shortcuts…).
- **Projects** — all projects in the current workspace.
- **Workspaces** — all workspaces.

Selecting an entry dispatches its corresponding `Message`.  `Esc` closes.
Up/Down arrows navigate; `Enter` confirms.

New state: `PaletteState`, `PaletteEntry`, `PaletteEntryKind` on `AppState`.  
New logic: `state/palette.rs` with `update_results` and `dispatch_entry`.  
New message enum: `PaletteMessage`.  
New view: `view/command_palette.rs`.

---

#### RFC-0013 — Bulk action modals

Five workflow modals rendered over the dashboard via `iced::widget::stack`:

| Modal | Trigger | State path |
|-------|---------|-----------|
| **Pull** | Selection bar `Pull…` | `state::sync` |
| **Tag** | Selection bar `Tag…` | `state::freezer` |
| **Switch branch** | Selection bar `Switch…` | `state::context_ops` |
| **Resolve** | Card `Resolve…` button | `state::conflict_ops` |
| **Changelog** | Command palette | `state::changelog` |

Each modal is pre-populated from the dashboard selection.  The existing
state machines (`SyncPhase`, `FreezerPhase`, `ChangelogPhase`, etc.) are
reused unchanged; only the view layer changed.

New state: `ActiveModal` enum on `AppState`.  
New view: `view/bulk_modals.rs` (pull_modal, tag_modal, switch_modal,
resolve_panel, changelog_modal).  
New `app_view` in `view/mod.rs` stacks modals over the base layout.

---

#### RFC-0014 — Project detail side panel

Clicking a project **name** (not the checkbox) opens a right-docked 300 px
panel showing:

- **Identity** — VCS, path, remote upstream.
- **Status** — branch, ahead, behind, dirty count, untracked, conflict flag.
- **Recent operations** — last 5 ops touching this project (icon + kind +
  timestamp).
- **Actions** — Refresh, Fetch, Remove from workspace.

Panel is semi-modal: the main view remains interactive while it is open.

New state: `DetailPanelState` on `AppState`.  
New message enum: `DetailPanelMessage` (Opened, Closed).  
New view: `view/detail_panel.rs`.

---

#### RFC-0015 — Workspace tabs

A horizontal tab strip replaces the sidebar workspace list at the top of
every screen.  Each tab shows the workspace name and a parenthetical count
of **Needs Attention** projects (e.g. `work (3)`).  The active workspace
tab has its button's `on_press` disabled.

`⌘1` – `⌘9` / `Ctrl+1` – `Ctrl+9` switches to workspace by index.
A `+` button at the end opens the create-workspace dialog.

New view: `view/workspace_tabs.rs`.  
Keyboard handling updated in the `subscription` function.

---

#### RFC-0016 — Keyboard shortcuts and cheat sheet

`?` toggles a centred overlay listing all 17 documented key bindings across
five contexts (Global, Dashboard, Selection, Palette, Modal).

Leader-key state machine: pressing `g` sets `LeaderKeyState::G`; a
subsequent `h` navigates to History, `s` to Settings.

New state: `KeyboardState` (cheat_sheet_open, leader: `LeaderKeyState`).  
New message enum: `KeyboardMessage` (CheatSheetToggled, LeaderGPressed,
LeaderCancelled).  
New view: `view/shortcuts_overlay.rs`.

---

#### RFC-0017 — Screen removal

**Deferred to v0.16.**  All five screens (Sync Center, Freezer, ContextOps,
ConflictResolution, Changelog) remain accessible via sidebar.  The modals
introduced in RFC-0013 provide the preferred workflow path for v0.12–v0.15.

---

### Changed

- `AppState` gains 8 new fields: `selection`, `activity`, `palette`,
  `grouping_mode`, `tier_collapse`, `keyboard`, `detail_panel`,
  `active_modal`.
- Dashboard view switches between legacy grid and tier grid based on
  `grouping_mode` (default: Auto / tier grid).
- `app_view` now renders workspace tabs at top, selection bar and activity
  strip at bottom, and modal/palette/cheat-sheet overlays via
  `iced::widget::stack`.
- Clicking a project name now opens the detail panel rather than doing nothing.
- `SyncCenterState` gains `selected_project_ids: HashSet<ProjectId>`.
- `ContextOpsState` gains `target_context: String`.
- `ChangelogState` gains `since_ref: String`.

### Tests

- **endringer unit:** 17 pass.
- **endringer integration:** 19 pass.
- **knotra-app:** `cargo check` clean, 0 errors, 0 warnings.


## [0.11.1] — 2026-05-23

### Added — UI/UX Redesign RFCs (0009 – 0017)

This is a **documentation-only release**.  No code changes; nine detailed
RFCs for the v0.12 → v0.16 UI/UX redesign are added under `rfcs/`.  Together
they describe the migration from screen-based navigation to a single-view,
selection-driven dashboard.

| RFC  | Title                                       | Target | Effort       |
|------|---------------------------------------------|--------|--------------|
| [0009](rfcs/0009-selection-model.md)        | Selection model and selection bar          | v0.12 | Medium        |
| [0010](rfcs/0010-attention-tiers.md)        | Three-tier attention grouping (Needs Attention / Active / Clean) | v0.13 | Medium |
| [0011](rfcs/0011-activity-strip.md)         | Activity strip at bottom of window         | v0.12 | Small–Medium  |
| [0012](rfcs/0012-command-palette.md)        | Command palette (⌘K) with fuzzy search     | v0.12 stub / v0.13 full | Medium |
| [0013](rfcs/0013-bulk-action-modals.md)     | Bulk action modals replacing 5 screens     | v0.14 | **Large**     |
| [0014](rfcs/0014-project-detail-panel.md)   | Right-docked project detail side panel     | v0.15 | Medium        |
| [0015](rfcs/0015-workspace-tabs.md)         | Workspace tabs at top + ⌘1/⌘2 shortcuts    | v0.15 | Small–Medium  |
| [0016](rfcs/0016-keyboard-shortcuts.md)     | Keyboard shortcuts table + `?` cheat sheet | v0.13 | Medium        |
| [0017](rfcs/0017-screen-removal.md)         | Removal of Sync Center / Freezer / ContextOps / Conflict Resolution / Changelog screens | v0.16 | Small–Medium |

### Design Source

The redesign rationale and IA decisions are documented in
[`docs/src/contributing/ui-ux-redesign.md`](docs/src/contributing/ui-ux-redesign.md).
Each RFC carries forward the relevant design decisions with implementation-level
detail — state shape, message variants, file boundaries, test plans.

### Status of 0001 – 0008

The technical RFCs from v0.11.0 are marked **Implemented** in their status
tables.  Code references in those RFCs reflect the shipped v0.11.0 surface.


## [0.11.0] — 2026-05-22

### Added / Fixed — RFC 0001–0008 Implementation

All eight RFCs introduced in v0.10.1 are implemented in this release.

---

#### RFC-0001 — Complete `HistoryMessage::LogCopyRequested`

`log_to_markdown(log: &OperationLog) -> String` added to `view/history.rs`.
Renders a full Markdown document covering: operation kind, timestamps, status
badge, per-project success/failure, commands executed, stdout/stderr excerpts,
and recovery hints.

`LogCopyRequested(id)` handler in `app.rs` now looks up the log entry, generates
the Markdown, and dispatches `Message::CopyToClipboard(text)` — wiring to
`iced::clipboard::write` that was already present but previously unreached.

New i18n keys: `history.copy_ok_prefix`, `history.copy_ok_suffix`,
`history.copy_miss` (en + ja).

---

#### RFC-0002 — `StashEntry.commit_id`

`crate::model::status::StashEntry` gains `commit_id: String` (8-char hex).
Mapped from `endringer_backend_core::types::StashEntry::commit_id` via
`CommitId::short()`.  jj stash entries continue to return an empty Vec
(jj has no stash concept).

---

#### RFC-0003 — jj Conflict Detection (Option B)

`ConflictStatus` gains a new field:

```rust
/// True when the detection mechanism was unavailable (e.g. `jj` absent).
/// UI should show "Unknown" rather than "No conflict."
#[serde(default)]
pub detection_unavailable: bool,
```

New `detect_jj_conflict(path: &str) -> ConflictStatus` helper in `vcs/jj.rs`:
returns `detection_unavailable: true` when the `jj` binary is absent rather
than silently returning `has_conflict: false` (a false negative).

Dashboard card shows "? Conflict detection unavailable" when this flag is set.

Architecture docs updated to list the jj CLI exception explicitly.

---

#### RFC-0004 — Ahead/Behind Counts via gix

`gix_ahead_behind(repo_path: &str) -> RemoteStatus` added to `vcs/git.rs`.
Replaces `read_remote_cli` (which spawned `git rev-list --left-right --count`).

Implementation:
1. Opens the repository with `endringer_backend_git::GitBackend` (gix).
2. Calls `status_digest()` to get the current branch name — no CLI.
3. Resolves the upstream tracking ref name via `git rev-parse --abbrev-ref
   --symbolic-full-name @{u}` — **one remaining CLI call**, only invoked when
   gix confirms a non-detached HEAD.
4. Counts commits with `git rev-list --count <from> ^<exclude>` (lightweight,
   no worktree traversal).

The "no upstream" path (most common for local-only repos) returns early from
the gix step without spawning any process.  `read_remote_cli` removed.

---

#### RFC-0005 — Annotated Tag Support in the Freezer

- `git::tag_create_annotated(project, name, message)` added to `vcs/git.rs`
  using `GitBackend::create_annotated_tag`.
- `VcsAdapter::create_tag_with_message(project, tag_name, message: Option<&str>)`
  added to `vcs/adapter.rs`: routes to `tag_create_annotated` when message is
  non-empty, `tag_create` (lightweight) when empty, jj `bookmark_create`
  regardless.
- `FreezerState.tag_message: String` added (empty = lightweight tag).
- `FreezerMessage::TagMessageChanged(String)` added.
- Freezer view gains a "Tag message" text input below the name field.
- `execute_freeze` dispatches `create_tag_with_message` instead of `create_tag`.
- New i18n keys: `freezer.tag_message_label`, `freezer.tag_message_hint`,
  `freezer.tag_message_jj_note` (en + ja).

---

#### RFC-0006 — jj `log_since` Accurate Range

`jj::log_since` rewritten to use `jj log -r <bookmark>..@` instead of
calling `list_commits()` and discarding the `since_ref`.  Consistent with
the Git implementation which uses `git log <ref>..HEAD`.  Returns an error
entry when the `jj` binary is absent.

---

#### RFC-0007 — Topology Scan Scope Documented (Option A)

Added a note to `docs/src/guide/freezer.md`:

> Dependency scanning reads `Cargo.toml` files only.
> Node.js, Python, Go, and other ecosystems are not scanned.

Architecture docs updated with the full jj CLI exception explanation.

---

#### RFC-0008 — `FsPoller::prune` on Workspace Switch

`FsPoller::prune(active_ids)` now called in:
- `WorkspaceMessage::WorkspaceSwitched` — prunes snapshots for the previous
  workspace's projects before switching.
- `WorkspaceMessage::DeleteWorkspaceConfirmed` — prunes before the workspace
  is removed from the list.

Prevents unbounded growth of the `FsPoller::snapshots` HashMap across
workspace switches.

---

### Changed

- `ConflictStatus` has a new `#[serde(default)]` field `detection_unavailable`.
  Existing serialised values deserialise cleanly (field defaults to `false`).
- `StashEntry` has a new `commit_id: String` field.
  Existing serialised history entries must be re-read; `commit_id` defaults
  to empty string for old entries (no `#[serde(default)]` annotation needed
  as `String` implements `Default`).


## [0.10.1] — 2026-05-05

### Added — `rfcs/` directory

Added a top-level `rfcs/` directory containing implementation specifications
for all open design questions identified in the v0.10.0 design-note review.

| RFC  | Title | Priority |
|------|-------|----------|
| [0001](rfcs/0001-history-log-copy.md) | Complete `HistoryMessage::LogCopyRequested` — generate Markdown from `OperationLog` and write to clipboard via `Message::CopyToClipboard` | **High** |
| [0002](rfcs/0002-stash-entry-commit-id.md) | Add `commit_id: String` to knotra's `StashEntry` domain type to align with `endringer-backend-core` | Medium |
| [0003](rfcs/0003-jj-conflict-detection.md) | jj conflict detection: choose between gix-based disk read vs. documented CLI exception; includes `ConflictStatus::detection_unavailable` design | Medium |
| [0004](rfcs/0004-ahead-behind-gix.md) | Replace `read_remote_cli` with a gix reference-walk; includes spike tasks and `merge_base` pseudocode | Low |
| [0005](rfcs/0005-annotated-tag-freezer.md) | Annotated tag support in the Freezer: `VcsAdapter::create_tag_with_message`, optional message field in Freezer UI | Medium |
| [0006](rfcs/0006-jj-log-since-range.md) | Fix `jj::log_since` to use `jj log -r <bookmark>..@` instead of returning all commits | Medium |
| [0007](rfcs/0007-topology-multi-manifest.md) | Topology scan multi-manifest: document Rust-only scope or add `package.json` / `pyproject.toml` parsers | Low |
| [0008](rfcs/0008-fspoller-prune-on-switch.md) | Call `FsPoller::prune` in `WorkspaceSwitched` handler to release stale snapshots | Low |

Each RFC follows a lightweight template (Summary / Problem / Design / Test Plan /
Security Considerations) and is extended with Requirements, External/Internal
Design, and Alternatives sections where scope warrants it.


## [0.10.0] — 2026-05-04

### Changed — endringer 0.19.2 migration

**Background:** endringer 0.19.2 was published to crates.io with gix updated
from 0.77 to 0.83 — the same version already used by knotra.  
This release replaces the hand-written knotra-internal VCS implementation layer
with the upstream endringer 0.19.2 backends, eliminating all gix-version
compatibility workarounds.

#### New workspace crates

Four upstream crates are now vendored in `crates/` and built as part of the
knotra workspace:

| Crate | Role |
|---|---|
| `endringer-backend-core` | `VcsBackend` trait + all public types |
| `endringer-backend-git` | `GitBackend` — gix-powered, `ThreadSafeRepository` |
| `endringer-backend-jj` | `JjBackend` — gix direct read, no `jj` binary required |
| `endringer-backend-async` | `AsyncRepository` — `spawn_blocking` async façade |

Sources are copied verbatim from endringer 0.19.2 with only crate-name
identifiers substituted (`endringer_core` → `endringer_backend_core`, etc.).

#### Internal endringer VCS layer rewritten

`crates/endringer/src/vcs/git.rs` and `vcs/jj.rs` now delegate **all reads**
to `AsyncRepository` (→ `endringer-backend-async` → gix):

| Operation | Before | After |
|---|---|---|
| Branch / HEAD | `git symbolic-ref` CLI | `AsyncRepository::status_digest` → gix |
| Working tree dirty/staged/untracked | `git status --porcelain` CLI | `AsyncRepository::worktree_status` → gix |
| Branch list | `git branch -a` CLI | `AsyncRepository::local_branches` + `remote_branches` → gix |
| Tag list | `git tag --sort` CLI | `endringer_backend_git::GitBackend::list_tags_sorted` → gix |
| Tag create | `git tag` CLI | `GitBackend::create_tag` → gix |
| Context switch dirty check | `git status` CLI | `AsyncRepository::worktree_status` → gix |
| Freeze validation dirty check | `git status` CLI | `GitBackend::worktree_status` → gix |
| jj status / branch / commits | `jj` CLI | `AsyncRepository::open_jj` → `JjBackend` → gix (no `jj` binary) |

Write operations (fetch, merge, stash, push, abort-merge) continue to use the
`git` / `jj` CLI because gix does not expose write APIs at this level.

#### New public VcsAdapter operations

- `VcsAdapter::stash_entries(project)` → `Vec<StashEntry>` (gix, no CLI)
- `VcsAdapter::worktree_status(project)` → `Option<BackendWorktreeStatus>` — full
  per-file staged/unstaged/untracked detail, gitignore-aware

#### jj binary dependency eliminated

`JjBackend::open` reads jj's underlying git object store directly with gix.
The `jj` binary is no longer required for read operations on jj repositories
(conflict detection still uses `jj log` CLI).

#### gix features updated

`Cargo.toml` workspace gix entry now includes:
- `blame` and `attributes` (required by endringer-backend-git)
- `parallel` — **required** for `ThreadSafeRepository` to implement `Send + Sync`.  
  Without `parallel`, gix 0.84 uses `Rc`-based internal pools which are not
  thread-safe. With `parallel`, pools switch to `Arc`, making
  `ThreadSafeRepository: Send + Sync`.

#### Removed

- `vcs/git.rs`: `gix_read_head`, `gix_read_working_tree` (Phase 9 hot-path
  stubs) — superseded by the full `endringer-backend-git` implementation.
- Direct `gix` dependency in `crates/endringer/Cargo.toml`.
- Mutex/unsafe-Send workarounds that were required during the aborted
  0.19.1 migration attempt.

### Fixed

- `log_since` uses `git log <ref>..HEAD` CLI range (not timestamp-based).
  The previous timestamp approach was unreliable when commits are created
  within the same second (e.g. in tests and CI).


## [0.9.1] — 2025-xx-xx

### Changed — Boundary Enforcement (endringer / knotra-app separation)

**Background:** The Phase 9 review identified three places where the VCS implementation details of `endringer` were leaking through its public surface.

**1. `gix_read_head` / `gix_read_working_tree` restricted to `pub(crate)`**

These functions are internal hot-path optimisations. Nothing outside `endringer` should call them directly. Callers always go through `VcsAdapter::read_project_status`.

**2. `vcs::git` and `vcs::jj` modules restricted to `pub(crate)`**

Previously `pub`, meaning any downstream crate could call `endringer::vcs::git::tag_create(...)` directly. Now only `VcsAdapter` (within the same crate) can access these modules.

**3. `VcsAdapter::create_tag`, `VcsAdapter::delete_tag`, `VcsAdapter::log_since` added**

These operations existed in `vcs::git` but had no `VcsAdapter` entry-point, forcing callers (including integration tests) to bypass the public API. They are now first-class operations on `VcsAdapter`, dispatching to Git or jj as appropriate.

| New method | Git | jj |
|---|---|---|
| `VcsAdapter::create_tag(project, name)` | `git tag <name>` | `jj bookmark create <name> -r @` |
| `VcsAdapter::delete_tag(project, name)` | `git tag -d <name>` | `jj bookmark delete <name>` |
| `VcsAdapter::log_since(project, since, until)` | `git log <since>..<until>` | `jj log -r <since>..@` |

**4. Integration tests updated to use `VcsAdapter` exclusively**

The three `endringer::vcs::git::*` direct calls in `git_integration.rs` are replaced with the new `VcsAdapter` methods. The integration test file now has zero imports of `endringer::vcs`.

**Result:** The public surface of `endringer` is now exactly: `VcsAdapter`, the `model::*` types, `FsPoller`, and `EndringerError`. No VCS-specific internals (`git`, `jj`, `gix`) are reachable from outside the crate.


## [0.9.0] — 2025-xx-xx

### Added
- Phase 9: Code Quality, Integration Tests & gix Hot-path.

**Integration test suite (`crates/endringer/tests/git_integration.rs`) — all §16.4 states:**

| State | Test | What is verified |
|---|---|---|
| Clean | `clean_repo_reports_synced` | No error, no dirty, no conflict, context present |
| Uncommitted | `repo_with_uncommitted_file_is_dirty` | `uncommitted_count > 0` |
| Untracked | `repo_with_untracked_file_shows_untracked_count` | `untracked_count > 0` |
| Ahead | `ahead_repo_shows_nonzero_ahead_count` | `remote.ahead == 1` |
| Behind | `behind_repo_shows_nonzero_behind_count` | `remote.behind > 0` |
| Ahead + Behind | `ahead_and_behind_repo` | Both ahead and behind > 0 |
| Conflict | `conflict_repo_shows_has_conflict` | `conflict.has_conflict == true` |
| Tag created | `tag_created_blocks_freeze_validation` + `tag_create_and_delete_roundtrip` | Tag existence blocking, create/delete cycle |
| Permission-error | `nonexistent_path_returns_read_error` | `read_error` set for missing path |
| jj project | `jj_repo_uses_jujutsu_vcs_kind` | VcsKind::Jujutsu detected (skipped if jj absent) |
| Repo exists | `repo_exists_returns_true/false` | `VcsAdapter::repo_exists` works correctly |
| List contexts | `list_contexts_returns_current_branch`, `includes_second_branch` | Branch list and current detection |
| Changelog | `log_since_collects_commits_since_tag` | 2 commits collected, subjects match |
| Freeze: clean | `clean_repo_passes_freeze_validation` | `all_ready()` true, no blockers |
| Freeze: dirty | `dirty_repo_blocks_freeze_validation` | Blockers non-empty |
| Context switch | `switch_context_changes_branch` | Branch changed, confirmed via `symbolic-ref` |
| Missing path | `repo_exists_returns_false_for_missing_path` | Returns false |
| (+ 2 more) | `ahead_and_behind_repo`, `tag_create_and_delete_roundtrip` | — |

19 integration tests. All use real `git` processes in tempdir sandboxes.

**Compiler warning elimination (0 warnings):**
- Added `#[allow(dead_code)]` to all message enums, state FSM enums, and impl blocks with intentionally forward-facing variants.
- Removed 12 unused import statements across 10 files.
- Prefixed 8 unused variable bindings with `_`.
- Removed duplicate `WorkspaceSwitched` match arm (unreachable pattern).
- Removed dead `view_validation_entry` function (superseded by `view_validation_entry_owned`).
- Removed dead `FsChangeMessage` warn by annotating with `#[allow(dead_code)]`.
- Fixed `mut` / `unused_mut` in freezer view.

**gix hot-path reads (Phase 1 deferred work):**
- `endringer/vcs/git.rs`: `gix_read_head(repo_path)` — opens repository with `gix::open`, reads `head.kind` for branch name and detached-HEAD state. Zero process spawns.
- `endringer/vcs/git.rs`: `gix_read_working_tree(repo_path)` — uses `gix` status iterator to count `Modification` (uncommitted) and `Untracked` entries. Zero process spawns.
- `read_blocking` now tries gix first for both reads, falls back to CLI on any gix error.
- Net effect: status reads for Git repositories now avoid two `std::process::Command` spawns for the most common path (clean or lightly dirty repositories).


## [0.8.0] — 2025-xx-xx

### Added
- Phase 8: Performance & Observability — FS monitoring, multi-workspace management, remote tag push, missing repository detection.

**File-system event monitoring (ROADMAP completion):**
- `endringer/watcher.rs`: `FsPoller` polls sentinel files (`.git/index`, `.git/HEAD`, `.git/refs/`, `.jj/working_copy/`) per registered project.
- `FsPoller::invalidate(id)` — force-resets one project's snapshot after a write operation.
- `FsPoller::prune(active_ids)` — removes stale snapshots for deleted projects.
- `fs_watcher::fs_watch_subscription` — iced `Subscription` active when `config.fs_watch_enabled = true`; fires `Message::FsWatchTick` at `fs_debounce_secs` intervals.
- `handle_fs_watch_tick` — on tick: polls the `FsPoller`, triggers targeted single-project refresh for ≤3 changed repos, falls back to full workspace refresh for larger change sets.
- Settings: FS watch toggle (Enabled/Disabled) and debounce interval.
- 4 existing watcher tests confirmed passing.

**Multi-workspace management:**
- `AppState.all_workspaces: Vec<Workspace>` — all loaded workspaces initialised at startup.
- `AppState.active_workspace_idx` — index of the active workspace.
- `state::workspace_mgr::WorkspaceMgrState` — dialogs for create/rename/delete.
- `WorkspaceMessage` extended: `CreateWorkspaceDialogOpened/NameChanged/Confirmed/Cancelled`, `RenameWorkspace*`, `DeleteWorkspace*`, `WorkspaceSwitched(WorkspaceId)` now actually switches the active workspace.
- Sidebar: workspace name shown under the app title; `+ WS` (create) and `✎` (rename) buttons; workspace switcher list when >1 workspace exists.
- Create/rename: dialog with name validation. Delete: confirmation guard; last workspace cannot be deleted.
- Workspaces persist as individual TOML files; deleted workspace files are removed from disk.
- 1 new unit test for `CreateWorkspaceDialog`.

**Remote tag push (post-freeze):**
- `endringer/git.rs`: `push_tags(project, tag_name)` — `git push origin <tag>`.
- `VcsAdapter::push_tag` dispatcher.
- `TagPushMessage::OfferShown/PushConfirmed/PushDeclined` — offered after a fully successful freeze.
- `BackgroundMessage::TagPushCompleted` — shows success/failure count in status bar.
- Sidebar: tag-push banner appears after a successful freeze with Confirm / ✕ dismiss.
- Concurrent push with semaphore cap.

**Missing repository detection:**
- `VcsAdapter::repo_exists(project)` — checks `.git` or `.jj` exists at the project path (synchronous, O(1)).
- On each workspace status refresh, missing projects are collected into `AppState.missing_projects`.
- Dashboard card: "✗ Repository path not found" row shown for missing projects.

### Changed
- `AppState.workspace` (single workspace) is now derived from `all_workspaces[active_workspace_idx]` at startup.
- `init()` initialises `all_workspaces` from all persisted workspace files (not just the first).
- `WorkspaceMessage::WorkspaceSwitched` now performs the actual switch and triggers a refresh.


## [0.7.0] — 2025-xx-xx

### Added
- Phase 7: Future Considerations — Conflict Resolution UI, Changelog Auto-aggregation, Dependency Topology Visualisation.

**Conflict Resolution UI (`Screen::ConflictResolution`):**
- `endringer/git`: `list_conflicted_files` (parses `git diff --name-status --diff-filter=U`), `mark_resolved` (`git add <file>`), `abort_merge` (`git merge --abort`).
- `endringer/jj`: `list_conflicted_files` via `jj resolve --list`.
- `VcsAdapter::list_conflicted_files`, `mark_resolved`, `abort_merge` dispatchers.
- `model::conflict`: `ConflictedFile` (path + `ConflictMarker`), `ProjectConflictDetail`.
- `state::conflict_ops::ConflictPhase` FSM: `Idle | Loading | Browsing | Operating | Done`.
- View: conflicted-project selector → file list with per-file marker, Open in Editor / Open Merge Tool / Mark Resolved buttons, Abort Merge, Re-check.
- Uses `LaunchMessage` (Phase 6) for editor/merge-tool integration.
- 2 new unit tests for `ProjectConflictDetail`.

**Changelog Auto-aggregation (`Screen::Changelog`):**
- `endringer/git`: `log_since(project, since_ref, until_ref)` — `git log <range>` with custom format, parses into `CommitEntry`.
- `endringer/jj`: `log_since` via `jj log` template.
- `VcsAdapter::collect_changelog` — concurrent collection with semaphore cap → `ChangelogDraft`.
- `VcsAdapter::list_tags` — loads available tags for the "since" selector.
- `model::changelog`: `CommitEntry`, `ProjectCommits`, `ChangelogDraft` with `to_markdown()` and `total_commits()`.
- `state::changelog::ChangelogState` — since-ref, project selection, available-tags list, phase FSM.
- View: since-ref text input, tag quick-selector, project checkboxes, generate button (guards `is_ready_to_collect()`), Markdown draft preview (first 50 lines), Copy button.
- 3 new unit tests for `ChangelogDraft` (markdown rendering, empty-project skip, total-commits sum).
- 3 new unit tests for `ChangelogState` (ready guard).

**Dependency Topology Visualisation (Freezer integration):**
- `model::topology`: `DependencyEdge`, `DependencyGraph` (direct + transitive dependents via BFS), `ImpactWarning`, `CargoManifest`, `parse_cargo_toml_str`.
- `VcsAdapter::scan_topology` — reads `Cargo.toml` from each project root, builds cross-project dependency graph (only retains edges where both ends are registered projects).
- `state::topology::TopologyState` with `compute_warnings` — produces `ImpactWarning` for projects that are dependencies of other registered projects.
- Freezer idle view: **Scan Dependencies** button triggers `TopologyMessage::ScanRequested`.
- Freezer validation view: topology impact warnings shown above the per-project entry table (e.g. "'shared-lib' is depended upon by: api, worker").
- 5 new unit tests: direct dependents, transitive BFS, Cargo.toml basic parse, workspace parse, warning description.
- 3 new unit tests for `TopologyState::compute_warnings`.

**Navigation additions:**
- Two new sidebar entries: **Conflicts** (`Screen::ConflictResolution`) and **Changelog** (`Screen::Changelog`).
- i18n: 30+ new keys for conflicts, changelog, and topology in both `en` and `ja`.


## [0.6.0] — 2025-xx-xx

### Added
- Phase 6: UX Polish — Settings screen, History screen, external tool launch, accessibility hardening, full documentation.

**Settings screen** (all config values exposed):
- Language selector (English / 日本語; immediate effect).
- Theme toggle (Dark / Light; immediate effect).
- Refresh interval, max concurrent reads, max log entries — all editable with live binding to `AppConfig`.
- External editor and merge-tool path fields.
- Save button with status feedback (`settings.saved_ok` / `settings.save_error`).
- Section grouping: Display / Refresh & Performance / External Tools / Logs.

**History screen** (searchable, expandable log):
- Full-text search over operation kind, project ID, stdout/stderr.
- Expand/collapse per entry (▶/▼ toggle) stored in `AppState.history_expanded`.
- Detail panel: per-project success/failure, commands executed (transparency), stderr excerpt, recovery hints.
- Copy button per entry (routes through `HistoryMessage::LogCopyRequested`).
- Rollback status badge.

**External tool launch** (`LaunchMessage`):
- `LaunchMessage::OpenInEditor(path)` and `OpenInMergeTool(path)`.
- Spawns configured binary via `std::process::Command`.
- Shows "not configured" hint when the path is empty.
- Records launch success/failure in status bar.

**State additions**:
- `AppState.history_expanded: HashSet<OperationId>` — expand state per log entry.
- `AppState.settings_edit: SettingsEdit` — text-buffer mirror of AppConfig for input widgets.
- `AppState.settings_save_msg: Option<String>` — last save result.
- `SettingsEdit::from_config()` — initialises buffer from config on startup and Settings open.

**i18n additions** — Settings, History, external-tool, and accessibility strings added to both `en` and `ja` catalogs (40+ new keys).

**SettingsMessage** extended: `ExternalEditorChanged`, `ExternalMergeToolChanged`, `MaxLogEntriesChanged`, `BackToDashboard`.

**HistoryMessage** extended: `BackToDashboard`.

**Documentation** (`docs/` — mdBook-ready):
- `guide/`: dashboard, sync_center, context_ops, freezer, history, settings (6 files).
- `reference/`: keyboard shortcuts, config file format, glossary/vocabulary, FAQ (4 files).
- `contributing/`: architecture, design philosophy, local development (3 files).
- `docs/book.toml` for `mdbook serve`.

### Changed
- `FreezerMessage::NameChanged` no longer updates `AppState.freezer_name` (removed field); all freezer state is now in `AppState.freezer`.
- Ctrl+T shortcut routes through `FreezerMessage::OpenRequested` (not raw Navigate).
- Ctrl+K shortcut routes through `ContextMessage::OpenRequested(None)`.
- Settings and History screens have their own Back buttons (emit domain messages, not raw Navigate).


## [0.5.0] — 2025-xx-xx

### Added
- Phase 5: Freezer — atomic cross-repository tag/bookmark creation with rollback.
- `endringer` domain types: `FreezeValidationEntry`, `FreezeValidation`, `FreezeProjectResult`, `FreezeResult`, `FreezeOutcome`.
- `endringer/git`: `tag_create`, `tag_delete`, `tag_exists_blocking`, `validate_for_freeze` (dirty + conflict + tag-existence checks).
- `endringer/jj`: `bookmark_create`, `bookmark_delete`, `bookmark_exists_blocking`, `validate_for_freeze`.
- `VcsAdapter::validate_freeze` — concurrent validation with semaphore cap; returns `FreezeValidation`.
- `VcsAdapter::execute_freeze` — sequential tag/bookmark creation; on any failure triggers rollback loop over all previously tagged projects; per-project rollback failures generate `RecoveryHint`.
- `FreezeOutcome` enum: `Success | RolledBack | RollbackFailed | NothingDone`.
- `state::freezer::FreezerState` — name field with validation, project selection map, phase FSM.
- `state::freezer::FreezerPhase`: `Idle | Validating | ValidationReady | Executing | Done`.
- Freezer view: name input with live validity check, project checkboxes, validation result table (per-project: ready/blocked/excluded, blockers and notes), confirm/re-validate/cancel buttons (execute blocked when any included project has blockers), done screen with full per-project status, rollback state, and recovery hints.
- `FreezerMessage` fully implemented: `OpenRequested`, `NameChanged`, `ProjectToggled`, `ValidateRequested`, `ExecuteConfirmed`, `Cancelled`, `RevalidateRequested`, `BackToDashboard`.
- Every freeze attempt persisted to operation log (success, partial failure, rollback, rollback failure).
- Ctrl+T shortcut now routes through `FreezerMessage::OpenRequested`.
- 6 new unit tests for `FreezerState` (name validation, selection, stale-entry pruning).

### Changed
- `AppState.freezer_name` (plain `String`) replaced by `AppState.freezer: FreezerState`.
- `BackgroundMessage` extended with `FreezeValidationDone(FreezeValidation)` and `FreezeExecutionDone(FreezeResult)`.
- `FreezerMessage` fully replaced with Phase 5 variants.


## [0.4.0] — 2025-xx-xx

### Added
- Phase 4: Context Operations — full branch/changeset switch screen.
- `endringer`: `ContextCandidate`, `ContextList` types in `status` model.
- `endringer`: `ContextSwitchResult` in `operation` model.
- `endringer/git`: `list_contexts` — lists local + remote branches, sorts current-first.
- `endringer/git`: `switch_context` — dirty-tree pre-check, `git switch` (with local-branch creation for remote targets), `RecoveryHint` on failure.
- `endringer/jj`: `list_contexts` — bookmarks + recent change-IDs via jj CLI templates.
- `endringer/jj`: `switch_context` — `jj edit <target>`, post-switch conflict check.
- `VcsAdapter::list_contexts` and `VcsAdapter::switch_context` dispatchers.
- `state::context::ContextOpsState` — phase FSM, candidate filter, list cache.
- `state::context::ContextPhase`: `Idle | LoadingList | BrowsingList | ConfirmSwitch | Switching | Done`.
- Context Ops view: project selector, branch list with search filter, pre-switch confirmation dialog (with dirty-tree warning), in-progress indicator, result screen with commands executed and recovery hints.
- Context list cached per project; invalidated on switch.
- After successful switch, the project's status card is automatically refreshed.
- `ContextMessage::OpenRequested(Option<ProjectId>)` — `Ctrl+K` shortcut navigates and optionally pre-selects a project.
- 4 new unit tests for `ContextOpsState` candidate filtering.

### Changed
- `BackgroundMessage` extended with `ContextListLoaded(ContextList)` and `ContextSwitchDone(ContextSwitchResult)`.
- `AppState` now carries `context_ops: ContextOpsState`.
- `ContextMessage` enum fully replaced with Phase 4 variants.


## [0.3.0] — 2025-xx-xx

### Added
- Phase 3: Bulk Sync — full Sync Center screen.
- `endringer`: `git::smart_pull` — fetch + ff-merge with optional stash/pop sequence.
- `endringer`: `jj::smart_pull` — fetch + post-fetch conflict detection.
- `endringer`: `SmartPullPlan`, `SmartPullPlanEntry`, `SmartPullDisposition`, `SmartPullProgress` domain types.
- `VcsAdapter::smart_pull` dispatch to Git/jj implementations.
- `state::sync::SyncCenterState` — selection map, disposition overrides, `build_plan`.
- `state::sync::SyncPhase` enum: `Idle | Planning | FetchRunning | AwaitingConfirm | PullRunning | Done`.
- `state::sync::SyncResult` / `ProjectOutcome` for result display.
- Sync Center view: project list with checkboxes, fetch/pull buttons, plan confirm, streaming progress, result table.
- Streaming per-project execution via `Task::run` + `iced::futures::stream::iter().then()`.
- Per-project result and recovery hints display in Done phase.
- Retry-failed-projects button post-run.
- Dirty-repo disposition selector (Pull / Stash+Pull / Fetch only / Exclude) in plan confirm step.
- Conflicted repos automatically excluded from Smart Pull plan.
- Recovery hints for stash-pop failure and jj conflict post-fetch.
- Bulk Sync button on Dashboard header navigates to Sync Center.
- 6 new unit tests for plan-building logic.

### Changed
- `SyncMessage` expanded with `OpenRequested`, `DispositionChanged`, `SmartPullPlanRequested`, `SmartPullConfirmed`, `SmartPullCancelled`, `RetryFailedRequested`.
- `BackgroundMessage` extended with `SmartPullProjectCompleted`, `SmartPullPlanReady`.
- `AppState` now carries `sync: SyncCenterState`.


## [0.2.0] — 2025-xx-xx

### Added
- Phase 2: Status Refresh — full live dashboard.
- Periodic background refresh via `iced::Subscription` (interval configurable; default 60 s).
- Keyboard shortcuts: `Ctrl+R` refresh, `Ctrl+K` context, `Ctrl+T` freezer, `Ctrl+/` search, `Esc` close dialog.
- Add-project dialog: name + path input, validation error display, auto-refresh on confirm.
- Remove-project confirm dialog with safe cancel path.
- Per-card Fetch button with in-flight spinner label.
- Status filter chips (Synced / Behind / Ahead / Uncommitted / Conflict / Error) with toggle + clear-all.
- Grouping display: named groups sorted alphabetically, ungrouped projects appended last.
- Group-header rows in the card grid.
- Stale-cards overlay during background refresh (cards remain visible while updating).
- `build_display_groups` helper with full filter + group logic in `state::dashboard`.
- Expanded i18n catalog: all new dialog, filter-chip, card-action, shortcut hint strings (en + ja).
- Per-project fetch result persisted to operation log; status auto-refreshed after fetch.
- Workspace saved to disk after project add/remove.
- `is_refreshing` guard prevents concurrent refresh tasks from stacking.

### Changed
- Tarball naming now uses full semver patch version (`knotra-v0.2.0`).
- `Message` extended with `Shortcut`, `Tick`, per-dialog sub-variants.
- `AppState` extended with `add_project_dialog`, `confirm_remove_dialog`, `fetching_projects`, `is_refreshing`.
- `state::dashboard` now exports `project_matches_filter`, `build_display_groups`, `ProjectGroup`, `GroupEntry`.


---

## [0.1.0] — 2025-xx-xx

### Added
- Cargo workspace with `endringer`, `snora 0.9`, and `knotra-app` crates.
- `endringer`: domain model types (`ProjectStatus`, `WorkspaceStatus`, `OperationLog`, `RecoveryHint`, …).
- `endringer`: async `VcsAdapter` dispatching to `gix`-based Git and jj CLI implementations.
- `snora`: colour palette, WCAG-AA status colours, i18n catalog (English / Japanese).
- `knotra-app`: Elm-architecture skeleton (`State` / `Message` / `Update` / `View`).
- Dashboard: card-grid layout for up to N projects, empty-state and refreshing-state display.
- Configuration: TOML config at `~/.config/knotra/config.toml` with safe fallback.
- Workspace persistence: per-workspace TOML files.
- Operation log persistence: per-operation JSON files in `~/.local/share/knotra/history/`.
- Unit tests for domain model invariants and filter logic.
