# Developer Handoff — RFC-060 Stage 3: release publishes only when every platform built

RFC: `rfcs/accepted/060-ci-covers-what-we-ship.md`. Covers **D4 and D5** (R9, R10).
Do this **after** Stages 1 and 2 — it depends on `advisories.yaml` existing (D4's gate) and
on the matrix having been exercised.

Baseline: `854414d`. **This handoff is immutable.**

## 0. What is broken today

`release.yaml` creates the GitHub Release **first**, then builds three platforms with
`fail-fast: false`, each uploading its own archive. A Windows failure therefore leaves a
**published** release with Linux and macOS attached and no Windows binary. Releases 0.24.0
through 0.28.0 happen to have succeeded on all three; nothing structural prevented the
partial case.

## 1. Four jobs (R9)

| Job | Needs | Does | Permissions |
|---|---|---|---|
| `notes` | — | extract this tag's CHANGELOG section, fail if absent, upload as an artifact | `contents: read` |
| `advisories` | — | Stage 1's presence check + `cargo deny … check advisories` | `contents: read` |
| `build` (matrix, `fail-fast: false`) | `notes`, `advisories` | build, archive, **upload artifact** — no release upload | `contents: read` |
| `publish` | `build` | download all artifacts; one `gh release create` with the notes and **all three** archives | `contents: write` |

Keep `fail-fast: false` — one platform failing should still show whether the others work —
but `publish` requires all three. **If any platform fails, nothing is published.**

Set `permissions: contents: read` at workflow level and grant `contents: write` **only** on
`publish`.

The CHANGELOG extraction, `--locked`, the archive layout, asset names, and `LICENSE`/
`NOTICE`/`README.md` staging are unchanged. Do not redesign them.

## 2. A dispatched run must not publish (R9)

Add `workflow_dispatch`. `publish` runs only for a real tag push:

```yaml
if: github.event_name == 'push'
```

That is how this is verified without creating a public release: dispatch on `ci/rfc-060`,
watch `notes` → `advisories` → `build` produce three downloadable artifacts, and `publish`
skip.

**Do not test this by pushing a version tag.** A tag matching the release patterns publishes
a real release on a public repository.

## 3. Recovery, stated in a comment

A transient failure is cleared by re-running the failed jobs; `publish` then proceeds. A
failure needing a code fix needs a **new tag** — a re-run rebuilds the same commit. Put that
in the workflow, where whoever hits it will read it.

## 4. Action majors (R10)

Across all three workflows: `actions/checkout@v7`, `actions/cache@v6`,
`actions/upload-artifact@v7`, `actions/download-artifact@v8`.

Release notes read for every major in between, so nothing here should surprise you: Node 24
runtimes (GitHub-hosted runners satisfy the minimum runner version); ESM migrations; checkout
v7 refusing fork checkouts under `pull_request_target`/`workflow_run`, neither of which we
use; download-artifact v8 **failing on an artifact digest mismatch by default** — keep that
default. `upload-artifact` v7's `archive: false` is opt-in; we do not use it.

First-party actions stay on **major tags**, matching current practice. Introduce **no
third-party action** (RFC non-goal).

## 5. Out of scope

Stage 1's and Stage 2's files; signing, notarisation, SBOMs; archive naming or layout;
knotra's version or cutting a release.

## 6. Verification

Local five gates unchanged (**303**), `git diff --check 854414d..HEAD`, plus the dispatched
branch run in §2. `git diff --stat` touches `.github/workflows/` only.

## 7. What to report back

The `release.yaml` diff; the dispatched run's URL showing **`publish` skipped** and three
artifacts present, with their names; confirmation no release or tag was created and the
branch was deleted; the action versions in all three workflows; gate output, gate five in the
range form.
