# RFC-056 - snora 0.25 to 0.37: typography and accessibility

| Field | Value |
|---|---|
| Status | Accepted (2026-08-19, project owner); amended 2026-08-19 (A1, A2, A3, A4, architect - see Amendments) |
| Priority | High - the version gap is free to cross, and crossing it is what makes the typography work possible |
| Effort | Medium-to-large - one dependency line, then four stages of adoption |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `Cargo.toml`, `crates/knotra-ui/src/theme.rs`, `crates/knotra-ui/src/widget/layout.rs`, `crates/knotra-app/src/view/**` |
| Related RFCs | `rfcs/done/022-...md` (the 0.25.0 migration this succeeds), `rfcs/done/021-...md` (the plain-language layer this is the typographic half of), `rfcs/done/033-...md` (the umbrella shape this follows) |
| Source | `.git-exclude/tmp/snora-0.25-to-0.37/` - seven release bundles, 0.25.0 through 0.37.1 |

## Summary

knotra pins `snora = "0.25"`. snora is at **0.37.1**, and the whole span is
**non-breaking for knotra** - verified, not assumed. Crossing it brings a
six-role typography scale and an accessibility vocabulary knotra currently has
no access to, and repairs a contrast defect in the presets knotra uses.

The version bump is one line. The reason to do it is everything it unlocks.

## Problem

### knotra has no typography system, and 38 call sites are below the readability floor

`knotra-ui` defines exactly two font constants - `FONT_BODY = 15.0` and
`FONT_SMALL = 13.0` (`widget/layout.rs:41,44`). The view tree then bypasses them
**90 times** with raw literals:

| Size | Occurrences | Against snora's floor |
|---|---|---|
| `.size(11)` | 32 | **below 12px** |
| `.size(10)` | 6 | **below 12px** |
| `.size(12)` | 29 | at the floor |
| `.size(13)` | 14 | ok |
| `.size(14)` / `.size(15)` / `.size(20)` | 3 / 3 / 3 | ok |

snora's readability floor is explicit: *"never a custom size below 12 logical
pixels ... size and contrast both have to clear a floor for text to be legible,
and this is the size half of that pair."* **38 call sites are under it.**

### `line_height` is never set anywhere in knotra

Zero occurrences across all three crates. Every wrapping paragraph knotra
renders - notice bodies, help text, the conflict panel's prose line added by
RFC-054, error sentences - uses iced's default line spacing.

snora's guidance: *"apply line-height to anything that might wrap."* knotra
applies it to nothing.

### The presets knotra uses shipped a contrast failure, and 0.34.0 repairs it

`KnotraTheme::light()`/`dark()` call `Tokens::light()`/`Tokens::dark()`
(`theme.rs:71,79`) - the built-in presets. snora 0.34.0 raised `border` because
it was failing WCAG SC 1.4.11:

| preset | before | after |
|---|---|---|
| `light` | 1.28:1 | **3.12:1** |
| `dark` | 1.19:1 | **3.17:1** |

`light`'s `text_muted` also moves `#6B7280` -> `#6A717E` (4.46:1 -> passing).

**This reaches knotra's own test suite.** `theme.rs:655` asserts the border
contrast is *below* AA, and its comment records *"Measured at 1.28:1 (light) and
1.32:1 (dark), both confirmed by a temporary probe."* The assertion still holds
after 0.34 - 3.12 is still under AA - but **its recorded figures become false**,
which is the class of stale comment this project has corrected four times.

### Pointer target size is unmeasured

snora's checklist mandates 24x24 logical pixels minimum, and warns against
*"zero or near-zero"* padding that collapses a target. knotra has **four**
explicit control heights (30, 32, 36, 250) and **sixteen** controls at
`.padding([0, 18])` - zero vertical padding. A button with no height and 12px
text is roughly a 15px line box tall.

This is a measurement gap, not a confirmed failure. It has never been checked.

### knotra is on the engine path and misses the design path's chrome

`view.rs:67` imports `snora::render`, not `snora::design::render`. Two
consequences, both currently invisible:

- The **dialog card** - fill, border, radius - is `design`-path only. knotra's
  modals are chromeless by default and each overlay supplies its own surface.
- The **modal dim** on the engine path is a hardcoded 40% black. snora 0.37.0
  moved the design path's `DIM_ALPHA` to 0.44 after measuring the card at
  2.85:1 against its own backdrop in `light`. knotra does not receive that fix,
  and over a dark background a 40% black dim composites weakly.

