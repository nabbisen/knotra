//! Dependency topology domain types.
//!
//! Parses `Cargo.toml` workspace and package manifests to build an inter-project
//! dependency graph. Used to warn users in the Freezer when a project they are
//! tagging has reverse dependencies that may be affected by the change.

use crate::model::project::ProjectId;
use serde::{Deserialize, Serialize};

/// A directed dependency edge: `from` depends on `to`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from_project_id: ProjectId,
    pub from_project_name: String,
    pub to_project_name: String, // matched by crate name, not ProjectId
    /// The declared version requirement string (e.g. `"^1.2"`).
    pub version_req: String,
    /// True when the dependency is via a `path = ...` entry.
    pub is_path_dep: bool,
}

/// The complete inter-project dependency graph for a workspace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub edges: Vec<DependencyEdge>,
}

impl DependencyGraph {
    /// Return all projects that directly depend on the given crate name.
    pub fn direct_dependents(&self, crate_name: &str) -> Vec<&DependencyEdge> {
        self.edges
            .iter()
            .filter(|e| e.to_project_name == crate_name)
            .collect()
    }

    /// Return all projects that transitively depend on `crate_name` (BFS).
    pub fn transitive_dependents(&self, crate_name: &str) -> Vec<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(crate_name.to_owned());

        while let Some(name) = queue.pop_front() {
            for edge in self.edges.iter().filter(|e| e.to_project_name == name) {
                if visited.insert(edge.from_project_name.clone()) {
                    queue.push_back(edge.from_project_name.clone());
                }
            }
        }
        visited.into_iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

/// A warning generated from the topology for the Freezer screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactWarning {
    /// The project being frozen.
    pub frozen_project_name: String,
    /// Projects that depend on the frozen project.
    pub dependent_projects: Vec<String>,
    pub is_transitive: bool,
}

impl ImpactWarning {
    pub fn description(&self) -> String {
        let deps = self.dependent_projects.join(", ");
        if self.is_transitive {
            format!(
                "Freezing '{}' may transitively affect: {}",
                self.frozen_project_name, deps
            )
        } else {
            format!(
                "'{}' is depended upon by: {}",
                self.frozen_project_name, deps
            )
        }
    }
}

/// Parsed summary of a Cargo.toml manifest.
#[derive(Debug, Clone, Default)]
pub struct CargoManifest {
    /// Name from `[package]` or `[workspace]`.
    pub package_name: Option<String>,
    /// `[workspace.members]` entries (for workspace roots).
    pub workspace_members: Vec<String>,
    /// Direct dependency names from `[dependencies]` and `[dev-dependencies]`.
    pub dependencies: Vec<ParsedDep>,
}

#[derive(Debug, Clone)]
pub struct ParsedDep {
    pub name: String,
    pub version_req: String,
    pub is_path: bool,
}

/// Parse a `Cargo.toml` file at the given path.
///
/// Returns a `CargoManifest` on success or a brief error string.
pub fn parse_cargo_toml(path: &str) -> Result<CargoManifest, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {e}"))?;
    parse_cargo_toml_str(&content)
}

/// Parse from a TOML string (testable without file system).
pub fn parse_cargo_toml_str(content: &str) -> Result<CargoManifest, String> {
    let table: toml::Table = content
        .parse()
        .map_err(|e| format!("TOML parse error: {e}"))?;

    let mut manifest = CargoManifest::default();

    // [package] name
    if let Some(toml::Value::Table(pkg)) = table.get("package")
        && let Some(toml::Value::String(name)) = pkg.get("name")
    {
        manifest.package_name = Some(name.clone());
    }

    // [workspace] members
    if let Some(toml::Value::Table(ws)) = table.get("workspace")
        && let Some(toml::Value::Array(members)) = ws.get("members")
    {
        for m in members {
            if let toml::Value::String(s) = m {
                manifest.workspace_members.push(s.clone());
            }
        }
    }

    // [dependencies] + [dev-dependencies]
    for section in &["dependencies", "dev-dependencies"] {
        if let Some(toml::Value::Table(deps)) = table.get(*section) {
            for (name, spec) in deps {
                let (version_req, is_path) = match spec {
                    toml::Value::String(v) => (v.clone(), false),
                    toml::Value::Table(t) => {
                        let ver = t
                            .get("version")
                            .and_then(|v| {
                                if let toml::Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| "*".to_owned());
                        let path = t.contains_key("path");
                        (ver, path)
                    }
                    _ => continue,
                };
                manifest.dependencies.push(ParsedDep {
                    name: name.clone(),
                    version_req,
                    is_path,
                });
            }
        }
    }

    Ok(manifest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::ProjectId;

    fn edge(from: &str, to: &str) -> DependencyEdge {
        DependencyEdge {
            from_project_id: ProjectId::new(),
            from_project_name: from.to_owned(),
            to_project_name: to.to_owned(),
            version_req: "^1.0".to_owned(),
            is_path_dep: false,
        }
    }

    #[test]
    fn direct_dependents_found() {
        let g = DependencyGraph {
            edges: vec![
                edge("api", "shared-lib"),
                edge("worker", "shared-lib"),
                edge("api", "other-lib"),
            ],
        };
        let deps = g.direct_dependents("shared-lib");
        assert_eq!(deps.len(), 2);
        let names: Vec<_> = deps.iter().map(|e| e.from_project_name.as_str()).collect();
        assert!(names.contains(&"api"));
        assert!(names.contains(&"worker"));
    }

    #[test]
    fn transitive_dependents_bfs() {
        // api → shared-lib → core-lib
        let g = DependencyGraph {
            edges: vec![edge("api", "shared-lib"), edge("shared-lib", "core-lib")],
        };
        let mut trans = g.transitive_dependents("core-lib");
        trans.sort();
        assert_eq!(trans, vec!["api", "shared-lib"]);
    }

    #[test]
    fn parse_cargo_toml_basic() {
        let toml = r#"
[package]
name = "my-crate"
version = "0.1.0"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }
local-lib = { path = "../local-lib", version = "0.2" }
"#;
        let manifest = parse_cargo_toml_str(toml).unwrap();
        assert_eq!(manifest.package_name.as_deref(), Some("my-crate"));
        assert_eq!(manifest.dependencies.len(), 3);
        let local = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "local-lib")
            .unwrap();
        assert!(local.is_path);
    }

    #[test]
    fn parse_cargo_toml_workspace() {
        let toml = r#"
[workspace]
members = ["crates/a", "crates/b"]
"#;
        let manifest = parse_cargo_toml_str(toml).unwrap();
        assert_eq!(manifest.workspace_members.len(), 2);
    }

    #[test]
    fn impact_warning_description() {
        let w = ImpactWarning {
            frozen_project_name: "shared-lib".to_owned(),
            dependent_projects: vec!["api".to_owned(), "worker".to_owned()],
            is_transitive: false,
        };
        assert!(w.description().contains("shared-lib"));
        assert!(w.description().contains("api"));
    }
}
