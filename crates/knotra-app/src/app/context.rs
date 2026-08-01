//! The context (VCS bookmark/branch/context) switching domain: `handle_context`
//! and the helpers used only by it (RFC-040 Stage 3 commit 5).

use iced::Task;
use knotra_vcs::{
    VcsAdapter,
    model::{
        operation::{ContextSwitchResult, ProjectOperationOutcome, ProjectOperationResult},
        project::Project,
    },
};

use super::shared;
use crate::{
    message::{BackgroundMessage, ContextMessage, Message},
    state::{AppState, OperationOwner, Screen, context::ContextPhase},
};

fn context_switch_disabled_reason(
    status: Option<&knotra_vcs::ProjectStatus>,
) -> Option<&'static str> {
    let status = status?;
    if status.read_error.is_some() {
        Some("plain.switch.reason_unavailable")
    } else if status.conflict.has_conflict {
        Some("plain.switch.reason_conflict")
    } else if status.working_tree.is_dirty() {
        Some("plain.switch.reason_dirty")
    } else {
        None
    }
}

fn blocked_context_switch_result(project: &Project, reason: String) -> ProjectOperationResult {
    ProjectOperationResult {
        project_id: project.id.clone(),
        outcome: ProjectOperationOutcome::Failed,
        success: false,
        skip_reason: None,
        commands_executed: vec![],
        stdout: String::new(),
        stderr: reason.clone(),
        exit_code: Some(1),
        error_message: Some(reason),
    }
}

pub(super) fn handle_context(state: &mut AppState, msg: ContextMessage) -> Task<Message> {
    match msg {
        ContextMessage::OpenRequested(preselect_id) => {
            state.active_modal = crate::state::ActiveModal::Switch;
            state.context_ops.phase = ContextPhase::Idle;

            // If a project was pre-selected (e.g. from a dashboard card shortcut), load it.
            if let Some(id) = preselect_id
                && let Some(project) = shared::find_project(state, &id)
            {
                state.context_ops.phase = ContextPhase::LoadingList(id.clone());
                return Task::perform(
                    async move { VcsAdapter::list_contexts(&project).await },
                    |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
                );
            }
            Task::none()
        }

        ContextMessage::ProjectSelected(id) => {
            let project = match shared::find_project(state, &id) {
                Some(p) => p,
                None => return Task::none(),
            };

            // Use cached list if present, otherwise fetch.
            if let Some(cached) = state.context_ops.cached_lists.get(&id).cloned() {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list: cached,
                    search: String::new(),
                };
                return Task::none();
            }

            state.context_ops.phase = ContextPhase::LoadingList(id.clone());
            Task::perform(
                async move { VcsAdapter::list_contexts(&project).await },
                |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
            )
        }

        ContextMessage::SearchChanged(s) => {
            if let ContextPhase::BrowsingList { search, .. } = &mut state.context_ops.phase {
                *search = s;
            }
            Task::none()
        }

        ContextMessage::SwitchTargetChosen(project_id, target, target_label) => {
            let project = match shared::find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let status = state
                .workspace_status
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id));
            let vcs_kind = status
                .map(|s| s.identity.vcs_kind)
                .unwrap_or(knotra_vcs::VcsKind::Git);

            let is_dirty = status.map(|s| s.working_tree.is_dirty()).unwrap_or(false);
            let disabled_reason_key = context_switch_disabled_reason(status);

            state.context_ops.phase = ContextPhase::ConfirmSwitch {
                project_id,
                project_name: project.name.clone(),
                target,
                target_label,
                vcs_kind,
                is_dirty,
                disabled_reason_key,
            };
            Task::none()
        }

        ContextMessage::SwitchConfirmed => {
            let (project_id, target, target_label, project_name, disabled_reason_key) =
                match &state.context_ops.phase {
                    ContextPhase::ConfirmSwitch {
                        project_id,
                        target,
                        target_label,
                        project_name,
                        disabled_reason_key,
                        ..
                    } => (
                        project_id.clone(),
                        target.clone(),
                        target_label.clone(),
                        project_name.clone(),
                        *disabled_reason_key,
                    ),
                    _ => return Task::none(),
                };
            if disabled_reason_key.is_some() {
                return Task::none();
            }

            let project = match shared::find_project(state, &project_id) {
                Some(p) => p,
                None => return Task::none(),
            };
            let Some(lease_id) = shared::acquire_operation(state, OperationOwner::ContextSwitch)
            else {
                return Task::none();
            };

            state.context_ops.phase = ContextPhase::Switching {
                project_id: project_id.clone(),
                target: target.clone(),
                target_label: target_label.clone(),
            };
            // Invalidate cached list for this project.
            state.context_ops.cached_lists.remove(&project_id);

            let unavailable_reason = state.t("plain.switch.reason_unavailable").to_owned();
            let conflict_reason = state.t("plain.switch.reason_conflict").to_owned();
            let dirty_reason = state.t("plain.switch.reason_dirty").to_owned();

            Task::perform(
                async move {
                    let latest_status = VcsAdapter::read_project_status(&project).await;
                    let blocked_reason =
                        context_switch_disabled_reason(Some(&latest_status)).map(|key| match key {
                            "plain.switch.reason_unavailable" => unavailable_reason.clone(),
                            "plain.switch.reason_conflict" => conflict_reason.clone(),
                            "plain.switch.reason_dirty" => dirty_reason.clone(),
                            _ => key.to_owned(),
                        });
                    let (result, hint) = if let Some(reason) = blocked_reason {
                        (blocked_context_switch_result(&project, reason), None)
                    } else {
                        VcsAdapter::switch_context(&project, &target).await
                    };
                    ContextSwitchResult {
                        project_id: project.id,
                        project_name,
                        target: target_label,
                        operation_result: result,
                        recovery_hint: hint,
                    }
                },
                move |result| {
                    Message::Background(BackgroundMessage::ContextSwitchDone { lease_id, result })
                },
            )
        }

        ContextMessage::SwitchCancelled => {
            // Return to browsing.
            let prev_id = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, .. } => Some(project_id.clone()),
                _ => None,
            };
            if let Some(id) = prev_id
                && let Some(cached) = state.context_ops.cached_lists.get(&id).cloned()
            {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list: cached,
                    search: String::new(),
                };
                return Task::none();
            }
            state.context_ops.phase = ContextPhase::Idle;
            Task::none()
        }

        ContextMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
        ContextMessage::BulkOpenRequested => {
            let selected = state.selection_summary().selected_ids;
            let Some(project_id) = selected.first().cloned().filter(|_| selected.len() == 1) else {
                return Task::none();
            };
            let Some(project) = shared::find_project(state, &project_id) else {
                return Task::none();
            };
            state.active_modal = crate::state::ActiveModal::Switch;
            state.context_ops.phase = ContextPhase::LoadingList(project_id);
            Task::perform(
                async move { VcsAdapter::list_contexts(&project).await },
                |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
            )
        }
        ContextMessage::BulkModalClosed => {
            if matches!(state.context_ops.phase, ContextPhase::Switching { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
        ContextMessage::Cancelled => {
            if matches!(state.context_ops.phase, ContextPhase::Switching { .. }) {
                return Task::none();
            }
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}
