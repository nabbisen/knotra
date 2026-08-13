//! Detail panel background completion (RFC-039 D4): recent-commits loaded.

use iced::Task;

use crate::{
    message::Message,
    state::{AppState, detail_panel::RecentCommitsPhase},
};

pub(super) fn recent_commits_loaded(
    state: &mut AppState,
    commits: knotra_vcs::RecentCommits,
) -> Task<Message> {
    let id = commits.project_id.clone();
    state
        .detail_panel
        .commits_cached
        .insert(id.clone(), commits.clone());
    if state.detail_panel.open_project_id.as_ref() == Some(&id) {
        state.detail_panel.commits_phase = RecentCommitsPhase::Loaded {
            project_id: id,
            commits,
        };
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn commits_for(id: &knotra_vcs::ProjectId, subject: &str) -> knotra_vcs::RecentCommits {
        knotra_vcs::RecentCommits {
            project_id: id.clone(),
            entries: vec![knotra_vcs::CommitEntry {
                hash: "deadbeef".to_owned(),
                subject: subject.to_owned(),
                author: "Test".to_owned(),
                date: chrono::Utc::now(),
            }],
            error: None,
        }
    }

    /// Cache filled, phase advanced (RFC-039 D4): mirrors
    /// `conflict_files_loaded`'s cache-insert-then-phase-transition pair.
    #[test]
    fn recent_commits_loaded_fills_cache_and_advances_phase_for_the_open_project() {
        let mut state = AppState::new(AppConfig::default());
        let id = knotra_vcs::ProjectId::new();
        state.detail_panel.open_project_id = Some(id.clone());
        let commits = commits_for(&id, "add widget");

        let _ = recent_commits_loaded(&mut state, commits.clone());

        assert_eq!(
            state.detail_panel.commits_cached.get(&id).unwrap().entries[0].subject,
            "add widget"
        );
        match &state.detail_panel.commits_phase {
            RecentCommitsPhase::Loaded { project_id, .. } => assert_eq!(project_id, &id),
            other => panic!("expected Loaded for the open project, got {other:?}"),
        }
    }

    /// Cache filled, phase left alone: the panel has since moved on to a
    /// different (or no) project by the time this background result lands,
    /// so the phase must not be clobbered with a result for a project that
    /// is no longer open — cache-insert still happens unconditionally
    /// (D4's "cache filled on completion"), same as `conflict_ops` never
    /// needing this guard only because its modal cannot switch projects
    /// mid-load; the detail panel can.
    #[test]
    fn recent_commits_loaded_does_not_clobber_the_phase_for_a_stale_project() {
        let mut state = AppState::new(AppConfig::default());
        let loaded_id = knotra_vcs::ProjectId::new();
        let now_open_id = knotra_vcs::ProjectId::new();
        state.detail_panel.open_project_id = Some(now_open_id.clone());
        state.detail_panel.commits_phase = RecentCommitsPhase::Loading(now_open_id.clone());
        let commits = commits_for(&loaded_id, "stale");

        let _ = recent_commits_loaded(&mut state, commits);

        assert!(state.detail_panel.commits_cached.contains_key(&loaded_id));
        assert!(matches!(
            &state.detail_panel.commits_phase,
            RecentCommitsPhase::Loading(id) if id == &now_open_id
        ));
    }
}
