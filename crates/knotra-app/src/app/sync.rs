//! The sync (smart pull / bulk fetch) domain: `handle_sync`, its
//! `start_bulk_fetch` continuation, and the helper used only by them
//! (RFC-040 Stage 3 commit 6).

use iced::Task;
use iced::futures::StreamExt;
use knotra_vcs::{
    VcsAdapter,
    model::{
        operation::{
            OperationId, ProjectOperationOutcome, ProjectOperationResult, SmartPullDisposition,
            SmartPullProgress, SmartPullSkipReason,
        },
        project::Project,
    },
};

use super::focus_ops::smart_pull_is_running;
use super::shared::{acquire_operation, clear_sync_retry_context};
use crate::{
    message::{BackgroundMessage, Message, SyncMessage},
    state::{
        AppState, OperationOwner,
        sync::{ProjectOutcome, SyncKind, SyncPhase, SyncResult},
    },
};

fn smart_pull_skip_reason_text(reason: &SmartPullSkipReason) -> &'static str {
    match reason {
        SmartPullSkipReason::Deselected => "Not selected.",
        SmartPullSkipReason::NoUpstream => "No update source is configured.",
        SmartPullSkipReason::Conflict => "Needs your choice first.",
        SmartPullSkipReason::MissingStatus => "Status is not available.",
        SmartPullSkipReason::ProjectNotFound => "Project was not found.",
    }
}

