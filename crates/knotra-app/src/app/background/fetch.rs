//! Fetch background completions (RFC-041 D1, Stage 2): fetch-retry
//! aggregation, single-project fetch, and bulk fetch.

use iced::Task;
use knotra_vcs::{
    OperationId, VcsAdapter,
    model::operation::{OperationKind, OperationLog, OperationResult, ProjectOperationResult},
};

use super::{persist_log, shared, skipped_retry_result};
use crate::{
    message::{BackgroundMessage, Message},
    state::{AppState, LoadPhase, OperationLeaseId},
};

pub(super) fn activity_fetch_retry_project_completed(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    operation_id: OperationId,
    result: ProjectOperationResult,
) -> Task<Message> {
    let Some(mut run) = state.activity.fetch_retry.take() else {
        return Task::none();
    };
    if run.lease_id != lease_id || run.operation_id != operation_id {
        state.activity.fetch_retry = Some(run);
        return Task::none();
    }
    run.completed.push(result);
    let done = run.completed.len() + run.exclusions.len();
    if let crate::state::LatestOpState::Running {
        operation_id: active_id,
        done: active_done,
        ..
    } = &mut state.activity.latest
        && *active_id == operation_id
    {
        *active_done = done;
    }
    let expected = run.total.saturating_sub(run.exclusions.len());
    if run.completed.len() < expected {
        state.activity.fetch_retry = Some(run);
        return Task::none();
    }

    let mut per_project = run.completed;
    per_project.extend(run.exclusions.iter().map(skipped_retry_result));
    let log = OperationLog {
        result: OperationResult {
            operation_id: run.operation_id,
            kind: OperationKind::Fetch,
            started_at: run.started_at,
            finished_at: chrono::Utc::now(),
            per_project,
            rollback_attempted: false,
            rollback_succeeded: None,
        },
        recovery_hints: Vec::new(),
    };
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    persist_log(&log, state);
    state.is_refreshing = true;
    state.load_phase = LoadPhase::Refreshing;
    shared::refresh_workspace_task(state)
}

pub(super) fn single_fetch_completed(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    log: OperationLog,
) -> Task<Message> {
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    for r in &log.result.per_project {
        state.fetching_projects.remove(&r.project_id);
    }
    persist_log(&log, state);

    let tasks: Vec<Task<Message>> = log
        .result
        .per_project
        .iter()
        .filter_map(|r| shared::find_project(state, &r.project_id))
        .map(|project| {
            Task::perform(
                async move { VcsAdapter::read_project_status(&project).await },
                |s| {
                    Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        knotra_vcs::WorkspaceStatus {
                            projects: vec![s],
                            last_refresh: Some(chrono::Utc::now()),
                        },
                    ))
                },
            )
        })
        .collect();
    Task::batch(tasks)
}

pub(super) fn bulk_fetch_completed(state: &mut AppState, log: OperationLog) -> Task<Message> {
    persist_log(&log, state);
    state.status_bar = Some(if log.result.any_failed() {
        format!(
            "{} {}, {} {}",
            log.result.successful_projects().len(),
            state.t("plain.activity.succeeded"),
            log.result.failed_projects().len(),
            state.t("plain.activity.failed"),
        )
    } else {
        format!(
            "{} {}",
            log.result.per_project.len(),
            state.t("plain.activity.check_complete")
        )
    });
    state.is_refreshing = true;
    state.load_phase = LoadPhase::Refreshing;
    shared::refresh_workspace_task(state)
}
