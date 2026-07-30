# Architecture

```
GUI Layer (knotra-app / iced / snora)
  State / Message / Update / View / Subscription

Application Layer (knotra-app)
  AppState, routing, task orchestration, persistence

Domain Layer (knotra-vcs / model)
  ProjectStatus, OperationLog, FreezeValidation, SmartPullPlan, …

VCS Adapter Layer (knotra-vcs / vcs)
  VcsAdapter → git.rs + jj.rs (reads via endringer, writes via CLI)

Local Persistence Layer (knotra-app)
  config.rs, persistence.rs
```

## Crates

- **`knotra-vcs`** — VCS facade: `VcsAdapter`, domain models, `FsPoller`. Reads delegate to `endringer-{core,git,jj,async}` 0.33.2 (crates.io); writes use the VCS CLI.
- **`knotra-ui`** — knotra UI foundation: `KnotraTheme`, `StatusColor`, i18n catalog, layout tokens.
- **`knotra-app`** — GUI binary (iced Elm architecture, snora layout engine).

## Concurrency

Status reads: concurrent behind `tokio::sync::Semaphore` (configurable cap).  
Writes: sequential via `stream::iter().then()` (Smart Pull, Freeze).  
UI thread: never blocked — all I/O via `Task`/`Subscription`.

## Failure architecture

Every bulk operation: plan → validate → confirm → execute → rollback (if needed) → log → recovery hints.

**Exception — jj conflict detection:**  
Conflict detection for jj repositories uses `jj log -r @ -T conflict` (CLI).  
The jj conflict flag is stored in a protobuf-encoded file (`.jj/working_copy/tree_state`)  
whose format is not part of jj's public API. Until the format is stable and  
documented, the CLI is the safe approach. When `jj` is absent, the conflict  
status is reported as `detection_unavailable: true` and the UI shows "Unknown"  
rather than a false "No conflict."
