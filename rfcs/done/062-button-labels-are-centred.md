# RFC-062 - Button labels are centred, and a disabled chip looks disabled

| Field | Value |
|---|---|
| Status | Implemented (main: f09287a) |
| Priority | Medium-high - visible on every main screen, reported by the owner |
| Effort | Small change at 22 sites, plus one helper, one guard, one chip fix |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-ui/src/widget/buttons.rs`, `crates/knotra-ui/src/widget/chip.rs`, the 12 files that build fixed-height buttons |
| Related | `rfcs/done/056-...md` Stage 4 (which set `BUTTON_HEIGHT` 44 / `SMALL_BUTTON_HEIGHT` 36), `.git-exclude/reviewed/211-...md` §6 (the chip finding), `.git-exclude/reviewed/212-...md` (the screenshots that show the defect) |
| Reported by | project owner, 2026-10-18: "invalid alignment on labels in buttons - not beautiful" |

## Summary

Every full-height button in knotra draws its label at the **top** of the button, with the
empty half below it. The cause is one fact about iced and one habit of ours, repeated by hand
at 22 sites. This RFC centres the labels through a single helper, adds a guard so a
hand-built button cannot bring the defect back, and fixes a latent defect in the same widget
layer: a disabled, unselected chip is indistinguishable from an enabled one.

## Problem

### What the screenshots show

In the Task 087 captures, the labels of *Select*, *Start getting latest*, *Save release
point*, and the selection bar's *Check for updates*, *Get latest safely*, *Save release
point* and *Change work area* all sit against the top edge of their buttons. The same
screenshots are now on the README and are about to go to the Microsoft Store.

### Why

**iced 0.14 never centres a button's content.** `button.rs`'s `layout` calls
`layout::padded`, which places the content at the padding offset — the top-left corner of
the content box — and stops there.

**knotra gives buttons a fixed height and a natural-height label**:

```rust
button(text(label).size(…).line_height(…))
    .height(BUTTON_HEIGHT)   // 44 px, from RFC-056 Stage 4's pointer-target work
    .padding([0, 18])        // no vertical padding
```

A body-sized label is about 22 px tall. In a 44 px button with no vertical padding, it sits
at the top and leaves about 22 px empty below. RFC-056 Stage 4 raised these buttons to a
proper pointer-target height, and nothing at the time looked at where the label landed.

**It is written by hand at 22 sites in 12 files.** That is why it appears everywhere at once,
and why fixing it site by site without a structural change would let the next hand-built
button reintroduce it.

### A latent defect in the same layer

knotra's own chip (`crates/knotra-ui/src/widget/chip.rs`, `unselected_style`) signals
**disabled** only through its fill: `Disabled => Color { a: 0.5, ..surface }`, with text and
border unchanged. snora measured exactly this construction at **1.00–1.05:1** against the
enabled fill and fixed theirs in 0.52 — a disabled unselected chip was indistinguishable from
an enabled one.

**Latent today**: the only call site always passes a message, so knotra never renders a
disabled chip. It becomes a real defect the day anyone does.

## Non-goals

- **Changing any button's size, padding, or width.** RFC-056 Stage 4's pointer targets stay.
- **Horizontal centring by stretching.** A label set to fill the button's *width* would make
  content-sized buttons expand across their rows. None of the 22 buttons has a fixed width,
  so their labels are already centred horizontally by symmetric padding. Only vertical
  placement is wrong.
- **Restyling buttons** — colour, border, radius, typography.
- **The snora 0.51/0.52 upgrade.** It is inert for knotra and goes separately.

## Decision

### D1. One helper produces a correctly placed label

`knotra_ui::widget::buttons` gains a label constructor for fixed-height buttons that returns
the text **filling the button's height and centred vertically within it** — iced's
`text(..).height(Length::Fill).align_y(Vertical::Center)`, or equivalent. It never sets the
width.

It takes the typography role the site already uses, so no label changes size.

### D2. All 22 sites use it

Every `button(…)` with `.height(BUTTON_HEIGHT)` or `.height(SMALL_BUTTON_HEIGHT)` builds its
label through D1. Where a site's content is not a plain label — an icon and text in a row, or
a two-line label — the same principle applies: the content fills the height and centres
itself vertically.

### D3. A guard keeps it fixed

A source-scanning test, in the shape of `text_size_guard.rs`: any button given a fixed height
must take its label from D1's helper. A new hand-built fixed-height button fails the build
with a message naming the file and line.

**Seen to fail before trusted**: plant a hand-built button, confirm the guard names it,
remove it.

### D4. A disabled chip carries its state in its label and border

`unselected_style` and `selected_style` dim the **text and border** when disabled, not only
the fill — the same remedy snora adopted. A test asserts that a disabled chip differs from an
enabled one in text colour and border colour, in both themes.

## Requirements

| # | Requirement |
|---|---|
| R1 | D1's helper exists in `knotra-ui`, fills height only, and is the only way fixed-height buttons build labels |
| R2 | All 22 sites migrated; the list of sites is reported, each with what its content is |
| R3 | D3's guard exists, is **seen to fail** on a planted hand-built button, and names the file and line |
| R4 | No button changes size, padding, width, colour, or typography |
| R5 | D4's chip change, with a test asserting disabled and enabled chips differ in text and border, in light and dark |
| R6 | **Before and after screenshots** of the same views as Task 087, from the same privacy-safe demo environment, showing every label centred |
| R7 | An `[Unreleased]` CHANGELOG entry: button labels are centred |
| R8 | Tests rise only by the guard and the chip test; all existing tests pass unchanged |

## Test Plan

- The guard, including the planted failure (R3).
- The chip test, in both themes (R5).
- Full gate set.
- Screenshots of the five Task 087 views, before and after, in the demo environment at
  `/tmp/knotra-demo/` — never one that shows the owner's paths (R6).

## Security Considerations

None. Presentation only; no input handling, process, file, or network change.

## Migration / rollout

User-visible, so it belongs in the next release, **0.30.0**.

**It also affects the Microsoft Store listing.** The screenshots captured in Task 087 show
the misaligned labels. If the first Store submission waits for this fix, its screenshots and
its package will both show the corrected UI; if it goes first, the screenshots should be
recaptured and the listing updated after 0.30.0. The README image should be recaptured either
way.
