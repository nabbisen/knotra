#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-014 — Project detail side panel.
//!
//! Opens as a right-docked panel when the user clicks a project name.
//! Showing all status fields, recent operations, and available actions.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};

use crate::{
    message::{DetailPanelMessage, Message, ProjectMessage, WorkspaceMessage},
    state::AppState,
};

pub fn view<'a>(state: &'a AppState) -> Option<Element<'a, Message>> {
    let id = state.detail_panel.open_project_id.as_ref()?;

    let project = state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))?;

    let status = state
        .workspace_status
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|ps| &ps.project_id == id));

    // --- Header ---
    let close_btn =
        button(text("✕").size(12)).on_press(Message::DetailPanel(DetailPanelMessage::Closed));

    let header = row![
        text(project.name.clone()).size(15),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    // --- Identity section ---
    let vcs = status
        .map(|s| s.identity.vcs_kind.to_string())
        .unwrap_or_else(|| "—".into());
    let path = project.path.clone();
    let remote = status
        .and_then(|s| s.remote.upstream.clone())
        .unwrap_or_else(|| "—".into());

    let identity = column![
        text("Identity").size(12),
        text(format!("VCS:    {}", vcs)).size(11),
        text(format!("Path:   {}", path)).size(11),
        text(format!("Remote: {}", remote)).size(11),
    ]
    .spacing(3);

    // --- Status section ---
    let status_col = if let Some(s) = status {
        let branch = s.context.as_ref().map(|c| c.label.as_str()).unwrap_or("—");
        let ahead = s.remote.ahead;
        let behind = s.remote.behind;
        let dirty = s.working_tree.uncommitted_count;
        let untracked = s.working_tree.untracked_count;
        let conflict = if s.conflict.has_conflict {
            "Yes"
        } else if s.conflict.detection_unavailable {
            "Unknown"
        } else {
            "No"
        };

        column![
            text("Status").size(12),
            text(format!("Branch:     {}", branch)).size(11),
            text(format!("Ahead:      {}", ahead)).size(11),
            text(format!("Behind:     {}", behind)).size(11),
            text(format!("Dirty:      {}", dirty)).size(11),
            text(format!("Untracked:  {}", untracked)).size(11),
            text(format!("Conflict:   {}", conflict)).size(11),
        ]
        .spacing(3)
    } else {
        column![text("Status").size(12), text("Loading…").size(11)]
    };

    // --- Recent operations section (last 5 involving this project) ---
    let recent_ops: Vec<Element<'_, Message>> = state
        .operation_logs
        .iter()
        .rev()
        .filter(|log| log.result.per_project.iter().any(|pp| &pp.project_id == id))
        .take(5)
        .map(|log| {
            let ok = log
                .result
                .per_project
                .iter()
                .find(|pp| &pp.project_id == id)
                .map(|pp| pp.success)
                .unwrap_or(false);
            let icon = if ok { "✓" } else { "✗" };
            text(format!(
                "{} {} — {}",
                icon,
                log.result.kind,
                log.result.started_at.format("%m/%d %H:%M").to_string()
            ))
            .size(11)
            .into()
        })
        .collect();

    let recent = column(
        std::iter::once(text("Recent operations").size(12).into()).chain(
            if recent_ops.is_empty() {
                vec![text("None").size(11).into()]
            } else {
                recent_ops
            },
        ),
    )
    .spacing(3);

    // --- Actions ---
    let refresh_btn = button(text("Refresh").size(12)).on_press(Message::Project(
        ProjectMessage::StatusRefreshRequested(id.clone()),
    ));

    let fetch_btn = button(text("Fetch").size(12))
        .on_press(Message::Project(ProjectMessage::FetchRequested(id.clone())));

    let remove_btn = button(text("Remove from workspace").size(12)).on_press(Message::Workspace(
        WorkspaceMessage::RemoveProjectRequested(id.clone()),
    ));

    let actions = column![
        text("Actions").size(12),
        row![refresh_btn, fetch_btn].spacing(6),
        remove_btn,
    ]
    .spacing(6);

    let content = column![header, identity, status_col, recent, actions,]
        .spacing(16)
        .padding(16);

    Some(
        container(scrollable(content))
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
            .into(),
    )
}
