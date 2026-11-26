# Change work area

Switch working branch (Git) or change-set (jj) across projects.

Select projects and click **Change work area** on the selection bar, or press
`Ctrl+K`. Opens a modal over the Dashboard.

**Workflow**: select projects → browse candidates (search-filtered) → click a target →
confirm (a warning is shown if the project has unsaved work) → result with commands
executed and a recovery hint on failure.

**VCS note**: Git uses `git switch`; jj uses `jj edit`. The UI presents both as
"change work area".
