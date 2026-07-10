# RFC-0014 — Project Detail Side Panel

| Field          | Value                                                                  |
|----------------|------------------------------------------------------------------------|
| Status      | Implemented (v0.12.0) |
| Priority       | Medium — drill-down replacement for ContextOps screen single-project case |
| Effort         | Medium — new panel widget, content layout, animation                   |
| Target version | v0.15                                                                  |
| Related        | RFC-0010 (cards intentionally show less; panel shows more)             |

## Summary

A right-docked, slide-in panel that opens when the user clicks a project's
name (not its checkbox).  The panel shows everything the card omits: full
status detail, recent operations on this project, path / remote URL / last
fetch, and per-project actions.  Replaces the use of the ContextOps screen
when working with a single project.

## Background

RFC-0010 reduced card density by moving most information off the card.
The user still needs a way to see that information.  A side panel is the
correct affordance: focused on one project at a time, doesn't require
navigating away from the dashboard, scales well as the amount of detail
grows.

This panel also serves single-project actions that don't make sense as
bulk operations: viewing the remote URL, copying the project path, opening
in a terminal, viewing this project's operation history filtered to just
itself.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | Click on a project's name in any tier opens the detail panel for that project |
| R2  | The panel docks to the right of the main window with a fixed width (≈420 px) |
| R3  | The dashboard remains visible to the left and stays interactive |
| R4  | Clicking another project's name swaps the panel content (no animation) |
| R5  | The panel has a close button (`✕`) and `Esc` closes it |
| R6  | The panel shows: full status, recent ops, identity (path/URL), per-project actions |
| R7  | Only one detail panel can be open at a time |
| R8  | The detail panel and the Resolve panel (RFC-0013) cannot both be open; opening one closes the other |
| R9  | The panel state is ephemeral — does not persist across restarts |
| R10 | The "Open in terminal" action launches the configured shell at the project path |

## External Design

### Visual

```
                                  ┌────────────────────────────────┐
┌────────────────────────────┐   │ alpha · Git                [✕] │
│  Dashboard (still visible) │   │ ───────────────────────────────│
│                            │   │ Status                          │
│  🔴  Needs attention (2)   │   │   Branch:        main           │
│                            │   │   Last commit:   abc1234        │
│  🟡  Active (4)            │   │   Remote:        origin         │
│   alpha   main · 2 dirty   │ ◀── (selected)                       │
│                            │   │ Working tree:                   │
│  ⚪  Clean (24)            │   │   2 modified (staged)           │
│                            │   │   1 modified (unstaged)         │
│                            │   │   3 untracked                   │
│                            │   │                                 │
│                            │   │ Remote                          │
│                            │   │   Upstream: origin/main         │
│                            │   │   Ahead:    0                   │
│                            │   │   Behind:   0                   │
│                            │   │   Last fetched: 3m ago          │
│                            │   │                                 │
│                            │   │ Identity                        │
│                            │   │   Path:   /home/me/code/alpha   │
│                            │   │           [Copy]  [Open]        │
│                            │   │   Remote: github.com/me/alpha   │
│                            │   │           [Copy]  [Open]        │
│                            │   │                                 │
│                            │   │ Recent operations               │
│                            │   │   ⓘ Fetched · 28 projects · 3m  │
│                            │   │   ✓ Smart pull · alpha · 12m    │
│                            │   │   ✓ Switch · alpha → main · 1h  │
│                            │   │   [Show all in History →]       │
│                            │   │                                 │
│                            │   │ Actions                         │
│                            │   │   [Open in editor]              │
│                            │   │   [Open in terminal]            │
│                            │   │   [Stash all changes…]          │
│                            │   │   [Switch branch…]              │
│                            │   │   [Remove from workspace…]      │
└────────────────────────────┘   └────────────────────────────────┘
```

### Sections

