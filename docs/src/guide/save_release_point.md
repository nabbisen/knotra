# Save release point

Atomic cross-repository tag (Git) or bookmark (jj) creation.

Select projects and click **Save release point** on the selection bar, or press
`Ctrl+T`. Opens a modal over the Dashboard.

**Workflow**: enter a release name → select projects → run the ready check (looks for
unsaved work, a project that needs a choice first, a name already in use, or a project
that could not be read) → confirm (blocked if any included project has a blocker) →
save (sequential, with automatic rollback on failure) → result (saved / rolled back /
rollback failed, with manual recovery commands).

Never overwrites an existing tag automatically — delete it first if intentional.
