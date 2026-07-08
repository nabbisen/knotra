# RFC-006 — Accurate `log_since` Range for jj

| Field    | Value                                                          |
|----------|----------------------------------------------------------------|
| Status   | Proposed                                                       |
| Priority | Medium — changelog output is incorrect for jj repositories     |
| Effort   | Small (replace `list_commits()` call with `jj log` CLI call)   |
| Related  | `crates/endringer/src/vcs/jj.rs` (`log_since`)                 |

## Summary

`jj::log_since` currently ignores the `since_ref` argument and returns all
commits from the working-copy change.  Replace this with a `jj log -r
<since_ref>..@` invocation that correctly limits the range.

## Problem

```rust
// vcs/jj.rs — current
pub async fn log_since(project, since_ref, _until_ref) {
    ...
    match repo.list_commits().await {   // ← since_ref is never used
        Ok(commits) => commits.into_iter().filter(|c| c.timestamp <= until)...
```

The `since_ref` parameter is silently discarded.  The Changelog screen always
returns every commit in the repository regardless of the "since" bookmark.

## Background

Git's `log_since` uses `git log <ref>..HEAD` after a timestamp-based approach
proved unreliable (identical timestamps in fast CI).  jj should use the same
ref-range approach via CLI because `JjBackend` does not expose commit-range
queries through the gix path.

## Design

### Replacement implementation

```rust
pub async fn log_since(
    project: &Project,
    since_ref: &str,
    _until_ref: Option<&str>,
) -> ProjectCommits {
    let path  = project.path.clone();
    let since = since_ref.to_owned();
    let pid   = project.id.clone();
    let pname = project.name.clone();

    tokio::task::spawn_blocking(move || {
        let rev  = format!("{since}..@");
        // Template: change_id(8)|first_line|author|ISO8601 timestamp
        let tmpl = r#"change_id.short(8) ++ "|" ++ description.first_line() ++ "|" ++ author.name() ++ "|" ++ committer.timestamp().format("%Y-%m-%dT%H:%M:%S+00:00") ++ "\n""#;
        let out  = std::process::Command::new("jj")
            .args(["log", "-r", &rev, "--no-graph", "-T", tmpl])
            .current_dir(&path)
            .output();

        match out {
            Err(e) => ProjectCommits { project_id: pid, project_name: pname,
                since_ref: since, entries: vec![],
                error: Some(format!("jj not available: {e}")) },
            Ok(o) if !o.status.success() =>
                ProjectCommits { project_id: pid, project_name: pname,
                    since_ref: since, entries: vec![],
                    error: Some(String::from_utf8_lossy(&o.stderr).trim().to_owned()) },
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let entries = text.lines().filter(|l| !l.trim().is_empty())
                    .filter_map(|l| {
                        let mut p = l.splitn(4, '|');
                        let hash    = p.next()?.to_owned();
                        let subject = p.next()?.to_owned();
                        let author  = p.next()?.to_owned();
                        let date    = p.next()?.trim()
                            .parse::<chrono::DateTime<chrono::Utc>>().ok()?;
                        Some(CommitEntry { hash, subject, author, date })
                    }).collect();
                ProjectCommits { project_id: pid, project_name: pname,
                    since_ref: since, entries, error: None }
            }
        }
    }).await.unwrap_or_else(|e| ProjectCommits {
        project_id: project.id.clone(), project_name: project.name.clone(),
        since_ref: since_ref.to_owned(), entries: vec![],
        error: Some(format!("task join: {e}")),
    })
}
```

### Behaviour when `jj` is absent

Returns `error: Some("jj not available: …")`.  The Changelog screen renders
per-project error rows already; this is consistent with existing error handling.

### Revision syntax

`since_ref` in the Changelog UI is typically a jj bookmark name (e.g.
`v1.0.0`).  The expression `v1.0.0..@` is valid jj syntax when `v1.0.0`
resolves to a bookmark or change-id prefix.

## Test Plan

- Document in `docs/src/reference/faq.md` that changelog collection for jj
  requires the `jj` binary.
- A full integration test is deferred until a jj-enabled CI environment exists.

## Security Considerations

`since_ref` is user-supplied and passed as a jj revision argument.  The jj CLI
validates revision syntax; no additional sanitisation is required for local
repository access.
