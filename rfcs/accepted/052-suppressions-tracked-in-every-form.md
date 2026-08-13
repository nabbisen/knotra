# RFC-052 - Suppressions are tracked in every form, not one spelling

| Field | Value |
|---|---|
| Status | Accepted (2026-08-13, project owner) |
| Priority | High - a guard this project has cited since RFC-043 does not check what its own documentation claims |
| Effort | Small-to-medium - three deletions, one guard rewritten, every survivor justified |
| Target | Production Readiness Reset - operational hygiene |
| Related files | `crates/knotra-app/src/dead_code_guard.rs`, `crates/knotra-app/src/view/{command_palette,shortcuts_overlay,detail_panel}.rs` |
| Related RFCs | `rfcs/done/043-...md` (the guard this repairs), `rfcs/done/048-...md`, `rfcs/done/049-...md` (two of the three files, rewritten without anyone reading line 1) |
| Found by | reviewing Handoff 070 - an `allow(` sweep run for an unrelated reason |

## Summary

`dead_code_guard.rs` matches the **literal string** `#[allow(dead_code)]`. Three files
carry `#![allow(unused_imports, unused_variables, dead_code)]` on line 1 and are therefore
invisible to it. RFC-043's "39 → 0" is true only of one spelling.

## Problem

### The guard's claim is broader than its implementation

Its doc comment (`:7`):

> This guard fails if any `#[allow(dead_code)]` appears anywhere in
> `crates/knotra-app/src/` other than at one of those named locations

Its implementation (`:102`):

```rust
let n = count_occurrences(&source, "#[allow(dead_code)]");
```

`#![allow(unused_imports, unused_variables, dead_code)]` does not contain that substring.
`EXPECTED` is `&[]`, the test passes, and three files are exempt.

### What is actually in the tree

| Location | Attribute | Status |
|---|---|---|
| `view/command_palette.rs:1` | `#![allow(unused_imports, unused_variables, dead_code)]` | invisible to the guard |
| `view/shortcuts_overlay.rs:1` | same | invisible |
| `view/detail_panel.rs:1` | same | invisible |
| `state/palette.rs:19` | `#[allow(clippy::large_enum_variant)]` | not tracked by anything |
| `message.rs:14` | `#[allow(clippy::large_enum_variant)]` | not tracked |
| `app/freezer.rs:15` | `#[allow(unreachable_patterns)]` | not tracked |
| `knotra-ui/src/widget/overlay.rs:89` | `#[allow(clippy::too_many_arguments)]` | outside the guard's scope |
| `knotra-vcs/src/vcs/git.rs:578` | `#[allow(dead_code)]` | outside the guard's scope |

**The irony worth recording**: RFC-043's own doc comment says the 39 it removed were
"most of them blanket, on whole enums, exempting every variant forever." Three
**file-level** blanket allows - a strictly broader form - survived it.

### They mask nothing today, which is the only good news

Forcing all three lints (`--force-warn dead_code --force-warn unused_imports
--force-warn unused_variables`) produces **zero** warnings in those three files. So this
is not active rot. It is three permanent exemptions that would conceal future rot, in one
screen and two overlays.

**Two of the three were rewritten this cycle** - `detail_panel.rs` under RFC-048,
`shortcuts_overlay.rs` under RFC-049 - and neither the implementer nor the reviewer read
line 1 of either file while doing it.

## Non-goals

- `knotra-ui`'s `clippy::too_many_arguments` (RFC-051). Its removal via a `ResolvedWidth`
  newtype is a separate, smaller item and should not ride on this RFC's acceptance.
- Re-litigating `knotra-vcs`'s `tag_exists`. D2 brings it **into view** as a justified
  entry; whether it should exist is a different question with its own history.
- Widening any lint beyond the ones already named in the surviving attributes.

## Decision

### D1. Delete the three file-level blanket allows

Verified to mask nothing. Keeping a suppression that suppresses nothing is pure future
risk.

### D2. The guard tracks every suppression, in every form, across all three crates

Rename it for what it does. It matches **any** `#[allow(...)]` or `#![allow(...)]` -
inner or outer, single-lint or multi-lint, any lint - and asserts an exact expected map,
the same shape it uses today.

Scope widens from `knotra-app/src/` to all three crates. A guard covering one of three
crates has the same defect in miniature as one matching one of several spellings: it
sounds complete and is not.

**Every survivor gets a named justification at its own site**, which is RFC-043's
discipline applied to the attributes it never looked at.

### D3. It must exclude itself

`dead_code_guard.rs` contains the literal `#[allow(dead_code)]` **six times** in its own
prose. A broader matcher will match its own documentation.

This project has hit self-matching three times (`067`, `068` twice), and the established
answer is `is_scan_target`-style exclusion for the guard's own file, plus rewording
elsewhere. **Name the exclusion in the file, do not let it be rediscovered.**

### D4. The doc comment states what the code checks

Whatever D2 lands on, the prose must describe it exactly. The defect this RFC repairs was
not the matcher - it was a matcher and a claim that disagreed, with only the claim being
read.

## Requirements

| # | Requirement |
|---|---|
| R1 | The three file-level blanket allows are removed; the workspace builds and lints clean at `-D warnings` |
| R2 | The guard matches inner **and** outer attributes, single- and multi-lint, any lint name |
| R3 | The guard covers `knotra-app`, `knotra-ui` and `knotra-vcs`, or states its true scope in both prose and name |
| R4 | Every surviving suppression appears in the expected map **and** carries a justification comment at its own site |
| R5 | The guard excludes its own file, and says so where a reader will find it (D3) |
| R6 | The guard has been seen to fail on **three** planted violations: an inner `#![allow(...)]`, a multi-lint `#[allow(a, b)]`, and a plain outer `#[allow(x)]` |
| R7 | The doc comment's claims match the implementation exactly (D4) |
| R8 | `crates/knotra-app/src/tests.rs` is not edited |

## Test Plan

- R6's three plants, each reported verbatim, each naming file and count.
- The expected map asserted exactly, so both an unjustified addition and a silent removal
  fail.

No behavioural tests: this deletes three attributes that suppress nothing and rewrites a
test.

## Security Considerations

Indirect but real, and it is the owner's stated reason for the policy: a suppression
reduces what the compiler will tell us, and a suppression nobody can see reduces it
invisibly. Three files were exempt from three lints for the whole of RFC-043's cleanup and
neither of the two RFCs that rewrote them noticed.

## Migration / rollout

No user-visible change whatsoever. No data, config, or API change.
