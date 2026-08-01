//! A single dashboard project row: identity, status/progress summary, and
//! the tier-specific action.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::guided_button;

use crate::{
    message::{ConflictOpsMessage, DetailPanelMessage, Message, SelectionMessage},
    state::{
        AppState,
        dashboard::{DashboardCause, DashboardEntry, DashboardTier, ProgressKind},
    },
};

pub(super) fn view_project_row<'a>(
    state: &'a AppState,
    entry: DashboardEntry<'a>,
) -> Element<'a, Message> {
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
