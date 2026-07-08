# Sync & Pull

Bulk fetch and Smart Pull across multiple repositories.

Open via the **Fetch** or **Pull** button on the selection bar, or select projects on the Dashboard and press `f` (fetch) or `p` (pull). Both open a modal over the Dashboard.

## Bulk Fetch

Concurrent fetch with configurable parallelism cap. Results stream in per-repository. Retry failed repositories individually.

## Smart Pull

Plan → Confirm → Execute → Result. Dirty repos default to fetch-only; the plan view lets you override to Stash→Pull→Pop per project. Conflicted repos are always excluded. Projects run sequentially to avoid stacked conflict states. Recovery hints are shown inline when stash-pop fails.
