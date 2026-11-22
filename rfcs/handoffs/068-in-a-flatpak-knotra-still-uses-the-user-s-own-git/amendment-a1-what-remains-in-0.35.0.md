# RFC-068 handoff — Amendment A1: what remains, in 0.35.0

Issued 2026-11-22. The owner approved Flatpak for 0.35.0 on 2026-11-22. This amendment brings the
handoff of 2026-10-21 up to date. Read the handoff first, then this.

**Start now.** This replaces the handoff's start condition. Baseline: `origin/main`. Report the
commit and its test count.

## 1. Already done: do not redo, verify only

| Handoff item | Done by | Verify and report |
|---|---|---|
| D4, the window's app ID | Task 106 | `main.rs` sets `application_id` to `io.github.nabbisen.knotra` on Linux |
| R9, the startup log's version | earlier work | `main.rs` logs `env!("CARGO_PKG_VERSION")` |
| Stage 2: D5, D6, R6, R7 (desktop entry, metainfo, icon, CI validation, the version test) | Task 106 | `packaging/linux/` holds all three; `ci.yaml` runs `desktop-file-validate` and `appstreamcli validate`; `metainfo_version.rs` exists |

**R8 is withdrawn.** The screenshots stay in `packaging/windows/screenshots/`. The metainfo
already links them at each release tag, and moving them gains nothing.

Stage 1's CHANGELOG entry ("knotra's icon on Linux desktops, and its startup log") is **not**
written. Both changes shipped already.

## 2. What remains: one stage, one review request

**From the handoff's Stage 1, exactly as written there:**

- **D1:** `host_command` in `knotra-vcs::process`, split into the pure `command_line` and the
  detection.
  - Migrate all 14 call sites. They are, at `origin/main` today:
    - `git.rs`: 31, 51, 263, 414, 492, 1024 and 1046;
    - `jj.rs`: 26, 37, 96, 310, 416 and 578;
    - `misc.rs`: 239.

    Line numbers have moved since the handoff; find each by `git grep`, and **list every site
    in the report**, with its line before and after.
  - Add `clippy.toml`'s `disallowed-methods`, with the one allow, and register it in the
    suppression map: **six**, plus any test-file allows, each registered and reported.
- **D2:** `restore_host_xdg_dirs`, the first statement of `main`, with the pure `host_xdg_dir`
  and its tests.
- **The network audit:** no knotra code path reaches gix's transport. Classify every
  endringer and gix call, by a script. A remote call is a **stop and report**.
- **The tests:** as the handoff lists them.

**The handoff's §3:** write `packaging/linux/flatpak-verification.md`, the owner's checklist, with
no manifest content.

## 3. Rule 0.1 still stands: no manifest

**Write no Flatpak manifest, anywhere.** The owner is deciding how a `.flatpak` will be built
(Review 293). Until a later amendment says otherwise, D7 holds in full.

## 4. CHANGELOG

None. D1 and D2 change nothing outside a Flatpak.

## 5. Evidence

Level 1 only: the tests, the clippy gate passing with `clippy.toml` in place, and the audit's
script and output. There are no captures: nothing on screen changes.

## 6. Report

- the verification table of §1;
- the 14 sites;
- the suppression map, with its count;
- the tests;
- the network audit;
- the checklist's path;
- gates, with the counts summed;
- knotra's status and reflog.
