# Dashboard

The Dashboard is knotra's home screen — a card-grid view of every registered repository.

## Card anatomy

Each card shows: project name, VCS badge (Git / jj), current work area
(branch or change-id), status indicator, and Local commits / Updates
available / Unsaved files / New files counts.

## Status indicators

Cards are grouped into three tiers by how much attention they need:
**Needs help** (action required), **In progress** (work or changes waiting),
and **All set** (nothing to do). The Needs help tier is shown first; All set
is collapsed by default.

Each status indicator uses both colour and text (WCAG AA compliant). The
chips at the top of the Dashboard are plain-language: **Needs help** /
**Unsaved work** / **Updates available** / **Local commits** / **All set**.
A project that needs help says why — commonly "A resolution choice is
needed" or "Project folder is missing" — in its own card and in the detail
panel.

## Filter chips, search, and grouping

Click status chips to filter cards. Use the search box to filter by project name. Projects with a `group` field are separated by header rows (alphabetically; ungrouped last).

## Keyboard shortcuts

- Move focus: `Tab` / `Shift+Tab` (wraps) · activate: `Enter` or `Space` · close or clear: `Esc`
- Between projects: `↑` / `↓` / `j` / `k`
- `Ctrl+R` refresh · `Ctrl+K` change work area · `Ctrl+T` save release point · `/` or `Ctrl+/` search

Every control on the Dashboard is reachable by `Tab` — see
[Keyboard Shortcuts](../reference/keyboard.md) for the full list.
