//! Sync Center view — bulk fetch and Smart Pull with per-project progress.

use endringer::model::operation::{SmartPullDisposition, SmartPullPlan};
use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{Message, SyncMessage},
    state::{
        sync::{ProjectOutcome, SyncKind, SyncPhase, SyncResult},
        AppState,
    },
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let body: Element<'_, Message> = match &state.sync.phase {
        SyncPhase::Idle              => view_idle(state),
        SyncPhase::Planning          => view_planning(state),
        SyncPhase::FetchRunning { total, done } => view_fetch_running(state, *total, *done),
        SyncPhase::AwaitingConfirm(plan) => view_confirm_plan(state, plan),
        SyncPhase::PullRunning { plan, completed } => {
            view_pull_running(state, plan.entries.len(), completed.len())
        }
        SyncPhase::Done(result) => view_done(state, result),
    };

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn view_header(state: &AppState) -> Element<'_, Message> {
    let back_btn = button(text("← Dashboard"))
        .on_press(Message::Navigate(crate::state::Screen::Dashboard));

    row![
        back_btn,
        text(state.t("nav.sync")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Idle — project list + operation buttons
// ---------------------------------------------------------------------------

fn view_idle(state: &AppState) -> Element<'_, Message> {
    let projects = state.workspace.as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);

    if projects.is_empty() {
        return container(text("No projects registered.").size(14))
            .padding(24)
            .into();
    }

    let statuses = state.workspace_status.as_ref()
        .map(|ws| ws.projects.as_slice())
        .unwrap_or(&[]);

    // Project list with checkboxes.
    let project_rows: Vec<Element<'_, Message>> = projects.iter().map(|project| {
        let included = state.sync.is_selected(&project.id);
        let status = statuses.iter().find(|s| s.project_id == project.id);
        let ctx = status.and_then(|s| s.context.as_ref())
            .map(|c| c.label.as_str())
            .unwrap_or("—");
        let dirty_badge: Element<'_, Message> = if status.map(|s| s.working_tree.is_dirty()).unwrap_or(false) {
            text(" ● Uncommitted").size(11).into()
        } else {
            Space::new().into()
        };
        let conflict_badge: Element<'_, Message> = if status.map(|s| s.conflict.has_conflict).unwrap_or(false) {
            text(" ⚠ Conflict").size(11).into()
        } else {
            Space::new().into()
        };
        let behind = status.map(|s| s.remote.behind).unwrap_or(0);
        let behind_badge: Element<'_, Message> = if behind > 0 {
            text(format!(" ↓{behind} Behind")).size(11).into()
        } else {
            Space::new().into()
        };

        let id = project.id.clone();
        row![
            checkbox(included)
                .label("")
                .on_toggle(move |v| Message::Sync(SyncMessage::ProjectToggled(id.clone(), v))),
            text(project.name.as_str()).size(13),
            text(format!("  {ctx}")).size(11),
            behind_badge,
            dirty_badge,
            conflict_badge,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 0])
        .into()
    }).collect();

    // Operation buttons.
    let fetch_btn = button(text(state.t("action.fetch")).size(13))
        .on_press(Message::Sync(SyncMessage::BulkFetchRequested));

    let pull_btn = button(text("Smart Pull").size(13))
        .on_press(Message::Sync(SyncMessage::SmartPullPlanRequested));

    column![
        text("Select projects and choose an operation:").size(13),
        column(project_rows).spacing(2).padding([4, 0]),
        row![fetch_btn, pull_btn].spacing(8).padding([8, 0]),
        shortcut_hint(),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Planning spinner
// ---------------------------------------------------------------------------

fn view_planning(state: &AppState) -> Element<'_, Message> {
    container(text("Building plan…").size(14))
        .padding(24)
        .into()
}

// ---------------------------------------------------------------------------
// Fetch running — progress bar equivalent
// ---------------------------------------------------------------------------

