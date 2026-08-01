//! The conflict-resolution domain: `handle_conflict_ops` and the helpers
//! used only by it (RFC-040 Stage 3 commit 4).
//!
//! `resolve_project_file_path` is re-exported by `app.rs` as
//! `pub(crate) use conflict_ops::resolve_project_file_path;` — `tests.rs`
//! addresses it as `crate::app::resolve_project_file_path` at three sites
//! (R3), so its path must not change even though its only in-crate caller
//! (`ConflictOpsMessage::OpenInEditorRequested`) lives here.

use std::path::{Component, Path, PathBuf};

use knotra_vcs::{VcsKind, model::project::Project};

use super::shared;
use crate::{
    message::{BackgroundMessage, ConflictOpsMessage, LaunchMessage, Message},
    state::{AppState, OperationOwner, Screen, conflict_ops::ConflictPhase},
};
use iced::Task;
use knotra_vcs::VcsAdapter;

pub(crate) fn resolve_project_file_path(
    project: &Project,
    file_path: &str,
) -> Result<PathBuf, &'static str> {
    let root = std::fs::canonicalize(&project.path).map_err(|_| "plain.error.path_missing")?;
    let raw = Path::new(file_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        if raw.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err("plain.resolve.file_outside_project");
        }
        root.join(raw)
    };
    let resolved = std::fs::canonicalize(&candidate).map_err(|_| "plain.resolve.file_missing")?;
    if !resolved.starts_with(&root) {
        return Err("plain.resolve.file_outside_project");
    }
    Ok(resolved)
}

fn active_conflict_project_id(state: &AppState) -> Option<knotra_vcs::ProjectId> {
    match &state.conflict_ops.phase {
        ConflictPhase::Loading(id)
        | ConflictPhase::Browsing { project_id: id, .. }
        | ConflictPhase::Operating { project_id: id, .. }
        | ConflictPhase::Done { project_id: id, .. } => Some(id.clone()),
        ConflictPhase::Idle => match &state.active_modal {
            crate::state::ActiveModal::Resolve(id) => Some(id.clone()),
            _ => None,
        },
    }
}

fn project_supports_git_conflict_actions(
    state: &AppState,
    project_id: &knotra_vcs::ProjectId,
) -> bool {
    state
        .workspace_status
        .as_ref()
        .and_then(|ws| {
            ws.projects
                .iter()
                .find(|status| &status.project_id == project_id)
        })
        .map(|status| status.identity.vcs_kind == VcsKind::Git)
        .unwrap_or_else(|| {
            shared::find_project(state, project_id)
                .map(|project| {
                    let path = Path::new(&project.path);
                    !path.join(".jj").is_dir() && path.join(".git").exists()
                })
                .unwrap_or(false)
        })
}

fn project_has_git_merge_state(state: &AppState, project_id: &knotra_vcs::ProjectId) -> bool {
    shared::find_project(state, project_id)
        .map(|project| {
            let path = Path::new(&project.path);
            path.join(".git").join("MERGE_HEAD").exists()
        })
        .unwrap_or(false)
}

