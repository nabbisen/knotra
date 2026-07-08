# Dashboard

The Dashboard is knotra's home screen — a card-grid view of every registered repository.

## Card anatomy

Each card shows: project name, VCS badge (Git / jj), current context (branch or change-id), status indicator, Ahead/Behind/Uncommitted/Untracked counts, and the time of the last status read.

## Status indicators

All indicators use both colour and text (WCAG AA compliant): **Synced** / **Behind** / **Ahead** / **Uncommitted** / **Conflict** / **Unknown**.

## Filter chips, search, and grouping

Click status chips to filter cards. Use the search box to filter by project name. Projects with a `group` field are separated by header rows (alphabetically; ungrouped last).

## Keyboard shortcuts

`Ctrl+R` refresh · `Ctrl+K` context switch · `Ctrl+T` tag/freeze · `Ctrl+/` search · `Esc` close modal  
Selection mode: `f` fetch · `p` pull · `t` tag · `b` switch · `Space` toggle selection
