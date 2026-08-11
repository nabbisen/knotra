//! Context Operations screen state.

use knotra_vcs::{ContextCandidate, ContextList, ContextSwitchResult, ContextTarget, ProjectId};

// ---------------------------------------------------------------------------
// Phase enum
// ---------------------------------------------------------------------------

/// The active phase of the Context Operations workflow.
#[derive(Debug, Clone, Default)]
pub enum ContextPhase {
    /// No project selected; waiting for the user to pick one.
    #[default]
    Idle,
    /// Loading the branch/changeset list for the selected project.
    LoadingList(ProjectId),
    /// List is ready; user is browsing candidates.
    BrowsingList {
        project_id: ProjectId,
        list: ContextList,
        /// Filter text typed by the user.
        search: String,
    },
    /// User chose a target; showing the confirmation dialog.
    ConfirmSwitch {
        project_id: ProjectId,
        project_name: String,
        target: ContextTarget,
        target_label: String,
        /// True when the working tree was detected as dirty.
        is_dirty: bool,
        disabled_reason_key: Option<&'static str>,
    },
    /// Switch in progress.
    Switching { target_label: String },
    /// Switch completed — show result.
    Done(ContextSwitchResult),
}

// ---------------------------------------------------------------------------
// State struct
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ContextOpsState {
    pub phase: ContextPhase,
    /// Context lists cached per project (invalidated on switch / refresh).
    pub cached_lists: std::collections::HashMap<ProjectId, ContextList>,
}

impl ContextOpsState {
    /// Return filtered candidates for the currently browsed project.
    pub fn filtered_candidates(&self) -> Vec<&ContextCandidate> {
        if let ContextPhase::BrowsingList { list, search, .. } = &self.phase {
            let q = search.to_lowercase();
            list.candidates
                .iter()
                .filter(|c| q.is_empty() || c.label.to_lowercase().contains(&q))
                .collect()
        } else {
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::{ContextCandidate, ContextList, ContextTarget, ProjectId, VcsKind};

    fn make_list(candidates: Vec<(&str, &str, bool)>) -> ContextList {
        ContextList {
            project_id: ProjectId::new(),
            vcs_kind: VcsKind::Git,
            candidates: candidates
                .into_iter()
                .map(|(label, target, is_current)| ContextCandidate {
                    label: label.to_owned(),
                    target: ContextTarget::GitLocalBranch {
                        name: target.to_owned(),
                    },
                    is_current,
                })
                .collect(),
            warning: None,
        }
    }

    #[test]
    fn filter_candidates_empty_search_returns_all() {
        let list = make_list(vec![
            ("main", "main", true),
            ("feature", "feature", false),
            ("hotfix", "hotfix", false),
        ]);
        let id = list.project_id.clone();
        let state = ContextOpsState {
            phase: ContextPhase::BrowsingList {
                project_id: id,
                list,
                search: String::new(),
            },
            ..Default::default()
        };
        assert_eq!(state.filtered_candidates().len(), 3);
    }

    #[test]
    fn filter_candidates_by_search_text() {
        let list = make_list(vec![
            ("main", "main", true),
            ("feature/x", "feature/x", false),
            ("feature/y", "feature/y", false),
        ]);
        let id = list.project_id.clone();
        let state = ContextOpsState {
            phase: ContextPhase::BrowsingList {
                project_id: id,
                list,
                search: "feature".to_owned(),
            },
            ..Default::default()
        };
        assert_eq!(state.filtered_candidates().len(), 2);
    }

    #[test]
    fn filter_candidates_case_insensitive() {
        let list = make_list(vec![("Main", "Main", true), ("Feature", "Feature", false)]);
        let id = list.project_id.clone();
        let state = ContextOpsState {
            phase: ContextPhase::BrowsingList {
                project_id: id,
                list,
                search: "main".to_owned(),
            },
            ..Default::default()
        };
        assert_eq!(state.filtered_candidates().len(), 1);
    }

    #[test]
    fn idle_phase_returns_no_candidates() {
        let state = ContextOpsState::default();
        assert!(state.filtered_candidates().is_empty());
    }
}
