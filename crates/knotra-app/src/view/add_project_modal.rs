//! RFC-0021 Phase 5 — Guided 2-step "Add project folder" dialog.
//!
//! Step 1: Choose the folder that contains your project.
//! Step 2: Give it a display name.
//!
//! The Browse button auto-advances to Step 2 when a folder is selected.
//! A typed path also advances on "Next" after validation.

use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Element, Length,
};

use knotra_ui::widget::{guided_button, guided_field_focused, BUTTON_HEIGHT, FONT_BODY, FONT_SMALL};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{AddProjectStep, AppState},
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let dialog = state.add_project_dialog.as_ref()?;

    let (step_label, _total) = match dialog.step {
        AddProjectStep::ChooseFolder  => (state.t("plain.add_project.step1_of2"), "1"),
        AddProjectStep::NameProject   => (state.t("plain.add_project.step2_of2"), "2"),
    };

    let close_btn = button(text("✕").size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 12])
        .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled));

    let header = row![
        column![
            text(state.t("plain.add_project.title")).size(FONT_BODY + 2.0),
            text(step_label).size(FONT_SMALL),
        ]
        .spacing(2),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = match dialog.step {
        // ------------------------------------------------------------------
        // Step 1 — Choose the folder
        // ------------------------------------------------------------------
        AddProjectStep::ChooseFolder => {
            let path_field = guided_field_focused(
                state.t("plain.add_project.folder_label"),
                state.t("plain.add_project.folder_hint"),
                &dialog.path,
                |s| Message::Workspace(WorkspaceMessage::AddProjectPathChanged(s)),
                dialog.error.as_deref(),
                knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone(),
            );

            let browse_btn = button(text(state.t("plain.add_project.browse")).size(FONT_BODY))
                .height(BUTTON_HEIGHT)
                .padding([0, 18])
                .on_press(Message::Workspace(WorkspaceMessage::BrowsePathRequested));

            let next_reason: Option<&str> = if dialog.path.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_folder"))
            } else {
                None
            };

            let footer = row![
                guided_button(
                    state.t("plain.add_project.next"),
                    (!dialog.path.trim().is_empty())
                        .then_some(Message::Workspace(WorkspaceMessage::AddProjectNextStep)),
                    next_reason,
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("action.cancel")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.add_project.step1_instruction")).size(FONT_BODY),
                path_field,
                browse_btn,
                footer,
            ]
            .spacing(14)
            .into()
        }

        // ------------------------------------------------------------------
        // Step 2 — Name the project
        // ------------------------------------------------------------------
        AddProjectStep::NameProject => {
            // Show the chosen folder as a read-only confirmation.
            let folder_display = container(
                column![
                    text(state.t("plain.add_project.folder_chosen")).size(FONT_SMALL),
                    text(&dialog.path).size(FONT_BODY),
                ]
                .spacing(4),
            )
            .padding([10, 14]);

            let name_field = guided_field_focused(
                state.t("plain.add_project.name_label"),
                state.t("dialog.add_project.name_hint"),
                &dialog.name,
                |s| Message::Workspace(WorkspaceMessage::AddProjectNameChanged(s)),
                dialog.error.as_deref(),
                knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
            );

            let add_reason: Option<&str> = if dialog.name.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_name"))
            } else {
                None
            };

            let footer = row![
                guided_button(
                    state.t("plain.add_project.add"),
                    (!dialog.name.trim().is_empty())
                        .then_some(Message::Workspace(WorkspaceMessage::AddProjectConfirmed)),
                    add_reason,
                ),
                Space::new().width(Length::Fill),
                button(text(state.t("plain.add_project.back")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 18])
                    .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.add_project.step2_instruction")).size(FONT_BODY),
                folder_display,
                name_field,
                footer,
            ]
            .spacing(14)
            .into()
        }
    };

    Some(
        container(
            column![header, body]
                .spacing(16)
                .padding(24),
        )
        .width(Length::Fixed(480.0))
        .into(),
    )
}
