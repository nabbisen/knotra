//! gix-specific conversion helpers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use endringer_backend_core::types::CommitId;

/// Converts a signed Unix timestamp (seconds) to [`SystemTime`].
/// Negative values (pre-1970) are saturated to `UNIX_EPOCH`.
pub(crate) fn seconds_to_systemtime(seconds: i64) -> SystemTime {
    if seconds >= 0 {
        UNIX_EPOCH + Duration::from_secs(seconds as u64)
    } else {
        UNIX_EPOCH
    }
}

/// Converts a `gix::ObjectId` to a backend-agnostic [`CommitId`].
pub(crate) fn gix_id_to_commit_id(id: gix::ObjectId) -> CommitId {
    CommitId::from_bytes(id.as_slice().to_vec())
}
