# Changelog

All notable changes to knotra are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
