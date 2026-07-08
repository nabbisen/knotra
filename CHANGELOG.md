# Changelog

All notable changes to knotra are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
  Without `parallel`, gix 0.83 uses `Rc`-based internal pools which are not
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
  Without `parallel`, gix 0.83 uses `Rc`-based internal pools which are not
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
