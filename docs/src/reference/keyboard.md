# Keyboard Shortcuts

Every control on every screen is reachable by `Tab` — this page lists the
shortcuts the in-app cheat sheet shows, in the same order. Open it from the
Command palette's "Show keyboard shortcuts" entry; no key opens it directly.

## Everywhere

| Shortcut | Action |
|---|---|
| `Tab` / `Shift+Tab` | Move to the next or previous control |
| `Enter` / `Space` | Press the highlighted control |
| `Esc` | Close the open window, or clear the selection |

Tabbing wraps: from the last control it moves to the first, and back. Inside a
dialog, `Tab` cycles within that dialog only and does not reach the window
behind it. Closing the dialog returns focus to the control that opened it.

## In a list

| Shortcut | Action |
|---|---|
| `↑` / `↓` | Move to the next or previous item |

On the Dashboard specifically, `↑` / `↓` / `j` / `k` move between project
names, skipping the controls within each row that `Tab` reaches, and skipping
projects in a collapsed section. While a text field holds focus these keys
type or do nothing, so they never interrupt a search.

## Main screens

| Shortcut | Action |
|---|---|
| `Ctrl+R` / `⌘R` | Refresh |
| `Ctrl+K` / `⌘K` | Change work area |
| `Ctrl+T` / `⌘T` | Save release point |
| `/` or `Ctrl+/` / `⌘/` | Go to the search field (Dashboard) |
| `Ctrl+Plus` / `Ctrl+=` | Zoom in |
| `Ctrl+Minus` | Zoom out |
| `Ctrl+0` | Reset zoom |

`Ctrl` and the listed shortcuts work the same on every platform; `⌘` is
accepted as an alias for `Ctrl` on macOS.

## What is not yet keyboard-accessible

- **Selection mode has no single-key shortcuts.** Check for updates, get
  latest safely, save a release point, and change work area are reached
  through the selection bar or the command palette once projects are
  selected, not by pressing a single letter key.
- **There is no screen-reader support.** knotra targets iced 0.14, which
  exposes no accessibility API.

These are tracked in `ROADMAP.md` under Phase 6.
