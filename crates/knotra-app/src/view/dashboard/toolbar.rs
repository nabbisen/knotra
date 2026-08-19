//! Dashboard toolbar: status filter chips, grouping/sorting selectors,
//! search, and the bulk-selection entry point.
//!
//! Migrated onto the RFC-035 Stage 1 primitives (`chip::filter`,
//! `select::pick_list`) in Stage 2 commit 2. Rebuilt as one region per the
//! External Design sketch: a chip row, then a single controls row — the
//! old four-stacked-elements layout and its `Space::new().width(Fill)`
//! spacer (which marooned the bulk-selection button at the far edge) are
//! gone.
//!
//! RFC-035 Stage 4/Handoff 028 added a second, compact composition
//! (`view_compact_toolbar`/`compact_focus_order`): chips beyond the first
//! three behind a `⋯` overflow, both selectors behind a `▾` disclosure.
//! **Standard/wide (`view_standard_toolbar`/`standard_focus_order`) are
//! left byte-for-byte as Stage 2 wrote them** — Handoff 028 §4 requires
//! those compositions stay unchanged, so the compact path accepts a little
//! duplication (the grouping/sorting select construction) rather than
//! factoring something standard would then depend on too.

use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::{BUTTON_HEIGHT, Tokens, chip, select, style};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter},
    state::{
        AppState, SelectionSummary,
        focus::{FocusOrder, FocusTarget},
    },
};

use super::width_mode::WidthMode;

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
    /// RFC-035 Handoff 028: compact-only controls.
    pub const TOOLBAR_OVERFLOW: &str = "dashboard.toolbar.overflow";
    pub const TOOLBAR_SELECTORS: &str = "dashboard.toolbar.selectors_disclosure";
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

/// Handoff 028 Ruling 6.3: compact shows these first chips directly; the
/// rest go behind the `⋯` overflow. A fixed split, stated as a constant
/// rather than derived — iced has no measure-and-overflow layout
/// primitive. Matches the External Design sketch, which names Needs help,
/// Unsaved work, and Updates available (`FILTERS`' own first three). If the
/// compact band later proves to fit more, this is a one-constant change.
const COMPACT_VISIBLE_CHIP_COUNT: usize = 3;

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

/// A single filter chip, built once and shared between the standard chip
/// row and the compact composition's visible/overflow chip groups — same
/// discipline as `row.rs`'s shared `selection_checkbox`/`name_button`.
fn chip_element<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    filter: &StatusFilter,
) -> Element<'a, Message> {
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
}

/// RFC-035 R15/Handoff 030 §4.1: the Select control's disabled reason must
/// be true to its actual cause. "No projects match this view" implies a
/// filter excluded everything, which is false when nothing is
/// **registered** at all (`066` observation 1) — that case gets its own
/// wording, matching the empty state's own copy, rather than reusing a
/// sentence about filtering. Shared between the standard and compact
/// toolbars so the two cannot pick different wording for the same cause.
fn select_mode_reason(state: &AppState, summary: &SelectionSummary) -> Option<&'static str> {
    if !summary.visible_ids.is_empty() {
        return None;
    }
    let no_projects_registered = state
        .workspace
        .as_ref()
        .is_none_or(|workspace| workspace.projects.is_empty());
    Some(if no_projects_registered {
        state.t("plain.selection.no_projects_registered")
    } else {
        state.t("plain.selection.no_visible_projects")
    })
}