## Non-goals

- Restyling. This adopts a scale; it does not redesign screens.
- Adopting `snora::design::render`. See D7 - named, evidenced, and deferred to
  its own decision.
- `snora_core::focus` F6 zone cycling (0.35.0). See D8.
- Changing knotra's colour choices beyond what the preset upgrade brings.
- Any `knotra-vcs` change.

## Evidence that the span is safe to cross

Verified against the release bundles and knotra's own tree:

| Claim | Evidence |
|---|---|
| No public item removed or renamed in `snora` 0.25 -> 0.32 | 153 items at 0.25.0, 157 at 0.32.0, compared as sets |
| Nothing removed or renamed 0.30 -> 0.33 in `snora` | 19 public items at both |
| The one rename (0.28 -> 0.29, `snora-dialog-card`) does not reach knotra | knotra has no `iced_test`, no `Simulator`, no snora identifier assertions |
| `snora-widgets` lost 16 items at 0.33.0 | knotra does not depend on `snora-widgets` directly |
| MSRV 1.88 | knotra declares `rust-version = "1.88"` |
| A `widgets` + `design` build is byte-identical across 0.32.0's crate split | measured by snora, stated in the 0.33.0 notes |

**The only behavioural change knotra receives is 0.34.0's contrast repair**, and
it is one-way: contrast increases, nothing becomes harder to see.

## Decision

### D1. Bump to 0.37, in one stage, before anything else

`snora = { version = "0.37", features = ["design", "lucide-icons"] }`.

Land it alone, with the appearance change absorbed and the stale figures
corrected, so that the typography work that follows sits on a stable base and
any visual surprise has exactly one candidate cause.

### D2. Absorb 0.34.0's border repair and correct knotra's recorded figures

Re-measure `theme.rs`'s border assertion and rewrite its comment to the new
values. The assertion's *conclusion* is expected to survive; its *evidence* is
not, and this project's rule is that a comment recording a measurement is a
claim.

### D3. Adopt the six typography roles; retire the ad-hoc scale

`body` / `body_small` / `label` / `title` / `heading` / `display`, via
`snora::design::style::text::*_size(&tokens)`.

**A precedent already exists in-tree**: `widget/chip.rs:52` already calls
`snora::design::style::text::label_size(tokens)`. This generalises one call site
to the other ninety.

`FONT_BODY` and `FONT_SMALL` are retired in favour of roles rather than
redefined in terms of them - two constants that ninety call sites ignore are not
a scale.

### D4. Nothing user-facing renders below 12 logical pixels

The 38 sub-floor call sites move up. Which role each becomes is a per-site
judgement (`body_small` at 14 is the usual answer for metadata currently at 11),
and the visible result is that some text gets larger. **That is the point**, and
it is the one part of this RFC a user will notice immediately.

### D4a. Superseded by Amendment A1

D4's cost basis was wrong. See A1.

### D5. Line-height on anything that wraps

`LineHeight::Relative(tokens.typography.<role>.line_height)` for prose:
notice bodies, help text, error sentences, RFC-054's conflict prose line. Not
for labels, which are single-line by construction.

### D6. Pointer targets are measured, then met

Establish what knotra's smallest interactive target actually is, then bring
anything under 24x24 up to it - by vertical padding from `tokens.spacing`, not
by a magic height.

Measurement first: this is currently a suspicion with arithmetic behind it, not
a finding.

### D7. `design::render` adoption is named and deferred

Moving from `snora::render` to `snora::design::render` would bring the styled
dialog card and the token-derived modal dim, including 0.37.0's repair. It would
also change how every one of knotra's seven overlays is framed, against a
`surface()` helper knotra rebuilt as recently as RFC-051 and RFC-053.

**Not bundled.** It is a visual change to every modal, it interacts with
`OverlayWidth`, and it deserves to be decided on its own evidence rather than
carried in on a typography RFC.

### D8. `snora_core::focus` is named and deferred

0.35.0 added pure zone-cycling (`next_zone`, F6/Shift+F6) with modal
containment. knotra has keyboard navigation across regions and is exactly its
audience. Also not bundled, for the same reason.

Worth recording now: snora 0.34.0 **corrected its own documentation** to say
focus rings *can* be styled when the application owns focus as state. knotra
already does this (`widget/ring.rs`, `with_focus_ring`), so knotra was right and
the old snora guidance was wrong.

