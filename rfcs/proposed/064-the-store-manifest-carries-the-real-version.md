# RFC-064 - The Store manifest carries the real version

| Field | Value |
|---|---|
| Status | Proposed 2026-10-18 |
| Priority | Medium - nothing ships wrong today; the committed file misleads its readers |
| Effort | Small - one manifest line, one test, one workflow step replaced and one removed |
| Target | Production Readiness Reset - distribution |
| Related files | `packaging/windows/AppxManifest.xml`, `.github/workflows/release.yaml` (`msix`), `packaging/windows/store-listing.md`, `crates/knotra-app/tests/store_manifest_version.rs` (new), `crates/knotra-app/Cargo.toml` |
| Related | `rfcs/done/061-...md` D3/R3 (the placeholder design this replaces), `rfcs/accepted/063-...md` (also edits `release.yaml`) |
| Decided by | project owner, 2026-10-18 - option B: commit the real version, verify instead of substitute |

## Summary

`packaging/windows/AppxManifest.xml` says `Version="0.0.0.0"`, and the `msix` job writes the
real version into a staging copy. The package is right, but the committed file is wrong
about what it describes, and its owner read it as a bug. This RFC commits the **real**
version, keeps the two versions in step with a test that runs on every push, and turns the
job's substitution into a check.

## Problem

RFC-061 D3 chose the placeholder for one reason: one version in one place, so release prep
cannot bump `Cargo.toml` and forget the manifest. That is the failure that broke 0.28.0.

The reason holds, but the placeholder is not the only way to meet it. A placeholder makes
**forgetting impossible by making the file meaningless**. A guard makes **forgetting
impossible by failing loudly**, and leaves the file meaningful. The second is the better
trade:

- a reader of the repository sees the version the Store package carries;
- the guard fails on every push and in the local gates, long before a release;
- the cost is one more line in release prep, which the guard will not let anyone miss.

## Non-goals

- Pre-release packaging (deferred `release.yaml` F4). The `msix` job still refuses a
  pre-release version, as it does today.
- Any change to the manifest's identity, assets or capabilities.
- Any change to what the GitHub release or crates.io receive.

## Decision

### D1. The manifest carries `<crate version>.0`

`Identity Version` becomes `0.30.0.0` now, matching `Cargo.toml`. From here on, release prep
bumps it with the other three version lines. Partner Center reserves the fourth part, so it
stays `0`.

### D2. A test keeps the two in step, and is never published

An integration test, `crates/knotra-app/tests/store_manifest_version.rs`. It compares
`env!("CARGO_PKG_VERSION")` with the manifest at
`CARGO_MANIFEST_DIR/../../packaging/windows/AppxManifest.xml`, and fails unless the manifest
equals the crate version's `MAJOR.MINOR.PATCH` plus `.0`. The failure message names both
values and says to bump the manifest with the crate. A missing or unreadable manifest is a
failure.

- **It checks the repository, not the crate, so it is not shipped with the crate.**
  `crates/knotra-app/Cargo.toml` gains `exclude = ["tests/store_manifest_version.rs"]`, with
  a one-line comment saying why. A crates.io download then has no such test, rather than a
  test that skips itself. No test anywhere decides at runtime whether to run.
- **Read the `Identity` element's `Version` attribute, not a substring.** The same file has
  `MinVersion` and `MaxVersionTested`; a loose `Version="…"` match would compare the wrong
  one. Take the text of the `<Identity …/>` element, then the attribute named exactly
  `Version` (preceded by whitespace) within it. No new dependency: knotra has no XML parser,
  and this one element does not justify adding one.
- **A pre-release crate version** (`0.31.0-rc.1`) compares by its numeric core (`0.31.0.0`).
  The test does not decide whether pre-releases are packaged; F4 does.
- **Seen to fail before trusted**: bump `Cargo.toml` alone in a scratch copy; the test must
  fail naming both versions.

### D3. The `msix` job verifies instead of substituting

Replace "Substitute the version into the staged manifest" with a check of the **committed**
manifest, placed directly after "Read the crate version" and before any staging work. It
reads `packaging/windows/AppxManifest.xml` as XML, takes `Package.Identity.Version`, and fails
unless it equals `$env:MSIX_VERSION`, naming both. Nothing is written. The placeholder count,
the string replacement and the `"0.0.0.0"` anchor all go.

**"Confirm the committed manifest is unchanged" is removed.** It guarded against the
substitution step editing the committed file. With no step writing anything, it checks a
condition that can no longer occur.

### D4. The listing guide says what is true

`packaging/windows/store-listing.md` "Before you submit": replace the placeholder note added
on 2026-10-18 with one saying the manifest carries the real version, that release prep bumps
it with `Cargo.toml`, and that a test fails if they differ.

## Requirements

| # | Requirement |
|---|---|
| R1 | `AppxManifest.xml` `Identity Version` is `0.30.0.0`; nothing else in the file changes |
| R2 | The D2 test passes at `0.30.0` and is **seen to fail** when only `Cargo.toml` is bumped, naming both values |
| R3 | The test reads the `Identity` element's `Version` attribute, so `MinVersion`/`MaxVersionTested` cannot satisfy or fail it |
| R4 | The test is excluded from the `knotra` package: `cargo package -p knotra --list` does not list it. It contains no skip path |
| R5 | `msix` writes nothing to the manifest; before staging, it fails, naming both values, if the committed version differs from `MSIX_VERSION` |
| R6 | The "committed manifest is unchanged" step is removed; every other `msix` step is unchanged, including the pre-release refusal |
| R7 | D4's wording replaces the placeholder note |
| R8 | Test count **312** (311 + the new test); suppression map unchanged |

## Test Plan

- Gates, **312**. The test is seen to fail on a planted `Cargo.toml`-only bump, which is
  then reverted.
- `cargo package -p knotra --list` does not list `tests/store_manifest_version.rs`, and
  `cargo +1.91 publish --workspace --dry-run --locked` still packages and verifies all three
  crates.
- **A dispatched run of `release.yaml`** after landing (mine): `msix` passes the new check
  and packs; its log shows the verified version.
- **The next release's prep task** bumps four lines, and its review confirms the guard
  passes.

## Security Considerations

None new. The job loses a write step on its staging copy; it gains no permission and no
input.

## Migration / rollout

- Release prep from now on: `Cargo.toml` lines 10, 20, 21 **and** the manifest's
  `Identity Version`. The release-prep task template says so.
- **Order with RFC-063.** Both edit `release.yaml`, in different jobs (`msix` here, the new
  `crates` there). Implement after RFC-063's commits, on top of them; the two may share one
  review request, each in its own commits.
