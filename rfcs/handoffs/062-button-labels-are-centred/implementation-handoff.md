# Developer Handoff — RFC-062: button labels are centred, and a disabled chip looks disabled

RFC: `rfcs/accepted/062-button-labels-are-centred.md`, **accepted 2026-10-18 (project owner)**.

Baseline: `d8ef997` (= `origin/main`), **309** tests (229 / 31 / 49), all gates green.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. What must not change, and why this one matters more than usual

**No button changes size, padding, width, colour or typography** (R4). Only where the label
sits inside its button changes.

**Never make a label fill the button's width.** Every one of the 22 buttons is content-sized
horizontally; a label set to fill the width would make those buttons stretch across their
rows. Their labels are already centred horizontally by the symmetric `[0, 18]` padding. Only
the vertical placement is wrong.

**The owner has decided the first Microsoft Store submission waits for this fix.** So your
"after" screenshots (§5) are not just evidence: they become the Store set and the new README
image. They are published. Task 087's privacy rules apply in full.

## 1. The cause, so the fix addresses it rather than a symptom

iced 0.14's `button` layout calls `layout::padded`, which places the content at the padding
offset — top-left — and never centres it. knotra's fixed-height buttons are:

```rust
button(text(label).size(…).line_height(…))
    .height(BUTTON_HEIGHT)   // 44 px; SMALL_BUTTON_HEIGHT is 36
    .padding([0, 18])
```

A ~22 px label in a 44 px button with no vertical padding sits at the top.

## 2. D1 — one helper in `knotra-ui` (R1)

In `crates/knotra-ui/src/widget/buttons.rs`, add a constructor for fixed-height button labels
that returns the text **filling the button's height and centred vertically** — iced's
`text(..).height(Length::Fill).align_y(alignment::Vertical::Center)`.

- It takes the typography role each site already uses (`body`, `body_small`, …) and applies
  that role's size **and** line height, so no label changes size.
- It **never sets the width** (§0).
- Its doc comment states the cause in one sentence — iced places button content top-left —
  so the next reader knows why the helper exists.

Naming and exact signature are yours.

## 3. D2 — migrate all 22 sites (R2)

```
crates/knotra-app/src/view/workspace_manager.rs:221, :250, :307, :336
crates/knotra-app/src/view/selection_bar.rs:296, :314
crates/knotra-app/src/view/overlays/changelog.rs:195, :247
crates/knotra-app/src/view/dashboard/empty.rs:27
crates/knotra-app/src/view/overlays/freezer.rs:407
crates/knotra-app/src/view/dashboard/row.rs:341
crates/knotra-app/src/view/dashboard/toolbar.rs:581, :641
crates/knotra-app/src/view/add_project_modal.rs:35, :73, :99, :167
crates/knotra-app/src/view/overlays/smart_pull.rs:570, :587
crates/knotra-app/src/view/overlays/conflict.rs:367
crates/knotra-app/src/view/overlays/context_switch.rs:350
crates/knotra-ui/src/widget/buttons.rs:56
```

Line numbers are at `d8ef997`; they will move as you edit. **Report each site with what its
content actually is.** Where the content is not a plain label — an icon beside text, or a
label that wraps to two lines — apply the same principle: the content fills the height and
centres itself vertically. If a site does not fit D1 at all, **report it** rather than
forcing it.

If you find a fixed-height button this list missed, that is a finding — add it and say so.

## 4. D3 — a guard (R3)

A source-scanning test in the shape of `crates/knotra-app/src/text_size_guard.rs`, over the
same roots: any `button(…)` given `.height(BUTTON_HEIGHT)` or `.height(SMALL_BUTTON_HEIGHT)` —
including `layout::`-qualified spellings — must take its label from D1's helper. A violation
fails with the file and line.

It must cope with multi-line formatting; most of these calls span several lines.

**Seen to fail before trusted**: plant one hand-built fixed-height button, run the guard,
quote its failure message, remove the plant.

The project rule for scanning guards applies: they match prose as readily as code, so a
comment that describes the pattern can trip it. **Reword the comment; never widen the
guard's exclusions** to make it pass.

## 5. D4 — a disabled chip looks disabled (R5)

`crates/knotra-ui/src/widget/chip.rs`: `unselected_style` and `selected_style` currently
signal `Disabled` only through the fill. Dim the **text and border** too, derived from the
existing tokens — snora's palette has no dedicated disabled colour, and snora's own 0.52 fix
dims text and border the same way. **Enabled appearance must not change.**

A test, in **both themes**: a disabled chip's text colour and border colour each differ from
the enabled chip's.

Disabled controls are exempt from WCAG's contrast minimums (SC 1.4.3 and 1.4.11 both exclude
inactive components), so dimming below 4.5:1 is correct here. Do not add a contrast-floor
assertion for the disabled state.

No call site renders a disabled chip today. This fix makes the primitive correct before one
does.

## 6. Screenshots — before, after, and what they become (R6)

**Before**: the Task 087 sets already exist at `.git-exclude/evidence/store-screenshots/en/`
and `/ja/`. Leave them untouched.

**After**: recapture the same five views, in English and in Japanese, from the demo
environment at `/tmp/knotra-demo/` (it still exists; if it is gone, rebuild it exactly as
Task 087 §1 describes). Same window size, same themes, same selections as the "before" set,
so the two compare one to one.

Save to `.git-exclude/evidence/store-screenshots-0.30/en/` and `/ja/`. **Not committed.**

**Task 087 §0's privacy rule governs every image**: nothing private — no username, no real
path, no hostname, no real repository name, no other window. Inspect each one.

**Prove R4 from the images**: for at least the *Select* button and the selection bar's four
buttons, report each button's outline position and size before and after. They must be
identical; only the label moves.

**Replace the README image**: copy the new `en/01-dashboard.png` over
`docs/src/images/dashboard.png`, in its **own commit**, separate from the code. The README's
link does not change.

## 7. CHANGELOG (R7)

Under `## [Unreleased]`:

```markdown
### Fixed — button labels sat at the top of their buttons

Labels on the larger buttons — such as Select, Get latest safely and Save release
point — were drawn against the top edge, leaving the lower half of the button
empty. They are now centred.
```

The chip fix is not user-visible today and gets no entry.

## 8. Out of scope

- Restyling buttons in any way (§0).
- The snora 0.51/0.52 upgrade — scheduled separately, after this.
- Any other visual problem you notice while capturing: **report it**, do not fix it here.
- `crates/knotra-vcs`.

## 9. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check d8ef997..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
```

Tests rise by the guard and the chip test only; name them and give the new total. All 309
existing tests pass unchanged. Suppression map **five**; dead-code probe **one** line.

## 10. What to report back

- D1's helper, quoted;
- **all 22 sites**, each with its content type, and any site that did not fit D1;
- the guard, and **its planted failure message, quoted**;
- the chip change and its test;
- every new image: file name, measured pixel size, byte size;
- **R4's button-outline comparison**, before and after;
- confirmation you inspected every new image for private data;
- the README image commit, separately;
- the CHANGELOG entry;
- gate output, gate five in the range form.
