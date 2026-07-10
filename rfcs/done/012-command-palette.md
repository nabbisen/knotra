# RFC-0012 — Command Palette (⌘K / Ctrl+K)

| Field          | Value                                                                  |
|----------------|------------------------------------------------------------------------|
| Status      | Implemented (v0.12.0) |
| Priority       | Medium — discoverability + power-user speed                            |
| Effort         | Medium — new overlay, fuzzy search, action registry                    |
| Target version | v0.12 (stub) → v0.13 (full)                                            |
| Related        | All redesign RFCs — every action gets a palette entry                  |

## Summary

Add a centered, modal command palette triggered by `⌘K` / `Ctrl+K`.  It
fuzzy-searches over a registry of actions (Fetch, Pull, Tag, switch
workspace, open settings, navigate to project, etc.) and executes the
chosen action on `Enter`.  Solves the discoverability problem created by
removing sidebar items, and gives keyboard-first users a faster path to
every feature.

## Background

The redesign removes sidebar entries for Sync Center, Freezer, ContextOps,
Conflict Resolution, and Changelog (RFC-0017).  Without those entries,
users need another way to discover what knotra can do.  A command palette
serves three purposes:

1. **Discoverability** — typing a keyword surfaces relevant actions.
2. **Keyboard speed** — power users execute any action in <1 second.
3. **Navigation** — jump to a specific project by name without scrolling.

This pattern is established in modern tools (VS Code, GitHub, Linear,
Slack).  Users transferring from those tools expect `⌘K`.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | `⌘K` (macOS) or `Ctrl+K` (Linux/Windows) opens the palette |
| R2  | The palette is a centered overlay over the main view |
| R3  | Typing filters the action list using fuzzy matching |
| R4  | `Up` / `Down` arrows move the selection |
| R5  | `Enter` executes the selected action |
| R6  | `Esc` closes the palette without action |
| R7  | The palette searches three categories: actions, projects, workspaces |
| R8  | Categories appear as headers within the result list |
| R9  | Actions show their keyboard shortcut on the right of the row, if any |
| R10 | The palette is i18n-aware: action labels render in the user's language |
| R11 | The palette remembers the most recently used items and surfaces them when empty |
| R12 | When a selection exists, action labels reflect the selection (e.g., "Tag 4 selected projects…") |
| R13 | The palette is registered as part of the standard event loop; no separate window |

## External Design

### Visual

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                  │
│         ┌─────────────────────────────────────────────────┐     │
│         │  🔍  pull_                                       │     │
│         │  ────────────────────────────────────────────── │     │
│         │  ACTIONS                                          │     │
│         │  → Smart pull 4 selected projects     ⏎          │     │
│         │    Pull all                                       │     │
│         │                                                   │     │
│         │  PROJECTS                                         │     │
│         │    pull-stream-tool                               │     │
│         │                                                   │     │
│         │  Press ↑↓ to navigate · ⏎ to select · Esc cancel │     │
│         └─────────────────────────────────────────────────┘     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

- Width: 600 px (fixed).
- Vertical position: 25% from top.
- Search field with leading magnifier icon and placeholder
  "Type to search actions, projects…".
- Below: scrollable list of results grouped by category.
- Selected row highlighted with the accent colour.
- Bottom: keyboard help line.

### Categories and ordering

Results are grouped and ordered:

1. **Actions** — operations the user can run.  Top priority because most
   palette usage is "do something."
2. **Projects** — for fast navigation; selecting a project focuses it on
   the dashboard.
3. **Workspaces** — for fast switching.

Within each category, fuzzy-match score determines order.

### Empty-query behaviour

When the search field is empty, the palette shows:

1. The 5 most recently used actions (persisted across sessions).
2. The 5 most recently focused projects.
3. All workspaces (typically few).

Recently-used is the killer feature for power users; once "Tag selected
projects" has been used, it's instant via `⌘K ⏎`.

### Action labels with selection awareness

Several actions are context-sensitive.  Labels reflect the current
selection:

| Selection state                | Action label                          |
|--------------------------------|---------------------------------------|
| 0 selected, workspace has 28    | "Fetch all (28 projects)"             |
| 0 selected                      | "Smart pull (no selection)" — disabled |
| 4 selected                      | "Tag 4 selected projects…"            |
| 1 selected                      | "Switch branch on 1 selected project…" |

Disabled actions are visible but greyed out and unselectable.

### Fuzzy matching

A standard subsequence-with-bonus algorithm:

- Match `pl` against `Smart pull all`: matches `p`(in "pull") then `l`(in
  "pull") — score boosted by being at word boundaries.
- Match `tg` against `Tag selected projects` — same.
- Match `ws-2` against `Switch to workspace work-2` — substring + numeric
  detection bonus.

