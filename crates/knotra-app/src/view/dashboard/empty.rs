//! Dashboard empty/error/confirmation states: no workspace, no projects, a
//! load error, no filter matches, and the remove-project confirmation.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button};

use crate::{
    message::{DashboardMessage, FilterMessage, Message, WorkspaceMessage},
    state::{AppState, LoadPhase},
};

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    match &state.load_phase {
        LoadPhase::Error(error) => column![
            view_error_notice(state, error, state.t("dashboard.no_workspace_error"), false,),
            button(text(state.t("dashboard.create_workspace"))).on_press(Message::Workspace(
                WorkspaceMessage::CreateWorkspaceDialogOpened,
            )),
        ]
        .spacing(10)
        .padding(24)
        .into(),
        _ => placeholder(state.t("plain.status.checking")),
    }
}

pub(super) fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text(state.t("plain.empty.welcome_title")).size(FONT_BODY + 6.0),
            text(state.t("plain.empty.welcome_body")).size(FONT_BODY),
            button(text(state.t("plain.empty.add_first")))
                .height(BUTTON_HEIGHT)
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened,)),
        ]
        .spacing(14)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(260.0))
    .center(Length::Fill)
    .into()
}

pub(super) fn view_error_notice<'a>(
    state: &'a AppState,
    error: &'a str,
    first_level_message: &'a str,
    retry_allowed: bool,
) -> Element<'a, Message> {
    let details_label = if state.dashboard_error_details_open {
        state.t("plain.hide_details")
    } else {
        state.t("plain.show_details")
    };
    let mut actions = row![
        button(text(details_label).size(12))
            .on_press(Message::Dashboard(DashboardMessage::ErrorDetailsToggled,)),
    ]
    .spacing(6);
    if retry_allowed {
        actions = actions.push(
            button(text(state.t("dashboard.try_again")).size(12))
                .on_press(Message::Dashboard(DashboardMessage::ErrorRetryRequested)),
        );
    }
    let mut notice = column![text(first_level_message).size(14), actions].spacing(6);
    if state.dashboard_error_details_open {
        notice = notice.push(text(error).size(11));
    }
    container(notice).width(Length::Fill).padding(12).into()
}

pub(super) fn no_matches(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text(state.t("plain.empty.no_match")).size(FONT_BODY + 2.0),
            button(text(state.t("dashboard.clear_filters")))
                .on_press(Message::Filter(FilterMessage::AllFiltersCleared)),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(220.0))
    .center(Length::Fill)
    .into()
}

pub(super) fn view_confirm_remove_dialog(state: &AppState) -> Element<'_, Message> {
    let Some(dialog) = &state.confirm_remove_dialog else {
        return Space::new().into();
    };
    container(
        column![
            text(state.t("plain.remove.title")).size(FONT_BODY + 2.0),
            text(dialog.project_name.as_str()).size(FONT_BODY),
            text(state.t("plain.remove.body")).size(FONT_SMALL),
            row![
                guided_button(
                    state.t("confirm.remove_no"),
                    Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
                    None,
                ),
                guided_button(
                    state.t("plain.remove.confirm"),
                    Some(Message::Workspace(
                        WorkspaceMessage::RemoveProjectConfirmed(dialog.project_id.clone(),)
                    )),
                    None,
                ),
            ]
            .spacing(12),
        ]
        .spacing(14)
        .padding(24),
    )
    .width(Length::Fixed(380.0))
    .into()
}

fn placeholder(message: &str) -> Element<'_, Message> {
    container(text(message).size(14))
        .width(Length::Fill)
        .height(Length::Fixed(250.0))
        .center(Length::Fill)
        .into()
}
