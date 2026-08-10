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
- [ ] Full keyboard navigation (tab order, focus visibility) — **RFC-036
      (`main: d20c7be`) and RFC-035 (`main: f605834`) are both complete.** Tab
      and Shift-Tab reach every control on the shell, the workspace dialogs, the
      dashboard toolbar, its rows and sections, and the selection bar; each
      renders a WCAG-AA-verified focus ring in both themes; `↑`/`↓`/`j`/`k` move
      between cards and `Enter` opens the focused card.
      **One gap remains, and it is why this stays unchecked:** the Group and Sort
      **select menus cannot be opened by keyboard** — Tab reaches them and the
      ring renders, but iced 0.14's `pick_list` handles no key press at all
      (verified: one `Event::Keyboard` occurrence, `ModifiersChanged`; no
      `operate`, no `Focusable` — `.git-exclude/reviewed/101-...md` Finding 2).
      Closing it needs a knotra-owned select widget, which is its own RFC, not a
      follow-up. Until then a keyboard-only user cannot change grouping or
      sorting, so "full" is not yet true.
      Two recorded limitations, neither a navigation gap: a focused *disabled*
      filled control's ring measures ~3.0-3.3:1 — WCAG 1.4.11 non-text contrast
      (3:1) in dark, marginal in light, and no colour choice improves it
      (`083` Finding 2); and no screen-reader/ARIA layer exists or is planned,
      since iced 0.14 exposes no accessibility API (RFC-033 non-goal).
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
- [x] Phase 2–6 (safe components, guided flows, setup, a11y) — deferred here;
      shipped across v0.20.0–v0.22.0
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
- [x] Phase 5 (guided setup / empty states / undo-for-removal) — shipped in v0.21.0
- [x] Phase 6 (accessibility hardening) — shipped in v0.22.0

## v0.21.0 — RFC-0021 Phase 5: guided setup, empty states, undo

- [x] 2-step Add Project guided dialog (Step 1: choose folder / Step 2: name it)
- [x] Browse auto-advances + auto-fills name; AddProjectStep enum + AddProjectNextStep
- [x] Welcome empty state, all-clean state, no-filter-match state
- [x] Undo snackbar for project removal; snapshot capture; UndoRemoval / DismissUndoSnackbar
- [x] 30 new i18n keys (EN + JA); wording guard passes
- [x] 0/0 warnings, 71 tests under 1.91
- [x] Phase 6 (accessibility hardening) — shipped in v0.22.0

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
- [ ] Draft RFC-039: Per-project VCS history for Git and jj (sequenced after
      RFC-038, so it can reuse the record-list pattern — see RFC-033)

### UI/UX foundation track

A second audit (`.git-exclude/reviewed/062-current-gui-ui-ux-audit.md`) found
the GUI is not production-ready as an *interaction and visual system* — a
distinct problem from the inert-control findings above. RFC-033 decides the
shared contracts; the rest implement them.

- [x] RFC-033 — UI/UX foundation, shell, and overlay contracts (umbrella) — `Accepted (main: bf07f1c)`
- [x] RFC-034 — Design foundation, application shell, and overlay host — `main: ce05a44`
- [x] RFC-036 — Keyboard navigation and focus traversal — `main: d20c7be`
- [x] RFC-035 — Dashboard and selection migration — `main: f605834`
- [x] RFC-037 — Mutating workflow overlays and remaining ad hoc layers — `main: bb04df2`.
      Six stages, twelve commits. `view/bulk_modals.rs` 1,337 ELOC → `view/overlays/`
      across six files, all under threshold; `modal_shell` and `guided_button` both
      deleted. `tests.rs` never edited. Two user-visible defects found and fixed
      while doing chrome work — the faded-selected-option bug (same class RFC-035
      fixed for Group/Sort) and the Remove confirmation styled identically to its
      own cancel. Reviews `131`-`136`.
- [ ] RFC-038 — Settings and History

RFC-036 was inserted ahead of RFC-035 after the July 2026 spike found no Tab
traversal existed anywhere in the application (`.git-exclude/reviewed/073`);
RFC-035's R22/R23 depend on it. Numbers are identifiers, not sequence — the
implementation order is recorded in "Sequencing" below.

### Operational hygiene track

