# RFC-071 handoff — Amendment A6: stage 3's chips, two size classifications, and its evidence

Adds **one commit and one capture set to stage 3**, on top of `df98747`. Ruling of Review
253. Stage 3 is approved once these land. Everything else in stage 3 stays.

## 1. The chips never break their labels

At 1000 px (`after/02-dashboard-1000-en.png`), the last chip shows "All" and "set" on two
lines. A chip's label must never wrap. When the chips do not fit on one line, **the row of
chips wraps** onto a second line instead.

- In `chip.rs`, the label does not wrap (`Wrapping::None`).
- In the toolbar, the chip row wraps its items (iced 0.14's `Row::wrap()`, or an equivalent
  you name in the report).

## 2. Two D14 classifications, corrected

D14 keeps 14 px for incidental metadata only: timestamps, the VCS kind, the branch beside a
name, counts inside metadata, keyboard hints.

- **"Keyboard hints"** means a key shown beside a control, such as "Esc" on a close button.
  The **keyboard shortcuts sheet** is something a user opens to read: every column of it is
  read to understand. **The whole sheet moves to `body`.**
  - Re-measure its three column widths (Task 098) at 16 px, in both languages, the same way
    as before.
  - Report the widths.
- **The detail panel's values** (the path, the branch, the counts) are the content the panel
  exists to show. **The values move to `body`.** The field labels above them stay
  `body_small`, in `text_secondary`, as captions.
  - A commit's author and date stay `body_small` (metadata).
- **Update `text_size_guard`'s map** to the new counts. Report the map.

## 3. The evidence, made comparable

This is stage 3's **second** capture set. A third needs my word first.

- **The same script and the same window size for before and after.** In stage 3's set,
  `before/03` and `before/04` are 1250×1361 while their after images are 2304×1296, so
  those pairs cannot be compared.
- **Before:** `3e55eaa`. **After:** A6's head.
- **History must have entries.** It was the screen whose layout (a result 1,600 px from its
  title) started RFC-071. An empty History does not answer the question.
  - To get entries, run "Check for updates" on fixture repositories that have a local bare
    remote, as every earlier fixture did.
- **Captures,** all at the same logical sizes, English unless marked:
  - Dashboard at 1920 and at 1000;
  - History at 1920 with at least three entries, one of them expanded with "Show details";
  - the ready check at 1920, in English and in Japanese;
  - the keyboard shortcuts sheet at 1920, in English and in Japanese;
  - the detail panel at 1920, in English.
- Save to `.git-exclude/evidence/rfc-071/stage-3-a6/{before,after}/`.

## 4. The report, corrected

Correct two statements in the A6 report, and confirm the two corrections below.

- **D7.** The report said `before/01` shows "content flush against the window's left edge
  with no margin at all", so that centring never worked. `before/01` shows the opposite:
  the toolbar at the left edge and the project list centred, starting about a fifth of the
  way across. Say what the `scrollable` `.width(Length::Fill)` change does in fact. Keep it
  if `align_left` needs it, and show that with one capture without it. Or keep it because
  the scrollbar then sits at the window's edge, and say so. Do not keep the claim.
- **Test count.** 289 + 1 + 35 + 20 + 36 = **381**, not 396.

**Before describing any image in a report, open it and describe what is there.** This is the
second such error (Review 249 §2: "Uncommitted:" for "Dirty:"). From now on, every image
claim in a report names the file it was checked against.

## Report

- the chip change, and its captures at 1000 px, en and ja;
- the sheet's re-measured widths;
- the guard map;
- the corrected D7 statement;
- the evidence: paths, sizes (all pairs equal), backend, privacy inspection;
- gates, with `git diff --check df98747..HEAD`.
