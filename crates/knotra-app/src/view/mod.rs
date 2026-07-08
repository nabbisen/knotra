//! View functions for each screen.
//!
//! Each sub-module exports a single `view(state) -> Element<Message>` function.
//! Views are pure — they only read `AppState` and emit `Message`s.

pub mod changelog_view;
pub mod conflict_ops;
pub mod context_ops;
pub mod dashboard;
pub mod freezer;
pub mod history;
pub mod settings;
pub mod sync_center;

use iced::{
    widget::{button, column, container, row, text},
    Element, Length,
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
        Screen::History            => history::view(state),
        Screen::Settings           => settings::view(state),
        Screen::ConflictResolution => conflict_ops::view(state),
        Screen::Changelog          => changelog_view::view(state),
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
    use crate::message::{WorkspaceMessage, TagPushMessage};

    let nav_items: &[(Screen, &str)] = &[
        (Screen::Dashboard,          "nav.dashboard"),
        (Screen::SyncCenter,         "nav.sync"),
        (Screen::ContextOps,         "nav.context"),
        (Screen::Freezer,            "nav.freezer"),
        (Screen::History,            "nav.history"),
        (Screen::Settings,           "nav.settings"),
        (Screen::ConflictResolution, "nav.conflicts"),
        (Screen::Changelog,          "nav.changelog"),
    ];

    let ws_name = state.workspace.as_ref()
        .map(|w| w.name.as_str())
        .unwrap_or("(none)");

    let mut nav = column![].spacing(2).padding([8, 0]);

    // App header + workspace name.
    nav = nav.push(
        container(
            column![
                text("knotra").size(16),
                text(ws_name).size(11),
            ].spacing(1)
        )
        .padding([10, 12])
        .width(Length::Fill),
    );

    // Workspace management row.
    nav = nav.push(
        row![
            button(text("+ WS").size(10))
                .on_press(Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened)),
            button(text("✎").size(10))
                .on_press(Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened)),
        ]
        .spacing(2)
        .padding([0, 8])
    );

    // Workspace switcher (show only if more than one workspace exists).
    if state.all_workspaces.len() > 1 {
        for (idx, ws) in state.all_workspaces.iter().enumerate() {
            let is_active = idx == state.active_workspace_idx;
            let label = if is_active {
                format!("▶ {}", ws.name)
            } else {
                format!("  {}", ws.name)
            };
            let ws_id = ws.id.clone();
            nav = nav.push(
                button(text(label).size(11))
                    .width(Length::Fill)
                    .on_press_maybe(if is_active { None } else {
                        Some(Message::Workspace(WorkspaceMessage::WorkspaceSwitched(ws_id)))
                    }),
            );
        }
    }

    // Main nav items.
    for (screen, key) in nav_items {
        let label = state.t(key);
        let btn = button(text(label))
            .width(Length::Fill)
            .on_press(Message::Navigate(screen.clone()));
        nav = nav.push(btn);
    }

    // Tag-push banner (if pending).
    if let Some(ref push) = state.pending_tag_push {
        let label = if push.is_pushing { "Pushing…" } else { "Push tags?" };
        nav = nav.push(
            container(
                column![
                    text(format!("Freeze: {}", push.freeze_name)).size(10),
                    row![
                        button(text(label).size(9))
                            .on_press_maybe(if push.is_pushing { None }
                                else { Some(Message::TagPush(TagPushMessage::PushConfirmed)) }),
                        button(text("✕").size(9))
                            .on_press(Message::TagPush(TagPushMessage::PushDeclined)),
                    ].spacing(2),
                ]
                .spacing(2)
            )
            .padding([4, 8])
            .width(Length::Fill),
        );
    }

    container(nav)
        .width(SIDEBAR_WIDTH)
        .height(Length::Fill)
        .into()
}
