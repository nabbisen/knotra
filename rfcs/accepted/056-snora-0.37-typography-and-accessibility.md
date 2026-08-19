# RFC-056 - snora 0.25 to 0.37: typography and accessibility

| Field | Value |
|---|---|
| Status | Accepted (2026-08-19, project owner) |
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
