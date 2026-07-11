//! Workspace management dialogs.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button, guided_field_focused,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{AppState, workspace_mgr},
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    if let Some(dialog) = &state.workspace_mgr.create_dialog {
        return Some(workspace_name_dialog(state, NameDialog::Create(dialog)));
    }

    if let Some(dialog) = &state.workspace_mgr.rename_dialog {
        return Some(workspace_name_dialog(state, NameDialog::Rename(dialog)));
    }

    if let Some(dialog) = &state.workspace_mgr.confirm_delete {
        return Some(delete_dialog(state, dialog));
    }

    None
}

#[derive(Clone, Copy)]
enum NameDialog<'a> {
    Create(&'a workspace_mgr::CreateWorkspaceDialog),
    Rename(&'a workspace_mgr::RenameWorkspaceDialog),
}

fn workspace_name_dialog<'a>(state: &'a AppState, dialog: NameDialog<'a>) -> Element<'a, Message> {
    let (title, confirm_label, value, error, confirm, cancel) = match dialog {
        NameDialog::Create(dialog) => (
            state.t("workspace.create.title"),
            state.t("workspace.create.confirm"),
            dialog.name.as_str(),
            dialog.error.as_deref(),
            WorkspaceMessage::CreateWorkspaceConfirmed,
            WorkspaceMessage::CreateWorkspaceCancelled,
        ),
        NameDialog::Rename(dialog) => (
            state.t("workspace.rename.title"),
            state.t("workspace.rename.confirm"),
            dialog.new_name.as_str(),
            dialog.error.as_deref(),
            WorkspaceMessage::RenameWorkspaceConfirmed,
            WorkspaceMessage::RenameWorkspaceCancelled,
        ),
    };

    let close_btn = button(text("x").size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 12])
        .on_press(Message::Workspace(cancel.clone()));

    let field = match dialog {
        NameDialog::Create(_) => guided_field_focused(
            state.t("workspace.name_label"),
            state.t("workspace.name_hint"),
            value,
            |s| Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(s)),
            error,
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
        ),
        NameDialog::Rename(_) => guided_field_focused(
            state.t("workspace.name_label"),
            state.t("workspace.name_hint"),
            value,
            |s| Message::Workspace(WorkspaceMessage::RenameWorkspaceNameChanged(s)),
            error,
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
        ),
    };

    let reason = if value.trim().is_empty() {
        Some(state.t("workspace.error.empty_name"))
    } else {
        None
    };

    let footer = row![
        guided_button(
            confirm_label,
            (!value.trim().is_empty()).then_some(Message::Workspace(confirm)),
            reason,
        ),
        Space::new().width(Length::Fill),
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(cancel)),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(title).size(FONT_BODY + 2.0),
                Space::new().width(Length::Fill),
                close_btn,
            ]
            .align_y(Alignment::Center),
            field,
            footer,
        ]
        .spacing(16)
        .padding(24),
    )
    .width(Length::Fixed(460.0))
    .into()
}

fn delete_dialog<'a>(
    state: &'a AppState,
    dialog: &'a crate::state::workspace_mgr::DeleteWorkspaceDialog,
) -> Element<'a, Message> {
    let body = format!(
        "{} \"{}\". {}",
        state.t("workspace.delete.body_prefix"),
        dialog.workspace_name,
        state.t("workspace.delete.body_suffix"),
    );
    let project_count = format!(
        "{} {}",
        dialog.project_count,
        state.t("workspace.delete.project_count_suffix"),
    );

    let can_delete = state.all_workspaces.len() > 1 && dialog.error.is_none();
    let reason = if state.all_workspaces.len() <= 1 {
        Some(state.t("workspace.delete.disabled_last"))
    } else {
        dialog.error.as_deref()
    };

    let footer = row![
        guided_button(
            state.t("workspace.delete.confirm"),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceConfirmed
            )),
            reason,
        ),
        Space::new().width(Length::Fill),
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled
            )),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(state.t("workspace.delete.title")).size(FONT_BODY + 2.0),
                Space::new().width(Length::Fill),
                button(text("x").size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 12])
                    .on_press(Message::Workspace(
                        WorkspaceMessage::DeleteWorkspaceCancelled
                    )),
            ]
            .align_y(Alignment::Center),
            text(body).size(FONT_BODY),
            text(project_count).size(FONT_SMALL),
            footer,
        ]
        .spacing(16)
        .padding(24),
    )
    .width(Length::Fixed(500.0))
    .into()
}
