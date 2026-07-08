# RFC-015 — Workspace Tabs

| Field          | Value                                                          |
|----------------|----------------------------------------------------------------|
| Status      | Implemented (v0.12.0) |
| Priority       | Low — quality-of-life improvement for multi-workspace users    |
| Effort         | Small — new top-bar widget, replace sidebar workspace list     |
| Target version | v0.15                                                          |
| Related        | RFC-008 (FsPoller prune); `state/workspace_mgr.rs`            |

## Summary

Replace the sidebar workspace list with a horizontal tab strip at the top
of the main window.  Each tab is a workspace; the tab strip includes a
`[+]` button to create a new workspace.  Workspace switches are bound to
`⌘1`, `⌘2`, … `⌘9` for the first nine workspaces.

## Background

Multi-workspace switching is currently a several-click action: open
sidebar, click workspace name in the list, wait for refresh.  For users
who switch frequently between (e.g.) work and personal workspaces, this
is excessive.  A tab strip makes the switch a single click or a one-key
shortcut.

Tabs also surface workspace identity persistently.  Today, the active
workspace is shown in a small label that users sometimes miss; tabs make
"which workspace am I in" visually inescapable.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | A horizontal tab strip occupies the top of the main window, below any global menu |
| R2  | Each workspace gets one tab; the active workspace tab is visually distinct |
| R3  | A `[+]` button at the right of the tab strip opens the "create workspace" dialog |
| R4  | Each tab shows the workspace name and an attention badge (count of Needs Attention projects, if > 0) |
| R5  | `⌘1` … `⌘9` switch to the corresponding tab (1-indexed) |
| R6  | Right-click on a tab opens a context menu: Rename, Delete, Duplicate |
| R7  | Long workspace names truncate to ≈12 characters with ellipsis; full name shown in tooltip |
| R8  | If there are more workspaces than fit, the strip horizontally scrolls; the active tab remains visible |
| R9  | The sidebar still exists but no longer contains the workspace list |
| R10 | Workspace tabs survive restart (same order as `~/.config/knotra/workspaces/` listing, alphabetised by name) |

## External Design

### Visual

```
┌─────────────────────────────────────────────────────────────────────┐
│ knotra   ┌─work (3)─┐ ┌─personal─┐ ┌─lab──┐ ┌─[+]┐    ⟳ ⚙ ? [search]│
│          └──────────┘ └──────────┘ └──────┘ └────┘                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Dashboard for "work" workspace                                      │
│                                                                      │
```

- Active tab: filled background using the accent color.
- Inactive tabs: muted background.
- Attention badge: small red circle with white count, embedded in the
  active or inactive tab.
- `[+]` button: outlined button at the end of the tab strip.

### Tab states

| State            | Visual                                                  |
|------------------|---------------------------------------------------------|
| Active           | Accent fill, white text                                 |
| Inactive (idle)  | Muted background, dim text                              |
| Inactive (attention) | Muted background + red badge with count            |
| Hover            | Slightly elevated background, accent border-bottom      |
| Drag (future)    | Slightly lifted; can reorder                            |

### Right-click context menu

```
┌─────────────────────┐
│ Rename…             │
│ Duplicate           │
│ ─────────────────── │
│ Delete…             │
└─────────────────────┘
```

### Create workspace flow

`[+]` opens an inline form (small modal centered):

```
┌─────────────────────────────────────┐
│ Create workspace                [✕] │
│ ─────────────────────────────────── │
│ Name:  [ my-new-workspace       ]   │
│                                     │
│ Copy projects from another?         │
│ ( ) Start empty                     │
│ ( ) Copy from "work"                │
│ ( ) Copy from "personal"            │
│                                     │
│              [Cancel]  [Create]     │
└─────────────────────────────────────┘
```

After create: new tab appears at the end; auto-activated.

### Rename flow

Inline editing on the tab itself (clicking Rename in context menu turns
the tab label into a text input).  `Enter` confirms, `Esc` cancels.

### Delete flow

A confirmation dialog (existing behaviour preserved):

