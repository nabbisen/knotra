//! Dashboard empty/error/confirmation states: no workspace, no projects, a
//! load error, no filter matches, and the remove-project confirmation.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, danger_maybe, ghost_maybe};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::AppState,
};

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    placeholder(state.t("plain.status.checking"))
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
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.remove.title")).size(FONT_BODY + 2.0),
            text(dialog.project_name.as_str()).size(FONT_BODY),
            text(state.t("plain.remove.body")).size(FONT_SMALL),
            row![
                // Neither call here ever carried a `reason` (both passed
                // `None`), so the plain `_maybe` constructors are the
                // better target than `reasoned` — RFC-037 Stage 6 §1b.
                // `danger_maybe` for the confirm button is a primitive-fit
                // improvement, not just a restyle: this is an irreversible
                // action (removes a project), which is exactly what
                // `buttons.rs`'s `danger`/`danger_maybe` doc comment names
                // as their reason for existing, and `guided_button` had no
                // equivalent semantic distinction from the Cancel button
                // beside it.
                ghost_maybe(
                    tokens,
                    state.t("confirm.remove_no"),
                    Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
                ),
                danger_maybe(
                    tokens,
                    state.t("plain.remove.confirm"),
                    Some(Message::Workspace(
                        WorkspaceMessage::RemoveProjectConfirmed(dialog.project_id.clone(),)
                    )),
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
