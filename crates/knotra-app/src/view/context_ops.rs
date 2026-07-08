//! Context Operations view — browse branches/changesets and switch context.

use knotra_vcs::{ContextSwitchResult, VcsKind};
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{ContextMessage, Message},
    state::{
        context::ContextPhase,
        AppState,
    },
};

// ---------------------------------------------------------------------------
// Top-level dispatcher
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);

    // Pre-compute strings that must outlive the match arm.
    let switching_msg = if let ContextPhase::Switching { target, .. } = &state.context_ops.phase {
        format!("{} → {}", state.t("context.switching"), target)
    } else {
        String::new()
    };
    let done_result = if let ContextPhase::Done(r) = &state.context_ops.phase {
        Some(r.clone())
    } else {
        None
    };

    let body: Element<'_, Message> = match &state.context_ops.phase {
        ContextPhase::Idle           => view_project_list(state),
        ContextPhase::LoadingList(_) => loading(state.t("context.loading").to_owned()),
        ContextPhase::BrowsingList { .. } => view_branch_list(state),
        ContextPhase::ConfirmSwitch { .. } => view_confirm(state),
        ContextPhase::Switching { .. } => loading(switching_msg),
        ContextPhase::Done(_) => view_done(state, done_result.unwrap()),
    };

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn view_header(state: &AppState) -> Element<'_, Message> {
    let back_label = state.t("context.back");
    // Back target depends on phase.
    let back_msg = match &state.context_ops.phase {
        ContextPhase::BrowsingList { .. }
        | ContextPhase::LoadingList(_) => Message::Context(ContextMessage::BackToDashboard),
        ContextPhase::ConfirmSwitch { .. } => Message::Context(ContextMessage::SwitchCancelled),
        _ => Message::Context(ContextMessage::BackToDashboard),
    };

    row![
        button(text(back_label)).on_press(back_msg),
        text(state.t("context.title")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Phase: Idle — project selector
// ---------------------------------------------------------------------------

fn view_project_list(state: &AppState) -> Element<'_, Message> {
    let projects = state.workspace.as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);

    if projects.is_empty() {
        return container(text(state.t("dashboard.no_projects")).size(14))
            .padding(24).into();
    }

    let statuses = state.workspace_status.as_ref()
        .map(|ws| ws.projects.as_slice())
        .unwrap_or(&[]);

    let rows: Vec<Element<'_, Message>> = projects.iter().map(|project| {
        let status = statuses.iter().find(|s| s.project_id == project.id);
        let ctx_label = status.and_then(|s| s.context.as_ref())
            .map(|c| c.label.as_str())
            .unwrap_or("—");
        let vcs = status.map(|s| s.identity.vcs_kind.to_string()).unwrap_or_default();
        let id = project.id.clone();

        let btn = button(
            row![
                text(project.name.as_str()).size(14),
                Space::new().width(Length::Fill),
                text(format!("{vcs}  {ctx_label}")).size(11),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .on_press(Message::Context(ContextMessage::ProjectSelected(id)))
        .width(Length::Fill);

        container(btn).width(Length::Fill).padding([2, 0]).into()
    }).collect();

    column![
        text(state.t("context.select_project")).size(13),
        column(rows).spacing(4),
    ]
    .spacing(12)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Phase: BrowsingList — branch/changeset picker
// ---------------------------------------------------------------------------

fn view_branch_list(state: &AppState) -> Element<'_, Message> {
    let (project_id, list, search) = match &state.context_ops.phase {
        ContextPhase::BrowsingList { project_id, list, search } => (project_id, list, search),
        _ => return Space::new().into(),
    };

    let project_name = state.workspace.as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == project_id))
        .map(|p| p.name.as_str())
        .unwrap_or("—");

    let search_box = text_input(state.t("context.search_placeholder"), search)
        .on_input(|s| Message::Context(ContextMessage::SearchChanged(s)))
        .width(Length::Fill);

    let filtered = state.context_ops.filtered_candidates();

    let candidate_rows: Vec<Element<'_, Message>> = if filtered.is_empty() {
        vec![text(state.t("context.no_candidates")).size(13).into()]
    } else {
        filtered.iter().map(|cand| {
            let badge = if cand.is_current {
                format!(" [{}]", state.t("context.current"))
            } else if cand.is_remote {
                format!(" [{}]", state.t("context.remote"))
            } else {
                String::new()
            };

            let id  = project_id.clone();
            let tgt = cand.target.clone();
            let label = format!("{}{}", cand.label, badge);

            let btn = button(text(label).size(13))
                .on_press_maybe(
                    if cand.is_current { None }
                    else {
                        Some(Message::Context(ContextMessage::SwitchTargetChosen(id, tgt)))
                    }
                )
                .width(Length::Fill);

            container(btn).width(Length::Fill).padding([1, 0]).into()
        }).collect()
    };

    // Warning from the VCS layer (e.g. detached HEAD).
    let warning_row: Option<Element<'_, Message>> = list.warning.as_ref().map(|w| {
        text(format!("⚠ {w}")).size(12).into()
    });

    let mut col = column![
        text(project_name).size(16),
        search_box,
        column(candidate_rows).spacing(2),
    ]
    .spacing(8)
    .padding(24);

    if let Some(w) = warning_row {
        col = col.push(w);
    }

    col.into()
}