pub(super) fn focus_order(state: &AppState, mode: WidthMode) -> FocusOrder<Message> {
    match mode {
        WidthMode::Compact => compact_focus_order(state),
        WidthMode::Standard | WidthMode::Wide => standard_focus_order(state),
    }
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
fn standard_focus_order(state: &AppState) -> FocusOrder<Message> {
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

/// Compact's order (RFC-035 R8, Handoff 028 §5): the first
/// [`COMPACT_VISIBLE_CHIP_COUNT`] chips, then the overflow control, then —
/// **only while `state.dashboard_toolbar_overflow_open`** — the remaining
/// chips, so a chip behind the overflow is reachable exactly when it is on
/// screen (the same conditional shape `standard_focus_order` already uses
/// for Clear filters). Search, then the selectors disclosure, then —
/// **only while `state.dashboard_toolbar_selectors_open`** — the grouping
/// and sorting selects. Clear filters and the bulk-selection entry point
/// keep the same guards `standard_focus_order` uses.
fn compact_focus_order(state: &AppState) -> FocusOrder<Message> {
    let mut order: FocusOrder<Message> = FILTERS[..COMPACT_VISIBLE_CHIP_COUNT]
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

    order.push((
        FocusTarget::control(focus_target::TOOLBAR_OVERFLOW),
        Some(Message::Dashboard(DashboardMessage::ToolbarOverflowToggled)),
    ));

    if state.dashboard_toolbar_overflow_open {
        order.extend(FILTERS[COMPACT_VISIBLE_CHIP_COUNT..].iter().map(|filter| {
            (
                FocusTarget::control(filter_focus_key(filter)),
                Some(Message::Filter(FilterMessage::StatusFilterToggled(
                    filter.clone(),
                ))),
            )
        }));
    }

    order.push((
        FocusTarget::text_input(knotra_ui::widget::focus_id::SEARCH.clone()),
        None,
    ));

    order.push((
        FocusTarget::control(focus_target::TOOLBAR_SELECTORS),
        Some(Message::Dashboard(
            DashboardMessage::ToolbarSelectorsToggled,
        )),
    ));

    if state.dashboard_toolbar_selectors_open {
        order.push((FocusTarget::control(focus_target::GROUP_SELECT), None));
        order.push((FocusTarget::control(focus_target::SORT_SELECT), None));
    }

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

pub(super) fn view_toolbar(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    match mode {
        WidthMode::Compact => view_compact_toolbar(state),
        WidthMode::Standard | WidthMode::Wide => view_standard_toolbar(state),
    }
}

fn view_standard_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    let chips = row(FILTERS
        .iter()
        .map(|filter| chip_element(state, tokens, filter))
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
        text(state.t("dashboard.grouping"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
        text(state.t("dashboard.sorting"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
        select_mode_reason(state, &summary),
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

/// Compact (RFC-035 R8, Handoff 028 §4): chips beyond
/// [`COMPACT_VISIBLE_CHIP_COUNT`] behind a `⋯` overflow; both selectors
/// behind a `▾` disclosure. Both are plain layout pushes (an extra row
/// appended when the corresponding `AppState` bool is set), not an
/// overlay/menu widget — the same shape a `guided_button`'s reason text or
/// a compact row's second line already uses, deliberately not the
/// "substantial widget with its own focus/keyboard/positioning behaviour"
/// Handoff 027 §7 ruled out.
fn view_compact_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    let visible_chips = row(FILTERS[..COMPACT_VISIBLE_CHIP_COUNT]
        .iter()
        .map(|filter| chip_element(state, tokens, filter))
        .collect::<Vec<_>>())
    .spacing(4);

    let overflow_button = disclosure_button(
        tokens,
        "⋯",
        Message::Dashboard(DashboardMessage::ToolbarOverflowToggled),
        is_focused(state, focus_target::TOOLBAR_OVERFLOW),
    );

    let mut chip_area = column![
        row![visible_chips, overflow_button]
            .spacing(4)
            .align_y(Alignment::Center)
    ]
    .spacing(4);

    if state.dashboard_toolbar_overflow_open {
        let overflow_chips = row(FILTERS[COMPACT_VISIBLE_CHIP_COUNT..]
            .iter()
            .map(|filter| chip_element(state, tokens, filter))
            .collect::<Vec<_>>())
        .spacing(4);
        chip_area = chip_area.push(overflow_chips);
    }

    let search = text_input(
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .id(knotra_ui::widget::focus_id::SEARCH.clone())
    .on_input(|value| Message::Filter(FilterMessage::SearchChanged(value)))
    .width(Length::Fixed(220.0));

    let selectors_button = disclosure_button(
        tokens,
        "▾",
        Message::Dashboard(DashboardMessage::ToolbarSelectorsToggled),
        is_focused(state, focus_target::TOOLBAR_SELECTORS),
    );

    let summary = state.selection_summary();
    let select_message = (!summary.visible_ids.is_empty())
        .then_some(Message::Selection(SelectionMessage::ModeEntered));
    let select = select_mode_button(
        tokens,
        state.t("plain.selection.enter"),
        select_message,
        select_mode_reason(state, &summary),
        is_focused(state, focus_target::SELECT_MODE),
    );

    let mut controls = column![
        row![search, selectors_button, select]
            .spacing(6)
            .align_y(Alignment::Center)
    ]
    .spacing(5);

    if state.dashboard_toolbar_selectors_open {
        controls = controls.push(compact_selectors_row(state, tokens));
    }

    if state.filter.is_active() {
        controls = controls.push(clear_filters_button(
            tokens,
            state.t("dashboard.clear_filters"),
            is_focused(state, focus_target::CLEAR_FILTERS),
        ));
    }

    container(column![chip_area, controls].spacing(5))
        .width(Length::Fill)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .into()
}

/// The grouping/sorting selects, revealed beneath the compact controls row
/// while `state.dashboard_toolbar_selectors_open`. Deliberately a second
/// copy of `view_standard_toolbar`'s equivalent construction rather than a
/// shared helper — see this module's own doc comment for why.
///
/// The selects themselves keep whatever keyboard reachability they have
/// today (Tab reaches them, ring renders; opening the menu still needs a
/// pointer — `101` Finding 2, an iced 0.14 constraint routed to its own RFC
/// by `104`, not this handoff's to fix).
fn compact_selectors_row<'a>(state: &'a AppState, tokens: &Tokens) -> Element<'a, Message> {
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
        text(state.t("dashboard.grouping"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
        text(state.t("dashboard.sorting"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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

    row![grouping, sorting]
        .spacing(6)
        .align_y(Alignment::Center)
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

    let btn = button(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    )
    .height(BUTTON_HEIGHT)
    .padding([0, 18])
    .on_press_maybe(on_press)
    .style(move |_theme, status| {
        style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
    });

    match reason {
        Some(r) if show_reason => column![
            btn,
            text(r)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
        ]
        .spacing(6)
        .into(),
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
    button(
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Filter(FilterMessage::AllFiltersCleared))
    .style(move |_theme, status| {
        style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
    })
    .into()
}

/// The compact toolbar's `⋯`/`▾` disclosure triggers — a plain glyph
/// button (not `icon_button_maybe`: these aren't lucide icons, they're the
/// literal glyphs the External Design sketch draws), `ghost`-styled with a
/// working focus ring, same shape as `select_mode_button`/
/// `clear_filters_button`.
fn disclosure_button<'a>(
    tokens: &Tokens,
    glyph: &'a str,
    on_press: Message,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(
        text(glyph)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    )
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
    .on_press(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style::ghost(&t, status)))
    .into()
}
