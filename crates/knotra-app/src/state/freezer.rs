//! Freezer screen state.

use std::collections::HashSet;

use knotra_vcs::{FreezeResult, FreezeValidation, ImpactWarning, ProjectId};

use super::OperationLeaseId;

// ---------------------------------------------------------------------------
// Phase FSM
// ---------------------------------------------------------------------------

/// The active phase of the Freezer workflow.
#[derive(Debug, Clone, Default)]
pub enum FreezerPhase {
    /// Idle — user is entering the freeze name and selecting projects.
    #[default]
    Idle,
    /// Validation in progress.
    Validating { lease_id: OperationLeaseId },
    /// Validation complete — awaiting user confirmation.
    ValidationReady(FreezeValidation),
    /// Execution in progress.
    Executing,
    /// Execution complete — show result.
    Done(FreezeResult),
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct FreezerState {
    pub phase: FreezerPhase,
    /// When the current execution phase started. Used for operation history.
    pub execution_started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// The name of the freeze point (tag / bookmark name).
    pub freeze_name: String,
    /// Optional annotation message. Empty = lightweight tag.
    pub tag_message: String,
    /// RFC-044 D1: impact warnings for the current `ValidationReady`,
    /// computed once at validation time from the freeze selection —
    /// not `FreezeValidation`'s own payload (R8 forbids reshaping that;
    /// `tests.rs` constructs it as a plain tuple in several places), and
    /// not a whole-workspace cache on `TopologyState` (that shape is what
    /// went stale before). Set in lockstep with `phase` entering
    /// `ValidationReady`, cleared wherever `phase` leaves it.
    pub impact_warnings: Vec<ImpactWarning>,
    /// Whether topology data existed to check against for the current
    /// `ValidationReady`. `false` is "not checked" — distinct from `true`
    /// with `impact_warnings` empty, "checked, found nothing" (R3).
    pub topology_checked: bool,
    /// Per-project inclusion: true = include in freeze.
    pub project_selection: std::collections::HashMap<ProjectId, bool>,
}

impl FreezerState {
    /// Initialise project selection from workspace (all included by default).
    pub fn init_selection(&mut self, project_ids: &[ProjectId]) {
        for id in project_ids {
            self.project_selection.entry(id.clone()).or_insert(true);
        }
        // Remove stale entries.
        let id_set: HashSet<_> = project_ids.iter().collect();
        self.project_selection.retain(|id, _| id_set.contains(id));
    }

    pub fn selected_ids(&self) -> HashSet<ProjectId> {
        self.project_selection
            .iter()
            .filter(|(_, v)| **v)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn freeze_name_is_valid(&self) -> bool {
        let n = self.freeze_name.trim();
        // Basic tag name rules: non-empty, no spaces, no control chars.
        !n.is_empty()
            && !n.contains(' ')
            && !n.contains('\t')
            && n.chars().all(|c| c.is_ascii() && !c.is_control())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::ProjectId;

    fn make_ids(n: usize) -> Vec<ProjectId> {
        (0..n).map(|_| ProjectId::new()).collect()
    }

    #[test]
    fn all_selected_by_default() {
        let ids = make_ids(3);
        let mut state = FreezerState::default();
        state.init_selection(&ids);
        assert_eq!(state.selected_ids().len(), 3);
    }

    #[test]
    fn deselect_one() {
        let ids = make_ids(3);
        let mut state = FreezerState::default();
        state.init_selection(&ids);
        state.project_selection.insert(ids[1].clone(), false);
        assert_eq!(state.selected_ids().len(), 2);
        assert_eq!(state.project_selection.get(&ids[1]), Some(&false));
    }

    #[test]
    fn valid_freeze_name_accepted() {
        let state = FreezerState {
            freeze_name: "v1.2.3".to_owned(),
            ..Default::default()
        };
        assert!(state.freeze_name_is_valid());
    }

    #[test]
    fn empty_freeze_name_rejected() {
        let state = FreezerState::default();
        assert!(!state.freeze_name_is_valid());
    }

    #[test]
    fn freeze_name_with_space_rejected() {
        let state = FreezerState {
            freeze_name: "v1 2 3".to_owned(),
            ..Default::default()
        };
        assert!(!state.freeze_name_is_valid());
    }

    #[test]
    fn stale_selection_entries_pruned_on_init() {
        let ids = make_ids(3);
        let mut state = FreezerState::default();
        state.init_selection(&ids);

        // Now "reinitialise" with only the first two projects.
        state.init_selection(&ids[..2]);
        assert_eq!(state.project_selection.len(), 2);
        assert!(!state.project_selection.contains_key(&ids[2]));
    }
}
