# RFC-063 - Publish to crates.io from the release pipeline

| Field | Value |
|---|---|
| Status | Accepted 2026-10-18 (project owner) |
| Priority | High - three releases reached GitHub but not crates.io before anyone noticed |
| Effort | Small - one job, three manifest lines per crate, no new secret |
| Target | Production Readiness Reset - distribution |
| Related files | `.github/workflows/release.yaml`, `crates/*/Cargo.toml` |
| Related | `.git-exclude/reviewed/218-...md` §7 (the finding), `rfcs/done/060-...md` D4 (the pipeline this extends), `rfcs/done/061-...md` (its last extension) |
| Decided by | project owner, 2026-10-18 - "Use CI. Trusted Publishing is available." |

## Summary

Pushing a version tag publishes the GitHub release and builds the Store package, but nothing
publishes to crates.io. 0.29.0, 0.29.1 and 0.30.0 were published there by hand on 2026-10-18,
after the owner noticed. This adds one job to `release.yaml` that publishes all three crates
with **crates.io Trusted Publishing**, and fixes the crates' missing repository link.

**No stored token.** The job proves its identity to crates.io with GitHub's OIDC token and
receives a token that lasts 30 minutes and is revoked when the job ends.

## Problem

1. **No publish step exists.** Not in a workflow, a script, a document, or git history. It was
   a manual routine that the RFC-060 pipeline replaced without carrying over.
2. **crates.io publication is irreversible.** A version cannot be deleted, only yanked, and
   its number can never be reused. A wrong upload is permanent, so the job must refuse to run
   unless what it is about to publish is exactly the tagged version.
3. **The crates have never linked to GitHub.** The workspace declares `repository`,
   `keywords` and `categories`, but no crate inherits them: `cargo metadata` reports
   `repository: None`, and crates.io shows no repository.

Facts about Trusted Publishing, from crates.io's documentation (read 2026-10-18):

- The crate must **already exist** on crates.io, and the person configuring it must be an
  owner. All three crates exist.
- Configuration is per crate: repository owner, repository name, **workflow filename**, and
  an optional **environment**.
- The workflow needs `permissions: id-token: write` and the action
  `rust-lang/crates-io-auth-action@v1`, whose `token` output is passed to `cargo publish` as
  `CARGO_REGISTRY_TOKEN`. The token expires after 30 minutes; the action's post step revokes
  it.

## Non-goals

- Publishing anything other than the three workspace crates.
- Changing `publish`, `msix`, or how the GitHub release is made.
- Back-filling versions: crates.io now holds 0.28.0 through 0.30.0, with no gap.
- Revoking the owner's personal crates.io API token. That is the owner's to do once this job
  has published one release.

## Decision

### D1. A `crates` job after `publish`

`needs: publish`, and the same `if: github.event_name == 'push'`. It runs **only after the
GitHub release exists**. If the GitHub side fails, nothing irreversible has happened. If
crates.io fails, the GitHub release is already out and this one job is re-run.

Its own permissions: `contents: read` and `id-token: write`. A job-level `permissions` block
replaces the workflow default, so `contents: read` must be listed again or checkout fails.
`id-token: write` exists nowhere else in the workflow.

### D2. A GitHub environment, `crates-io`

The job declares `environment: crates-io`, and the owner sets the same name in each crate's
trusted-publisher configuration. crates.io then accepts a token only from **this job in this
workflow**. Another job in `release.yaml`, or a future workflow, cannot publish even with
`id-token: write`. The owner may also add protection rules to the environment, such as
restricting it to tags or requiring approval; the RFC does not depend on them.

### D3. A version guard before anything uploads

The first step fails the job unless the tag (`$GITHUB_REF_NAME`) equals the workspace
version in `Cargo.toml` **and** the version of each of `knotra`, `knotra-ui` and
`knotra-vcs` that `cargo metadata` reports. A tag pushed on the wrong commit then stops the
job here, before an irreversible upload.

### D4. Re-runnable after a partial publish

`cargo publish --workspace` uploads in dependency order and stops at the first error. If a
run fails after one crate is up, re-running must finish the rest, not fail on the crate
already present. So the job publishes per crate, in the order `knotra-ui`, `knotra-vcs`,
`knotra` (the order cargo chose for 0.29.0–0.30.0). Before each upload it checks whether that
version is already on crates.io. If it is, the job skips that crate with a notice. If not, it
runs `cargo publish -p <crate>`.

### D5. Dispatched runs do a dry run

