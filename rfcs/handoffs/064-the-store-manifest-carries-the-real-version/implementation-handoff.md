# Developer Handoff — RFC-064: the Store manifest carries the real version

RFC: `rfcs/accepted/064-the-store-manifest-carries-the-real-version.md`, **accepted 2026-10-19
(project owner)**.

Baseline: **on top of your RFC-063 commits** (its handoff plus `amendment-a1.md`). Both
change `release.yaml`, in different jobs. **311** tests before this change.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The point of this change

The owner read `Version="0.0.0.0"` as a bug. It was a placeholder that CI filled in, and the
repository gave no sign of it. After this change, **every version a reader sees in the
repository is the real one**, and a test fails loudly if two of them disagree. Judge every
line you write against that: no placeholders, no tests that skip themselves, no comment
describing a mechanism that no longer exists.

## 1. Commit 1 — the manifest and its test (D1, D2)

**`packaging/windows/AppxManifest.xml`**: `Identity`'s `Version="0.0.0.0"` becomes
`Version="0.30.0.0"`. Nothing else in the file changes (R1).

**`crates/knotra-app/tests/store_manifest_version.rs`**, a new integration test:

- Expected value: the `MAJOR.MINOR.PATCH` core of `env!("CARGO_PKG_VERSION")`, plus `.0`.
  For `0.30.0` that is `0.30.0.0`; for a pre-release such as `0.31.0-rc.1` it is `0.31.0.0`.
- Actual value: read
  `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/windows/AppxManifest.xml")`.
  If the file is missing or unreadable, **fail** with the path.
- **Read the right attribute (R3).** Find the `<Identity` element and take its text up to the
  closing `/>`. Within that text only, find the attribute named exactly `Version`: preceded
  by whitespace, then `="`. `MinVersion=` and `MaxVersionTested=` sit on a different element,
  and must not be able to satisfy or fail the test even if they move into `Identity`. If
  `Identity` or its `Version` is absent, fail and say which.
- On a mismatch, the message names both values and says what to do, for example:
  `AppxManifest.xml Identity Version is 0.30.0.0 but the crate is 0.31.0 (expected 0.31.0.0)
  - bump packaging/windows/AppxManifest.xml with Cargo.toml`.
- No new dependency. Standard library only.
- One `#[test]` function. Its doc comment says why the test exists: the manifest's version
  must match the crate's, release prep bumps both, and this test is what catches a missed
  bump.

**`crates/knotra-app/Cargo.toml`** (R4), in `[package]`:

```toml
# Checks the repository's Store manifest, which is not part of this crate.
exclude = ["tests/store_manifest_version.rs"]
```

Confirm `cargo +1.91 package -p knotra --list` no longer lists it.

**Seen to fail before trusted (R2).** In a scratch copy, bump only `Cargo.toml`'s workspace
version (and its two internal pins) to `0.30.1`, run `cargo +1.91 test -p knotra --test
store_manifest_version`, and report the exact failure. Then show it passing on the real
tree. Do the plant in a scratch copy, **not** in the working tree.

## 2. Commit 2 — the `msix` job and the listing guide (D3, D4)

In `.github/workflows/release.yaml`, job `msix`:

1. **"Read the crate version"** — its logic is unchanged, but its leading comment (`R3: the
   version comes from Cargo.toml's workspace version - the one place release prep bumps -
   never hand-maintained here …`) is no longer true. Rewrite it in the file's style: the
   version comes from `Cargo.toml`; the manifest carries the same version, kept in step by
   `tests/store_manifest_version.rs` and checked again by the next step (RFC-064).
2. **New step directly after it**, before "Stage the package directory": read
   `packaging/windows/AppxManifest.xml` as XML, take `Package.Identity.Version`, and fail
   with `::error::` naming both values unless it equals `$env:MSIX_VERSION`. On success,
   print the verified version, so the run log shows it. It writes nothing.
3. **Remove "Substitute the version into the staged manifest"** entirely, with its comment
   block.
4. **Remove "Confirm the committed manifest is unchanged"** entirely, with its comment
   (R6). Nothing in the job writes to a tracked file any more, so it would check a
   condition that cannot occur.

Every other `msix` step stays as it is, including the pre-release refusal in "Read the crate
version". Do not touch the other jobs; RFC-063's are yours from the previous handoff.

In `packaging/windows/store-listing.md`, "Before you submit", replace the bullet beginning
`` **`AppxManifest.xml` says `Version="0.0.0.0"` on purpose.** `` with (R7):

```markdown
- **`AppxManifest.xml` carries the real version**, the crate version plus `.0`
  (0.30.0 is 0.30.0.0). Release prep bumps it together with `Cargo.toml`; a test
  fails if they differ, and the release job checks it again before packing.
```

## 3. Out of scope

- Any source change outside the new test. No CHANGELOG entry: nothing a user runs changes.
- Pre-release packaging (deferred F4), and every other `release.yaml` job.
- RFC-061's text. It is a done RFC, a historical record; RFC-064 supersedes its D3 for the
  future.

Across both commits, `git diff --stat` against your RFC-063 head shows `AppxManifest.xml`,
`crates/knotra-app/Cargo.toml`, the new test, `release.yaml` and `store-listing.md` only.

## 4. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <RFC-063 head>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
cargo +1.91 publish --workspace --dry-run --locked
```

**312** (231 / 32 / 49). The new test runs as its own `test result` line under `-p knotra`.
Suppression map **five**, dead-code probe **one** line, `advisories ok`, and the dry run
packages and verifies all three crates. **No `cargo publish` without `--dry-run`, ever.**

`git grep -n '0\.0\.0\.0' -- packaging .github` must print nothing.

If `actionlint` is available, run it on `release.yaml`; if not, say so.

**The dispatched run on GitHub is mine**, after review and push.

## 5. What to report back

- the manifest diff (one line);
- the test file and the `exclude` lines as committed, and the `cargo package --list` result;
- the planted failure's exact output, then the passing run;
- the `msix` job as committed, from "Read the crate version" to "Pack the MSIX";
- the `store-listing.md` diff;
- the `git grep` result, `actionlint` output or its absence, and gate output with the
  `diff --check` range.

RFC-063 and RFC-064 may share one review request, each in its own commits.
