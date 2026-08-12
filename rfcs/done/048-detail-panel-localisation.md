# RFC-048 - The detail panel is localised, and text outside the catalog is caught

| Field | Value |
|---|---|
| Status | Implemented (main: ba9cf30) |
| Priority | High - a shipped panel that is entirely English for every Japanese user |
| Effort | Small-to-medium - 22 strings, one layout fix, one guard |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-app/src/view/detail_panel.rs`, `crates/knotra-app/src/view/shortcuts_overlay.rs`, `crates/knotra-ui/src/i18n.rs`, `crates/knotra-app/src/dead_code_guard.rs` (the guard's model) |
| Related RFCs | `rfcs/done/021-...md` (the plain-language layer this predates), `rfcs/done/042-...md` (catalog integrity - the guards that could not see this), `rfcs/done/043-...md` (the allow-list guard shape D3 mirrors) |
| Blocks | RFC-039, whose natural home is a new section in `detail_panel.rs` |

## Summary

`view/detail_panel.rs` makes **zero** `state.t()` calls and hardcodes **18** English
strings. `view/shortcuts_overlay.rs` hardcodes **4**. Every other view file is clean.

No guard can see this, because every existing guard checks text that already reaches the
catalog.

## Problem

### Two files, and only two

A survey of all 22 files under `view/`:

| File | `t()` calls | literals | Verdict |
|---|---|---|---|
| `detail_panel.rs` | **0** | **18** | unlocalised |
| `shortcuts_overlay.rs` | **0** | **4** | unlocalised - **and a further 58 strings this survey could not see**: `BINDINGS`' 19x3 field accesses and one glyph-led `text("X  Close")`. Found during implementation; see `157` SS3 and RFC-049 |
| `overlays/conflict.rs` | 17 | 4 | `command:`/`stdout:`/`stderr:`/`error:` on raw tool output - justified |
| `settings.rs` | 30 | **2** | `text("English")` and `text("日本語")` - language names, correctly untranslated. The RFC said 1; the ASCII-only survey grep missed the second (see `157` SS2) |
| the other 18 | 3-49 | 0 | clean |

A Japanese user opening the project detail panel gets an English panel: section headers
(`Identity`, `Status`, `Recent operations`, `Actions`), every field label (`Branch:`,
`Ahead:`, `Behind:`, `Dirty:`, `Untracked:`, `Conflict:`), the buttons (`Refresh`,
`Fetch`, `Remove from workspace`), and the states (`Loading…`, `None`).

### Why four guards missed it

- `every_literal_t_call_names_an_existing_key` iterates over `.t("` occurrences. **A file
  with none has no violations** and passes trivially.
- `all_keys_are_localised_in_both_catalogs` compares the two catalogs to each other.
- `first_level_wording_has_no_developer_jargon` inspects catalog **values**.
- `status_bar_and_settings_save_msg_always_route_through_t` covers two named fields.

Together they establish that **what reaches the catalog is correct**. Nothing establishes
that user-facing text reaches the catalog at all. That is the gap, and it is why this
survived RFC-021, RFC-042 and RFC-038.

### The labels are space-padded, so this is not a find-and-replace

```rust
text(format!("VCS:    {}", vcs))
text(format!("Branch:     {}", branch))
text(format!("Untracked:  {}", untracked))
```

The column alignment is **literal spaces sized to the English words**. Translating the
labels breaks it, and padding a translated string to match is worse - it bakes a layout
constant into a catalog value that a translator would have to preserve blindly.

## Non-goals

- **Re-wording.** See D1. Whether the panel should say something plainer than "Branch" is
  a separate question and is explicitly deferred.
- Translating `conflict.rs`'s diagnostic prefixes - raw tool output is the category
  RFC-038 A1 already ruled English by design.
- `text("English")` in the language switcher. A language is named in its own language.
- RFC-039. This unblocks it; it does not start it.

## Decision

### D1. Localise; do not re-word. Keys go under `detail.*`, not `plain.detail.*`

The detail panel is the **expert surface** that RFC-021's plain-language layer defers
*to* - it is where a user goes for `Ahead`/`Behind`/`Dirty`. Those terms are appropriate
here in a way they are not on the dashboard.

Putting the keys under `plain.` would subject them to `first_level_wording_has_no_developer_jargon`,
which forbids `fetch`, `branch` and `conflict` - forcing a re-wording exercise inside what
should be a localisation fix. `detail.*` is not a first-level prefix, exactly as
`settings.*` and `history.*` are not.

**Localisation and plain-language are separate concerns.** Conflating them here would
triple the scope and bury a shipped defect behind a wording debate.

### D2. `shortcuts_overlay.rs` uses the existing `shortcut.*` prefix

Four strings: the title and three column headers. `shortcut.*` already exists and is
**already properly localised in both catalogs** (`shortcut.refresh` = "Ctrl+R  更新"), so
this is four new keys in an established namespace.

### D3. Labels and values are laid out structurally, not by space-padding

Two columns, sized by the layout engine. No catalog value carries alignment whitespace,
in either locale.

### D4. A guard that catches text which never reaches the catalog

Model it on `dead_code_guard.rs`, which this project already uses for exactly this shape:
scan a directory, count occurrences, assert an **exact expected map**, and require any new
entry to be named with a justification.

Scan `crates/knotra-app/src/view/` for `text("<letter>…")` and `text(format!("<letter>…"))`.
The right end state is the two justified files:

| File | Count | Justification |
|---|---|---|
| `overlays/conflict.rs` | 4 | raw tool output prefixes (RFC-038 A1's category) |
| `settings.rs` | 1 | a language name, not translatable |

Everything else must be zero. A leading-letter test deliberately excludes glyphs (`✕`,
`⚠`, `✓`), which are signals rather than language - the same reasoning `StatusSummary`'s
doc comment already gives for keeping the glyph out of the catalog.

**Where it lives is the implementer's to propose**, with the trade stated: beside
`dead_code_guard.rs` as a knotra-app source contract, or in `i18n.rs` beside the other
text guards.

## Requirements

| # | Requirement |
|---|---|
| R1 | `detail_panel.rs` and `shortcuts_overlay.rs` make no user-facing English literal; all 22 strings route through the catalog |
| R2 | Every new key exists in **both** catalogs |
| R3 | English wording is **unchanged** from what ships today (D1) - this RFC translates, it does not re-word |
| R4 | No catalog value contains alignment whitespace; the label/value layout is structural (D3) |
| R5 | D4's guard exists, with an exact expected map, and **has been seen to fail on a planted violation** |
| R6 | All existing catalog guards stay green |
| R7 | `crates/knotra-vcs` is not modified |
| R8 | Glyph-only strings are not routed through the catalog |

## Test Plan

- D4's guard, with its planted violation reported verbatim.
- The guard's expected map is asserted exactly, so removing a justified exception or
  adding an unjustified one both fail.

No behavioural tests: this changes where strings come from, not what the panel does.

## Security Considerations

None directly. One indirect: a user who cannot read the panel describing what knotra is
about to do to their repository cannot give informed consent to it. The guard is the part
that keeps this fixed.

## Migration / rollout

No data, config, or schema change. Japanese users gain a translated detail panel and
shortcuts overlay. English users see identical text (R3) with the alignment produced by
layout rather than by padding.
