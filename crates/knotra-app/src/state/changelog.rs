//! Changelog aggregation screen state.

use knotra_vcs::{ChangelogDraft, ProjectId};

#[derive(Debug, Clone, Default)]
pub enum ChangelogPhase {
    #[default]
    Idle,
    /// Collecting commits across projects.
    Collecting,
    /// Draft ready for review / copy.
    Ready(ChangelogDraft),
}

#[derive(Debug, Default)]
pub struct ChangelogState {
    /// The "since" reference (tag or commit hash) for the Changelog modal.
    pub since_ref: String,
    pub phase: ChangelogPhase,
    /// Available tags loaded from the first registered project (used for selector).
    pub available_tags: Vec<String>,
    /// Per-project inclusion.
    pub project_selection: std::collections::HashMap<ProjectId, bool>,
}

impl ChangelogState {
    pub fn init_selection(&mut self, project_ids: &[ProjectId]) {
        for id in project_ids {
            self.project_selection.entry(id.clone()).or_insert(true);
        }
        let id_set: std::collections::HashSet<_> = project_ids.iter().collect();
        self.project_selection.retain(|id, _| id_set.contains(id));
    }

    pub fn selected_ids(&self) -> Vec<ProjectId> {
        self.project_selection
            .iter()
            .filter(|(_, v)| **v)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn is_ready_to_collect(&self) -> bool {
        !self.since_ref.trim().is_empty()
            && self.project_selection.values().any(|&v| v)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::ProjectId;

    #[test]
    fn not_ready_without_since_ref() {
        let ids = vec![ProjectId::new()];
        let mut s = ChangelogState::default();
        s.init_selection(&ids);
        assert!(!s.is_ready_to_collect());
    }

    #[test]
    fn ready_with_since_ref_and_selection() {
        let ids = vec![ProjectId::new()];
        let mut s = ChangelogState::default();
        s.init_selection(&ids);
        s.since_ref = "v1.0.0".to_owned();
        assert!(s.is_ready_to_collect());
    }

    #[test]
    fn not_ready_when_all_deselected() {
        let id = ProjectId::new();
        let mut s = ChangelogState::default();
        s.init_selection(std::slice::from_ref(&id));
        s.since_ref = "v1.0.0".to_owned();
        s.project_selection.insert(id, false);
        assert!(!s.is_ready_to_collect());
    }
}
