//! Async completions: `handle_background` and the six helpers used only by
//! it (RFC-040 Stage 5).
//!
//! **Over the 500-ELOC threshold by design, per RFC-040 R2/D2.** This module
//! is one `match` with 20 top-level arms, each binding variables out of its
//! own message pattern (e.g. `SmartPullProjectCompleted { lease_id,
//! project_id, .. }`). Splitting it by arm is not a move: each arm would
//! need an invented signature to carry those bindings across a function
//! boundary, which is writing new code around a verbatim body - the one
//! thing this RFC's stages have deliberately avoided, and the wrong trade
//! in the application's most concurrency-sensitive function. The split is
//! deferred to its own RFC, with its own design pass and its own tests,
//! immediately after RFC-040 closes - not dropped, not forgotten.

use iced::Task;
use knotra_vcs::model::operation::{
    OperationKind, OperationLog, OperationResult, ProjectOperationOutcome, ProjectOperationResult,
    RetryExclusionReason,
};

use super::shared;
use crate::{
    message::{BackgroundMessage, Message},
    persistence::save_operation_log,
    state::{
        ActivityRetryAction, AppState, LoadPhase, OperationLeaseId, RetryAvailability,
        RetryExclusion, RetryUnavailableReason,
        sync::{ProjectOutcome, SyncKind, SyncPhase, SyncResult},
    },
};

mod conflict;
mod context_switch;
mod fetch;
mod freeze;
mod status;

fn find_project_name(state: &AppState, id: &knotra_vcs::ProjectId) -> Option<String> {
    shared::find_project(state, id).map(|p| p.name)
}

fn merge_workspace_status(state: &mut AppState, new: knotra_vcs::WorkspaceStatus) {
    if let Some(existing) = &mut state.workspace_status {
        for ps in new.projects {
            if let Some(pos) = existing
                .projects
                .iter()
                .position(|p| p.project_id == ps.project_id)
            {
                existing.projects[pos] = ps;
            } else {
                existing.projects.push(ps);
            }
        }
        existing.last_refresh = new.last_refresh;
    } else {
        state.workspace_status = Some(new);
    }
    state.reconcile_selection_with_display();
}

fn persist_log(log: &OperationLog, state: &mut AppState) {
    if let Err(e) = save_operation_log(log, &state.paths) {
        tracing::warn!("failed to save operation log: {e}");
        state.status_bar = Some(state.t("plain.activity.log_save_failed").to_owned());
    }
    state.operation_logs.insert(0, log.clone());
    state.operation_logs.truncate(state.config.max_log_entries);
    let failed_ids: Vec<_> = log
        .result
        .per_project
        .iter()
        .filter(|result| result.effective_outcome() == ProjectOperationOutcome::Failed)
        .map(|result| result.project_id.clone())
        .collect();
    let retry = if failed_ids.is_empty() {
        RetryAvailability::NotApplicable
    } else {
        match log.result.kind {
            OperationKind::Fetch => {
                RetryAvailability::Available(ActivityRetryAction::FetchFailed {
                    source_operation_id: log.result.operation_id.clone(),
                    project_ids: failed_ids,
                })
            }
            OperationKind::SmartPull => {
                RetryAvailability::Available(ActivityRetryAction::ReviewSmartPull {
                    source_operation_id: log.result.operation_id.clone(),
                    project_ids: failed_ids,
                })
            }
            OperationKind::ContextSwitch => {
                RetryAvailability::Unavailable(RetryUnavailableReason::ContextSwitch)
            }
            OperationKind::Freeze => RetryAvailability::Unavailable(RetryUnavailableReason::Freeze),
            OperationKind::FreezeRollback => {
                RetryAvailability::Unavailable(RetryUnavailableReason::FreezeRollback)
            }
            OperationKind::StatusRefresh => {
                RetryAvailability::Unavailable(RetryUnavailableReason::StatusRefresh)
            }
        }
    };
    state.activity.latest = crate::state::LatestOpState::Completed {
        log: log.clone(),
        retry,
    };
    state.activity.completed_secs = 0;
}

