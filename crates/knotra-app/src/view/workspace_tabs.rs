#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-015 — Workspace tab strip at the top of the window.

use iced::{
    widget::{button, container, row, text},
    Alignment, Element, Length,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::AppState,
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let tabs: Vec<Element<'_, Message>> = state
        .all_workspaces
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let is_active = i == state.active_workspace_idx;

            // Count "needs attention" projects (RFC-010 badge).
            let attention = state.workspace_status.as_ref()
                .filter(|_| is_active)
                .map(|wss| {
                    let missing = &state.missing_projects;
                    ws.projects.iter()
                        .filter(|p| {
                            if missing.contains(&p.id) { return true; }
                            let status = wss.projects.iter()
                                .find(|ps| ps.project_id == p.id);
                            let (tier, _) = crate::state::tier::compute_tier(
                                status, true,
                            );
                            tier == crate::state::AttentionTier::NeedsAttention
                        })
                        .count()
                })
                .unwrap_or(0);

            let badge = if attention > 0 {
                format!("{} ({})", ws.name, attention)
            } else {
                ws.name.clone()
            };

            let btn = button(text(badge).size(13))
                .on_press_maybe(if is_active {
                    None
                } else {
                    Some(Message::Workspace(
                        WorkspaceMessage::WorkspaceSwitched(ws.id.clone())
                    ))
                });
            let _ = is_active; // styling via theming later
            btn.into()
        })
        .collect();

    let new_btn = button(text("+").size(13))
        .on_press(Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened));

    container(
        row(tabs)
            .push(new_btn)
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([4, 8])
    )
    .width(Length::Fill)
    .into()
}
