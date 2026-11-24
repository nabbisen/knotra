//! Dashboard empty/error states: no workspace, no projects, a load error,
//! no filter matches. The remove-project confirmation moved to
//! `view::remove_project_dialog` (RFC-066 D6) — it is a real dialog over
//! any screen now, not a dashboard-only inline block.

use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{Tokens, primary_control, secondary_control};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::AppState,
    view::ids,
};

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    placeholder(&state.theme.tokens, state.t("plain.status.checking"))
}

pub(super) fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    // RFC-073 Amendment A2: `primary` — the sole action on this screen
    // (RFC-071 D5's "one filled button per area"). RFC-075 stage 2 §3
    // (Review 298 inventory §2d): now reachable by Tab, where it was not
    // before.
    container(
        column![
            text(state.t("plain.empty.welcome_title"))
                .size(snora::design::style::text::heading_size(tokens)),
            text(state.t("plain.empty.welcome_body"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            primary_control(
                tokens,
                state.t("plain.empty.add_first"),
                Message::Workspace(WorkspaceMessage::AddProjectDialogOpened),
                ids::empty::add_first_project(),
            ),
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
    // RFC-073 Amendment A2/D8: matching the toolbar's own
    // `clear_filters_button` in both style and size, since it is the same
    // action. RFC-075 stage 2 §3 (Review 298 inventory §2d): now reachable
    // by Tab, with its own id distinct from the toolbar's own Clear
    // filters (`view::ids::empty::clear_filters`).
    container(
        column![
            text(state.t("plain.empty.no_match"))
                .size(snora::design::style::text::title_size(tokens)),
            secondary_control(
                tokens,
                state.t("dashboard.clear_filters"),
                Message::Filter(FilterMessage::AllFiltersCleared),
                ids::empty::clear_filters(),
            ),
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
