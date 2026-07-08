#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-013 — Bulk action modal views.
//!
//! Five modals replacing the dedicated screens for Pull, Tag, Switch,
//! Resolve (conflict), and Changelog workflows.  Each modal opens over
//! the dashboard and closes on completion or Esc.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};

use endringer::ProjectId;

use crate::{
    message::{
        ChangelogMessage, ConflictOpsMessage, ContextMessage, FreezerMessage, Message, SyncMessage,
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Modal shell
// ---------------------------------------------------------------------------

/// Wrap the inner content of any modal in a shared shell with title bar.
fn modal_shell<'a>(
    title: &'a str,
    close_msg: Message,
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    let close_btn = button(text("✕").size(13)).on_press(close_msg);

    let header = row![
        text(title).size(15),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([0, 0]);

    container(column![header, inner].spacing(12).padding(20))
        .width(Length::Fixed(580.0))
        .into()
}

// ---------------------------------------------------------------------------
// 1. Smart Pull modal
// ---------------------------------------------------------------------------

pub fn pull_modal(state: &AppState) -> Element<'_, Message> {
    let sync = &state.sync;

    let project_rows: Vec<Element<'_, Message>> = match &sync.phase {
        crate::state::sync::SyncPhase::Idle => {
            let selected: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .filter(|p| state.selection.contains(&p.id))
                        .collect()
                })
                .unwrap_or_default();
            selected
                .iter()
                .map(|p| text(format!("• {}", p.name)).size(12).into())
                .collect()
        }
        crate::state::sync::SyncPhase::Planning
        | crate::state::sync::SyncPhase::AwaitingConfirm(_) => {
            vec![text("Computing plan…").size(12).into()]
        }
        crate::state::sync::SyncPhase::FetchRunning { done, total } => {
            vec![
                text(format!("Fetching… {}/{}", done, total))
                    .size(12)
                    .into(),
            ]
        }
        crate::state::sync::SyncPhase::PullRunning { completed, .. } => completed
            .iter()
            .map(|p| {
                let name = project_name_for(state, &p.project_id);
                text(format!("⟳ {}", name)).size(12).into()
            })
            .collect(),
        crate::state::sync::SyncPhase::Done(result) => result
            .per_project
            .iter()
            .map(|pp| {
                let name = project_name_for(state, &pp.project_id);
                let icon = if pp.success { "✓" } else { "✗" };
                text(format!("{} {}", icon, name)).size(12).into()
            })
            .collect(),
    };

    let project_list = scrollable(column(project_rows).spacing(4)).height(Length::Fixed(200.0));

    let footer: Element<'_, Message> = match &sync.phase {
        crate::state::sync::SyncPhase::Idle => button(text("Plan Pull").size(13))
            .on_press(Message::Sync(SyncMessage::PlanRequested))
            .into(),
        crate::state::sync::SyncPhase::AwaitingConfirm(_) => row![
            button(text("Execute").size(13)).on_press(Message::Sync(SyncMessage::ExecuteRequested)),
            button(text("Cancel").size(13)).on_press(Message::Sync(SyncMessage::Cancelled)),
        ]
        .spacing(8)
        .into(),
        crate::state::sync::SyncPhase::PullRunning { .. }
        | crate::state::sync::SyncPhase::FetchRunning { .. }
        | crate::state::sync::SyncPhase::Planning => text("Pulling…").size(12).into(),
        crate::state::sync::SyncPhase::Done(_) => button(text("Close").size(13))
            .on_press(Message::Sync(SyncMessage::ModalClosed))
            .into(),
    };

    let inner = column![project_list, footer].spacing(12);

    modal_shell(
        "Pull projects",
        Message::Sync(SyncMessage::ModalClosed),
        inner.into(),
    )
}

// ---------------------------------------------------------------------------
// 2. Tag modal  (Freezer)
// ---------------------------------------------------------------------------

pub fn tag_modal(state: &AppState) -> Element<'_, Message> {
    let freezer = &state.freezer;

    let name_input = text_input("e.g. v1.2.3", &freezer.freeze_name)
        .on_input(|s| Message::Freezer(FreezerMessage::NameChanged(s)))
        .padding([6, 10])
        .size(13);

    let msg_input = text_input("Optional message (annotated tag)", &freezer.tag_message)
        .on_input(|s| Message::Freezer(FreezerMessage::TagMessageChanged(s)))
        .padding([6, 10])
        .size(13);

    // Validation results if available
    let validation: Element<'_, Message> = match &freezer.phase {
        crate::state::freezer::FreezerPhase::Validating => text("Validating…").size(12).into(),
        crate::state::freezer::FreezerPhase::ValidationReady(validation) => {
            let blockers: Vec<Element<'_, Message>> = validation
                .entries
                .iter()
                .filter(|v| v.is_blocked())
                .map(|v| {
                    text(format!(
                        "✗ {}: {}",
                        v.project_name,
                        v.blockers.first().map(|s| s.as_str()).unwrap_or("blocked")
                    ))
                    .size(11)
                    .into()
                })
                .collect();
            if blockers.is_empty() {
                text("✓ All projects ready").size(12).into()
            } else {
                column(blockers).spacing(3).into()
            }
        }
        _ => Space::new().into(),
    };

    let footer: Element<'_, Message> = match &freezer.phase {
        crate::state::freezer::FreezerPhase::Idle => button(text("Validate").size(13))
            .on_press(Message::Freezer(FreezerMessage::ValidateRequested))
            .into(),
        crate::state::freezer::FreezerPhase::ValidationReady(validation)
            if validation.all_ready() =>
        {
            row![
                button(text("Execute").size(13))
                    .on_press(Message::Freezer(FreezerMessage::ExecuteConfirmed)),
                button(text("Cancel").size(13))
                    .on_press(Message::Freezer(FreezerMessage::BulkModalClosed)),
            ]
            .spacing(8)
            .into()
        }
        crate::state::freezer::FreezerPhase::Executing => text("Tagging…").size(12).into(),
        crate::state::freezer::FreezerPhase::Done(_) => button(text("Close").size(13))
            .on_press(Message::Freezer(FreezerMessage::BulkModalClosed))
            .into(),
        _ => button(text("Validate").size(13))
            .on_press(Message::Freezer(FreezerMessage::ValidateRequested))
            .into(),
    };

    let inner = column![
        text("Tag name:").size(12),
        name_input,
        text("Message (optional):").size(12),
        msg_input,
        validation,
        footer,
    ]
    .spacing(10);

    modal_shell(
        "Tag selected projects",
        Message::Freezer(FreezerMessage::BulkModalClosed),
        inner.into(),
    )
}

