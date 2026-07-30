# Keyboard Shortcuts

## Navigation

| Shortcut | Action |
|---|---|
| `Tab` | Move focus to the next control |
| `Shift+Tab` | Move focus to the previous control |
| `Enter` / `Space` | Activate the focused control |
| `Esc` | Close the open modal or dialog |

While a text input holds focus, `Enter`, `Space`, and `/` go to the input as
typed characters rather than acting as shortcuts.

Inside a dialog, `Tab` cycles within that dialog and does not reach the window
behind it. Closing the dialog returns focus to the control that opened it.

## Commands

| Shortcut | Action |
|---|---|
| `Ctrl+R` / `⌘R` | Refresh all project statuses |
| `Ctrl+K` / `⌘K` | Open Context Switch |
| `Ctrl+T` / `⌘T` | Open Freezer (Tag modal) |
| `/` | Focus the search box |
| `Ctrl+/` / `⌘/` | Focus the search box (returns to Dashboard) |

## What is not yet keyboard-accessible

knotra's focus model is new as of 0.24.0 and does not cover everything yet:

- **The Dashboard shows no focus indicator.** `Tab` does move focus through
  section headers, row checkboxes, project names, and row actions, and `Enter`
  or `Space` activates whichever one holds focus — but none of them draws a
  focus ring, so you cannot see where focus currently is. Only the top bar and
  the workspace dialogs indicate focus visually.
- **Arrow keys and `j` / `k` do not move between project cards**, and there is
  no single-key shortcut to open the focused card's detail panel beyond `Enter`
  on the focused project name.
- **Selection mode has no single-key shortcuts.** Fetch, pull, tag, and switch
  are reached through the selection bar or the command palette, not by pressing
  `f`, `p`, `t`, or `b`.
- **There is no screen-reader support.** knotra targets iced 0.14, which
  exposes no accessibility API.

These are tracked in `ROADMAP.md` under Phase 6.