The fuzzy crate to use: [`nucleo-matcher`](https://crates.io/crates/nucleo-matcher)
(MIT, pure Rust, used by Helix editor).  ~30 KB binary cost.

### Recently used persistence

```
~/.local/share/knotra/palette_recent.json
```

Schema:
```json
{
  "actions":   ["action.tag_selected", "action.fetch_all", ...],
  "projects":  ["uuid-1", "uuid-2", ...],
  "max_each":  10
}
```

Updated on every successful action; pruned to `max_each` entries.

### Interaction details

| Trigger        | Effect                                                      |
|----------------|-------------------------------------------------------------|
| `⌘K` / `Ctrl+K` | Open palette (or focus search field if already open)        |
| Type characters | Update search results live (debounced 50 ms)                |
| `↑` / `↓`       | Move selection within results                               |
| `Tab`           | Move focus to next category                                 |
| `Enter`         | Execute selected action                                     |
| `Esc`           | Close palette                                               |
| Click row       | Execute that row's action                                   |
| Click outside   | Close palette                                               |

## Internal Design

### New types

```rust
// state/palette.rs
pub struct PaletteState {
    pub open:           bool,
    pub query:          String,
    pub results:        Vec<PaletteEntry>,
    pub selected_index: usize,
    pub recent_actions: Vec<String>,
    pub recent_projects: Vec<ProjectId>,
}

#[derive(Clone, Debug)]
pub enum PaletteEntry {
    Action  { id: String, label: String, shortcut: Option<String>, enabled: bool },
    Project { id: ProjectId, name: String },
    Workspace { id: WorkspaceId, name: String },
}
```

### Messages

```rust
pub enum PaletteMessage {
    OpenRequested,
    Closed,
    QueryChanged(String),
    SelectionMoved(i32),  // +1 down, -1 up
    EntryActivated,        // Enter or click
    EntryClicked(usize),   // direct index
}
```

### Action registry

A central registry of all palette-accessible actions.  Each action is a
`PaletteAction`:

```rust
// state/palette.rs
pub struct PaletteAction {
    pub id:        &'static str,    // stable, used in recent_actions
    pub i18n_key:  &'static str,    // "palette.action.tag_selected"
    pub shortcut:  Option<&'static str>,  // "t" or "Ctrl+R"
    /// Returns the label string given current state (for selection-aware labels).
    pub label_fn:  fn(&AppState) -> String,
    /// Returns true if the action is currently enabled.
    pub enabled_fn: fn(&AppState) -> bool,
    /// Dispatch function.
    pub dispatch:  fn(&mut AppState) -> Task<Message>,
}

pub fn all_actions() -> &'static [PaletteAction] {
    static ACTIONS: &[PaletteAction] = &[
        PaletteAction {
            id: "fetch_all",
            i18n_key: "palette.fetch_all",
            shortcut: Some("f"),
            label_fn: |state| {
                let n = state.workspace.as_ref().map(|ws| ws.projects.len()).unwrap_or(0);
                state.t_with("palette.fetch_all_count", &[("n", &n.to_string())])
            },
            enabled_fn: |state| !state.is_refreshing,
            dispatch: |state| start_bulk_fetch_all(state),
        },
        PaletteAction {
            id: "tag_selected",
            i18n_key: "palette.tag_selected",
            shortcut: Some("t"),
            label_fn: |state| {
                let n = state.selection.len();
                if n == 0 {
                    state.t("palette.tag_no_selection")
                } else {
                    state.t_with("palette.tag_selected_count", &[("n", &n.to_string())])
                }
            },
            enabled_fn: |state| !state.selection.is_empty(),
            dispatch: |state| open_tag_modal(state),
        },
        // ... ~30 actions total
    ];
    ACTIONS
}
```

### Search

```rust
// state/palette.rs
pub fn compute_results(query: &str, state: &AppState) -> Vec<PaletteEntry> {
    if query.is_empty() {
        return recent_entries(state);
    }

    let matcher = nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT);
    let pattern = nucleo_matcher::pattern::Pattern::parse(query,
        nucleo_matcher::pattern::CaseMatching::Smart);

    let mut scored: Vec<(i64, PaletteEntry)> = Vec::new();

    for action in all_actions() {
        let label = (action.label_fn)(state);
        if let Some(score) = pattern.score(label.as_str(), &matcher) {
            scored.push((score, PaletteEntry::Action {
                id: action.id.to_string(),
                label,
                shortcut: action.shortcut.map(str::to_string),
                enabled: (action.enabled_fn)(state),
            }));
        }
    }
    if let Some(ws) = &state.workspace {
        for p in &ws.projects {
            if let Some(score) = pattern.score(p.name.as_str(), &matcher) {
                scored.push((score, PaletteEntry::Project {
                    id: p.id.clone(),
                    name: p.name.clone(),
                }));
            }
        }
    }
    for ws in &state.all_workspaces {
        if let Some(score) = pattern.score(ws.name.as_str(), &matcher) {
            scored.push((score, PaletteEntry::Workspace {
                id: ws.id.clone(),
                name: ws.name.clone(),
            }));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));  // highest score first
    scored.into_iter().take(20).map(|(_, e)| e).collect()
}
```

### Subscription

Global keyboard subscription gains:

```rust
keyboard::on_key_press(|key, modifiers| match (key, modifiers) {
    (Key::Character(c), m) if c == "k" && (m == Modifiers::CTRL || m == Modifiers::COMMAND) =>
        Some(Message::Palette(PaletteMessage::OpenRequested)),
    _ => None,
})
```

When the palette is open, all other keyboard handling is suppressed.

### View

```rust
// view/palette.rs
pub fn overlay<'a>(state: &AppState) -> Option<Element<'a, Message>> {
    if !state.palette.open { return None; }

    let search = text_input("Type to search…", &state.palette.query)
        .on_input(|s| Message::Palette(PaletteMessage::QueryChanged(s)));

    let rows: Vec<Element<Message>> = state.palette.results.iter().enumerate()
        .map(|(i, entry)| palette_row(state, i, entry))
        .collect();

    let body = column![
        row![text("🔍").size(18), search].spacing(8),
        column(rows).spacing(0),
        text(state.t("palette.help_line")).size(11).muted(),
    ].spacing(8).padding(16);

    Some(modal_overlay(state, body))  // centered overlay component
}
```

### Project navigation

When a `PaletteEntry::Project` is activated:

```rust
PaletteMessage::EntryActivated => {
    match &state.palette.results[state.palette.selected_index] {
        PaletteEntry::Project { id, .. } => {
            state.focused_project = Some(id.clone());
            state.palette.open = false;
            // Scroll dashboard to this project; expand its tier if needed.
            Task::done(Message::Dashboard(DashboardMessage::ScrollTo(id.clone())))
        }
        // ... action dispatch, workspace switch
    }
}
```

### Workspace switch via palette

```rust
PaletteEntry::Workspace { id, .. } => {
    state.palette.open = false;
    Task::done(Message::Workspace(WorkspaceMessage::WorkspaceSwitched(id.clone())))
}
```

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.12   | Palette overlay, search field, simple substring matching, project navigation, workspace switching.  No selection-aware action labels yet. |
| 2     | v0.13   | Action registry with ~30 actions; nucleo-matcher integration; selection-aware labels |
| 3     | v0.14   | Recently-used persistence; recent_entries on empty query |
| 4     | v0.16   | Polish: keyboard shortcuts shown in rows; categorisation headers |

## Test Plan

### Unit tests

1. **`palette_empty_query_shows_recent`** — recent_actions = [a, b];
   compute_results("") starts with a, b.
2. **`palette_fuzzy_matches_partial`** — query "pl" matches "Smart pull" with
   a positive score.
3. **`palette_disabled_action_appears_disabled`** — `tag_selected` with
   `selection.is_empty()` → enabled = false.
4. **`palette_workspace_results_ordered_by_score`** — query "wrk" with
   workspaces "work", "personal", "workspace-3" → "work" first.
5. **`palette_max_20_results`** — query matching 50 things → at most 20
   returned.
6. **`palette_close_clears_query`** — open, type, close → next open starts
   with empty query.

### Manual test plan

1. `⌘K` → palette opens.
2. Type "fet" → "Fetch all" is the top result.
3. Press Enter → fetch starts, palette closes.
4. `⌘K` again → "Fetch all" is now first in the recent-used list.
5. Type partial project name → project entry appears in Projects section.
6. Press Enter → palette closes, that project's card is focused on the
   dashboard.
7. `Esc` mid-search → palette closes without action.

## Open Questions

### Q1 — Should the palette show keyboard shortcuts on first launch?

A first-launch banner "Try ⌘K to search" might help discoverability.
**Tentative answer**: yes, dismissible.  Show once after the first
workspace has ≥3 projects.

### Q2 — Nucleo as a dependency

Adds ~30 KB binary + a runtime cost.  Acceptable.  Alternative: a simple
custom subsequence matcher.  Recommendation: use nucleo; it's well-tested
and small.

### Q3 — Action label updates while typing

If selection changes mid-search (impossible? user can't multitask), do
labels update?  Labels are recomputed on every query change, so a
post-selection re-open shows up-to-date labels.

## Security Considerations

None.  Palette only dispatches existing in-process actions.  No new IPC.
