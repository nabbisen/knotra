# Developer Handoff — RFC-060 Stage 2: tests on every platform we ship

RFC: `rfcs/accepted/060-ci-covers-what-we-ship.md`. Covers **D3** (R7, R8, R12).
Independent of Stage 1; do not touch `deny.toml`, `advisories.yaml` or `release.yaml`.

Baseline: `854414d`. **This handoff is immutable.**

## 0. What is broken today

We ship Linux, macOS and Windows; CI tests only Linux. Measured at this source:
`cargo check --workspace --all-targets --target x86_64-pc-windows-gnu` fails with **one**
error — `crates/knotra-app/src/tests.rs`, `resolve_project_file_path_rejects_symlink_escape`,
calls `std::os::unix::fs::symlink` unguarded. Every other Unix-only test in the tree is
already gated, and `knotra-ui`'s and `knotra-vcs`'s test targets compile for Windows as they
stand.

## 1. Port the test — do not gate it out (R7)

That test guards a **security property**: a file path must not escape the project through a
symlink. Windows has symlinks, and its path handling differs most. Gating it to Unix would
drop the check exactly where it is most worth having.

```rust
#[cfg(unix)]
std::os::unix::fs::symlink(&outside_file, &link).expect("symlink");
#[cfg(windows)]
std::os::windows::fs::symlink_file(&outside_file, &link).expect("symlink");
```

I compiled this shape for both `x86_64-pc-windows-gnu` and Linux, all targets: both clean.
**Whether it passes when run on Windows is unmeasured** — that is what §3 is for. Creating a
symlink on Windows needs privilege; GitHub's runners have it. If the test fails there,
that is a finding (§3), not something to gate away.

## 2. `ci.yaml`: matrix the test job (R7)

- `strategy: fail-fast: false`, `matrix.os: [ubuntu-latest, macos-latest, windows-latest]`.
- The three `cargo test -p …` steps unchanged.
- The apt step becomes `if: runner.os == 'Linux'`.
- Cache keys already carry `runner.os`; leave them.
- `lint` and `msrv` stay Linux-only (RFC non-goal).

## 3. Measure before landing (R8)

Accepting RFC-060 authorised **one throwaway branch, `ci/rfc-060`**, to run the new workflow
with `workflow_dispatch` before anything reaches `main`. Delete it afterwards.

**Report each platform's result with its log.** A macOS or Windows failure is a **finding**:
report it. Do not skip the test, do not add `continue-on-error`, do not gate it to Linux, and
do not land the matrix red. If a failure reveals a real defect, it gets its own fix and its
own review — say so and stop.

Likely failure shapes, so they are recognisable rather than surprising: path separators and
case-insensitivity on Windows; git's line-ending conversion (the `knotra-vcs` harness already
sets `GIT_CONFIG_NOSYSTEM`, which neutralises the usual source); `jj` absent on runners —
`jj_available()` should skip, and if it does not, that is a finding too.

## 4. Test counts (R12)

**Linux stays 303 exactly** — the port adds no test. macOS and Windows counts are **reported
as measured**; expect them lower, because Unix-only tests are gated there. **Name each test
that does not run on each platform.** Do not "fix" a lower count.

## 5. Out of scope

Stage 1's and Stage 3's files; clippy or MSRV on non-Linux; any product fix a new failure
reveals; `crates/knotra-vcs` source.

## 6. Verification

Local five gates (**303**, 223/31/49), `git diff --check 854414d..HEAD`, plus
`cargo +1.91 check --workspace --all-targets --target x86_64-pc-windows-gnu` — which must now
pass, where it fails at baseline.

## 7. What to report back

The ported test, quoted; the matrix diff; **per-platform results with logs and counts**, and
the gated-test names per platform; the branch run's URL and confirmation the branch was
deleted; gate output, gate five in the range form.
