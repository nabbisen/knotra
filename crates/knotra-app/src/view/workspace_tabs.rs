//! RFC-015 — Workspace tab strip at the top of the window.
//! RFC-019 — Migrated to snora `app_tab_bar` / `TabBar`.

use iced::{
    Alignment, Element, Length,
    widget::{button, container, row, space, text},
};
use snora::{LayoutDirection, Tab, TabAction, TabBar, widget::app_tab_bar};

use crate::{
    message::{Message, WorkspaceMessage},
    state::AppState,
};

/// Workspace ID newtype alias used as `TabId`.
use knotra_vcs::model::workspace::WorkspaceId;

pub fn view(state: &AppState) -> Element<'_, Message> {
    // Build Tab list, embedding the attention badge into the label.
    let tabs: Vec<Tab<WorkspaceId>> = state
        .all_workspaces
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let is_active = i == state.active_workspace_idx;

            let attention = state.workspace_status.as_ref()
                .filter(|_| is_active)
                .map(|wss| {
                    let missing = &state.missing_projects;
                    ws.projects.iter()
                        .filter(|p| {
                            if missing.contains(&p.id) { return true; }
                            let status = wss.projects.iter()
                                .find(|ps| ps.project_id == p.id);
                            let (tier, _) = crate::state::tier::compute_tier(status, true);
                            tier == crate::state::AttentionTier::NeedsAttention
                        })
                        .count()
                })
                .unwrap_or(0);

            let label = if attention > 0 {
                format!("{} ({})", ws.name, attention)
            } else {
                ws.name.clone()
            };

            Tab { id: ws.id.clone(), label, icon: None }
        })
        .collect();

    // Active workspace id (None → empty workspace list, use a sentinel).
    let active_id = state.all_workspaces
        .get(state.active_workspace_idx)
        .map(|ws| ws.id.clone());

    let tab_strip: Element<'_, Message> = if let Some(active) = active_id {
        app_tab_bar(
            TabBar { tabs, active },
            &|action: TabAction<WorkspaceId>| match action {
                TabAction::Pressed(id) => Message::Workspace(
                    WorkspaceMessage::WorkspaceSwitched(id)
                ),
            },
            LayoutDirection::Ltr,
        )
    } else {
        // No workspaces yet — render an empty spacer.
        space().width(Length::Fill).into()
    };

    // Fixed action buttons: new workspace, history, settings.
    let new_btn = button(text("+").size(13))
        .on_press(Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened));
    let history_btn = button(text("⊟").size(13))
        .on_press(Message::Navigate(crate::state::Screen::History));
    let settings_btn = button(text("⚙").size(13))
        .on_press(Message::Navigate(crate::state::Screen::Settings));

    container(
        row![
            tab_strip,
            new_btn,
            space().width(Length::Fixed(4.0)),
            history_btn,
            settings_btn,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([0, 8]),
    )
    .width(Length::Fill)
    .into()
}
