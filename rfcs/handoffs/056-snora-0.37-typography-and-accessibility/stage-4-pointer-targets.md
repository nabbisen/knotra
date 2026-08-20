# RFC-056 Stage 4 — pointer targets, label widths, and the border boundary

Companion execution document for
[`rfcs/accepted/056-snora-0.37-typography-and-accessibility.md`](../../accepted/056-snora-0.37-typography-and-accessibility.md).
Status inherited from RFC-056. **Stage 4 of four — the last.**

**Read A4 first.** Two of the three items here were added by amendment after the
RFC was accepted, because they arose from work the RFC itself caused.

Baseline: `a600a33` (Stage 3), **301** tests, all gates green — re-run by me.

**Everything factual below verified at `a600a33`.**

**This handoff is immutable.** Corrections after issue go in a new document.

**Revised before issue, 2026-08-20.** snora 0.38.1 shipped RFC-072 between
drafting and handover, and it makes an assertion knotra already ships
non-conformant — see §4, added. This document had not been handed over, so it is
revised rather than followed by a correction; RFC-056 A5 records the reasoning.

## 1. Pointer targets (D6 / R7)

snora's checklist: **24×24 logical pixels minimum**, 44×44 preferred, and
*"spacing tokens (`tokens.spacing.sm` or larger) are used for padding rather than
zero or near-zero values that would collapse the target."*

**The arithmetic, and where it comes from:**

- iced's `button` default padding is **5.0 top and bottom**
  (`iced_widget-0.14/src/button.rs:462`).
- snora's own `make_button` sets **no padding and no height** — it relies on that
  default (`snora-widgets-0.38.0/src/design/button.rs:55-66`). So an unmodified
  snora button is roughly `18.2 + 10 = 28px` and clears 24.
- knotra has **28 call sites** that pass `.padding([0, …])`, overriding that
  default to **zero vertical**. Only **four** explicit `.height()` values exist in
  the whole view tree, so most of those 28 are the text line box alone.

Line boxes after Stages 2–3: `label` ≈ **18.2px**, `body_small` ≈ **17.6px**,
`body` ≈ **22.4px**. **All three are under 24.**

**Measure before fixing.** This is arithmetic, not a rendered measurement, and
some of the 28 may sit inside a container that supplies height. Report the actual
smallest target you find, and how you established it.

**The fix, where one is needed**, is a spacing token, not a magic number:
`tokens.spacing.xs` is 4.0 (→ ≈26px, clears) and `sm` is 8.0 (→ ≈34px,
comfortable). Prefer `sm` where it does not disturb a dense row; say where you
chose `xs` and why.

**Do not add explicit heights.** A height fixes the target and breaks when the
text size changes — which is exactly what Stages 2–3 just did to the constants in
§2.

## 2. The label widths (A4 / R11)

`detail_panel.rs`:

```rust
/// Fits `detail.label_remote` ("Remote:") … at the `body_small` role
const IDENTITY_LABEL_WIDTH: f32 = 56.0;
/// Fits `detail.label_untracked` ("Untracked:") … at the `body_small` role
const STATUS_LABEL_WIDTH: f32 = 72.0;
```

Both values were derived when those labels rendered at **11px**. Stage 2 moved
them to `body_small` (13.0) and the comments were rewritten to claim the fit holds
at the new role. **Nobody measured that.** `"Untracked:"` needs roughly 65–72px in
a 72.0px column.

Two acceptable outcomes:

- **re-derive both at 13px** — scaling by 13/11 gives roughly 66 and 85; or
- **keep the values and say the fit is unverified**, naming 11px as where it was
  last measured.

Either is fine. **A comment asserting a fit nobody measured is not.**

## 3. The border boundary assertion (A4 / R12)

knotra asserts `border` **only as a text colour** — `< AA_NORMAL`, justifying
`NoticeTone`'s exclusion of `Tone::Neutral`. Nothing asserts it meets **SC
1.4.11's 3:1 as a boundary**, which is what snora raised it for in 0.34.0.

