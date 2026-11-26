# Get latest safely

Bulk, safe updates across multiple repositories, from the selection bar.

**Check for updates** — select projects and click **Check for updates**. A concurrent,
read-only check with a configurable parallelism cap; results stream in per repository.
Retry failed repositories individually. Nothing on disk changes.

**Get latest safely** — select projects and click **Get latest safely**. Opens a modal
over the Dashboard: Plan → Confirm → Execute → Result. Dirty repos default to check-only; the
plan view lets you override to stash → pull → pop per project. Conflicted repos are
always excluded. Projects run sequentially, to avoid stacked conflict states. Recovery
hints are shown inline when stash-pop fails.
