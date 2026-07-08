# Architecture

```
GUI Layer (knotra-app / iced)
  State / Message / Update / View / Subscription

Application Layer (knotra-app)
  AppState, routing, task orchestration, persistence

Domain Layer (endringer / model)
  ProjectStatus, OperationLog, FreezeValidation, SmartPullPlan, …

VCS Adapter Layer (endringer / vcs)
  VcsAdapter → git.rs + jj.rs (CLI-based read/write)

Local Persistence Layer (knotra-app)
  config.rs, persistence.rs
```

## Crates

- **`endringer`** — VCS abstraction + domain models
- **`snora`** — theme, i18n, widget constants  
- **`knotra-app`** — GUI binary (iced Elm architecture)

## Concurrency

Status reads: concurrent behind `tokio::sync::Semaphore` (configurable cap).  
Writes: sequential via `stream::iter().then()` (Smart Pull, Freeze).  
UI thread: never blocked — all I/O via `Task`/`Subscription`.

## Failure architecture

Every bulk operation: plan → validate → confirm → execute → rollback (if needed) → log → recovery hints.
