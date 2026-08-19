//! A single dashboard project row: identity, status/progress summary, and
//! the tier-specific action.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, Tokens, checkbox, style};
use knotra_vcs::model::project::Project;

use crate::{
    message::{ConflictOpsMessage, DetailPanelMessage, Message, SelectionMessage},
    state::{
        AppState,
        dashboard::{DashboardCause, DashboardEntry, DashboardTier, ProgressKind},
        focus::FocusTarget,
    },
};

use super::width_mode::WidthMode;

/// Bounded row-track widths (R9), sized for the 1000-1279px standard-width
/// breakpoint (`.git-exclude/reviewed/062-...md` "Standard, 1000-1279px:
/// bounded three-track project rows"): identity, then status/progress, then
/// the tier action, each a fixed width rather than `FillPortion` so the
/// three columns line up down the entire list. Measured against a 1000px
/// window (`SIDEBAR_WIDTH` 180px leaves ~820px of content, minus the
/// dashboard body's 24px horizontal padding, the row's own 16px padding, and
/// 16px of inter-column spacing, leaves ~764px before a scrollbar) — the sum
/// here (700px of tracks + 16px of gaps = 716px) keeps a margin at that
/// floor. Stage 4 centres and may revisit these; they must not grow
/// indefinitely at wide widths (the audit's explicit warning), so this stage
/// keeps them fixed rather than picking something that only works at one
/// size.
const IDENTITY_TRACK_WIDTH: f32 = 280.0;
const MIDDLE_TRACK_WIDTH: f32 = 320.0;
const ACTION_TRACK_WIDTH: f32 = 100.0;

/// Stable per-row `FocusTarget` keys, shared between `dashboard/mod.rs`'s
/// `focus_order` and this module's [`is_focused`] — one expression per
/// target rather than the `format!` string duplicated in each place
/// (Handoff 025 §7.5, same discipline as `toolbar.rs`/`section.rs`).
pub(super) fn checkbox_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.checkbox")
}

pub(super) fn name_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.name")
}

pub(super) fn action_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.action")
}

fn is_focused(state: &AppState, key: &str) -> bool {
    state.dashboard_focus.as_ref() == Some(&FocusTarget::control_dynamic(key.to_owned()))
}

pub(super) fn view_project_row<'a>(
    state: &'a AppState,
    entry: DashboardEntry<'a>,
    mode: WidthMode,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let project = entry.project;

    let checkbox_element = selection_checkbox(state, tokens, project);
    let name = name_button(state, tokens, project);
    let action = row_action(state, tokens, &entry, project);
    let work_area = entry
        .status
        .and_then(|status| status.context.as_ref())
        .map(|context| context.label.as_str())
        .unwrap_or(state.t("dashboard.work_area_unknown"));

    match mode {
        WidthMode::Compact => {
            view_compact_row(state, &entry, checkbox_element, name, action, work_area)
        }
        WidthMode::Standard | WidthMode::Wide => {
            view_standard_row(state, &entry, checkbox_element, name, action, work_area)
        }
    }
}

/// The selection checkbox, when selection mode is on — shared between the
/// standard and compact layouts, which place it differently but never
/// change its behaviour or focus target.
fn selection_checkbox<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    project: &'a Project,
) -> Option<Element<'a, Message>> {
    if !state.selection_mode {
        return None;
    }
    let id = project.id.clone();
    let is_checked = state.selection.contains(&project.id);
    Some(checkbox(
        tokens,
        format!(
            "{} — {}",
            state.t("plain.selection.select_project"),
            project.name
        ),
        is_checked,
        move |_checked| Message::Selection(SelectionMessage::Toggled(id.clone())),
        is_focused(state, &checkbox_key(&project.id)),
    ))
}

/// The project name link — shared between layouts.
fn name_button<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    project: &'a Project,
) -> Element<'a, Message> {
    let focused = is_focused(state, &name_key(&project.id));
    let t = tokens.clone();
    button(
        text(project.name.as_str())
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::DetailPanel(DetailPanelMessage::Opened(
        project.id.clone(),
    )))
    .style(move |_theme, status| style::with_focus_ring(&t, focused, style::ghost(&t, status)))
    .into()
}

/// The tier-specific row action — shared between layouts. `Space::new()` for
/// InProgress/AllSet (no action slot, per RFC-032 R10's tier density).
fn row_action<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    entry: &DashboardEntry<'a>,
    project: &'a Project,
) -> Element<'a, Message> {
    if entry.tier != DashboardTier::NeedsHelp {
        return Space::new().into();
    }
    let focused = is_focused(state, &action_key(&project.id));
    if entry.cause == Some(DashboardCause::Conflict) {
        // The row's one mutating action (RFC-032 R10): opens the
        // conflict-resolution overlay, so it carries `secondary`'s
        // stronger weight.
        //
        // RFC-035 R13/Handoff 030 §6.2: no busy-reason text here anymore.
        // With multiple needs-help/conflict rows on screen, each printing
        // "Busy" while the interlock is held is the same duplication R13
        // targets in the selection bar, just repeated per row instead of
        // per action. The button still disables correctly (`on_press_maybe`
        // stays gated on `!is_busy()`); it just stops re-explaining why,
        // the same way row.rs's other actions never explained "busy" either.
        row_action_button(
            tokens,
            state.t("dashboard.resolve"),
            style::secondary,
            (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
            )),
            None,
            focused,
        )
    } else {
        // Dispatches the same `DetailPanel::Opened` message the project
        // name does — a link-weight affordance to the same destination,
        // not a distinct mutating action, so `ghost` matches the name's
        // own treatment rather than `secondary`.
        row_action_button(
            tokens,
            state.t("plain.show_details"),
            style::ghost,
            Some(Message::DetailPanel(DetailPanelMessage::Opened(
                project.id.clone(),
            ))),
            None,
            focused,
        )
    }
}

