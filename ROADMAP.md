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
- [x] Full keyboard navigation (tab order, focus visibility)
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

## v0.11.0 — RFC 001–008 Implementation

All design issues identified in the v0.10.0 design-note review are resolved.

- [x] RFC-001 `LogCopyRequested` → `log_to_markdown` + `CopyToClipboard`
- [x] RFC-002 `StashEntry.commit_id: String`
- [x] RFC-003 `ConflictStatus::detection_unavailable` + jj CLI exception documented
- [x] RFC-004 `gix_ahead_behind()` — gix-based upstream resolution
- [x] RFC-005 Annotated tag support in Freezer (`create_tag_with_message`)
- [x] RFC-006 jj `log_since` uses `jj log -r <bookmark>..@`
- [x] RFC-007 Topology scan Cargo.toml-only scope documented
- [x] RFC-008 `FsPoller::prune` on workspace switch and delete

## v0.15.0 — Published-crate migration (RFC-018)

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