```
┌─────────────────────────────────────┐
│ Delete workspace "lab"?         [✕] │
│ ─────────────────────────────────── │
│ This will remove 4 projects from    │
│ knotra (the repositories on disk    │
│ are not touched).                   │
│                                     │
│ Type "lab" to confirm:              │
│ [ ____________________ ]            │
│                                     │
│              [Cancel]  [Delete]     │
└─────────────────────────────────────┘
```

### Keyboard

| Shortcut       | Effect                                       |
|----------------|----------------------------------------------|
| `⌘1` … `⌘9`    | Switch to the 1st … 9th workspace tab        |
| `⌘T`           | Create new workspace (opens the [+] dialog)  |
| `⌘W`           | Reserved — does NOT close the window; mapped to "close current modal" (RFC-013) |
| `⌘Shift+]`     | Switch to next workspace tab                 |
| `⌘Shift+[`     | Switch to previous workspace tab             |

The `⌘W` collision is intentional: closing the window via `⌘W` is rare
for desktop apps; reusing it for modals matches common conventions in
many tools.

### Sidebar reduction

The sidebar after this RFC:

```
┌─────────────────┐
│                 │
│   ⚙  Settings    │
│   ?  Help        │
│                 │
│ (mostly empty)  │
│                 │
└─────────────────┘
```

Most users will find the sidebar nearly empty.  RFC-017 considers
removing it entirely.

## Internal Design

### State

```rust
// state/mod.rs
pub struct AppState {
    pub all_workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
    pub workspace: Option<Workspace>,    // for compatibility; can be derived

    // No changes; tab data is derived from all_workspaces.
}
```

### Messages

The existing `WorkspaceMessage` covers most actions.  Add:

```rust
pub enum WorkspaceMessage {
    // ... existing ...

    /// User clicked `[+]`.
    CreateRequested,

    /// User clicked a tab.
    TabClicked(usize),

    /// User pressed ⌘N (1-indexed; 1 = first tab).
    SwitchToIndex(usize),

    /// User pressed ⌘Shift+]  or ⌘Shift+[
    SwitchNext,
    SwitchPrev,

    /// User picked Rename from context menu.
    RenameRequested(WorkspaceId),
    /// User typed the new name and pressed Enter.
    RenameCommitted(WorkspaceId, String),
    /// User pressed Esc during inline rename.
    RenameCancelled,

    /// User picked Duplicate from context menu.
    DuplicateRequested(WorkspaceId),
}
```

### Attention badge computation

```rust
// state/workspace.rs
pub fn attention_count_for(state: &AppState, ws_id: &WorkspaceId) -> u32 {
    state.all_workspace_statuses.get(ws_id)
        .map(|s| s.projects.iter()
            .filter(|p| matches!(
                classify(p, ...), Classified { tier: Tier::NeedsAttention, .. }
            ))
            .count() as u32)
        .unwrap_or(0)
}
```

This requires knotra to maintain status for **all** workspaces, not only
the active one.  This is a behavioural change worth flagging:

#### Behaviour change — all-workspace status polling

Today, only the active workspace is polled.  Switching workspaces triggers
a fresh read.

For the attention badge to be useful, knotra needs to know the status of
inactive workspaces.  Options:

| Option | Description                                                  | Cost |
|--------|--------------------------------------------------------------|------|
| A      | Poll all workspaces in the background at the same interval   | High — N× refresh load |
| B      | Poll each workspace once on knotra startup; refresh inactive only on a slower interval (e.g., 5 minutes) | Moderate |
| C      | Show stale-on-load attention counts; refresh per-workspace only when switching | Low — but badges may be outdated |
| D      | No badges on inactive tabs; user only sees attention for active workspace | Trivial |

**Recommendation**: Option B for v0.15.  Configurable via Settings:
"Inactive workspace refresh interval: 5 minutes."

### View

