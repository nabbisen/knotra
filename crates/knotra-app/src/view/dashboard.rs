//! Dashboard view: card grid showing all project statuses at a glance.

use endringer::model::status::ProjectStatus;
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};
use snora::{theme::StatusColor, widget::CARD_GAP};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::{AppState, LoadPhase, dashboard::project_status_color},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let toolbar = view_toolbar(state);
    let body: Element<'_, Message> = match &state.load_phase {
        LoadPhase::Startup => view_startup(state),
        LoadPhase::Refreshing => view_refreshing(state),
        LoadPhase::Error(_) => view_error(state),
        LoadPhase::Ready => view_cards(state),
    };

    column![header, toolbar, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

fn view_header(state: &AppState) -> Element<'_, Message> {
    let workspace_name = state
        .workspace
        .as_ref()
        .map(|w| w.name.as_str())
        .unwrap_or("—");

    let last_updated = state
        .workspace_status
        .as_ref()
        .and_then(|ws| ws.last_refresh)
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_owned());

    let refresh_btn = button(text(state.t("dashboard.refresh")))
        .on_press(Message::Workspace(WorkspaceMessage::RefreshRequested));

    let bulk_sync_btn = button(text(state.t("dashboard.bulk_sync")));

    row![
        text(workspace_name).size(20),
        text(format!(
            "  {}  {}",
            state.t("dashboard.last_updated"),
            last_updated
        ))
        .size(13),
        Space::new().width(Length::Fill),
        refresh_btn,
        bulk_sync_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    let search = text_input(
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .on_input(|s| Message::Filter(FilterMessage::SearchChanged(s)))
    .width(220);

    let filter_btn = button(text(state.t("dashboard.filter")));
    let group_btn = button(text(state.t("dashboard.group")));

    row![
        filter_btn,
        group_btn,
        Space::new().width(Length::Fill),
        search
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        bottom: 8.0,
        left: 12.0,
        right: 12.0,
    })
    .into()
}

fn view_startup(state: &AppState) -> Element<'_, Message> {
    centered_message(state.t("dashboard.no_projects"))
}

fn view_refreshing(state: &AppState) -> Element<'_, Message> {
    centered_message(state.t("status.refreshing"))
}

fn view_error(state: &AppState) -> Element<'_, Message> {
    let msg = match &state.load_phase {
        LoadPhase::Error(m) => m.as_str(),
        _ => "",
    };
    column![
        text(state.t("error.read_failed")).size(16),
        text(msg).size(13),
        button(text(state.t("dashboard.refresh")))
            .on_press(Message::Workspace(WorkspaceMessage::RefreshRequested)),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn view_cards(state: &AppState) -> Element<'_, Message> {
    let projects = state
        .workspace
        .as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);
    let statuses = state
        .workspace_status
        .as_ref()
        .map(|ws| ws.projects.as_slice())
        .unwrap_or(&[]);

    if projects.is_empty() {
        return column![
            centered_message(state.t("dashboard.no_projects")),
            button(text(state.t("dashboard.add_project")))
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened)),
        ]
        .spacing(12)
        .padding(24)
        .into();
    }

    const COLS: usize = 4;
    let mut grid_rows: Vec<Element<'_, Message>> = Vec::new();
    let mut current_row: Vec<Element<'_, Message>> = Vec::new();

    for project in projects.iter() {
        if !state.filter.search_text.is_empty()
            && !project
                .name
                .to_lowercase()
                .contains(&state.filter.search_text.to_lowercase())
        {
            continue;
        }

        let status = statuses.iter().find(|s| s.project_id == project.id);
        let card = view_project_card(state, project, status);
        current_row.push(card);

        if current_row.len() == COLS {
            let r: Vec<Element<'_, Message>> = current_row.drain(..).collect();
            grid_rows.push(row(r).spacing(CARD_GAP).into());
        }
    }

    if !current_row.is_empty() {
        grid_rows.push(row(current_row).spacing(CARD_GAP).into());
    }

    column(grid_rows).spacing(CARD_GAP).padding(12).into()
}

fn view_project_card<'a>(
    state: &'a AppState,
    project: &'a endringer::model::project::Project,
    status: Option<&'a ProjectStatus>,
) -> Element<'a, Message> {
    let vcs_label = status
        .map(|s| s.identity.vcs_kind.to_string())
        .unwrap_or_else(|| "—".to_owned());

    let context_label = status
        .and_then(|s| s.context.as_ref())
        .map(|c| c.label.as_str())
        .unwrap_or("—");

    let status_color = status
        .map(project_status_color)
        .unwrap_or(StatusColor::Unknown);

    let status_label = match status_color {
        StatusColor::Healthy => state.t("status.healthy"),
        StatusColor::Behind => state.t("status.behind"),
        StatusColor::Ahead => state.t("status.ahead"),
        StatusColor::Dirty => state.t("status.dirty"),
        StatusColor::Conflict => state.t("status.conflict"),
        StatusColor::Unknown => state.t("status.unknown"),
    };

    let ahead = status.map(|s| s.remote.ahead).unwrap_or(0);
    let behind = status.map(|s| s.remote.behind).unwrap_or(0);
    let uncommitted = status
        .map(|s| s.working_tree.uncommitted_count)
        .unwrap_or(0);
    let untracked = status.map(|s| s.working_tree.untracked_count).unwrap_or(0);
    let updated = status
        .map(|s| s.refreshed_at.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_owned());

    let mut card_col = column![
        row![
            text(project.name.clone()).size(14),
            Space::new().width(Length::Fill),
            text(vcs_label).size(11),
        ]
        .align_y(Alignment::Center),
        text(context_label.to_owned()).size(12),
        text(status_label).size(13),
        row![
            stat_cell(state.t("card.ahead"), ahead),
            stat_cell(state.t("card.behind"), behind),
            stat_cell(state.t("card.uncommitted"), uncommitted),
            stat_cell(state.t("card.untracked"), untracked),
        ]
        .spacing(8),
        text(format!("{} {}", state.t("card.updated"), updated)).size(11),
    ]
    .spacing(4)
    .padding([12, 14]);

    if let Some(err) = status.and_then(|s| s.read_error.as_ref()) {
        card_col = card_col.push(text(err.as_str()).size(11));
    }

    container(card_col).width(Length::FillPortion(1)).into()
}

fn stat_cell(label: &str, value: u32) -> Element<'_, Message> {
    column![text(value.to_string()).size(15), text(label).size(10),]
        .align_x(Alignment::Center)
        .into()
}

fn centered_message(msg: &str) -> Element<'_, Message> {
    container(text(msg).size(14))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
