# RFC-017 — Screen Removal and Sidebar Cleanup

| Field          | Value                                                                     |
|----------------|---------------------------------------------------------------------------|
| Status      | Proposed |
| Priority       | Low — cleanup phase; deletes code rather than adding behaviour            |
| Effort         | Medium — careful removal across many files; deprecation messaging         |
| Target version | v0.16                                                                     |
| Related        | RFC-009 through RFC-016 (all redesign RFCs)                             |

## Summary

Remove the five screens replaced by modals in RFC-013 (Sync Center,
Freezer, Context Ops, Conflict Resolution, Changelog), and reduce the
sidebar to just Settings and Help (or eliminate it entirely if those
become accessible via the global command palette).  This is the final
step of the UI/UX redesign: the cleanup that pays the maintenance
dividend.

## Background

Once all bulk actions live in modals (RFC-013) and all global navigation
goes through the command palette (RFC-012) or workspace tabs (RFC-015),
the original sidebar navigation has very few entries left.  Each screen
still in place costs:

- Maintenance: View module, message variants, state struct, tests.
- Cognitive load: two ways to do the same thing.
- Visual clutter: sidebar entries that no longer fit the new model.

This RFC removes the no-longer-needed screens, ending the v0.11 → v0.16
migration cleanly.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | Remove `Screen::SyncCenter`, `Screen::Freezer`, `Screen::ContextOps`, `Screen::ConflictResolution`, `Screen::Changelog` |
| R2  | Remove the corresponding view modules: `view/sync_center.rs`, `view/freezer.rs`, `view/context_ops.rs`, `view/conflict_ops.rs`, `view/changelog_view.rs` |
| R3  | Keep the state modules they depend on; modals use the same state |
| R4  | The History screen remains (full-screen for browsing the complete log) |
| R5  | The Settings screen remains (full-screen for configuration) |
| R6  | The Dashboard becomes the only "main" screen; everything else is overlay or panel |
| R7  | Sidebar: keep only Settings (`⚙`) and Help (`?`) — and only if the user opts in via a Settings toggle to "Show sidebar" |
| R8  | Document the removal in `CHANGELOG.md` and `docs/src/migration/v0.16.md` |
| R9  | Provide a one-time migration notice to users on first v0.16 launch |

## External Design

### Final UI shape

```
┌─────────────────────────────────────────────────────────────┐
│ knotra   [work] [personal] [+]              ⟳ ⚙ ?  [search] │  ← top bar
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   🔴  Needs attention (2)                                    │
│   🟡  Active (4)                                             │
│   ⚪  Clean (24)   ▶                                          │
│                                                              │
│                                                              │
│   [4 selected — Fetch  Pull  Tag…  Switch…  ⋯]              │  ← selection bar
│                                                              │
│   ⓘ Last: Fetched 28 projects · 28 ok                  [›]   │  ← activity strip
└─────────────────────────────────────────────────────────────┘
```

- No sidebar (the icons `⚙` and `?` move to the top bar).
- Three layers: dashboard, selection bar, activity strip.
- Modals overlay; detail panel docks right; cheat sheet overlays.

### What still exists

| Element              | Location                                            |
|----------------------|-----------------------------------------------------|
| Dashboard            | Main view                                           |
| Settings             | Full-screen, reached via `⚙` icon or palette        |
| History              | Full-screen, reached via popover or palette         |
| Help                 | Full-screen, reached via `?` icon (NOT same as cheat sheet `?` key) |
| Command palette      | Overlay (RFC-012)                                  |
| Cheat sheet          | Overlay (RFC-016)                                  |
| Workspace tabs       | Top bar (RFC-015)                                  |
| Detail panel         | Right dock (RFC-014)                               |
| Resolve panel        | Right dock (RFC-013)                               |
| Selection bar        | Bottom (RFC-009)                                   |
| Activity strip       | Bottom (RFC-011)                                   |

### One-time migration banner

On first launch of v0.16 (detected by comparing previous version in config),
a dismissible top banner appears:

```
┌─────────────────────────────────────────────────────────────────┐
│ ⓘ knotra has been redesigned. Bulk actions are now on the       │
│   selection bar at the bottom. Press ? to see the new keyboard  │
│   shortcuts.                          [Show what's new] [Dismiss]│
└─────────────────────────────────────────────────────────────────┘
```

"Show what's new" opens `docs/src/migration/v0.16.md` in the help screen.
"Dismiss" clears the banner permanently (stored as `seen_v0.16_intro: true`
in config).

## Internal Design

### Code removal

