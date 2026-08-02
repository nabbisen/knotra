//! Dashboard toolbar: status filter chips, grouping/sorting selectors,
//! search, and the bulk-selection entry point.
//!
//! Migrated onto the RFC-035 Stage 1 primitives (`chip::filter`,
//! `select::pick_list`) in Stage 2 commit 2. Rebuilt as one region per the
//! External Design sketch: a chip row, then a single controls row — the
//! old four-stacked-elements layout and its `Space::new().width(Fill)`
//! spacer (which marooned the bulk-selection button at the far edge) are
//! gone.

use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, Tokens, chip, select, style};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter},
    state::{
        AppState,
        focus::{FocusOrder, FocusTarget},
    },
};

/// Stable keys for the toolbar's `FocusTarget`s (RFC-036), shared between
/// [`focus_order`] (Tab/Shift-Tab + activation) and [`view_toolbar`] (which
/// control currently draws the ring). Kept as one list so the two cannot
/// drift — same discipline as `shell.rs`'s and `workspace_manager.rs`'s own
/// `focus_target` modules.
mod focus_target {
    pub const FILTER_NEEDS_HELP: &str = "dashboard.toolbar.filter.needs_help";
    pub const FILTER_DIRTY: &str = "dashboard.toolbar.filter.dirty";
    pub const FILTER_BEHIND: &str = "dashboard.toolbar.filter.behind";
    pub const FILTER_AHEAD: &str = "dashboard.toolbar.filter.ahead";
    pub const FILTER_CONFLICT: &str = "dashboard.toolbar.filter.conflict";
    pub const FILTER_ALL_SET: &str = "dashboard.toolbar.filter.all_set";
    pub const GROUP_SELECT: &str = "dashboard.toolbar.group";
    pub const SORT_SELECT: &str = "dashboard.toolbar.sort";
    pub const CLEAR_FILTERS: &str = "dashboard.toolbar.clear_filters";
    pub const SELECT_MODE: &str = "dashboard.toolbar.select_mode";
}

/// The six status filters, in the order the chip row renders them.
const FILTERS: [StatusFilter; 6] = [
    StatusFilter::NeedsHelp,
    StatusFilter::Dirty,
    StatusFilter::Behind,
    StatusFilter::Ahead,
    StatusFilter::Conflict,
    StatusFilter::AllSet,
];

fn filter_focus_key(filter: &StatusFilter) -> &'static str {
    match filter {
        StatusFilter::NeedsHelp => focus_target::FILTER_NEEDS_HELP,
        StatusFilter::Dirty => focus_target::FILTER_DIRTY,
        StatusFilter::Behind => focus_target::FILTER_BEHIND,
        StatusFilter::Ahead => focus_target::FILTER_AHEAD,
        StatusFilter::Conflict => focus_target::FILTER_CONFLICT,
        StatusFilter::AllSet => focus_target::FILTER_ALL_SET,
    }
}

/// Whether the toolbar control keyed `key` currently draws the RFC-036
/// focus ring — plain equality against `state.dashboard_focus`, same
/// pattern as `shell.rs`'s/`workspace_manager.rs`'s own `is_focused`.
fn is_focused(state: &AppState, key: &'static str) -> bool {
    state.dashboard_focus.as_ref() == Some(&FocusTarget::control(key))
}

/// The toolbar's Tab/Shift-Tab focus targets (RFC-036 R2), in visual order:
/// the six filter chips, the grouping select, the sorting select, search,
/// Clear filters (when rendered), then the bulk-selection entry point —
/// **before** `dashboard::focus_order` appends its section/row targets
/// (RFC-035 Handoff 022 §7.4). Without this, `chip::filter`'s `is_focused`
/// has nothing to ever be `true`, and its ring is dead code.
///
/// **Clear filters is guarded on the same condition that renders it**
/// (`state.filter.is_active()`, matching `view_toolbar`'s own condition
/// exactly) — same shape as `dashboard/mod.rs`'s row-checkbox target being
/// guarded on `state.selection_mode`. An unconditional target here would
/// be a focus black hole: Tab would stop on a control that is not on
/// screen (Handoff 023 §5).
///
/// The grouping/sorting selects pair with `None`: same as `search`'s
/// `text_input` entry, there is no single message that "activates" a
/// select the way a click activates a button — opening the menu is the
/// widget's own interaction, not something this handoff invents a message
/// for (§6). Tab reaches them and the ring renders; opening still needs a
/// pointer. Recorded as a known limitation, not silently claimed as full
/// keyboard operability.
pub(super) fn focus_order(state: &AppState) -> FocusOrder<Message> {
    let mut order: FocusOrder<Message> = FILTERS
        .iter()
        .map(|filter| {
            (
                FocusTarget::control(filter_focus_key(filter)),
                Some(Message::Filter(FilterMessage::StatusFilterToggled(
                    filter.clone(),
                ))),
            )
        })
        .collect();

    order.push((FocusTarget::control(focus_target::GROUP_SELECT), None));
    order.push((FocusTarget::control(focus_target::SORT_SELECT), None));
    order.push((
        FocusTarget::text_input(knotra_ui::widget::focus_id::SEARCH.clone()),
        None,
    ));

    if state.filter.is_active() {
        order.push((
            FocusTarget::control(focus_target::CLEAR_FILTERS),
            Some(Message::Filter(FilterMessage::AllFiltersCleared)),
        ));
    }

    let summary = state.selection_summary();
    let select_message = (!summary.visible_ids.is_empty())
        .then_some(Message::Selection(SelectionMessage::ModeEntered));
    order.push((
        FocusTarget::control(focus_target::SELECT_MODE),
        select_message,
    ));

    order
}

