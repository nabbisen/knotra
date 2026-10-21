# Developer Handoff — RFC-068: in a Flatpak, knotra still uses the user's own Git and Jujutsu

RFC: `rfcs/accepted/068-in-a-flatpak-knotra-still-uses-the-user-s-own-git.md`, **accepted
2026-10-21 (project owner).** Read it in full first, especially D7.

**Start after RFC-066 Stage 3 is approved.** Baseline: `origin/main` at that point; report
the commit and its test count.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. Two rules before anything else

1. **Write no Flatpak manifest**, in the repository, in `.git-exclude/`, or anywhere else, and
   no Flathub submission text. RFC D7 explains why: Flathub's policy, and the owner's own
   submission depends on it. If you find you cannot verify something without a manifest,
   **stop and report**. The owner verifies in a real sandbox with their own build (§3).
2. **Outside a Flatpak, nothing may change.** Every existing test must pass unmodified,
   except where this handoff names the change.

Two stages, two review requests.

## Stage 1 — the code (D1, D2, D4, R9)

**D1, `host_command`.** In `knotra-vcs`, a new module `process`, exported from `lib.rs`:

- `pub fn host_command(program: &str, cwd: Option<&Path>) -> Command`, as RFC D1.
- Split for testing:
  - a pure `fn command_line(in_flatpak: bool, program, cwd) -> (String, Vec<String>, Option<PathBuf>)`
    returns the program, the leading arguments, and the working folder to set;
  - `host_command` calls it with the cached detection.
- Inside: `flatpak-spawn` with `["--host", "--watch-bus", "--directory=<cwd>", program]`, and
  no working folder (`flatpak-spawn` sets it on the host). Outside: `program`, no leading
  arguments, `cwd` as the working folder.
- Detection: `Path::new("/.flatpak-info").exists()`, in a `OnceLock`.
- Doc comment: what the function guarantees to the caller, in user terms (the user's own
  installation runs), not the mechanism alone.

**Migrate all 14 call sites:** `knotra-vcs/src/vcs/git.rs` (7), `jj.rs` (6),
`knotra-app/src/app/misc.rs:213` (1). Each keeps its arguments and its handling of the
output. **List every site in the report**, with its line before and after.

**Enforce it:** a workspace `clippy.toml`:

```toml
disallowed-methods = [
  { path = "std::process::Command::new", reason = "use knotra_vcs::process::host_command, so a Flatpak build runs the user's own git, jj and tools (RFC-068 D1)" },
]
```

Then `#[allow(clippy::disallowed_methods)]` on the one line in `host_command`. Add it to the
suppression map, which goes from five to **six**, with RFC-068 as its reason. Tests that
start processes directly (`knotra-vcs/tests/git_integration.rs` builds fixture repositories
with `git`) may carry a file-level allow. Register those in the map too, and report each.

**D2, settings folders.** In `knotra-app/src/main.rs`, as the **first statement** of `main`:

- `knotra_vcs::process::restore_host_xdg_dirs()` (or a function in `knotra-app`; your
  choice, reported).
- It does nothing outside a Flatpak.
- Inside, it sets the four `XDG_*_HOME` variables to the folder a pure
  `fn host_xdg_dir(var, host_value: Option<&OsStr>, home: &Path) -> PathBuf` chooses: the
  `HOST_XDG_*` value if set and absolute, else the XDG default under `home`.
- One `unsafe` block around the four `set_var` calls. Comment: sound because it runs before
  any other thread exists, and it is the first statement of `main`.

**D4.** In `main.rs`, under `#[cfg(target_os = "linux")]`, set
`window::Settings::platform_specific` with `application_id: "io.github.nabbisen.knotra".into()`.
Check the field name against iced 0.14's `window::settings::PlatformSpecific` for Linux,
and report what you found.

**R9.** The startup log line becomes `knotra v{} starting` with `env!("CARGO_PKG_VERSION")`.

**Network (RFC D3, "Not needed"):** show that no knotra code path reaches `gix`'s network
transport.

1. List every `endringer`/`gix` call `knotra-vcs` makes (a script over the source, not by
   eye).
2. Classify each as local (refs, status, objects, config) or remote (connect, fetch,
   `ls-remote`, push).
3. Expected: no remote. If one exists, **stop and report**; do not route it yourself.

**Tests (Stage 1):**

- `command_line(false, …)`: equals today's program, no leading arguments, the working folder;
- `command_line(true, …)`: the exact `flatpak-spawn` argument list, with and without a
  working folder;
