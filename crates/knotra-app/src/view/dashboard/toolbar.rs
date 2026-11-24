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
//! (`view_compact_toolbar`): chips beyond the first three behind a `⋯`
//! overflow, both selectors behind a `▾` disclosure. Handoff 028 §4's
//! "standard stays byte-for-byte unchanged" rule held until RFC-073 D2,
//! which explicitly changes the standard/wide composition too: Group and
//! Sort move into one "Arrange" control, shared by both bands
//! (`arrange_button`/`arrange_panel`).
//!
//! **RFC-075 stage 2 §3/§4**: every control here is a real snora Tab stop
//! now (`view::ids::toolbar`), and `focus_order` is gone — reading order
//! is the layout itself (D2). The Arrange panel's two selects are
//! `select_control`, closing RFC-075's own problem #2 (a select unreachable
//! by keyboard); their open state lives in `AppState::arrange_select_open`
//! (see that field's own doc comment).

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::{
    Tokens, centered_button_row, chip::filter_control, icon, secondary_control,
    secondary_control_maybe, select::select_control,
};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter},
    state::{AppState, ArrangeSelect, SelectionSummary, dashboard::badge_for_filter},
    view::ids,
};

use super::width_mode::WidthMode;

/// The five status filters, in the order the chip row renders them.
/// RFC-073 D5 removed the sixth, `Conflict` ("Needs a choice") — every
/// project needing a resolution choice already needs help, and its row
/// still says why (`DashboardCause::Conflict`'s own cause text).
const FILTERS: [StatusFilter; 5] = [
    StatusFilter::NeedsHelp,
    StatusFilter::Dirty,
    StatusFilter::Behind,
    StatusFilter::Ahead,
    StatusFilter::AllSet,
];

fn filter_id(filter: &StatusFilter) -> iced::widget::Id {
    match filter {
        StatusFilter::NeedsHelp => ids::toolbar::filter_needs_help(),
        StatusFilter::Dirty => ids::toolbar::filter_dirty(),
        StatusFilter::Behind => ids::toolbar::filter_behind(),
        StatusFilter::Ahead => ids::toolbar::filter_ahead(),
        StatusFilter::AllSet => ids::toolbar::filter_all_set(),
    }
}

/// A single filter chip, built once and shared between the standard chip
/// row and the compact composition's chip row — same discipline as
/// `row.rs`'s shared `selection_checkbox`/`name_button`.
fn chip_element<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    filter: &StatusFilter,
) -> Element<'a, Message> {
    let active = state.filter.has_status_filter(filter);
    let (tone, glyph) = badge_for_filter(filter);
    let count = state.status_filter_count(filter);
    filter_control(
        tokens,
        state.t(filter.label_key()),
        (tone, &glyph),
        count,
        active,
        Message::Filter(FilterMessage::StatusFilterToggled(filter.clone())),
        filter_id(filter),
    )
}

/// RFC-035 R15/Handoff 030 §4.1: the Select control's disabled reason must
/// be true to its actual cause — `AppState::visible_projects_unavailable_reason_key`
/// (shared with the command palette's own equivalent check, so the two
/// cannot pick different wording for the same cause) distinguishes nothing
/// registered, every match sitting in a closed group (RFC-073 D16 W3), and
/// the filter itself matching nothing. Task 119: the shared method returns a
/// key now, not text, so this is the one place that translates it for the
/// toolbar's own display — `select_mode_button` renders its `reason`
/// directly, it does not call `t()` a second time.
fn select_mode_reason(state: &AppState, summary: &SelectionSummary) -> Option<&'static str> {
    if !summary.visible_ids.is_empty() {
        return None;
    }
    state
        .visible_projects_unavailable_reason_key()
        .map(|key| state.t(key))
}

pub(super) fn view_toolbar(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    match mode {
        WidthMode::Compact => view_compact_toolbar(state),
        WidthMode::Standard | WidthMode::Wide => view_standard_toolbar(state),
    }
}