pub(super) fn handle_sync(state: &mut AppState, msg: SyncMessage) -> Task<Message> {
    match msg {
        SyncMessage::OpenRequested => {
            clear_sync_retry_context(state);
            if let Some(ws) = &state.workspace {
                state.sync.init_selection(&ws.projects);
            }
            state.active_modal = crate::state::ActiveModal::Pull;
            Task::none()
        }

        SyncMessage::ProjectToggled(id, included) => {
            state.sync.project_selection.insert(id, included);
            Task::none()
        }

        SyncMessage::DispositionChanged(id, disposition) => {
            state
                .sync
                .disposition_overrides
                .insert(id.clone(), disposition.clone());
            if let SyncPhase::AwaitingConfirm(plan) = &mut state.sync.phase
                && let Some(entry) = plan.entries.iter_mut().find(|entry| entry.project_id == id)
            {
                entry.disposition = disposition;
                entry.skip_reason = None;
            }
            Task::none()
        }

        SyncMessage::PlanRequested => {
            if state.sync.retry_preparation.is_none() {
                state.sync.retry_exclusions.clear();
            }
            // Open the pull modal and start planning.
            state.active_modal = crate::state::ActiveModal::Pull;
            state.sync.phase = SyncPhase::Planning;
            Task::done(Message::Sync(SyncMessage::SmartPullPlanRequested))
        }
        SyncMessage::ExecuteRequested => {
            if let SyncPhase::AwaitingConfirm(plan) = &state.sync.phase {
                Task::done(Message::Sync(SyncMessage::SmartPullConfirmed(plan.clone())))
            } else {
                Task::none()
            }
        }
        SyncMessage::BulkFetchRequested => {
            let (ids, fetchable_ids): (Vec<_>, Vec<_>) = if state.selection_mode {
                let summary = state.selection_summary();
                state.sync.selected_project_ids = summary.selected_ids.iter().cloned().collect();
                if let Some(ws) = &state.workspace {
                    state
                        .sync
                        .set_selection(ws.projects.as_slice(), &state.selection.selected_ids);
                }
                (summary.selected_ids, summary.fetchable_ids)
            } else {
                let ids = state.sync.selected_ids();
                let fetchable_ids = ids
                    .iter()
                    .filter(|id| !state.missing_projects.contains(*id))
                    .cloned()
                    .collect();
                (ids, fetchable_ids)
            };
            start_bulk_fetch(state, ids, fetchable_ids)
        }

        SyncMessage::BulkFetchAllRequested => {
            let Some(ws) = &state.workspace else {
                return Task::none();
            };
            let ids: Vec<_> = ws
                .projects
                .iter()
                .map(|project| project.id.clone())
                .collect();
            let fetchable_ids: Vec<_> = ids
                .iter()
                .filter(|id| !state.missing_projects.contains(*id))
                .cloned()
                .collect();
            state.sync.selected_project_ids = fetchable_ids.iter().cloned().collect();
            state.sync.project_selection.clear();
            for project in &ws.projects {
                state
                    .sync
                    .project_selection
                    .insert(project.id.clone(), fetchable_ids.contains(&project.id));
            }
            start_bulk_fetch(state, ids, fetchable_ids)
        }

        SyncMessage::SmartPullPlanRequested => {
            let Some(lease_id) = acquire_operation(state, OperationOwner::SmartPullPreparation)
            else {
                state.sync.phase = SyncPhase::Idle;
                return Task::none();
            };
            state.sync.phase = SyncPhase::Planning;
            // Build the plan synchronously from existing status.
            let selected_projects: Vec<Project> = state
                .workspace
                .as_ref()
                .map(|w| {
                    if state.sync.selected_project_ids.is_empty() {
                        w.projects.clone()
                    } else {
                        w.projects
                            .iter()
                            .filter(|p| state.sync.selected_project_ids.contains(&p.id))
                            .cloned()
                            .collect()
                    }
                })
                .unwrap_or_default();
            let plan = state
                .sync
                .build_plan(&selected_projects, state.workspace_status.as_ref());
            state.sync.phase = SyncPhase::AwaitingConfirm(plan.clone());
            state.operation_interlock.release_if_matches(lease_id);
            Task::done(Message::Background(BackgroundMessage::SmartPullPlanReady(
                plan,
            )))
        }

        SyncMessage::SmartPullConfirmed(plan) => {
            let Some(lease_id) = acquire_operation(state, OperationOwner::SmartPullExecution)
            else {
                return Task::none();
            };
            let projects_map: std::collections::HashMap<_, _> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .map(|p| (p.id.clone(), p.clone()))
                        .collect()
                })
                .unwrap_or_default();

            let entries = plan.entries.clone();
            state.sync.phase = SyncPhase::PullRunning {
                plan,
                lease_id,
                started_at: chrono::Utc::now(),
                completed: Vec::new(),
            };

            use iced::futures::stream;

            let pull_stream = stream::iter(entries).then(move |entry| {
                let project = projects_map.get(&entry.project_id).cloned();
                async move {
                    let Some(project) = project else {
                        return SmartPullProgress {
                            project_id: entry.project_id.clone(),
                            project_name: entry.project_name.clone(),
                            result: knotra_vcs::model::operation::ProjectOperationResult {
                                project_id: entry.project_id,
                                outcome: ProjectOperationOutcome::Failed,
                                success: false,
                                skip_reason: None,
                                commands_executed: vec![],
                                stdout: String::new(),
                                stderr: String::new(),
                                exit_code: None,
                                error_message: Some("project not found".to_owned()),
                            },
                            recovery_hint: None,
                        };
                    };

                    match entry.disposition {
                        SmartPullDisposition::Excluded => SmartPullProgress {
                            project_id: project.id.clone(),
                            project_name: entry.project_name.clone(),
                            result: knotra_vcs::model::operation::ProjectOperationResult {
                                project_id: project.id.clone(),
                                outcome: ProjectOperationOutcome::Skipped,
                                success: true,
                                skip_reason: entry
                                    .skip_reason
                                    .as_ref()
                                    .map(smart_pull_skip_reason_text)
                                    .map(str::to_owned),
                                commands_executed: vec![],
                                stdout: "[excluded]".to_owned(),
                                stderr: String::new(),
                                exit_code: Some(0),
                                error_message: None,
                            },
                            recovery_hint: None,
                        },
                        SmartPullDisposition::FetchOnly => {
                            let r = VcsAdapter::fetch(&project).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: None,
                            }
                        }
                        SmartPullDisposition::Pull | SmartPullDisposition::StashAndPull => {
                            let stash =
                                matches!(entry.disposition, SmartPullDisposition::StashAndPull);
                            let (r, hint) = VcsAdapter::smart_pull(&project, stash).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: hint,
                            }
                        }
                    }
                }
            });

            Task::run(pull_stream, move |progress| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted {
                    lease_id,
                    progress,
                })
            })
        }

        SyncMessage::SmartPullCancelled => {
            clear_sync_retry_context(state);
            state.sync.phase = SyncPhase::Idle;
            Task::none()
        }

        SyncMessage::ModalClosed => {
            if !smart_pull_is_running(state) {
                clear_sync_retry_context(state);
                state.active_modal = crate::state::ActiveModal::None;
            }
            Task::none()
        }
        SyncMessage::Cancelled => {
            if !smart_pull_is_running(state) {
                clear_sync_retry_context(state);
                state.active_modal = crate::state::ActiveModal::None;
            }
            Task::none()
        }
        SyncMessage::BulkPullRequested => {
            clear_sync_retry_context(state);
            state.active_modal = crate::state::ActiveModal::Pull;
            state.sync.phase = SyncPhase::Planning;
            state.sync.selected_project_ids = state.selection.selected_ids.clone();
            if let Some(ws) = &state.workspace {
                state
                    .sync
                    .set_selection(&ws.projects, &state.selection.selected_ids);
            }
            Task::done(Message::Sync(SyncMessage::SmartPullPlanRequested))
        }
    }
}

