#![allow(unused_imports)]
//! Dashboard view: card grid with filter chips, grouping, and add-project dialog.

use endringer::model::{project::Project, status::ProjectStatus};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};
use snora::{theme::StatusColor, widget::CARD_GAP};

use crate::{
    message::{
        DetailPanelMessage, FilterMessage, Message, ProjectMessage, SelectionMessage, StatusFilter,
        SyncMessage, TierMessage, WorkspaceMessage,
    },
    state::{
        AppState, AttentionTier, GroupingMode, LoadPhase,
        dashboard::{build_display_groups, project_status_color},
        tier::compute_tier,
    },
};

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let toolbar = view_toolbar(state);

    let body: Element<'_, Message> = match &state.load_phase {
        LoadPhase::Startup => placeholder(state.t("status.refreshing")),
        LoadPhase::Refreshing => {
            // Show stale cards (if any) with a "refreshing" notice overlaid.
            if state.workspace_status.is_some() {
                column![
                    text(state.t("dashboard.refreshing_count")).size(12),
                    view_card_grid(state),
                ]
                .spacing(4)
                .into()
            } else {
                placeholder(state.t("status.refreshing"))
            }
        }
        LoadPhase::Error(_) => view_error(state),
        LoadPhase::Ready => {
            if state.grouping_mode == GroupingMode::Auto {
                view_tier_grid(state)
            } else {
                view_card_grid(state)
            }
        }
    };

    // Layer dialogs on top when open.
    let mut root =
        column![header, toolbar, scrollable(body).height(Length::Fill)].height(Length::Fill);

    // Persistent status bar.
    if let Some(ref msg) = state.status_bar {
        root = root.push(
            container(text(msg.as_str()).size(12))
                .width(Length::Fill)
                .padding([2, 8]),
        );
    }

    // add_project_dialog is now rendered as a centered stack overlay in view/mod.rs.

    if state.confirm_remove_dialog.is_some() {
        return column![root, view_confirm_remove_dialog(state)]
            .height(Length::Fill)
            .into();
    }

    root.into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

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

    let refresh_label = if state.is_refreshing {
        state.t("status.refreshing")
    } else {
        state.t("dashboard.refresh")
    };

    let refresh_btn = button(text(refresh_label)).on_press_maybe(if state.is_refreshing {
        None
    } else {
        Some(Message::Workspace(WorkspaceMessage::RefreshRequested))
    });

    let add_btn = button(text(state.t("dashboard.add_project")))
        .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened));

    let bulk_btn = button(text(state.t("dashboard.bulk_sync")))
        .on_press(Message::Sync(crate::message::SyncMessage::OpenRequested));

    row![
        text(workspace_name).size(20),
        text(format!(
            "  {} {}",
            state.t("dashboard.last_updated"),
            last_updated
        ))
        .size(12),
        Space::new().width(Length::Fill),
        add_btn,
        refresh_btn,
        bulk_btn,
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Toolbar: filter chips + search box
// ---------------------------------------------------------------------------

fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    // Filter chips row.
    let chips = view_filter_chips(state);

    // Clear-all button (only shown when a filter is active).
    let clear_btn: Option<Element<'_, Message>> = if state.filter.is_active() {
        Some(
            button(text("✕ Clear"))
                .on_press(Message::Filter(FilterMessage::AllFiltersCleared))
                .into(),
        )
    } else {
        None
    };

    // Search box.
    let search = text_input(
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .on_input(|s| Message::Filter(FilterMessage::SearchChanged(s)))
    .width(200);

    // Group selector placeholder (full picker in Phase 6).
    let group_btn = button(text(state.t("dashboard.group_by")));

    let mut toolbar_row = row![chips].spacing(6).align_y(Alignment::Center);
    if let Some(btn) = clear_btn {
        toolbar_row = toolbar_row.push(btn);
    }
    toolbar_row = toolbar_row
        .push(Space::new().width(Length::Fill))
        .push(group_btn)
        .push(search);

    container(toolbar_row)
        .width(Length::Fill)
        .padding(Padding {
            top: 0.0,
            bottom: 8.0,
            left: 12.0,
            right: 12.0,
        })
        .into()
}

