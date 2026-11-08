//! Dashboard empty/error states: no workspace, no projects, a load error,
//! no filter matches. The remove-project confirmation moved to
//! `view::remove_project_dialog` (RFC-066 D6) — it is a real dialog over
//! any screen now, not a dashboard-only inline block.

use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, Tokens, centered_button_label, style};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::AppState,
};

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    placeholder(&state.theme.tokens, state.t("plain.status.checking"))
}

pub(super) fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    // RFC-073 Amendment A2: `primary` — the sole action on this screen
    // (RFC-071 D5's "one filled button per area"), unwired like every
    // other not-yet-`FocusTarget`-wired dashboard-empty-state control.
    let t = tokens.clone();
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
            .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened,))
            .style(move |_theme, status| {
                style::with_focus_ring(&t, false, style::primary(&t, status))
            }),
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
    // RFC-073 Amendment A2/D8: `secondary`, `BUTTON_HEIGHT` — matching the
    // toolbar's own `clear_filters_button` in both style and size, since
    // it is the same action.
    let t = tokens.clone();
    container(
        column![
            text(state.t("plain.empty.no_match"))
                .size(snora::design::style::text::title_size(tokens)),
            button(centered_button_label(
                text(state.t("dashboard.clear_filters"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
            ))
            .height(BUTTON_HEIGHT)
            .padding([0, 12])
            .on_press(Message::Filter(FilterMessage::AllFiltersCleared))
            .style(move |_theme, status| {
                style::with_focus_ring(&t, false, style::secondary(&t, status))
            }),
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
