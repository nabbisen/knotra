# Freezer (Tag / Bookmark)

Atomic cross-repository tag (Git) or bookmark (jj) creation.

Open via the **Tag…** button on the selection bar, or press `t` with projects selected. Opens a modal over the Dashboard.

**Workflow**: enter freeze name → select projects → validate (checks dirty state, conflict, tag existence) → confirm (blocked if any included project has a blocker) → execute (sequential, with automatic rollback on failure) → result (success / rolled-back / rollback-failed with manual recovery commands).

Never overwrites an existing tag automatically — delete it first if intentional.
