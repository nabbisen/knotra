# Developer Handoff — RFC-060 Stage 1: the advisory gate

RFC: `rfcs/accepted/060-ci-covers-what-we-ship.md`, **accepted 2026-09-22 (project owner)**.
Covers **D1 and D2** (R1–R6, R11). Stages 2 and 3 are separate handoffs; do not touch
`ci.yaml` or `release.yaml` here.

Baseline: `854414d`. Source is identical to `feac8cf`, where I last ran the full gate set:
**303** tests, all green. Everything factual below was measured by me at that source.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. Order: Task 080 first

Task 080 (`.git-exclude/tasks/developer/080-lift-anyhow-and-crossbeam-epoch.md`) must be
**landed and reviewed** before this stage lands, or the gate arrives failing.

Before you start, confirm in `Cargo.lock`: `anyhow` **1.0.104**, `crossbeam-epoch`
**0.9.21**. If either is still at 1.0.102 / 0.9.18, **stop** — Stage 1 is not ready.

## 1. `deny.toml` at the repository root

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"
unmaintained = "all"
unsound = "all"
unused-ignored-advisory = "deny"
ignore = [
  # Each entry states the path it reaches us by and what retires it.
  { id = "RUSTSEC-2026-0253", reason = "lru 0.16.4, use-after-free in LruCache::pop(). Runtime path via cryoglyph -> iced_wgpu -> iced_renderer -> iced. Unfixable here: cryoglyph pins lru ^0.16 and has no release lifting it to >=0.18.2. Retires when cryoglyph does." },
  { id = "RUSTSEC-2024-0436", reason = "paste 1.0.15, unmaintained. Build-time proc-macro via iced's macOS path. No successor published. Retires when iced drops it or a successor ships." },
  { id = "RUSTSEC-2026-0206", reason = "rustybuzz 0.20.1, unmaintained. Runtime, iced's text shaping. No successor published. Retires when iced moves off it." },
  { id = "RUSTSEC-2026-0192", reason = "ttf-parser 0.25.1, unmaintained. Runtime, iced's font parsing. No successor published. Retires when iced moves off it." },
]
```

**No `targets` key** (RFC D1): we ship Linux, macOS and Windows, so every platform is
scanned. Narrowing to the host would hide `paste`, which is absent from a Linux build graph.

**Do not add entries for anything else.** A new advisory is a stop-and-report, not an
ignore — accepting one is an owner-visible decision.

## 2. The key-presence check (R2)

`cargo-deny`'s defaults are permissive: `unsound` defaults to direct dependencies only, and
**every** knotra dependency but 15 of 676 is transitive. A missing key is therefore a silent
hole, not a visible error. Measured: with `unsound` removed and `lru` not accepted, the scan
**passed with exit 0 and never mentioned `lru`**.

So the workflow checks presence before scanning. Plain shell, no new tool (R11):

```bash
missing=0
for key in yanked unmaintained unsound unused-ignored-advisory; do
  grep -Eq "^[[:space:]]*${key}[[:space:]]*=" deny.toml || { echo "::error::deny.toml is missing [advisories] ${key}"; missing=1; }
done
[ "$missing" -eq 0 ]
```

A commented-out key must **not** satisfy it. Measured: this form fails on both a removed key
and a `# ` commented one.

## 3. `.github/workflows/advisories.yaml`

- `permissions: contents: read`.
- Triggers: push to `main` and pull requests **touching** `Cargo.lock`, `Cargo.toml`,
  `**/Cargo.toml`, `deny.toml`, or this workflow; **`schedule`, weekly**; `workflow_dispatch`.
- Install exactly `cargo install --locked cargo-deny --version 0.20.2` (declares Rust 1.88,
  below our pinned 1.91), cached. **No third-party action and no container** (R5) — the
  published `cargo-deny-action` is a Docker image pinned only by tag.
- Steps: the §2 presence check, then
  `cargo +1.91 deny --manifest-path Cargo.toml check advisories`.
  Note `--config` is a **top-level** flag in 0.20.2, not a `check` flag; the default
  `deny.toml` at the repo root needs neither.
- No apt packages: `cargo-deny` resolves the graph and compiles nothing of ours.

Record in a comment, so nobody "fixes" it later: GitHub disables scheduled workflows in a
public repository after **60 days without repository activity**, and runs them only on the
default branch.

## 4. Prove the gate fails before trusting it (R2, R3)

Three probes, each **seen to fail**, then restored. Run them locally with a config **outside
the repo** so the committed `deny.toml` is never the thing you broke:

| Probe | Expected |
|---|---|
| remove `unsound` | presence check fails; and with `lru` unaccepted the scan would pass silently — show both |
| comment out `unsound` | presence check fails |
| add a stale entry, e.g. `RUSTSEC-2026-0194` (`quick-xml`, fixed in Task 079) | scan fails `advisory-not-detected`, naming the line |

Then the real config on the real tree: **`advisories ok`, exit 0** (R4). Quote each result.

## 5. Out of scope

- `ci.yaml`, `release.yaml`, the test matrix, the symlink test — Stages 2 and 3.
- `licenses`, `bans`, `sources` checks; `cargo audit`; any contributor-side requirement.
- Fixing or re-accepting advisories beyond §1's four.
- Any source file, `Cargo.toml`, or `Cargo.lock` change. `git diff --stat` shows
  **`deny.toml` and `.github/workflows/advisories.yaml`** only.

## 6. Verification

The five local gates unchanged (**303**, 223/31/49; suppression map five; dead-code probe
one line), plus §4's probes. `git diff --check 854414d..HEAD`.

## 7. What to report back

The `deny.toml` as committed; the workflow file; **each probe's output verbatim**, including
the silent-pass one; the clean run's output and exit code; confirmation `Cargo.lock` and
`Cargo.toml` are untouched; gate output, gate five in the range form.