Added 2026-07-30 on the owner's approval, from
`.git-exclude/reviewed/081-preparation-review-044-carry-forward-audit.md`. These
are the release-gate items the reset declared and then routed around: three of
`044`'s five findings were still unresolved twelve days later, and one had grown
34%. Scheduled as their own theme rather than folded into RFC-035, so they are
not deferred again.

- [x] **RFC-040 — `app.rs` decomposition. Complete 2026-08-01**
      (`rfcs/done/040-app-module-decomposition.md`, `main: 54e5d5d`).
      **`app.rs` 3,255 → 270 ELOC — 92%** — across six stages and sixteen
      commits, into eleven modules. `tests.rs` was **never edited**, so the same
      166 tests guarded every commit without once being adjusted to fit. One
      cross-handler edge in the finished structure (`misc → workspace`);
      `shared.rs` and `focus_ops.rs` depend on no handler. Non-import ELOC drift
      across the moving stages: **+10**, all rustfmt signature wrapping.
      Reviews `086`-`093`.
- [x] **RFC-041 — split `handle_background`. Complete 2026-08-10**
      (`rfcs/done/041-background-module-decomposition.md`, `main: f3e69aa`).
      **`background.rs` 761 ELOC → a `background/` directory of seven files**,
      every one under the threshold: `mod.rs` 173, `smart_pull.rs` 279,
      `freeze.rs` 164, `fetch.rs` 109, `context_switch.rs` 66, `status.rs` 65,
      `conflict.rs` 50. That closes RFC-040's one accepted exception, so the
      `app/` tree now has none. Four stages, six commits, `tests.rs` **never
      edited** — the same 255 tests guarded every stage. Reviews `125`-`128`.

      **Correcting what this entry previously said.** It recorded, from `092`,
      that `SmartPullProjectCompleted` constructs state before the match and runs
      a shared tail after it, and concluded **the unit of extraction is not the
      arm**. The implementation disproved that: `handle_background`'s body *is*
      the match, with nothing before or after it, and all twenty-one moved items
      — eighteen arm bodies and three whole helpers — came out byte-identical
      extracted arm-by-arm. That same fact is why the eighteen early `return`s
      were semantically neutral, which is the reasoning the whole split rests on,
      so the correction matters beyond bookkeeping (`128` §5, RFC-041 §"The fact
      that makes this cheap and safe").
- [x] Supporting: committed CI gate workflow. **Done** — Handoff
      `011-ci-gate-workflow.md`, `.github/workflows/ci.yaml`. Runs fmt, clippy,
      the three test suites, and `git diff --check` on every push and PR, plus an
      **MSRV job** (Handoff `036`) that reads `rust-version` from the manifest so
      the two cannot disagree. `workflow_dispatch` added 2026-08-10 so a push
      touching only path-ignored files can still be verified.

      It has already paid for itself twice over. This entry used to note that
      manual discipline "is why trailing whitespace reached a commit during the
      0.24.0 release" — the same thing happened again in the 0.26.0 cycle, and
      this time CI caught it before the tag (`129` A4). That failure also exposed
      that the local form of the fifth gate, bare `git diff --check`, inspects the
      working tree against the index and therefore verifies nothing when
      everything is committed. The local gate is now
      `git diff --check <base>..HEAD`, matching what CI runs.
- [x] Supporting: `docs/` accuracy. **Closed 2026-07-31, smaller than scoped.**
      The `docs/src/guide/` claim was wrong — `b5e1c81` updated those three pages
      in the same commit that removed the views, and they describe the current
      modal architecture correctly; the error is recorded in
      `.git-exclude/reviewed/085-...md`. The real defects found and fixed were
      `docs/src/contributing/architecture.md`'s stale `endringer` version
      (`7cbd3fc`) and `docs/src/reference/keyboard.md`, which claimed full
      keyboard accessibility and documented four unbound keys (`37e607f`).
- [x] Supporting: Git integration test hermeticity — **done 2026-08-01**
      (`ee840a4`, review `096`). `cargo test -p knotra-vcs` now passes with no
      environment supplied, and under a hostile one — verified against a
      `gitconfig` carrying a different identity and a stdin-blocking editor.
      Every git invocation in the suite builds through one `git_command` helper,
      so adding a variable means editing one function. Known limitation, carried
      deliberately: `knotra-vcs`'s own `run_git` sets no env, so a *library*
      write path that ever needs `git commit`, a CLI annotated tag, or credential
      handling would inherit ambient config again. No exercised path does today.

The supporting items carry no product design decisions and are tracked as
Developer Handoffs rather than standalone RFCs, per the governance policy's
allowance for supporting work outside an RFC where the relationship is explicit
and approved. They may run in parallel with RFC-040; they touch different files.

### Sequencing

Updated 2026-08-10, after 0.26.0 shipped. **The operational hygiene track is now
complete** — RFC-040 and RFC-041 both closed, CI gates committed and proven,
Git tests hermetic. The UI/UX foundation track has three RFCs left, none written.

1. **RFC-037 — mutating workflow overlays and remaining ad hoc layers.**
   **Drafted 2026-08-10** (`rfcs/proposed/037-mutating-workflow-overlays.md`),
   awaiting owner acceptance. `view/bulk_modals.rs` is 1,337 ELOC holding all
   five overlays behind a hand-rolled `modal_shell`. It carries one
   verification-track item — the `guided_button` window — and only if its D5 is
   accepted; see below. It does **not** carry `view/settings.rs`'s hardcoded
   English, which is RFC-038's per RFC-033 H4. An earlier version of this line
   said otherwise and was wrong.
2. **RFC-038 — Settings and History.**
3. **RFC-039 — per-project VCS history for Git and jj.** Sequenced last of the
   three so it can reuse RFC-038's record-list pattern (RFC-033).
4. **A knotra-owned select widget** — still unscheduled and unwritten, and still
   the single thing standing between line 81 and being ticked: iced 0.14's
   `pick_list` cannot be opened by keyboard, so Group and Sort are unreachable
   for a keyboard-only user. Its own RFC when it comes.

Two small items banked from the 0.26.0 cycle, neither worth its own RFC and both
suitable to fold into whichever handoff comes next:

- **`rust-version.workspace = true` in the three member crates.** The declared
  MSRV is inherited by none of them, so it reaches neither Cargo nor crates.io
  and has been inert for every published version (`130`).
- **The save-failure detail gap.** A dangling-symlink refusal names the link and
  the missing directory when saved from Settings, but a Group / Sort / collapse
  failure shows a fixed string and logs the detail — and that is the frequent
  path (`129` A2).

Carried debt, neither blocking: live captures owed for Handoffs 031 and 032 while
the render environment is unavailable (`111`), and whether `↑`/`↓` should be
gated during text entry (`114`).

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
      — `view/settings.rs` still carries hardcoded English; **RFC-038**, per
      RFC-033 H4 (long recorded here as RFC-037; RFC-036 was reused for keyboard
      navigation, which shifted every later number by one)
- [ ] UI contract tests or smoke tests prove visible controls reach the intended message handler, task, and result state
      — substantial coverage added per RFC; not yet complete across all surfaces
- [x] Git integration tests are hermetic against global/user Git config
      — done 2026-08-01 (`ee840a4`, review `096`). Verified passing both with no
      environment supplied and under a hostile one (foreign identity in
      `GIT_CONFIG_GLOBAL`, stdin-blocking editor). The library's own `run_git`
      remains un-isolated, which is harmless while no exercised write path needs
      an identity or an editor — the trigger is recorded in the hygiene track.
- [ ] `guided_field` remains; `guided_button` is deleted
      — the RFC-034 R7 parallel-systems window. **19 live call sites in four
      files**, measured 2026-08-10: `bulk_modals.rs` 11, `add_project_modal.rs` 4,
      `workspace_manager.rs` 2, `dashboard/empty.rs` 2. This entry used to say the
      window "closes when RFC-035..037 migrate their last call sites"; it does
      not. RFC-035 is closed and left `dashboard/empty.rs` unmigrated, and eight
      sites across three files are owned by **no scheduled RFC** — RFC-038 is
      Settings and History, RFC-039 is per-project VCS history, and neither calls
      them. RFC-037 D5 proposes absorbing all nineteen so the helpers can actually
      be deleted; that is an open owner decision.
- [x] `cargo +1.91 fmt --check` passes
- [x] `cargo +1.91 clippy --workspace --all-targets` passes
- [x] `cargo +1.91 test -p knotra-vcs`, `cargo +1.91 test -p knotra-ui`, and `cargo +1.91 test -p knotra` pass in the documented release environment

### Release gate

Production release remains **No-Go** until **both** the RFC drafting track and
the UI/UX foundation track are complete, the accepted RFCs are implemented, and
the verification track passes with current evidence.