## Amendments

### A1. The floor is 12, not 14, and `Density::Compact` was never the answer (2026-08-19, architect)

**Source: snora's reply to our migration review, 2026-08-19.** Recorded before
Stage 2 begins; Stage 1 (Handoff 077) is unaffected.

Two corrections, both in our favour, both to claims of mine.

**1. `Density::Compact` is a spacing scale, not a type scale.** RFC-056's
question to snora asked whether to design for its arrival. snora: *"it is a
**spacing** scale, not a type scale ... Resolving it would change padding and
gaps, never a text size. Do not design for its arrival on this question."*

I read `Density::Compact` in `tokens.md` and inferred it addressed density in
general. It addresses padding. The question was worth asking - snora treated the
ambiguity as a documentation defect on their side - but the premise was mine and
it was wrong.

**2. D4 over-costed the floor by roughly double.** D4 wrote that
*"`body_small` at 14 is the usual answer for metadata currently at 11"*, making
the change ~27% on knotra's densest rows. snora:

> The floor is **12**, not 14 ... Your 11px rows need **11 -> 12**, about 9%.
> Your 12px rows are already compliant. The role guidance is a preference for
> staying inside the scale, not a second floor at 14.

Corrected figures, re-measured:

| Size | Sites | Status under the real floor |
|---|---|---|
| 11px | 32 | must reach 12 - about **9%**, not 27% |
| 10px | 6 | must reach 12 |
| 12px | **29** | **already compliant** - no change |
| 13px and above | 23 | already compliant |

**38 sites move, not 90**, and they move by one or two pixels rather than by a
role step.

**The resolution, which snora's own documentation enables and I missed.** R3
requires sizes come from token roles, not literals; the floor permits a custom
12.0. Those pull in opposite directions only if knotra uses snora's default
scale unaltered. It need not: `typography.md` states *"`Typography` is a plain,
non-`#[non_exhaustive]` struct - an application supplying its own `Tokens` can
set every field."*

So **knotra supplies its own `Typography` with `body_small` at 12.0**, and the
38 sub-floor sites become `body_small`. Sizes still come from a role (R3 holds),
the floor is met, and knotra's density survives.

**This is safe from snora's chrome, and only because of which role we pick.**
Verified from the **0.37.2 source of all four snora crates**, not from the
documentation sentence I first cited:

- `snora-design` is tokens only - no widgets, and `body_small` appears in it
  solely as a field definition and in tests.
- `snora-style` *defines* the six `*_size` helpers.
- `snora-widgets` *calls* exactly two: `label_size` and `body_size`
  (`design/chip.rs`, `design/notice.rs`, `design/button.rs`, `design/progress.rs`).
- `snora` (the engine) calls none.

So `body_small_size`, `title_size`, `heading_size` and `display_size` are called
by **nothing in any snora crate**. Overriding `body_small` reaches knotra's own
text and nothing snora renders. **Overriding `label` or `body` would**, and this
RFC does not.

*Evidence note, recorded because the conclusion survived but its support did
not:* this amendment first cited `typography.md`'s *"not applied by any widget in
`snora-widgets`"*. That sentence is scoped to one crate, and knotra reaches these
widgets through `snora::design::*`, so the citation did not cover the case it was
being used for. The source check above does, and it was run before any of this
was implemented.

**R2 is unchanged** - nothing renders below 12. What changes is the cost and the
mechanism.

### A2. Target 0.38, and line-height has helpers now (2026-08-19, architect)

**Source: snora 0.38.0 and its letter, 2026-08-19.** Recorded before Stage 1 is
issued; Handoff 077 is revised rather than amended, because it had not been
handed over.

**0.38.0 exists because of our report.** We told them we had set no line-height
anywhere; they went looking for why we found the size half of the problem and not
the other, and the answer was theirs - *"the size half had a floor and a helper;
the line-height half had neither."*

**The target moves 0.37 -> 0.38.** Not cosmetic: `snora = "0.37"` is `^0.37`, and
cargo treats a 0.x minor bump as incompatible, so it will **not** resolve 0.38.
Reaching it requires an explicit change, and doing it now is one bump instead of
two. 0.38.0 is purely additive over 0.37, carries no rendered change by their
statement, and contains what Stage 3 needs.

**D5's idiom is withdrawn.** RFC-056 D5 specified
`LineHeight::Relative(tokens.typography.<role>.line_height)`, quoting
`typography.md`'s *"line-height is not wrapped in a helper."* 0.38.0 adds six:

