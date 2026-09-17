# RFC-059 - Upgrade snora from 0.38 to 0.49

| Field | Value |
|---|---|
| Status | Accepted 2026-09-15 (project owner) |
| Priority | High - knotra carries two live input defects that 0.41.0 fixes |
| Effort | Small - one manifest line, the lockfile, two comments, one CHANGELOG entry, evidence |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `Cargo.toml`, `Cargo.lock`, `crates/knotra-app/src/view.rs`, `crates/knotra-ui/src/theme.rs`, `CHANGELOG.md` |
| Related RFCs | `rfcs/done/056-...md` Stage 1 (the 0.38 bump this repeats), `rfcs/done/058-...md` (whose comments this re-verifies), `rfcs/done/055-...md` D3 (verify against the running binary, not by reasoning) |
| Decided by | project owner, 2026-09-15 - upgrade approved as the first of five decisions |

## Summary

knotra is on snora **0.38.0**. The latest is **0.49.0**. Upgrading fixes two input
defects knotra has today, and was measured before this RFC was written — not inferred
from snora's release notes:

- it **compiles against 0.49 with zero errors**, clippy clean;
- **all 303 tests pass unchanged**;
- the lockfile moves **exactly eleven packages** — five snora crates up, six unused
  graphics packages out — and nothing else;
- **every snora file knotra renders through** was diffed 0.38 → 0.49: each change is
  documentation or a refactor with identical values. No colour, size, or layout knotra
  draws moves.

The only behaviour that changes for a knotra user is the fix itself.

## Problem

### Two defects knotra has on 0.38.0

knotra wires snora's close handler for modals (`on_close_modals`, `view.rs`), so a
click on the dim closes the open dialog. On 0.38.0 that interacts badly with two gaps in
snora's overlay rendering, verified in the source of both versions:

| | 0.38.0 | 0.49.0 |
|---|---|---|
| Dialog content | `center(dialog.content)` — not opaque | `center(opaque(dialog.content))` |
| Dim, pointer | `mouse_area(dim).on_press(close)` | `.on_press(close).on_scroll(...)` |
| Dim, container | plain `container` | wrapped in `opaque` |

What a knotra user experiences on 0.38.0:

1. **Clicking empty space or plain text inside a dialog closes it.** The click is not
   captured by the dialog, falls through to the dim, and the dim dispatches Close. Any
   text typed into the dialog is lost with it.
2. **The scroll wheel over a dialog scrolls the screen behind it.** The dim handles
   presses but not scroll, so scroll events pass through.

This affects all five `Dialog` surfaces (workspace manager, Pull, Tag, Switch,
Changelog). The resolve `Sheet` already contained pointer input at 0.38.0.

**Not a confirmation bypass, for knotra specifically.** snora's 0.41 letter warned that a
click could pass *through* a confirmation to the control it guards. That happens only for
an application with **no** close handler on the dim. knotra has one, so the dim captures
the click. The real exposure is accidental dismissal and scroll leakage — quieter, but
real, and it can discard a user's input mid-task.

### What was measured, and how

| Check | Result |
|---|---|
| `cargo check --workspace --all-targets` at 0.49 | 0 errors, 0 warnings |
| `clippy --workspace --all-targets -D warnings` at 0.49 | clean |
| tests at 0.49 | 223 + 31 + 49 = **303**, all pass, identical to 0.38 |
| lockfile movement | `snora`, `snora-core`, `snora-design`, `snora-style`, `snora-widgets` 0.38.0 → 0.49.0; removed `float_next_after`, `lyon`, `lyon_algorithms`, `lyon_geom`, `lyon_path`, `lyon_tessellation`; **nothing else** |
| `windows` / `gpu-allocator` / `wgpu-hal` | unchanged — no downward re-resolution (the Windows-only break another snora consumer hit at 0.42) |
| `cargo check --target x86_64-pc-windows-gnu` | passes at 0.38 **and** at 0.49 |
| minimum Rust | snora 0.49 = 1.88, iced 0.14 = 1.88, knotra = 1.88 — **unchanged** |
| iced | stays **0.14.0** |

All in a scratch copy of the tree, not the working tree.

### Breaking changes across all eleven migration guides

snora publishes a guide for every minor release; all eleven from 0.38→0.39 through
0.48→0.49 were read. Only two changes are breaking at compile time, and neither reaches
knotra:

