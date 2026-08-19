# RFC-056 Stage 3 — line-height

Companion execution document for
[`rfcs/accepted/056-snora-0.37-typography-and-accessibility.md`](../../accepted/056-snora-0.37-typography-and-accessibility.md).
Status inherited from RFC-056. **Stage 3 of four** — D5 only.

Baseline: `025bb3f` (Stage 2), **300** tests, all gates green — re-run by me.

**Everything factual below verified at `025bb3f`.**

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. Why this stage exists at all

knotra sets line-height **nowhere** — zero occurrences across three crates. We
reported that to snora, and **snora 0.38.0 shipped the six `*_line_height`
helpers because of it**: they went looking for why we found the size half of the
problem and not the other, and the answer was that the size half had a floor and
a helper and the line-height half had neither.

This stage is the reason that release exists. It should use what it shipped.

## 1. The number that shapes every decision here

**iced 0.14's default `LineHeight` is `Relative(1.3)`** (`iced_core-0.14.0/src/text.rs:217`).
Every knotra text today renders at 1.3, whatever its role.

Against snora's multipliers:

| Role | snora | vs iced's 1.3 | On a line |
|---|---|---|---|
| `body` (16px) | 1.4 | **+7.7%** | +1.6px |
| `body_small` (13px) | 1.35 | **+3.8%** | +0.65px |
| `title` (18px) | 1.3 | **identical — a no-op** | 0 |
| `heading` (24px) | 1.25 | −3.8% | **tighter** |
| `label` (14px) | 1.2 | −7.7% | **tighter** |
| `display` | 1.2 | — | unused |

**Three of the six roles would tighten or do nothing.** That is not an argument
for applying leading everywhere; it is the argument for applying it where it
does work.

## 2. Scope: `body` and `body_small` only (D5 / R4)

Apply `body_line_height(&tokens)` and `body_small_line_height(&tokens)` beside
their existing `*_size(&tokens)` calls.

```rust
.size(snora::design::style::text::body_size(tokens))
.line_height(snora::design::style::text::body_line_height(tokens))
```

The helpers return `LineHeight::Relative`, not `f32` — deliberately, per snora:
*"1.4 is a plausible-looking absolute line height and a catastrophic one."*

**Do not apply to `label`, `title`, `heading` or `display`:**

- **`title`** is 1.3 — byte-for-byte iced's default. Applying it changes nothing
  and adds 7 call sites of noise.
- **`label`** and **`heading`** are *tighter* than default. Applying them would
  compress line boxes for text that is single-line by construction — a change
  with no readability benefit, against snora's own rule that *"a label is one
  line; line-height has nothing to do."*

If you think one of those four should be included, **say so with the reason** —
but the default is no.

## 3. Uniform per role, not per site — and why

snora's guidance is *"apply line-height to anything that might wrap."* Applied
literally that is a judgement call at each of **149 `body_small`** and **83
`body`** sites, most of which never wrap.

**Apply it uniformly to both roles instead.** The cost of over-applying is
**+0.65px** on a `body_small` line and **+1.6px** on a `body` line. That is small
enough that per-site adjudication would cost more than it saves — and uniformity
is what makes §4's guard possible at all.

This is a lean with a number behind it. If you find a site where the extra
leading is visibly wrong, that is a finding worth reporting, not a reason to go
selective across 232 sites.

## 4. The guard (R4)

Extend `text_size_guard.rs`, or add a sibling in its shape: **every
`body_size(` / `body_small_size(` call is paired with the matching
`*_line_height(` call on the same widget.**

This is only assertable because §3 chose uniformity. Say in the guard's doc
comment that it is, so nobody later "simplifies" the rule and silently removes
the property.

`label_size(`, `title_size(`, `heading_size(` must **not** require a pairing —
they are correctly unpaired.

**R3 discipline**: plant a `body_size(` call with no line-height, confirm the
guard names the file and count, revert, re-run. Report verbatim.

## 5. Out of scope

**No size changes.** Stage 2 settled every size; if a role assignment looks wrong
to you, report it — do not fix it here.
**No pointer-target or padding work** — Stage 4.
**No `Typography` edits**: `body_small.size` stays 13.0 and its `line_height`
multiplier stays at snora's 1.35. If you want to override a multiplier, that is a
decision for the RFC, not this handoff.
No `design::render` (D7), no `snora_core::focus` (D8).
`crates/knotra-vcs` — **zero lines**. `crates/knotra-app/src/tests.rs` — zero
lines expected. The suppression map stays at **five**.

## 6. Carried into Stage 4, not here

`detail_panel.rs`'s `IDENTITY_LABEL_WIDTH` (56.0) and `STATUS_LABEL_WIDTH` (72.0)
carry comments claiming they fit their labels *"at the `body_small` role"*, but
their values were derived at 11px — reviewed/168 §3. **Leave them.** Stage 4 owns
it. Mentioned so you do not fix it in passing and split the finding across two
reviews.

## 7. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check 025bb3f..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Baseline **300**; expect **+1**, the guard. The last command stays at **1** line.

`knotra-ui`'s contrast suite must stay at 29 — leading is not colour. **If it
moves, stop and report.**

## 8. What to report back

A review request, paths relative to the project root, stating:

- the count of sites that gained a line-height, per role;
- confirmation `label`, `title`, `heading` and `display` gained none, and that
  `title`'s exclusion is understood as a no-op rather than an oversight;
- **the planted-violation output verbatim** (§4);
- any site where the added leading looked wrong to you on reading the code;
- gate output, gate five in the range form.

You cannot see this either. The pairing is checkable and the result is not —
whether a 7.7% looser body paragraph reads better is the owner's call, and it is
the whole point of the stage.