```rust
.size(snora::design::style::text::body_size(&tokens))
.line_height(snora::design::style::text::body_line_height(&tokens))
```

Verified present in the 0.38.0 source (`snora-style/src/text.rs:45-105`), one per
role. They return `LineHeight::Relative`, not `f32`, deliberately - *"1.4 is a
plausible-looking absolute line height and a catastrophic one."*

The old form still works; `tokens.typography` is public and stays supported.
**Stage 3 uses the helpers**, because they are the documented form and they make
the size and leading calls read as a pair.

**Their re-check, answered: nothing to unwind.** They asked whether we had
written our own wrapper because they said none existed. We had not - `line_height`
and `LineHeight` have **zero occurrences** across all three knotra crates. We
skipped leading entirely rather than hand-rolling it, so the withdrawn claim cost
us nothing.

**A1 is confirmed by their documentation, not only by our reading.**
`readability.md` now states the floor is 12 and nothing else, with the role
preference stated separately as a preference - and names our resolution outright:
redefine `body_small` on your own `Tokens`.

**One dependency we checked and do not have.** snora's prefab widgets still do
not apply line-height internally, deferred deliberately, and they asked whether
our migration depends on it. It does not: our two `notice()` call sites
(`overlays/conflict.rs:81`, `overlays/changelog.rs:110`) carry short,
non-wrapping messages - "Done.", "We could not finish that action." Saying
otherwise would move their priority on a dependency we cannot demonstrate.

### A3. `body_small` is 13.0, not 12.0 - and the constants are not ignored (2026-08-19, architect)

**Recorded while scoping Stage 2, before it was issued.** Two measurements in
this RFC were wrong, and they point the same way.

**1. The Problem section says ninety call sites "bypass" knotra's two font
constants. They do - but 150 others use them.** Measured:

| Form | Occurrences |
|---|---|
| `.size(FONT_BODY)` | 80 |
| `.size(FONT_SMALL)` | 70 |
| `.size(FONT_BODY + 2.0)` / `+ 6.0)` / `.size(FONT_SMALL + 1.0)` | 9 |
| raw `.size(<literal>)` | 90 |

D3 called them *"two constants that ninety call sites ignore"*. That is true of
ninety and false of a hundred and fifty. knotra does have a scale; it is
two-valued, escaped by a third of its sites, and derived from by arithmetic in
nine more.

**This makes Stage 2 cheaper, not dearer.** Redefining `FONT_BODY`/`FONT_SMALL`
in terms of roles fixes 150 sites at two definitions. Only the 90 raw literals
need per-site judgement.

**2. A1's `body_small = 12.0` would shrink 70 compliant sites.** A1 chose 12.0
to keep the 11px rows dense, reasoning from an 11px baseline. But `FONT_SMALL`
**is 13.0**, used at 70 sites: knotra's dense default is already 13, and the 11px
sites are the outliers.

At `body_small = 12.0`, those 70 sites shrink 13 -> 12 to gain one pixel on 32.
**That is a regression on the many to spare the few**, and neither of us can see
it happen.

**`body_small` is 13.0.** Then:

| Current | Sites | Becomes | Delta |
|---|---|---|---|
| 10px | 6 | `body_small` 13 | up, was below floor |
| 11px | 32 | `body_small` 13 | up, was below floor |
| 12px | 29 | `body_small` 13 | up 1 |
| 13px + `FONT_SMALL` | 14 + 70 | `body_small` 13 | **unchanged** |

**Nothing shrinks.** That is the rule Stage 2 is held to (R10), and it is the one
A1 would have broken.

The floor still governs: 13 clears 12 with a pixel to spare, and snora's
`readability.md` now states the floor is 12 and nothing else.

**A1's mechanism stands** - knotra supplies its own `Typography`, and overriding
`body_small` reaches nothing snora renders. Only the value changes.

### A4. Stage 4 also carries two items this RFC did not anticipate (2026-08-19, architect)

Recorded **before** Stage 4 is issued, because RFC 000 forbids a handoff
widening an RFC's scope on its own authority: *"Handoffs must not override RFC
decisions. If handoff work uncovers a design conflict, update the RFC first."*
Both items arrived after acceptance, from work this RFC caused.