| Release | Break | knotra |
|---|---|---|
| 0.40.0 | `lucide-icons` no longer enables iced's `advanced` transitively | declares `advanced` itself (`Cargo.toml`) |
| 0.45.0 | `snora::design::{Emphasis, Size}` removed | uses neither — verified by enumerating every `snora::design` import |

### Appearance changes, checked in source rather than trusted from the guides

Every snora file whose output knotra renders was diffed between the two versions:

| File | Change |
|---|---|
| `snora-style`: `button.rs`, `container.rs` (holds `card_raised`), `theme.rs` | documentation link formatting only |
| `snora-style`: `text.rs` | documentation only |
| `snora-widgets`: `design/card.rs`, `design/chip.rs` | documentation only |
| `snora-widgets`: `design/notice.rs` | refactor into named functions with **identical** colours — `Tone::Neutral` still maps to `border`, action label still uses it |
| `snora-design`: `typography.rs` | one doc comment; no size or line-height value moved |
| `snora-design`: `presets/light.rs`, `presets/dark.rs`, `contrast.rs` | unchanged |
| `snora`: `overlay/sheet.rs` | panel style unchanged (`background.weak` border); one internal helper extracted |

0.41.0 also repaired colours in snora's menu, sidebar, tab, and breadcrumb widgets.
**knotra uses none of them.** 0.39.0 corrected snora's documentation about a dialog
card's edge against the dim; **knotra makes no such claim** anywhere in code or RFCs.

## Non-goals

- **Finishing overlay keyboard focus, F6 region cycling, high-contrast themes, CI
  coverage.** Each is its own RFC, written against 0.49.
- **Any visual change.** None is expected, and none is wanted here.
- **Upgrading iced or `lucide-icons`**, or bumping knotra's own version. Release timing is
  the owner's.
- **The `lru` advisory.** It reaches knotra through iced, not snora, and no snora version
  changes it.

## Alternatives considered

**Stop at 0.41.0**, the first release carrying the fix. Rejected: 0.42.0–0.49.0 were
measured and add nothing that breaks knotra, and the four RFCs that follow are written
against 0.49. Stopping short means repeating this measurement later for no gain.

**Wait.** Rejected: both defects are live, and the upgrade is measured as safe.

## Decision

### D1. Bump to 0.49.0

`Cargo.toml`: `snora = { version = "0.49", ... }`, then `cargo update -p snora`. The
lockfile must move exactly as measured above.

### D2. The fix is proven in the running app, not assumed

Nothing in the test suite can observe pointer routing between overlay layers. The fix is
trusted only once seen, on both versions, following
`.git-exclude/reference/002-keyboard-evidence-runbook.md` — bare absolute-coordinate
`xdotool`, compositor IPC for focus and capture, an isolated config environment.

Three observations, each captured at 0.38 and at 0.49:

1. type into a dialog, click a non-interactive area inside it — **0.38 closes and loses
   the text; 0.49 keeps both**;
2. scroll over an open dialog with scrollable content behind — **0.38 scrolls the
   background; 0.49 does not**;
3. click the dim outside the dialog — **closes in both**. This guards the fix from
   breaking knotra's own close route.

### D3. Two comments pinned to 0.38.0 are re-verified and re-cited without line numbers

Current claims, not history:

- **`view.rs`, the `ActiveModal::Resolve` arm** cites
  `snora-core-0.38.0/src/overlay.rs:206-227`. At 0.49.0 `Sheet` still has only
  `new`/`at`/`with_size`, but those lines are now 214–232 — the drift RFC-058's follow-up
  warned about. Re-cite by symbol and version.
- **`theme.rs`, `with_knotra_typography`** says the `body_small` override was "verified
  against the 0.38.0 source of all four snora crates". **snora has five crates** at both
  0.38.0 and 0.49.0. Re-verify all five at 0.49.0 and state what was checked.

The Sheet's measured 1.29–1.35:1 edge stays correct: its style and the palette it reads
are unchanged.

Historical records — `notice.rs`'s "re-measured after the snora 0.38 bump", `theme.rs`'s
RFC-056 Stage 1 figures — describe past events and are left as written.

### D4. A CHANGELOG entry, in user terms

Under `[Unreleased]`: clicking inside a dialog no longer closes it, and scrolling over a
dialog no longer scrolls the screen behind. Prior snora upgrades each got an entry.

