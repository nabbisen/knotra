//! The freezer (release/tag) domain: `handle_freezer` and its
//! `start_freeze_execution` continuation (RFC-040 Stage 3 commit 3).

use iced::Task;
use knotra_vcs::VcsAdapter;

use super::focus_ops;
use super::shared;
use crate::{
    message::{BackgroundMessage, FreezerMessage, Message},
    state::{AppState, OperationOwner, focus, freezer::FreezerPhase},
};

pub(super) fn handle_freezer(state: &mut AppState, msg: FreezerMessage) -> Task<Message> {
    #[allow(unreachable_patterns)]
    match msg {
        FreezerMessage::OpenRequested => {
            shared::cancel_freezer_validation(state);
            // Reinitialise project selection from workspace.
            if let Some(ws) = &state.workspace {
                let ids: Vec<_> = ws.projects.iter().map(|p| p.id.clone()).collect();
                state.freezer.init_selection(&ids);
            }
            state.pending_tag_push = None;
            state.freezer.execution_started_at = None;
            state.freezer.phase = FreezerPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Tag;
            Task::none()
        }

        FreezerMessage::NameChanged(name) => {
            shared::cancel_freezer_validation(state);
            state.freezer.freeze_name = name;
            // Reset to Idle when the name changes after validation.
            if matches!(state.freezer.phase, FreezerPhase::ValidationReady(_)) {
                state.freezer.phase = FreezerPhase::Idle;
            }
            Task::none()
        }

        FreezerMessage::TagMessageChanged(s) => {
            state.freezer.tag_message = s;
            Task::none()
        }
        FreezerMessage::ExecuteConfirmed => start_freeze_execution(state),
        FreezerMessage::BulkOpenRequested => {
            shared::cancel_freezer_validation(state);
            state.active_modal = crate::state::ActiveModal::Tag;
            state.freezer.phase = FreezerPhase::Idle;
            state.freezer.execution_started_at = None;
            state.pending_tag_push = None;
            // Pre-populate freeze selection
            state.freezer.project_selection = state
                .selection
                .selected_ids
                .iter()
                .map(|id| (id.clone(), true))
                .collect();
            focus_ops::open_overlay_focus(
                state,
                focus::FocusTarget::text_input(knotra_ui::widget::focus_id::RELEASE_NAME.clone()),
            )
        }
        FreezerMessage::BulkModalClosed => {
            if focus_ops::freezer_is_running(state) {
                return Task::none();
            }
            shared::cancel_freezer_validation(state);
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }

        FreezerMessage::ValidateRequested => {
            if !state.freezer.freeze_name_is_valid() {
                return Task::none(); // view blocks the button; defensive guard
            }

            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.clone())
                .unwrap_or_default();
            let selection = state.freezer.selected_ids();
            let freeze_name = state.freezer.freeze_name.clone();
            let max = state.config.max_concurrent_reads;
            let Some(lease_id) = shared::acquire_operation(state, OperationOwner::FreezeValidation)
            else {
                return Task::none();
            };

            state.freezer.phase = FreezerPhase::Validating { lease_id };
            state.freezer.execution_started_at = None;

            Task::perform(
                async move {
                    VcsAdapter::validate_freeze(&projects, &selection, &freeze_name, max).await
                },
                move |validation| {
                    Message::Background(BackgroundMessage::FreezeValidationDone {
                        lease_id,
                        validation,
                    })
                },
            )
        }
    }
}

fn start_freeze_execution(state: &mut AppState) -> Task<Message> {
    let validation = match &state.freezer.phase {
        FreezerPhase::ValidationReady(validation)
            if validation.all_ready() && validation.ready_count() > 0 =>
        {
            validation.clone()
        }
        _ => return Task::none(),
    };

    let projects: Vec<_> = state
        .workspace
        .as_ref()
        .map(|ws| ws.projects.clone())
        .unwrap_or_default();
    let tag_message = state.freezer.tag_message.trim().to_owned();
    let tag_message = (!tag_message.is_empty()).then_some(tag_message);
    let Some(lease_id) = shared::acquire_operation(state, OperationOwner::FreezeExecution) else {
        return Task::none();
    };

    state.freezer.execution_started_at = Some(chrono::Utc::now());
    state.freezer.phase = FreezerPhase::Executing;
    state.pending_tag_push = None;

    Task::perform(
        async move {
            VcsAdapter::execute_freeze_with_message(&projects, &validation, tag_message.as_deref())
                .await
        },
        move |result| {
            Message::Background(BackgroundMessage::FreezeExecutionDone { lease_id, result })
        },
    )
}