On `workflow_dispatch`, `publish` is skipped, and so is `crates`. To keep the packaging
verifiable without a release, a dispatched run instead runs
`cargo publish --workspace --dry-run`, in a job with **no**
`id-token` permission and no environment. Implementer's choice: a separate `crates-dry-run`
job, or a conditional step. Either way, a dispatched run can never obtain an upload token.

### D6. The crates inherit the workspace metadata

Each `crates/*/Cargo.toml` gains `repository.workspace = true`, `keywords.workspace = true`
and `categories.workspace = true`, next to the fields it already inherits. The change takes
effect from the next published version.

## Requirements

| # | Requirement |
|---|---|
| R1 | `crates` runs only on a tag push and only after `publish` succeeds |
| R2 | `id-token: write` appears in **only** the `crates` job; it declares `environment: crates-io` |
| R3 | No stored crates.io token: no `secrets.*` reference for crates.io anywhere |
| R4 | The D3 guard runs before any upload and fails, naming both values, on any mismatch |
| R5 | D4: an already-published crate version is skipped with a notice, not an error |
| R6 | D5: a dispatched run dry-runs all three crates and holds no upload token |
| R7 | The toolchain is `$TOOLCHAIN` (1.91), and the Linux build dependencies are installed as `build` installs them, since `cargo publish` verifies by building |
| R8 | `publish` and `msix` are unchanged, apart from `crates` naming `publish` in its `needs` |
| R9 | D6: `cargo metadata` reports `repository: https://github.com/nabbisen/knotra` for all three crates |
| R10 | Actions referenced by major tag, matching the file's convention: `rust-lang/crates-io-auth-action@v1` |
| R11 | No source change; test count stays **311** |

## Test Plan

- **A dispatched run** on the implementing branch: `crates-dry-run` (or the D5 step) packages
  and verifies all three crates; `publish` and `crates` skipped (R6).
- **The guard, seen to fail**: in a scratch copy, run the guard step's script with
  `GITHUB_REF_NAME` set to a version other than the workspace's. It must fail and name both
  values (R4).
- **The skip, seen to work**: run the D4 loop's check with the version set to `0.30.0`. All
  three are already published, so all three must report "already published, skipping"
  without uploading (R5). Use the check alone, not `cargo publish`.
- `cargo metadata` shows the repository for all three (R9). Local gates unchanged, **311**.
- **The first real use is the next release**, and its readiness report must confirm the new
  version on crates.io for all three crates.

## Security Considerations

This **removes** a long-lived credential from the release path rather than adding one.
Trusted Publishing has crates.io check the job's identity: repository, workflow file and
environment, all signed by GitHub. The resulting token lasts 30 minutes and is revoked when
the job ends.

The trust boundary is `release.yaml` running in the `crates-io` environment. Anyone who can
push a tag can publish, exactly as anyone who can push a tag can cut a GitHub release today.
The environment's protection rules are where the owner can narrow that.

`rust-lang/crates-io-auth-action` is maintained by the Rust project. It is the only new
third-party action, and the only one that ever holds an upload token.

## Migration / rollout

Owner, once, **before the first release that uses this**:

1. On crates.io, for each of `knotra`, `knotra-ui`, `knotra-vcs`: Settings → Trusted
   Publishing → Add → GitHub. Owner `nabbisen`, repository `knotra`, workflow filename
   `release.yaml`, environment `crates-io`.
2. On GitHub, create the environment `crates-io` under repository Settings → Environments.
   Protection rules are optional.

If step 1 is missing, the job fails at authentication. The GitHub release is already out by
then, and the remedy is to configure the publisher and re-run the job. Nothing is lost.

Afterwards, once one release has published through CI, the owner may revoke the personal API
token used on 2026-10-18.

## Amendment A1 (2026-10-18) - the version guard moves to the first job

**Supersedes D3's placement and R4's location.** As accepted, the tag-vs-version guard ran
inside `crates`, which runs after `publish`. A tag pushed on the wrong commit would therefore
already have created a public GitHub release, named for one version and carrying another,
before anything stopped it.

The guard runs instead as the **first step of `notes`**, on tag pushes only. Its logic is
unchanged: `$GITHUB_REF_NAME` must equal the `[workspace.package]` version and the version
`cargo metadata --no-deps` reports for `knotra`, `knotra-ui` and `knotra-vcs`. Every other job
depends on `notes` (`build` needs it; `publish`, `msix` and `crates` need `build`), so a
mismatch publishes **nothing anywhere**. `crates` carries no guard of its own: a second copy
would only be a second place to keep in step.
