//! Helpers called by handlers belonging to two or more different message
//! domains (RFC-040 D1/D3). This is the only criterion for landing a
//! function here — not "feels generic" or "sounds reusable". A helper used
//! by exactly one domain stays with that domain's own module, even if moved
//! later, so this file does not become app.rs's problem reproduced under a
//! better name.

use crate::state::{AppState, OperationLeaseId, OperationOwner};

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
