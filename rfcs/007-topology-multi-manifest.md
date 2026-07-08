# RFC-007 — Topology Scan: Multi-Manifest Support

| Field    | Value                                                         |
|----------|---------------------------------------------------------------|
| Status   | Proposed — decision required                                  |
| Priority | Low — current Rust-only scope is acceptable for v0.10         |
| Effort   | Medium (new parser per manifest type)                         |
| Related  | `crates/endringer/src/model/topology.rs`, `vcs/adapter.rs`    |

## Summary

`VcsAdapter::scan_topology` reads only `Cargo.toml` manifests.  Decide
whether to add support for other ecosystems (`package.json`, `pyproject.toml`,
`go.mod`) or to document the current scope explicitly.

## Current limitation

```rust
// vcs/adapter.rs
let cargo_path = format!("{}/Cargo.toml", project.path);
let Ok(manifest) = parse_cargo_toml(&cargo_path) else { continue; };
```

Projects that use Node.js, Python, or Go manifest files produce no topology
edges.  The Freezer's "Scan Dependencies" button silently returns an empty
graph for these projects.

## Options

### Option A — Document Rust-only scope (recommended for v0.10.x)

Add a note in the Freezer UI below the scan button:

```
Dependency scan reads Cargo.toml files only.
```

And in `docs/src/guide/freezer.md`:

> The **Scan Dependencies** button performs a static analysis of `Cargo.toml`
> files in each project root.  Node.js, Python, and Go projects are not
> scanned.

No code change required.

### Option B — Add `package.json` support

`package.json` `"dependencies"` and `"devDependencies"` objects are plain JSON.
Use `serde_json` (already a transitive dependency via the workspace) to parse.

```rust
pub fn parse_package_json(content: &str) -> Result<CargoManifest, String> {
    let v: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| e.to_string())?;
    let name = v["name"].as_str().map(str::to_owned);
    let deps = ["dependencies", "devDependencies"].iter()
        .filter_map(|k| v[k].as_object())
        .flatten()
        .map(|(name, spec)| ParsedDep {
            name: name.clone(),
            version_req: spec.as_str()
                .or_else(|| spec["version"].as_str())
                .unwrap_or("*").to_owned(),
            is_path: spec.get("file").is_some(),
        })
        .collect();
    Ok(CargoManifest { package_name: name, workspace_members: vec![], dependencies: deps })
}
```

### Option C — Plugin / trait-based scanner

Define a `ManifestParser` trait and register parsers for each ecosystem.
Over-engineered for the current scale; defer.

## Recommendation

**Implement Option A now.**  Document the Rust-only scope.  Plan Option B
as a separate RFC when a user need is confirmed.

## Test Plan

None for Option A.  Option B would add unit tests for `parse_package_json`
similar to the existing `parse_cargo_toml_str` tests.

## Security Considerations

Manifest files are read from local disk only.  Package names from
`package.json` are not sanitised (they are displayed in the UI only, not
executed).
