//! Dependency topology screen state.

use knotra_vcs::{DependencyGraph, ImpactWarning};

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum TopologyPhase {
    #[default]
    Idle,
    Scanning,
    Ready(DependencyGraph),
    Error(String),
}

#[derive(Debug, Default)]
pub struct TopologyState {
    pub phase: TopologyPhase,
    /// Impact warnings cached for the Freezer screen (updated after each scan).
    pub impact_warnings: Vec<ImpactWarning>,
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
}