// ---------------------------------------------------------------------------
// Phase: ConfirmSwitch
// ---------------------------------------------------------------------------

fn view_confirm(state: &AppState) -> Element<'_, Message> {
    let (project_name, target, vcs_kind, is_dirty) = match &state.context_ops.phase {
        ContextPhase::ConfirmSwitch { project_name, target, vcs_kind, is_dirty, .. } => {
            (project_name.clone(), target.clone(), *vcs_kind, *is_dirty)
        }
        _ => return Space::new().into(),
    };

    // VCS-specific explanation.
    let vcs_note: &str = match vcs_kind {
        VcsKind::Git      => "git switch",
        VcsKind::Jujutsu  => "jj edit",
    };

    let mut col = column![
        text(state.t("context.confirm.title")).size(18),
        text(format!("{} `{}`   ({})", state.t("context.confirm.body"), target, vcs_note)).size(14),
        text(format!("Project: {}", project_name)).size(12),
    ]
    .spacing(8)
    .padding(24);

    if is_dirty {
        col = col.push(text(state.t("context.confirm.dirty_warn")).size(13));
        col = col.push(text(state.t("context.confirm.dirty_hint")).size(12));
    }

    col = col.push(
        row![
            button(text(state.t("context.confirm.switch")))
                .on_press(Message::Context(ContextMessage::SwitchConfirmed)),
            button(text(state.t("context.confirm.cancel")))
                .on_press(Message::Context(ContextMessage::SwitchCancelled)),
        ]
        .spacing(8)
        .padding([8, 0]),
    );

    col.into()
}

// ---------------------------------------------------------------------------
// Phase: Done
// ---------------------------------------------------------------------------

fn view_done(state: &AppState, result: ContextSwitchResult) -> Element<'_, Message> {
    let success = result.operation_result.success;
    let title_key = if success { "context.done.success" } else { "context.done.failure" };

    let mut col = column![
        text(state.t(title_key)).size(18),
        text(format!("Project: {}", result.project_name)).size(13),
        text(format!("Target:  {}", result.target)).size(13),
    ]
    .spacing(6)
    .padding(24);

    // Commands executed (transparency).
    if !result.operation_result.commands_executed.is_empty() {
        col = col.push(text(state.t("context.done.commands")).size(12));
        for cmd in &result.operation_result.commands_executed {
            col = col.push(text(format!("  $ {cmd}")).size(11));
        }
    }

    // Stderr on failure.
    if !success && !result.operation_result.stderr.is_empty() {
        let preview: String = result.operation_result.stderr
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n");
        col = col.push(text(preview).size(11));
    }

    // Recovery hint.
    if let Some(hint) = result.recovery_hint.clone() {
        col = col.push(text(state.t("context.done.recovery")).size(12));
        col = col.push(text(hint.situation.clone()).size(11));
        for cmd in &hint.suggested_commands {
            col = col.push(text(format!("  $ {cmd}")).size(11));
        }
    }

    // Navigation.
    let switch_again_label = if success {
        "Switch Another"
    } else {
        "Try Again"
    };

    col = col.push(
        row![
            button(text(state.t("context.back")))
                .on_press(Message::Context(ContextMessage::BackToDashboard)),
            button(text(switch_again_label))
                .on_press(Message::Context(ContextMessage::BackToDashboard)),
        ]
        .spacing(8)
        .padding([8, 0]),
    );

    col.into()
}

// ---------------------------------------------------------------------------
// Loading spinner placeholder
// ---------------------------------------------------------------------------

fn loading(msg: String) -> Element<'static, Message> {
    container(text(msg).size(14))
        .width(Length::Fill)
        .height(250)
        .center_x(Length::Fill)
        .center_y(250)
        .into()
}
