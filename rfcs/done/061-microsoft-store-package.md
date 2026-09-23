# RFC-061 - Build a Microsoft Store package in the release pipeline

| Field | Value |
|---|---|
| Status | Implemented (main: cac4af4) |
| Priority | Medium - a distribution channel the owner has already prepared for; nothing is broken without it |
| Effort | Small - one job, no new dependency, no new secret |
| Target | Production Readiness Reset - distribution |
| Related files | `.github/workflows/release.yaml`, `packaging/windows/AppxManifest.xml` |
| Related | `.git-exclude/reviewed/198-...md` (the packaging review), `.git-exclude/reviewed/199-...md` (the scope decision), `rfcs/done/060-...md` D4 (the four-job pipeline this extends) |
| Decided by | project owner, 2026-09-22 - Store listing in 0.29.0's successor, artifact only |

## Summary

`packaging/windows/` has held a Store manifest and tile assets since `eef4aed`, and nothing
builds anything from them. This adds one job to the release pipeline that packs the Windows
binary we already ship into an `.msix`, and uploads it as a **workflow artifact** for manual
submission to Partner Center.

**No signing certificate. No new CI secret. No change to what is published.**

## Problem

Three facts decide the whole design, and two of them were measured rather than assumed.

**1. The Store signs submissions; we do not.** Microsoft's signing guidance lists, for
*Microsoft Store distribution*: **"Signed by the Store on submission — Free"**. Every other
distribution route needs a real certificate. So producing this package adds no signing
identity, no secret, and no trust surface.

**2. An unsigned `.msix` cannot be installed.** The same guidance: *"Windows requires MSIX
packages to be signed with a valid code signing certificate."* Attaching an unsigned package
to the public GitHub release would therefore publish a file **no user can install** — worse
than not shipping it, because it looks like a download.

**3. `makeappx.exe` availability on the runner is unverified.** GitHub's Windows image lists
Windows SDK `10.0.26100.0`, and `makeappx` ships in that SDK at
`C:\Program Files (x86)\Windows Kits\10\bin\<build>\<arch>\makeappx.exe`, but the image
inventory does not enumerate it and a code search of `actions/runner-images` found nothing.
**This RFC does not assume it. R1 proves it before anything depends on it.**

## Non-goals

- **Automated Partner Center submission.** It needs Azure AD credentials as CI secrets — the
  first new secret this project would take on. A *first* submission also needs listing copy,
  screenshots, age rating and privacy answers entered by hand regardless, so automating the
  upload buys nothing yet. Revisit after the first submission exists.
- **Signing anything**, or producing a sideloadable package.
- **`.msixupload`.** `makeappx` does not create one; Partner Center accepts a plain `.msix`
  for a desktop app.
- **arm64 or a bundle.** We ship x64 only, and the manifest declares `x64`.
- **Changing what the GitHub release publishes.** The three archives stay exactly as they are.

## Decision

### D1. A sibling job, not a step inside `build`

```
notes ─┐
       ├─ build ─┬─ publish   (tag pushes only)
advisories ─┘    └─ msix      (every run, including dispatch)
```

`msix` runs on `windows-latest`, `needs: build`, downloads the **`release-Windows-x64`
artifact**, unpacks it, adds the manifest and assets, and packs.

**Why a sibling rather than a step in the Windows build leg:** `publish` declares
`needs: build`. A packaging failure inside `build` would fail that job and **block the
release of all three binaries**. As a sibling, `msix` can fail without `publish` noticing —
which is the correct priority. A Store package is an extra channel; the release is not.

**Why it repacks the published archive** rather than the raw build output: the package then
contains byte-for-byte what we shipped, and `LICENSE`/`NOTICE` come along for free, since the
archive already stages them (Apache-2.0 §4).

### D2. A workflow artifact, not a release asset

`msix` uploads `release-Windows-msix`. It is **not** attached to the GitHub release, for
Problem §2's reason: an unsigned package is not installable, and publishing one invites users
to download a file that cannot work.

The owner downloads it from the run and submits it to Partner Center, which signs it.

### D3. The version is derived, never edited

`AppxManifest.xml` carries `Version="0.0.0.0"`, a placeholder. The job substitutes
`<crate version>.0` — a four-part version whose revision **must** be `0`, which is what
Partner Center reserves.

Read from `Cargo.toml`'s workspace version, into a **copy** of the manifest in the staging
directory. **The committed manifest keeps its placeholder** and is never rewritten by CI.

Hand-maintaining it would repeat the failure that broke the 0.28.0 release: a version living
in more than one place with only one updated.

### D4. `msix` runs on dispatched runs too

`build` runs on `workflow_dispatch`, so `msix` does too. That is deliberate: it makes the
whole thing verifiable **without cutting a release**, which is how R1 gets answered and how
any future change here gets tested. It is the same property Stage 3's dry run established.

## Requirements

| # | Requirement |
|---|---|
| R1 | The job **locates `makeappx.exe` explicitly** and fails with the paths it searched if absent. **Verified on a dispatched run before any tag depends on it** — if it is missing, stop and report; do not silently install an SDK as part of this RFC |
| R2 | The package contains the shipped `knotra.exe`, `AppxManifest.xml` at its root, `Assets/`, `LICENSE` and `NOTICE` |
| R3 | The manifest's version is `<crate version>.0`, substituted into a staging copy; **`packaging/windows/AppxManifest.xml` is unchanged by CI** |
| R4 | `makeappx` runs **with** semantic validation (no `/nv`) — it checks every file the manifest references is present, which is exactly the failure this job could otherwise ship |
| R5 | The `.msix` is uploaded as a workflow artifact and **not** attached to the GitHub release |
| R6 | `publish` is untouched: its `needs`, its `if`, and its `gh release create` arguments are unchanged, and a `msix` failure cannot block it |
| R7 | No new secret, no third-party action, no signing step |
| R8 | No source file, no test, no `Cargo.toml` change. Test count stays **309** |

## Test Plan

- **A dispatched run of `release.yaml`** on `main`: `msix` succeeds, uploads one artifact,
  `publish` still skipped, the three existing archives still produced (R1, R4, R5).
- **Download the artifact and inspect it**: an `.msix` is a zip — unpack it and confirm
  `AppxManifest.xml` at the root with the substituted version, `knotra.exe`, `Assets/`,
  `LICENSE`, `NOTICE` (R2, R3).
- **Confirm `packaging/windows/AppxManifest.xml` still reads `Version="0.0.0.0"`** after the
  run (R3).
- **A deliberate failure**: temporarily point the manifest at a missing asset and confirm
  `makeappx` validation fails the job rather than producing a package (R4), then restore.
- Local gates unchanged, **309** (R8).

## Security Considerations

The reason this is small: **the Store signs what we submit**, so no certificate, no key, and
no secret enters CI. `GITHUB_TOKEN` is not needed by this job at all — it neither reads nor
writes releases.

`msix` inherits the workflow's `contents: read`. Write permission stays confined to
`publish`, unchanged.

The package ships the same binary the release ships, from the same artifact, so it introduces
no second build path that could diverge from what was tested.

## Migration / rollout

None for users. Nothing about the GitHub release changes.

The first Partner Center submission is manual and the owner's: listing copy, screenshots, age
rating and privacy answers. **If the Store identity in the committed manifest does not match
what Partner Center issued, submission fails there** — not at build, and not for users.
