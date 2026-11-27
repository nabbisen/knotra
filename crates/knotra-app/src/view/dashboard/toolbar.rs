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
    message::{
        DashboardMessage, FilterMessage, Message, SelectionMessage, StatusFilter, SyncMessage,
    },
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

/// RFC-074 Stage 2 D1/D5: "Get latest safely" with nothing selected.
/// Shown only while at least one project shows "Updates available" (the
/// same condition the "Updates available" chip's own count above zero
/// uses, RFC-073's calm-by-default), and never in selection mode, where
/// the selection bar already carries this same action for the selection.
/// Opens the plan for every project — the same `BulkPullRequested`
/// message the palette's own "Get latest safely" dispatches with nothing
/// selected; its own empty-`selected_ids` path already covers every
/// project (`app/sync.rs`'s `SmartPullPlanRequested`), so no new message
/// is needed here.
/// `pub(crate)`, not `pub(super)`: `view.rs` reads the exact same
/// condition this function's own call site below does, to decide whether
/// `ids::toolbar::get_latest()` belongs in the page's `reserved` list —
/// one shared predicate, so the two can never disagree about whether the
/// button is actually drawn.
pub(crate) fn get_latest_button_visible(state: &AppState) -> bool {
    !state.selection_mode && state.status_filter_count(&StatusFilter::Behind) > 0
}

fn toolbar_get_latest<'a>(state: &'a AppState, tokens: &Tokens) -> Option<Element<'a, Message>> {
    if !get_latest_button_visible(state) {
        return None;
    }
    let on_press = (!state.operation_interlock.is_busy())
        .then_some(Message::Sync(SyncMessage::BulkPullRequested));
    Some(secondary_control_maybe(
        tokens,
        state.t("plain.get_latest"),
        on_press,
        ids::toolbar::get_latest(),
    ))
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
    .id(ids::toolbar::search())
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
    let mut controls_children: Vec<Element<'_, Message>> =
        vec![arrange, search.into(), clear, select];
    // RFC-074 Stage 2: reading order after "Select"; wraps to its own
    // line like the chip row above, rather than being cut or squeezed.
    if let Some(get_latest) = toolbar_get_latest(state, tokens) {
        controls_children.push(get_latest);
    }
    let controls_row = row(controls_children)
        .spacing(6)
        .align_y(Alignment::Start)
        .wrap()
        .vertical_spacing(4);
    let mut controls = column![controls_row].spacing(5);

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
    .id(ids::toolbar::search())
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
    let mut controls_children: Vec<Element<'_, Message>> = vec![search.into(), arrange, select];
    // RFC-074 Stage 2: reading order after "Select"; may wrap to its own
    // line in the compact band, but is never cut or squeezed.
    if let Some(get_latest) = toolbar_get_latest(state, tokens) {
        controls_children.push(get_latest);
    }
    let controls_row = row(controls_children)
        .spacing(6)
        .align_y(Alignment::Start)
        .wrap()
        .vertical_spacing(4);
    let mut controls = column![controls_row].spacing(5);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::AppState;
    use knotra_vcs::{
        ConflictStatus, Project, ProjectStatus, RemoteStatus, RepositoryIdentity,
        SmartPullDisposition, VcsKind, WorkingTreeStatus, Workspace, WorkspaceStatus,
    };

    /// One project, with or without updates available, nothing else set.
    fn state_with_one_project(behind: bool) -> AppState {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("api-gateway", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id,
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    ahead: 0,
                    behind: if behind { 2 } else { 0 },
                    upstream: Some("origin/main".to_owned()),
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: ConflictStatus::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state
    }

    /// Handoff §4 row 3: "The button's visibility" — shown with one
    /// project behind, hidden with none, hidden in selection mode.
    #[test]
    fn get_latest_button_visible_with_updates_available_not_in_selection_mode() {
        let state = state_with_one_project(true);
        assert!(get_latest_button_visible(&state));
    }

    #[test]
    fn get_latest_button_hidden_with_no_project_behind() {
        let state = state_with_one_project(false);
        assert!(!get_latest_button_visible(&state));
    }

    #[test]
    fn get_latest_button_hidden_in_selection_mode_even_with_updates_available() {
        let mut state = state_with_one_project(true);
        state.selection_mode = true;
        assert!(!get_latest_button_visible(&state));
    }

    /// Handoff §4 row 4/§1: the button and the palette's own "Get latest
    /// safely" (with nothing selected) dispatch the identical message —
    /// `app/sync.rs`'s `BulkPullRequested` handler is a pure function of
    /// `state`, so the same message from either path builds the same plan.
    /// Proven by driving the real click through the real view, not by
    /// reading `toolbar_get_latest`'s own private `on_press` field.
    #[test]
    fn get_latest_button_click_dispatches_the_same_message_as_the_palette() {
        let state = state_with_one_project(true);
        let label = state.t("plain.get_latest").to_owned();

        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, crate::view::dashboard::WidthMode::Standard),
            iced_test::core::Size::new(1280.0, 800.0),
        );
        ui.click(label.as_str())
            .unwrap_or_else(|e| panic!("the button must be clickable: {e}"));
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages.len(), 1);
        assert!(matches!(
            messages[0],
            Message::Sync(SyncMessage::BulkPullRequested)
        ));

        // The palette's own entry dispatches the exact same message —
        // `state::palette::ACTIONS`' own `action.pull_selected` row,
        // unconditional on selection.
        let mut palette_state = state;
        palette_state.palette.query = "get latest safely".to_owned();
        crate::state::palette::update_results(&mut palette_state);
        let index = palette_state
            .palette
            .results
            .iter()
            .position(|entry| entry.payload == "action.pull_selected")
            .expect("the palette lists \"Get latest safely\" with no selection");
        let dispatched = crate::state::palette::dispatch_entry(&palette_state, index);
        assert!(matches!(
            dispatched,
            crate::state::palette::PaletteDispatch::Dispatched(Message::Sync(
                SyncMessage::BulkPullRequested
            ))
        ));

        // Handoff §4 row 4: "the same rows" — driven to the real plan, not
        // just the same message. The project must be covered as selected
        // (`Pull`), not `Excluded`/`Deselected` — the exact shape of a
        // real bug this stage found (`app/sync.rs`'s own `BulkPullRequested`
        // handler, see its own doc comment).
        let _ = crate::app::update(&mut palette_state, messages[0].clone());
        let _ = crate::app::update(
            &mut palette_state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        let crate::state::sync::SyncPhase::AwaitingConfirm(plan) = &palette_state.sync.phase else {
            panic!("expected a reviewed plan");
        };
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].disposition, SmartPullDisposition::Pull);
        assert_eq!(plan.entries[0].skip_reason, None);
    }

    /// Handoff §4 row 5: unavailable while busy — the button is still
    /// drawn (so it stays a stable Tab stop, not a vanishing one), but a
    /// click publishes nothing.
    #[test]
    fn get_latest_button_click_does_nothing_while_busy() {
        let mut state = state_with_one_project(true);
        state
            .operation_interlock
            .try_acquire(crate::state::OperationOwner::BulkFetch)
            .expect("sanity: the interlock must be free to acquire");
        assert!(get_latest_button_visible(&state), "still drawn while busy");

        let label = state.t("plain.get_latest").to_owned();
        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, crate::view::dashboard::WidthMode::Standard),
            iced_test::core::Size::new(1280.0, 800.0),
        );
        let _ = ui.click(label.as_str());
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "a disabled control must not publish a message on click"
        );
    }
}