**The failure mode:** if a future release regressed `border`, our existing
assertion would keep passing — *more* comfortably, since it asserts the ratio
stays **under** 4.5. The repair Stage 1 absorbed is unprotected by our own suite.

Add the assertion, **against the binding pair per preset**:

| preset | binding surface | current |
|---|---|---|
| `light` | `surface` | 3.1207:1 |
| `dark` | `surface_raised` | 3.1653:1 |

Both clear 3.0 with little room, so **assert `>= 3.0` and report the measured
values** — do not assert a tighter figure that a legitimate snora palette edit
would break.

`AA_LARGE` (3.0) already exists in `theme.rs` and is the right constant.

**Why `dark` uses `surface_raised`**: snora chose the border colour to clear the
binding pair per preset, and for `dark` that is the tighter `surface_raised`
(3.17) rather than `surface` (3.50). Asserting against `surface` there would track
the looser constraint and miss a regression.

## 4. An assertion we already ship, which snora has now said not to write (A5 / R13)

snora 0.38.1's `api-governance.md` states that **every contrast threshold they
ship is a floor** — a ratio is *at least* its threshold, **no maximum is
guaranteed**, and the only value change their covenant permits **raises a failing
ratio**. The consumer-facing consequence is explicit:

> do not assert that a snora colour stays below a threshold; if your decision
> depends on a colour being illegible, assert it against your own colour.

`theme.rs` does exactly that:

```rust
let neutral_ratio = contrast_ratio(p.border, surface);
assert!(neutral_ratio < AA_NORMAL, "... NoticeTone's exclusion of Neutral is now stale");
```

**Not hypothetical.** `border` moved 1.28 → 3.12 in 0.34.0 because it was
*failing*. A further repair past 4.5 fails this assertion — not because anything
regressed, but because snora improved a colour we asserted would stay bad. knotra
would be the reason a repair was held back. Our instance is cited in snora's
`feature-gating-criteria.md`, attributed.

**The purpose survives; the subject must change.** The assertion justifies
`NoticeTone` excluding `Tone::Neutral`, which depends on `border` being illegible
as *text*. Two acceptable outcomes:

- assert the exclusion against **knotra's own colour** rather than snora's
  `border`; or
- **drop the assertion** and justify the exclusion by the decision itself,
  recording in the comment that the ratio is snora's to move.

**Propose which, with reasoning.** What is settled is that no knotra test may
depend on a snora colour staying below a threshold.

Note the interaction with §3: R12 asserts a **floor** (`>= 3.0`), which is the
direction snora guarantees, so it is unaffected — and it is why §3 says assert
`>= 3.0` and not something tighter. snora also states a repair is judged **only**
on the failing pair and preserves no other, so a tighter figure would be asserting
something they have explicitly not promised.

## 5. Out of scope

No size or line-height changes — Stages 2 and 3 are settled. No colour changes:
R12 **asserts** the existing `border`, it does not alter it. No `design::render`
(D7), no `snora_core::focus` (D8). `crates/knotra-vcs` — **zero lines**. The
suppression map stays at **five**.

If a target cannot be fixed with padding alone — a control genuinely constrained
by its container — **stop and report** rather than reaching for a height.

## 6. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check a600a33..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Baseline **301**; expect a rise — R12's assertion at minimum. The last command
stays at **1** line.

`knotra-ui`'s contrast suite gains R12 and must otherwise stay green. **If an
existing contrast assertion moves, stop and report** — R12 adds a check, it does
not change a colour.

## 7. What to report back

A review request, paths relative to the project root, stating:

- **the smallest interactive target you measured**, and how;
- which sites you changed, with the spacing token used and why;
- your choice on §2, and the resulting comment text;
- **your choice on §4**, with reasoning, and confirmation no assertion depends on
  a snora colour staying below a threshold (R13);
- **R12's measured ratios for both presets**, and confirmation it asserts against
  the binding surface;
- gate output, gate five in the range form.

The targets and the label fit are things you cannot see. The **ratios** are not —
those are arithmetic, and they are the half of this stage that can be proven.

This is RFC-056's last stage. If anything in Stages 1–3 looks wrong to you from
here, say so now rather than after the RFC closes.