/// Standard/wide layout (R9): bounded three-track row, unchanged from Stage 3.
fn view_standard_row<'a>(
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    checkbox_element: Option<Element<'a, Message>>,
    name: Element<'a, Message>,
    action: Element<'a, Message>,
    work_area: &'a str,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let mut identity = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        identity = identity.push(checkbox_element);
    }
    let mut identity_details = column![name].spacing(2);
    if entry.tier == DashboardTier::NeedsHelp {
        let vcs = entry
            .status
            .map(|status| status.identity.vcs_kind.to_string())
            .unwrap_or_else(|| state.t("status.unknown").to_owned());
        identity_details = identity_details.push(
            text(vcs)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }
    identity = identity.push(identity_details);

    let middle: Element<'_, Message> = match entry.tier {
        DashboardTier::NeedsHelp => text(cause_label(state, entry.cause))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into(),
        DashboardTier::InProgress => {
            let count = entry
                .relevant_count
                .map(|count| format!("{}: {}", progress_label(state, count.kind), count.value))
                .unwrap_or_else(|| state.t("plain.status.unsaved_work").to_owned());
            column![
                text(work_area)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
                text(count)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
            ]
            .spacing(2)
            .into()
        }
        DashboardTier::AllSet => text(work_area)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into(),
    };

    container(
        row![
            container(identity).width(Length::Fixed(IDENTITY_TRACK_WIDTH)),
            container(middle).width(Length::Fixed(MIDDLE_TRACK_WIDTH)),
            container(action).width(Length::Fixed(ACTION_TRACK_WIDTH)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .into()
}

/// Compact layout (R8, 800-999px): two lines. Line one is identity and the
/// row action (pushed to the far edge by a `Fill` spacer, same idiom the
/// toolbar/shell use for pinning a control to the opposite end of a row);
/// line two is a single combined status/reason line. Tier density is
/// unchanged from the standard layout (RFC-032 R10) — this only rearranges
/// the same content onto two lines instead of three tracks.
fn view_compact_row<'a>(
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    checkbox_element: Option<Element<'a, Message>>,
    name: Element<'a, Message>,
    action: Element<'a, Message>,
    work_area: &'a str,
) -> Element<'a, Message> {
    let mut line_one = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        line_one = line_one.push(checkbox_element);
    }
    line_one = line_one.push(name);
    line_one = line_one.push(Space::new().width(Length::Fill));
    line_one = line_one.push(action);

    let line_two = compact_status_line(state, entry, work_area);

    container(
        column![
            line_one,
            text(line_two)
                .size(snora::design::style::text::body_small_size(
                    &state.theme.tokens
                ))
                .line_height(snora::design::style::text::body_small_line_height(
                    &state.theme.tokens
                ))
        ]
        .spacing(2),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .into()
}

/// The compact layout's single status/reason line, folding what the
/// standard layout spreads across the identity column's VCS sub-line and
/// the middle track into one line per tier — same content, not more or
/// less (RFC-032 R10).
fn compact_status_line(state: &AppState, entry: &DashboardEntry<'_>, work_area: &str) -> String {
    match entry.tier {
        DashboardTier::NeedsHelp => {
            let vcs = entry
                .status
                .map(|status| status.identity.vcs_kind.to_string())
                .unwrap_or_else(|| state.t("status.unknown").to_owned());
            format!("{vcs} · {}", cause_label(state, entry.cause))
        }
        DashboardTier::InProgress => {
            let count = entry
                .relevant_count
                .map(|count| format!("{}: {}", progress_label(state, count.kind), count.value))
                .unwrap_or_else(|| state.t("plain.status.unsaved_work").to_owned());
            format!("{work_area} · {count}")
        }
        DashboardTier::AllSet => work_area.to_owned(),
    }
}

/// The row's tier-specific action (R6): `secondary` for the row's one
/// mutating action, `ghost` for a link-weight affordance — the caller
/// chooses which via `style_fn`, one of `style::secondary`/`style::ghost`'s
/// plain-function signatures (same shape `select_mode_button` in
/// `toolbar.rs` closes over, but parametrized here since a row's action
/// varies by cause). Replaces `guided_button`'s disabled-reason-text
/// composition on top of a real semantic style plus a working focus ring,
/// keyed to the row's `dashboard.row.{id}.action` target.
fn row_action_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
    on_press: Option<Message>,
    reason: Option<&'a str>,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let show_reason = on_press.is_none();

    let btn = button(
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style_fn(&t, status)));

    match reason {
        Some(r) if show_reason => column![
            btn,
            text(r)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
        ]
        .spacing(4)
        .into(),
        _ => btn.into(),
    }
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
