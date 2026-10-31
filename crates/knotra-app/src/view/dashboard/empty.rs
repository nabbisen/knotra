//! Dashboard empty/error states: no workspace, no projects, a load error,
//! no filter matches. The remove-project confirmation moved to
//! `view::remove_project_dialog` (RFC-066 D6) — it is a real dialog over
//! any screen now, not a dashboard-only inline block.

use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, Tokens, centered_button_label};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::AppState,
};

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    placeholder(&state.theme.tokens, state.t("plain.status.checking"))
}

pub(super) fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.empty.welcome_title"))
                .size(snora::design::style::text::heading_size(tokens)),
            text(state.t("plain.empty.welcome_body"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            button(centered_button_label(text(
                state.t("plain.empty.add_first")
            )))
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

pub(super) fn no_matches(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.empty.no_match"))
                .size(snora::design::style::text::title_size(tokens)),
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

fn placeholder<'a>(tokens: &Tokens, message: &'a str) -> Element<'a, Message> {
    container(
        text(message)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(250.0))
    .center(Length::Fill)
    .into()
}
