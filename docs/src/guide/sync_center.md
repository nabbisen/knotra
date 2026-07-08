# Sync Center

Bulk fetch and Smart Pull across multiple repositories.

Open via **Bulk Sync ▾** on the Dashboard or the sidebar.

## Bulk Fetch

Concurrent fetch with configurable parallelism cap. Results stream in per-repository. Retry failed repositories individually.

## Smart Pull

Plan → Confirm → Execute → Result. Dirty repos default to fetch-only; user can override to Stash→Pull→Pop per project. Conflicted repos are always excluded. Projects run sequentially to avoid stacked conflict states. Recovery hints provided when stash-pop fails.
