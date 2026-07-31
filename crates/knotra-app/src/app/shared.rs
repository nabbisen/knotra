//! Helpers called by handlers belonging to two or more different message
//! domains (RFC-040 D1/D3). This is the only criterion for landing a
//! function here — not "feels generic" or "sounds reusable". A helper used
//! by exactly one domain stays with that domain's own module, even if moved
//! later, so this file does not become app.rs's problem reproduced under a
//! better name.

use iced::Task;
use knotra_vcs::VcsAdapter;

use crate::{
    message::{BackgroundMessage, Message},
    state::{AppState, OperationLeaseId, OperationOwner, freezer::FreezerPhase, sync::SyncPhase},
};

pub(super) fn find_project(
    state: &AppState,
    id: &knotra_vcs::ProjectId,
) -> Option<knotra_vcs::Project> {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id).cloned())
}

pub(super) fn acquire_operation(
    state: &mut AppState,
    owner: OperationOwner,
) -> Option<OperationLeaseId> {
    let lease = state.operation_interlock.try_acquire(owner);
    if lease.is_none() {
        state.status_bar = Some(state.t("plain.activity.busy").to_owned());
    }
    lease
}

/// Called from `init`, `handle_tick`, `handle_workspace`, `handle_background`,
/// and `handle_fs_watch_tick` (RFC-040 Stage 3 sweep) — every context that can
/// legitimately trigger a fresh workspace-status read shares this one `Task`
/// constructor rather than each building its own.
pub(super) fn refresh_workspace_task(state: &AppState) -> Task<Message> {
    let workspace = match &state.workspace {
        Some(ws) => ws.clone(),
        None => return Task::none(),
    };
    let max = state.config.max_concurrent_reads;
    Task::perform(
        async move { VcsAdapter::read_workspace_status(&workspace, max).await },
        |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(s)),
    )
}

/// Called from `handle_freezer` and `focus_ops::close_topmost_layer`. Single
/// handler-domain by call count alone, but `focus_ops` (not a handler) must
/// not depend on a handler module (RFC-040 risk table), so this lands here
/// rather than in `freezer.rs`.
pub(super) fn cancel_freezer_validation(state: &mut AppState) {
    if let FreezerPhase::Validating { lease_id } = state.freezer.phase {
        state.operation_interlock.release_if_matches(lease_id);
        state.freezer.phase = FreezerPhase::Idle;
    }
}

/// Called from `handle_sync`, `handle_workspace`, and
/// `focus_ops::close_topmost_layer` — shared on domain count alone (sync +
/// workspace), independent of the `focus_ops` boundary concern that also
/// applies to it.
pub(super) fn clear_sync_retry_context(state: &mut AppState) {
    invalidate_retry_preparation(state);
    state.sync.retry_exclusions.clear();
}

/// Called from `clear_sync_retry_context` (above, same file) and
/// `start_activity_smart_pull_review` (`activity` domain). Moved here with
/// `clear_sync_retry_context` rather than left in `activity.rs`: if it
/// stayed, `shared.rs` would need to import from a handler module to call it
/// from `clear_sync_retry_context`, inverting the dependency direction the
/// RFC establishes (handlers depend on `shared`, not the reverse).
pub(super) fn invalidate_retry_preparation(state: &mut AppState) {
    if let Some(preparation) = state.sync.retry_preparation.take() {
        state
            .operation_interlock
            .release_if_matches(preparation.lease_id);
        if matches!(state.sync.phase, SyncPhase::RetryPreparing) {
            state.sync.phase = SyncPhase::Idle;
        }
    }
}
