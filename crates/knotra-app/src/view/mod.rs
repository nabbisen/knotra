//! View functions for each screen.
//!
//! Each sub-module exports a single `view(state) -> Element<Message>` function.
//! Views are pure — they only read `AppState` and emit `Message`s.

pub mod context_ops;
pub mod dashboard;
pub mod freezer;
pub mod history;
pub mod settings;
pub mod sync_center;

use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Element, Length,
};

use crate::{message::Message, state::{AppState, Screen}};
use snora::widget::SIDEBAR_WIDTH;

/// Render the full application layout: sidebar + main content area.
pub fn app_view(state: &AppState) -> Element<'_, Message> {
    let sidebar = view_sidebar(state);
    let content: Element<'_, Message> = match &state.screen {
        Screen::Dashboard => dashboard::view(state),
        Screen::SyncCenter => sync_center::view(state),
        Screen::ContextOps => context_ops::view(state),
        Screen::Freezer => freezer::view(state),
        Screen::History => history::view(state),
        Screen::Settings => settings::view(state),
    };

    let layout = row![
        sidebar,
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
    ]
    .height(Length::Fill);

    let mut outer = column![layout].height(Length::Fill);

    if let Some(ref msg) = state.status_bar {
        outer = outer.push(
            container(text(msg.as_str()).size(12))
                .width(Length::Fill)
                .padding([2, 8]),
        );
    }

    outer.into()
}

fn view_sidebar(state: &AppState) -> Element<'_, Message> {
    let nav_items: &[(Screen, &str)] = &[
        (Screen::Dashboard, "nav.dashboard"),
        (Screen::SyncCenter, "nav.sync"),
        (Screen::ContextOps, "nav.context"),
        (Screen::Freezer, "nav.freezer"),
        (Screen::History, "nav.history"),
        (Screen::Settings, "nav.settings"),
    ];

    let mut nav = column![]
        .spacing(2)
        .padding([8, 0]);

    nav = nav.push(
        container(text("knotra").size(18))
            .padding([12, 16])
            .width(Length::Fill),
    );

    for (screen, key) in nav_items {
        let label = state.t(key);
        let btn = button(text(label))
            .width(Length::Fill)
            .on_press(Message::Navigate(screen.clone()));
        nav = nav.push(btn);
    }

    container(nav)
        .width(SIDEBAR_WIDTH)
        .height(Length::Fill)
        .into()
}