fn start_bulk_fetch(
    state: &mut AppState,
    ids: Vec<knotra_vcs::ProjectId>,
    fetchable_ids: Vec<knotra_vcs::ProjectId>,
) -> Task<Message> {
    if ids.is_empty() {
        return Task::none();
    }

    let project_map: std::collections::HashMap<_, _> = state
        .workspace
        .as_ref()
        .map(|ws| {
            ws.projects
                .iter()
                .map(|p| (p.id.clone(), p.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut skipped = Vec::new();
    let mut skipped_results = Vec::new();
    let projects: Vec<_> = fetchable_ids
        .iter()
        .filter_map(|id| project_map.get(id).cloned())
        .collect();
    for id in ids {
        if !project_map.contains_key(&id) || state.missing_projects.contains(&id) {
            let project_name = project_map
                .get(&id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| state.t("plain.project").to_owned());
            let result = ProjectOperationResult {
                project_id: id.clone(),
                outcome: ProjectOperationOutcome::Skipped,
                success: true,
                skip_reason: Some(state.t("plain.fetch.skipped_unavailable").to_owned()),
                commands_executed: Vec::new(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error_message: None,
            };
            skipped_results.push(result.clone());
            skipped.push(ProjectOutcome {
                project_id: id,
                project_name,
                outcome: result.effective_outcome(),
                success: result.success,
                skip_reason: result.skip_reason,
                commands_executed: result.commands_executed,
                stdout: result.stdout,
                stderr: result.stderr,
                log_expanded: false,
            });
        }
    }
    let total = projects.len() + skipped.len();
    if total == 0 {
        return Task::none();
    }
    let done = skipped.len();
    if projects.is_empty() {
        state.sync.phase = SyncPhase::Done(SyncResult {
            kind: SyncKind::Fetch,
            per_project: skipped,
            recovery_hints: Vec::new(),
        });
        return Task::none();
    }
    let Some(lease_id) = acquire_operation(state, OperationOwner::BulkFetch) else {
        return Task::none();
    };
    let operation_id = OperationId::new();
    state.sync.phase = SyncPhase::FetchRunning {
        operation_id,
        lease_id,
        started_at: chrono::Utc::now(),
        total,
        done,
        completed: skipped,
        operation_results: skipped_results,
    };

    use iced::futures::stream;

    let project_stream = stream::iter(projects)
        .then(move |project| async move { VcsAdapter::fetch(&project).await });

    Task::run(project_stream, move |per_project_result| {
        Message::Background(BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            progress: SmartPullProgress {
                project_id: per_project_result.project_id.clone(),
                project_name: String::new(),
                result: per_project_result,
                recovery_hint: None,
            },
        })
    })
}