fn skipped_retry_result(exclusion: &RetryExclusion) -> ProjectOperationResult {
    ProjectOperationResult {
        project_id: exclusion.project_id.clone(),
        outcome: ProjectOperationOutcome::Skipped,
        success: true,
        skip_reason: Some(exclusion.reason.code().to_owned()),
        commands_executed: Vec::new(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        error_message: None,
    }
}

pub(super) fn handle_background(state: &mut AppState, msg: BackgroundMessage) -> Task<Message> {
    match msg {
        BackgroundMessage::WorkspaceStatusRefreshed(new_status) => {
            status::workspace_status_refreshed(state, new_status)
        }

        BackgroundMessage::ActivityFetchRetryProjectCompleted {
            lease_id,
            operation_id,
            result,
        } => fetch::activity_fetch_retry_project_completed(state, lease_id, operation_id, result),

        BackgroundMessage::SmartPullRetryStatusReady {
            request_id,
            workspace_id,
            lease_id,
            statuses,
        } => {
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
                if state.active_modal == crate::state::ActiveModal::Pull
                    && current_workspace_matches
                {
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

        BackgroundMessage::SmartPullPlanReady(plan) => {
            // Already set in handle_sync; this message lets the view re-render.
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);
            Task::none()
        }

        BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            mut progress,
        } => {
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
                    project_id: exclusion.project_id.clone(),
                    project_name: find_project_name(state, &exclusion.project_id)
                        .unwrap_or_else(|| state.t("plain.project").to_owned()),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some(exclusion.reason.code().to_owned()),
                    commands_executed: Vec::new(),
                    stdout: String::new(),
                    stderr: String::new(),
                    log_expanded: false,
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
                        project_id: progress.project_id.clone(),
                        project_name: progress.project_name.clone(),
                        outcome: progress.result.effective_outcome(),
                        success: progress.result.success,
                        skip_reason: progress.result.skip_reason.clone(),
                        commands_executed: progress.result.commands_executed.clone(),
                        stdout: progress.result.stdout.clone(),
                        stderr: progress.result.stderr.clone(),
                        log_expanded: false,
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
                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::Fetch,
                            per_project,
                            recovery_hints: vec![],
                        });
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
                                project_id: p.project_id.clone(),
                                project_name: p.project_name.clone(),
                                outcome: p.result.effective_outcome(),
                                success: p.result.success,
                                skip_reason: p.result.skip_reason.clone(),
                                commands_executed: p.result.commands_executed.clone(),
                                stdout: p.result.stdout.clone(),
                                stderr: p.result.stderr.clone(),
                                log_expanded: false,
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
                            recovery_hints: hints.clone(),
                        });

                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::SmartPull,
                            per_project: outcomes,
                            recovery_hints: hints,
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

        BackgroundMessage::SingleFetchCompleted { lease_id, log } => {
            fetch::single_fetch_completed(state, lease_id, log)
        }

        BackgroundMessage::BulkFetchCompleted(log) => fetch::bulk_fetch_completed(state, log),

        BackgroundMessage::SmartPullCompleted(log)
        | BackgroundMessage::ContextSwitchCompleted(log)
        | BackgroundMessage::FreezeCompleted(log) => {
            persist_log(&log, state);
            Task::none()
        }

        BackgroundMessage::TagPushCompleted {
            lease_id,
            success_count,
            fail_count,
        } => freeze::tag_push_completed(state, lease_id, success_count, fail_count),

        BackgroundMessage::MissingProjectsDetected(ids) => {
            status::missing_projects_detected(state, ids)
        }

        BackgroundMessage::ConflictFilesLoaded(detail) => {
            conflict::conflict_files_loaded(state, detail)
        }

        BackgroundMessage::ConflictOperationCompleted {
            lease_id,
            result,
            detail,
        } => conflict::conflict_operation_completed(state, lease_id, result, detail),

        BackgroundMessage::ChangelogDraftReady { request_id, draft } => {
            status::changelog_draft_ready(state, request_id, draft)
        }

        BackgroundMessage::TagsLoaded(tags) => freeze::tags_loaded(state, tags),

        BackgroundMessage::TopologyScanned(graph) => status::topology_scanned(state, graph),

        BackgroundMessage::FreezeValidationDone {
            lease_id,
            validation,
        } => freeze::freeze_validation_done(state, lease_id, validation),

        BackgroundMessage::FreezeExecutionDone { lease_id, result } => {
            freeze::freeze_execution_done(state, lease_id, result)
        }

        BackgroundMessage::ContextListLoaded(list) => {
            context_switch::context_list_loaded(state, list)
        }

        BackgroundMessage::ContextSwitchDone { lease_id, result } => {
            context_switch::context_switch_done(state, lease_id, result)
        }

        BackgroundMessage::TaskError { description } => status::task_error(state, description),
    }
}
