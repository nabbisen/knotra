//! RFC-012 — Command palette: entry building and fuzzy search.

use crate::state::{AppState, PaletteEntry, PaletteEntryKind};

/// Built-in action entries.  Each `payload` is a unique action key that the
/// message handler in `app.rs` matches on.
const ACTIONS: &[(&str, &str)] = &[
    ("Fetch all projects",                "action.fetch_all"),
    ("Pull selected projects",            "action.pull_selected"),
    ("Tag selected projects…",            "action.tag_selected"),
    ("Switch branch on selected…",        "action.switch_selected"),
    ("Generate changelog for selected…",  "action.changelog_selected"),
    ("Add project to workspace",          "action.add_project"),
    ("Remove project from workspace",     "action.remove_project"),
    ("Create new workspace",              "action.workspace_create"),
    ("Switch to next workspace",          "action.workspace_next"),
    ("Select all projects",               "action.select_all"),
    ("Clear selection",                   "action.selection_clear"),
    ("Open Settings",                     "action.settings_open"),
    ("Open History",                      "action.history_open"),
    ("Toggle dark mode",                  "action.toggle_theme"),
    ("Refresh workspace",                 "action.refresh"),
    ("Show keyboard shortcuts",           "action.shortcuts_show"),
];

/// Rebuild the `results` list based on the current query.
/// Simple substring match (case-insensitive); a fuzzy matcher can be swapped
/// in later without changing the surrounding logic.
pub fn update_results(state: &mut AppState) {
    let q = state.palette.query.to_lowercase();
    let mut entries: Vec<PaletteEntry> = Vec::new();

    // --- Actions ---
    for (label, payload) in ACTIONS {
        if q.is_empty() || label.to_lowercase().contains(&q) {
            entries.push(PaletteEntry {
                kind:    PaletteEntryKind::Action,
                label:   label.to_string(),
                payload: payload.to_string(),
            });
        }
    }

    // --- Projects ---
    if let Some(ws) = &state.workspace {
        for p in &ws.projects {
            if q.is_empty() || p.name.to_lowercase().contains(&q) {
                entries.push(PaletteEntry {
                    kind:    PaletteEntryKind::Project,
                    label:   format!("Project: {}", p.name),
                    payload: p.id.to_string(),
                });
            }
        }
    }

    // --- Workspaces ---
    for ws in &state.all_workspaces {
        if q.is_empty() || ws.name.to_lowercase().contains(&q) {
            entries.push(PaletteEntry {
                kind:    PaletteEntryKind::Workspace,
                label:   format!("Workspace: {}", ws.name),
                payload: ws.id.to_string(),
            });
        }
    }

    // Cap at 12 results for display.
    entries.truncate(12);
    let len = entries.len();
    state.palette.results = entries;
    // Keep highlight in bounds.
    if state.palette.highlighted >= len {
        state.palette.highlighted = len.saturating_sub(1);
    }
}

/// Dispatch the currently highlighted palette entry.
/// Returns the Message to emit (if any).
pub fn dispatch_entry(state: &AppState) -> Option<crate::message::Message> {
    use crate::message::Message;
    use crate::state::{PaletteEntryKind, Screen};

    let entry = state.palette.results.get(state.palette.highlighted)?;
    match entry.kind {
        PaletteEntryKind::Action => match entry.payload.as_str() {
            "action.settings_open"  => Some(Message::Navigate(Screen::Settings)),
            "action.history_open"   => Some(Message::Navigate(Screen::History)),
            "action.refresh"        => Some(Message::Workspace(
                crate::message::WorkspaceMessage::RefreshRequested,
            )),
            "action.select_all"     => Some(Message::Selection(
                crate::message::SelectionMessage::SelectAll,
            )),
            "action.selection_clear" => Some(Message::Selection(
                crate::message::SelectionMessage::Clear,
            )),
            "action.add_project"    => Some(Message::Workspace(
                crate::message::WorkspaceMessage::AddProjectDialogOpened,
            )),
            "action.shortcuts_show" => Some(Message::KeyEvent(
                crate::message::KeyboardMessage::CheatSheetToggled,
            )),
            _ => None,
        },
        PaletteEntryKind::Workspace => {
            // Find the workspace by comparing its id.to_string() with payload.
            let ws = state.all_workspaces.iter()
                .find(|ws| ws.id.to_string() == entry.payload);
            ws.map(|ws| Message::Workspace(
                crate::message::WorkspaceMessage::WorkspaceSwitched(ws.id.clone()),
            ))
        }
        PaletteEntryKind::Project => {
            // Jump focus to the project's card (future: scroll + highlight).
            // For now just close the palette.
            None
        }
    }
}