fn view_fetch_running(state: &AppState, total: usize, done: usize) -> Element<'_, Message> {
    column![
        text(format!("Fetching… ({done} / {total})")).size(14),
        text("Please wait — operations are running concurrently.").size(12),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Confirm plan — user reviews before execution
// ---------------------------------------------------------------------------

fn view_confirm_plan<'a>(state: &'a AppState, plan: &'a SmartPullPlan) -> Element<'a, Message> {
    let pull_count     = plan.pull_count();
    let excluded_count = plan.excluded_count();

    let mut rows: Vec<Element<'_, Message>> = Vec::new();

    rows.push(text(format!("Smart Pull Plan — {} project(s) will be pulled, {} excluded",
        pull_count, excluded_count)).size(14).into());
    rows.push(text("Review and adjust dispositions before confirming:").size(12).into());

    for entry in &plan.entries {
        let disp_label = match entry.disposition {
            SmartPullDisposition::Pull          => "Pull (ff-only)",
            SmartPullDisposition::StashAndPull  => "Stash → Pull → Pop",
            SmartPullDisposition::FetchOnly     => "Fetch only (dirty)",
            SmartPullDisposition::Excluded      => "Excluded",
        };

        let dirty_note = if entry.has_conflict {
            " [conflict — excluded]"
        } else if entry.is_dirty {
            " [dirty]"
        } else {
            ""
        };

        let id_stash = entry.project_id.clone();
        let id_fetch = entry.project_id.clone();
        let id_excl  = entry.project_id.clone();

        // Disposition selector buttons (only for non-conflicted).
        let disp_row: Element<'_, Message> = if !entry.has_conflict {
            row![
                button(text("Pull").size(10))
                    .on_press(Message::Sync(SyncMessage::DispositionChanged(
                        id_stash.clone(), SmartPullDisposition::Pull))),
                button(text("Stash+Pull").size(10))
                    .on_press(Message::Sync(SyncMessage::DispositionChanged(
                        id_fetch.clone(), SmartPullDisposition::StashAndPull))),
                button(text("Fetch only").size(10))
                    .on_press(Message::Sync(SyncMessage::DispositionChanged(
                        id_excl.clone(), SmartPullDisposition::FetchOnly))),
                button(text("Exclude").size(10))
                    .on_press(Message::Sync(SyncMessage::DispositionChanged(
                        entry.project_id.clone(), SmartPullDisposition::Excluded))),
            ]
            .spacing(4)
            .into()
        } else {
            Space::new().into()
        };

        rows.push(
            row![
                text(format!("  {}{}  →  {}", entry.project_name, dirty_note, disp_label)).size(12),
                Space::new().width(Length::Fill),
                disp_row,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([2, 0])
            .into(),
        );
    }

    let plan_clone = plan.clone();
    rows.push(
        row![
            button(text("Execute Smart Pull"))
                .on_press(Message::Sync(SyncMessage::SmartPullConfirmed(plan_clone))),
            button(text("Cancel"))
                .on_press(Message::Sync(SyncMessage::SmartPullCancelled)),
        ]
        .spacing(8)
        .padding([8, 0])
        .into(),
    );

    column(rows).spacing(6).padding(24).into()
}

// ---------------------------------------------------------------------------
// Pull running — streaming progress
// ---------------------------------------------------------------------------

fn view_pull_running(_state: &AppState, total: usize, done: usize) -> Element<'_, Message> {
    column![
        text(format!("Smart Pull in progress… ({done} / {total})")).size(14),
        text("Operations run sequentially to avoid conflicts.").size(12),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Done — result summary
// ---------------------------------------------------------------------------

fn view_done<'a>(state: &'a AppState, result: &'a SyncResult) -> Element<'a, Message> {
    let title = match result.kind {
        SyncKind::Fetch     => "Fetch Complete",
        SyncKind::SmartPull => "Smart Pull Complete",
    };

    let summary = if result.all_succeeded() {
        format!("✓ All {} project(s) succeeded.", result.success_count())
    } else {
        format!("⚠ {} succeeded, {} failed.", result.success_count(), result.fail_count())
    };

    let mut rows: Vec<Element<'_, Message>> = vec![
        text(title).size(18).into(),
        text(summary).size(13).into(),
    ];

    for outcome in &result.per_project {
        rows.push(view_project_outcome(outcome));
    }

    // Recovery hints.
    if !result.recovery_hints.is_empty() {
        rows.push(text("Recovery hints:").size(13).into());
        for hint in &result.recovery_hints {
            rows.push(text(format!("  {}: {}", hint.project_id, hint.situation)).size(12).into());
            for cmd in &hint.suggested_commands {
                rows.push(text(format!("    $ {cmd}")).size(11).into());
            }
        }
    }

    // Action buttons.
    let mut action_row = row![
        button(text("Back to Dashboard"))
            .on_press(Message::Navigate(crate::state::Screen::Dashboard)),
        button(text("Run Again"))
            .on_press(Message::Sync(SyncMessage::SmartPullCancelled)),
    ]
    .spacing(8);

    if result.fail_count() > 0 {
        action_row = action_row.push(
            button(text("Retry Failed"))
                .on_press(Message::Sync(SyncMessage::RetryFailedRequested)),
        );
    }

    rows.push(action_row.into());

    column(rows).spacing(8).padding(24).into()
}

fn view_project_outcome(outcome: &ProjectOutcome) -> Element<'_, Message> {
    let status_icon = if outcome.success { "✓" } else { "✗" };
    let status_label = if outcome.success { "ok" } else { "failed" };

    column![
        row![
            text(format!("{status_icon} {} — {status_label}", outcome.project_name)).size(13),
        ]
        .spacing(4),
        // Show first command executed for transparency.
        if let Some(cmd) = outcome.commands_executed.first() {
            { let e: Element<'_, Message> = text(format!("  $ {cmd}")).size(10).into(); e }
        } else {
            Space::new().into()
        },
        // Show stderr if failed.
        if !outcome.success && !outcome.stderr.is_empty() {
            let preview: String = outcome.stderr.lines().take(3).collect::<Vec<_>>().join("\n");
            { let e: Element<'_, Message> = text(format!("  {preview}")).size(10).into(); e }
        } else {
            Space::new().into()
        },
    ]
    .spacing(2)
    .into()
}

// ---------------------------------------------------------------------------
// Shortcut hint
// ---------------------------------------------------------------------------

fn shortcut_hint<'a>() -> Element<'a, Message> {
    text("Tip: Ctrl+R refreshes the dashboard status").size(11).into()
}