```rust
// view/workspace_tabs.rs
pub fn tabs<'a>(state: &AppState) -> Element<'a, Message> {
    let mut row = row![];
    for (i, ws) in state.all_workspaces.iter().enumerate() {
        let is_active = i == state.active_workspace_idx;
        let attention = attention_count_for(state, &ws.id);
        let tab = tab_view(state, ws, is_active, attention);
        row = row.push(tab);
    }
    row = row.push(plus_button());
    row.spacing(2).into()
}

fn tab_view<'a>(
    state: &AppState,
    ws: &Workspace,
    is_active: bool,
    attention: u32,
) -> Element<'a, Message> {
    let label = truncate(&ws.name, 12);
    let mut row = row![text(label)];
    if attention > 0 {
        row = row.push(badge(attention));
    }
    button(row)
        .style(if is_active { active_tab_style() } else { idle_tab_style() })
        .on_press(Message::Workspace(WorkspaceMessage::TabClicked(/* index */ ???)))
        .into()
}
```

### Keyboard subscription

Adds:

```rust
keyboard::on_key_press(|key, modifiers| match (key, modifiers) {
    (Key::Character(c), m) if m == Modifiers::COMMAND => {
        if let Some(n) = c.parse::<usize>().ok().filter(|&n| n >= 1 && n <= 9) {
            Some(Message::Workspace(WorkspaceMessage::SwitchToIndex(n - 1)))
        } else { None }
    }
    (Key::Character(c), m) if c == "]" && m == Modifiers::COMMAND | Modifiers::SHIFT =>
        Some(Message::Workspace(WorkspaceMessage::SwitchNext)),
    (Key::Character(c), m) if c == "[" && m == Modifiers::COMMAND | Modifiers::SHIFT =>
        Some(Message::Workspace(WorkspaceMessage::SwitchPrev)),
    _ => None,
})
```

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.15   | Tab strip at top; sidebar workspace list hidden when ≥2 workspaces |
| 2     | v0.15   | Background polling of inactive workspaces (Option B) |
| 3     | v0.16   | Sidebar removed entirely if it becomes near-empty (per RFC-017) |

## Test Plan

### Unit tests

1. **`tabs_show_workspace_names`** — 3 workspaces → 3 tabs with their names.
2. **`active_tab_marked`** — `active_workspace_idx = 1` → only the second
   tab has the active style.
3. **`cmd1_switches_to_first_workspace`** — fire SwitchToIndex(0); state
   updates.
4. **`cmd_out_of_range_is_noop`** — SwitchToIndex(9) when only 3
   workspaces → no change.
5. **`badge_shows_attention_count`** — workspace has 3 NeedsAttention
   projects → badge = 3.
6. **`new_workspace_appears_at_end`** — create "lab"; tab list ends with
   lab.

### Manual

1. Two workspaces visible as tabs.  Click each; main view switches.
2. `⌘2` → switch to second workspace.
3. Right-click second tab → Rename → type new name → Enter.  Tab updates.
4. Right-click → Delete → confirm with the workspace name.  Tab removed.
5. Make 12 workspaces; tabs scroll horizontally.  Press `⌘5`; the fifth
   tab becomes visible.

## Open Questions

### Q1 — Polling cost for many workspaces

With 5 workspaces × 30 projects each at a 5-minute refresh, that's 150
status reads every 5 minutes — about 0.5/s on average.  Acceptable.

For users with very many workspaces (10+), consider increasing the
inactive interval automatically.  Or disable inactive polling entirely.

### Q2 — Tab reordering

Should users be able to drag tabs to reorder?  **Tentative answer**: no
for v0.15.  Order = alphabetical by name.  Reordering via drag-and-drop is
a future enhancement.

### Q3 — Default name for new workspace

When the user clicks `[+]`, the name input is pre-filled with what?
**Tentative answer**: an auto-generated unique name like "workspace-3"
based on existing count.  User can edit before pressing Create.

### Q4 — Sidebar entirely removed?

After this RFC + RFC-017, the sidebar contains just Settings and Help.
Worth keeping?  **Tentative answer**: yes — settings and help benefit
from a clear left-rail home that's never modal.  RFC-017 decides.

## Security Considerations

None.  Tab strip is presentation only.
