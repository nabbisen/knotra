# knotra Roadmap

knotra is developed in focused phases. Each phase ships as a named archive.

---

## Phase 1 — Foundation (`knotra-v0.1`)

**Goal:** A compilable, runnable application skeleton with a working dashboard that displays an empty state.

- [x] Cargo workspace setup (`endringer`, `snora`, `knotra-app`)
- [x] Domain model types (`ProjectStatus`, `WorkspaceStatus`, `OperationLog`, …)
- [x] VCS adapter: `gix`-based Git status reading
- [x] VCS adapter: jj CLI-based status reading
- [x] `snora`: theme, i18n (en / ja), widget constants
- [x] Elm-architecture: `State` / `Message` / `Update` / `View`
- [x] Dashboard: empty state, card grid layout
- [x] Configuration loading (TOML)
- [x] Workspace persistence
- [x] Operation log persistence (JSON)
- [x] Project-level tests

---

## Phase 2 — Status Refresh (`knotra-v0.2`)

**Goal:** Live project status cards on the dashboard.

- [x] Async workspace status refresh (concurrent, throttled)
- [x] Per-card: context, ahead/behind, uncommitted, untracked, conflict
- [x] Manual refresh button
- [x] Filter by status / group / search
- [x] Grouping by case / tag
- [x] Add / remove projects from workspace UI

---

## Phase 3 — Bulk Sync (`knotra-v0.3`)

**Goal:** Safe multi-repository fetch and Smart Pull.

- [x] Sync Center screen
- [x] Bulk `git fetch` / `jj git fetch`
- [x] Smart Pull with dirty-state detection
- [x] Dirty project handling (exclude / stash prompt)
- [x] Real-time progress display
- [x] Partial-failure handling and re-try

---

## Phase 4 — Context Operations (`knotra-v0.4`)

**Goal:** Quick context switch across repositories.

- [x] Context Operations screen
- [x] Git branch list and checkout
- [x] jj change-set and bookmark switch
- [x] Pre-switch confirmation dialog
- [x] Failure display with log

---

## Phase 5 — Freezer (`knotra-v0.5`)

**Goal:** Atomic cross-repository static-point creation.

- [x] Freezer screen
- [x] Pre-execution validation (clean state, tag absence)
- [x] Atomic tag / bookmark creation
- [x] Automatic rollback on partial failure
- [x] Manual recovery hints when rollback fails
- [x] History record for every freeze attempt

---

## Phase 6 — UX Polish (`knotra-v0.6`)

**Goal:** Accessible, consistent, keyboard-navigable UI.

- [x] Keyboard shortcuts (⌘/Ctrl+R refresh, ⌘/Ctrl+K context switch, …)
- [ ] Full keyboard navigation (tab order, focus visibility) — RFC-036
      Stages 1-4 deliver Tab/Shift-Tab traversal, overlay entry/trap/return,
      and a visible focus ring for the shell; dashboard card-to-card arrow
      movement and Enter-to-open the detail panel are RFC-035's, not built
      here; the three workspace-manager dialogs have entry/trap/return but
      no visible ring yet, pending an open ring-mechanism decision
      (`.git-exclude/reviewed/078-rfc-036-stage-3-review.md` Finding 1)
- [x] WCAG AA contrast verification
- [x] Unified status vocabulary audit
- [x] Settings screen (all config values exposed)
- [x] External editor / merge-tool launch
- [x] History screen with search and log copy
- [x] mdBook documentation (`docs/src`)

---

## Future Considerations (Phase 7+)

- [x] Conflict resolution UI with direct merge-tool launch
- [x] Dependency topology visualisation (`Cargo.toml` static analysis)
- [x] Changelog auto-aggregation from multiple repositories
- [x] File-system event monitoring (optional, off by default)

## Phase 8 — Performance & Observability (`knotra-v0.8`)

**Goal:** Complete the ROADMAP and harden the runtime.

- [x] File-system event monitoring (polling-based, configurable interval)
- [x] Multi-workspace management (create / rename / delete / switch)
- [x] Remote tag push after successful freeze
- [x] Missing repository path detection on cards

