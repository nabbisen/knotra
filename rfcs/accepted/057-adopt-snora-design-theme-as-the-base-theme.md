# RFC-057 - Adopt `snora::design::theme()` as knotra's base theme

| Field | Value |
|---|---|
| Status | Accepted (2026-08-21, project owner) |
| Priority | Medium-high - a contrast surface no test of ours can see |
| Effort | Small change, unmeasured blast radius - see D2 |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-ui/src/theme.rs`, `crates/knotra-app/src/main.rs` |
| Related RFCs | `rfcs/accepted/056-...md` (the migration that made this visible), `rfcs/done/052-...md` (the guard-checks-one-spelling shape this repeats) |
| Found by | the dev team, in the snora 0.39.1 survey (`reviewed/171`) |

## Summary

`KnotraTheme.base` is iced's stock `Theme::Light`/`Dark`. `main.rs` hands it to
iced as the application theme, so **every widget without an explicit style renders
from iced's palette rather than knotra's tokens** — and knotra's contrast suite
never touches it, so such a widget can fail contrast and pass every gate.

`snora::design::theme(&Tokens) -> iced::Theme` exists to close exactly this, and
knotra has never adopted it.

## Problem

### `base` is the application theme, and it is not ours

```rust
// theme.rs:86,94
base: iced::Theme::Light,   // and Theme::Dark
// main.rs:34
.theme(|state: &state::AppState| state.theme.base.clone())
```

`KnotraTheme` carries knotra's tokens for knotra's own widgets, and hands iced a
palette that has nothing to do with them.

**Five `scrollable` call sites are confirmed unstyled** — `dashboard/mod.rs`,
`history.rs`, `detail_panel.rs`, `settings.rs`, `widget/overlay.rs`. Every one
renders its scrollbar from iced's colours.

### No test of ours can see it

`theme.rs`'s contrast suite exercises `KnotraTheme::light().tokens` directly.
**`.base` appears in that file only in the two constructors** — the suite never
reads it. A scrollbar failing contrast passes every gate we have, silently.

This is RFC-052's shape again: a suite that proves what it looks at, read as
proving more. There it was one spelling of an attribute; here it is one half of a
theme.

### It also blocks a measurement we cannot otherwise take

snora measured their `Sheet` for the first time in 0.39.0 and found its
border-to-fill contrast at **1.02–1.35:1** against a 3.0 floor, documented and
unfixed. knotra's conflict-resolve panel is a `Sheet` (`view.rs:170`).

**Their figures assume the application's `iced::Theme` is
`snora::design::theme(&tokens)`** — because the `Sheet` reads
`extended_palette()`, not snora-design tokens. Ours is not, so **knotra's actual
Sheet contrast is unknown, not known-bad.** This RFC is the prerequisite for
measuring it.

## Non-goals

- **Fixing the `Sheet`.** Measure first; whether it needs a knotra-owned bordered
  container or an upstream report is a separate decision on evidence we do not yet
  have.
- High-contrast presets, modal focus trapping, `snora::focus` zone cycling — three
  further survey findings, all owner decisions, none of them this.
- Changing any knotra token value. This changes what *iced* is told, not what
  knotra renders through its own helpers.

## Decision

### D1. `base` becomes `snora::design::theme(&tokens)`

In both constructors, built from the same `Tokens` the struct already carries -
including knotra's `body_small` override, since the theme is derived from the
palette and the override is typographic.

`snora-style`'s own doc comment shows precisely knotra's call shape
(`.theme(move |_state| iced_theme.clone())`), so this is the intended use.

### D2. The blast radius is measured, not asserted

**I could not settle it by inspection and will not pretend otherwise.** knotra
styles many widgets through wrapper helpers - `card::raised`, `style::` functions,
`snora::design::button::*` - rather than an inline `.style()`, so grepping for
`.style(` near a widget cannot distinguish "styled by its wrapper" from
"unstyled". A count produced that way would be wrong in both directions.

**Establishing which widgets actually fall through to `base` is the first task of
this RFC**, not a premise of it. The five scrollbars are confirmed; everything
else is open.

### D3. The suite gains coverage of `base`

Whatever else changes, `theme.rs`'s tests must exercise the colours iced will
actually use. Today they cannot fail on `base` because they never read it.

The form is the implementer's to propose - asserting against
`base.extended_palette()`'s roles is the obvious candidate, since that is what
iced hands widgets and what snora's `Sheet` reads.

## Requirements

| # | Requirement |
|---|---|
| R1 | `KnotraTheme::light()`/`dark()` set `base` from `snora::design::theme(&tokens)` |
| R2 | The set of widgets that render from `base` is **measured and reported**, by a method that survives wrapper-applied styling (D2) |
| R3 | `theme.rs`'s suite asserts contrast on colours reachable through `base`; it must be capable of failing when `base` is wrong |
| R4 | knotra's existing token-based contrast assertions still pass unchanged, or any movement is reported with old and new ratios |
| R5 | The `Sheet`'s actual contrast is **measured and reported** under the new theme - not fixed |
| R6 | No knotra token value changes |
| R7 | `crates/knotra-vcs` is not modified; the suppression map stays at five |

## Test Plan

- R3's assertion is the durable half: it is what makes a future regression in
  `base` visible at all.
- R4 is a regression check, not new coverage - the token suite should be
  indifferent to this change, and if it is not, that is a finding.
- R5 produces a number, not a fix. If it clears 3.0 the `Sheet` finding closes; if
  it does not, that is a separate decision with evidence attached.

## Security Considerations

None directly. One accessibility note: a scrollbar is how a keyboard-and-pointer
user knows there is content below the fold. Rendering it from a palette nothing
verifies is the same class of gap as text below the legibility floor - invisible
to us, and load-bearing for the user least able to work around it.

## Migration / rollout

No data, config, or API change. Users see whatever D2's measurement turns up -
at minimum, scrollbars drawn from knotra's palette instead of iced's. The extent
is the thing this RFC exists to establish before promising anything about it.