pub(super) fn handle_conflict_ops(state: &mut AppState, msg: ConflictOpsMessage) -> Task<Message> {
    match msg {
        ConflictOpsMessage::OpenRequested(preselect) => {
            state.conflict_ops.phase = ConflictPhase::Idle;
            if let Some(id) = preselect {
                state.active_modal = crate::state::ActiveModal::Resolve(id.clone());
                return Task::done(Message::ConflictOps(ConflictOpsMessage::ProjectSelected(
                    id,
                )));
            }
            Task::none()
        }

        ConflictOpsMessage::ProjectSelected(id) => {
            if let Some(cached) = state.conflict_ops.cached.get(&id).cloned() {
                state.conflict_ops.phase = ConflictPhase::Browsing {
                    project_id: id,
                    detail: cached,
                };
                return Task::none();
            }
            let project = match shared::find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::list_conflicted_files(&project).await },
                |d| Message::Background(BackgroundMessage::ConflictFilesLoaded(d)),
            )
        }

        ConflictOpsMessage::RecheckRequested(id) => {
            state.conflict_ops.cached.remove(&id);
            let project = match shared::find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            state.conflict_ops.phase = ConflictPhase::Loading(id);
            Task::perform(
                async move { VcsAdapter::list_conflicted_files(&project).await },
                |d| Message::Background(BackgroundMessage::ConflictFilesLoaded(d)),
            )
        }

        ConflictOpsMessage::MarkResolvedRequested {
            project_id,
            file_path,
        } => {
            let project = match shared::find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            if !project_supports_git_conflict_actions(state, &project_id) {
                state.conflict_ops.phase = ConflictPhase::Done {
                    project_id,
                    success: false,
                    message: state.t("plain.resolve.unsupported").to_owned(),
                    result: None,
                };
                return Task::none();
            }
            let Some(lease_id) = shared::acquire_operation(state, OperationOwner::ConflictMutation)
            else {
                return Task::none();
            };
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: project_id.clone(),
                action: state.t("plain.resolve.marking").to_owned(),
            };
            state.conflict_ops.cached.remove(&project_id);
            Task::perform(
                async move {
                    let result = VcsAdapter::mark_resolved(&project, &file_path).await;
                    let detail = VcsAdapter::list_conflicted_files(&project).await;
                    (result, detail)
                },
                move |(result, detail)| {
                    Message::Background(BackgroundMessage::ConflictOperationCompleted {
                        lease_id,
                        result,
                        detail,
                    })
                },
            )
        }

        ConflictOpsMessage::AbortMergeRequested(id) => {
            if !project_supports_git_conflict_actions(state, &id)
                || !project_has_git_merge_state(state, &id)
            {
                state.conflict_ops.phase = ConflictPhase::Done {
                    project_id: id,
                    success: false,
                    message: state.t("plain.resolve.stop_unavailable").to_owned(),
                    result: None,
                };
                return Task::none();
            }
            let project = match shared::find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let Some(lease_id) = shared::acquire_operation(state, OperationOwner::ConflictMutation)
            else {
                return Task::none();
            };
            state.conflict_ops.phase = ConflictPhase::Operating {
                project_id: id.clone(),
                action: state.t("plain.resolve.stopping").to_owned(),
            };
            state.conflict_ops.cached.remove(&id);
            Task::perform(
                async move {
                    let result = VcsAdapter::abort_merge(&project).await;
                    let detail = VcsAdapter::list_conflicted_files(&project).await;
                    (result, detail)
                },
                move |(result, detail)| {
                    Message::Background(BackgroundMessage::ConflictOperationCompleted {
                        lease_id,
                        result,
                        detail,
                    })
                },
            )
        }

        ConflictOpsMessage::AbortMergeConfirmed(id) => Task::done(Message::ConflictOps(
            ConflictOpsMessage::AbortMergeRequested(id),
        )),

        ConflictOpsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ConflictOpsMessage::FileMarkedResolved(path) => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            Task::done(Message::ConflictOps(
                ConflictOpsMessage::MarkResolvedRequested {
                    project_id,
                    file_path: path,
                },
            ))
        }
        ConflictOpsMessage::OpenInEditorRequested(path) => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            let Some(project) = shared::find_project(state, &project_id) else {
                state.status_bar = Some(state.t("plain.error.path_missing").to_owned());
                return Task::none();
            };
            let resolved = match resolve_project_file_path(&project, &path) {
                Ok(path) => path,
                Err(key) => {
                    state.status_bar = Some(state.t(key).to_owned());
                    return Task::none();
                }
            };
            Task::done(Message::Launch(LaunchMessage::OpenInEditor(
                resolved.to_string_lossy().into_owned(),
            )))
        }
        ConflictOpsMessage::AbortRequested => {
            let Some(project_id) = active_conflict_project_id(state) else {
                return Task::none();
            };
            Task::done(Message::ConflictOps(
                ConflictOpsMessage::AbortMergeRequested(project_id),
            ))
        }
        ConflictOpsMessage::PanelClosed => {
            if matches!(state.conflict_ops.phase, ConflictPhase::Operating { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}
