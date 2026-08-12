# RFC-049 - The shortcuts overlay is localised, and its stale twin is removed

| Field | Value |
|---|---|
| Status | Accepted (2026-08-12, project owner) |
| Priority | High - 58 English strings in a shipped overlay, plus four catalog entries that describe bindings which do not exist |
| Effort | Small - one table, one file, an established pattern |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-app/src/view/shortcuts_overlay.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/048-...md` (this is the gap its survey could not see), `rfcs/done/038-...md` (the `label_key` pattern D2 follows and the guard D4 mirrors) |
| Found by | the dev team, out of scope, in Review Request 067 §4 |

## Summary

RFC-048 localised four strings in `shortcuts_overlay.rs` and declared the file done. It
has **58 more**, invisible to that RFC's survey because they are struct fields rather
than `text("literal")` arguments.

Separately, four `shortcut.*` catalog keys describe keyboard shortcuts that **nothing
renders and that partly do not exist**.

## Problem

### The bindings table is a second text surface

`BINDINGS` (`shortcuts_overlay.rs:20`) is 19 entries of three `&'static str` fields,
rendered at `:137-141` through `text(b.keys)`, `text(b.context)`, `text(b.desc)`.

| Field | Distinct values | Example |
|---|---|---|
| `keys` | 19 | `"Ctrl+K / ⌘K"`, `"Esc"`, `"g h"` |
| `context` | **5** | `Global`, `Dashboard`, `Selection`, `Modal`, `Palette` |
| `desc` | 19 | `"Open command palette"`, `"Check for updates (fetch)"` |

Field accesses are invisible to any scan of `text("…")` arguments, however the pattern is
written - RFC-048's guard names this as blind spot 3, and it is the reason this survived
that RFC.

`text("✕  Close")` (`:119`) is a fifth case: glyph-led, so the guard's leading-letter test
skips it, while `Close` inside it is untranslated English.

### Four catalog keys describe a UI that does not exist

`shortcut.refresh`, `shortcut.context`, `shortcut.freezer`, `shortcut.search` -
**zero code referents in the entire workspace**, in either locale. Eight catalog entries
rendered by nothing.

They are not merely orphaned. They are **wrong**:

| Key says | The table says |
|---|---|
| `Ctrl+K  Context` | `Ctrl+K / ⌘K` → *Open command palette* |
| `Ctrl+/  Search` | `/` → *Focus search field* |
| `Ctrl+T  Freezer` | **no `Ctrl+T` binding exists anywhere in the code** |

A comment added by Handoff 067 (`i18n.rs:893`) refers to them as "the shell's own shortcut
hints", which is what they look like and not what they are. They are a stale copy of an
older keyboard map, kept alive by the fact that nothing reads them.

## Non-goals

- **Re-wording.** As RFC-048 D1: translate what ships. The `desc` values already carry
  RFC-021's plain-language shape - *"Check for updates (fetch)"* - and keep it.
- Changing any actual key binding, or reconciling the table against the real handlers.
  **This RFC does not verify that the table is accurate**; it makes it translatable and
  removes a copy that is provably not. Auditing the table against `app/shortcuts.rs`
  is worth doing and is not this.
- The `keys` column. See D1.

## Decision

### D1. `keys` stays a literal; `context` and `desc` are localised

`Esc`, `Enter`, `Shift`, `Ctrl` and `⌘` are the legends printed on the hardware. They are
the same on a Japanese keyboard, and translating them would break the correspondence
between the overlay and the key the user is looking for.

`context` and `desc` are prose about knotra and are localised: **19 `desc` + 5 `context`
= 24 keys**, both locales.

### D2. The table stores catalog keys, not text

```rust
struct Binding { keys: &'static str, context_key: &'static str, desc_key: &'static str }
```

The view resolves them at render time. This is the house pattern - `StatusSummary.label_key`,
`RetryExclusionReason::i18n_key()` - and it keeps the table a table.

Keys stay under `shortcut.*`, which is not a first-level prefix, so
`first_level_wording_has_no_developer_jargon` does not apply. That matters: four `desc`
values legitimately contain `fetch`, `pull`, `tag` and `branch` as the expert term in
parentheses, which is RFC-021's own construction.

### D3. Remove the four stale keys, and the comment that vouches for them

All four, both locales, plus the `i18n.rs:893` comment describing them as live.

### D4. A guard, because these lookups are invisible to the existing one

`every_literal_t_call_names_an_existing_key` matches the literal text `.t("`. Resolving
`b.desc_key` through a variable is exactly what it cannot see - the same gap that required
a dedicated guard for `label_en` in RFC-038.

A test must assert that **every** `Binding`'s `context_key` and `desc_key` resolves in
**both** catalogs. Driven from `BINDINGS` itself, so adding a row without adding its keys
fails.

## Requirements

| # | Requirement |
|---|---|
| R1 | No user-facing English literal remains in `shortcuts_overlay.rs`, including inside `text("✕  Close")` |
| R2 | `keys` values are unchanged and remain literals (D1) |
| R3 | English `context`/`desc` wording is **unchanged** - this RFC translates, it does not re-word |
| R4 | 24 keys added to **both** catalogs |
| R5 | The four stale `shortcut.*` keys are removed from both catalogs, with their zero-referent status re-confirmed first, and the `i18n.rs:893` comment corrected |
| R6 | D4's guard exists, is driven from `BINDINGS`, and **has been seen to fail on a planted violation** - a row whose key is absent from one catalog |
| R7 | RFC-048's `text(...)`-literal guard stays green, and its expected map is unchanged |
| R8 | `crates/knotra-vcs` is not modified |

## Test Plan

- D4's guard, with its planted violation reported verbatim - plant a row referencing a key
  present in English and absent from Japanese, which is the failure that would otherwise
  ship as a key rendered as its own name.
- No behavioural tests: this changes where strings come from.

## Security Considerations

None. One correctness note: a user who cannot read the shortcut list cannot discover the
keyboard paths that RFC-036 built, which is an accessibility regression for every
non-English user rather than a security one.

## Migration / rollout

No data or schema change. Japanese users gain a translated shortcuts overlay. English
users see identical text (R3). Four catalog entries that rendered nowhere disappear.
