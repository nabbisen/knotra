//! The project detail panel domain: `handle_detail_panel` (RFC-039 D4).
//!
//! `DetailPanelMessage::Opened` mirrors `ConflictOpsMessage::ProjectSelected`
//! (`conflict_ops.rs`): cache checked first, a background task dispatched
//! only on a miss, the cache filled and the phase advanced together on
//! completion (`app/background/detail_panel.rs`).

use iced::Task;

use super::shared;
use crate::{
    message::{BackgroundMessage, DetailPanelMessage, Message},
    state::{AppState, detail_panel::RecentCommitsPhase},
};
use knotra_vcs::VcsAdapter;

/// Recent-commits entry count (RFC-039 D6) — matches "Recent operations"
/// directly above it in the panel. Not configurable.
pub(super) const RECENT_COMMITS_LIMIT: usize = 5;

pub(super) fn handle_detail_panel(state: &mut AppState, msg: DetailPanelMessage) -> Task<Message> {
    match msg {
        DetailPanelMessage::Opened(id) => {
            state.detail_panel.open_project_id = Some(id.clone());
            if let Some(cached) = state.detail_panel.commits_cached.get(&id).cloned() {
                state.detail_panel.commits_phase = RecentCommitsPhase::Loaded {
                    project_id: id,
                    commits: cached,
                };
                return Task::none();
            }
            let project = match shared::find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            state.detail_panel.commits_phase = RecentCommitsPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::recent_commits(&project, RECENT_COMMITS_LIMIT).await },
                |commits| Message::Background(BackgroundMessage::RecentCommitsLoaded(commits)),
            )
        }
        DetailPanelMessage::Closed => {
            state.detail_panel.open_project_id = None;
            state.detail_panel.commits_phase = RecentCommitsPhase::Idle;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use knotra_vcs::{Project, RecentCommits, Workspace};

    fn state_with_project(name: &str) -> (AppState, knotra_vcs::ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new(name, "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Test")
        });
        (state, id)
    }

    /// Cache miss (RFC-039 D4, mirroring `ConflictOpsMessage::ProjectSelected`
    /// at a cold `conflict_ops::cached`): the phase transitions to `Loading`
    /// for the opened project rather than dispatching straight to `Loaded`.
    #[test]
    fn opened_with_a_cold_cache_enters_loading() {
        let (mut state, id) = state_with_project("alpha");

        let _ = handle_detail_panel(&mut state, DetailPanelMessage::Opened(id.clone()));

        assert_eq!(state.detail_panel.open_project_id, Some(id.clone()));
        assert!(matches!(
            state.detail_panel.commits_phase,
            RecentCommitsPhase::Loading(loading_id) if loading_id == id
        ));
    }

    /// Cache hit: the synchronous `Loaded` transition, no background task —
    /// same shape as `conflict_ops`'s `cached.get(&id).cloned()` branch.
    #[test]
    fn opened_with_a_warm_cache_enters_loaded_synchronously() {
        let (mut state, id) = state_with_project("alpha");
        let cached = RecentCommits {
            project_id: id.clone(),
            entries: vec![],
            error: None,
        };
        state
            .detail_panel
            .commits_cached
            .insert(id.clone(), cached.clone());

        let _ = handle_detail_panel(&mut state, DetailPanelMessage::Opened(id.clone()));

        match &state.detail_panel.commits_phase {
            RecentCommitsPhase::Loaded {
                project_id,
                commits,
            } => {
                assert_eq!(project_id, &id);
                assert_eq!(commits.entries.len(), cached.entries.len());
            }
            other => panic!("expected Loaded from a warm cache, got {other:?}"),
        }
    }

    /// `Closed` resets both the open project and the commits phase — a
    /// stale `Loading`/`Loaded` from the previously open project must not
    /// leak into the next `Opened` (`RecentCommitsPhase::Idle` is the only
    /// state the view's catch-all is meant to see between panels).
    #[test]
    fn closed_resets_open_project_and_phase() {
        let (mut state, id) = state_with_project("alpha");
        let _ = handle_detail_panel(&mut state, DetailPanelMessage::Opened(id));

        let _ = handle_detail_panel(&mut state, DetailPanelMessage::Closed);

        assert_eq!(state.detail_panel.open_project_id, None);
        assert!(matches!(
            state.detail_panel.commits_phase,
            RecentCommitsPhase::Idle
        ));
    }
}