fn view_filter_chips(state: &AppState) -> Element<'_, Message> {
    let filters: &[(StatusFilter, &'static str)] = &[
        (StatusFilter::Healthy, "filter.healthy"),
        (StatusFilter::Behind, "filter.behind"),
        (StatusFilter::Ahead, "filter.ahead"),
        (StatusFilter::Dirty, "filter.dirty"),
        (StatusFilter::Conflict, "filter.conflict"),
        (StatusFilter::Error, "filter.error"),
    ];

    let mut chips: Vec<Element<'_, Message>> = Vec::new();

    for (sf, key) in filters {
        let active = state.filter.has_status_filter(sf);
        let label = format!("{}{}", state.t(key), if active { " ✓" } else { "" });
        let btn = button(text(label).size(12)).on_press(Message::Filter(
            FilterMessage::StatusFilterToggled(sf.clone()),
        ));
        chips.push(btn.into());
    }

    row(chips).spacing(4).into()
}

// ---------------------------------------------------------------------------
// Card grid with grouping
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tier-based card grid (RFC-010)
// ---------------------------------------------------------------------------

fn view_tier_grid(state: &AppState) -> Element<'_, Message> {
    use iced::Length;
    use iced::widget::{button, column, container, row, text};
    use snora::widget::CARD_GAP;

    let projects = state
        .workspace
        .as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);
    let wss = state.workspace_status.as_ref();

    // Classify all projects into tiers.
    let mut needs_att: Vec<_> = Vec::new();
    let mut active: Vec<_> = Vec::new();
    let mut clean: Vec<_> = Vec::new();

    for p in projects {
        let status = wss.and_then(|w| w.projects.iter().find(|ps| ps.project_id == p.id));
        let missing = state.missing_projects.contains(&p.id);
        let (tier, cause) = compute_tier(status, !missing);
        match tier {
            AttentionTier::NeedsAttention => needs_att.push((p, status, cause)),
            AttentionTier::Active => active.push((p, status, cause)),
            AttentionTier::Clean => clean.push((p, status, cause)),
        }
    }

    let mut page: Vec<Element<'_, Message>> = Vec::new();

    // Helper: render a collapsible tier section.
    macro_rules! tier_section {
        ($entries:expr, $label:expr, $icon:expr, $tier:expr, $collapsed:expr) => {{
            if !$entries.is_empty() {
                let toggle_btn = button(
                    text(format!(
                        "{} {} ({})  {}",
                        $icon,
                        $label,
                        $entries.len(),
                        if $collapsed { "▶" } else { "▼" }
                    ))
                    .size(13),
                )
                .on_press(Message::Tier(TierMessage::Toggled($tier)));
                page.push(
                    container(toggle_btn)
                        .width(Length::Fill)
                        .padding([4, 0])
                        .into(),
                );
                if !$collapsed {
                    for (proj, status, _cause) in &$entries {
                        page.push(view_project_card(state, proj, *status));
                    }
                }
            }
        }};
    }

    tier_section!(
        needs_att,
        "Needs attention",
        "🔴",
        AttentionTier::NeedsAttention,
        state.tier_collapse.needs_attention
    );
    tier_section!(
        active,
        "Active",
        "🟡",
        AttentionTier::Active,
        state.tier_collapse.active
    );
    tier_section!(
        clean,
        "Clean",
        "⚪",
        AttentionTier::Clean,
        state.tier_collapse.clean
    );

    if page.is_empty() {
        return placeholder("No projects match the current filter.");
    }
    column(page).spacing(CARD_GAP).padding(12).into()
}

