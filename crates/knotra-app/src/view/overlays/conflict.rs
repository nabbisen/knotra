//! 4. Conflict resolve panel (right-docked sheet) — RFC-037 Stage 1.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button};
use knotra_vcs::{ProjectId, VcsKind};

use super::project_name_for;
use crate::{
    message::{ConflictOpsMessage, Message},
    state::AppState,
};

pub fn resolve_panel<'a>(state: &'a AppState, project_id: &'a ProjectId) -> Element<'a, Message> {
    let name = project_name_for(state, project_id);
    let ops = &state.conflict_ops;
    let git_actions_supported = conflict_actions_supported_for_project(state, project_id);
    let abort_supported = git_actions_supported && project_has_git_merge_state(state, project_id);
    let editor_configured = state.config.external_editor.is_some();

    let content: Element<'_, Message> = match &ops.phase {
        crate::state::conflict_ops::ConflictPhase::Loading(id) if id == project_id => {
            text(state.t("plain.resolve.loading"))
                .size(FONT_BODY)
                .into()
        }
        crate::state::conflict_ops::ConflictPhase::Operating {
            project_id: id,
            action,
        } if id == project_id => column![
            text(action).size(FONT_BODY),
            text(state.t("plain.resolve.working_hint")).size(FONT_SMALL),
        ]
        .spacing(8)
        .into(),
        crate::state::conflict_ops::ConflictPhase::Done {
            project_id: id,
            success,
            message,
            result,
        } if id == project_id => {
            let title = if *success {
                state.t("plain.resolve.done")
            } else {
                state.t("plain.resolve.failed")
            };
            let details_label = if state.show_op_details {
                state.t("plain.hide_details")
            } else {
                state.t("plain.show_details")
            };
            let mut result_col = column![
                text(title).size(FONT_BODY + 2.0),
                text(message).size(FONT_BODY),
            ]
            .spacing(8)
            .push(
                button(text(details_label).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::ToggleOpDetails),
            );

            if state.show_op_details
                && let Some(result) = result
            {
                for command in &result.commands_executed {
                    result_col =
                        result_col.push(text(format!("command: {command}")).size(FONT_SMALL));
                }
                if !result.stdout.is_empty() {
                    result_col = result_col
                        .push(text(format!("stdout: {}", result.stdout)).size(FONT_SMALL));
                }
                if !result.stderr.is_empty() {
                    result_col = result_col
                        .push(text(format!("stderr: {}", result.stderr)).size(FONT_SMALL));
                }
                if let Some(error) = &result.error_message {
                    result_col = result_col.push(text(format!("error: {error}")).size(FONT_SMALL));
                }
            }

            result_col.into()
        }
        _ => {
            let detail = match &ops.phase {
                crate::state::conflict_ops::ConflictPhase::Browsing {
                    project_id: id,
                    detail,
                } if id == project_id => Some(detail),
                _ => ops.cached.get(project_id),
            };

            if let Some(detail) = detail {
                if detail.conflicted_files.is_empty() {
                    text(state.t("plain.resolve.no_files"))
                        .size(FONT_BODY)
                        .into()
                } else {
                    let file_rows: Vec<Element<'_, Message>> = detail
                        .conflicted_files
                        .iter()
                        .map(|f| {
                            let editor_reason = (!editor_configured)
                                .then_some(state.t("plain.resolve.editor_not_configured"));
                            let open_editor_msg =
                                editor_configured.then_some(Message::ConflictOps(
                                    ConflictOpsMessage::OpenInEditorRequested(f.path.clone()),
                                ));

                            let mark_control: Element<'_, Message> = if git_actions_supported {
                                button(
                                    text(state.t("plain.resolve.mark_done")).size(FONT_SMALL + 1.0),
                                )
                                .height(36.0)
                                .padding([0, 10])
                                .on_press(Message::ConflictOps(
                                    ConflictOpsMessage::MarkResolvedRequested {
                                        project_id: project_id.clone(),
                                        file_path: f.path.clone(),
                                    },
                                ))
                                .into()
                            } else {
                                text(state.t("plain.resolve.unsupported"))
                                    .size(FONT_SMALL)
                                    .into()
                            };

                            column![
                                row![
                                    text("!").size(FONT_BODY).width(Length::Fixed(22.0)),
                                    text(&f.path).size(FONT_BODY).width(Length::Fill),
                                    Space::new().width(Length::Fixed(8.0)),
                                    guided_button(
                                        state.t("plain.resolve.open_editor"),
                                        open_editor_msg,
                                        editor_reason,
                                    ),
                                    mark_control,
                                ]
                                .align_y(Alignment::Center)
                                .spacing(6),
                            ]
                            .spacing(4)
                            .into()
                        })
                        .collect();
                    scrollable(column(file_rows).spacing(8))
                        .height(Length::Fill)
                        .into()
                }
            } else {
                text(state.t("plain.resolve.loading"))
                    .size(FONT_BODY)
                    .into()
            }
        }
    };

    let stop_control: Element<'_, Message> = if abort_supported {
        button(text(state.t("plain.resolve.stop_attempt")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::ConflictOps(
                ConflictOpsMessage::AbortMergeRequested(project_id.clone()),
            ))
            .into()
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    let close_msg = (!matches!(
        ops.phase,
        crate::state::conflict_ops::ConflictPhase::Operating { .. }
    ))
    .then_some(Message::ConflictOps(ConflictOpsMessage::PanelClosed));

    let footer = row![
        stop_control,
        Space::new().width(Length::Fill),
        button(text(state.t("action.close")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press_maybe(close_msg.clone()),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(format!("{} — {}", state.t("plain.resolve.title"), name))
                    .size(FONT_BODY + 2.0),
                Space::new().width(Length::Fill),
                button(text("✕").size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 12])
                    .on_press_maybe(close_msg),
            ]
            .align_y(Alignment::Center),
            text(state.t("plain.resolve.instruction")).size(FONT_BODY),
            content,
            footer,
        ]
        .spacing(14)
        .padding(20),
    )
    .width(Length::Fixed(340.0))
    .height(Length::Fill)
    .into()
}

fn conflict_actions_supported_for_project(state: &AppState, project_id: &ProjectId) -> bool {
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
            state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|project| &project.id == project_id))
                .map(|project| {
                    let path = std::path::Path::new(&project.path);
                    !path.join(".jj").is_dir() && path.join(".git").exists()
                })
                .unwrap_or(false)
        })
}

fn project_has_git_merge_state(state: &AppState, project_id: &ProjectId) -> bool {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|project| &project.id == project_id))
        .map(|project| {
            let path = std::path::Path::new(&project.path);
            path.join(".git").join("MERGE_HEAD").exists()
        })
        .unwrap_or(false)
}