```
crates/knotra-app/src/
  state/
    sync.rs           ← keep (modal uses it)
    freezer.rs        ← keep (modal uses it)
    context.rs        ← keep (modal uses it)
    conflict_ops.rs   ← keep (panel uses it)
    changelog.rs      ← keep (modal uses it)
  view/
    sync_center.rs    ← REMOVE
    freezer.rs        ← REMOVE  (replaced by view::modal::tag)
    context_ops.rs    ← REMOVE
    conflict_ops.rs   ← REMOVE
    changelog_view.rs ← REMOVE
    dashboard.rs      ← keep
    history.rs        ← keep
    settings.rs       ← keep
    modal/
      pull.rs         ← new in RFC-013
      tag.rs          ← new in RFC-013
      switch.rs       ← new in RFC-013
      changelog.rs    ← new in RFC-013
      resolve.rs      ← new in RFC-013 (docked panel, not centered modal)
    detail.rs         ← new in RFC-014
    activity.rs       ← new in RFC-011
    shortcuts.rs      ← new in RFC-016
    palette.rs        ← new in RFC-012
    workspace_tabs.rs ← new in RFC-015
```

### Screen enum cleanup

```rust
// Before (v0.11.0)
pub enum Screen {
    Dashboard,
    SyncCenter,
    ContextOps,
    Freezer,
    History,
    Settings,
    ConflictResolution,
    Changelog,
}

// After (v0.16)
pub enum Screen {
    Dashboard,
    History,
    Settings,
}
```

### Message variants cleanup

`Message::Sync`, `Message::Context`, `Message::Freezer`, `Message::ConflictOps`,
`Message::Changelog` continue to exist — they now dispatch to modal /
panel handlers instead of full-screen view handlers.  The variant names
are kept for code stability; only the dispatch destination changes.

### Sidebar removal

```rust
// view/mod.rs (before)
pub fn view(state: &AppState) -> Element<'_, Message> {
    row![sidebar::view(state), screen::view(state)].into()
}

// view/mod.rs (after)
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        workspace_tabs::view(state),
        main::view(state),     // Dashboard / Settings / History switcher
        selection_bar::view(state).unwrap_or(empty()),
        activity::strip(state).unwrap_or(empty()),
    ]
    .into()
}
```

If a user opts to keep the sidebar:

```rust
pub fn view(state: &AppState) -> Element<'_, Message> {
    let main_area = column![
        workspace_tabs::view(state),
        main::view(state),
        selection_bar::view(state).unwrap_or(empty()),
        activity::strip(state).unwrap_or(empty()),
    ];
    if state.config.show_sidebar {
        row![sidebar::view(state), main_area].into()
    } else {
        main_area.into()
    }
}
```

The sidebar in that case contains only Settings and Help links.

### Settings → "Migration" tab

A new tab in Settings: "Migration."

```
Settings → Migration
  ☐ Show sidebar (legacy)
  ☐ Show filter chips on Dashboard (legacy)
  
  These options preserve v0.11 behaviours.
  They will be removed in v0.17.
```

This gives users a final escape hatch for one release, after which the
options are removed.

### Test suite cleanup

Tests for removed view modules should be deleted along with the modules.
Tests for the state modules (which continue to be used by modals) remain.

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.16   | Remove view modules; remove sidebar (default); add migration toggles |
| 2     | v0.17   | Remove migration toggles (no more "legacy mode") |

The deprecation banner stays for one release only.

## Test Plan

### Unit tests

1. **`removed_screens_not_in_enum`** — `Screen` enum has only Dashboard,
   History, Settings.
2. **`sidebar_hidden_by_default`** — fresh config → `show_sidebar = false`.
3. **`migration_banner_shows_on_first_v0_16_launch`** — config previous
   version v0.11.0 → banner state = true.
4. **`migration_banner_dismissed_persists`** — set seen_v0_16_intro =
   true → banner does not appear on next launch.

### Manual

1. Upgrade from v0.11.0 → v0.16: banner appears.  Dismiss.  Restart →
   banner does not return.
2. Sidebar absent by default; verify Settings reachable via top-right `⚙`.
3. Settings → Migration → enable "Show sidebar": sidebar reappears with
   Settings + Help only.

## Open Questions

### Q1 — Help screen content

What goes in the Help screen?  **Tentative answer**: an in-app version of
`docs/src/quickstart.md` plus links to:
- Online full documentation
- GitHub repository
- Report an issue
- Cheat sheet (?)
- Migration guide for users coming from older versions

### Q2 — What about external Markdown links?

Documentation lives in `docs/src/`.  Should the Help screen render
embedded Markdown or open the browser?  **Tentative answer**: render
embedded.  Use a small subset (`pulldown-cmark` already in dependency
tree).

### Q3 — Sidebar resurrection paths

If a user disables the sidebar, then disables the top bar Settings/Help
buttons (via custom styling, not exposed by knotra), they may be stuck
with no way to reach Settings.  **Mitigation**: top bar buttons cannot
be disabled.  Defensive `⌘,` shortcut always opens Settings.

## Security Considerations

None.  Removal of UI code does not affect security boundaries.  All
operations and their permissions remain unchanged.
