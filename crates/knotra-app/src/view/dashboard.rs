//! Dashboard view for grouping, sorting, filtering, and bulk selection.

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{
        ConflictOpsMessage, DashboardMessage, DetailPanelMessage, FilterMessage, Message,
        SelectionMessage, StatusFilter, WorkspaceMessage,
    },
    state::{
        AppState, LoadPhase,
        dashboard::{
            DashboardCause, DashboardEntry, DashboardSection, DashboardSectionKey, DashboardTier,
            ProgressKind,
        },
        focus::{FocusOrder, FocusTarget},
    },
};

/// Tab/Shift-Tab focus targets for the dashboard's rows (RFC-036 R2, Stage 4):
/// collapsible section headers, row checkboxes (selection mode only), and
/// row actions. Card-to-card `↑`/`↓`/`j`/`k` movement is not this - that is
/// RFC-035's.
///
/// Iterates `DashboardDisplay::sections` in the exact order and with the
/// exact `!collapsed` filter `build_dashboard_display` used to compute
/// `ordered_selectable_ids` - this is that same computation's row targets,
/// not a second ordering (RFC-036 Stage 4 change scope). A dedicated test
/// asserts the two ID sequences are identical.
pub fn focus_order(state: &AppState) -> FocusOrder<Message> {
    let display = state.dashboard_display();
    let mut order = Vec::new();

    for section in &display.sections {
        if let DashboardSectionKey::Tier(tier) = section.key
            && tier != DashboardTier::NeedsHelp
        {
            order.push((
                FocusTarget::control_dynamic(format!("dashboard.section.{tier:?}")),
                Some(Message::Dashboard(DashboardMessage::TierToggled(tier))),
            ));
        }

        if section.collapsed {
            continue;
        }

        for entry in &section.entries {
            let id = &entry.project.id;

            if state.selection_mode {
                order.push((
                    FocusTarget::control_dynamic(format!("dashboard.row.{id}.checkbox")),
                    Some(Message::Selection(SelectionMessage::Toggled(id.clone()))),
                ));
            }

            // The name/detail-link button - present on every row regardless
            // of tier, and the most common row interaction.
            order.push((
                FocusTarget::control_dynamic(format!("dashboard.row.{id}.name")),
                Some(Message::DetailPanel(DetailPanelMessage::Opened(id.clone()))),
            ));

            // The tier-specific action button. Only NeedsHelp rows render
            // one (`view_project_row`'s `action` slot is a plain `Space`,
            // not a button, for InProgress/AllSet).
            if entry.tier == DashboardTier::NeedsHelp {
                let action_message = if entry.cause == Some(DashboardCause::Conflict) {
                    (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                        ConflictOpsMessage::OpenRequested(Some(id.clone())),
                    ))
                } else {
                    Some(Message::DetailPanel(DetailPanelMessage::Opened(id.clone())))
                };
                order.push((
                    FocusTarget::control_dynamic(format!("dashboard.row.{id}.action")),
                    action_message,
                ));
            }
        }
    }

    order
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut body = column![view_header(state), view_toolbar(state)]
        .height(Length::Fill)
        .spacing(4);
    body = body.push(scrollable(view_body(state)).height(Length::Fill));

    if let Some(message) = &state.status_bar {
        body = body.push(
            container(text(message).size(12))
                .width(Length::Fill)
                .padding([3, 12]),
        );
    }

    if state.confirm_remove_dialog.is_some() {
        return column![body, view_confirm_remove_dialog(state)]
            .height(Length::Fill)
            .into();
    }
    body.into()
}

fn view_header(state: &AppState) -> Element<'_, Message> {
    // RFC-034 R13/R14: the workspace name lives in the shell switcher now,
    // not repeated here. This is the RFC's one migrated page header; the
    // toolbar below (grouping/sorting/filtering/selection) is RFC-035.
    let refresh: Element<'_, Message> = if state.is_refreshing {
        text(state.t("plain.status.checking")).size(13).into()
    } else {
        button(text(state.t("plain.check_now")).size(13))
            .on_press(Message::Workspace(WorkspaceMessage::RefreshRequested))
            .into()
    };

    crate::view::shell::page_header(state.t("nav.dashboard"), refresh)
}

fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    let filters = [
        StatusFilter::NeedsHelp,
        StatusFilter::Dirty,
        StatusFilter::Behind,
        StatusFilter::Ahead,
        StatusFilter::Conflict,
        StatusFilter::AllSet,
    ];
    let filter_rows = column![
        row(filters[..3]
            .iter()
            .map(|filter| filter_button(state, filter))
            .collect::<Vec<_>>())
        .spacing(4),
        row(filters[3..]
            .iter()
            .map(|filter| filter_button(state, filter))
            .collect::<Vec<_>>())
        .spacing(4),
    ]
    .spacing(4);

    let grouping = row![
        text(state.t("dashboard.grouping")).size(12),
        choice_button(
            state,
            state.t("dashboard.grouping.attention"),
            state.config.dashboard_grouping == DashboardGrouping::Attention,
            Message::Dashboard(DashboardMessage::GroupingChanged(
                DashboardGrouping::Attention,
            )),
        ),
        choice_button(
            state,
            state.t("dashboard.grouping.project_group"),
            state.config.dashboard_grouping == DashboardGrouping::ProjectGroup,
            Message::Dashboard(DashboardMessage::GroupingChanged(
                DashboardGrouping::ProjectGroup,
            )),
        ),
        choice_button(
            state,
            state.t("dashboard.grouping.none"),
            state.config.dashboard_grouping == DashboardGrouping::None,
            Message::Dashboard(DashboardMessage::GroupingChanged(DashboardGrouping::None)),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let sorting = row![
        text(state.t("dashboard.sorting")).size(12),
        choice_button(
            state,
            state.t("dashboard.sorting.recommended"),
            state.config.dashboard_sort == DashboardSort::Recommended,
            Message::Dashboard(DashboardMessage::SortChanged(DashboardSort::Recommended)),
        ),
        choice_button(
            state,
            state.t("dashboard.sorting.name"),
            state.config.dashboard_sort == DashboardSort::NameAscending,
            Message::Dashboard(DashboardMessage::SortChanged(DashboardSort::NameAscending)),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let search = text_input(
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .id(knotra_ui::widget::focus_id::SEARCH.clone())
    .on_input(|value| Message::Filter(FilterMessage::SearchChanged(value)))
    .width(Length::Fixed(220.0));
    let summary = state.selection_summary();
    let select = guided_button(
        state.t("plain.selection.enter"),
        (!summary.visible_ids.is_empty())
            .then_some(Message::Selection(SelectionMessage::ModeEntered)),
        summary
            .visible_ids
            .is_empty()
            .then(|| state.t("plain.selection.no_visible_projects")),
    );
    let clear: Element<'_, Message> = if state.filter.is_active() {
        button(text(state.t("dashboard.clear_filters")).size(12))
            .on_press(Message::Filter(FilterMessage::AllFiltersCleared))
            .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    container(
        column![
            filter_rows,
            grouping,
            sorting,
            row![search, clear, Space::new().width(Length::Fill), select]
                .spacing(6)
                .align_y(Alignment::Center),
        ]
        .spacing(5),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 0.0,
        right: 12.0,
        bottom: 8.0,
        left: 12.0,
    })
    .into()
}

fn filter_button<'a>(state: &'a AppState, filter: &StatusFilter) -> Element<'a, Message> {
    let active = state.filter.has_status_filter(filter);
    button(text(format!(
        "{}{}",
        state.t(filter.label_key()),
        if active { " *" } else { "" }
    )))
    .on_press(Message::Filter(FilterMessage::StatusFilterToggled(
        filter.clone(),
    )))
    .into()
}

fn choice_button<'a>(
    _state: &'a AppState,
    label: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    button(text(format!("{label}{}", if active { " *" } else { "" })).size(12))
        .on_press_maybe((!active).then_some(message))
        .into()
}

fn view_body(state: &AppState) -> Element<'_, Message> {
    if state.workspace.is_none() {
        return view_without_workspace(state);
    }

    let projects_empty = state
        .workspace
        .as_ref()
        .is_none_or(|workspace| workspace.projects.is_empty());
    if projects_empty {
        return empty_workspace(state);
    }

    let display = state.dashboard_display();
    let mut content: Vec<Element<'_, Message>> = Vec::new();
    match &state.load_phase {
        LoadPhase::Startup | LoadPhase::Refreshing => content.push(
            container(text(state.t("plain.status.checking")).size(12))
                .width(Length::Fill)
                .padding([5, 12])
                .into(),
        ),
        LoadPhase::Error(error) => content.push(view_error_notice(
            state,
            error,
            state.t("dashboard.load_failed"),
            true,
        )),
        LoadPhase::Ready => {}
    }

    if display.sections.is_empty() {
        content.push(no_matches(state));
    } else {
        for section in display.sections {
            content.push(view_section(state, section));
        }
    }
    column(content)
        .spacing(8)
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 16.0,
            left: 12.0,
        })
        .into()
}

fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    match &state.load_phase {
        LoadPhase::Error(error) => column![
            view_error_notice(state, error, state.t("dashboard.no_workspace_error"), false,),
            button(text(state.t("dashboard.create_workspace"))).on_press(Message::Workspace(
                WorkspaceMessage::CreateWorkspaceDialogOpened,
            )),
        ]
        .spacing(10)
        .padding(24)
        .into(),
        _ => placeholder(state.t("plain.status.checking")),
    }
}

fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text(state.t("plain.empty.welcome_title")).size(FONT_BODY + 6.0),
            text(state.t("plain.empty.welcome_body")).size(FONT_BODY),
            button(text(state.t("plain.empty.add_first")))
                .height(BUTTON_HEIGHT)
                .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened,)),
        ]
        .spacing(14)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(260.0))
    .center(Length::Fill)
    .into()
}

fn view_error_notice<'a>(
    state: &'a AppState,
    error: &'a str,
    first_level_message: &'a str,
    retry_allowed: bool,
) -> Element<'a, Message> {
    let details_label = if state.dashboard_error_details_open {
        state.t("plain.hide_details")
    } else {
        state.t("plain.show_details")
    };
    let mut actions = row![
        button(text(details_label).size(12))
            .on_press(Message::Dashboard(DashboardMessage::ErrorDetailsToggled,)),
    ]
    .spacing(6);
    if retry_allowed {
        actions = actions.push(
            button(text(state.t("dashboard.try_again")).size(12))
                .on_press(Message::Dashboard(DashboardMessage::ErrorRetryRequested)),
        );
    }
    let mut notice = column![text(first_level_message).size(14), actions].spacing(6);
    if state.dashboard_error_details_open {
        notice = notice.push(text(error).size(11));
    }
    container(notice).width(Length::Fill).padding(12).into()
}

fn view_section<'a>(state: &'a AppState, section: DashboardSection<'a>) -> Element<'a, Message> {
    let mut elements = vec![section_header(
        state,
        section.key,
        section.entries.len(),
        section.collapsed,
    )];
    if !section.collapsed {
        elements.extend(
            section
                .entries
                .into_iter()
                .map(|entry| view_project_row(state, entry)),
        );
    }
    column(elements).spacing(3).into()
}

fn section_header<'a>(
    state: &'a AppState,
    key: DashboardSectionKey,
    entry_count: usize,
    collapsed: bool,
) -> Element<'a, Message> {
    let (label, toggle) = match key {
        DashboardSectionKey::Tier(tier) => {
            let label = match tier {
                DashboardTier::NeedsHelp => state.t("tier.needs_attention").to_owned(),
                DashboardTier::InProgress => state.t("tier.active").to_owned(),
                DashboardTier::AllSet => state.t("tier.clean").to_owned(),
            };
            let toggle = (tier != DashboardTier::NeedsHelp).then_some(tier);
            (label, toggle)
        }
        DashboardSectionKey::ProjectGroup(Some(group)) => (group, None),
        DashboardSectionKey::ProjectGroup(None) => (state.t("group.ungrouped").to_owned(), None),
        DashboardSectionKey::Flat => (state.t("dashboard.all_projects").to_owned(), None),
    };
    let label = format!(
        "{} ({}){}",
        label,
        entry_count,
        if toggle.is_some() {
            if collapsed { " +" } else { " -" }
        } else {
            ""
        }
    );
    if let Some(tier) = toggle {
        button(text(label).size(13))
            .on_press(Message::Dashboard(DashboardMessage::TierToggled(tier)))
            .width(Length::Fill)
            .into()
    } else {
        container(text(label).size(13))
            .width(Length::Fill)
            .padding([5, 8])
            .into()
    }
}