pub(super) fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    let chips = row(FILTERS
        .iter()
        .map(|filter| {
            let active = state.filter.has_status_filter(filter);
            chip::filter(
                tokens,
                state.t(filter.label_key()),
                active,
                is_focused(state, filter_focus_key(filter)),
                Some(Message::Filter(FilterMessage::StatusFilterToggled(
                    filter.clone(),
                ))),
            )
        })
        .collect::<Vec<_>>())
    .spacing(4);

    let grouping_options = vec![
        (
            DashboardGrouping::Attention,
            state.t("dashboard.grouping.attention").to_owned(),
        ),
        (
            DashboardGrouping::ProjectGroup,
            state.t("dashboard.grouping.project_group").to_owned(),
        ),
        (
            DashboardGrouping::None,
            state.t("dashboard.grouping.none").to_owned(),
        ),
    ];
    let grouping = row![
        text(state.t("dashboard.grouping")).size(12),
        select::pick_list(
            tokens,
            grouping_options,
            Some(state.config.dashboard_grouping),
            is_focused(state, focus_target::GROUP_SELECT),
            |value| Message::Dashboard(DashboardMessage::GroupingChanged(value)),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let sorting_options = vec![
        (
            DashboardSort::Recommended,
            state.t("dashboard.sorting.recommended").to_owned(),
        ),
        (
            DashboardSort::NameAscending,
            state.t("dashboard.sorting.name").to_owned(),
        ),
    ];
    let sorting = row![
        text(state.t("dashboard.sorting")).size(12),
        select::pick_list(
            tokens,
            sorting_options,
            Some(state.config.dashboard_sort),
            is_focused(state, focus_target::SORT_SELECT),
            |value| Message::Dashboard(DashboardMessage::SortChanged(value)),
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

    let clear: Element<'_, Message> = if state.filter.is_active() {
        clear_filters_button(
            tokens,
            state.t("dashboard.clear_filters"),
            is_focused(state, focus_target::CLEAR_FILTERS),
        )
    } else {
        Space::new().width(Length::Shrink).into()
    };

    let summary = state.selection_summary();
    let select_message = (!summary.visible_ids.is_empty())
        .then_some(Message::Selection(SelectionMessage::ModeEntered));
    let select = select_mode_button(
        tokens,
        state.t("plain.selection.enter"),
        select_message,
        summary
            .visible_ids
            .is_empty()
            .then(|| state.t("plain.selection.no_visible_projects")),
        is_focused(state, focus_target::SELECT_MODE),
    );

    container(
        column![
            chips,
            row![grouping, sorting, search, clear, select]
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

/// The bulk-selection entry point, token-styled with a working focus ring
/// (RFC-036 R2/D7) — `guided_button` cannot carry `is_focused` (RFC-034 R7
/// keeps it stable for rows until Stage 3), so this reproduces its
/// disabled-reason-text composition on top of a real `secondary` button
/// style instead, via `style::with_focus_ring` directly (the same pattern
/// `workspace_manager.rs`'s dialog buttons use), so the control this
/// handoff explicitly adds to `focus_order` (§7.4) actually renders a
/// ring rather than becoming a second instance of the gap `chip` had
/// before Handoff 020.
fn select_mode_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    reason: Option<&'a str>,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let show_reason = on_press.is_none();

    let btn = button(text(label).size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 18])
        .on_press_maybe(on_press)
        .style(move |_theme, status| {
            style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
        });

    match reason {
        Some(r) if show_reason => column![btn, text(r).size(FONT_SMALL)].spacing(6).into(),
        _ => btn.into(),
    }
}

/// Clear filters, token-styled with a working focus ring — the same
/// `style::secondary` + `with_focus_ring` treatment as
/// [`select_mode_button`] (Handoff 023 §6), since this control gained a
/// real `focus_order` entry and would otherwise be reachable and operable
/// but invisible.
fn clear_filters_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(text(label).size(12))
        .on_press(Message::Filter(FilterMessage::AllFiltersCleared))
        .style(move |_theme, status| {
            style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
        })
        .into()
}
