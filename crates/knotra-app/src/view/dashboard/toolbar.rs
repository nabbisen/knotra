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
//! Handoff 028 §4's "standard stays byte-for-byte unchanged" rule held
//! until RFC-073 D2, which explicitly changes the standard/wide composition
//! too: Group and Sort move into one "Arrange" control, shared by both
//! bands (`arrange_button`/`arrange_panel`) — the duplication Handoff 028
//! accepted between the two compositions' grouping/sorting construction is
//! gone with it, since both bands now call the same panel builder.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};
use knotra_ui::widget::{
    BUTTON_HEIGHT, Tokens, centered_button_label, centered_button_row, chip, icon, select, style,
};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::{DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter},
    state::{
        AppState, SelectionSummary,
        dashboard::badge_for_filter,
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
    pub const FILTER_ALL_SET: &str = "dashboard.toolbar.filter.all_set";
    /// RFC-073 D2: replaces the old separate `GROUP_SELECT`/`SORT_SELECT`
    /// targets in both bands — one "Arrange" control, opening a menu that
    /// holds both selects unchanged, is now the single Tab stop.
    pub const ARRANGE: &str = "dashboard.toolbar.arrange";
    pub const CLEAR_FILTERS: &str = "dashboard.toolbar.clear_filters";
    pub const SELECT_MODE: &str = "dashboard.toolbar.select_mode";
    /// RFC-035 Handoff 028: compact-only.
    pub const TOOLBAR_OVERFLOW: &str = "dashboard.toolbar.overflow";
}

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
    let (tone, glyph) = badge_for_filter(filter);
    let count = state.status_filter_count(filter);
    chip::filter(
        tokens,
        state.t(filter.label_key()),
        (tone, &glyph),
        count,
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
/// the five filter chips, Arrange, search, Clear filters (when rendered),
/// then the bulk-selection entry point — **before** `dashboard::focus_order`
/// appends its section/row targets (RFC-035 Handoff 022 §7.4). Without
/// this, `chip::filter`'s `is_focused` has nothing to ever be `true`, and
/// its ring is dead code.
///
/// **Clear filters is guarded on the same condition that renders it**
/// (`state.filter.is_active()`, matching `view_toolbar`'s own condition
/// exactly) — same shape as `dashboard/mod.rs`'s row-checkbox target being
/// guarded on `state.selection_mode`. An unconditional target here would
/// be a focus black hole: Tab would stop on a control that is not on
/// screen (Handoff 023 §5).
///
/// RFC-073 D2: Arrange carries `Some(ArrangeToggled)` — unlike the two
/// selects it replaces, it is a real button with a real activation message,
/// so this is a genuine keyboard-operability gain, not just a rename. The
/// grouping/sorting selects themselves keep no `FocusTarget` of their own
/// any more (see `arrange_panel`'s own doc comment for why that loses
/// nothing a plain Tab stop on them ever bought).
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

    order.push((
        FocusTarget::control(focus_target::ARRANGE),
        Some(Message::Dashboard(DashboardMessage::ArrangeToggled)),
    ));
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
/// for Clear filters). Search, then Arrange (RFC-073 D2: "it takes their
/// place in the existing overflow" — the same disclosure trigger as
/// before, now sharing `standard_focus_order`'s single `ARRANGE` target
/// and `ArrangeToggled` message instead of its own compact-only pair).
/// Clear filters and the bulk-selection entry point keep the same guards
/// `standard_focus_order` uses.
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
        FocusTarget::control(focus_target::ARRANGE),
        Some(Message::Dashboard(DashboardMessage::ArrangeToggled)),
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
    // it (`chip::filter`'s `Wrapping::None` is the other half of this).
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
/// `view_standard_toolbar` drew directly before this change. Same
/// `with_focus_ring`/`ghost` treatment as [`disclosure_button`]'s compact
/// counterpart, but with a text label (`dashboard.arrange`) rather than a
/// bare glyph, matching the rest of the standard band's controls.
fn arrange_button<'a>(state: &'a AppState, tokens: &Tokens) -> Element<'a, Message> {
    let t = tokens.clone();
    let focused = is_focused(state, focus_target::ARRANGE);
    // RFC-073 Amendment A1: a chevron follows the label, the same icon
    // helper and size as the workspace switcher's own disclosure cue
    // (`shell.rs`'s `switcher_trigger`) — down while closed, up while
    // open, so "Arrange" shows that it opens something before it is ever
    // pressed.
    let chevron = if state.dashboard_arrange_open {
        icon::chevron_up()
    } else {
        icon::chevron_down()
    };
    // RFC-073 Amendment A2: `centered_button_row` is `centered_button_label`
    // for a `Row` — an icon+text row can't go through the `Text`-only
    // helper, so this centres the row itself, the same way
    // `header_control` does. The button's own fixed height is spelled
    // `Length::Fixed(BUTTON_HEIGHT)`; `button_label_guard` now treats that
    // the same as the bare constant it always scanned for.
    button(centered_button_row(
        row![
            text(state.t("dashboard.arrange"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            icon::icon_element(&chevron),
        ]
        .spacing(6),
    ))
    .height(Length::Fixed(BUTTON_HEIGHT))
    .padding([0, 18])
    .on_press(Message::Dashboard(DashboardMessage::ArrangeToggled))
    .style(move |_theme, status| style::with_focus_ring(&t, focused, style::ghost(&t, status)))
    .into()
}

/// The grouping/sorting selects, revealed beneath [`arrange_button`] (or
/// beside compact's own `▾` trigger) while `state.dashboard_arrange_open`.
/// RFC-073 D2 unified what Handoff 028 had kept as two separate
/// constructions (standard's always-visible row and compact's
/// `compact_selectors_row`) — the panel's content and behaviour are
/// identical in both bands now, so one function serves both.
///
/// Neither select carries a `FocusTarget` of its own: Tab now stops once,
/// on `arrange_button`/its compact equivalent (D2's own focus-order
/// change), and opening either select's own menu was already a
/// pointer-only interaction before this change — the `is_focused: false`
/// here is an explicit, honest "unwired", not a regression from a working
/// ring to a dead one.
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
        select::pick_list(
            tokens,
            grouping_options,
            Some(state.config.dashboard_grouping),
            false,
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
    // RFC-071 Amendment A3 D14: a field label — `body`.
    let sorting = row![
        text(state.t("dashboard.sorting"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        select::pick_list(
            tokens,
            sorting_options,
            Some(state.config.dashboard_sort),
            false,
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

/// Compact (RFC-035 R8, Handoff 028 §4): chips beyond
/// [`COMPACT_VISIBLE_CHIP_COUNT`] behind a `⋯` overflow; Arrange (RFC-073
/// D2 — "it takes their place in the existing overflow") behind a `▾`
/// disclosure. Both are plain layout pushes (an extra row appended when
/// the corresponding `AppState` bool is set), not an overlay/menu widget —
/// the same shape a `guided_button`'s reason text or a compact row's
/// second line already uses, deliberately not the "substantial widget
/// with its own focus/keyboard/positioning behaviour" Handoff 027 §7 ruled
/// out.
fn view_compact_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    let overflow_button = disclosure_button(
        tokens,
        "⋯",
        Message::Dashboard(DashboardMessage::ToolbarOverflowToggled),
        is_focused(state, focus_target::TOOLBAR_OVERFLOW),
    );

    // Task 110 §1: the visible chips and the "⋯" trigger that follows them
    // are one wrapping row, not a fixed-width row — the same `.wrap()`
    // `view_standard_toolbar`'s own five-chip row already uses. At the
    // zoom and width an older reader can choose (Review 280 §2.1: zoom
    // 150%, an 800px window, logical ~533), a plain row let the last
    // chip's own label clamp narrower than its count digit had room for,
    // and pushed "⋯" off the visible window entirely; wrapped, the chips
    // (and "⋯", if it too does not fit) continue onto a second line
    // instead, so nothing is ever cut.
    let mut visible_elements: Vec<Element<'_, Message>> = FILTERS[..COMPACT_VISIBLE_CHIP_COUNT]
        .iter()
        .map(|filter| chip_element(state, tokens, filter))
        .collect();
    visible_elements.push(overflow_button);

    let mut chip_area =
        column![row(visible_elements).spacing(4).wrap().vertical_spacing(4)].spacing(4);

    if state.dashboard_toolbar_overflow_open {
        let overflow_chips = row(FILTERS[COMPACT_VISIBLE_CHIP_COUNT..]
            .iter()
            .map(|filter| chip_element(state, tokens, filter))
            .collect::<Vec<_>>())
        .spacing(4);
        chip_area = chip_area.push(overflow_chips);
    }

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

    let arrange_trigger = disclosure_button(
        tokens,
        "▾",
        Message::Dashboard(DashboardMessage::ArrangeToggled),
        is_focused(state, focus_target::ARRANGE),
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

    // RFC-073 Amendment A2: top-aligned, same reason as the standard band.
    let mut controls = column![
        row![search, arrange_trigger, select]
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

    let btn = button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 18])
    .on_press_maybe(on_press)
    .style(move |_theme, status| {
        style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
    });

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
    // RFC-071 Amendment A3 D14: a button label — `body`.
    // RFC-073 D8: `BUTTON_HEIGHT`, matching Arrange/Search/Select in the
    // same controls row.
    button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
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
    button(centered_button_label(
        text(glyph)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
    .on_press(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style::ghost(&t, status)))
    .into()
}
