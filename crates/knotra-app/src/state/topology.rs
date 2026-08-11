//! Dependency topology screen state.

use knotra_vcs::{DependencyGraph, FreezeValidation, ImpactWarning};

#[derive(Debug, Clone, Default)]
pub enum TopologyPhase {
    #[default]
    Idle,
    Scanning,
    Ready(DependencyGraph),
}

#[derive(Debug, Default)]
pub struct TopologyState {
    pub phase: TopologyPhase,
}

impl TopologyState {
    /// Compute impact warnings for a set of projects about to be frozen.
    pub fn compute_warnings(
        &self,
        graph: &DependencyGraph,
        freezing: &[String],
    ) -> Vec<ImpactWarning> {
        freezing
            .iter()
            .filter_map(|name| {
                let direct = graph.direct_dependents(name);
                if direct.is_empty() {
                    return None;
                }
                let deps: Vec<String> =
                    direct.iter().map(|e| e.from_project_name.clone()).collect();
                Some(ImpactWarning {
                    frozen_project_name: name.clone(),
                    dependent_projects: deps,
                    is_transitive: false,
                })
            })
            .collect()
    }

    /// Impact warnings for the projects a `FreezeValidation` actually
    /// includes, plus whether topology data exists to check against at all
    /// (RFC-044 D1/D3). `false` means "not checked" — distinct from `true`
    /// with an empty `Vec`, which means "checked, found nothing" — the
    /// distinction R3 requires the Freezer to state.
    pub fn warnings_for(&self, validation: &FreezeValidation) -> (Vec<ImpactWarning>, bool) {
        match &self.phase {
            TopologyPhase::Ready(graph) => {
                let freezing: Vec<String> = validation
                    .entries
                    .iter()
                    .filter(|e| e.included)
                    .map(|e| e.project_name.clone())
                    .collect();
                (self.compute_warnings(graph, &freezing), true)
            }
            TopologyPhase::Idle | TopologyPhase::Scanning => (Vec::new(), false),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_vcs::{DependencyEdge, DependencyGraph, ProjectId};

    fn edge(from: &str, to: &str) -> DependencyEdge {
        DependencyEdge {
            from_project_id: ProjectId::new(),
            from_project_name: from.to_owned(),
            to_project_name: to.to_owned(),
            version_req: "^1.0".to_owned(),
            is_path_dep: true,
        }
    }

    #[test]
    fn warnings_generated_for_dependents() {
        let graph = DependencyGraph {
            edges: vec![edge("api", "shared-lib"), edge("worker", "shared-lib")],
        };
        let state = TopologyState::default();
        let warnings = state.compute_warnings(&graph, &["shared-lib".to_owned()]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].dependent_projects.len(), 2);
    }

    #[test]
    fn no_warnings_when_no_dependents() {
        let graph = DependencyGraph {
            edges: vec![edge("api", "external-crate")],
        };
        let state = TopologyState::default();
        let warnings = state.compute_warnings(&graph, &["api".to_owned()]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warning_description_includes_names() {
        let w = ImpactWarning {
            frozen_project_name: "core".to_owned(),
            dependent_projects: vec!["frontend".to_owned()],
            is_transitive: false,
        };
        assert!(w.description().contains("core"));
        assert!(w.description().contains("frontend"));
    }

    fn validation_entry(name: &str, included: bool) -> knotra_vcs::FreezeValidationEntry {
        knotra_vcs::FreezeValidationEntry {
            project_id: ProjectId::new(),
            project_name: name.to_owned(),
            included,
            is_clean: true,
            tag_exists: false,
            notes: Vec::new(),
            blockers: Vec::new(),
        }
    }

    fn validation(entries: Vec<knotra_vcs::FreezeValidationEntry>) -> FreezeValidation {
        FreezeValidation {
            freeze_name: "v1.0.0".to_owned(),
            entries,
        }
    }

    #[test]
    fn warnings_for_reports_not_checked_when_topology_idle() {
        let state = TopologyState::default();
        let v = validation(vec![validation_entry("shared-lib", true)]);
        let (warnings, checked) = state.warnings_for(&v);
        assert!(!checked);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warnings_for_reports_checked_with_no_dependents() {
        let graph = DependencyGraph {
            edges: vec![edge("api", "external-crate")],
        };
        let state = TopologyState {
            phase: TopologyPhase::Ready(graph),
        };
        let v = validation(vec![validation_entry("api", true)]);
        let (warnings, checked) = state.warnings_for(&v);
        assert!(checked);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warnings_for_finds_dependents_of_included_projects() {
        let graph = DependencyGraph {
            edges: vec![edge("api", "shared-lib"), edge("worker", "shared-lib")],
        };
        let state = TopologyState {
            phase: TopologyPhase::Ready(graph),
        };
        let v = validation(vec![validation_entry("shared-lib", true)]);
        let (warnings, checked) = state.warnings_for(&v);
        assert!(checked);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].dependent_projects.len(), 2);
    }

    #[test]
    fn warnings_for_ignores_excluded_projects() {
        // "shared-lib" has a real dependent, but it is not included in this
        // freeze — D1's whole point: only the freeze selection is checked,
        // not every workspace project.
        let graph = DependencyGraph {
            edges: vec![edge("api", "shared-lib")],
        };
        let state = TopologyState {
            phase: TopologyPhase::Ready(graph),
        };
        let v = validation(vec![
            validation_entry("shared-lib", false),
            validation_entry("api", true),
        ]);
        let (warnings, checked) = state.warnings_for(&v);
        assert!(checked);
        assert!(warnings.is_empty());
    }
}
