# RFC-060 - CI covers what we ship: every platform, every advisory class, releases only when all builds succeed

| Field | Value |
|---|---|
| Status | Accepted 2026-09-22 (project owner) |
| Priority | High - two fixable advisories sat unreported for months; two of three shipped platforms never run tests |
| Effort | Medium - one new workflow, two rewritten workflows, one config file, one test ported |
| Target | Production Readiness Reset - build and release integrity |
| Related files | `.github/workflows/ci.yaml`, `.github/workflows/release.yaml`, new `.github/workflows/advisories.yaml`, new `deny.toml`, `crates/knotra-app/src/tests.rs` |
| Related | Task 080 (must land before the advisory gate is enabled); `.git-exclude/reviewed/186-...md` (the miss this prevents) |
| Decided by | project owner, 2026-09-15 - CI coverage approved as the second of five decisions |

## Summary

knotra's CI checks one platform and no dependencies. Three gaps, each **measured** before
this RFC was written:

1. **No advisory scanning.** On 2026-09-15 a scanner ran against our lockfile for the first
   time and found a **vulnerability** (`crossbeam-epoch`, published 2026-07-06) and an
   unsoundness advisory (`anyhow`, 2026-06-25), both fixable, both missed by a manual
   review the same week.
2. **Tests run only on Linux; we ship Windows and macOS.** Measured: knotra's test code
   **does not compile for Windows** — one security test calls a Unix-only API.
3. **A release is published before anything is built.** A platform failure leaves a public
   release with a binary missing.

This RFC closes all three, and updates CI's GitHub actions from two major versions behind.

## Problem

### 1. Nothing tells us when a dependency becomes unsafe

Advisories are published against crates we already depend on, with no change on our side.
Today we learn of one only if someone writes to us. The scanner run on 2026-09-15 found six:

| Advisory | Crate | Class | Fixable |
|---|---|---|---|
| `RUSTSEC-2026-0204` | `crossbeam-epoch` 0.9.18 | vulnerability | yes — Task 080 |
| `RUSTSEC-2026-0190` | `anyhow` 1.0.102 | unsound | yes — Task 080 |
| `RUSTSEC-2026-0253` | `lru` 0.16.4 | unsound, run time | no — held by `cryoglyph` |
| `RUSTSEC-2024-0436` | `paste` 1.0.15 | unmaintained | no successor |
| `RUSTSEC-2026-0206` | `rustybuzz` 0.20.1 | unmaintained | no successor |
| `RUSTSEC-2026-0192` | `ttf-parser` 0.25.1 | unmaintained | no successor |

### 2. A scanner's defaults can hide an entire class

snora learned this in 0.49.0, and we reproduced it: `cargo-deny`'s `unsound` setting is a
**scope**, and its default covers only direct dependencies. Of the **676** external
packages in knotra's lockfile, **15** are direct dependencies; the other **661** are
transitive.

Measured with `cargo-deny` 0.20.2 against our lockfile, `lru` not accepted:

| Config | Exit | `lru` reported |
|---|---|---|
| `unsound = "all"` | 1 | **yes** |
| `unsound` key absent | **0** | **no — not mentioned at all** |

A missing line turns a memory-safety advisory into a green gate. Checking values cannot
catch this; only checking that every key is **present** can.

### 3. Two of three shipped platforms never run a test

`ci.yaml` runs three jobs, all `ubuntu-latest`. `release.yaml` builds Windows and macOS —
0.28.0 succeeded on both — but builds are not tests.

Measured: `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu` fails on
**one** error. `crates/knotra-app/src/tests.rs`,
`resolve_project_file_path_rejects_symlink_escape`, calls `std::os::unix::fs::symlink`
unguarded. Every other Unix-only test is already gated, and `knotra-ui`'s and
`knotra-vcs`'s test code compiles for Windows as is.

That test guards a **security property** — a project file path must not escape the project
through a symlink — and Windows has symlinks. Gating it out would drop the check on the
platform where path handling differs most. Porting it to
`std::os::windows::fs::symlink_file` under `#[cfg(windows)]` was compiled for Windows and
Linux; both compile. It has not been *run* on Windows.

**Whether the tests *pass* on Windows and macOS is not measured.** No Windows or macOS
machine is available here. The first real run is part of implementation (D3).

### 4. A release can be published broken

`release.yaml` creates the GitHub Release first, then builds three platforms with
`fail-fast: false`, each uploading its own archive. If Windows fails, Linux and macOS still
attach, and the public release has no Windows binary.

