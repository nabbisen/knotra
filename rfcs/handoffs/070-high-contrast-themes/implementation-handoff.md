# Developer Handoff — RFC-070: high-contrast themes

RFC: `rfcs/accepted/070-high-contrast-themes.md`, **accepted 2026-10-31 (project owner), with
D5's texts.** Read it in full first, and RFC-071's handoff Amendment A1 D13: this RFC extends
its contrast suite.

**Start after RFC-071 stage 2 is approved.** It may run before or after RFC-071 stages 3 and
4; I set the order when stage 2 is approved. Baseline: `origin/main` at that point. Report
the commit and its test count.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard

The two high-contrast themes restyle all of knotra through tokens alone. No screen, widget
or rule knows which theme is active. Light and dark do not move by one pixel.

**One stage, one review request.**

## Steps

1. **`ThemeChoice` and its saved form (D1).**
   - The enum and `AppConfig.theme`, default `Dark`. Delete `dark_theme` as a field.
   - **Loading:**
     - `theme` wins when present.
     - With only `dark_theme`, it maps to `Dark` or `Light`.
     - An unknown `theme` value falls back to the default with a warning, through
       `load_config`'s existing `(value, Option<String>)` shape.
   - **Saving** writes `theme` only.
   - **Tests:** four round-trips; `dark_theme = true` → `Dark`; `dark_theme = false` →
     `Light`; an unknown value → default plus warning.
   - The existing older-file tests (`config.rs:307`, `:331`, `:342`-`:348`) move to `theme`.
     Report each change.
2. **`KnotraTheme::from_choice` (D2).**
   - Build it from `Tokens::light()`, `dark()`, `high_contrast_light()` and
     `high_contrast_dark()`.
   - Replace `state.rs:593` and `misc.rs:121` with calls to it.
   - Make `SettingsMessage::ThemeChanged` carry a `ThemeChoice`.
   - **`KnotraTheme.dark`:** after RFC-071 stage 2, if nothing reads it, remove it.
     Otherwise report the readers.
   - **R2:** `git grep -n 'KnotraTheme::\(light\|dark\)()' -- crates/knotra-app` finds
     nothing.
3. **Settings (D3).**
   - Replace the two buttons (`settings.rs:141`, `:146`) and `active_theme_note` (`:168`)
     with one `select` from `knotra-ui`, labelled `settings.theme_label`.
   - Options, in this order: Light, Dark, High contrast light, High contrast dark.
   - It joins Settings' focus order where the buttons were. Report the order before and
     after; RFC-066's tests for Settings change by exactly that.
4. **Contrast suite (D4).**
   - Extend A1 D13's suite to the two presets, with D4's high-contrast floors.
   - It must name any role and use that falls short. If one does, **stop and report**. Do
     not override the preset.
5. **Window edges (D4).**
   - In both high-contrast themes, every window is outlined in `border`: dialogs, the
     Resolve sheet, menus, tooltips.
   - Check the code path of each, and report any that relies on a surface tint.
6. **Texts (D5).** Add the two keys, English and Japanese exactly. The unused-key test
   passes.
7. **CHANGELOG `[Unreleased]`:** RFC-070 R7's entry, exactly.

## Evidence (under the GUI evidence rules)

**Question 1:** in each high-contrast theme, is every text, edge, status and focus outline
clearly visible?

- The dashboard with every status present.
- The ready check.
- The command palette with nothing selected.
- One control with the keyboard focus outline.
- Each of these in HC light and HC dark, English.
- Settings with the theme select open, Japanese.

**Question 2 (R6):** do light and dark stay the same?

- Run one script on the baseline build and on your final build: the dashboard, the ready
  check, and the command palette, in light and in dark.
- Compare the pairs **by machine** (pixel difference count). Each pair must be 0.
- Settings is excluded, since D3 changes it on purpose.

**Rules for all captures:**

- isolate all four XDG folders, and run one instance;
- use fictional repositories under `/tmp`;
- save to `.git-exclude/evidence/rfc-070/`, and do not commit them;
- inspect every image for privacy.

## Out of scope

- Following the desktop's own theme or high-contrast setting.
- A custom theme editor.
- Any change to snora's presets.
- Any per-theme special case in any view.
- RFC-071's own stages.

## Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
```

Requirements:

- Suppression map: **five**.
- Dead-code probe: **one** line.
- `advisories ok`.
- Report all test counts, `knotra-vcs` as 20 + 36, and explain the delta.

## What to report back

- The config loading rules and their tests, with each moved older test.
- `from_choice`, R2's grep, and what became of `KnotraTheme.dark`.
- The Settings select, and its focus order before and after.
- The contrast suite's measured table for all four themes.
- The window-edge check, per window kind.
- The catalog diff, and the CHANGELOG entry as committed.
- Question 1's images: paths, sizes, backend, privacy inspection.
- Question 2's pixel counts per pair.
- Gate output, with the `diff --check` range.
