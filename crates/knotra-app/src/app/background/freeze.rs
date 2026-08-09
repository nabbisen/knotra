//! Freeze/tag background completions (RFC-041 D1, Stage 3): tag-push
//! completion, available-tags loading, freeze validation, and freeze
//! execution — plus the two helpers only freeze execution's push offer
//! needs (RFC-041 D3).

use iced::Task;
use knotra_vcs::{
    FreezeResult, FreezeValidation, OperationId, VcsKind, model::operation::ProjectOperationOutcome,
};

use super::{persist_log, shared};
use crate::{
    message::Message,
    state::{AppState, OperationLeaseId, PendingTagPush, freezer::FreezerPhase},
};

fn git_push_offer_for_freeze(
    state: &AppState,
    result: &knotra_vcs::FreezeResult,
) -> Option<(String, Vec<knotra_vcs::ProjectId>)> {
    if result.outcome != knotra_vcs::FreezeOutcome::Success {
        return None;
    }

    let ids: Vec<_> = result
        .project_results
        .iter()
        .filter(|r| r.success && project_is_git_for_push(state, &r.project_id))
        .map(|r| r.project_id.clone())
        .collect();

    (!ids.is_empty()).then(|| (result.freeze_name.clone(), ids))
}

fn project_is_git_for_push(state: &AppState, project_id: &knotra_vcs::ProjectId) -> bool {
    if let Some(status) = state.workspace_status.as_ref().and_then(|ws| {
        ws.projects
            .iter()
            .find(|status| &status.project_id == project_id)
    }) {
        return status.identity.vcs_kind == VcsKind::Git;
    }

    let Some(project) = shared::find_project(state, project_id) else {
        return false;
    };
    let path = std::path::Path::new(&project.path);
    !path.join(".jj").is_dir() && path.join(".git").exists()
}

pub(super) fn tag_push_completed(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    success_count: usize,
    fail_count: usize,
) -> Task<Message> {
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    state.pending_tag_push = None;
    state.status_bar = Some(if fail_count == 0 {
        format!(
            "{} — {} {}",
            state.t("plain.release.shared_status"),
            success_count,
            state.t("plain.release.projects_suffix")
        )
    } else {
        format!(
            "{}: {} {} {} {}",
            state.t("plain.release.share_failed_status"),
            success_count,
            state.t("plain.release.succeeded_suffix"),
            fail_count,
            state.t("plain.release.failed_suffix")
        )
    });
    Task::none()
}

pub(super) fn tags_loaded(state: &mut AppState, tags: Vec<String>) -> Task<Message> {
    state.changelog.available_tags = tags;
    Task::none()
}

pub(super) fn freeze_validation_done(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    validation: FreezeValidation,
) -> Task<Message> {
    if !matches!(
        state.freezer.phase,
        FreezerPhase::Validating {
            lease_id: active_lease
        } if active_lease == lease_id
    ) {
        return Task::none();
    }
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    state.freezer.phase = FreezerPhase::ValidationReady(validation);
    Task::none()
}

pub(super) fn freeze_execution_done(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    result: FreezeResult,
) -> Task<Message> {
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    use knotra_vcs::model::operation::{OperationKind, OperationLog, OperationResult};

    let started_at = state
        .freezer
        .execution_started_at
        .take()
        .unwrap_or_else(chrono::Utc::now);
    let finished_at = chrono::Utc::now();

    // Build per-project entries for the operation log.
    let per_project: Vec<_> = result
        .project_results
        .iter()
        .map(|r| knotra_vcs::model::operation::ProjectOperationResult {
            project_id: r.project_id.clone(),
            outcome: ProjectOperationOutcome::from_success(r.success),
            success: r.success,
            skip_reason: None,
            commands_executed: r.commands_executed.clone(),
            stdout: r.stdout.clone(),
            stderr: r.stderr.clone(),
            exit_code: None,
            error_message: if r.success {
                None
            } else {
                Some("freeze failed".to_owned())
            },
        })
        .collect();

    let hints: Vec<_> = result
        .project_results
        .iter()
        .filter_map(|r| r.recovery_hint.clone())
        .collect();

    let op_log = OperationLog {
        result: OperationResult {
            operation_id: OperationId::new(),
            kind: OperationKind::Freeze,
            started_at,
            finished_at,
            per_project,
            rollback_attempted: result.project_results.iter().any(|r| r.rollback_attempted),
            rollback_succeeded: {
                let any_rb = result.project_results.iter().any(|r| r.rollback_attempted);
                if any_rb {
                    Some(
                        result
                            .project_results
                            .iter()
                            .filter(|r| r.rollback_attempted)
                            .all(|r| r.rollback_succeeded == Some(true)),
                    )
                } else {
                    None
                }
            },
        },
        recovery_hints: hints,
    };
    persist_log(&op_log, state);

    let push_offer = git_push_offer_for_freeze(state, &result);
    state.freezer.phase = FreezerPhase::Done(result);

    state.pending_tag_push = push_offer.map(|(freeze_name, project_ids)| PendingTagPush {
        freeze_name,
        project_ids,
        is_pushing: false,
    });
    Task::none()
}