| Section            | Content                                                          |
|--------------------|------------------------------------------------------------------|
| Header             | Project name + VCS badge + close button                          |
| Status             | Branch, current commit short hash, default branch indicator       |
| Working tree       | All counters with labels (modified/staged/unstaged/untracked/conflicts)  |
| Remote             | Upstream, ahead, behind, last fetched timestamp                  |
| Identity           | Path (with Copy + Open buttons), remote URL                       |
| Recent operations  | Up to 5 most recent involving this project; link to full history |
| Actions            | Per-project actions: editor, terminal, stash, switch, remove     |

### Interactions

| Trigger                          | Effect                                              |
|----------------------------------|-----------------------------------------------------|
| Click project name on card       | Open panel for that project                         |
| Click different project name     | Swap content (no close/open animation)              |
| Click `✕`                        | Close panel                                         |
| `Esc` (panel focused)            | Close panel                                         |
| Click outside panel (on dashboard) | Panel stays open; dashboard remains interactive    |
| `Copy` button next to a value    | Copy that value to clipboard                        |
| `Open` button next to a path     | Open path in system file manager                    |
| `Open` button next to URL        | Open URL in default browser                         |
| Resize window                    | Panel width fixed; main view shrinks                |

### Animation

- First open: slide in from the right (250 ms).
- Subsequent project changes: content swap, no panel animation.
- Close: slide out to the right (200 ms).

### Empty / loading states

- Recent operations section shows "No recent operations" if empty.
- If the project status hasn't loaded yet, the body shows a centered
  spinner with "Loading status…"
- If `status.read_error` is set, the body shows a stylised error block
  with the error message and a "Refresh" button.

### Side panel + selection bar interaction

When the selection bar is also visible at the bottom, the panel does
**not** affect it.  The selection bar continues to span the full window
width (including under the panel).  This is acceptable because the
selection bar is short and the panel is wide.

Alternative considered: shrink the selection bar to the dashboard area
only.  Rejected: more visual noise; the bar is fine as a global element.

## Internal Design

### State

```rust
// state/mod.rs
pub struct AppState {
    // ... existing fields ...

    /// Project whose detail panel is open. None = no panel.
    pub detail_panel: Option<ProjectId>,
}
```

### Messages

```rust
pub enum DetailMessage {
    Opened(ProjectId),
    Closed,
    /// User clicked Copy on a value (path, URL, hash, ...).
    CopyRequested(String),
    /// User clicked an inline action button.
    ActionRequested(ProjectAction),
}

pub enum ProjectAction {
    OpenInEditor,
    OpenInTerminal,
    OpenInFileManager,
    StashAll,
    SwitchBranch,   // routes to RFC-0013 Switch modal with single-project selection
    RemoveFromWorkspace,
}
```

### Mutual exclusion with Resolve panel

```rust
DetailMessage::Opened(id) => {
    state.detail_panel = Some(id);
    state.resolve_panel = None;        // close conflict panel if open
    Task::none()
}
```

Similarly, opening the Resolve panel closes the detail panel.

### Recent operations filter

```rust
// state/detail.rs
pub fn recent_ops_for(state: &AppState, id: &ProjectId) -> Vec<&OperationLog> {
    state.operation_logs.iter()
        .rev()
        .filter(|log| log.result.per_project.iter().any(|p| p.project_id == *id))
        .take(5)
        .collect()
}
```

### View

```rust
// view/detail.rs
pub fn panel<'a>(state: &AppState) -> Option<Element<'a, Message>> {
    let id = state.detail_panel.as_ref()?;
    let project = state.workspace.as_ref()?
        .projects.iter().find(|p| p.id == *id)?;
    let status  = state.workspace_status.as_ref()?
        .projects.iter().find(|s| s.project_id == *id);

    let header   = header_view(state, project);
    let status_s = status_section(state, status);
    let wt_s     = working_tree_section(state, status);
    let remote_s = remote_section(state, status);
    let id_s     = identity_section(state, project);
    let recent_s = recent_ops_section(state, id);
    let actions  = actions_section(state, project);

    let body = column![ header, status_s, wt_s, remote_s, id_s, recent_s, actions ]
        .spacing(20).padding(16);

    Some(container(body)
        .width(420)
        .style(panel_style())
        .into())
}
```