## Phase 9 — Code Quality, Integration Tests & gix Hot-path (`knotra-v0.9`)

**Goal:** Zero warnings, spec-mandated test coverage, faster reads.

- [x] Integration test suite — all §16.4 repository states
- [x] Compiler warning elimination (0 warnings)
- [x] gix-based hot-path for HEAD and working-tree reads

## Phase 10 — endringer 0.19.2 migration (`knotra-v0.10`)

**Goal:** Replace the hand-written knotra VCS layer with the upstream
endringer 0.19.2 library backends.

- [x] endringer-backend-{core,git,jj,async} vendored in workspace
- [x] gix `parallel` feature added (required for `ThreadSafeRepository: Send+Sync`)
- [x] vcs/git.rs reads delegated to `AsyncRepository` (gix, no CLI)
- [x] vcs/jj.rs reads delegated to `JjBackend` (gix, `jj` binary not required)
- [x] `VcsAdapter::stash_entries` and `worktree_status` added
- [x] `log_since` uses CLI ref-range (`git log <ref>..HEAD`)
- [x] 0 warnings, 36 endringer tests pass, knotra-app check clean

## v0.11.0 — RFC 0001–0008 Implementation

All design issues identified in the v0.10.0 design-note review are resolved.

- [x] RFC-0001 `LogCopyRequested` → `log_to_markdown` + `CopyToClipboard`
- [x] RFC-0002 `StashEntry.commit_id: String`
- [x] RFC-0003 `ConflictStatus::detection_unavailable` + jj CLI exception documented
- [x] RFC-0004 `gix_ahead_behind()` — gix-based upstream resolution
- [x] RFC-0005 Annotated tag support in Freezer (`create_tag_with_message`)
- [x] RFC-0006 jj `log_since` uses `jj log -r <bookmark>..@`
- [x] RFC-0007 Topology scan Cargo.toml-only scope documented
- [x] RFC-0008 `FsPoller::prune` on workspace switch and delete

## v0.15.0 — Published-crate migration (RFC-0018)

Supersedes Phase 10's vendoring: the in-tree `endringer-backend-*` crates
*were* the published `endringer` crates at 0.14, so knotra now consumes them
from crates.io rather than carrying a fork.

- [x] Remove in-tree `endringer-backend-{core,git,jj,async}`; depend on
      published `endringer-{core,git,jj,async}` 0.19.2
- [x] Rename facade `endringer` → `knotra-vcs` (VcsAdapter + model + CLI writes)
- [x] Rename foundation `snora` → `knotra-ui` (resolves the published-`snora`
      name collision; theme + i18n catalog stay knotra-owned)
- [x] `knotra-app` import renames only; app logic unchanged
- [x] 0 warnings (check + clippy, all targets), 69 tests pass under 1.91

## v0.16.0 — snora layout adoption (RFC-0019)

Adopts the snora 0.18 layout framework, replacing knotra's hand-rolled
overlay z-stack with snora's `render(AppLayout)` engine.

- [x] `snora = "0.18"` added to `knotra-app`
- [x] `app_view` re-layered: `ActiveModal` variants → `AppLayout::dialog` /
      `AppLayout::sheet`; on_close_modals wired via `ShortcutMessage::Close`
- [x] `ShortcutMessage::Close` extended to also clear `active_modal`
- [x] Workspace tab strip → `snora::widget::app_tab_bar` (direction-aware)
- [x] Dead `knotra-ui::nav_menu` module removed
- [x] 0 warnings (check + clippy, all targets), 69 tests pass under 1.91

## v0.17.0 — Screen removal (RFC-0017)

Final cleanup of the v0.11–v0.16 redesign: deletes 1,262 lines of legacy
full-screen views and trims the `Screen` enum to three variants.

- [x] Remove `view/sync_center.rs`, `view/freezer.rs`, `view/context_ops.rs`,
      `view/conflict_ops.rs`, `view/changelog_view.rs`
- [x] `Screen` enum: Dashboard / History / Settings only
- [x] All legacy `state.screen = Screen::Legacy` → open corresponding
      `ActiveModal` or panel
- [x] State modules retained (modals use the same state)
- [x] 0 warnings (check + clippy, all targets), 69 tests pass under 1.91

