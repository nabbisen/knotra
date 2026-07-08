# RFC-008 — Prune `FsPoller` Snapshots on Workspace Switch

| Field    | Value                                                         |
|----------|---------------------------------------------------------------|
| Status      | Implemented                      |
| Priority | Low — memory issue only; no incorrect behaviour               |
| Effort   | Trivial (3 lines)                                             |
| Related  | `crates/knotra-app/src/app.rs` (`WorkspaceMessage::WorkspaceSwitched`) |

## Summary

When the user switches workspaces, the `FsPoller` retains stale mtime
snapshots for projects that belong to the previous workspace.  These snapshots
are never consulted again (the active project set has changed) but are never
released.  Over many workspace switches the internal `snapshots` `HashMap`
grows without bound.

## Problem

```rust
// app.rs — WorkspaceSwitched handler (current)
WorkspaceMessage::WorkspaceSwitched(id) => {
    state.active_workspace_idx = idx;
    state.workspace = state.all_workspaces.get(idx).cloned();
    // ...
    refresh_workspace_task(state)
    // FsPoller.snapshots still holds entries for the OLD workspace's projects.
}
```

`FsPoller::prune(&[ProjectId])` is already implemented and is called from the
`handle_fs_watch_tick` path.  It is just not called here.

## Design

In `handle_workspace`, after updating `state.workspace`, collect the active
project IDs and call `prune`:

```rust
WorkspaceMessage::WorkspaceSwitched(id) => {
    if let Some(idx) = state.all_workspaces.iter().position(|ws| ws.id == id) {
        state.active_workspace_idx = idx;
        state.workspace = state.all_workspaces.get(idx).cloned();

        // Prune stale FsPoller snapshots for the previous workspace.
        let active_ids: Vec<ProjectId> = state.workspace.as_ref()
            .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
            .unwrap_or_default();
        state.fs_poller.prune(&active_ids);

        state.workspace_status = None;
        state.load_phase = LoadPhase::Refreshing;
        state.is_refreshing = true;
        return refresh_workspace_task(state);
    }
    Task::none()
}
```

The same prune call should be added to `WorkspaceMessage::DeleteWorkspaceConfirmed`
for symmetry, although deletion already resets the active workspace.

## Test Plan

Add a unit test in `crates/knotra-app/src/tests.rs`:

**`fspoller_snapshots_pruned_on_workspace_switch`**

1. Construct an `AppState` with two workspaces, each with two projects.
2. Simulate a switch from workspace A to workspace B by calling the
   `WorkspaceSwitched` message handler directly.
3. Assert that `state.fs_poller` contains no snapshot keys for projects
   from workspace A.

## Security Considerations

None.
