//! Conflict background completions (RFC-041 D1, Stage 1): conflict file
//! loading and conflict-mutation completion.

use iced::Task;
use knotra_vcs::model::operation::ProjectOperationResult;

use crate::{
    message::Message,
    state::{AppState, OperationLeaseId, conflict_ops::ConflictPhase},
};

pub(super) fn conflict_files_loaded(
    state: &mut AppState,
    detail: knotra_vcs::ProjectConflictDetail,
) -> Task<Message> {
    let id = detail.project_id.clone();
    state.conflict_ops.cached.insert(id.clone(), detail.clone());
    state.conflict_ops.phase = ConflictPhase::Browsing {
        project_id: id,
        detail,
    };
    Task::none()
}

pub(super) fn conflict_operation_completed(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    result: ProjectOperationResult,
    detail: knotra_vcs::ProjectConflictDetail,
) -> Task<Message> {
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    let id = detail.project_id.clone();
    let success = result.success;
    let message = if success {
        state.t("plain.resolve.done").to_owned()
    } else {
        state.t("plain.resolve.failed").to_owned()
    };
    state.conflict_ops.cached.insert(id.clone(), detail.clone());
    if success {
        state.conflict_ops.phase = ConflictPhase::Browsing {
            project_id: id,
            detail,
        };
    } else {
        state.conflict_ops.phase = ConflictPhase::Done {
            project_id: id,
            success,
            message,
            result: Some(result),
        };
    }
    Task::none()
}
