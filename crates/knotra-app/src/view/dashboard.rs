#![allow(unused_imports)]
//! Dashboard view: card grid with filter chips, grouping, and add-project dialog.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};
use knotra_ui::{
    theme::StatusColor,
    widget::{BUTTON_HEIGHT, CARD_GAP, FONT_BODY},
};
use knotra_vcs::model::{project::Project, status::ProjectStatus};

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

    // add_project_dialog is now rendered as a centered stack overlay in view.rs.

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
        .map(|ws| ws.name.as_str())
        .unwrap_or("—");

    // Minimal header: workspace name + refresh indicator.
    // Add project / bulk sync are accessible via ⌘K or the selection bar.
    let right: Element<'_, Message> = if state.is_refreshing {
        text(format!("⟳  {}", state.t("plain.status.checking")))
            .size(14)
            .into()
    } else {
        button(text(format!("⟳  {}", state.t("plain.check_now"))).size(14))
            .on_press(Message::Workspace(WorkspaceMessage::RefreshRequested))
            .into()
    };

    row![
        text(workspace_name).size(18),
        Space::new().width(Length::Fill),
        right,
    ]
    .align_y(Alignment::Center)
    .padding([8, 14])
    .into()
}

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
    .id(knotra_ui::widget::focus_id::SEARCH.clone())
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
// Tier-based card grid (RFC-0010)
// ---------------------------------------------------------------------------

fn view_tier_grid(state: &AppState) -> Element<'_, Message> {
    use iced::Length;
    use iced::widget::{button, column, container, row, text};
    use knotra_ui::widget::CARD_GAP;

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
        state.t("tier.needs_attention"),
        "🔴",
        AttentionTier::NeedsAttention,
        state.tier_collapse.needs_attention
    );
    tier_section!(
        active,
        state.t("tier.active"),
        "🟡",
        AttentionTier::Active,
        state.tier_collapse.active
    );
    tier_section!(
        clean,
        state.t("tier.clean"),
        "⚪",
        AttentionTier::Clean,
        state.tier_collapse.clean
    );

    if page.is_empty() {
        // All tiers are empty — either all projects are clean and the filter
        // isn't set, or the filter matches nothing.
        let has_filter =
            !state.filter.search_text.is_empty() || !state.filter.status_filters.is_empty();
        let msg = if has_filter {
            state.t("plain.empty.no_match")
        } else {
            state.t("plain.empty.all_clean")
        };
        let hint = if has_filter {
            ""
        } else {
            state.t("plain.empty.all_clean_hint")
        };
        return container(
            column![text(msg).size(FONT_BODY + 2.0), text(hint).size(FONT_BODY),]
                .spacing(8)
                .align_x(iced::Alignment::Center),
        )
        .width(iced::Length::Fill)
        .padding([40, 0])
        .center_x(iced::Length::Fill)
        .into();
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
        // Welcome empty state — guides the user to their first action.
        return container(
            column![
                text(state.t("plain.empty.welcome_title")).size(FONT_BODY + 6.0),
                text(state.t("plain.empty.welcome_body")).size(FONT_BODY),
                button(text(state.t("plain.empty.add_first")).size(FONT_BODY))
                    .height(BUTTON_HEIGHT)
                    .padding([0, 24])
                    .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened)),
            ]
            .spacing(16)
            .align_x(iced::Alignment::Center),
        )
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .center(iced::Length::Fill)
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
                let r: Vec<Element<'_, Message>> = std::mem::take(&mut current_row);
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

    // Clicking the name opens the detail panel (RFC-0014)
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
    // First-level wording uses plain language (UX review). The technical terms
    // (Synced / Behind / Ahead / Uncommitted / Conflict) remain available in
    // the project detail panel and operation history under "Show details".
    match color {
        StatusColor::Healthy => state.t("plain.status.all_set"),
        StatusColor::Behind => state.t("plain.status.behind"),
        StatusColor::Ahead => state.t("plain.status.ahead"),
        StatusColor::Dirty => state.t("plain.status.unsaved_work"),
        StatusColor::Conflict => state.t("plain.status.needs_choice"),
        StatusColor::Unknown => state.t("plain.status.not_sure"),
    }
}

// ---------------------------------------------------------------------------
// Add-project dialog
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Confirm-remove dialog
// ---------------------------------------------------------------------------

