# Developer Handoff — RFC-058: the resolve `Sheet` carries its own boundary

Issued per `.git-exclude/roles/high-capability-model-operating-instructions.md` §5.
RFC: `rfcs/accepted/058-the-resolve-sheet-carries-its-own-boundary.md`,
**accepted 2026-08-22 (project owner)**, with amendments **A1** and **A2** applied before
issue — read both, A1 changes what "done" looks like.

Baseline: `0fbfbb2`, **303** tests, all gates green — re-run by me.

**Everything factual below verified at `0fbfbb2`.**

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. Read this first: nothing is broken

This is not a fix. Your RFC-057 R5 measurement was correct and the panel it measured is
**compliant today** — the sheet's invisible border is outside a card that draws its own at
3.17–3.38:1. RFC-058 §Problem has the chain.

You are locking in a property that already holds. Every assertion you add should pass the
first time you run it. **If one fails, stop and report** — that means the tree does not
match what the RFC measured, and the RFC is wrong before your code is.

## 1. Widen the boundary assertion (D2 / R1)

`crates/knotra-ui/src/theme.rs:730`,
`border_meets_the_wcag_1_4_11_boundary_floor_on_its_binding_surface_in_both_themes`.

Today it asserts **two** pairs: `border` vs `surface` in light, `border` vs
`surface_raised` in dark. Make it **six** — `border` against `surface`, `surface_raised`,
and `background`, in both presets.

`background` is the addition that matters: it is the sheet's fill, the surface knotra's
card border is seen against, and it is asserted in **neither** preset today.

A2 has all six measured values. The tightest is **light `border` vs `surface` at 3.1207**
— already asserted, not new, recorded so you know which pair has the least headroom.

### A1: the count may not move, and that is fine

Widening in place keeps the count at **303**. Splitting into six tests raises it. **Both
satisfy D2** — the binding rule is *no count falls*. Pick the shape you think reads better
and **say which you picked** in your report; do not contort the code to make a number move.

The test's name says "on its binding surface" (singular). If you widen in place, the name
is now wrong — rename it to match what it asserts.

## 2. Prove it can fail (R2)

R3 discipline: a guard nobody has seen fail is not a guard.

Move one preset's `border` toward the surface it is being tested against, confirm the
assertion fails, confirm the message **names which pair**, restore, confirm green.

Pick a pair with room — `dark` `border` vs `background` (3.8079) needs the largest nudge
and is the least ambiguous demonstration. Do not plant on light/`surface` (3.1207); a
violation there is hard to distinguish from the pair simply being tight.

**Report the failure message verbatim.** Six assertions that all say "border vs surface
failed" would be worse than two that name their pair.

## 3. Write down the two removable choices (D3 / R3)

The protection holds because of a chain nobody recorded. Two comments:

- **`crates/knotra-app/src/view.rs:170`** — the single `Sheet` call site. Record that
  `Sheet` has **no style hook** (`new`/`at`/`with_size` only,
  `snora-core-0.38.0/src/overlay.rs:194-227`) and that its content must therefore supply
  its own boundary.
- **`crates/knotra-ui/src/widget/overlay.rs:108`, `surface`** — record that its border is
  load-bearing for the sheet, not decorative, and why removing it would pass every gate.

These stop nothing. They put the reason in front of whoever is about to remove it, which
is all a comment can do and more than exists now.

## 4. Do not assert a snora colour stays below anything (D4 / R4)

snora RFC-072: every published threshold is a **floor**, with no guaranteed maximum. A
future snora release may legitimately raise `background.weak` against `background.base`,
and an assertion that it stays low would fail on their improvement.

RFC-056 A5 caught knotra doing exactly this once. Do not reintroduce it.

The 1.29 / 1.35 sheet figures are **report only**. Nothing you write turns them into a
test. If you find yourself reaching for `assert!(ratio < ...)` anywhere, that is the
mistake — stop and report instead.

## 5. What you are not doing

- **Not styling the `Sheet`.** There is no hook. Nothing here reaches snora's panel.
- **Not writing to snora.** They documented this themselves in 0.39.0 RFC-077.
- **Not changing a token value, a colour, or a pixel** (R5). If a screenshot would differ,
  something has gone wrong.
- **Not asserting `resolve_panel` returns something bordered.** iced elements are not
  introspectable after construction; RFC-058 Non-goals says so and does not want an
  approximation of it. §3's comment is the coverage for that half.
- `crates/knotra-vcs` — **zero lines** (R6).

## 6. Out of scope

Everything outside `theme.rs`, `view.rs`, and `overlay.rs` (R5). The suppression map stays
at **five** (R6) — a new lint is reported, not suppressed.

## 7. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check 0fbfbb2..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Baseline **303** (223 + 31 + 18 + 31). Expect **303, or higher if you split** (A1); no
count falls. `knotra` and `knotra-vcs` stay at **223** and **18 + 31** — this handoff's
only test change is in `knotra-ui`. Last command stays at **1** line (pre-existing
`git.rs::tag_exists`).

## 8. What to report back

A review request, paths relative to the project root, stating:

- **which shape you picked** for the six pairs (widened in place / split), and the
  resulting count (A1);
- **the six ratios as your code measures them**, so they can be compared against A2 —
  if any differs, that is the finding, not a rounding note;
- **R2's planted violation**: what you moved, the failure message verbatim, and
  confirmation of green after restoring;
- the two comments from §3, quoted;
- confirmation that **no assertion anywhere bounds a snora colour from above** (R4);
- confirmation the suppression map is still five, and that no colour or token value moved;
- gate output, gate five in the range form.
