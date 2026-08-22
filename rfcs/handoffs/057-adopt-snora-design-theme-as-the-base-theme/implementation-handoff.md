# RFC-057 — implementation handoff

Companion execution document for
[`rfcs/accepted/057-adopt-snora-design-theme-as-the-base-theme.md`](../../accepted/057-adopt-snora-design-theme-as-the-base-theme.md).
Status inherited from RFC-057.

Baseline: `925d9fb` (RFC-056 Stage 4), **302** tests, all gates green — re-run by me.

**Everything factual below verified at `925d9fb`.**

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. This is your finding

RFC-057 exists because your snora 0.39.1 survey found it. The RFC and this
document add measurement requirements and a guard; the diagnosis is yours.

## 1. The change (R1)

`theme.rs:86,94`:

```rust
base: iced::Theme::Light,   // → snora::design::theme(&tokens)
base: iced::Theme::Dark,    // → snora::design::theme(&tokens)
```

Built from the same `Tokens` the struct already carries, **after**
`with_knotra_typography` — the theme derives from the palette, and knotra's
`body_small` override is typographic, so ordering is not load-bearing but
consistency is.

`snora-style-0.38.0/src/theme.rs:366`'s own doc shows exactly knotra's call shape
(`.theme(move |_state| iced_theme.clone())`), which `main.rs:34` already uses.

## 2. The blast radius (R2) — and why I could not measure it

**I tried and got a wrong answer. Do not repeat my method.**

I counted widget constructors with a nearby inline `.style(` and got *zero styled*
for every widget including `container` — obviously false. knotra styles through
wrapper helpers (`card::raised`, `style::` functions, `snora::design::button::*`),
so proximity to `.style(` cannot distinguish "styled by its wrapper" from
"unstyled".

**Confirmed unstyled, by direct inspection:** the five `scrollable` sites —
`dashboard/mod.rs`, `history.rs`, `detail_panel.rs`, `settings.rs`,
`widget/overlay.rs`. Everything else is open.

**R2's purpose is the owner's visual judgement, not the guard.** §3's assertion
protects the palette regardless of which widgets consume it. So: establish what
you reliably can, by whatever method survives wrapper styling, and **state the
method's limits rather than a confident number**. "These five confirmed, this
class undetermined, here is why" is the answer I want. A number I have to retract
costs more than a gap I can see.

## 3. The suite must become capable of failing (R3)

`theme.rs`'s contrast suite reads `KnotraTheme::light().tokens` and **never
`.base`** — `.base` appears in that file only in the two constructors. A widget
rendering from `base` can fail contrast and pass every gate we have.

`iced::Theme::extended_palette()` is public (`iced_core-0.14/src/theme.rs:138`)
and returns the `Extended` palette iced actually hands widgets — `background`
with `base`/`weak`/`weaker`/`weakest` tiers, each a `Pair` of colour and text.

Assert contrast on the roles reachable through it. **The test must be able to
fail**: verify that by planting a wrong `base` (stock `iced::Theme::Light` is
exactly the plant, since it is what we ship today) and confirming it goes red.
Report that output verbatim.

## 4. The `Sheet` measurement (R5) — arithmetic, not rendering

snora measured their `Sheet` at **1.02–1.35:1** border-to-fill against a 3.0
floor, documented and unfixed. knotra's conflict-resolve panel is one
(`view.rs:170`). **Their figures assume the app theme is
`snora::design::theme(&tokens)`** — because the `Sheet` reads
`extended_palette()`, not snora-design tokens.

So under R1 the figure becomes knowable, and it is **computable without a
renderer**: query `extended_palette().background.weak` and `.base` from the new
theme and compute the WCAG ratio, both presets.

**Measure and report. Do not fix.** If it clears 3.0 the finding closes; if not,
that is a decision with evidence attached — a knotra-owned bordered container, or
an upstream report — and it is not this handoff's to take.

## 5. The regression check (R4)

Every existing token-based contrast assertion should be **indifferent** to this
change — they read `tokens`, not `base`. If one moves, that is a finding: report
the old and new ratio rather than adjusting a threshold.

## 6. One carry from RFC-056's last review

`theme.rs:699`, in R12's doc comment, says
`notice_tone_colors_meet_wcag_aa_against_surface_in_both_themes` *"asserts a
`< AA_NORMAL` ceiling"*. **That assertion was removed in Stage 4.** Line 656 has
the same fact in past tense and is correct; 699 was missed.

One clause. You are opening this file anyway.

## 7. Out of scope

**No knotra token value changes** (R6) — this changes what *iced* is told, not
what knotra renders through its own helpers. No `Sheet` fix (§4). No
high-contrast presets, no modal focus trapping, no `snora::focus` zone cycling —
three further survey findings, all owner decisions.
`crates/knotra-vcs` — **zero lines**. The suppression map stays at **five**.

## 8. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check 925d9fb..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Baseline **302**; expect a rise — R3's assertion at minimum. The last command
stays at **1** line.

## 9. What to report back

- what you established about the blast radius, **and the limits of how you
  established it** (§2);
- **the planted-violation output verbatim**, proving R3's test can fail (§3);
- **the `Sheet`'s measured ratios**, both presets, and whether they clear 3.0 (§4);
- confirmation no existing token assertion moved, or the ratios if one did;
- gate output, gate five in the range form.

The ratios are arithmetic and provable. Whether the newly token-derived
scrollbars and chrome *look* right is the owner's, as it has been every stage.
