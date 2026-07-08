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
    use chrono::Utc;

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
        refreshed_at: Utc::now(),
        read_error: None,
    };

    assert!(status.is_healthy());
}

#[test]
fn project_status_unhealthy_when_behind() {
    use crate::model::status::{ConflictStatus, RemoteStatus, RepositoryIdentity};
    use chrono::Utc;

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
        refreshed_at: Utc::now(),
        read_error: None,
    };

    assert!(!status.is_healthy());
    assert!(status.is_behind());
}

#[test]
fn vcs_kind_display() {
    assert_eq!(VcsKind::Git.to_string(), "Git");
    assert_eq!(VcsKind::Jujutsu.to_string(), "jj");
}
