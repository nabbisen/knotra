# RFC-0022 — Migrate to snora 0.25.0

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| Status         | Implemented (v0.23.0)                                                 |
| Priority       | Low — version bump; design feature evaluated and deferred            |
| Effort         | Minimal (bump) + evaluation                                          |
| Target version | v0.23.0                                                              |
| Related        | RFC-0019 (snora layout adoption); RFC-0021 (plain-language layer)    |

## Summary

snora 0.25.0 introduces the **Snora Design System** — an opt-in `design`
feature providing an iced-free token crate (`snora-design`), a semantic
palette with paired status-text foregrounds, automated WCAG AA contrast
tests, an iced style bridge, and button/card/notice/chip/progress helpers.

This RFC updates knotra from `snora 0.18.1` to `0.25.0` and records the
decision on whether to adopt the new `design` feature.

**Decision: take the version bump; do NOT enable the `design` feature now.**

## What knotra uses from snora

knotra-app consumes snora only as a **layout engine**:

- `AppLayout`, `Dialog`, `Sheet`, `SheetEdge`, `SheetSize`, `render`,
  `LayoutDirection` — overlay composition (`view/mod.rs`)
- `Tab`, `TabBar`, `TabAction`, `widget::app_tab_bar` — workspace tabs
  (`view/workspace_tabs.rs`)

knotra does **not** use snora's styling. Visual design lives in `knotra-ui`:
`KnotraTheme`, `StatusColor`, `guided_button` / `guided_field`, layout tokens.

## Breaking changes in the 0.18 → 0.25 range

Two breaking changes exist in the range; neither affects knotra:

1. **`Palette::roles()` removed from public API (v0.24).** Relevant only to
   code calling `palette.roles()`. knotra does not use snora's `Palette`.
   No impact.

2. **Chip selected-state visual change (v0.24).** Affects `snora::design::chip`
   only. knotra does not use snora chips. No impact.

Everything knotra uses (`AppLayout` / `Dialog` / `Sheet` / `render` /
`app_tab_bar` / `TabBar` / `Tab` / `TabAction` / `SheetEdge` / `SheetSize` /
`LayoutDirection`) is unchanged across the entire 0.18 → 0.25 range. iced
stays at 0.14, so there is no framework-upgrade churn. **Zero source changes.**

Verified: `cargo +1.91 clippy --workspace --all-targets` 0/0; 71 tests pass.
`snora-design` is correctly absent from `Cargo.lock` (the `design` feature is
not enabled).

## Why the `design` feature is deferred (not rejected)

The Snora Design System is well-built and genuinely strong on accessibility.
But knotra **already has** a complete, working equivalent in `knotra-ui`:

| snora::design provides | knotra-ui already has |
|---|---|
| `Tokens`, `Palette` (18 roles) | `KnotraTheme`, `StatusColor` (6 roles, WCAG AA verified in RFC-0021 Phase 6) |
| `button::{primary,secondary,ghost,danger}` | `guided_button` (disabled-with-reason) |
| automated contrast tests | Phase 6 contrast pass + the `first_level_wording` guard |
| focus tokens (documents iced 0.14 limitation) | `focus_id` + `focus_input()` (RFC-0021 Phase 6) |
| `card`, `notice`, `chip`, `progress` | bulk-modal components, activity strip, status badges |

Enabling `design` now would mean either:
- running **two parallel design systems** (snora's + knotra-ui's), which is
  exactly the "complicated and messy" outcome knotra avoids; or
- a **large rip-and-replace migration** of knotra-ui's styling onto snora's
  tokens — significant churn for no user-visible benefit, since knotra's
  contrast is already WCAG AA after Phase 6.

Neither earns its place right now. Per knotra's "less is more" principle, the
lean choice is to stay on knotra-ui's own design layer.

## When to revisit

The `design` feature becomes worth a dedicated RFC if:
- knotra-ui's styling accumulates enough maintenance burden that delegating
  to snora's tokens would *reduce* total complexity; or
- snora promotes `design` to default-on (it is opt-in today, and the snora
  maintainers state default-on requires its own RFC and size/build-cost
  review); or
- knotra wants a primitive snora offers that knotra-ui lacks (e.g. the
  `notice` banner or `progress` card) and building it locally would clearly
  be more work than adopting snora's.

At that point the migration would be a deliberate, scoped piece of work —
deleting knotra-ui styling in favour of `snora::design` — not a dependency
bump. This RFC explicitly leaves that door open.

## Implementation

One change: `crates/knotra-app/Cargo.toml`:

```toml
# before
snora = "0.18.1"
# after
snora = "0.25.0"
```

No source changes. No new feature flags. `design` remains disabled.

## Open questions

None.
