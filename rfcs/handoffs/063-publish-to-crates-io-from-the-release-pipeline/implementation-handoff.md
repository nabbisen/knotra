# Developer Handoff — RFC-063: publish to crates.io from the release pipeline

RFC: `rfcs/accepted/063-publish-to-crates-io-from-the-release-pipeline.md`, **accepted
2026-10-18 (project owner)**.

Baseline: the RFC-063 acceptance commit on `main`. **311** tests (230 / 32 / 49), all gates
green.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. Why care is needed here

**A crates.io upload cannot be undone.** A version can only be yanked, never deleted, and its
number can never be reused. Every decision below exists to make a wrong upload impossible,
not merely unlikely.

**You never upload anything.** Not a real publish and not a test publish. No
`cargo publish` without `--dry-run`, anywhere, at any point. The only real upload this job
ever does happens on the next release tag, which is mine and the owner's to push.

**The owner's setup is done**: each crate has a GitHub trusted publisher (owner `nabbisen`,
repository `knotra`, workflow `release.yaml`, environment `crates-io`), and the GitHub
environment `crates-io` exists. The workflow filename and environment name are therefore
**fixed**: renaming either breaks publishing.

## 1. D6 — the crates inherit the workspace metadata (R9)

In each of `crates/knotra-app/Cargo.toml`, `crates/knotra-ui/Cargo.toml` and
`crates/knotra-vcs/Cargo.toml`, add next to the fields already inherited:

```toml
repository.workspace    = true
keywords.workspace      = true
categories.workspace    = true
```

Match the file's existing alignment. Confirm with
`cargo +1.91 metadata --no-deps --format-version 1` that all three report
`repository: https://github.com/nabbisen/knotra`, keywords `["git", "jj"]` and categories
`["development-tools"]`. The `manifest has no documentation, homepage or repository` warning
from `cargo package` must be gone.

Its own commit.

## 2. D1–D4 — the `crates` job (R1–R5, R7, R8, R10)

A new job in `.github/workflows/release.yaml`, after `msix`, with a comment block in the
file's style naming RFC-063 and saying why each choice was made.

```yaml
  crates:
    name: Publish to crates.io
    needs: publish
    runs-on: ubuntu-latest
    if: github.event_name == 'push'
    environment: crates-io
    permissions:
      contents: read
      id-token: write
```

`contents: read` must be listed. A job-level block **replaces** the workflow default, and
without it checkout fails.

Steps, in this order:

1. **Checkout** — `actions/checkout@v7`, as elsewhere in the file.
2. **Toolchain and Linux build dependencies** — the same commands `build` uses
   (`rustup toolchain install "$TOOLCHAIN" --profile minimal` and the same `apt-get` package
   list). `cargo publish` verifies each crate by building it, so it needs both (R7).
3. **The version guard (D3, R4)** — before anything authenticates. Fail unless
   `$GITHUB_REF_NAME` equals **each** of: the `[workspace.package]` version in `Cargo.toml`,
   and the version `cargo metadata --no-deps` reports for `knotra`, `knotra-ui` and
   `knotra-vcs`. On a mismatch, print both values in a `::error::` line and exit non-zero.
4. **Authenticate** — `uses: rust-lang/crates-io-auth-action@v1` with `id: auth` (R10).
   Authenticate **after** the guard, so a bad tag never obtains a token.
5. **Publish per crate (D4, R5)**, in the order `knotra-ui`, `knotra-vcs`, `knotra`. For each
   one:
   - Check whether the version is already on crates.io by reading the sparse index,
     `https://index.crates.io/kn/ot/<crate>`, one JSON object per line, each with a `"vers"`
     field. Match the version **exactly**, as a parsed field, not a substring: `0.30.0` must
     not match `0.30.0-rc.1` or `10.30.0`. Send a `User-Agent` header naming knotra, as
     crates.io asks.
   - If it is present: `::notice::<crate> <version> is already on crates.io, skipping`, and
     continue.
   - If it is absent: `cargo +"$TOOLCHAIN" publish -p <crate> --locked`, with
     `CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}` set **on that step only**.
   - If the index request itself fails (network, HTTP error), **fail the job**. Never
     treat "could not check" as "absent".

   cargo waits for each upload to appear in the index before it returns, so `knotra` resolves
   its two siblings when its turn comes.

Do not touch `publish`, `msix`, `build`, `notes` or `advisories` (R8). Pin no new action by
SHA, matching the file's major-tag convention (R10).

## 3. D5 — the dispatched dry run (R6)

On `workflow_dispatch`, run `cargo +"$TOOLCHAIN" publish --workspace --dry-run --locked` with
the same toolchain and Linux dependencies. Put it in a separate job, `crates-dry-run`,
`if: github.event_name == 'workflow_dispatch'`, with **no** `id-token` permission and **no**
`environment`. It may run in parallel with `build`; it needs nothing.

Measured on `main` today: a dry run of an **already-published** version warns
(`crate knotra@0.30.0 already exists on crates.io index`) and still packages and verifies, so
a dispatch between releases works.

## 4. Out of scope

- Any source or test change. Test count stays **311** (R11).
- `deny.toml`, `ci.yaml`, `advisories.yaml`.
- The CHANGELOG: nothing a user runs changes.
- The deferred `release.yaml` items F2 (slash in a branch name) and F4 (pre-release tags and
  `msix`). Do not fix them in passing.

`git diff --stat <baseline>..HEAD` shows the three crate `Cargo.toml` files and
`release.yaml` only, in two commits.

## 5. Verification

Local gates:

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
cargo +1.91 publish --workspace --dry-run --locked
```

**311**, suppression map **five**, dead-code probe **one** line, `advisories ok`, and the dry
run packages and verifies all three crates.

**The guard and the skip, seen to work (R4, R5)**, run locally as scripts with the step's
exact logic, **never with a real `cargo publish`**:

- the guard with `GITHUB_REF_NAME=0.30.1`: fails, naming `0.30.1` and `0.30.0`;
- the guard with `GITHUB_REF_NAME=0.30.0` and the workspace at `0.30.0`: passes;
- the index check for `0.30.0`: reports "already on crates.io" for all three crates;
- the index check for `0.30.1`: reports "absent" for all three;
- the index check against an unreachable URL: fails, and does not report "absent".

For each, report the command and its output. To run a step's script locally, extract it into
a scratch file outside the repository; do not add a script file to the repository.

Workflow syntax: if `actionlint` is available, run it on `release.yaml` and report the
output. If it is not, say so; do not install tools to get it.

**The dispatched run on GitHub is mine**, after review and push: `workflow_dispatch` runs
the workflow file from the ref you select, and I verify `crates-dry-run` there.

## 6. What to report back

- the D6 diff and the `cargo metadata` output for the three crates;
- the full `crates` and `crates-dry-run` jobs as committed;
- §5's five local script checks, with commands and output;
- `actionlint` output, or that it was unavailable;
- gate output, with the `diff --check` range;
- confirmation that **no `cargo publish` without `--dry-run` was run**, and that crates.io
  still lists 0.30.0 as the latest for all three crates (`https://index.crates.io/kn/ot/<crate>`).
