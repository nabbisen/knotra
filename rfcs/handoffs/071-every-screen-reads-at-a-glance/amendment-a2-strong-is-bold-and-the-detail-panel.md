# RFC-071 handoff — Amendment A2: "strong" is bold; the detail panel's labels stack

Adds **two commits to stage 1**, on top of `b0e2a45`. Ruling of Review 248. Stage 1 is
approved only once these land; stage 2 starts after that. Everything else in the handoff
and in A1 stands.

## Why

1. **Japanese strong text changes typeface.**
   - Requesting `Weight::Semibold` makes the font system look for a 600-weight face for
     Japanese. On this machine, Noto Sans CJK JP has Regular, Medium and Bold, but no
     SemiBold. Noto Serif CJK JP has SemiBold. So the serif (Mincho) face wins (`fc-list
     :lang=ja`).
   - In stage 1's after images, 「実行内容」, 「リリースポイントを記録」 and the group headers
     are drawn in a different, serif typeface, while the before images and every regular
     line are sans (Gothic).
   - Mixed typefaces break the RFC's standard. Weight was meant to reinforce, and here it
     changes the voice of the text.
   - **Bold (700)** exists in the sans family of every script on all three platforms:
     - Linux: Noto Sans and Noto Sans CJK JP Bold;
     - Windows: Segoe UI and Yu Gothic UI Bold;
     - macOS: the system font and Hiragino Sans W6/W7.
2. **The detail panel's labels were sized by guess, and D1 makes the guess wrong.**
   - `IDENTITY_LABEL_WIDTH` (67) and `STATUS_LABEL_WIDTH` (86)
     (`view/detail_panel.rs:66`, `:71`) were scaled from older sizes. Their own comments say
     "not re-verified against a render".
   - At 14 px, "Remote:" needs about 71 px and "Untracked:" about 92 px. The labels then
     break mid-word.
   - Any fixed label width is a guess for the next font, platform or language. A layout
     that needs no width removes the guess.

## Commit 1 — `strong` is bold

- `strong()` applies `iced::font::Weight::Bold`. Update its doc comment: why Bold, and
  which fonts led to it.
- `text_weight_guard` is unchanged: the one site is the same.
- **D2's check gains a condition.** Strong text must be visibly heavier **and in the same
  typeface as the regular text beside it**, in Latin and Japanese. A change of typeface is
  a **stop and report**, like smearing.

## Commit 2 — the detail panel's labels sit above their values

- In `detail_panel.rs`, each field shows its label on its own line, with the value directly
  below it. The label keeps its role (`body_small`), in `text_secondary`. The value keeps
  its role.
- Delete `IDENTITY_LABEL_WIDTH`, `STATUS_LABEL_WIDTH` and `field_row`'s `label_width`
  parameter.
- A long value (a path) wraps under its own label, so nothing has to be measured.
- No text changes.

## Evidence

This is stage 1's **second** capture set. A third needs my word first.

- **The question:** is strong text heavier and in the same typeface as regular text, in
  both languages? And does every detail-panel label sit whole on its line?
- **Captures:** the same script as stage 1's set, run on `b0e2a45` (before) and on A2's
  head (after):
  - Dashboard and History, English and Japanese;
  - the detail panel of the conflicted project, English and Japanese.
- Save to `.git-exclude/evidence/rfc-071/stage-1-a2/{before,after}/`.
- **Dispatch the release workflow on A2's head** and report the run ID. The owner's Windows
  check uses that run's package, not `41182688012`.

## CHANGELOG

No new entry. Stage 1's entry stands. "Drawn stronger" is still true.

## Report

- `strong()`'s diff.
- The detail panel's diff, and `git grep -n 'LABEL_WIDTH'` empty.
- **D2's check, part by part, first.** For each image, name the typeface of the strong text
  against the regular text (Gothic / Mincho, sans / serif) as found.
- Paths, sizes, backend, privacy inspection.
- Gates, with `git diff --check b0e2a45..HEAD`, and the dispatched run ID.
