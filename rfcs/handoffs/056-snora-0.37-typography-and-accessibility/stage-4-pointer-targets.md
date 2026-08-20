# RFC-056 Stage 4 — pointer targets, label widths, and the border boundary

Companion execution document for
[`rfcs/accepted/056-snora-0.37-typography-and-accessibility.md`](../../accepted/056-snora-0.37-typography-and-accessibility.md).
Status inherited from RFC-056. **Stage 4 of four — the last.**

**Read A4 first.** Two of the three items here were added by amendment after the
RFC was accepted, because they arose from work the RFC itself caused.

Baseline: `a600a33` (Stage 3), **301** tests, all gates green — re-run by me.

**Everything factual below verified at `a600a33`.**

**This handoff is immutable.** Corrections after issue go in a new document.

**Revised twice before issue.** Second revision 2026-08-20 — §1's measurement was
wrong and its instruction contradicted knotra's own correct pattern. See §1.

**Revised before issue, 2026-08-20.** snora 0.38.1 shipped RFC-072 between
drafting and handover, and it makes an assertion knotra already ships
non-conformant — see §4, added. This document had not been handed over, so it is
revised rather than followed by a correction; RFC-056 A5 records the reasoning.

## 1. Pointer targets (D6 / R7)

**This section was wrong in the first draft. Corrected here before issue.**

The first draft said *"only four explicit `.height()` values exist in the whole
view tree"* and told you not to add heights. Both were wrong, from one bad grep:
I matched `.height(<numeric literal>)` and missed every `.height(BUTTON_HEIGHT)`.

**What is actually there:**

| Form | Count |
|---|---|
| `.height(BUTTON_HEIGHT)` / `SMALL_BUTTON_HEIGHT` | **22** |
| `.height(<literal>)` | 5 |
| `.padding([0, N])` | 28 |

`BUTTON_HEIGHT` is **44.0** and `SMALL_BUTTON_HEIGHT` **36.0**
(`knotra-ui/src/widget/layout.rs:35,38`), with a comment explaining when each
applies. **44 is WCAG's *preferred* target size, not the 24 minimum** — knotra
already has a deliberate, correct pointer-target system, and the first draft told
you not to use it.

**Of the 28 zero-vertical-padding sites, 24 sit on a height-constrained control**
and are fine — `.padding([0, 18])` on a 44px-tall button is horizontal padding on
a target that already clears everything.

**Four have no height in scope:**

```
view/shortcuts_overlay.rs:154
view/shell.rs:211
widget/field.rs:66
widget/field.rs:133
```

**Those four are the work.** Measure them: line box after Stages 2–3 is `label`
≈ 18.2px, `body_small` ≈ 17.6px, `body` ≈ 22.4px, and iced's button default adds
5.0 top and bottom — so a control that overrides padding to zero and sets no
height is roughly 18px, under the 24 floor.

**Use the existing constants where the control is a button.** `SMALL_BUTTON_HEIGHT`
(36) for dense inline controls, `BUTTON_HEIGHT` (44) for primary actions — that
is what `layout.rs`'s own comment says and it is already right. Reach for a
spacing token (`tokens.spacing.xs` = 4.0, `sm` = 8.0) only where a height would be
wrong for the widget, and say which you chose and why.

**Verify my four before fixing them.** My window-based check looked ±8 lines for a
height on the same builder chain; a control whose height is set further away, or
by a container, would look unpaired to it and is not. **If one of the four turns
out to be height-constrained after all, report that rather than padding it.**

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