### D5. Windows is checked as far as this machine can check it

Our CI builds only Linux, but we ship Windows. Until the CI RFC closes that, this upgrade
carries its own check: no `windows` package moves down in the lockfile, and the workspace
compiles for `x86_64-pc-windows-gnu`. `msvc` is not installed locally; the `gnu` target
compiles the same Rust in the `windows` crates, which is where the known break occurred.

## Requirements

| # | Requirement |
|---|---|
| R1 | `snora` manifest version is `0.49`; `cargo update -p snora` moves **exactly** the five snora crates to 0.49.0 and removes the six packages listed above. Any other movement: **stop and report** |
| R2 | No knotra source change beyond D3's two comments. If compiling at 0.49 needs one, **stop and report** — the measurement says it does not |
| R3 | All gates pass; tests **303** exactly (223 / 31 / 49); suppression map **five**; dead-code probe **one** line |
| R4 | D2's three observations captured at **both** 0.38 and 0.49, with the dialog used named. If a synthetic scroll cannot be delivered, **report the tooling gap** rather than substituting a different observation |
| R5 | No `windows` package version decreases in `Cargo.lock`; `cargo check --workspace --target x86_64-pc-windows-gnu` passes |
| R6 | D3's comments re-verified at 0.49.0 and re-cited by symbol and version, **no line numbers**. If a re-verified figure differs, **stop and report** |
| R7 | One `[Unreleased]` CHANGELOG entry per D4 |
| R8 | iced stays 0.14.0; `rust-version` stays 1.88; `crates/knotra-vcs` untouched |

## Test Plan

- Full gate set at 0.49.
- Lockfile audit against R1 and R5, from `Cargo.lock` itself, not command output.
- `cargo check --workspace --target x86_64-pc-windows-gnu`.
- D2's running-app evidence at both versions, per the runbook.

## Security Considerations

Pointer containment is a UI-integrity property: an overlay that lets input fall through
can act on something the user did not intend. This upgrade restores it.

No dependency is added; six are removed. No advisory changes — the four fixable ones are
already cleared, and `lru` is unaffected. snora forbids `unsafe` code in all five crates
since 0.47.0.

## Migration / rollout

None for users beyond the fix. Configuration, data, and public API are untouched. No
release is bundled.

## Amendments

### A1 — retarget from 0.49.0 to 0.50.0 (2026-09-17, before implementation began)

snora 0.50.0 was released after this RFC was accepted. No knotra commit toward RFC-059
existed — verified, none after `d2727cf`. Retargeting now avoids a second upgrade cycle for
a release measured to be inert for knotra.

**Measured at `d2727cf`, in a scratch copy, the same way as the original measurement:**

| Check | Result at 0.50.0 |
|---|---|
| lockfile movement | **identical to 0.49**: the five snora crates 0.38.0 → 0.50.0; the same six removals; nothing else |
| `windows` / `gpu-allocator` / `wgpu-hal` | unchanged |
| `anyhow` / `crossbeam-epoch` | unchanged — snora's own `anyhow` bump does not reach our lockfile |
| check / clippy `-D warnings`, all targets | clean |
| tests | 223 + 31 + 49 = **303**, all pass |
| `cargo check --target x86_64-pc-windows-gnu` | passes |
| minimum Rust | 1.88, unchanged |

**Source diff, 0.49.0 → 0.50.0.** `snora-core`, `snora-design` and `snora-style` are
byte-identical. Three files changed: `snora/src/lib.rs` (a version string in a doc example),
and in `snora-widgets` `sidebar.rs` plus `design/widget.rs` (the sidebar rail's button
padding, snora RFC-099). **knotra uses neither the sidebar nor `design::widget` chrome**; the
one snora widget module knotra imports, `snora::widget::icon`, is unchanged. knotra does
not use `iced_test`, the one other item 0.50's guide flags.

**D3's two claims re-verified at 0.50.0:** `Sheet` still has only `new`/`at`/`with_size`
(same lines as 0.49.0); its panel border is still `background.weak`; `body_small` is still
read only by `snora-style`'s `text.rs` definitions and its `#[cfg(test)]` module.

**Effect.** Every `0.49` / `0.49.0` in D1, D2, D3, D5, R1, R4, R5 and R6 reads `0.50` /
`0.50.0`. No decision, requirement, stop condition, or scope changes. The file keeps its
name, so existing citations stay valid.
