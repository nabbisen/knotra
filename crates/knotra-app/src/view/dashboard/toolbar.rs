//! Dashboard toolbar: status filter chips, grouping/sorting selectors,
//! search, and the bulk-selection entry point.

use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::guided_button;

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter},
    state::AppState,
};

pub(super) fn view_toolbar(state: &AppState) -> Element<'_, Message> {
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
