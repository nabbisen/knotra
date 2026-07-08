//! Main navigation bar rendered below the workspace tabs.
//!
//! Shows top-level screens.  Actions that were previously on the deprecated
//! sidebar (Sync Center, Freezer, etc.) are omitted here — they are
//! accessible via the command palette or the selection bar.
//!
//! Layout (left to right):
//!   Dashboard | History | Settings        ⚙ (always-visible settings icon)

use iced::{
    widget::{button, container, row, text, Space},
    Alignment, Element, Length,
};

use snora::nav_menu::{NavItem, nav_bar};

use crate::{
    message::{Message, SettingsMessage, WorkspaceMessage},
    state::{AppState, Screen},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let items: Vec<NavItem<'_, Message>> = vec![
        NavItem {
            label:   state.t("nav.dashboard"),
            active:  state.screen == Screen::Dashboard,
            message: Message::Navigate(Screen::Dashboard),
        },
        NavItem {
            label:   state.t("nav.history"),
            active:  state.screen == Screen::History,
            message: Message::Navigate(Screen::History),
        },
        NavItem {
            label:   state.t("nav.settings"),
            active:  state.screen == Screen::Settings,
            message: Message::Navigate(Screen::Settings),
        },
    ];

    // "Add project" shortcut on the right — prominent and always reachable.
    let add_btn = button(text(state.t("dashboard.add_project")).size(13))
        .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened))
        .padding([4, 12]);

    container(
        row![
            nav_bar(items),
            Space::new().width(Length::Fill),
            add_btn,
        ]
        .align_y(Alignment::Center)
        .padding([0, 8]),
    )
    .width(Length::Fill)
    .into()
}