fn view_standard_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    // RFC-071 Amendment A6 §1: the chip row wraps onto a second line when
    // it does not fit, rather than letting a chip's own label wrap inside
    // it (`chip::filter_control`'s `Wrapping::None` is the other half of
    // this).
    let chips = row(FILTERS
        .iter()
        .map(|filter| chip_element(state, tokens, filter))
        .collect::<Vec<_>>())
    .spacing(4)
    .wrap()
    .vertical_spacing(4);

    // RFC-073 D2: Group and Sort move into one "Arrange" control.
    let arrange = arrange_button(state, tokens);

    // RFC-073 handoff Amendment A3 §2: `BUTTON_HEIGHT` through the shared
    // builder — this was the field the capture caught sitting short beside
    // 44px controls.
    let search = knotra_ui::widget::styled_text_input(
        tokens,
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .id(knotra_ui::widget::focus_id::SEARCH.clone())
    .on_input(|value| Message::Filter(FilterMessage::SearchChanged(value)))
    .width(Length::Fixed(220.0));

    let clear: Element<'_, Message> = if state.filter.is_active() {
        clear_filters_button(tokens, state.t("dashboard.clear_filters"))
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
    );

    // RFC-073 Amendment A2: aligned to the top (`Start`, not `Center`), so
    // "Select"'s own reason line (rendered beneath it when disabled) grows
    // the row's height without dropping "Arrange" and Search out of line
    // with "Select"'s own top edge.
    let mut controls = column![
        row![arrange, search, clear, select]
            .spacing(6)
            .align_y(Alignment::Start)
    ]
    .spacing(5);

    if state.dashboard_arrange_open {
        controls = controls.push(arrange_panel(state, tokens));
    }

    container(column![chips, controls].spacing(5))
        .width(Length::Fill)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .into()
}

