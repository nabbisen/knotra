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

- [ ] Context Operations screen
- [ ] Git branch list and checkout
- [ ] jj change-set and bookmark switch
- [ ] Pre-switch confirmation dialog
- [ ] Failure display with log

---

## Phase 5 — Freezer (`knotra-v0.5`)

**Goal:** Atomic cross-repository static-point creation.

- [ ] Freezer screen
- [ ] Pre-execution validation (clean state, tag absence)
- [ ] Atomic tag / bookmark creation
- [ ] Automatic rollback on partial failure
- [ ] Manual recovery hints when rollback fails
- [ ] History record for every freeze attempt

---

## Phase 6 — UX Polish (`knotra-v0.6`)

**Goal:** Accessible, consistent, keyboard-navigable UI.

- [ ] Keyboard shortcuts (⌘/Ctrl+R refresh, ⌘/Ctrl+K context switch, …)
- [ ] Full keyboard navigation (tab order, focus visibility)
- [ ] WCAG AA contrast verification
- [ ] Unified status vocabulary audit
- [ ] Settings screen (all config values exposed)
- [ ] External editor / merge-tool launch
- [ ] History screen with search and log copy
- [ ] mdBook documentation (`docs/src`)

---

## Future Considerations (Phase 7+)

- Conflict resolution UI with direct merge-tool launch
- Dependency topology visualisation (`Cargo.toml` static analysis)
- Changelog auto-aggregation from multiple repositories
- File-system event monitoring (optional, off by default)
