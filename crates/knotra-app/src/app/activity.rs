//! The activity domain: `handle_activity`, its `start_*` retry continuations,
//! and the helpers used only by them (RFC-040 Stage 3 commit 2).

use std::path::Path;

use iced::Task;
use iced::futures::StreamExt;
use knotra_vcs::{
    VcsAdapter,
    model::{
        operation::{OperationId, RetryExclusionReason},
        project::Project,
    },
};

use super::shared;
use crate::{
    message::{ActivityMessage, BackgroundMessage, Message},
    state::{
        ActivityRetryAction, AppState, OperationOwner, RetryAvailability, RetryExclusion,
        RetryUnavailableReason, Screen, sync::SmartPullRetryPreparation, sync::SyncPhase,
    },
};

fn split_retry_targets(
    state: &AppState,
    ids: &[knotra_vcs::ProjectId],
) -> (Vec<Project>, Vec<RetryExclusion>) {
    let mut projects = Vec::new();
    let mut exclusions = Vec::new();
    for id in ids {
        let Some(project) = shared::find_project(state, id) else {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::NotInActiveWorkspace,
            });
            continue;
        };
        if !Path::new(&project.path).exists() {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::ProjectPathMissing,
            });
        } else if !VcsAdapter::repo_exists(&project) {
            exclusions.push(RetryExclusion {
                project_id: id.clone(),
                reason: RetryExclusionReason::UnsupportedRepository,
            });
        } else {
            projects.push(project);
        }
    }
    (projects, exclusions)
}

pub(super) fn handle_activity(state: &mut AppState, msg: ActivityMessage) -> Task<Message> {
    match msg {
        ActivityMessage::RetryRequested {
            source_operation_id,
        } => {
            let action = match &state.activity.latest {
                crate::state::LatestOpState::Completed {
                    retry: RetryAvailability::Available(action),
                    ..
                } => action.clone(),
                _ => return Task::none(),
            };
            match action {
                ActivityRetryAction::FetchFailed {
                    source_operation_id: expected,
                    project_ids,
                } if expected == source_operation_id => {
                    return start_activity_fetch_retry(state, expected, project_ids);
                }
                ActivityRetryAction::ReviewSmartPull {
                    source_operation_id: expected,
                    project_ids,
                } if expected == source_operation_id => {
                    return start_activity_smart_pull_review(state, expected, project_ids);
                }
                _ => return Task::none(),
            }
        }
        ActivityMessage::DetailsRequested { operation_id } => {
            state.history_expanded.insert(operation_id);
            state.screen = Screen::History;
        }
    }
    Task::none()
}

fn start_activity_fetch_retry(
    state: &mut AppState,
    source_operation_id: OperationId,
    project_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    let (projects, exclusions) = split_retry_targets(state, &project_ids);
    if projects.is_empty() {
        mark_activity_retry_unavailable(state, &source_operation_id);
        state.status_bar = Some(state.t("plain.activity.none_available").to_owned());
        return Task::none();
    }
    let Some(lease_id) = shared::acquire_operation(state, OperationOwner::ActivityFetchRetry)
    else {
        return Task::none();
    };
    let operation_id = OperationId::new();
    let total = projects.len() + exclusions.len();
    state.activity.latest = crate::state::LatestOpState::Running {
        operation_id: operation_id.clone(),
        label: state.t("plain.activity.retrying_fetch").to_owned(),
        done: exclusions.len(),
        total,
    };
    state.activity.fetch_retry = Some(crate::state::FetchRetryRun {
        operation_id: operation_id.clone(),
        lease_id,
        started_at: chrono::Utc::now(),
        total,
        completed: Vec::new(),
        exclusions,
    });

    use iced::futures::stream;
    let stream = stream::iter(projects)
        .then(move |project| async move { VcsAdapter::fetch(&project).await });
    Task::run(stream, move |result| {
        Message::Background(BackgroundMessage::ActivityFetchRetryProjectCompleted {
            lease_id,
            operation_id: operation_id.clone(),
            result,
        })
    })
}

fn start_activity_smart_pull_review(
    state: &mut AppState,
    source_operation_id: OperationId,
    project_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    shared::invalidate_retry_preparation(state);
    let (projects, exclusions) = split_retry_targets(state, &project_ids);
    if projects.is_empty() {
        mark_activity_retry_unavailable(state, &source_operation_id);
        state.status_bar = Some(state.t("plain.activity.none_available").to_owned());
        return Task::none();
    }
    let Some(workspace_id) = state
        .workspace
        .as_ref()
        .map(|workspace| workspace.id.clone())
    else {
        return Task::none();
    };
    let Some(lease_id) =
        shared::acquire_operation(state, OperationOwner::ActivitySmartPullPreparation)
    else {
        return Task::none();
    };
    let request_id = state.sync.next_retry_preparation_id();
    let eligible_ids: Vec<_> = projects.iter().map(|project| project.id.clone()).collect();
    state.sync.selected_project_ids = eligible_ids.iter().cloned().collect();
    state.sync.project_selection.clear();
    if let Some(workspace) = &state.workspace {
        for project in &workspace.projects {
            state
                .sync
                .project_selection
                .insert(project.id.clone(), eligible_ids.contains(&project.id));
        }
    }
    state.sync.disposition_overrides.clear();
    state.sync.retry_exclusions = exclusions.clone();
    state.sync.retry_preparation = Some(SmartPullRetryPreparation {
        id: request_id,
        workspace_id: workspace_id.clone(),
        source_operation_id,
        lease_id,
        eligible_ids,
        exclusions,
    });
    state.sync.phase = SyncPhase::RetryPreparing;
    state.active_modal = crate::state::ActiveModal::Pull;

    Task::perform(
        async move {
            let mut statuses = Vec::with_capacity(projects.len());
            for project in projects {
                statuses.push(VcsAdapter::read_project_status(&project).await);
            }
            statuses
        },
        move |statuses| {
            Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
                request_id,
                workspace_id: workspace_id.clone(),
                lease_id,
                statuses,
            })
        },
    )
}

fn mark_activity_retry_unavailable(state: &mut AppState, source_operation_id: &OperationId) {
    if let crate::state::LatestOpState::Completed { log, retry } = &mut state.activity.latest
        && &log.result.operation_id == source_operation_id
    {
        *retry = RetryAvailability::Unavailable(RetryUnavailableReason::NoEligibleTargets);
    }
}
