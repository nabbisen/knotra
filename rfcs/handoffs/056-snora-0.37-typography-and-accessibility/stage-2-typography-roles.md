# Developer Handoff 078 — RFC-056 Stage 2: typography roles and the 12px floor

Issued per `.git-exclude/roles/high-capability-model-operating-instructions.md` §5.
RFC: `rfcs/accepted/056-snora-0.37-typography-and-accessibility.md`, **accepted
2026-08-19, amended A1/A2/A3**. This is **Stage 2 of four** — D3 and D4 only.

**Read A3 first.** It corrects two measurements in the RFC's own Problem section
and changes the central value of this stage.

Baseline: `c98c577` (your Stage 1 commit), **299** tests, all gates green — re-run
by me.

**Everything factual below verified at `c98c577`.**

**This handoff is immutable.** Corrections after issue go in a new document.

## 1. What the tree actually looks like

The RFC said ninety sites "bypass" knotra's two font constants. True — and 150
others use them:

| Form | Occurrences | Value |
|---|---|---|
| `.size(FONT_BODY)` | 80 | 15.0 |
| `.size(FONT_SMALL)` | 70 | 13.0 |
| `.size(FONT_BODY + 2.0)` | 7 | 17.0 |
| `.size(FONT_BODY + 6.0)` | 1 | 21.0 |
| `.size(FONT_SMALL + 1.0)` | 1 | 14.0 |
| raw `.size(<literal>)` | 90 | 10, 11, 12, 13, 14, 15, 20 |

**This makes the stage cheaper.** 150 sites move by redefining two constants.
Only the 90 raw literals need per-site judgement.

## 2. The rule that governs every decision here (R10)

**No text size decreases.** Not one site.

This is why A3 exists: A1 specified `body_small = 12.0`, which would have shrunk
all 70 `FONT_SMALL` sites from 13 to 12 in order to gain one pixel on 32 outliers
— a regression on the many to spare the few, invisible to both of us.

**If a role assignment would shrink a site, do not make it. Report it.**

## 3. The custom scale (D3 / A1 / A3 / R2 / R3)

knotra supplies its own `Typography`. `Tokens::light()` / `dark()`
(`theme.rs:71,79`) return a `Tokens`; override the role, then store it.

**`body_small` is 13.0**, not snora's 14.0 and not A1's 12.0.

Safe because nothing snora renders uses it — verified in RFC-056 A1 against the
0.38.0 source of all four crates: `snora-widgets` calls only `label_size` and
`body_size`; `body_small_size`, `title_size`, `heading_size` and `display_size`
are called by nothing. **Do not override `label` or `body`** — those reach
snora's own chrome.

Proposed mapping. **This is a lean; you hold the code:**

| Current | Sites | Role | Delta |
|---|---|---|---|
| 10, 11 | 38 | `body_small` (13) | up — **were below the floor** |
| 12 | 29 | `body_small` (13) | up 1 |
| 13, `FONT_SMALL` | 84 | `body_small` (13) | **unchanged** |
| 14, `FONT_SMALL + 1.0` | 4 | `label` (14) | unchanged |
| 15, `FONT_BODY` | 83 | `body` (16) | up 1 |
| 17 (`FONT_BODY + 2.0`) | 7 | `title` (18) | up 1 |
| 20, 21 (`FONT_BODY + 6.0`) | 4 | **your call** | `title` (18) shrinks — forbidden by R10 |

**The last row is the one I cannot settle from here.** `heading` is 24 — a
four-pixel jump on a shell/settings/history title. `title` is 18 and would
shrink. A third custom role value is available if neither fits. **Propose, with
reasoning, and say what it does to those four sites.**

## 4. The floor (D4 / R2)

After §3, nothing renders below **13**, which clears snora's floor of 12.

snora's `readability.md` now states the floor is 12 **and nothing else**, with
the role preference stated separately — they revised it after our report. There
is no second floor at 14.

## 5. Retiring the constants

`FONT_BODY` and `FONT_SMALL` (`knotra-ui/src/widget/layout.rs:41,44`) go, in
favour of the role helpers — `snora::design::style::text::body_size(&tokens)` and
friends, which need `&tokens` at the call site where the constants needed
nothing.

**If a call site cannot reach `tokens`, stop and report.** That is a structural
obstacle, not a chore, and I would rather rule on it than have a constant kept
alive quietly to route around it.

`widget/chip.rs:52` already calls `label_size(tokens)` — the pattern exists.

## 6. The guard (R8)

A fourth source-scanning guard, in the shape of the three we already have: **no
raw `.size(<numeric literal>)` under `crates/knotra-app/src/view/` or
`crates/knotra-ui/src/widget/`**, with an exact expected map of justified
exceptions.

**Exclude the guard's own file**, as `suppressions_guard` and
`text_outside_catalog_guard` do — this codebase has hit self-matching four times.

**R3 discipline**: plant a `.size(11)` in a clean view file, confirm the guard
names the file and count, revert, re-run. Report verbatim.

## 7. Out of scope

**No line-height** — Stage 3, and it uses 0.38's `*_line_height` helpers.
**No pointer-target work** — Stage 4. No colour, spacing, or padding changes. No
`design::render` (D7), no `snora_core::focus` (D8).
`crates/knotra-vcs` — **zero lines**. `crates/knotra-app/src/tests.rs` — zero
lines expected; if the `Tokens` override forces one, stop and report.
The suppression map stays at **five**.

## 8. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check c98c577..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Baseline **299**; expect **+1**, the guard. The last command stays at **1** line.

knotra-ui's contrast suite must stay green — it asserts colour, not size, so this
stage should not touch it. **If it moves, stop and report**: that would mean the
`Tokens` override reached something it should not have.

## 9. What to report back

A review request, paths relative to the project root, stating:

- **the final size mapping**, as a table, with the delta per row;
- **every site that would have shrunk**, and what you did instead (R10);
- your decision on §3's last row, with reasoning;
- how the custom `Typography` is constructed, and confirmation that `label` and
  `body` are untouched;
- **the planted-violation output verbatim** (§6);
- confirmation nothing renders below 13;
- gate output, gate five in the range form.

You cannot see any of this. Say so — the sizes are checkable and the result is
not, and the owner is the one who will judge whether a denser screen got looser.
