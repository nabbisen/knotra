//! Unit tests for `knotra_vcs`.

use crate::model::{
    project::ProjectId,
    status::{ProjectStatus, VcsKind, WorkingTreeStatus},
};

#[test]
fn working_tree_status_is_dirty() {
    let dirty = WorkingTreeStatus {
        uncommitted_count: 3,
        untracked_count: 0,
    };
    assert!(dirty.is_dirty());

    let clean = WorkingTreeStatus {
        uncommitted_count: 0,
        untracked_count: 0,
    };
    assert!(!clean.is_dirty());
}

#[test]
fn project_id_is_unique() {
    let a = ProjectId::new();
    let b = ProjectId::new();
    assert_ne!(a, b);
}

#[test]
fn project_status_healthy_requires_no_errors() {
    use crate::model::status::{ConflictStatus, RemoteStatus, RepositoryIdentity};

    let id = ProjectId::new();
    let status = ProjectStatus {
        project_id: id.clone(),
        identity: RepositoryIdentity {
            path: "/tmp/repo".to_owned(),
            vcs_kind: VcsKind::Git,
        },
        context: None,
        remote: RemoteStatus::default(),
        working_tree: WorkingTreeStatus::default(),
        conflict: ConflictStatus::default(),
        refreshed_at: jiff::Timestamp::now(),
        read_error: None,
    };

    assert!(status.is_healthy());
}

#[test]
fn project_status_unhealthy_when_behind() {
    use crate::model::status::{ConflictStatus, RemoteStatus, RepositoryIdentity};

    let id = ProjectId::new();
    let status = ProjectStatus {
        project_id: id.clone(),
        identity: RepositoryIdentity {
            path: "/tmp/repo".to_owned(),
            vcs_kind: VcsKind::Git,
        },
        context: None,
        remote: RemoteStatus {
            ahead: 0,
            behind: 2,
            upstream: Some("origin/main".to_owned()),
        },
        working_tree: WorkingTreeStatus::default(),
        conflict: ConflictStatus::default(),
        refreshed_at: jiff::Timestamp::now(),
        read_error: None,
    };

    assert!(!status.is_healthy());
    assert!(status.is_behind());
}

#[test]
fn vcs_kind_display() {
    assert_eq!(VcsKind::Git.to_string(), "Git");
    assert_eq!(VcsKind::Jujutsu.to_string(), "Jujutsu");
}

/// Task 113: `git.rs`'s and `jj.rs`'s commit-date parse (`%aI` from git,
/// jj's own `+00:00`-suffixed template) both hand a plain RFC3339 string,
/// with an explicit non-UTC offset here, to `.parse::<jiff::Timestamp>()`.
/// 2024-03-04T14:06:07+09:00 is 2024-03-04T05:06:07Z (same calendar day,
/// -9h, chosen so the offset conversion is actually exercised) — confirmed
/// against chrono's own `.parse::<DateTime<Utc>>()` before this task's
/// migration, same expected second count, before the parse call itself
/// was switched over.
#[test]
fn commit_date_parse_resolves_a_non_utc_offset_to_the_same_instant() {
    let parsed = "2024-03-04T14:06:07+09:00"
        .parse::<jiff::Timestamp>()
        .expect("must parse");
    assert_eq!(parsed.as_second(), 1_709_528_767);
}
