//! RFC-0012 / RFC-028 — Command palette registry, results, and dispatch.

use crate::{
    message::{
        ChangelogMessage, ContextMessage, DetailPanelMessage, FreezerMessage, KeyboardMessage,
        Message, SelectionMessage, SyncMessage, WorkspaceMessage,
    },
    state::{AppState, PaletteEntry, PaletteEntryKind, Screen},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAvailability {
    Enabled,
    Disabled(&'static str),
    Hidden,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum PaletteDispatch {
    Dispatched(Message),
    Disabled(&'static str),
    Noop,
}

struct PaletteAction {
    id: &'static str,
    label_key: &'static str,
    availability: fn(&AppState) -> PaletteAvailability,
    dispatch: fn(&AppState) -> Option<Message>,
}

const ACTIONS: &[PaletteAction] = &[
    PaletteAction {
        id: "action.fetch_all",
        label_key: "palette.action.check_all",
        availability: workspace_has_fetchable_project_unless_busy,
        dispatch: |_| Some(Message::Sync(SyncMessage::BulkFetchAllRequested)),
    },
    PaletteAction {
        id: "action.pull_selected",
        label_key: "plain.get_latest",
        availability: selection_has_upstream_unless_busy,
        dispatch: |_| Some(Message::Sync(SyncMessage::BulkPullRequested)),
    },
    PaletteAction {
        id: "action.tag_selected",
        label_key: "plain.save_release_point",
        availability: selection_non_empty_unless_busy,
        dispatch: |_| Some(Message::Freezer(FreezerMessage::BulkOpenRequested)),
    },
    PaletteAction {
        id: "action.switch_selected",
        label_key: "plain.change_work_area",
        availability: selection_exactly_one_unless_busy,
        dispatch: |_| Some(Message::Context(ContextMessage::BulkOpenRequested)),
    },
    PaletteAction {
        id: "action.changelog_selected",
        label_key: "palette.action.changelog_selected",
        availability: selection_non_empty,
        dispatch: |_| Some(Message::Changelog(ChangelogMessage::BulkOpenRequested)),
    },
    PaletteAction {
        id: "action.add_project",
        label_key: "palette.action.add_project",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| Some(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened)),
    },
    PaletteAction {
        id: "action.remove_project",
        label_key: "palette.action.remove_project",
        availability: selection_exactly_one_for_remove,
        dispatch: |state| {
            state
                .selection_summary()
                .selected_ids
                .first()
                .cloned()
                .map(WorkspaceMessage::RemoveProjectRequested)
                .map(Message::Workspace)
        },
    },
    PaletteAction {
        id: "action.workspace_create",
        label_key: "palette.action.workspace_create",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| {
            Some(Message::Workspace(
                WorkspaceMessage::CreateWorkspaceDialogOpened,
            ))
        },
    },
    PaletteAction {
        id: "action.workspace_next",
        label_key: "palette.action.workspace_next",
        availability: next_workspace_available,
        dispatch: |state| {
            let len = state.all_workspaces.len();
            if len < 2 {
                return None;
            }
            let next_idx = (state.active_workspace_idx + 1) % len;
            state
                .all_workspaces
                .get(next_idx)
                .map(|ws| Message::Workspace(WorkspaceMessage::WorkspaceSwitched(ws.id.clone())))
        },
    },
    PaletteAction {
        id: "action.select_all",
        label_key: "plain.select_visible_projects",
        availability: visible_projects_available,
        dispatch: |_| Some(Message::Selection(SelectionMessage::SelectAll)),
    },
    PaletteAction {
        id: "action.selection_clear",
        label_key: "palette.action.clear_selection",
        availability: selection_clear_available,
        dispatch: |_| Some(Message::Selection(SelectionMessage::Clear)),
    },
    PaletteAction {
        id: "action.settings_open",
        label_key: "palette.action.open_settings",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| Some(Message::Navigate(Screen::Settings)),
    },
    PaletteAction {
        id: "action.history_open",
        label_key: "palette.action.open_history",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| Some(Message::Navigate(Screen::History)),
    },
    PaletteAction {
        id: "action.toggle_theme",
        label_key: "palette.action.toggle_theme",
        availability: |_| PaletteAvailability::Hidden,
        dispatch: |_| None,
    },
    PaletteAction {
        id: "action.refresh",
        label_key: "palette.action.refresh",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| Some(Message::Workspace(WorkspaceMessage::RefreshRequested)),
    },
    PaletteAction {
        id: "action.shortcuts_show",
        label_key: "palette.action.shortcuts",
        availability: |_| PaletteAvailability::Enabled,
        dispatch: |_| Some(Message::KeyEvent(KeyboardMessage::CheatSheetToggled)),
    },
];

fn unless_busy(
    state: &AppState,
    available: fn(&AppState) -> PaletteAvailability,
) -> PaletteAvailability {
    if state.operation_interlock.is_busy() {
        PaletteAvailability::Disabled("plain.activity.busy")
    } else {
        available(state)
    }
}

fn workspace_has_fetchable_project_unless_busy(state: &AppState) -> PaletteAvailability {
    unless_busy(state, workspace_has_fetchable_project)
}

fn selection_has_upstream_unless_busy(state: &AppState) -> PaletteAvailability {
    unless_busy(state, selection_has_upstream)
}

fn selection_non_empty_unless_busy(state: &AppState) -> PaletteAvailability {
    unless_busy(state, selection_non_empty)
}

fn selection_exactly_one_unless_busy(state: &AppState) -> PaletteAvailability {
    unless_busy(state, selection_exactly_one)
}

#[cfg(test)]
pub(crate) fn visible_action_ids(state: &AppState) -> Vec<&'static str> {
    ACTIONS
        .iter()
        .filter(|action| !matches!((action.availability)(state), PaletteAvailability::Hidden))
        .map(|action| action.id)
        .collect()
}

pub fn update_results(state: &mut AppState) {
    let q = state.palette.query.to_lowercase();
    let mut entries: Vec<PaletteEntry> = Vec::new();

    for action in ACTIONS {
        let availability = (action.availability)(state);
        if matches!(availability, PaletteAvailability::Hidden) {
            continue;
        }
        let label = state.t(action.label_key).to_owned();
        if matches_query(&q, &label) || matches_query(&q, action.id) {
            entries.push(PaletteEntry {
                kind: PaletteEntryKind::Action,
                label,
                payload: action.id.to_owned(),
                disabled_reason_key: disabled_reason(availability),
            });
        }
    }

    if let Some(ws) = &state.workspace {
        for p in &ws.projects {
            let label = format!("{}: {}", state.t("palette.kind.project"), p.name);
            if matches_query(&q, &label) {
                entries.push(PaletteEntry {
                    kind: PaletteEntryKind::Project,
                    label,
                    payload: p.id.to_string(),
                    disabled_reason_key: None,
                });
            }
        }
    }

    for ws in &state.all_workspaces {
        let label = format!("{}: {}", state.t("palette.kind.workspace"), ws.name);
        if matches_query(&q, &label) {
            let disabled = (state.workspace.as_ref().map(|active| active.id.clone())
                == Some(ws.id.clone()))
            .then_some("palette.disabled.already_open");
            entries.push(PaletteEntry {
                kind: PaletteEntryKind::Workspace,
                label,
                payload: ws.id.to_string(),
                disabled_reason_key: disabled,
            });
        }
    }

    entries.truncate(12);
    let len = entries.len();
    state.palette.results = entries;
    if state.palette.highlighted >= len {
        state.palette.highlighted = len.saturating_sub(1);
    }
}

pub fn dispatch_entry(state: &AppState) -> PaletteDispatch {
    let Some(entry) = state.palette.results.get(state.palette.highlighted) else {
        return PaletteDispatch::Noop;
    };
    if let Some(reason) = entry.disabled_reason_key {
        return PaletteDispatch::Disabled(reason);
    }

    match entry.kind {
        PaletteEntryKind::Action => dispatch_action(state, &entry.payload),
        PaletteEntryKind::Workspace => {
            let ws = state
                .all_workspaces
                .iter()
                .find(|ws| ws.id.to_string() == entry.payload);
            ws.map(|ws| {
                PaletteDispatch::Dispatched(Message::Workspace(
                    WorkspaceMessage::WorkspaceSwitched(ws.id.clone()),
                ))
            })
            .unwrap_or(PaletteDispatch::Noop)
        }
        PaletteEntryKind::Project => state
            .workspace
            .as_ref()
            .and_then(|ws| {
                ws.projects
                    .iter()
                    .find(|p| p.id.to_string() == entry.payload)
            })
            .map(|project| {
                PaletteDispatch::Dispatched(Message::DetailPanel(DetailPanelMessage::Opened(
                    project.id.clone(),
                )))
            })
            .unwrap_or(PaletteDispatch::Noop),
    }
}

fn dispatch_action(state: &AppState, payload: &str) -> PaletteDispatch {
    let Some(action) = ACTIONS.iter().find(|action| action.id == payload) else {
        return PaletteDispatch::Noop;
    };
    match (action.availability)(state) {
        PaletteAvailability::Enabled => (action.dispatch)(state)
            .map(PaletteDispatch::Dispatched)
            .unwrap_or(PaletteDispatch::Noop),
        PaletteAvailability::Disabled(reason) => PaletteDispatch::Disabled(reason),
        PaletteAvailability::Hidden => PaletteDispatch::Noop,
    }
}

fn disabled_reason(availability: PaletteAvailability) -> Option<&'static str> {
    match availability {
        PaletteAvailability::Enabled | PaletteAvailability::Hidden => None,
        PaletteAvailability::Disabled(reason) => Some(reason),
    }
}

fn matches_query(query: &str, label: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(query)
}

fn workspace_has_fetchable_project(state: &AppState) -> PaletteAvailability {
    let Some(ws) = &state.workspace else {
        return PaletteAvailability::Disabled("palette.disabled.no_workspace");
    };
    let any_fetchable = ws
        .projects
        .iter()
        .any(|project| !state.missing_projects.contains(&project.id));
    if any_fetchable {
        PaletteAvailability::Enabled
    } else {
        PaletteAvailability::Disabled("palette.disabled.no_fetchable_projects")
    }
}

fn selection_has_upstream(state: &AppState) -> PaletteAvailability {
    let summary = state.selection_summary();
    if summary.selected_count == 0 {
        return PaletteAvailability::Disabled("plain.disabled.choose_one");
    }
    if summary.has_upstream {
        PaletteAvailability::Enabled
    } else {
        PaletteAvailability::Disabled("plain.disabled.no_upstream")
    }
}

fn selection_non_empty(state: &AppState) -> PaletteAvailability {
    if state.selection_summary().selected_count > 0 {
        PaletteAvailability::Enabled
    } else {
        PaletteAvailability::Disabled("plain.disabled.choose_one")
    }
}

fn selection_exactly_one(state: &AppState) -> PaletteAvailability {
    match state.selection_summary().selected_count {
        1 => PaletteAvailability::Enabled,
        0 => PaletteAvailability::Disabled("plain.disabled.choose_one"),
        _ => PaletteAvailability::Disabled("plain.selection.choose_one_work_area"),
    }
}

fn selection_exactly_one_for_remove(state: &AppState) -> PaletteAvailability {
    match state.selection_summary().selected_count {
        1 => PaletteAvailability::Enabled,
        0 => PaletteAvailability::Disabled("plain.disabled.choose_one"),
        _ => PaletteAvailability::Disabled("palette.disabled.choose_one_to_remove"),
    }
}

fn visible_projects_available(state: &AppState) -> PaletteAvailability {
    if state.selection_summary().visible_ids.is_empty() {
        PaletteAvailability::Disabled("plain.selection.no_visible_projects")
    } else {
        PaletteAvailability::Enabled
    }
}

fn selection_clear_available(state: &AppState) -> PaletteAvailability {
    if state.selection_mode || state.selection_summary().selected_count > 0 {
        PaletteAvailability::Enabled
    } else {
        PaletteAvailability::Disabled("palette.disabled.no_selection_to_clear")
    }
}

fn next_workspace_available(state: &AppState) -> PaletteAvailability {
    if state.all_workspaces.len() > 1 {
        PaletteAvailability::Enabled
    } else {
        PaletteAvailability::Disabled("palette.disabled.only_one_workspace")
    }
}