// ---------------------------------------------------------------------------
// 3. Switch Branch modal
// ---------------------------------------------------------------------------

pub fn switch_modal(state: &AppState) -> Element<'_, Message> {
    let ctx = &state.context_ops;

    let branch_input = text_input("Branch name…", &ctx.target_context)
        .on_input(|s| Message::Context(ContextMessage::TargetChanged(s)))
        .padding([6, 10])
        .size(13);

    let footer: Element<'_, Message> = match ctx.phase {
        crate::state::context::ContextPhase::Idle => button(text("Switch").size(13))
            .on_press(Message::Context(ContextMessage::BulkSwitchRequested))
            .into(),
        crate::state::context::ContextPhase::Switching { .. } => text("Switching…").size(12).into(),
        crate::state::context::ContextPhase::Done(_) => button(text("Close").size(13))
            .on_press(Message::Context(ContextMessage::BulkModalClosed))
            .into(),
        _ => Space::new().into(),
    };

    let inner = column![
        text("Switch all selected projects to branch:").size(12),
        branch_input,
        footer,
    ]
    .spacing(10);

    modal_shell(
        "Switch branch",
        Message::Context(ContextMessage::BulkModalClosed),
        inner.into(),
    )
}

// ---------------------------------------------------------------------------
// 4. Conflict resolution panel (right-docked, not a centred modal)
// ---------------------------------------------------------------------------

pub fn resolve_panel<'a>(state: &'a AppState, project_id: &'a ProjectId) -> Element<'a, Message> {
    let name = project_name_for(state, project_id);
    let ops = &state.conflict_ops;

    let file_rows: Vec<Element<'_, Message>> = ops
        .cached
        .values()
        .flat_map(|d| d.conflicted_files.iter())
        .map(|f| {
            let resolved = false; // resolved tracking added in future
            let icon = if resolved { "✓" } else { "✗" };
            row![
                text(format!("{} {}", icon, f.path)).size(12),
                Space::new().width(Length::Fill),
                button(text("Mark resolved").size(11)).on_press(Message::ConflictOps(
                    ConflictOpsMessage::FileMarkedResolved(f.path.clone())
                )),
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .into()
        })
        .collect();

    let footer = row![
        button(text("Abort merge").size(12))
            .on_press(Message::ConflictOps(ConflictOpsMessage::AbortRequested)),
        Space::new().width(Length::Fill),
        button(text("Close").size(12))
            .on_press(Message::ConflictOps(ConflictOpsMessage::PanelClosed)),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(format!("Conflicts — {}", name)).size(14),
                Space::new().width(Length::Fill),
                button(text("✕").size(12))
                    .on_press(Message::ConflictOps(ConflictOpsMessage::PanelClosed)),
            ]
            .align_y(Alignment::Center),
            scrollable(column(file_rows).spacing(6)).height(Length::Fill),
            footer,
        ]
        .spacing(12)
        .padding(16),
    )
    .width(Length::Fixed(320.0))
    .height(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// 5. Changelog modal
// ---------------------------------------------------------------------------

pub fn changelog_modal(state: &AppState) -> Element<'_, Message> {
    let cl = &state.changelog;

    let since_input = text_input("Since tag / ref…", &cl.since_ref)
        .on_input(|s| Message::Changelog(ChangelogMessage::SinceRefChanged(s)))
        .padding([6, 10])
        .size(13);

    let content: Element<'_, Message> = match &cl.phase {
        crate::state::changelog::ChangelogPhase::Idle => button(text("Generate").size(13))
            .on_press(Message::Changelog(ChangelogMessage::CollectRequested))
            .into(),
        crate::state::changelog::ChangelogPhase::Collecting => text("Collecting…").size(12).into(),
        crate::state::changelog::ChangelogPhase::Ready(draft) => column![
            scrollable(text(format!("{:?}", draft)).size(11)).height(Length::Fixed(240.0)),
            row![
                button(text("Copy to clipboard").size(12))
                    .on_press(Message::CopyToClipboard(format!("{:?}", draft))),
                button(text("Close").size(12))
                    .on_press(Message::Changelog(ChangelogMessage::ModalClosed)),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .into(),
    };

    let inner = column![
        text("Generate changelog since:").size(12),
        since_input,
        content,
    ]
    .spacing(10);

    modal_shell(
        "Generate Changelog",
        Message::Changelog(ChangelogMessage::ModalClosed),
        inner.into(),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_name_for(state: &AppState, id: &ProjectId) -> String {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| id.to_string())
}