/// RFC-073 D2: the single control, beside Search, that opens
/// [`arrange_panel`] — replacing the two always-visible selects
/// `view_standard_toolbar` drew directly before this change.
/// RFC-073 Amendment A6 §1: now also `view_compact_toolbar`'s own Arrange
/// control, verbatim — Compact no longer builds a bare-glyph disclosure
/// trigger of its own. RFC-075 stage 2 §3: on `snora::design::control`.
fn arrange_button<'a>(state: &'a AppState, tokens: &Tokens) -> Element<'a, Message> {
    let t = tokens.clone();
    // RFC-073 Amendment A1: a chevron follows the label, the same icon
    // helper and size as the workspace switcher's own disclosure cue —
    // down while closed, up while open, so "Arrange" shows that it opens
    // something before it is ever pressed.
    let chevron = if state.dashboard_arrange_open {
        icon::chevron_up()
    } else {
        icon::chevron_down()
    };
    snora::design::control(
        tokens,
        centered_button_row(
            row![
                text(state.t("dashboard.arrange"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                icon::icon_element(&chevron),
            ]
            .spacing(6),
        ),
    )
    .height(Length::Fixed(knotra_ui::widget::BUTTON_HEIGHT))
    .id(ids::toolbar::arrange())
    .padding([0.0, 18.0])
    .on_press(Message::Dashboard(DashboardMessage::ArrangeToggled))
    .style(move |_theme, status| knotra_ui::widget::style::ghost(&t, status))
    .into()
}

/// The grouping/sorting selects, revealed beneath [`arrange_button`] (or
/// beside compact's own trigger) while `state.dashboard_arrange_open`.
/// RFC-075 stage 2 §3: each is now `select_control`, reachable and
/// operable from the keyboard — the exact gap RFC-075 exists to close
/// (problem #2). Each carries its own `.id(...)` now; at most one is open
/// at a time, tracked in `AppState::arrange_select_open`.
fn arrange_panel<'a>(state: &'a AppState, tokens: &Tokens) -> Element<'a, Message> {
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
    // RFC-071 Amendment A3 D14: a field label — `body`.
    let grouping = row![
        text(state.t("dashboard.grouping"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        select_control(
            tokens,
            grouping_options,
            Some(state.config.dashboard_grouping),
            state.arrange_select_open == Some(ArrangeSelect::Group),
            |value| Message::Dashboard(DashboardMessage::GroupingChanged(value)),
            |opened| Message::Dashboard(DashboardMessage::ArrangeSelectOpenChanged(
                ArrangeSelect::Group,
                opened
            )),
            ids::toolbar::arrange_group(),
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
    // RFC-071 Amendment A3 D14: a field label — `body`.
    let sorting = row![
        text(state.t("dashboard.sorting"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        select_control(
            tokens,
            sorting_options,
            Some(state.config.dashboard_sort),
            state.arrange_select_open == Some(ArrangeSelect::Sort),
            |value| Message::Dashboard(DashboardMessage::SortChanged(value)),
            |opened| Message::Dashboard(DashboardMessage::ArrangeSelectOpenChanged(
                ArrangeSelect::Sort,
                opened
            )),
            ids::toolbar::arrange_sort(),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    row![grouping, sorting]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}

/// Compact (RFC-035 R8, Handoff 028 §4; RFC-073 Amendment A6 §1). Since
/// Task 110 the chip row wraps onto a second line rather than clamping or
/// running off-screen, so Compact shows every status filter directly, in
/// the same wrapping row Standard uses, and reuses [`arrange_button`]
/// byte-for-byte rather than a glyph-only disclosure trigger of its own.
fn view_compact_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    // Task 110 §1: wraps onto a second line rather than clamping — the
    // same `.wrap()` `view_standard_toolbar`'s own five-chip row uses.
    let chips = row(FILTERS
        .iter()
        .map(|filter| chip_element(state, tokens, filter))
        .collect::<Vec<_>>())
    .spacing(4)
    .wrap()
    .vertical_spacing(4);

    // RFC-073 handoff Amendment A3 §2: same shared builder as the standard
    // band's search field above.
    let search = knotra_ui::widget::styled_text_input(
        tokens,
        state.t("dashboard.search_placeholder"),
        &state.filter.search_text,
    )
    .id(knotra_ui::widget::focus_id::SEARCH.clone())
    .on_input(|value| Message::Filter(FilterMessage::SearchChanged(value)))
    .width(Length::Fixed(220.0));

    let arrange = arrange_button(state, tokens);

    let summary = state.selection_summary();
    let select_message = (!summary.visible_ids.is_empty())
        .then_some(Message::Selection(SelectionMessage::ModeEntered));
    let select = select_mode_button(
        tokens,
        state.t("plain.selection.enter"),
        select_message,
        select_mode_reason(state, &summary),
    );

    // RFC-073 Amendment A2: top-aligned, same reason as the standard band.
    let mut controls = column![
        row![search, arrange, select]
            .spacing(6)
            .align_y(Alignment::Start)
    ]
    .spacing(5);

    if state.dashboard_arrange_open {
        controls = controls.push(arrange_panel(state, tokens));
    }

    if state.filter.is_active() {
        controls = controls.push(clear_filters_button(
            tokens,
            state.t("dashboard.clear_filters"),
        ));
    }

    container(column![chips, controls].spacing(5))
        .width(Length::Fill)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .into()
}

/// The bulk-selection entry point, token-styled — RFC-075 stage 2 §3: on
/// `secondary_control_maybe`, reproducing `reasoned`'s disabled-reason-
/// beneath composition (the ring itself is now `Control`'s own).
fn select_mode_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    reason: Option<&'a str>,
) -> Element<'a, Message> {
    let show_reason = on_press.is_none();
    let btn = secondary_control_maybe(tokens, label, on_press, ids::toolbar::select_mode());

    match reason {
        // RFC-071 Amendment A3 D14: a reason — `body`.
        Some(r) if show_reason => column![
            btn,
            text(r)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ]
        .spacing(6)
        .into(),
        _ => btn,
    }
}

/// Clear filters, token-styled — RFC-075 stage 2 §3: on `secondary_control`
/// directly, since it always carries a real `on_press` while it is drawn
/// at all.
fn clear_filters_button<'a>(tokens: &Tokens, label: &'a str) -> Element<'a, Message> {
    secondary_control(
        tokens,
        label,
        Message::Filter(FilterMessage::AllFiltersCleared),
        ids::toolbar::clear_filters(),
    )
}
