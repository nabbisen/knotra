# RFC-058 - The resolve `Sheet` carries its own boundary

| Field | Value |
|---|---|
| Status | Accepted 2026-08-22 (project owner) |
| Priority | Low - no defect reaches a user; this is undeclared protection |
| Effort | Small - one assertion widened, two comments |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-ui/src/theme.rs` |
| Related RFCs | `rfcs/done/057-...md` R5 (measure the `Sheet`, do not fix it - this answers it), `rfcs/done/056-...md` A5 (the RFC-072 rule this RFC must not break) |
| Found by | me, following RFC-057 R5's measurement to its conclusion |

## Summary

RFC-057 R5 required the `Sheet`'s contrast be **measured and reported, not fixed**. The
dev team measured it: snora's sheet panel draws a 1px border that sits at **1.347882:1**
(light) / **1.290915:1** (dark) against its own fill - roughly a third of SC 1.4.11's 3:1
non-text boundary floor. `Sheet` has no style hook, so knotra cannot change it.

This RFC reports the conclusion: **knotra is not exposed.** The panel a user sees has a
compliant boundary in both presets, drawn by knotra's own content, and snora's invisible
border sits outside it carrying no information the content does not already carry.

The change proposed here is therefore not a fix. It is that the protection currently holds
by coincidence of two unrelated choices, neither written down, and both silently
removable.

## Problem

### The sheet's own chrome is invisible, and unreachable

`snora::Sheet` exposes `new` / `at` / `with_size` and nothing else - no style hook. The
engine wraps whatever content it is given in a panel it styles itself: fill from
`background.base`, 1px border from `background.weak`. Those two are close enough together
to be, in practice, one flat rectangle.

snora identified this themselves in 0.39.0 (RFC-077). No report from us is needed.

The only lever knotra has is **the content it passes in**.

### knotra pulls that lever, without having decided to

knotra has exactly one `Sheet` call site - `view.rs:170`, the conflict-resolve panel. Its
content is `resolve_panel`, whose outermost element is knotra's `overlay::surface`, which
wraps in `raised_card`, which is `snora::design::card::raised`, which draws:

- fill `tokens.palette.surface_raised`
- **border `tokens.palette.border`, 1px**

That border is measured, against the two surfaces it can physically neighbour:

| adjacency | light | dark |
|---|---|---|
| `border` vs `surface_raised` - inside the card | **3.3808** | **3.1653** |
| `border` vs `background` - the sheet's fill, outside the card | **3.3808** | **3.8079** |

All four clear 3.0. The panel is bounded on both sides of its own edge, in both presets.

(Light's two figures are identical because `background` and `surface_raised` are both pure
white in that preset - checked against `snora-design-0.38.0/src/presets/`, not assumed.
knotra's `with_knotra_typography` touches typography only, so these are the values that
reach the screen.)

### So the defect is real, documented upstream, and does not reach us

Which would end the matter, except for how it holds.

### It holds by two undeclared choices

The protection depends on a chain nobody wrote down:

1. every `Sheet` in knotra is handed content that draws its own border, and
2. `overlay::surface` is that kind of content.

Both are removable without a single gate turning red. Pass a bare `column` to
`Sheet::new`, or restyle `surface` onto a borderless container, and the resolve panel
loses its only perceivable edge - silently, because the thing that would then be the edge
belongs to snora and no test of ours measures it.

There is also a gap in the assertion that looks like it covers this and does not.
`theme.rs:730` asserts `border` against `surface` in light and against `surface_raised` in
dark - one pair per preset. **Neither preset tests `border` against `background`**, and
`background` is the pair that does the sheet protecting. The suite would pass if the
protecting pair failed.

## Non-goals

- **Styling the `Sheet`.** There is no hook. Nothing here reaches snora's panel.
- **Reporting upstream.** snora documented it in 0.39.0 RFC-077 before we asked. A letter
  would spend an owner relay to tell them what they already published.
- **Any visual change.** No pixel moves. No token value changes.
- **Asserting a `Sheet` boundary at render time.** iced elements are not introspectable
  after construction; there is no honest test that `resolve_panel` returns something
  bordered. That half stays covered by comment, and this RFC says so rather than
  pretending otherwise.

## Decision

### D1. Nothing is fixed, because nothing is broken

knotra's resolve panel meets SC 1.4.11 in both presets today. This is recorded as the
answer to RFC-057 R5.

### D2. The assertion is widened to the pair that actually protects

`theme.rs`'s boundary test grows from two pairs to the full matrix: `border` against
`surface`, `surface_raised`, and `background`, in **both** presets - six assertions where
there are now two. The four new ones include both figures in the table above.

This is worth having independently of the `Sheet`: `border` is knotra's boundary colour
against every surface tier, and testing one tier per preset was always narrower than the
claim the test's name makes.

### D3. The two undeclared choices are written down where they are removable

A comment at `view.rs:170` recording that `Sheet` has no style hook and its content must
therefore supply its own boundary, and a matching note on `overlay::surface` recording
that its border is load-bearing for that reason.

This does not stop either change. It makes a reviewer who is about to make one read why
the border is there first, which is the most a comment can do and more than exists now.

### D4. No requirement here asserts a snora colour stays below anything

snora RFC-072 makes every published threshold a **floor** with no guaranteed maximum: a
future release may legitimately raise `background.weak` against `background.base`, and an
assertion that it stays low would fail on a snora improvement. RFC-056 A5 caught exactly
that mistake in knotra once.

So the 1.29 / 1.35 figures appear in this document as **report only**. Nothing in R1-R6
turns them into a test.

## Requirements

| # | Requirement |
|---|---|
| R1 | `theme.rs`'s boundary test asserts `palette.border` against `surface`, `surface_raised`, **and** `background`, in both presets - six pairs, each `>= AA_LARGE` |
| R2 | The widened assertion is **seen to fail** on a planted violation before it is trusted (R3 discipline), and the planted case is reported |
| R3 | `view.rs:170` and `overlay::surface` each carry a comment recording that the `Sheet` has no style hook and its content supplies the boundary (D3) |
| R4 | **No assertion anywhere places an upper bound on a snora-derived colour** (D4) |
| R5 | No token value changes; no styling change; no pixel moves. `git diff` touches `theme.rs`, `view.rs`, and `overlay.rs` only |
| R6 | `crates/knotra-vcs` unmodified; the suppression map stays at **five** |

## Test Plan

- The six-pair boundary assertion, run in both presets.
- R2's planted violation: temporarily move one preset's `border` toward its `background`,
  confirm the new pair fails and names which pair, restore, confirm green. Report the
  failure message - a guard that has never failed is a guard nobody has checked.
- Full gate set; test count rises by the assertions added, no count falls.

## Security Considerations

None. No input handling, no VCS surface, no file or process access. The change is a test
widening and two comments.

## Migration / rollout

None required. No behaviour, no visual output, and no public API changes. Nothing to
sequence and nothing for a user to notice.

## Open question for the owner

None. This RFC needs an accept or a redirect, not a decision from you on its content.

## Amendments

### A1 - the Test Plan's count expectation was wrong (2026-08-22, before issue)

The Test Plan said "test count rises by the assertions added". That silently assumed the
six pairs become six **tests**. D2 asks for the existing test to be *widened*, which adds
six assertions inside one test and leaves the count **unchanged at 303**.

Both shapes satisfy D2. The binding expectation is **no count falls**; whether it rises
depends on a structural choice this RFC does not make. The implementer picks the shape and
reports which.

Recorded rather than edited in place: the RFC was accepted with the wrong line in it, and
a handoff may not override an RFC.

### A2 - the six figures, measured (2026-08-22, before issue)

The RFC's table quoted four of the six pairs. All six, probed through
`KnotraTheme::light()`/`dark()` and `snora::design::contrast::contrast_ratio` at baseline
`0fbfbb2` - not hand-computed:

| pair | light | dark |
|---|---|---|
| `border` vs `surface` | **3.1207** | 3.5047 |
| `border` vs `surface_raised` | 3.3808 | 3.1653 |
| `border` vs `background` | 3.3808 | 3.8079 |

All six clear 3.0, so R1's widened assertion passes as written - it locks in a property
that already holds, which is the whole point of it.

**The tightest pair is `light` `border` vs `surface` at 3.1207 - 0.12 of headroom.** That
pair is not the one protecting the `Sheet`, and it is already asserted today. It is
recorded because it is the pair a future token change would break first.
