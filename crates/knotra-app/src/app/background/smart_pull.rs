//! Smart pull background completions (RFC-041 D1, Stage 4, final): retry
//! status aggregation, plan readiness, and per-project completion — plus
//! the one helper only this domain needs (RFC-041 D3).

use iced::Task;
use knotra_vcs::{
    SmartPullPlan, SmartPullProgress, WorkspaceId,
    model::operation::{
        OperationKind, OperationLog, OperationResult, ProjectOperationOutcome, RetryExclusionReason,
    },
};

use super::{merge_workspace_status, persist_log, shared, skipped_retry_result};
use crate::{
    message::Message,
    state::{
        AppState, LoadPhase, OperationLeaseId, RetryExclusion,
        sync::{ProjectOutcome, RetryPreparationId, SyncPhase, SyncResult},
    },
};

fn find_project_name(state: &AppState, id: &knotra_vcs::ProjectId) -> Option<String> {
    shared::find_project(state, id).map(|p| p.name)
}

pub(super) fn smart_pull_retry_status_ready(
    state: &mut AppState,
    request_id: RetryPreparationId,
    workspace_id: WorkspaceId,
    lease_id: OperationLeaseId,
    statuses: Vec<knotra_vcs::ProjectStatus>,
) -> Task<Message> {
    let Some(preparation) = state.sync.retry_preparation.clone() else {
        return Task::none();
    };
    let current_workspace_matches = state
        .workspace
        .as_ref()
        .is_some_and(|workspace| workspace.id == workspace_id);
    if preparation.id != request_id
        || preparation.workspace_id != workspace_id
        || preparation.lease_id != lease_id
    {
        return Task::none();
    }
    let source_matches = matches!(
        &state.activity.latest,
        crate::state::LatestOpState::Completed { log, .. }
            if log.result.operation_id == preparation.source_operation_id
    );
    let expected_ids: std::collections::HashSet<_> =
        preparation.eligible_ids.iter().cloned().collect();
    let returned_ids: std::collections::HashSet<_> = statuses
        .iter()
        .map(|status| status.project_id.clone())
        .collect();
    let status_ids_match = statuses.len() == expected_ids.len()
        && returned_ids.len() == statuses.len()
        && returned_ids == expected_ids;
    if !source_matches
        || !status_ids_match
        || !current_workspace_matches
        || state.active_modal != crate::state::ActiveModal::Pull
        || !matches!(state.sync.phase, SyncPhase::RetryPreparing)
    {
        state.sync.retry_preparation = None;
        state.operation_interlock.release_if_matches(lease_id);
        if state.active_modal == crate::state::ActiveModal::Pull && current_workspace_matches {
            state.sync.phase = SyncPhase::RetryPreparationFailed;
        }
        return Task::none();
    }

    let mut exclusions = preparation.exclusions;
    let mut readable = Vec::new();
    for status in statuses {
        if status.read_error.is_some() {
            exclusions.push(RetryExclusion {
                project_id: status.project_id.clone(),
                reason: RetryExclusionReason::StatusUnavailable,
            });
        } else {
            readable.push(status);
        }
    }
    state.sync.retry_preparation = None;
    state.operation_interlock.release_if_matches(lease_id);
    state.sync.retry_exclusions = exclusions;

    if readable.is_empty() {
        state.sync.phase = SyncPhase::RetryPreparationFailed;
        return Task::none();
    }

    let readable_ids: std::collections::HashSet<_> = readable
        .iter()
        .map(|status| status.project_id.clone())
        .collect();
    merge_workspace_status(
        state,
        knotra_vcs::WorkspaceStatus {
            projects: readable,
            last_refresh: Some(chrono::Utc::now()),
        },
    );
    state.sync.selected_project_ids = readable_ids.clone();
    if let Some(workspace) = &state.workspace {
        for project in &workspace.projects {
            state
                .sync
                .project_selection
                .insert(project.id.clone(), readable_ids.contains(&project.id));
        }
    }
    let selected_projects: Vec<_> = state
        .workspace
        .as_ref()
        .map(|workspace| {
            workspace
                .projects
                .iter()
                .filter(|project| readable_ids.contains(&project.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let plan = state
        .sync
        .build_plan(&selected_projects, state.workspace_status.as_ref());
    state.sync.phase = SyncPhase::AwaitingConfirm(plan);
    Task::none()
}

pub(super) fn smart_pull_plan_ready(state: &mut AppState, plan: SmartPullPlan) -> Task<Message> {
    // Already set in handle_sync; this message lets the view re-render.
    state.sync.phase = SyncPhase::AwaitingConfirm(plan);
    Task::none()
}

pub(super) fn smart_pull_project_completed(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    mut progress: SmartPullProgress,
) -> Task<Message> {
    // Fill in the project name if missing.
    if progress.project_name.is_empty()
        && let Some(name) = find_project_name(state, &progress.project_id)
    {
        progress.project_name = name;
    }
    let retry_exclusions = state.sync.retry_exclusions.clone();
    let retry_outcomes: Vec<ProjectOutcome> = retry_exclusions
        .iter()
        .map(|exclusion| ProjectOutcome {
            project_name: find_project_name(state, &exclusion.project_id)
                .unwrap_or_else(|| state.t("plain.project").to_owned()),
            outcome: ProjectOperationOutcome::Skipped,
            skip_reason: Some(exclusion.reason.code().to_owned()),
            commands_executed: Vec::new(),
            stderr: String::new(),
        })
        .collect();

    let mut completed_log: Option<OperationLog> = None;
    let mut completed_lease: Option<OperationLeaseId> = None;

    match &mut state.sync.phase {
        SyncPhase::FetchRunning {
            operation_id,
            lease_id: phase_lease_id,
            started_at,
            done,
            total,
            completed,
            operation_results,
        } => {
            if *phase_lease_id != lease_id {
                return Task::none();
            }
            *done += 1;
            let done_val = *done;
            let total_val = *total;

            let outcome = ProjectOutcome {
                project_name: progress.project_name.clone(),
                outcome: progress.result.effective_outcome(),
                skip_reason: progress.result.skip_reason.clone(),
                commands_executed: progress.result.commands_executed.clone(),
                stderr: progress.result.stderr.clone(),
            };
            operation_results.push(progress.result.clone());
            completed.push(outcome);

            if done_val >= total_val {
                let per_project = completed.clone();
                completed_log = Some(OperationLog {
                    result: OperationResult {
                        operation_id: operation_id.clone(),
                        kind: OperationKind::Fetch,
                        started_at: *started_at,
                        finished_at: chrono::Utc::now(),
                        per_project: operation_results.clone(),
                        rollback_attempted: false,
                        rollback_succeeded: None,
                    },
                    recovery_hints: Vec::new(),
                });
                completed_lease = Some(lease_id);
                state.sync.phase = SyncPhase::Done(SyncResult { per_project });
            }
        }
        SyncPhase::PullRunning {
            plan,
            lease_id: phase_lease_id,
            started_at,
            completed,
        } => {
            if *phase_lease_id != lease_id {
                return Task::none();
            }
            if let Some(hint) = progress.recovery_hint.clone() {
                // Recovery hint collected.
                let _ = hint;
            }
            completed.push(progress.clone());

            let expected = plan.entries.len();
            let got = completed.len();
            if got >= expected {
                // Build final result from completed.
                let mut outcomes: Vec<ProjectOutcome> = completed
                    .iter()
                    .map(|p| ProjectOutcome {
                        project_name: p.project_name.clone(),
                        outcome: p.result.effective_outcome(),
                        skip_reason: p.result.skip_reason.clone(),
                        commands_executed: p.result.commands_executed.clone(),
                        stderr: p.result.stderr.clone(),
                    })
                    .collect();
                outcomes.extend(retry_outcomes);

                let hints: Vec<_> = completed
                    .iter()
                    .filter_map(|p| p.recovery_hint.clone())
                    .collect();

                let mut logged_results: Vec<_> =
                    completed.iter().map(|p| p.result.clone()).collect();
                logged_results.extend(retry_exclusions.iter().map(skipped_retry_result));
                completed_log = Some(OperationLog {
                    result: OperationResult {
                        operation_id: plan.id.clone(),
                        kind: OperationKind::SmartPull,
                        started_at: started_at.to_owned(),
                        finished_at: chrono::Utc::now(),
                        per_project: logged_results,
                        rollback_attempted: false,
                        rollback_succeeded: None,
                    },
                    recovery_hints: hints,
                });

                state.sync.phase = SyncPhase::Done(SyncResult {
                    per_project: outcomes,
                });
                state.sync.retry_exclusions.clear();
                completed_lease = Some(lease_id);

                // Trigger status refresh.
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
            }
        }
        _ => {}
    }
    if let Some(log) = completed_log {
        if let Some(lease_id) = completed_lease {
            state.operation_interlock.release_if_matches(lease_id);
        }
        persist_log(&log, state);
        state.is_refreshing = true;
        state.load_phase = LoadPhase::Refreshing;
        return shared::refresh_workspace_task(state);
    }
    Task::none()
}