## v0.18.0 — endringer 0.33.1 migration (RFC-0020)

Zero-effort stability upgrade: the version bump is risk-free and the
stability signal is strong (317 tests, typed errors, audited contracts).

- [x] `endringer-* 0.19.2` → `0.33.1` in `knotra-vcs/Cargo.toml`
- [x] No source code changes required
- [x] 0/0 warnings, 36 knotra-vcs tests pass against 0.33.1

## v0.19.0 — Plain-language layer, Phase 1 (RFC-0021)

Adopts the external UX review: first-level wording becomes goal-oriented
plain language; expert terms move behind "Show details".

- [x] Plain wording for tiers, card status, and selection-bar actions
- [x] Routed through the i18n catalog (en + ja) — no parallel string system
- [x] 44px touch targets + 15px body token
- [x] Regression guard: forbidden-jargon + localisation-coverage tests
- [ ] Phase 2–6 (safe components, guided flows, setup, a11y) — deferred
- [x] 0/0 warnings, 71 tests pass under 1.91

## v0.20.0 — RFC-0021 Phases 2–4: guided modal flows

- [x] `guided_button` (disabled-with-reason) and `guided_field` helpers in knotra-ui
- [x] "Get latest safely" modal: preparing → review plan → in-progress → result
- [x] Plain dispositions ("Get latest", "Check only", "Get latest anyway", "Skip")
- [x] "Save release point" modal: input → ready check → saving → result
- [x] Disabled primary button with plain reason text
- [x] Plain result wording ("Saved", "Undone", "We could not undo everything")
- [x] "Open in editor" in conflict resolve panel; ConflictOpsMessage::OpenInEditorRequested
- [x] show_op_details + Message::ToggleOpDetails for all result views
- [x] 72 i18n keys (EN + JA); wording guard caught 2 violations during dev
- [x] 0/0 warnings, 71 tests pass under 1.91
- [ ] Phase 5 (guided setup / empty states / undo-for-removal) — next
- [ ] Phase 6 (accessibility hardening) — later

## v0.21.0 — RFC-0021 Phase 5: guided setup, empty states, undo

- [x] 2-step Add Project guided dialog (Step 1: choose folder / Step 2: name it)
- [x] Browse auto-advances + auto-fills name; AddProjectStep enum + AddProjectNextStep
- [x] Welcome empty state, all-clean state, no-filter-match state
- [x] Undo snackbar for project removal; snapshot capture; UndoRemoval / DismissUndoSnackbar
- [x] 30 new i18n keys (EN + JA); wording guard passes
- [x] 0/0 warnings, 71 tests under 1.91
- [ ] Phase 6 (accessibility hardening) — next

## v0.22.0 — RFC-0021 Phase 6: accessibility hardening (complete)

- [x] WCAG AA contrast fix: light-theme Behind/Dirty #E65100 → #BF4600 (3.5→4.71:1)
- [x] focus_id constants + focus_input() helper in knotra-ui::widget
- [x] guided_field_focused variant for ID-assigned inputs
- [x] Auto-focus: Add Project (open + step 2 advance), palette, release name, switch target
- [x] Accessible labels: ⟳ Refresh, ⊟ History, ⚙ Settings, + New workspace, › Details
- [x] Confirm remove dialog: guided_button, 44px, safe-first order, plain wording
- [x] Modal width: Fixed(600) → Fill + max_width(580) for 800px windows
- [x] Shortcuts overlay: plain-language binding descriptions
- [x] 33 pre-existing catalog gaps closed (history.*, settings.*, plain.remove.*, etc)
- [x] 0/0 warnings, 71 tests under 1.91
- RFC-0021 complete (all 6 phases)

## v0.23.0 — snora 0.25.0 migration (RFC-0022)

- [x] snora 0.18.1 → 0.25.0 in knotra-app/Cargo.toml
- [x] No source changes; iced stays 0.14; layout-engine API unchanged
- [x] Two breaking changes in range (Palette::roles, chip visual) — both in
      the design surface knotra doesn't use; no impact