fn view_card_grid(state: &AppState) -> Element<'_, Message> {
    let projects = state
        .workspace
        .as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);

    if projects.is_empty() {
        return column![
            placeholder(state.t("dashboard.no_projects")),
            button(text(state.t("dashboard.add_project")))
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened)),
        ]
        .spacing(12)
        .padding(24)
        .into();
    }

    let groups = build_display_groups(projects, state.workspace_status.as_ref(), &state.filter);

    if groups.iter().all(|g| g.entries.is_empty()) {
        return placeholder("No projects match the current filter.");
    }

    const COLS: usize = 4;
    let mut page: Vec<Element<'_, Message>> = Vec::new();

    for group in &groups {
        // Group header (skip for the lone-ungrouped case when no named groups).
        if let Some(name) = group.name {
            page.push(
                container(text(name).size(13))
                    .width(Length::Fill)
                    .padding([4, 0])
                    .into(),
            );
        }

        // Card rows.
        let mut current_row: Vec<Element<'_, Message>> = Vec::new();
        for entry in &group.entries {
            current_row.push(view_project_card(state, entry.project, entry.status));
            if current_row.len() == COLS {
                let r: Vec<Element<'_, Message>> = current_row.drain(..).collect();
                page.push(row(r).spacing(CARD_GAP).into());
            }
        }
        if !current_row.is_empty() {
            page.push(row(current_row).spacing(CARD_GAP).into());
        }
    }

    column(page).spacing(CARD_GAP).padding(12).into()
}

// ---------------------------------------------------------------------------
// Project card
// ---------------------------------------------------------------------------

fn view_project_card<'a>(
    state: &'a AppState,
    project: &'a Project,
    status: Option<&'a ProjectStatus>,
) -> Element<'a, Message> {
    let vcs_label = status
        .map(|s| s.identity.vcs_kind.to_string())
        .unwrap_or_else(|| "—".to_owned());

    let context_label = status
        .and_then(|s| s.context.as_ref())
        .map(|c| c.label.clone())
        .unwrap_or_else(|| "—".to_owned());

    let status_color = status
        .map(project_status_color)
        .unwrap_or(StatusColor::Unknown);
    let status_label = status_color_label(state, status_color);

    let ahead = status.map(|s| s.remote.ahead).unwrap_or(0);
    let behind = status.map(|s| s.remote.behind).unwrap_or(0);
    let uncommitted = status
        .map(|s| s.working_tree.uncommitted_count)
        .unwrap_or(0);
    let untracked = status.map(|s| s.working_tree.untracked_count).unwrap_or(0);
    let updated = status
        .map(|s| s.refreshed_at.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_owned());

    let is_fetching = state.fetching_projects.contains(&project.id);

    // Header row: checkbox | name  |  VCS badge
    let is_selected = state.selection.contains(&project.id);
    let checkbox_label = if is_selected { "☑" } else { "☐" };
    let select_btn = button(text(checkbox_label).size(13)).on_press(Message::Selection(
        SelectionMessage::Toggled(project.id.clone()),
    ));

    // Clicking the name opens the detail panel (RFC-014)
    let name_btn = button(text(project.name.clone()).size(14)).on_press(Message::DetailPanel(
        DetailPanelMessage::Opened(project.id.clone()),
    ));

    let header_row = row![
        select_btn,
        name_btn,
        Space::new().width(Length::Fill),
        text(vcs_label).size(11),
    ]
    .align_y(Alignment::Center);

    // Status badge + context
    let status_row = row![
        text(status_label).size(12),
        text("  ").size(12),
        text(context_label).size(12),
    ]
    .align_y(Alignment::Center);

    // Stat cells
    let stats_row = row![
        stat_cell("↑", state.t("card.ahead"), ahead),
        stat_cell("↓", state.t("card.behind"), behind),
        stat_cell("●", state.t("card.uncommitted"), uncommitted),
        stat_cell("?", state.t("card.untracked"), untracked),
    ]
    .spacing(10);

    // Action buttons
    let fetch_label = if is_fetching {
        "Fetching…"
    } else {
        state.t("card.action.fetch")
    };
    let fetch_btn = button(text(fetch_label).size(11)).on_press_maybe(if is_fetching {
        None
    } else {
        Some(Message::Project(ProjectMessage::FetchRequested(
            project.id.clone(),
        )))
    });

    let remove_btn = button(text(state.t("card.action.remove")).size(11)).on_press(
        Message::Workspace(WorkspaceMessage::RemoveProjectRequested(project.id.clone())),
    );

    let actions_row = row![fetch_btn, remove_btn]
        .spacing(4)
        .align_y(Alignment::Center);

    // Error row
    let mut card_col = column![
        header_row,
        status_row,
        stats_row,
        text(format!("{} {}", state.t("card.updated"), updated)).size(10),
        actions_row,
    ]
    .spacing(5)
    .padding([12, 14]);

    if let Some(err) = status.and_then(|s| s.read_error.as_ref()) {
        card_col = card_col.push(text(format!("⚠ {}", err)).size(11));
    }

    // Missing-path warning (repo directory not found).
    if state.missing_projects.contains(&project.id) {
        card_col = card_col.push(text("✗ Repository path not found").size(11));
    }

    container(card_col).width(Length::FillPortion(1)).into()
}

