# RFC-053 - Three suppressions removed

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | Medium - hygiene, on evidence already gathered |
| Effort | Small - two deletions, one newtype, seven call sites |
| Target | Production Readiness Reset - operational hygiene |
| Related files | `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/app/freezer.rs`, `crates/knotra-ui/src/widget/overlay.rs`, `crates/knotra-app/src/suppressions_guard.rs`, `CHANGELOG.md` |
| Related RFCs | `rfcs/done/052-...md` (the guard and inventory this shrinks), `rfcs/done/051-...md` (whose signature this revises before release) |
| Owner decisions | All three items approved 2026-08-13 |

## Summary

RFC-052's inventory has eight suppressions. Three can go: two are provably inert, and the
third exists only because of a menu I wrote badly. **Eight to five.**

## Problem

### Two suppress nothing

Verified twice — by the implementer in Handoff 071 §3, and independently at review by
forcing each lint:

| Site | Attribute | Forced result |
|---|---|---|
| `state/palette.rs:19` | `clippy::large_enum_variant` | **fires** at `:27` — keep |
| `message.rs:14` | `clippy::large_enum_variant` | does not fire — **inert** |
| `app/freezer.rs:15` | `unreachable_patterns` | does not fire — **inert** |

`palette.rs` firing is what makes the two silences evidence rather than absence of
evidence: the method demonstrably detects a live suppression.

Handoff 071 correctly declined to remove them — lint changes were out of that RFC's
scope, and proof that something is inert is not authority to delete it. This RFC is that
authority.

### The third exists because of a bad menu

RFC-051 D3 offered two shapes for `surface()`: keep `OverlayWidth` and add a width
parameter, or resolve at the call site and pass a bare `f32`. The first was chosen for
enforcement — a call site cannot pass `742.0` where a variant belongs — and it pushed the
function to **eight** parameters, requiring this codebase's first
`#[allow(clippy::too_many_arguments)]`.

**A third shape has both properties and was not offered:**

```rust
pub struct ResolvedWidth(f32);   // field private

impl OverlayWidth {
    pub fn resolve(self, available: f32) -> ResolvedWidth { … }
}

pub fn surface(tokens, width: ResolvedWidth, title, on_close, is_close_focused, body, footer)
```

Seven parameters — no allow — and `742.0` is still unpassable, because `ResolvedWidth`'s
only constructor is `OverlayWidth::resolve`.

## Non-goals

- `knotra-vcs`'s `tag_exists`. It stays in the map with Handoff 071's inference-flagged
  comment. Its own question remains open and separate.
- The three `cfg_attr(test, allow(dead_code))` attributes from RFC-052 A1. They are
  justified and load-bearing for the test target.
- `state/palette.rs`'s `large_enum_variant`. It fires.
- The conflict row's layout - a separately approved item, deliberately not bundled here
  so that accepting suppression removals does not imply accepting a layout change.

## Decision

### D1. Remove the two inert attributes

`message.rs:14` and `app/freezer.rs:15`. The gate must stay clean at
`--workspace --all-targets -D warnings`, which is the check that matters and the one an
earlier measurement of mine skipped.

### D2. `ResolvedWidth` replaces the eighth parameter

`surface()` takes `width: ResolvedWidth` and drops `available`. `OverlayWidth::pixels`
stays private; `resolve` is its only public route.

Seven call sites pass `OverlayWidth::<Variant>.resolve(state.window_width)`. The compiler
finds them all.

**No arithmetic changes.** RFC-051's fractions, floors and ceilings are correct and stay
exactly as they are; this moves where the result is wrapped, nothing else.

### D3. `#[allow(clippy::too_many_arguments)]` goes with it

Along with the comment justifying it, which stops being true.

### D4. The CHANGELOG entry is revised, not appended to

RFC-051 added an `## [Unreleased]` entry describing `surface()` gaining `available: f32`.
This RFC removes that parameter again. **Both changes are unreleased** — 0.27.0 predates
them — so the entry must describe the **net** change from the last released state, not a
sequence of two breaking changes to code no one has consumed.

Update it in place. A reader of the release notes should learn what changed since 0.27.0,
not what happened between two commits.

### D5. The guard's map drops to five

```
knotra-app/src/state/palette.rs
knotra-app/src/view/command_palette.rs
knotra-app/src/view/detail_panel.rs
knotra-app/src/view/shortcuts_overlay.rs
knotra-vcs/src/vcs/git.rs
```

## Requirements

| # | Requirement |
|---|---|
| R1 | `message.rs` and `app/freezer.rs` carry no suppression; the gate passes at `--all-targets -D warnings` |
| R2 | `ResolvedWidth`'s field is private and `OverlayWidth::resolve` is its only constructor |
| R3 | `surface()` takes seven parameters and carries no `too_many_arguments` allow |
| R4 | RFC-051's width arithmetic is unchanged; its tests still assert the same values |
| R5 | The guard's expected map is exactly D5's five entries |
| R6 | `CHANGELOG.md`'s `[Unreleased]` entry is revised in place to describe the net change since 0.27.0 (D4) |
| R7 | `crates/knotra-vcs` is not modified |
| R8 | `crates/knotra-app/src/tests.rs` is not edited |

## Test Plan

No new behaviour, so no new behavioural tests.

- The guard's map is asserted exactly; removing three entries is itself the test that they
  are gone and that nothing else moved.
- RFC-051's three arithmetic tests must still assert the same numbers. If reaching the
  value through `ResolvedWidth` requires a mechanical change to how they call it, that is
  expected - **what must not change is any asserted value**.

## Security Considerations

None. One hygiene note: this codebase's first clippy allow lasted one RFC, which is the
right lifetime for a suppression introduced by a design error rather than a constraint.

## Migration / rollout

No user-visible change. One `knotra-ui` API revision, folded into the existing unreleased
entry rather than stacked on it.
