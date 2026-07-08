//! Conflict Resolution view.

use knotra_vcs::ProjectConflictDetail;
use iced::{
    widget::{button, column, container, row, scrollable, text, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{ConflictOpsMessage, LaunchMessage, Message},
    state::{
        conflict_ops::ConflictPhase,
        AppState,
    },
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = row![
        button(text("← Dashboard"))
            .on_press(Message::ConflictOps(ConflictOpsMessage::BackToDashboard)),
        text(state.t("conflicts.title")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0));

    // Pre-extract all data that must outlive the match.
    let browsing = if let ConflictPhase::Browsing { project_id, detail } = &state.conflict_ops.phase {
        Some((project_id.clone(), detail.clone()))
    } else { None };

    let done_info = if let ConflictPhase::Done { success, message, project_id } = &state.conflict_ops.phase {
        Some((*success, message.clone(), project_id.clone()))
    } else { None };

    let op_msg = if let ConflictPhase::Operating { action, .. } = &state.conflict_ops.phase {
        action.clone()
    } else { String::new() };

    // Build body from pre-extracted data.
    let body: Element<'static, Message> = match &state.conflict_ops.phase {
        ConflictPhase::Idle             => view_project_list_owned(state),
        ConflictPhase::Loading(_)       => spinner("Loading…"),
        ConflictPhase::Browsing { .. }  => {
            let (id, detail) = browsing.unwrap();
            view_file_list_owned(state, id, detail)
        }
        ConflictPhase::Operating { .. } => spinner(&op_msg),
        ConflictPhase::Done { .. }      => {
            let (ok, msg, pid) = done_info.unwrap();
            view_done_owned(state, ok, msg, pid)
        }
    };

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

fn view_project_list_owned(state: &AppState) -> Element<'static, Message> {
    let projects = state.workspace.as_ref()
        .map(|w| w.projects.clone())
        .unwrap_or_default();

    let conflicted: Vec<_> = projects.iter().filter(|p| {
        state.workspace_status.as_ref()
            .and_then(|ws| ws.projects.iter().find(|s| s.project_id == p.id))
            .map(|s| s.conflict.has_conflict)
            .unwrap_or(false)
    }).cloned().collect();

    if conflicted.is_empty() {
        return container(text(state.t("conflicts.no_conflicts").to_owned()).size(14))
            .padding(24).into();
    }

    let rows: Vec<Element<'static, Message>> = conflicted.into_iter().map(|p| {
        let id = p.id.clone();
        let nm = p.name.clone();
        button(text(nm).size(13))
            .on_press(Message::ConflictOps(ConflictOpsMessage::ProjectSelected(id)))
            .width(Length::Fill)
            .into()
    }).collect();

    column(rows).spacing(4).padding(24).into()
}

fn view_file_list_owned(
    state: &AppState,
    project_id: knotra_vcs::ProjectId,
    detail: ProjectConflictDetail,
) -> Element<'static, Message> {
    let project_name = state.workspace.as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| p.id == project_id).map(|p| p.name.clone()))
        .unwrap_or_else(|| "—".to_owned());

    let project_path = state.workspace.as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| p.id == project_id).map(|p| p.path.clone()))
        .unwrap_or_default();

    if detail.is_resolved() {
        return column![
            text(state.t("conflicts.resolved_all").to_owned()).size(14),
            button(text(state.t("conflicts.recheck").to_owned()))
                .on_press(Message::ConflictOps(ConflictOpsMessage::RecheckRequested(project_id))),
        ].spacing(8).padding(24).into();
    }

    let editor_configured = state.config.external_editor.is_some();
    let mergetool_configured = state.config.external_merge_tool.is_some();

    let mark_label   = state.t("conflicts.mark_resolved").to_owned();
    let editor_label = state.t("conflicts.open_editor").to_owned();
    let merge_label  = state.t("conflicts.open_mergetool").to_owned();
    let files_header = state.t("conflicts.files_header").to_owned();
    let recheck_label= state.t("conflicts.recheck").to_owned();
    let abort_label  = state.t("conflicts.abort_merge").to_owned();

    let mut items: Vec<Element<'static, Message>> = vec![
        text(project_name).size(16).into(),
        text(files_header).size(13).into(),
    ];

    if let Some(note) = detail.note {
        items.push(text(note).size(11).into());
    }

    for file in detail.conflicted_files {
        let abs_path = format!("{}/{}", project_path, file.path);
        let ap1 = abs_path.clone();
        let ap2 = abs_path.clone();
        let pid  = project_id.clone();
        let fkey = file.path.clone();

        let file_row = row![
            text(format!("⚡ {}  [{}]", file.path, file.marker)).size(12),
            Space::new().width(Length::Fill),
            button(text(editor_label.clone()).size(10))
                .on_press_maybe(if editor_configured {
                    Some(Message::Launch(LaunchMessage::OpenInEditor(ap1)))
                } else { None }),
            button(text(merge_label.clone()).size(10))
                .on_press_maybe(if mergetool_configured {
                    Some(Message::Launch(LaunchMessage::OpenInMergeTool(ap2)))
                } else { None }),
            button(text(mark_label.clone()).size(10))
                .on_press(Message::ConflictOps(ConflictOpsMessage::MarkResolvedRequested {
                    project_id: pid,
                    file_path:  fkey,
                })),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into();
        items.push(file_row);
    }

    let pid_recheck = project_id.clone();
    let pid_abort   = project_id.clone();
    items.push(
        row![
            button(text(recheck_label)).on_press(Message::ConflictOps(ConflictOpsMessage::RecheckRequested(pid_recheck))),
            button(text(abort_label)).on_press(Message::ConflictOps(ConflictOpsMessage::AbortMergeRequested(pid_abort))),
            button(text("← Back")).on_press(Message::ConflictOps(ConflictOpsMessage::BackToDashboard)),
        ]
        .spacing(8)
        .padding([8, 0])
        .into(),
    );

    column(items).spacing(6).padding(24).into()
}

fn view_done_owned(
    state: &AppState,
    success: bool,
    message: String,
    project_id: knotra_vcs::ProjectId,
) -> Element<'static, Message> {
    let icon      = if success { "✓" } else { "✗" };
    let recheck   = state.t("conflicts.recheck").to_owned();
    column![
        text(format!("{icon} {message}")).size(14),
        button(text(recheck))
            .on_press(Message::ConflictOps(ConflictOpsMessage::RecheckRequested(project_id))),
        button(text("← Back"))
            .on_press(Message::ConflictOps(ConflictOpsMessage::BackToDashboard)),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn spinner(msg: &str) -> Element<'static, Message> {
    container(text(msg.to_owned()).size(14))
        .width(Length::Fill)
        .height(250)
        .center_x(Length::Fill)
        .center_y(250)
        .into()
}