**1. `detail_panel.rs`'s label-column widths** (`reviewed/168` §3).
`IDENTITY_LABEL_WIDTH` (56.0) and `STATUS_LABEL_WIDTH` (72.0) were derived when
those labels rendered at 11px. Stage 2 moved them to `body_small` (13.0), and the
comments were rewritten to claim the fit holds *"at the `body_small` role"* -
which nobody measured. `"Untracked:"` needs roughly 65-72px in a 72.0px column.

Either re-derive both, or state in the comment that the fit is unverified and
name 11px as where it was last measured. Stage 4 already opens this file.

**2. `border` has no boundary assertion** (`reviewed/170` §5).
knotra asserts `border` **only as a text colour** - `< AA_NORMAL`, to justify
`NoticeTone` excluding `Tone::Neutral`. Nothing checks that it meets **SC
1.4.11's 3:1 as a boundary**, which is precisely what snora raised it for in
0.34.0 (`light` 1.28 -> 3.12, `dark` 1.19 -> 3.50 against `surface`).

If a future snora release regressed `border`, knotra's only assertion would keep
passing - and pass *more* comfortably, since it asserts the ratio stays *under*
4.5. The repair we absorbed in Stage 1 is unprotected by our own suite.

snora opened RFC-071 for the same shape in their own tests. This is that defect
one layer down, in a consumer.

**Note on which pair binds**: for `dark` the tighter pair is `surface_raised`
(3.17), not `surface` (3.50) - snora's border colour was chosen to clear the
binding pair per preset, `surface` for `light` and `surface_raised` for `dark`.
A boundary assertion should track the binding pair, not the looser one.

**R11 and R12 added.** Both are accessibility gaps this RFC's own work exposed,
and Stage 4 is the last stage - there is no later one to hold them.

## Stages

| Stage | Content | Why here |
|---|---|---|
| 1 | D1 + D2 - the bump, the absorbed contrast repair, corrected figures | One cause for any visual change |
| 2 | D3 + D4 - roles adopted, floor met | The largest, and the user-visible one |
| 3 | D5 - line-height on wrapping text | Depends on Stage 2's roles |
| 4 | D6 - pointer targets measured and met | Independent; last because it is a measurement first |

## Requirements

| # | Requirement |
|---|---|
| R1 | `snora = "0.37"`, `design` + `lucide-icons` retained; workspace builds and all gates pass |
| R2 | No user-facing text renders below **12** logical pixels |
| R3 | No raw `.size(<literal>)` remains in the view tree for user-facing text; sizes come from token roles |
| R4 | Wrapping text carries a line-height from its role's multiplier |
| R5 | `theme.rs`'s border-contrast assertion is re-measured and its comment states the post-0.34 figures |
| R6 | knotra's existing contrast assertions still pass, or their change is reported with the new ratio |
| R7 | The smallest interactive pointer target is measured and reported, and anything under 24x24 is raised |
| R8 | A guard prevents a new sub-floor size or raw literal from re-entering the view tree |
| R9 | `crates/knotra-vcs` is not modified; the suppression map stays at five |
| R10 | **A3.** No text size decreases. Any site whose role assignment would shrink it is reported, not shrunk |
| R11 | **A4.** `detail_panel.rs`'s label-column widths either re-derive at 13px or state the fit as unverified; no comment claims a fit nobody measured |
| R12 | **A4.** `border` is asserted against SC 1.4.11's 3:1 as a boundary, per preset, against the binding surface |

## Test Plan

- **R8's guard** is the durable half. knotra already has three source-scanning
  guards (`suppressions_guard`, `text_outside_catalog_guard`, the catalog pair);
  a fourth asserting "no raw `.size(` literal under `view/`" with an exact
  justified-exception map is the same shape, and the same shape that has caught
  four regressions this cycle.
- Contrast assertions in `theme.rs` are the existing suite and must be re-run
  and re-reported, not assumed.
- Typography and pointer-target changes are **not** pixel-verifiable here. Say
  so, as `070` and `073` did, rather than implying otherwise.

## Security Considerations

None directly. One accessibility note, which is this RFC's whole point: text
below the legibility floor and boundaries below the contrast floor are barriers
for exactly the users least able to work around them, and knotra currently ships
both.

## Migration / rollout

No data, config, or schema change. Users see: slightly more visible borders,
some small text larger, wrapping prose with more breathing room, and larger
click targets where they were collapsed. `high_contrast_light` and
`high_contrast_dark` presets are unaffected by the border repair - they already
passed.
