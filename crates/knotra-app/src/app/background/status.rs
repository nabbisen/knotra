//! Status/misc background completions (RFC-041 D1, Stage 1): workspace
//! status refresh, topology scan, changelog draft, missing-project
//! detection, and task-level errors.

use iced::Task;
use knotra_vcs::{ProjectId, WorkspaceStatus};

use crate::{
    message::Message,
    state::{AppState, LoadPhase, changelog::ChangelogPhase, topology::TopologyPhase},
};

pub(super) fn workspace_status_refreshed(
    state: &mut AppState,
    new_status: WorkspaceStatus,
) -> Task<Message> {
    state.dashboard_error_details_open = false;
    // Detect missing-path projects.
    if let Some(ws) = &state.workspace {
        let missing: Vec<_> = ws
            .projects
            .iter()
            .filter(|p| !knotra_vcs::VcsAdapter::repo_exists(p))
            .map(|p| p.id.clone())
            .collect();
        if missing != state.missing_projects.iter().cloned().collect::<Vec<_>>() {
            state.missing_projects = missing.into_iter().collect();
        }
    }
    super::merge_workspace_status(state, new_status);
    state.load_phase = LoadPhase::Ready;
    state.is_refreshing = false;
    state.status_bar = None;
    Task::none()
}

pub(super) fn topology_scanned(
    state: &mut AppState,
    graph: knotra_vcs::DependencyGraph,
) -> Task<Message> {
    // Compute impact warnings for the Freezer.
    if let Some(ws) = &state.workspace {
        let names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();
        state.topology.impact_warnings = state.topology.compute_warnings(&graph, &names);
    }
    state.topology.phase = TopologyPhase::Ready(graph);
    Task::none()
}

pub(super) fn changelog_draft_ready(
    state: &mut AppState,
    request_id: u64,
    draft: knotra_vcs::ChangelogDraft,
) -> Task<Message> {
    if state.changelog.active_request_id == Some(request_id) {
        state.changelog.active_request_id = None;
        state.changelog.phase = ChangelogPhase::Ready(draft);
    }
    Task::none()
}

pub(super) fn missing_projects_detected(
    state: &mut AppState,
    ids: Vec<ProjectId>,
) -> Task<Message> {
    state.missing_projects = ids.into_iter().collect();
    state.reconcile_selection_with_display();
    Task::none()
}

pub(super) fn task_error(state: &mut AppState, description: String) -> Task<Message> {
    state.load_phase = LoadPhase::Error(description.clone());
    state.is_refreshing = false;
    state.dashboard_error_details_open = false;
    state.status_bar = Some(state.t("dashboard.load_failed").to_owned());
    Task::none()
}
