# Context Switch

Switch working branch (Git) or change-set (jj) across projects.

Open via the **Switch…** button on the selection bar, or press `b` with projects selected. Also reachable via `Ctrl+K` / `⌘K`. Opens a modal over the Dashboard.

**Workflow**: select projects → browse candidates (search-filtered) → click target → confirm (dirty-tree warning shown) → result with commands executed and recovery hint on failure.

**VCS note**: Git uses `git switch`; jj uses `jj edit`. The UI presents both as "context switch".
