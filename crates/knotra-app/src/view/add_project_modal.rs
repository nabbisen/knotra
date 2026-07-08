//! Centered "Add project" modal — replaces the old bottom-appended dialog.
//!
//! Rendered as an `iced::widget::stack` overlay when
//! `state.add_project_dialog.is_some()`.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text, text_input},
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::AppState,
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let dialog = state.add_project_dialog.as_ref()?;

    let name_input = text_input(state.t("dialog.add_project.name_hint"), &dialog.name)
        .on_input(|s| Message::Workspace(WorkspaceMessage::AddProjectNameChanged(s)))
        .padding([6, 10]);

    let path_input = text_input(state.t("dialog.add_project.path_hint"), &dialog.path)
        .on_input(|s| Message::Workspace(WorkspaceMessage::AddProjectPathChanged(s)))
        .padding([6, 10])
        .width(Length::Fill);

    let browse_btn = button(text(state.t("dialog.add_project.browse")).size(12))
        .on_press(Message::Workspace(WorkspaceMessage::BrowsePathRequested))
        .padding([6, 10]);

    let path_row = row![path_input, browse_btn]
        .spacing(6)
        .align_y(Alignment::Center);

    let confirm_btn = button(text(state.t("dialog.add_project.confirm")).size(13))
        .on_press(Message::Workspace(WorkspaceMessage::AddProjectConfirmed));

    let cancel_btn = button(text(state.t("dialog.add_project.cancel")).size(13))
        .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled));

    let mut body = column![
        row![
            text(state.t("dialog.add_project.title")).size(16),
            Space::new().width(Length::Fill),
            button(text("✕").size(12))
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
        ]
        .align_y(Alignment::Center),
        // --- Name ---
        text(state.t("dialog.add_project.name_label")).size(12),
        name_input,
        // --- Path ---
        text(state.t("dialog.add_project.path_label")).size(12),
        path_row,
    ]
    .spacing(10)
    .padding(24);

    if let Some(ref err) = dialog.error {
        body = body.push(text(err.as_str()).size(12));
    }

    body = body.push(row![confirm_btn, cancel_btn].spacing(8));

    Some(container(body).width(Length::Fixed(440.0)).into())
}