- `host_xdg_dir`: a set absolute `HOST_XDG_*`; unset; set but relative (treated as unset,
  per the XDG spec), for each of the four;
- clippy proves the migration: with `clippy.toml` in place, the gate fails on any
  remaining `Command::new`. Report the gate passing.

**CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Fixed — knotra's icon on Linux desktops, and its startup log

On Linux, the window now identifies itself as `io.github.nabbisen.knotra`, so a desktop
that has knotra's desktop entry installed shows knotra's icon for its window. The startup
log line also showed an old version number; it now shows the real one.
```

## Stage 2 — the desktop files (D5, D6, R6–R8)

**Wait for `packaging/linux/icons/io.github.nabbisen.knotra.svg` to be on `main`.** The
architect commits it after the owner approves it. If it is not there when you reach this
stage, **stop and report**.

**R8, the screenshots move.** `git mv packaging/windows/screenshots packaging/screenshots`.
Update every reference: `store-listing.md`, any review-free doc, and the privacy rule's text.
`git grep -n 'windows/screenshots'` must print nothing outside `.git-exclude/`.

**D5.** In `packaging/linux/`:

- `io.github.nabbisen.knotra.desktop`, with the keys of RFC D5:
  - `Name=knotra`;
  - `Comment=` the English short description of `store-listing.md`, first sentence;
  - `Keywords=git;jj;jujutsu;repository;`.
- `io.github.nabbisen.knotra.metainfo.xml`, with the elements of RFC D5:
  - **Description:** the English description from `store-listing.md`, as approved,
    paragraph by paragraph. Do not reword it.
  - **Screenshots:** the five English images, by
    `https://raw.githubusercontent.com/nabbisen/knotra/<version>/packaging/screenshots/en/<file>`,
    with the English captions from `store-listing.md`. `<version>` is the crate version.
  - **Releases:** one `<release version="<crate version>" date="<its CHANGELOG date>">`,
    its description the CHANGELOG lead line of that version.
- The icon is the architect's file; do not edit it.

**D6.** A test in `knotra-app/tests/linux_metainfo_version.rs`, excluded from the package
like `store_manifest_version.rs`:

- the newest `<release version>` equals the crate version;
- every screenshot URL's `<version>` segment equals the crate version.

From then on, release prep edits the metainfo: one new `<release>`, and the screenshot
URLs' tag. Add that to the release-prep wording in your report, so I can update the
checklist.

**CI.** In `ci.yaml`'s Linux job, install `desktop-file-utils` and `appstream`, then run:

```
desktop-file-validate packaging/linux/io.github.nabbisen.knotra.desktop
appstreamcli validate --strict --no-net packaging/linux/io.github.nabbisen.knotra.metainfo.xml
```

Run both locally too and show the output. Fix the files until both pass. Do not loosen a
check.

## 3. The owner's verification checklist — write it, do not run it

Write `packaging/linux/flatpak-verification.md` for the owner to run in their own Flatpak
build. It holds **no manifest content**. Each step says what to do and what must be seen:

1. Open a project anywhere under the home folder; status matches `git status` in a
   terminal.
2. With a pattern only in `~/.config/git/ignore`, a matching file is not shown as changed
   (D2).
3. "Check for updates" over SSH, and over HTTPS with a credential helper: both succeed
   with no prompt that a terminal would not show.
4. Save a release point; the tag's tagger is the user's own `user.name` and `user.email`.
5. "Open in editor" opens the configured editor.
6. knotra's settings are those of the tar.gz build (same workspaces).
7. The Flatpak has no network permission, and steps 1–5 still work.
8. The window shows knotra's icon in the dock or taskbar.

## Out of scope

The manifest, the submission and every Flathub interaction. `.deb`, `.rpm`, Snap. A Linux
aarch64 build.

## Verification, both stages

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <stage baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
```

Suppression map **six** after Stage 1, plus any test-file allows you registered and
reported. Dead-code probe **one** line. `advisories ok`. Explain each test count delta.

## What to report back

- Stage 1:
  - each migrated site, before and after;
  - `clippy.toml` and the suppression map;
  - the D2 function and its `unsafe` comment;
  - the D4 field, as iced names it;
  - the network classification table;
  - tests;
  - the CHANGELOG entry as committed.
- Stage 2:
  - the moved paths and the `git grep` result;
  - both files;
  - the validators' output;
  - the D6 test;
  - the release-prep wording;
  - the checklist.
- Both: gate output, with the `diff --check` range.
