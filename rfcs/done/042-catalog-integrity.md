# RFC-042 - Catalog Integrity: Missing Keys Render As Their Own Names

| Field | Value |
|---|---|
| Status | Implemented (main: f49cdec) |
| Priority | High - four user-facing messages are broken in shipped releases, including the one shown on every successful Settings save |
| Effort | Small - four catalog entries, one design decision, one guard |
| Target | Production Readiness Reset - operational hygiene track |
| Related files | `crates/knotra-ui/src/i18n.rs`, `crates/knotra-app/src/app/misc.rs` |
| Related RFCs | `rfcs/done/021-plain-language-layer.md` (the catalog and its two-tier guard policy), `rfcs/accepted/038-settings-and-history.md` (Stage 1 widens the symmetry guard; this RFC is what that question uncovered) |

## Implementation Record

| Commit | Content |
|---|---|
| `1b04047` | The guards (R2, R8) - **red in isolation by design**, see below |
| `92b13a7` | The four missing keys (R1) |
| `02ed062` | The five hardcoded `app/` strings (R7) |
| `f49cdec` | `t()`'s `debug_assert!` on a miss (R4) |

Handoff 048. The symmetry-guard widening (2a) landed earlier, in `3f2e83f`, as
Handoff 047 commit 1.

**Outcome.** Nine broken user-facing strings fixed: four keys that rendered as their
own names - `settings.saved_ok` appeared verbatim in the status bar on every
successful save - and five strings in `app/` that never reached the catalog at all,
two of them shown after every bulk fetch. Catalog 398 -> 406, eight keys added to
each locale, added sets identical, symmetric.

**The guards caught nine real defects on their first execution.** `1b04047` is
deliberately red: the guards were committed before the fixes, so `cargo test -p
knotra-ui` at that commit reports 22 passed, 2 failed, naming every genuine defect
RFC-042 exists to close. That is stronger evidence than a synthetic plant, and it
lives in history rather than in a document. The commit message states the failure is
intentional and names what turns it green, which is what separates a documented red
commit from a broken one.

**R3 earned its place.** Both guards produced false positives on first run - they
scan `crates/` recursively, which includes `i18n.rs`, whose own doc comments contain
`.t("...")`-shaped text. Traced by reading the output rather than widening the
pattern until it passed, then fixed with a scoped exclusion. The reviewer's own
attempt to re-prove a guard silently no-opped, because the plant's anchor did not
exist in the target file and `str.replace` reported nothing - nearly reporting a
working guard as broken. A guard nobody has watched fail is not known to work, and
neither is a proof nobody has watched apply.

**Namespace choice took the harder option.** The five `app/` strings went under
`plain.activity.*`, which `FIRST_LEVEL_PREFIXES` polices, rather than an unpoliced
namespace that would also have satisfied the requirement. "Fetch" - a `FORBIDDEN_EN`
term - is absent from the new wording, following `plain.activity.kind_fetch`'s
existing rendering of that operation as "Check for updates".

D3 option D (typed keys) remains out of scope, as drafted.

Review artifact: `.git-exclude/reviewed/138-handoff-048-review-rfc-042-approved.md`.

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

### D4. A third class the D2 guard cannot see: strings that never call `t()`

**Added 2026-08-10, before implementation, after measuring the call sites.**

D2's guard checks that every key named in a `.t("literal")` exists. It cannot see a
user-facing string that never calls `t()` at all. Five such strings exist in `app/`,
verified at `46ee262`:

| Location | String | When the user sees it |
|---|---|---|
| `background/fetch.rs:107` | `"Fetch — {} ok, {} failed"` | after every bulk fetch that had a failure |
| `background/fetch.rs:113` | `"Fetch complete — {} projects"` | after every successful bulk fetch |
| `misc.rs:118` | `"Copy command sent."` | copying a command |
| `misc.rs:172` | `"FS watching disabled."` | toggling filesystem watch off |
| `misc.rs:208` | `"Launched: {} {:?}"` | launching an editor or merge tool |

The two in `background/fetch.rs` are the most visible in the application: bulk fetch
is knotra's core operation, and **both** branches are hardcoded. They also contain
"Fetch", which `FORBIDDEN_EN` bans from first-level wording - the guard never sees it
because the string never becomes a catalog key.

Not every `status_bar` assignment is affected. `app/changelog.rs:109` and
`app/background/freeze.rs:61` compose their messages from `t()` lookups on both
branches, which is the correct shape and the model to follow.

**These are in scope.** They are the same defect as the four missing keys - the code
and the catalog disagree about a user-facing string - and excluding them would leave
the RFC's own remedy demonstrably incomplete.

**Guard:** a literal-scan over assignments to known user-facing sinks - `status_bar`,
`settings_save_msg` - that contain a string literal and no `t()` call. Narrower than
"find all user-facing strings", which is not mechanically decidable, and it covers
exactly the shape that failed here.

Note that RFC-038 Stage 1 would not have caught these either: it is scoped to
`view/settings.rs` and `view/history.rs`, and all five live in `app/`.

## Requirements

| # | Requirement |
|---|---|
| R1 | `settings.saved_ok`, `settings.save_error`, `tool.not_configured`, `tool.launch_failed` exist in **both** catalogs |
| R2 | A guard fails if any `.t("literal")` in `crates/` names a key absent from `en_strings()` |
| R3 | The guard is demonstrated to fail before it passes — introduce a bogus key locally, confirm the test catches it, remove it. Report that it was done |
| R4 | `t()`'s miss behaviour follows D3, and the chosen option is stated |
| R5 | No existing catalog key is renamed or removed |
| R7 | The five hardcoded strings in `app/` (D4) are moved into the catalog, both locales |
| R8 | A guard fails if an assignment to `status_bar` or `settings_save_msg` contains a string literal and no `t()` call |
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