The view function returns `Option<Element>`; the global view dispatcher
(RFC-0013) renders it as a right-aligned layer on the stack.

### Open in terminal — platform handling

```rust
fn open_in_terminal(path: &str) -> std::io::Result<()> {
    let terminal = std::env::var("KNOTRA_TERMINAL")
        .or_else(|_| std::env::var("TERMINAL"))
        .unwrap_or_else(|_| default_terminal_for_platform().to_string());

    std::process::Command::new(&terminal)
        .args(terminal_args_for(&terminal, path))
        .spawn()
        .map(|_| ())
}

fn default_terminal_for_platform() -> &'static str {
    #[cfg(target_os = "macos")]   { "open" }       // open -a Terminal
    #[cfg(target_os = "linux")]   { "x-terminal-emulator" }
    #[cfg(target_os = "windows")] { "wt" }         // Windows Terminal
}
```

Configurable in Settings → External tools.  Default fallback if the
preferred terminal is not found: print a status bar message.

### Open path / URL

```rust
DetailMessage::ActionRequested(ProjectAction::OpenInFileManager) => {
    let path = project.path.clone();
    Task::perform(async move {
        let _ = open::that(&path);   // `open` crate (~10 KB)
    }, |_| Message::NoOp)
}
```

The [`open`](https://crates.io/crates/open) crate handles platform-
specific URL/path opening (xdg-open / open / start).

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.15   | Panel with all sections, single-project actions (editor, terminal, file manager, stash) |
| 2     | v0.16   | Switch branch action routes to RFC-0013 Switch modal with single-project selection |
| 3     | v0.16   | Recent ops link routes to filtered History view |

## Test Plan

### Unit tests

1. **`detail_panel_opens_for_clicked_project`** — fire
   `DetailMessage::Opened(id)`, assert `state.detail_panel == Some(id)`.
2. **`detail_panel_closes_on_escape`** — open, fire
   `DetailMessage::Closed`, assert None.
3. **`detail_panel_swap_no_close`** — open A, then open B.  State goes
   directly to `Some(B)` (no None in between).
4. **`opening_detail_closes_resolve`** — `resolve_panel = Some`,
   `DetailMessage::Opened(id)` → both fields adjusted: detail set,
   resolve cleared.
5. **`recent_ops_filter_includes_only_this_project`** — operation_logs
   has ops involving A and B; recent_ops_for(A) returns only A's.
6. **`recent_ops_max_5`** — operation_logs has 10 ops for A; returns 5.

### Manual

1. Click project name → panel slides in from right.
2. Click another project name → content swaps without animation.
3. Click `Open in terminal` → terminal opens at project path.
4. Click `Copy` next to remote URL → clipboard contains the URL.
5. Click `✕` or press Esc → panel slides out.
6. Open Resolve panel from a Needs Attention card → detail panel closes.

## Open Questions

### Q1 — Panel layout in narrow windows

If the window is < 1100 px wide, the panel and the main view become
cramped.  **Tentative answer**: when window width drops below the
threshold, the panel becomes a full-window modal (Esc to close).
Implementation: conditional on `iced::window::events`.

### Q2 — Should the panel show contained operations as runnable retries?

Recent operations for this project — should clicking on a successful one
re-run it?  **Tentative answer**: no for v0.15.  Re-running has subtle
semantics (e.g., do we re-fetch with the same options?).  The link can
take users to filtered History where they see what happened; running
again is a separate action they pick from the action list.

### Q3 — Per-project history filtering

Should knotra surface "operations involving this project" as a filtered
view of History?  **Tentative answer**: yes; the recent-ops section
already does this for the last 5.  The "Show all in History" link should
take the user to the History screen with a pre-applied filter.

## Security Considerations

- `Open in terminal` launches a configured shell; user controls which
  binary via Settings.  Document the env vars and their precedence.
- `Open in file manager` and `Open URL` use the system handlers.  No
  shell injection risk because we pass paths as arguments, not embedded
  in shell commands.
- `Copy` writes to the system clipboard.  No risk beyond what the user
  initiated.
