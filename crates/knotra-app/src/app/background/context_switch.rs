//! Context-switch background completions (RFC-041 D1, Stage 2): context
//! list loading and context-switch completion.
//!
//! Named `context_switch.rs`, not `context.rs` (RFC-041 D5) — `app/context.rs`
//! already exists one level up; RFC-040 R1a lost a review round to exactly
//! this collision.

use iced::Task;
use knotra_vcs::{ContextList, ContextSwitchResult, OperationId, VcsAdapter};

use super::{persist_log, shared};
use crate::{
    message::{BackgroundMessage, Message},
    state::{AppState, OperationLeaseId, context::ContextPhase},
};

pub(super) fn context_list_loaded(state: &mut AppState, list: ContextList) -> Task<Message> {
    let id = list.project_id.clone();
    state
        .context_ops
        .cached_lists
        .insert(id.clone(), list.clone());
    // Only update phase if we were waiting for this exact project.
    if matches!(&state.context_ops.phase, ContextPhase::LoadingList(loading_id) if loading_id == &id)
    {
        state.context_ops.phase = ContextPhase::BrowsingList {
            project_id: id,
            list,
            search: String::new(),
        };
    }
    Task::none()
}

pub(super) fn context_switch_done(
    state: &mut AppState,
    lease_id: OperationLeaseId,
    result: ContextSwitchResult,
) -> Task<Message> {
    if !state.operation_interlock.release_if_matches(lease_id) {
        return Task::none();
    }
    use knotra_vcs::model::operation::{OperationKind, OperationLog, OperationResult};

    // Build an operation log entry.
    let op_log = OperationLog {
        result: OperationResult {
            operation_id: OperationId::new(),
            kind: OperationKind::ContextSwitch,
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
            per_project: vec![result.operation_result.clone()],
            rollback_attempted: false,
            rollback_succeeded: None,
        },
        recovery_hints: result.recovery_hint.clone().into_iter().collect(),
    };
    persist_log(&op_log, state);

    state.context_ops.phase = ContextPhase::Done(result);

    // Refresh the project's status card after a switch.
    let project = match &state.context_ops.phase {
        ContextPhase::Done(r) => shared::find_project(state, &r.project_id),
        _ => None,
    };
    if let Some(p) = project {
        Task::perform(
            async move { VcsAdapter::read_project_status(&p).await },
            |s| {
                Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                    knotra_vcs::WorkspaceStatus {
                        projects: vec![s],
                        last_refresh: Some(chrono::Utc::now()),
                    },
                ))
            },
        )
    } else {
        Task::none()
    }
}