- [x] Snora Design System evaluated: deferred (knotra-ui already covers it)
- [x] 0/0 warnings, 71 tests under 1.91

## Production Readiness Reset — UI/UX and User Functions

**Status:** Production release No-Go. Prior roadmap/RFC completion marks are
historical implementation claims, not production-readiness proof.

The July 2026 audit found that knotra has architecture and concept fragments,
but the UI/UX and user functions are not sufficiently designed and implemented
for production. Several visible controls mutate hidden state, silently close,
loop messages, render debug output, or route to placeholder handlers. The next
work must first convert these findings into lifecycle-managed RFCs under
`rfcs/proposed/`, then implement and verify them one by one.

Primary evidence:

- `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`
- `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`
- `.git-exclude/reviewed/062-current-gui-ui-ux-audit.md` (July 2026 GUI audit)

### RFC drafting track

- [x] Draft RFC: Workspace management completion
- [x] Draft RFC: Smart Pull modal execution completion
- [x] Draft RFC: Freezer / release point execution completion
- [x] Draft RFC: Conflict resolution action completion and editor-launch hardening
- [x] Draft RFC: Selection mode and bulk-selection completion
- [x] Draft RFC: Command palette action completion
- [x] Draft RFC: Typed context switching and context switch modal completion
- [x] Draft RFC: Changelog modal completion
- [x] Draft RFC: Activity retry semantics
- [x] Draft RFC: Dashboard grouping, sorting, and tier-density implementation
- [ ] Draft RFC-038: Per-project VCS history for Git and jj (sequenced after
      RFC-037, so it can reuse the record-list pattern — see RFC-033)

### UI/UX foundation track

A second audit (`.git-exclude/reviewed/062-current-gui-ui-ux-audit.md`) found
the GUI is not production-ready as an *interaction and visual system* — a
distinct problem from the inert-control findings above. RFC-033 decides the
shared contracts; the rest implement them.

- [x] RFC-033 — UI/UX foundation, shell, and overlay contracts (umbrella) — `Accepted (main: 71b4796)`
- [x] RFC-034 — Design foundation, application shell, and overlay host — `main: 0f5c0c5`
- [ ] RFC-035 — Dashboard and selection migration
- [ ] RFC-036 — Mutating workflow overlays and remaining ad hoc layers
- [ ] RFC-037 — Settings and History

### Implementation and verification track

Ticked items were verified on 2026-07-28 at `9b66e09`; evidence is recorded in
`.git-exclude/reviewed/`. Unticked items are genuinely open, with the reason
given.

- [ ] Every visible control either works, is disabled with a clear reason, or is hidden
      — functionally addressed by RFC-023..032; the visual/affordance half is
      RFC-035..037 (audit findings 3 and 5)
- [x] Dashboard uses the intended tier-specific information density in the active render path
      — RFC-032 R10, verified in `060`
- [ ] All primary workflows have complete validation, confirmation, progress, result, error, and recovery states
      — implemented across RFC-024..031; not yet systematically re-verified as a whole
- [ ] User-facing strings are routed through the i18n catalog where production UI renders them
      — `view/settings.rs` still carries hardcoded English; RFC-037
- [ ] UI contract tests or smoke tests prove visible controls reach the intended message handler, task, and result state
      — substantial coverage added per RFC; not yet complete across all surfaces
- [ ] Git integration tests are hermetic against global/user Git config
      — still requires externally supplied `GIT_CONFIG_*` / `GIT_EDITOR` / `TMPDIR`;
      the harness is not self-isolating
- [ ] `guided_button` and `guided_field` deleted; no legacy control helper remains
      — the RFC-034 R7 parallel-systems window; closes when RFC-035..037 migrate
      their last call sites
- [x] `cargo +1.91 fmt --check` passes
- [x] `cargo +1.91 clippy --workspace --all-targets` passes
- [x] `cargo +1.91 test -p knotra-vcs`, `cargo +1.91 test -p knotra-ui`, and `cargo +1.91 test -p knotra` pass in the documented release environment

### Release gate

Production release remains **No-Go** until **both** the RFC drafting track and
the UI/UX foundation track are complete, the accepted RFCs are implemented, and
the verification track passes with current evidence.
