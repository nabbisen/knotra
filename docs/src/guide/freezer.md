# Freezer

Atomic cross-repository tag (Git) or bookmark (jj) creation.

**Workflow**: enter freeze name → select projects → validate (checks dirty state, conflict, tag existence) → confirm (blocked if any included project has a blocker) → execute (sequential, with automatic rollback on failure) → result (success / rolled-back / rollback-failed with manual recovery commands).

Never overwrites an existing tag automatically — delete it first if intentional.