fn stat_cell<'a>(icon: &'a str, label: &'a str, value: u32) -> Element<'a, Message> {
    column![
        row![text(icon).size(11), text(value.to_string()).size(14)]
            .spacing(2)
            .align_y(Alignment::Center),
        text(label).size(9),
    ]
    .align_x(Alignment::Center)
    .into()
}

fn status_color_label(state: &AppState, color: StatusColor) -> &'static str {
    match color {
        StatusColor::Healthy => state.t("status.healthy"),
        StatusColor::Behind => state.t("status.behind"),
        StatusColor::Ahead => state.t("status.ahead"),
        StatusColor::Dirty => state.t("status.dirty"),
        StatusColor::Conflict => state.t("status.conflict"),
        StatusColor::Unknown => state.t("status.unknown"),
    }
}

// ---------------------------------------------------------------------------
// Add-project dialog
// ---------------------------------------------------------------------------

fn view_add_project_dialog(state: &AppState) -> Element<'_, Message> {
    let dialog = match &state.add_project_dialog {
        Some(d) => d,
        None => return Space::new().into(),
    };

    let mut col = column![
        text(state.t("dialog.add_project.title")).size(18),
        text(state.t("dialog.add_project.name_label")).size(13),
        text_input(state.t("dialog.add_project.name_hint"), &dialog.name,)
            .on_input(|s| Message::Workspace(WorkspaceMessage::AddProjectNameChanged(s))),
        text(state.t("dialog.add_project.path_label")).size(13),
        text_input(state.t("dialog.add_project.path_hint"), &dialog.path,)
            .on_input(|s| Message::Workspace(WorkspaceMessage::AddProjectPathChanged(s))),
        row![
            button(text(state.t("dialog.add_project.confirm")))
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectConfirmed)),
            button(text(state.t("dialog.add_project.cancel")))
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .padding(24);

    if let Some(ref err) = dialog.error {
        col = col.push(text(err.as_str()).size(12));
    }

    container(col).width(400).into()
}

// ---------------------------------------------------------------------------
// Confirm-remove dialog
// ---------------------------------------------------------------------------

fn view_confirm_remove_dialog(state: &AppState) -> Element<'_, Message> {
    let dialog = match &state.confirm_remove_dialog {
        Some(d) => d,
        None => return Space::new().into(),
    };

    let id = dialog.project_id.clone();
    let _id2 = id.clone();

    container(
        column![
            text(state.t("confirm.remove_project")).size(15),
            text(dialog.project_name.as_str()).size(14),
            row![
                button(text(state.t("confirm.remove_yes"))).on_press(Message::Workspace(
                    WorkspaceMessage::RemoveProjectConfirmed(id)
                )),
                button(text(state.t("confirm.remove_no")))
                    .on_press(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .padding(24),
    )
    .width(360)
    .into()
}

// ---------------------------------------------------------------------------
// Error + placeholder
// ---------------------------------------------------------------------------

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

fn placeholder(msg: &str) -> Element<'_, Message> {
    container(text(msg).size(14))
        .width(Length::Fill)
        .height(250)
        .center_x(Length::Fill)
        .center_y(250)
        .into()
}
