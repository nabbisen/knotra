//! File-system watcher integration for knotra-app.
//!
//! Provides an `iced::Subscription` that polls registered repositories for
//! sentinel file changes and emits `Message::FsChange` when a change is
//! detected. The subscription is only active when `fs_watch_enabled = true`
//! in the application config.
//!
//! # Debouncing
//!
//! The poller runs at `fs_debounce_secs` intervals. The `FsPoller` itself
//! computes the diff, so rapid consecutive polls do not flood the UI — only
//! the *first* detection of each change triggers a message.

use iced::{Subscription, time};
use std::time::Duration;

use endringer::FsPoller;

use crate::{message::Message, state::AppState};

/// The message emitted when a FS change is detected in one or more projects.
///
/// Carries the project IDs that changed so the update handler can refresh
/// only the affected projects rather than the whole workspace.
#[derive(Debug, Clone)]
pub struct FsChangeMessage {
    pub changed_project_ids: Vec<endringer::ProjectId>,
}

/// Build the FS-watch subscription from the current app state.
///
/// Returns `Subscription::none()` when FS watching is disabled in config.
pub fn fs_watch_subscription(state: &AppState) -> Subscription<Message> {
    if !state.config.fs_watch_enabled {
        return Subscription::none();
    }

    let interval = Duration::from_secs(u64::from(state.config.fs_debounce_secs.max(1)));

    // We use the periodic tick as a trigger and rely on AppState.fs_poller (held
    // in AppState) to compute the diff each tick.
    time::every(interval).map(|_| Message::FsWatchTick)
}
