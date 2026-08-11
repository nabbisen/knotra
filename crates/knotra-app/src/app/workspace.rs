//! The workspace domain: `handle_workspace` and `persist_workspace`, its only
//! caller's helper (RFC-040 Stage 4).
//!
//! Depends on `focus_ops` and `shared`, the intended layering (D7).
//! Depended on by `misc.rs` (`handle_dashboard`'s error-retry path) - a
//! one-directional edge; this module must not depend on `misc.rs`.

use iced::Task;
use knotra_vcs::model::{project::Project, workspace::Workspace};

use super::focus_ops;
use super::shared;
use crate::{
    config::AppPaths,
    message::{Message, WorkspaceMessage},
    persistence::{delete_workspace_file, save_workspace},
    state::{
        AddProjectDialog, AppState, ConfirmRemoveDialog, LoadPhase, focus,
        workspace_mgr::{
            CreateWorkspaceDialog, DeleteWorkspaceDialog, RenameWorkspaceDialog,
            next_active_index_after_delete, validate_workspace_name,
        },
    },
};

fn persist_workspace(paths: &AppPaths, ws: &Workspace) {
    if let Err(e) = save_workspace(ws, paths) {
        tracing::warn!("failed to save workspace: {e}");
    }
}

pub(super) fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                state.status_bar = Some(state.t("status.refreshing").to_owned());
                shared::refresh_workspace_task(state)
            } else {
                Task::none()
            }
        }

        WorkspaceMessage::AddProjectDialogOpened => {
            state.add_project_dialog = Some(AddProjectDialog::default());
            focus_ops::open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone(),
                ),
            )
        }
        WorkspaceMessage::AddProjectNameChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectPathChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.path = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectNextStep => {
            let err_msg = state.t("plain.add_project.error_no_folder").to_owned();
            if let Some(d) = &mut state.add_project_dialog {
                if d.path.trim().is_empty() {
                    d.error = Some(err_msg);
                } else {
                    d.error = None;
                    d.step = crate::state::AddProjectStep::NameProject;
                }
            }
            focus_ops::open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
                ),
            )
        }
        WorkspaceMessage::AddProjectConfirmed => {
            let dialog = match state.add_project_dialog.take() {
                Some(d) => d,
                None => return Task::none(),
            };
            let name = dialog.name.trim().to_owned();
            let path = dialog.path.trim().to_owned();
            if name.is_empty() || path.is_empty() {
                state.add_project_dialog = Some(AddProjectDialog {
                    name: dialog.name,
                    path: dialog.path,
                    error: Some(state.t("dialog.add_project.error_empty").to_owned()),
                    ..Default::default()
                });
                return Task::none();
            }
            // Clear any pending undo when a new project is added.
            state.recent_removal = None;
            let project = Project::new(name, path);
            let paths = state.paths.clone();
            if let Some(ws) = &mut state.workspace {
                ws.add_project(project);
                persist_workspace(&paths, ws);
            }
            state.reconcile_selection_with_display();
            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            shared::refresh_workspace_task(state)
        }
        WorkspaceMessage::AddProjectCancelled => {
            state.add_project_dialog = None;
            Task::none()
        }
        WorkspaceMessage::BrowsePathRequested => Task::future(async {
            let folder = rfd::AsyncFileDialog::new()
                .set_title("Select project folder")
                .pick_folder()
                .await;
            let path = folder.map(|f| f.path().to_string_lossy().into_owned());
            Message::Workspace(crate::message::WorkspaceMessage::BrowsePathSelected(path))
        }),
        WorkspaceMessage::BrowsePathSelected(path_opt) => {
            if let Some(path) = path_opt
                && let Some(d) = &mut state.add_project_dialog
            {
                // Auto-fill name from folder name if not already set.
                if d.name.is_empty()
                    && let Some(name) = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                {
                    d.name = name.to_owned();
                }
                d.path = path;
                d.error = None;
                // Auto-advance to step 2 once a folder is chosen.
                d.step = crate::state::AddProjectStep::NameProject;
            }
            focus_ops::open_overlay_focus(
                state,
                focus::FocusTarget::text_input(
                    knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
                ),
            )
        }
        WorkspaceMessage::RemoveProjectRequested(id) => {
            let name = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            state.confirm_remove_dialog = Some(ConfirmRemoveDialog {
                project_id: id,
                project_name: name,
            });
            Task::none()
        }
        WorkspaceMessage::RemoveProjectConfirmed(id) => {
            state.confirm_remove_dialog = None;
            // Capture snapshots before removing so undo can restore exactly.
            let removed_project = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id).cloned());
            let removed_status = state
                .workspace_status
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == id).cloned());

            let paths = state.paths.clone();
            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
                persist_workspace(&paths, ws);
            }
            if let Some(ws_status) = &mut state.workspace_status {
                ws_status.projects.retain(|s| s.project_id != id);
            }
            state.fetching_projects.remove(&id);
            state.reconcile_selection_with_display();
            if state.selection.selected_ids.is_empty() {
                state.selection_mode = false;
            }

            // Store undo opportunity. Cleared by next user action or explicit dismiss.
            if let Some(project) = removed_project {
                state.recent_removal = Some(crate::state::UndoableRemoval {
                    project,
                    status_snapshot: removed_status,
                });
            }
            Task::none()
        }
        WorkspaceMessage::RemoveProjectCancelled => {
            state.confirm_remove_dialog = None;
            Task::none()
        }
        WorkspaceMessage::UndoRemoval => {
            if let Some(removal) = state.recent_removal.take() {
                let paths = state.paths.clone();
                if let Some(ws) = &mut state.workspace {
                    ws.projects.push(removal.project);
                    persist_workspace(&paths, ws);
                }
                if let Some(ws_status) = &mut state.workspace_status
                    && let Some(snap) = removal.status_snapshot
                {
                    ws_status.projects.push(snap);
                }
                state.reconcile_selection_with_display();
            }
            Task::none()
        }
        WorkspaceMessage::DismissUndoSnackbar => {
            state.recent_removal = None;
            Task::none()
        }

        // --- Multi-workspace management ---
        WorkspaceMessage::CreateWorkspaceDialogOpened => {
            state.workspace_mgr.switcher_open = false;
            state.workspace_mgr.create_dialog = Some(CreateWorkspaceDialog::default());
            focus_ops::enter_overlay_focus(state)
        }
        WorkspaceMessage::CreateWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.create_dialog {
                d.name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::CreateWorkspaceConfirmed => {
            let raw_name = state
                .workspace_mgr
                .create_dialog
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();

            let name = match validate_workspace_name(&raw_name, &state.all_workspaces, None) {
                Ok(name) => name,
                Err(err) => {
                    let msg = state.t(err.i18n_key()).to_owned();
                    if let Some(d) = &mut state.workspace_mgr.create_dialog {
                        d.error = Some(msg);
                    }
                    return Task::none();
                }
            };

            let ws = knotra_vcs::Workspace::new(name);
            if let Err(e) = save_workspace(&ws, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.save_failed"));
                if let Some(d) = &mut state.workspace_mgr.create_dialog {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.all_workspaces.push(ws);
            state.active_workspace_idx = state.all_workspaces.len().saturating_sub(1);
            state.workspace = state.all_workspaces.last().cloned();
            state.clear_selection_mode();
            state.workspace_status = None;
            state.load_phase = LoadPhase::Refreshing;
            state.is_refreshing = true;
            state.workspace_mgr.create_dialog = None;
            Task::batch([
                shared::refresh_workspace_task(state),
                focus_ops::close_overlay_focus(state),
            ])
        }
        WorkspaceMessage::CreateWorkspaceCancelled => {
            state.workspace_mgr.create_dialog = None;
            focus_ops::close_overlay_focus(state)
        }

        WorkspaceMessage::RenameWorkspaceDialogOpened => {
            state.workspace_mgr.switcher_open = false;
            let current = state
                .workspace
                .as_ref()
                .map(|ws| ws.name.clone())
                .unwrap_or_default();
            state.workspace_mgr.rename_dialog = Some(RenameWorkspaceDialog {
                new_name: current,
                error: None,
            });
            focus_ops::enter_overlay_focus(state)
        }
        WorkspaceMessage::RenameWorkspaceNameChanged(s) => {
            if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                d.new_name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::RenameWorkspaceConfirmed => {
            let raw_name = state
                .workspace_mgr
                .rename_dialog
                .as_ref()
                .map(|d| d.new_name.clone())
                .unwrap_or_default();

            let current_id = state.workspace.as_ref().map(|ws| ws.id.clone());
            let name = match validate_workspace_name(
                &raw_name,
                &state.all_workspaces,
                current_id.as_ref(),
            ) {
                Ok(name) => name,
                Err(err) => {
                    let msg = state.t(err.i18n_key()).to_owned();
                    if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                        d.error = Some(msg);
                    }
                    return Task::none();
                }
            };

            let mut renamed = match state.workspace.clone() {
                Some(ws) => ws,
                None => return Task::none(),
            };
            renamed.name = name;
            if let Err(e) = save_workspace(&renamed, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.save_failed"));
                if let Some(d) = &mut state.workspace_mgr.rename_dialog {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.workspace = Some(renamed.clone());
            if let Some(entry) = state.all_workspaces.get_mut(state.active_workspace_idx) {
                *entry = renamed;
            }
            state.workspace_mgr.rename_dialog = None;
            focus_ops::close_overlay_focus(state)
        }
        WorkspaceMessage::RenameWorkspaceCancelled => {
            state.workspace_mgr.rename_dialog = None;
            focus_ops::close_overlay_focus(state)
        }

        WorkspaceMessage::DeleteWorkspaceRequested => {
            state.workspace_mgr.switcher_open = false;
            if state.all_workspaces.len() <= 1 {
                if let Some(ws) = state.workspace.as_ref() {
                    state.workspace_mgr.confirm_delete = Some(DeleteWorkspaceDialog {
                        workspace_id: ws.id.clone(),
                        workspace_name: ws.name.clone(),
                        project_count: ws.projects.len(),
                        error: Some(state.t("workspace.delete.disabled_last").to_owned()),
                    });
                }
                return focus_ops::enter_overlay_focus(state);
            }

            if let Some(ws) = state.workspace.as_ref() {
                state.workspace_mgr.confirm_delete = Some(DeleteWorkspaceDialog {
                    workspace_id: ws.id.clone(),
                    workspace_name: ws.name.clone(),
                    project_count: ws.projects.len(),
                    error: None,
                });
            }
            focus_ops::enter_overlay_focus(state)
        }
        WorkspaceMessage::DeleteWorkspaceConfirmed => {
            if state.all_workspaces.len() <= 1 {
                let msg = state.t("workspace.delete.disabled_last").to_owned();
                if let Some(d) = &mut state.workspace_mgr.confirm_delete {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            let delete_id = state
                .workspace_mgr
                .confirm_delete
                .as_ref()
                .map(|d| d.workspace_id.clone());
            let deleted_idx = delete_id
                .as_ref()
                .and_then(|id| state.all_workspaces.iter().position(|ws| &ws.id == id))
                .unwrap_or(state.active_workspace_idx);
            let Some(deleted_ws) = state.all_workspaces.get(deleted_idx).cloned() else {
                return Task::none();
            };

            if let Err(e) = delete_workspace_file(&deleted_ws, &state.paths) {
                let msg = format!("{} {e}", state.t("workspace.error.delete_failed"));
                if let Some(d) = &mut state.workspace_mgr.confirm_delete {
                    d.error = Some(msg);
                }
                return Task::none();
            }

            state.all_workspaces.remove(deleted_idx);
            state.active_workspace_idx =
                next_active_index_after_delete(deleted_idx, state.all_workspaces.len() + 1);
            state.workspace = state
                .all_workspaces
                .get(state.active_workspace_idx)
                .cloned();
            state.clear_selection_mode();
            state.workspace_status = None;
            let active_ids: Vec<knotra_vcs::ProjectId> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
                .unwrap_or_default();
            state.fs_poller.prune(&active_ids);
            state.load_phase = LoadPhase::Refreshing;
            state.is_refreshing = true;
            state.workspace_mgr.confirm_delete = None;
            Task::batch([
                shared::refresh_workspace_task(state),
                focus_ops::close_overlay_focus(state),
            ])
        }
        WorkspaceMessage::DeleteWorkspaceCancelled => {
            state.workspace_mgr.confirm_delete = None;
            focus_ops::close_overlay_focus(state)
        }

        WorkspaceMessage::SwitcherToggled => {
            state.workspace_mgr.switcher_open = !state.workspace_mgr.switcher_open;
            Task::none()
        }
        WorkspaceMessage::WorkspaceSwitched(id) => {
            state.workspace_mgr.switcher_open = false;
            if let Some(idx) = state.all_workspaces.iter().position(|ws| ws.id == id) {
                shared::clear_sync_retry_context(state);
                state.active_workspace_idx = idx;
                state.workspace = state.all_workspaces.get(idx).cloned();
                state.clear_selection_mode();
                // Prune stale FsPoller snapshots from the previous workspace.
                let active_ids: Vec<knotra_vcs::ProjectId> = state
                    .workspace
                    .as_ref()
                    .map(|ws| ws.projects.iter().map(|p| p.id.clone()).collect())
                    .unwrap_or_default();
                state.fs_poller.prune(&active_ids);
                state.workspace_status = None;
                state.load_phase = LoadPhase::Refreshing;
                state.is_refreshing = true;
                return shared::refresh_workspace_task(state);
            }
            Task::none()
        }
    }
}