fn view_confirm_remove_dialog(state: &AppState) -> Element<'_, Message> {
    use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button};

    let dialog = match &state.confirm_remove_dialog {
        Some(d) => d,
        None => return Space::new().into(),
    };

    let id = dialog.project_id.clone();

    container(
        column![
            text(state.t("plain.remove.title")).size(FONT_BODY + 2.0),
            text(dialog.project_name.as_str().to_string()).size(FONT_BODY),
            text(state.t("plain.remove.body")).size(FONT_SMALL),
            // Safe action (Cancel) on the left, risky (Remove) on the right.
            row![
                guided_button(
                    state.t("confirm.remove_no"),
                    Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
                    None,
                ),
                guided_button(
                    state.t("plain.remove.confirm"),
                    Some(Message::Workspace(
                        WorkspaceMessage::RemoveProjectConfirmed(id)
                    )),
                    None,
                ),
            ]
            .spacing(12),
        ]
        .spacing(14)
        .padding(24),
    )
    .width(380)
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
#[allow(dead_code)]
fn card_needs_attention<'a>(
    state: &'a AppState,
    project: &'a knotra_vcs::Project,
    status: Option<&'a knotra_vcs::ProjectStatus>,
    cause: Option<crate::state::tier::AttentionCause>,
) -> Element<'a, Message> {
    use crate::state::tier::AttentionCause;

    // One-line problem description — no technical jargon.
    let problem = match &cause {
        Some(AttentionCause::PathNotFound) => "folder not found".to_owned(),
        Some(AttentionCause::Conflict) => "merge conflict".to_owned(),
        Some(AttentionCause::ConflictDetectionUnavailable) => "conflict status unknown".to_owned(),
        Some(AttentionCause::DetachedHead) => "detached HEAD".to_owned(),
        Some(AttentionCause::OperationFailed) => "last operation failed".to_owned(),
        Some(AttentionCause::DirtyForLong) => "uncommitted for a long time".to_owned(),
        None => status
            .and_then(|s| s.read_error.as_deref())
            .map(|e| e.to_owned())
            .unwrap_or_else(|| "needs attention".to_owned()),
    };

    // One focused action button.
    let action: Element<'_, Message> = match &cause {
        Some(AttentionCause::Conflict) => button(text("Resolve").size(12))
            .on_press(Message::ConflictOps(
                crate::message::ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
            ))
            .into(),
        Some(AttentionCause::PathNotFound) => button(text("Remove").size(12))
            .on_press(Message::Workspace(
                crate::message::WorkspaceMessage::RemoveProjectRequested(project.id.clone()),
            ))
            .into(),
        _ => button(text("Refresh").size(12))
            .on_press(Message::Project(
                crate::message::ProjectMessage::StatusRefreshRequested(project.id.clone()),
            ))
            .into(),
    };

    let name_btn = button(text(project.name.as_str()).size(13)).on_press(Message::DetailPanel(
        crate::message::DetailPanelMessage::Opened(project.id.clone()),
    ));

    let inner = row![
        name_btn,
        text("  —  ").size(12),
        text(problem).size(12),
        Space::new().width(Length::Fill),
        action,
    ]
    .align_y(iced::Alignment::Center)
    .padding([8, 12]);

    // Selection mode: show checkbox on the left
    let inner: Element<'_, Message> = if state.selection_mode {
        let is_sel = state.selection.contains(&project.id);
        let cb =
            button(text(if is_sel { "☑" } else { "☐" }).size(12)).on_press(Message::Selection(
                crate::message::SelectionMessage::Toggled(project.id.clone()),
            ));
        row![cb, inner].align_y(iced::Alignment::Center).into()
    } else {
        inner.into()
    };

    container(inner).width(iced::Length::Fill).into()
}

// ---------------------------------------------------------------------------
// Active card: name  branch
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn card_active<'a>(
    state: &'a AppState,
    project: &'a knotra_vcs::Project,
    status: Option<&'a knotra_vcs::ProjectStatus>,
) -> Element<'a, Message> {
    let branch = status
        .and_then(|s| s.context.as_ref())
        .map(|ctx| ctx.label.as_str())
        .unwrap_or("");

    let name_btn = button(text(project.name.as_str()).size(13)).on_press(Message::DetailPanel(
        crate::message::DetailPanelMessage::Opened(project.id.clone()),
    ));

    let inner = row![name_btn, text(branch).size(11),]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([6, 12]);

    let inner: Element<'_, Message> = if state.selection_mode {
        let is_sel = state.selection.contains(&project.id);
        let cb =
            button(text(if is_sel { "☑" } else { "☐" }).size(12)).on_press(Message::Selection(
                crate::message::SelectionMessage::Toggled(project.id.clone()),
            ));
        row![cb, inner].align_y(iced::Alignment::Center).into()
    } else {
        inner.into()
    };

    iced::widget::container(inner)
        .width(iced::Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Clean card: name only (single line, subdued)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn card_clean<'a>(state: &'a AppState, project: &'a knotra_vcs::Project) -> Element<'a, Message> {
    let name_btn = button(text(project.name.as_str()).size(13)).on_press(Message::DetailPanel(
        crate::message::DetailPanelMessage::Opened(project.id.clone()),
    ));

    let inner: Element<'_, Message> = if state.selection_mode {
        let is_sel = state.selection.contains(&project.id);
        let cb =
            button(text(if is_sel { "☑" } else { "☐" }).size(12)).on_press(Message::Selection(
                crate::message::SelectionMessage::Toggled(project.id.clone()),
            ));
        row![cb, name_btn]
            .align_y(iced::Alignment::Center)
            .padding([4, 12])
            .into()
    } else {
        iced::widget::container(name_btn.padding([4, 12]))
            .width(iced::Length::Fill)
            .into()
    };

    inner
}