### 5. CI's actions are two major versions behind

`actions/checkout@v5` (current v7), `actions/cache@v5` (current v6). Release notes read for
every major in between: Node 24 runtime (GitHub-hosted runners satisfy it), ESM migration,
checkout v7 refusing fork checkouts under `pull_request_target`/`workflow_run` (neither
used here). `download-artifact` v8 fails on an artifact digest mismatch by default — the
secure default, kept.

## Non-goals

- **License, bans, and sources checks.** `cargo-deny` can do them. License policy for a
  distributed binary is a legal decision for the owner, not a CI detail; not bundled here.
- **clippy and MSRV on Windows/macOS.** Test builds compile the test targets on every
  platform; lint stays Linux.
- **Code signing, notarisation, SBOMs, dependency-update bots.**
- **Pinning first-party `actions/*` by commit SHA.** Current practice is major tags; this
  RFC introduces **no third-party action**, so the trust surface does not grow.
- **Fixing the two advisories.** Task 080.
- **Adding anything a contributor must install.** `deny.toml` is inert without
  `cargo-deny`; the tool runs only in CI — respecting the owner's standing constraint
  recorded in `ci.yaml`.

- **A second, lockfile-level scanner (`cargo audit`) alongside it.** It answers a different
  question and would report entries for targets we never build; one gate with a stated
  scope is clearer than two with overlapping ones.

## Decision

### D1. An advisory gate with every class set explicitly

A root `deny.toml`:

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"
unmaintained = "all"
unsound = "all"
unused-ignored-advisory = "deny"
ignore = [
  # one entry per accepted advisory; see D1 rules
]
```

Vulnerabilities always fail; `cargo-deny` has no setting to downgrade them.

**Why `cargo-deny`, and what question it answers.** `cargo-deny` scans the **resolved
dependency graph**; `cargo audit` reads **`Cargo.lock`**, which carries entries no target we
build ever compiles. Both are legitimate; they answer different questions, and a graph scan
is the one that matches "what do we ship". `cargo-deny` is also the only one of the two
documented to fail on an acceptance that no longer matches (`unused-ignored-advisory`),
which is what keeps the accepted list from going stale.

**No target filter, deliberately.** `[graph]` sets `all-features` and leaves `targets`
unset, so every platform is scanned. knotra ships Linux, macOS and Windows: a macOS-only
advisory such as `paste`'s must fail our gate even though it is absent from a Linux build
graph. Narrowing to the host target would hide exactly the advisories the Windows and macOS
jobs exist to cover.

**Accepted advisories** — the four with no fix — are listed by ID. Each entry's `reason`
states the path it reaches us by and the condition that retires it. With
`unused-ignored-advisory = "deny"`, an entry that no longer matches — because upstream
fixed it and we updated — **fails the gate** until removed. Measured: a planted entry for
the already-fixed `quick-xml` advisory fails with `advisory-not-detected`, pointing at the
line.

**A presence check runs before the scan** and fails if any of `yanked`, `unmaintained`,
`unsound`, `unused-ignored-advisory` is absent from `[advisories]`. A commented-out key does
not count. Measured with a plain-shell implementation: passes a complete config, fails on a
removed key, fails on a commented-out key.

**Installed, not borrowed.** CI runs `cargo install --locked cargo-deny --version 0.20.2`,
cached. It declares Rust 1.88 as its minimum, below our 1.91 toolchain; building it in CI
is verified by D3's branch run, not assumed. The published
`cargo-deny-action` runs a Docker image pinned only by tag; this adds no third-party action
and no container.

### D2. When the gate runs

A new `advisories.yaml`, `permissions: contents: read`, triggered by:

- push to `main` and pull requests touching `Cargo.lock`, `Cargo.toml`, `**/Cargo.toml`,
  `deny.toml`, or the workflow itself;
- **a weekly schedule** — advisories are published without any change on our side;
- `workflow_dispatch`.

**Known limit, recorded rather than worked around:** GitHub disables scheduled workflows in
a public repository after 60 days without repository activity, and runs them only on the
default branch.

**The release workflow runs the same gate before building** (D4). A release is never cut
against an unexamined advisory.

### D3. Tests on every shipped platform

`ci.yaml`'s test job becomes a matrix over `ubuntu-latest`, `macos-latest`,
`windows-latest`, `fail-fast: false`, running the three test crates as today. Linux build
dependencies install only `if: runner.os == 'Linux'`. Cache keys already carry
`runner.os`.

The symlink-escape test is **ported, not gated out**: `#[cfg(unix)]` symlink /
`#[cfg(windows)]` `symlink_file`.

