//! Async completions: `handle_background` dispatches each
//! `BackgroundMessage` variant to its domain submodule — `status`,
//! `conflict`, `fetch`, `context_switch`, `freeze`, `smart_pull` — each
//! holding that domain's arms and any helper used only within it.
//!
//! Split by RFC-041, across four stages, out of a single 761-ELOC file that
//! RFC-040 R2/D2 had left as a declared exception to the 500-ELOC threshold:
//! that file was one `match` with 20 top-level arms, each binding variables
//! out of its own message pattern, which made it look like splitting by arm
//! would require inventing signatures around a verbatim body — RFC-041's
//! design pass found the signatures were derived, not invented, and the
//! split proceeded on that basis (RFC-041, `041-background-module-decomposition.md`).
//!
//! What stays here: `handle_background` itself, the three helpers called
//! from more than one domain (`persist_log`, `merge_workspace_status`,
//! `skipped_retry_result`), and the one arm spanning three domains
//! (`SmartPullCompleted | ContextSwitchCompleted | FreezeCompleted`) rather
//! than being duplicated three ways or given a home it doesn't belong in.

use iced::Task;
use knotra_vcs::model::operation::{
    OperationKind, OperationLog, ProjectOperationOutcome, ProjectOperationResult,
};

use super::shared;
use crate::{
    message::{BackgroundMessage, Message},
    persistence::save_operation_log,
    state::{
        ActivityRetryAction, AppState, RetryAvailability, RetryExclusion, RetryUnavailableReason,
    },
};

mod conflict;
mod context_switch;
mod detail_panel;
mod fetch;
mod freeze;
mod smart_pull;
mod status;

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

/// Task 103: `RetryExclusion` carries only an id — this looks its name up
/// via `AppState::find_project_name` (across every workspace, not only the
/// active one) rather than storing an empty string.
fn skipped_retry_result(state: &AppState, exclusion: &RetryExclusion) -> ProjectOperationResult {
    ProjectOperationResult {
        project_id: exclusion.project_id.clone(),
        project_name: state.find_project_name(&exclusion.project_id),
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
        } => fetch::activity_fetch_retry_project_completed(state, lease_id, operation_id, *result),

        BackgroundMessage::SmartPullRetryStatusReady {
            request_id,
            workspace_id,
            lease_id,
            statuses,
        } => smart_pull::smart_pull_retry_status_ready(
            state,
            request_id,
            workspace_id,
            lease_id,
            statuses,
        ),

        BackgroundMessage::SmartPullPlanReady(plan) => {
            smart_pull::smart_pull_plan_ready(state, plan)
        }

        BackgroundMessage::SmartPullProjectCompleted { lease_id, progress } => {
            smart_pull::smart_pull_project_completed(state, lease_id, *progress)
        }

        BackgroundMessage::SingleFetchCompleted { lease_id, log } => {
            fetch::single_fetch_completed(state, lease_id, log)
        }

        BackgroundMessage::TagPushCompleted {
            lease_id,
            success_count,
            fail_count,
        } => freeze::tag_push_completed(state, lease_id, success_count, fail_count),

        BackgroundMessage::ConflictFilesLoaded(detail) => {
            conflict::conflict_files_loaded(state, detail)
        }

        BackgroundMessage::ConflictOperationCompleted {
            lease_id,
            result,
            detail,
        } => conflict::conflict_operation_completed(state, lease_id, *result, detail),

        BackgroundMessage::ChangelogDraftReady { request_id, draft } => {
            status::changelog_draft_ready(state, request_id, draft)
        }

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
            context_switch::context_switch_done(state, lease_id, *result)
        }

        BackgroundMessage::RecentCommitsLoaded(commits) => {
            detail_panel::recent_commits_loaded(state, commits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::{Project, Workspace, model::operation::RetryExclusionReason};

    /// Task 103: `skipped_retry_result` is the one construction site that
    /// starts with only an id and must look the name up —
    /// `AppState::find_project_name`, across every workspace, not only the
    /// active one. Both outcomes it can reach: found, and genuinely absent
    /// (never an empty string either way).
    #[test]
    fn skipped_retry_result_resolves_the_project_name_or_none() {
        let project = Project::new("dirty-1", "/tmp/dirty-1");
        let mut workspace = Workspace::new("Demo");
        workspace.projects.push(project.clone());
        let mut state = AppState::new(crate::config::AppConfig::default());
        state.all_workspaces = vec![workspace];

        let found = RetryExclusion {
            project_id: project.id.clone(),
            reason: RetryExclusionReason::ProjectPathMissing,
        };
        let result = skipped_retry_result(&state, &found);
        assert_eq!(result.project_name.as_deref(), Some("dirty-1"));

        let missing = RetryExclusion {
            project_id: knotra_vcs::ProjectId::new(),
            reason: RetryExclusionReason::NotInActiveWorkspace,
        };
        let result = skipped_retry_result(&state, &missing);
        assert_eq!(result.project_name, None);
    }
}