fn view_project_row<'a>(state: &'a AppState, entry: DashboardEntry<'a>) -> Element<'a, Message> {
    let project = entry.project;
    let mut identity = row![].spacing(4).align_y(Alignment::Center);
    if state.selection_mode {
        identity = identity.push(
            button(text(if state.selection.contains(&project.id) {
                "[x]"
            } else {
                "[ ]"
            }))
            .width(Length::Fixed(38.0))
            .on_press(Message::Selection(SelectionMessage::Toggled(
                project.id.clone(),
            ))),
        );
    }
    let name = button(text(project.name.as_str()).size(13)).on_press(Message::DetailPanel(
        DetailPanelMessage::Opened(project.id.clone()),
    ));
    let mut identity_details = column![name].spacing(2);
    if entry.tier == DashboardTier::NeedsHelp {
        let vcs = entry
            .status
            .map(|status| status.identity.vcs_kind.to_string())
            .unwrap_or_else(|| state.t("status.unknown").to_owned());
        identity_details = identity_details.push(text(vcs).size(11));
    }
    identity = identity.push(identity_details);

    let work_area = entry
        .status
        .and_then(|status| status.context.as_ref())
        .map(|context| context.label.as_str())
        .unwrap_or(state.t("dashboard.work_area_unknown"));
    let middle: Element<'_, Message> = match entry.tier {
        DashboardTier::NeedsHelp => text(cause_label(state, entry.cause)).size(12).into(),
        DashboardTier::InProgress => {
            let count = entry
                .relevant_count
                .map(|count| format!("{}: {}", progress_label(state, count.kind), count.value))
                .unwrap_or_else(|| state.t("plain.status.unsaved_work").to_owned());
            column![text(work_area).size(12), text(count).size(11)]
                .spacing(2)
                .into()
        }
        DashboardTier::AllSet => text(work_area).size(12).into(),
    };

    let action: Element<'_, Message> = if entry.tier == DashboardTier::NeedsHelp {
        if entry.cause == Some(DashboardCause::Conflict) {
            guided_button(
                state.t("dashboard.resolve"),
                (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                    ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
                )),
                state
                    .operation_interlock
                    .is_busy()
                    .then(|| state.t("plain.activity.busy")),
            )
        } else {
            button(text(state.t("plain.show_details")).size(12))
                .on_press(Message::DetailPanel(DetailPanelMessage::Opened(
                    project.id.clone(),
                )))
                .into()
        }
    } else {
        Space::new().width(Length::Fixed(100.0)).into()
    };

    container(
        row![
            container(identity).width(Length::FillPortion(4)),
            container(middle).width(Length::FillPortion(5)),
            action,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .into()
}

fn cause_label(state: &AppState, cause: Option<DashboardCause>) -> &'static str {
    match cause {
        Some(DashboardCause::MissingPath) => state.t("dashboard.cause.missing_path"),
        Some(DashboardCause::Conflict) => state.t("dashboard.cause.conflict"),
        Some(DashboardCause::ConflictDetectionUnavailable) => {
            state.t("dashboard.cause.conflict_detection_unavailable")
        }
        Some(DashboardCause::ReadUnavailable) => state.t("dashboard.cause.read_unavailable"),
        Some(DashboardCause::DetachedContext) => state.t("dashboard.cause.detached_context"),
        Some(DashboardCause::StatusUnknown) | None => state.t("dashboard.cause.status_unknown"),
    }
}

fn progress_label(state: &AppState, kind: ProgressKind) -> &'static str {
    match kind {
        ProgressKind::Uncommitted => state.t("dashboard.progress.uncommitted"),
        ProgressKind::Untracked => state.t("dashboard.progress.untracked"),
        ProgressKind::Ahead => state.t("dashboard.progress.ahead"),
        ProgressKind::Behind => state.t("dashboard.progress.behind"),
    }
}

fn no_matches(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text(state.t("plain.empty.no_match")).size(FONT_BODY + 2.0),
            button(text(state.t("dashboard.clear_filters")))
                .on_press(Message::Filter(FilterMessage::AllFiltersCleared)),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(220.0))
    .center(Length::Fill)
    .into()
}

fn view_confirm_remove_dialog(state: &AppState) -> Element<'_, Message> {
    let Some(dialog) = &state.confirm_remove_dialog else {
        return Space::new().into();
    };
    container(
        column![
            text(state.t("plain.remove.title")).size(FONT_BODY + 2.0),
            text(dialog.project_name.as_str()).size(FONT_BODY),
            text(state.t("plain.remove.body")).size(FONT_SMALL),
            row![
                guided_button(
                    state.t("confirm.remove_no"),
                    Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
                    None,
                ),
                guided_button(
                    state.t("plain.remove.confirm"),
                    Some(Message::Workspace(
                        WorkspaceMessage::RemoveProjectConfirmed(dialog.project_id.clone(),)
                    )),
                    None,
                ),
            ]
            .spacing(12),
        ]
        .spacing(14)
        .padding(24),
    )
    .width(Length::Fixed(380.0))
    .into()
}

fn placeholder(message: &str) -> Element<'_, Message> {
    container(text(message).size(14))
        .width(Length::Fill)
        .height(Length::Fixed(250.0))
        .center(Length::Fill)
        .into()
}