**Measuring before landing.** Accepting this RFC authorises pushing one throwaway branch,
`ci/rfc-060`, to run the new workflows with `workflow_dispatch` before anything reaches
`main`, and deleting it afterwards. Any macOS or Windows test failure is **reported as a
finding with its log** — not skipped, not `continue-on-error`, not gated out. A failure that
reveals a product defect is fixed and reviewed on its own; the matrix does not land red.

### D4. A release is published only when every platform built

`release.yaml` becomes four jobs:

| Job | Needs | Does | Permissions |
|---|---|---|---|
| `notes` | — | extract this version's CHANGELOG section; fail if missing; upload as artifact | read |
| `advisories` | — | D1's presence check and scan | read |
| `build` (matrix, `fail-fast: false`) | `notes`, `advisories` | build, archive, upload artifact | read |
| `publish` | `build` | download all artifacts; `gh release create` with notes and **all** archives at once | **write** |

If any platform fails, **nothing is published**. The tag exists; the release does not. A
transient failure is cleared by re-running the failed jobs. A failure that needs a code fix
needs a new tag — a re-run rebuilds the same commit. Write permission is confined to
the one job that publishes.

`release.yaml` also gains `workflow_dispatch`. A dispatched run executes `notes`,
`advisories`, and `build` — producing downloadable artifacts — and **skips `publish`**. That
is how the new release flow is verified without creating a public release.

### D5. Actions at current majors

`actions/checkout@v7`, `actions/cache@v6`, `actions/upload-artifact@v7`,
`actions/download-artifact@v8`, across all three workflows.

## Requirements

| # | Requirement |
|---|---|
| R1 | `deny.toml` per D1: all four keys set, the four unfixable advisories accepted by ID, each `reason` naming path and retirement condition |
| R2 | The presence check fails on an absent or commented-out key — **seen to fail** on both before it is trusted |
| R3 | `unused-ignored-advisory = "deny"` **seen to fail** on a planted stale entry |
| R4 | With Task 080 landed, the scan passes with exactly the four accepted; without it, fails on exactly `anyhow` and `crossbeam-epoch` |
| R5 | `cargo-deny` installed at exactly 0.20.2 with `--locked`; no third-party action or container added |
| R6 | `advisories.yaml` per D2, including the weekly schedule, `contents: read` |
| R7 | Test job matrix over Linux, macOS, Windows per D3; symlink test ported, not gated |
| R8 | D3's branch run completed and each platform's result reported with logs; failures reported, never suppressed |
| R9 | `release.yaml` per D4; `publish` is the only job with write permission; a dispatched run skips `publish` and produces artifacts for all three platforms |
| R10 | Actions at D5's majors in all three workflows |
| R11 | No contributor-side requirement added; the five local gates are unchanged |
| R12 | Linux test count stays **303** exactly — the port adds no test. macOS and Windows counts are **reported as measured**; they are expected to be lower, because Unix-only tests are gated there, and each gated test is named |

## Test Plan

- **Gate probes, each seen to fail before trusted:** a removed key and a commented-out key
  (R2); a planted stale ignore (R3); the scan against the tree before and after Task 080
  (R4).
- **Branch run** of `ci.yaml` and `advisories.yaml` via `workflow_dispatch` on
  `ci/rfc-060` — all three platforms (R8).
- **Dispatched `release.yaml`** on the branch: `publish` skipped, three artifacts
  downloadable (R9).
- Local gates unchanged (R11).

## Security Considerations

The advisory gate is the point of this RFC. Supply-chain choices made to avoid widening
trust: `cargo-deny` comes from crates.io at an exact version with `--locked`; the advisory
database is fetched from RustSec's repository over HTTPS at run time; no third-party action
or container is introduced. Permissions narrow: every new or rewritten job is read-only
except `publish`.

Accepted advisories stay visible in `deny.toml` with reasons, and cannot outlive their fix.

## Migration / rollout

Order: **Task 080 lands first**, or the gate lands failing. Independent of RFC-059.

Contributors: no change. Maintainers: a release now appears only after all three platforms
build; a platform failure means no release until it is fixed, instead of a partial one.
