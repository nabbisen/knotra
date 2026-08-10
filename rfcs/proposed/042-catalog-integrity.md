# RFC-042 - Catalog Integrity: Missing Keys Render As Their Own Names

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - four user-facing messages are broken in shipped releases, including the one shown on every successful Settings save |
| Effort | Small - four catalog entries, one design decision, one guard |
| Target | Production Readiness Reset - operational hygiene track |
| Related files | `crates/knotra-ui/src/i18n.rs`, `crates/knotra-app/src/app/misc.rs` |
| Related RFCs | `rfcs/done/021-plain-language-layer.md` (the catalog and its two-tier guard policy), `rfcs/accepted/038-settings-and-history.md` (Stage 1 widens the symmetry guard; this RFC is what that question uncovered) |

## Summary

`Locale::t()` falls back to returning the key itself when a key is missing. **Four
keys are referenced in code and absent from the catalog**, so four user-facing
messages currently render as raw identifiers — `settings.saved_ok` appears verbatim
in the status bar every time a user saves their settings successfully.

This RFC fixes the four, decides what `t()` should do about a miss, and adds a guard
so the class cannot recur silently.

## Background

### The mechanism

`i18n.rs:46-48`:

```rust
pub fn t(&self, key: Key) -> &'static str {
    self.strings.get(key).copied().unwrap_or(key)
}
```

A missing key is not an error, a panic, or an empty string. It renders as its own
name. That is a reasonable choice for never crashing a GUI over a string, and it is
exactly why these four have survived unnoticed.

### The four, verified at `46ee262`

| Key | Call site | What the user sees |
|---|---|---|
| `settings.saved_ok` | `misc.rs:181`, `:182` | `settings.saved_ok` in the Settings panel **and** the status bar, on every successful save |
| `settings.save_error` | `misc.rs:185` | `settings.save_error <io error>` when a save fails |
| `tool.not_configured` | `misc.rs:202` | `tool.not_configured` when no editor or merge tool is set |
| `tool.launch_failed` | `misc.rs:211` | `tool.launch_failed <name>: <error>` when launching one fails |

Each confirmed absent by exact-string search of `i18n.rs`, and confirmed reachable by
reading the call sites.

`settings.save_error` is doubly unfortunate: it is the path that reports a refused
save, which is the case `129` A2 already flagged as under-communicated. The message
explaining why a save failed is itself broken.

### What is **not** wrong, checked and cleared

Two adjacent worries were measured and neither holds:

- **Orphan keys: none.** All 381 catalog keys appear in the source. An earlier count
  of 113 "never referenced" was an artifact of matching only literal `.t("…")` calls
  and missing dynamic constructors — `err.i18n_key()`, `action.label_key`,
  `exclusion.reason.i18n_key()`, `context_target_kind_key(…)`.
- **Catalog symmetry: intact.** 381 English keys, 381 Japanese, zero gaps in either
  direction. Recorded because both figures were initially reported wrong from a regex
  that missed rustfmt-wrapped `m.insert(` calls.

Both are stated because the corrected numbers are the useful ones and the wrong ones
were nearly acted on.

### Why the existing guards missed this

`i18n.rs`'s two tests both filter on `FIRST_LEVEL_PREFIXES` — `plain.`, `tier.`,
`workspace.`, `dashboard.`, `filter.`. Neither `settings.` nor `tool.` is among them,
so neither guard looks at these keys at all.

More fundamentally: **both guards inspect the catalog, and neither inspects the
code.** Symmetry compares two catalogs; jargon inspects catalog values. Nothing has
ever checked that a key the code asks for actually exists. That is the gap this RFC
closes.

## Motivation

1. **Four broken messages are shipped**, in 0.27.0 and every release before it.
2. **The most visible one fires on a success path** — a user who saves settings
   correctly is shown an identifier.
3. **The class is silent by construction.** `unwrap_or(key)` means a typo in a key
   name never fails a test, never logs, and renders something that looks almost
   plausible.

## Non-goals

- **Not changing the catalog's shape.** No move to typed keys, enums, or an external
  file format. See Alternatives.
- **Not the symmetry-guard widening** — that is RFC-038 Stage 1's first commit,
  already in flight, and it addresses a different invariant.
