//! A single dashboard project row: identity, status/progress summary, and
//! the tier-specific action.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_SMALL, Tokens, checkbox, style};

use crate::{
    message::{ConflictOpsMessage, DetailPanelMessage, Message, SelectionMessage},
    state::{
        AppState,
        dashboard::{DashboardCause, DashboardEntry, DashboardTier, ProgressKind},
        focus::FocusTarget,
    },
};

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
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let project = entry.project;
    let mut identity = row![].spacing(4).align_y(Alignment::Center);
    if state.selection_mode {
        let id = project.id.clone();
        let is_checked = state.selection.contains(&project.id);
        identity = identity.push(checkbox(
            tokens,
            format!(
                "{} — {}",
                state.t("plain.selection.select_project"),
                project.name
            ),
            is_checked,
            move |_checked| Message::Selection(SelectionMessage::Toggled(id.clone())),
            is_focused(state, &checkbox_key(&project.id)),
        ));
    }
    let name_focused = is_focused(state, &name_key(&project.id));
    let name_tokens = tokens.clone();
    let name = button(text(project.name.as_str()).size(13))
        .on_press(Message::DetailPanel(DetailPanelMessage::Opened(
            project.id.clone(),
        )))
        .style(move |_theme, status| {
            style::with_focus_ring(
                &name_tokens,
                name_focused,
                style::ghost(&name_tokens, status),
            )
        });
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

    let action_focused = is_focused(state, &action_key(&project.id));
    let action: Element<'_, Message> = if entry.tier == DashboardTier::NeedsHelp {
        if entry.cause == Some(DashboardCause::Conflict) {
            // The row's one mutating action (RFC-032 R10): opens the
            // conflict-resolution overlay, so it carries `secondary`'s
            // stronger weight.
            row_action_button(
                tokens,
                state.t("dashboard.resolve"),
                style::secondary,
                (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                    ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
                )),
                state
                    .operation_interlock
                    .is_busy()
                    .then(|| state.t("plain.activity.busy")),
                action_focused,
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
                action_focused,
            )
        }
    } else {
        Space::new().into()
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

    let btn = button(text(label).size(12))
        .height(BUTTON_HEIGHT)
        .padding([0, 12])
        .on_press_maybe(on_press)
        .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style_fn(&t, status)));

    match reason {
        Some(r) if show_reason => column![btn, text(r).size(FONT_SMALL)].spacing(4).into(),
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
