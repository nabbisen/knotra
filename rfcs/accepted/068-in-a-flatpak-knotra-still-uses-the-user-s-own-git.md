# RFC-068 - In a Flatpak, knotra still uses the user's own Git and Jujutsu

| Field | Value |
|---|---|
| Status | Accepted |
| Priority | Medium - prerequisite for publishing on Flathub |
| Effort | Medium - one process helper and 14 call sites, one startup step, Linux metadata files, CI validation |
| Target | the release after the Flathub question (§Owner inputs) is answered |
| Related files | `crates/knotra-vcs/src/vcs/{git,jj}.rs`, `crates/knotra-vcs/src/lib.rs`, `crates/knotra-app/src/{main,app/misc}.rs`, new `packaging/linux/`, `packaging/windows/store-listing.md`, `.github/workflows/ci.yaml` |
| Decided by | project owner, 2026-10-21 - "Yes. Approved." Icon: the owner asked the architect to make it (Owner input 1) |
| Decided by | project owner, 2026-10-21 - "Yes. Approved." Owner input 1: the owner asked the architect to make the icon |
| Related | `.git-exclude/reviewed/227-flatpak-assessment.md` (the research and its corrections); RFC-064 (version kept in step with the crate, the model for D6's test) |

## Summary

knotra promises that "it works through the Git and Jujutsu already installed on your
computer, so your existing settings and credentials keep working as they are". Inside a
Flatpak sandbox, as knotra is built today, that promise breaks. The sandbox has no `git` or
`jj`, and it moves the user's settings folders.

This RFC makes knotra keep the promise when it runs in a Flatpak:

- every outside program (Git, Jujutsu, the user's editor and merge tool) runs on the user's
  own system, through `flatpak-spawn --host`;
- knotra reads the same settings folders the user's own Git reads.

Outside a Flatpak, nothing changes. The RFC also adds the files a Linux software centre
reads (desktop entry, AppStream description, icon), validated in CI.

**What this RFC does not do, by Flathub's policy:** write the Flatpak manifest, or open
or handle any Flathub submission. Those are the owner's own work (§D7).

## Problem

Measured at `71eb43a`; Flatpak facts from https://docs.flatpak.org (cited in Review 227).

1. **No `git` or `jj` inside.** The freedesktop Platform runtime ships neither. knotra
   starts them by name at **14 places**: 7 in `knotra-vcs/src/vcs/git.rs`, 6 in `jj.rs`, and
   the editor or merge tool in `knotra-app/src/app/misc.rs:213`. In a sandbox, each fails, or
   finds a bundled copy that knows nothing of the user's setup: credential helpers, hooks,
   `~/.ssh`, signing, editor.
2. **The settings folders move.** Flatpak sets `XDG_CONFIG_HOME` (and `XDG_DATA_HOME`,
   `XDG_CACHE_HOME`, `XDG_STATE_HOME`) to folders under `~/.var/app/<id>/`, overriding the
   host's. knotra reads some repository state in-process, through `endringer-git` (gix).
   Those reads would miss the user's `~/.config/git/config` and `~/.config/git/ignore`, so
   knotra could show a file as changed that the user's own `git status` ignores. knotra's own
   settings would also live in a different place than in the tar.gz build.
3. **The desktop does not know knotra.** No desktop entry, no AppStream description, no
   icon larger than 150×150, and no Wayland app ID on the window. A software centre needs
   all four; a Wayland desktop needs the app ID to match the window to its icon.
4. **The startup log says `knotra v0.2.0 starting`** (`main.rs`), whatever the version.

## Design

### D1. One way to start an outside program

A new public function in `knotra-vcs`:

```rust
/// Builds the command that runs `program` on the user's own system, in `cwd`.
/// Inside a Flatpak it goes through `flatpak-spawn --host`, so the user's own
/// installation, settings and credentials apply; elsewhere it is the program itself.
pub fn host_command(program: &str, cwd: Option<&Path>) -> std::process::Command
```

- **Outside a Flatpak:** `Command::new(program)`, with `current_dir(cwd)`. This is exactly
  today's behaviour.
- **Inside a Flatpak:** `flatpak-spawn --host --watch-bus [--directory=<cwd>] <program>`.
  `--watch-bus` ends the child if knotra exits. The caller adds arguments as before.
  `flatpak-spawn` passes stdout, stderr and the exit status through, so every caller's
  handling of the output stays as it is.
- **Detection:** once, cached. knotra is in a Flatpak when `/.flatpak-info` exists, the file
  Flatpak creates in every sandbox.
- **All 14 call sites use it.** A `clippy.toml` `disallowed-methods` entry for
  `std::process::Command::new` makes any new direct call fail the lint gate. The helper
  carries the one allowed use. The suppression map grows from five to six, and the handoff
  records why.

knotra passes no environment variables, temporary files or stdin to these commands
(checked at `71eb43a`), so nothing else needs forwarding.

### D2. The same settings folders as the user's Git

At the very start of `main`, before any thread exists, and only inside a Flatpak: set
`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME` back to the host's
values. Flatpak exposes these as `HOST_XDG_*` when the host set them; otherwise they are the
XDG defaults under `$HOME` (`~/.config`, `~/.local/share`, `~/.cache`, `~/.local/state`).

- In-process reads (gix) then see the same `~/.config/git` files the user's `git` sees.
- knotra's own settings and data live in the same place as in the tar.gz build. A user who
  switches between the two keeps their workspaces. Nothing to explain, nothing to migrate.
- **Trade-off:** this departs from Flatpak's per-app folders. Uninstalling the Flatpak
  leaves `~/.config/knotra`, exactly as removing the tar.gz build does. That is the honest
  consequence of "knotra uses your own setup".

`std::env::set_var` needs `unsafe` in edition 2024. The one block sits first in `main`,
with a comment stating why it is sound: no other thread exists yet.

### D3. What a Flatpak of knotra needs from the sandbox

This RFC writes no manifest (D7). It does fix what the app requires, so the owner's manifest
and exception request can state it exactly:

| Permission | Why knotra needs it |
|---|---|
| `--filesystem=host` | projects can be anywhere the user keeps them; gix reads them in-process; the paths knotra stores must be the same inside and outside the sandbox |
| `--talk-name=org.freedesktop.Flatpak` | `flatpak-spawn --host` (D1) |
| `--socket=wayland`, `--socket=fallback-x11`, `--device=dri` | the window and wgpu rendering |

**Not needed:**

- `--share=network`. knotra's own code makes no network connection. Fetch, pull and push go
  through `git` and `jj` (`git.rs:722`, `jj.rs:449`), which D1 runs on the host. `gix`'s
  transport crates are in the dependency tree, so the handoff confirms that no knotra code
  path reaches them. If one does, it is routed to the host's `git`, so the row stays out.
  A Flatpak of knotra with no network permission is a true statement of "it never sends
  your data anywhere".
- `--socket=ssh-auth`, `gpg-agent`: Git runs on the host, where those already work.

### D4. The window's app ID

On Linux, `window::Settings::platform_specific.application_id = "io.github.nabbisen.knotra"`.
That is the Wayland app ID and the X11 class, matching the desktop entry's name, so the
desktop shows knotra's icon on its window. This holds for every Linux build, not only the
Flatpak. Harmless where no desktop entry is installed.

### D5. Linux desktop files, in `packaging/linux/`

- `io.github.nabbisen.knotra.desktop`: name, comment, `Exec=knotra`,
  `Icon=io.github.nabbisen.knotra`, `Categories=Development;RevisionControl;`, and
  `StartupWMClass` matching D4.
- `io.github.nabbisen.knotra.metainfo.xml`:
  - id, name and summary;
  - the description: the English Store listing's text, as approved;
  - `developer id="io.github.nabbisen"`;
  - `project_license` Apache-2.0, `metadata_license` CC0-1.0;
  - `launchable`;
  - `url` homepage and bugtracker;
  - OARS content rating with no content;
  - screenshots and releases, as below.
- **Screenshots:** the five English Store screenshots, linked by raw GitHub URL **at the
  release tag**, as Flathub requires. They move from `packaging/windows/screenshots/` to
  `packaging/screenshots/`, since both stores now use them. `store-listing.md` follows. The
  privacy rule moves with them: commit only after the privacy check.
- **Releases:** one `<release version date>` per release, its description the CHANGELOG
  lead line.
- **Icon:** `packaging/linux/icons/io.github.nabbisen.knotra.svg`, or a PNG of at least
  256×256, from the owner's artwork (Owner input 1).

**CI** (`ci.yaml`, Linux job): `desktop-file-validate` and
`appstreamcli validate --strict --no-net`. Both are Ubuntu packages.

### D6. Release prep keeps the metainfo in step

A test like RFC-064's `store_manifest_version`: the newest `<release>` in the metainfo
equals the crate version. Release prep then touches five places: `Cargo.toml` lines 10, 20
and 21, the AppxManifest, and the metainfo. The test is excluded from the crate package, as
the AppxManifest test is.

### D7. Kept apart, by Flathub's policy

Flathub's requirements say the manifest "must not contain AI-generated or AI-assisted
content", and that AI must not open or handle submission pull requests, or write their
messages or replies. So:

- **The architect and dev tiers write no Flatpak manifest**, not even a local test one. A
  draft the owner might start from would make the owner's manifest AI-assisted.
- **The owner** writes the manifest, makes the submission, requests the exceptions (D3's
  table gives the facts to state), and makes the AI disclosure. Each later release's Flathub
  update is also the owner's (release checklist).
- **Verification** of D1 and D2 in a real sandbox happens with the owner's own build. The
  handoff gives a checklist the owner can run in minutes; the dev tier's tests cover
  everything up to that point.

## Requirements

- R1. `host_command` as in D1; outside a Flatpak it builds the same `Command` as today
  (tested: program, arguments, working folder).
- R2. Inside a Flatpak, it builds `flatpak-spawn --host --watch-bus --directory=<cwd>
  <program> <args…>`. Tested with detection injected: the check is a pure function of
  "is `/.flatpak-info` present".
- R3. No `std::process::Command::new` outside `host_command`; enforced by clippy.
- R4. D2's restore runs first in `main`, only in a Flatpak. The folder choice is a pure,
  tested function of the `HOST_XDG_*` and `HOME` values.
- R5. D4's app ID on Linux.
- R6. D5's files. CI validates them: `desktop-file-validate`, `appstreamcli validate
  --strict`.
- R7. D6's test, and release prep's checklist names the fifth place.
- R8. The screenshots move to `packaging/screenshots/`; every reference follows.
- R9. The startup log line uses the real version (`env!("CARGO_PKG_VERSION")`).
- R10. No Flatpak manifest anywhere in the repository or `.git-exclude/`.
- R11. CHANGELOG `[Unreleased]`: one entry, for D4 (the icon on Linux desktops) and R9. D1
  and D2 change nothing outside a Flatpak.

## Owner inputs

1. **Icon artwork:** an SVG of knotra's logo, or a PNG of at least 256×256. The Windows
   tiles are 150×150 at most, so upscaling them would be blurry. The source of the existing
   logo is yours.
2. **Before the handoff, your question to Flathub** (Review 227 recommends asking first,
   because a refused exception can be refused permanently). Whether knotra can be granted
   D3's `host` and `org.freedesktop.Flatpak` exceptions, given the AI disclosure. While you
   ask: whether its Japanese UI text, drafted by AI and approved by you, meets the requirement
   that translations are not "machine-generated". Both are their rules to interpret. I
   state them here only so you can ask about them; I am not predicting the answer.

## Out of scope

The manifest, the submission and every Flathub interaction (D7); `.deb`, `.rpm`, Snap; a
Linux aarch64 build (Flathub builds aarch64 by default; whether to opt out belongs in the
owner's manifest).

## Risks

- **`flatpak-spawn` environment.** The host command gets the Flatpak session helper's
  environment, not the user's shell's. `SSH_AUTH_SOCK` is normally in the session. A
  variable set only in a shell profile is not. The owner's checklist covers an SSH fetch.
- **D2's `unsafe`.** It is sound only before threads start. It is the first statement of
  `main`, and a comment says so.
- **gix and `--filesystem=host`.** Some host paths stay hidden even with `host` (`/usr`,
  `/etc`). A project kept there would not open in the Flatpak; that is accepted.
