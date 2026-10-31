# RFC-070 - High-contrast themes

| Field | Value |
|---|---|
| Status | Accepted |
| Priority | Medium - for users who find the normal themes hard to read; builds on RFC-071's colour roles |
| Effort | Small - two theme constructors, one setting with a compatible saved form, one Settings control, the contrast suite extended |
| Target | 0.33.0, after RFC-071 stage 2 is approved (`.git-exclude/reviewed/244-release-plan-0.33.0-scope.md` §7) |
| Related files | `crates/knotra-ui/src/theme.rs`, `crates/knotra-app/src/{config,state}.rs`, `crates/knotra-app/src/app/misc.rs`, `crates/knotra-app/src/view/settings.rs`, `crates/knotra-ui/src/i18n.rs` |
| Decided by | project owner, 2026-10-31 - "Accepted", with D5's texts |
| Related | RFC-071 (D3 tone badges, D6 placeholders, D8a lines, handoff A1 D13's contrast suite); RFC-057 (snora palette adopted) |

## Summary

knotra offers a light and a dark theme. Some users, often older users or people with low
vision, need more than either gives. snora 0.53 ships two high-contrast presets. This RFC
offers them in Settings as two more themes, "High contrast light" and "High contrast dark",
and holds them to stricter contrast floors than the normal themes.

Because RFC-071 moves every colour onto snora's tokens, the high-contrast themes restyle the
whole app with **no per-screen work**. This RFC adds the choice, saves it, and proves the
result.

## Problem

Measured at `37f463e`.

1. **Only two themes.**
   - `AppConfig.dark_theme: bool` (`config.rs:35`, default `true`) chooses between
     `KnotraTheme::light()` and `dark()`.
   - That choice is written out twice, in `state.rs:593` and `misc.rs:121`.
2. **The presets exist and are unused.** `snora::design::Tokens::high_contrast_light()` and
   `high_contrast_dark()` (`snora-design` `tokens.rs:53`, `:59`), measured against their
   background:

   | Role | HC light | HC dark |
   |---|---|---|
   | main text | 21.0 : 1 | 21.0 : 1 |
   | secondary text | 17.4 | 16.8 |
   | muted text | 11.4 | 11.5 |
   | accent | 10.2 | 11.8 |
   | warning / danger / success | 8.4 / 8.8 / 8.4 | 14.9 / 10.3 / 13.4 |
   | border | 21.0 | 21.0 |

   The focus ring is 3 px (the presets' `assemble(palette(), 3.0)`), against 2 px in the
   normal themes.
3. **The Settings control is two plain buttons.** "Dark" and "Light" are unstyled iced
   buttons (`settings.rs:141`, `:146`), the current one disabled, followed by a separate
   "Active: Dark" note. A two-way choice built from buttons does not grow to four.

## Design

### D1. One theme choice, saved compatibly

- **A new enum `ThemeChoice`:** `Light`, `Dark`, `HighContrastLight`, `HighContrastDark`.
- **`AppConfig.theme: ThemeChoice`** replaces `dark_theme`. The default is `Dark`, today's
  default.
- **Saved form:** `theme = "light" | "dark" | "high_contrast_light" | "high_contrast_dark"`.
- **Older settings files keep their choice.** A file with `dark_theme` and no `theme` loads
  as `Dark` or `Light` accordingly. The next save writes `theme` only.
- **A file with an unknown `theme` value** loads with the default and a warning, under
  knotra's defaults-plus-warning contract (`config.rs`).
- **Tests:** every saved form round-trips; both older forms load; an unknown value falls
  back.

### D2. One place builds a theme

- **`KnotraTheme::from_choice(ThemeChoice)`** is the only constructor the app calls. It is
  built from the matching `snora::design::Tokens` preset.
- `state.rs` and `misc.rs` call it instead of their own `if dark { … } else { … }`.
- **`KnotraTheme.dark: bool`** stays only if something still reads it after RFC-071 stage 2
  removes `StatusColor`. If nothing does, it is removed. The handoff reports which.

### D3. Settings: one control for the theme

- The two buttons and the "Active:" note give way to one **select**: knotra's own
  `knotra-ui/src/widget/select.rs`, as Group and Sort use.
- **Label:** the existing "Theme". **Options:** the existing "Light" and "Dark", plus D5's
  two new ones.
- The select shows the current choice, so the separate note goes.
- **The change applies at once**, as today, and is saved like every other setting.

### D4. Held to higher floors

RFC-071 handoff A1 D13's contrast suite gains the two presets, with stricter floors:

| Use | Normal themes (A1) | High-contrast themes |
|---|---|---|
| content text | 7 : 1 | 7 : 1 |
| labels of controls, unavailable labels, placeholders | 4.5 : 1 | **7 : 1** |
| lines, outlines, focus ring, badges against the page (non-text) | 3 : 1 | **4.5 : 1** |

- **A window's edge.** In HC light, `surface` equals `background` (both white), so a window
  is told apart from the page by its outline and RFC-066's dim alone. The handoff verifies
  that every window's edge is drawn in `border` in both HC themes, and reports any that
  relies on a surface tint.
- **Nothing else changes per theme.** No screen, widget or rule gets a high-contrast
  special case. If one seems to need it, **stop and report**: it means a colour escaped the
  tokens.

### D5. Texts (owner approval needed)

| Key | English | Japanese |
|---|---|---|
| `settings.theme_high_contrast_light` | High contrast light | ハイコントラスト（ライト） |
| `settings.theme_high_contrast_dark` | High contrast dark | ハイコントラスト（ダーク） |

If nothing else uses `settings.active_prefix` after D3, it stays: the language row still
uses it.

## Requirements

- R1. D1's enum, saved form and compatibility, with its tests.
- R2. `from_choice` is the only constructor the app calls. `git grep` shows no other
  `KnotraTheme::light()` / `dark()` call outside `knotra-ui`.
- R3. Settings has one theme select with four options, and no separate active-theme note.
- R4. The contrast suite covers all four themes with D4's floors, and passes.
- R5. Every window's edge is drawn in `border` in both HC themes.
- R6. **Light and dark are unchanged.** The same script runs on the build before and the
  build after, on the dashboard, a window, and the command palette, in light and dark. The
  captures must be **pixel-identical**, by machine comparison. Settings is the one screen
  allowed to differ.
- R7. CHANGELOG `[Unreleased]`:

```markdown
### Added — high-contrast themes

Settings → Theme now also offers "High contrast light" and "High contrast dark": black and
white, with strong outlines and a thicker focus outline, for anyone who finds the normal
themes hard to read. Your current theme is kept.
```

## Evidence (under the GUI evidence rules)

**The question:** in each high-contrast theme, is every text, edge, status and focus
outline clearly visible?

The captures:

- the dashboard with every status present;
- one window: the ready check;
- the command palette with nothing selected, so that unavailable commands show;
- one control with the keyboard focus outline;
- each in HC light and HC dark, in English;
- Settings with the theme select open, in Japanese.

And R6's machine comparison for light and dark.

## Out of scope

- **Following the desktop's own high-contrast or light/dark setting automatically.** It is
  a separate question, about what the platform reports and how a user overrides it; it can
  have its own RFC later.
- **Custom colours or a theme editor.**
- **Any change to snora's presets.** If D4 finds a preset short of a floor, it goes to the
  snora team, not into a knotra override.

## Owner inputs

1. **Accept**, with D5's texts.
