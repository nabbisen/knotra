# Developer Handoff — RFC-061: build a Microsoft Store package

RFC: `rfcs/accepted/061-microsoft-store-package.md`, **accepted 2026-09-23 (project owner)**.

Baseline: `0bcbfde` (= `origin/main`, tagged `0.29.0`), **309** tests, all gates green, CI
green on all three platforms.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The one thing that must not happen

**`publish` must not change, and a packaging failure must not block a release** (R6).

`publish` declares `needs: build`. Your new job also declares `needs: build` — a **sibling**,
never a step inside the Windows build leg. If `msix` fails, `publish` neither knows nor cares.
That ordering is the whole reason the RFC puts it there; do not "simplify" it into the build
job.

Do not touch `publish`'s `needs`, its `if`, or its `gh release create` arguments. The release
that just shipped 0.29.0 is the thing you are protecting.

## 1. Find `makeappx.exe` before depending on it (R1)

**Unverified**: GitHub's Windows image lists Windows SDK `10.0.26100.0`, and `makeappx` ships
in that SDK, but the image inventory does not name it and a code search of
`actions/runner-images` found nothing.

So the job's first real step **locates it and fails loudly**:

- search `C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe`, take the
  highest-versioned match;
- also consider `C:\Program Files (x86)\Windows Kits\10\App Certification Kit\makeappx.exe`,
  documented as an alternative location;
- if none exists, **print every path searched and fail**.

**If it is absent, stop and report.** Do not add an SDK install, a `winget`/`choco` step, or a
third-party action to work around it — that is a different decision, with its own cost, and it
is not in this RFC.

## 2. The job

`windows-latest`, `needs: build`. It needs a checkout (for `packaging/windows/` and
`Cargo.toml`) **and** the built archive.

1. `actions/checkout@v7`.
2. `actions/download-artifact@v8` with `name: release-Windows-x64` — the archive `build`
   already uploads. Repacking what we shipped, rather than rebuilding, is deliberate: the
   package then contains byte-for-byte the binary users get, and `LICENSE`/`NOTICE` are
   already inside it.
3. Extract the `.zip`. **`7z` is already used by this workflow on Windows**; `unzip` is not
   guaranteed in Git Bash. Use `7z x`.
4. Stage a package directory: the extracted files **plus** `AppxManifest.xml` at its root and
   `Assets/` beside it.
5. Substitute the version (§3).
6. Pack (§4), then upload as `release-Windows-msix` via `actions/upload-artifact@v7`.

**Shell:** the workflow sets `defaults: run: shell: bash`, which on Windows is Git Bash.
Windows paths with spaces (`C:\Program Files (x86)\…`) need care there. Using
`shell: pwsh` for the steps that invoke `makeappx` is acceptable and probably cleaner — your
call, but whichever you choose, quote paths and **say in your report which you used and why**.

## 3. The version (R3)

`AppxManifest.xml` ships `Version="0.0.0.0"`. Partner Center needs a four-part version whose
**fourth part is `0`** — it reserves the revision field — so the value is
`<crate version>.0`, e.g. `0.29.0.0`.

- Read the version from `Cargo.toml`'s workspace `version` — the same single source release
  prep bumps. **Never hand-maintain it**: a version in two places with one updated is exactly
  what broke the 0.28.0 release.
- Substitute into the **staging copy only**. `packaging/windows/AppxManifest.xml` must still
  read `Version="0.0.0.0"` after the run, and your report must confirm that.
- **Anchor the substitution on the placeholder value `"0.0.0.0"`, not on `Version=`.** The
  same file has `MinVersion=` and `MaxVersionTested=` attributes; a loose pattern would
  rewrite the wrong one and produce a package that fails at submission rather than at build.

## 4. Pack with validation on (R4)

```
makeappx pack /d <staging dir> /p knotra-<version>.msix
```

**Do not pass `/nv`.** Semantic validation is documented to check that *every file the
manifest references is present in the package* — precisely the defect this job could
otherwise ship silently, since a missing tile asset is invisible until submission.

## 5. Artifact, not release asset (R5)

Upload as a workflow artifact named `release-Windows-msix`. **Do not attach it to the GitHub
release.**

Reason, from Microsoft's own signing guidance: *"Windows requires MSIX packages to be signed
with a valid code signing certificate"*, and Store submissions are *"signed by the Store on
submission"*. An unsigned package is therefore **not installable** — publishing one would
offer users a download that cannot work. The owner fetches it from the run and submits it.

## 6. Verify it without cutting a release

`build` runs on `workflow_dispatch`, so your job does too, and `release.yaml` is already
registered on `main` — the dispatch works today, no waiting.

Dispatch `release.yaml` against `main` and report:

- `msix` succeeded; **`publish` still skipped**; the three existing archives still produced;
- **unpack the `.msix` and list what is inside it** — it is a zip. Confirm `AppxManifest.xml`
  at the root **with the substituted version**, `knotra.exe`, `Assets/`, `LICENSE`, `NOTICE`
  (R2, R3);
- `packaging/windows/AppxManifest.xml` in the repo still reads `Version="0.0.0.0"`.

**Then prove the validation works** (R4): temporarily point the staging manifest at an asset
that does not exist, dispatch again, and confirm `makeappx` **fails the job** instead of
producing a package. Restore, and report both runs. A validation nobody has seen fail is not
a validation.

## 7. Out of scope

- Signing anything; `.msixupload`; a bundle; arm64.
- Automated Partner Center submission — needs Azure AD secrets, explicitly deferred.
- Any change to `publish`, to the three archives, or to what the release contains.
- Any source file, test, or `Cargo.toml` change. **309**, unchanged.
- Installing an SDK to make §1 pass.

`git diff --stat` shows `.github/workflows/release.yaml` only.

## 8. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check 0bcbfde..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

**309** (229 / 31 / 49), suppression map **five**, dead-code probe **one** line — this
handoff touches no Rust.

## 9. What to report back

- the `release.yaml` diff, and which shell you used for the `makeappx` steps and why;
- **where `makeappx.exe` was found**, and the paths your search covers;
- the dispatched run URL: `msix` result, `publish` skipped, three archives present;
- **the `.msix`'s contents, listed**, including the manifest's substituted version;
- confirmation the committed manifest still reads `Version="0.0.0.0"`;
- **the deliberate-failure run** (§6) and its error;
- gate output, gate five in the range form.
