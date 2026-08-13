//! Changelog auto-aggregation domain types.

use crate::model::project::ProjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single commit entry collected for the changelog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEntry {
    /// Short commit hash (8 chars).
    pub hash: String,
    /// Commit subject (first line of message).
    pub subject: String,
    /// Author name.
    pub author: String,
    /// Commit date.
    pub date: DateTime<Utc>,
}

/// Commits collected from one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommits {
    pub project_id: ProjectId,
    pub project_name: String,
    /// The "since" reference used (tag name or commit hash).
    pub since_ref: String,
    /// Commits in reverse-chronological order.
    pub entries: Vec<CommitEntry>,
    pub error: Option<String>,
}

/// The most recent commits for one project, with no since-ref (RFC-039 D2)
/// — the project detail panel's "Recent commits" section, not changelog
/// generation. A distinct type from `ProjectCommits` rather than a reuse:
/// that type's `since_ref: String` has no meaning for this query, and a
/// `String` field accepts anything put in it — the same shape of defect
/// RFC-046 spent an RFC removing from `skip_reason`, which held a stable
/// code from three writers and locale-baked rendered prose from a fourth
/// for as long as nothing distinguished the two meanings. `project_name` is
/// also dropped: the panel already has it from the project it opened for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentCommits {
    pub project_id: ProjectId,
    /// Commits in reverse-chronological order.
    pub entries: Vec<CommitEntry>,
    pub error: Option<String>,
}

/// A complete changelog draft spanning multiple repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogDraft {
    /// The freeze / release name this changelog is for.
    pub release_name: String,
    /// UTC timestamp of generation.
    pub generated_at: DateTime<Utc>,
    pub projects: Vec<ProjectCommits>,
}

impl ChangelogDraft {
    /// Render the draft as a Markdown string.
    pub fn to_markdown(&self) -> String {
        let mut md = format!(
            "# Changelog — {}\n\n_Generated {}_\n\n",
            self.release_name,
            self.generated_at.format("%Y-%m-%d %H:%M UTC")
        );

        for proj in &self.projects {
            if proj.entries.is_empty() && proj.error.is_none() {
                continue; // nothing to show
            }
            md.push_str(&format!("## {}\n\n", proj.project_name));
            if let Some(ref e) = proj.error {
                md.push_str(&format!("_Error collecting commits: {e}_\n\n"));
                continue;
            }
            if proj.entries.is_empty() {
                md.push_str("_No commits since last release._\n\n");
                continue;
            }
            md.push_str(&format!("_(since `{}`)_\n\n", proj.since_ref));
            for entry in &proj.entries {
                md.push_str(&format!(
                    "- `{}` {} — _{}_\n",
                    &entry.hash[..8.min(entry.hash.len())],
                    entry.subject,
                    entry.author
                ));
            }
            md.push('\n');
        }
        md
    }

    /// Total number of commits across all projects.
    pub fn total_commits(&self) -> usize {
        self.projects.iter().map(|p| p.entries.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::ProjectId;

    fn make_entry(hash: &str, subject: &str) -> CommitEntry {
        CommitEntry {
            hash: hash.to_owned(),
            subject: subject.to_owned(),
            author: "Author".to_owned(),
            date: Utc::now(),
        }
    }

    #[test]
    fn markdown_includes_project_name_and_commits() {
        let draft = ChangelogDraft {
            release_name: "v1.0.0".to_owned(),
            generated_at: Utc::now(),
            projects: vec![ProjectCommits {
                project_id: ProjectId::new(),
                project_name: "api-server".to_owned(),
                since_ref: "v0.9.0".to_owned(),
                entries: vec![make_entry("abcdef12", "Add rate limiting")],
                error: None,
            }],
        };
        let md = draft.to_markdown();
        assert!(md.contains("## api-server"));
        assert!(md.contains("Add rate limiting"));
        assert!(md.contains("v1.0.0"));
    }

    #[test]
    fn markdown_skips_empty_projects() {
        let draft = ChangelogDraft {
            release_name: "v2.0.0".to_owned(),
            generated_at: Utc::now(),
            projects: vec![ProjectCommits {
                project_id: ProjectId::new(),
                project_name: "unchanged-lib".to_owned(),
                since_ref: "v1.9.0".to_owned(),
                entries: vec![],
                error: None,
            }],
        };
        let md = draft.to_markdown();
        assert!(!md.contains("## unchanged-lib"));
    }

    #[test]
    fn total_commits_sums_all_projects() {
        let draft = ChangelogDraft {
            release_name: "v1.0.0".to_owned(),
            generated_at: Utc::now(),
            projects: vec![
                ProjectCommits {
                    project_id: ProjectId::new(),
                    project_name: "a".to_owned(),
                    since_ref: "v0.9.0".to_owned(),
                    entries: vec![make_entry("aaa", "x"), make_entry("bbb", "y")],
                    error: None,
                },
                ProjectCommits {
                    project_id: ProjectId::new(),
                    project_name: "b".to_owned(),
                    since_ref: "v0.9.0".to_owned(),
                    entries: vec![make_entry("ccc", "z")],
                    error: None,
                },
            ],
        };
        assert_eq!(draft.total_commits(), 3);
    }
}
