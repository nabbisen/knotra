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

## Moving between projects

| Shortcut | Action |
|---|---|
| `↑` / `k` | Move to the previous project |
| `↓` / `j` | Move to the next project |
| `Enter` | Open the focused project's detail panel |

Arrow and `j`/`k` movement steps between project names, skipping the controls
within each row that `Tab` reaches. Projects in collapsed sections are skipped.
While a text field holds focus these keys type or do nothing, so they never
interrupt a search.

## What is not yet keyboard-accessible

- **The Group and Sort menus cannot be opened by keyboard.** `Tab` reaches them
  and they show a focus ring, but they cannot be opened without a pointer, so
  grouping and sorting cannot be changed by keyboard alone.
- **Selection mode has no single-key shortcuts.** Fetch, pull, tag, and switch
  are reached through the selection bar or the command palette, not by pressing
  `f`, `p`, `t`, or `b`.
- **There is no screen-reader support.** knotra targets iced 0.14, which
  exposes no accessibility API.

These are tracked in `ROADMAP.md` under Phase 6.
