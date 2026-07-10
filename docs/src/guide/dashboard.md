# Dashboard

The Dashboard is knotra's home screen — a card-grid view of every registered repository.

## Card anatomy

Each card shows: project name, VCS badge (Git / jj), current context (branch or change-id), status indicator, Ahead/Behind/Uncommitted/Untracked counts, and the time of the last status read.

## Status indicators

Cards are grouped into three tiers by how much attention they need:
**Needs help** (action required), **In progress** (work or changes waiting),
and **All set** (nothing to do). The Needs help tier is shown first; All set
is collapsed by default.

Each status indicator uses both colour and text (WCAG AA compliant). The
first-level labels are plain-language: **All set** / **Updates available** /
**Unshared changes** / **Unsaved work** / **Needs your choice** / **Not sure
yet**. The exact technical state (Synced / Behind / Ahead / Uncommitted /
Conflict / Unknown) is shown in the project detail panel and operation
history under "Show details".

## Filter chips, search, and grouping

Click status chips to filter cards. Use the search box to filter by project name. Projects with a `group` field are separated by header rows (alphabetically; ungrouped last).

## Keyboard shortcuts

`Ctrl+R` refresh · `Ctrl+K` context switch · `Ctrl+T` tag/freeze · `Ctrl+/` search · `Esc` close modal  
Selection mode: `f` fetch · `p` pull · `t` tag · `b` switch · `Space` toggle selection