- **Not reviewing the jargon guard's scope.** Its narrowness is a deliberate
  RFC-0021 decision: `status.*`, `card.*`, and `action.*` are named in its own doc
  comment as the expert layer. Widening it would fail on 8 keys that are correct as
  written, `settings.merge_tool_label` among them.
- **No `tests.rs` edits.**

## Decision

### D1. Add the four keys to both catalogs

Under their existing namespaces, with wording consistent with neighbouring keys.
`tool.*` is a namespace with no current entries — check whether one already fits
before creating it.

### D2. Guard: every literally-referenced key must exist

A test that scans `crates/` for `.t("…")` literals and asserts each is present in
`en_strings()`. This is what would have caught all four.

It cannot see dynamic keys — `err.i18n_key()` and friends build key strings at
runtime — so it is a partial guard. That is fine: it covers the form that failed,
and D3 covers the rest.

**Where it lives needs a decision.** It has to read source files, which no existing
`knotra-ui` test does. Proposing it as a `knotra-ui` test that walks the workspace is
one option; a small build-time or CI check is another. The implementer should
propose, with the tradeoff stated.

### D3. Decide `t()`'s behaviour on a miss

The real design question, and the reason this is an RFC rather than a bug fix.

| Option | Effect |
|---|---|
| **A. Keep `unwrap_or(key)`** | Never crashes; hides every miss, including dynamic ones. Status quo |
| **B. `debug_assert!` then fall back** | Catches misses in tests and debug builds, including dynamically constructed keys; release behaviour unchanged |
| **C. Return a marked string** (e.g. `⟨key⟩`) | Visible in any build, no crash, but ships a marker to users |
| **D. Typed keys** | Compile-time impossibility. Correct in principle; 381 keys and 272 call sites |

**Recommendation: B, with D2.** D2 catches literal keys at CI time with no runtime
change; B catches the dynamic ones during any test or debug run. Together they cover
both forms without altering what a released binary does.

D is the real fix and out of proportion here — it is a refactor of every call site to
close a hole that B and D2 close for a fraction of the cost. Worth its own RFC if the
catalog keeps growing.

## Requirements

| # | Requirement |
|---|---|
| R1 | `settings.saved_ok`, `settings.save_error`, `tool.not_configured`, `tool.launch_failed` exist in **both** catalogs |
| R2 | A guard fails if any `.t("literal")` in `crates/` names a key absent from `en_strings()` |
| R3 | The guard is demonstrated to fail before it passes — introduce a bogus key locally, confirm the test catches it, remove it. Report that it was done |
| R4 | `t()`'s miss behaviour follows D3, and the chosen option is stated |
| R5 | No existing catalog key is renamed or removed |
| R6 | `crates/knotra-app/src/tests.rs` is not edited |

R3 matters: a guard that has never been seen to fail is not known to work, and this
whole RFC exists because a check that looked adequate was not.

## Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <base>..HEAD
```

The count is expected to rise — D2's guard and any D3 coverage are new tests.

**Manual check worth doing once:** run the app, press Save Settings, and confirm the
status bar shows a sentence rather than an identifier. This is a defect a user would
notice immediately and no automated test currently observes.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| The guard's source-scanning is brittle — regexes over Rust source have already produced two wrong answers while investigating this | A guard that silently stops matching, giving false confidence | R3 requires proving it fails on a planted bad key |
| `debug_assert!` fires in existing tests on a dynamic key nobody knew was missing | A test suite that suddenly fails | That is the guard working. If it happens, report the keys rather than weakening the assert |
| Wording for the four new keys is invented rather than recovered | User-facing text decided incidentally | Match neighbouring keys' tone; the strings are short and their meaning is unambiguous from context |

## Alternatives considered

**Just add the four keys.** Fixes today's instances and leaves the class open. The
four went unnoticed across at least four releases precisely because nothing looks.

**Typed keys (D3 option D).** The only option that makes this impossible rather than
detectable. Rejected on proportion, not principle: 381 keys, 272 literal call sites,
plus the dynamic constructors would need enum variants. Reconsider if the catalog
grows substantially.

**Make `t()` panic on a miss.** Rejected — crashing a GUI over a missing string is
worse than showing an identifier, and it converts a cosmetic defect into a
denial-of-service on whatever screen holds the key.
