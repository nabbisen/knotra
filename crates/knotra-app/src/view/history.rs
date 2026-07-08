//! History view — searchable, expandable operation log.

use endringer::model::operation::{OperationLog, OperationResult};
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{HistoryMessage, Message},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header  = view_header(state);
    let toolbar = view_toolbar(state);
    let body    = view_body(state);

    column![header, toolbar, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn view_header(state: &AppState) -> Element<'_, Message> {
    row![
        button(text("← Dashboard"))
            .on_press(Message::History(HistoryMessage::BackToDashboard)),
        text(state.t("history.title")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Search toolbar
// ---------------------------------------------------------------------------

fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    row![
        text_input(state.t("history.search_hint"), &state.history_search)
            .on_input(|s| Message::History(HistoryMessage::SearchChanged(s)))
            .width(Length::Fill),
    ]
    .padding(Padding { top: 0.0, bottom: 8.0, left: 12.0, right: 12.0 })
    .into()
}

// ---------------------------------------------------------------------------
// Body: log entry list
// ---------------------------------------------------------------------------

fn view_body(state: &AppState) -> Element<'_, Message> {
    if state.operation_logs.is_empty() {
        return container(text(state.t("history.empty")).size(14))
            .width(Length::Fill)
            .height(250)
            .center_x(Length::Fill)
            .center_y(250)
            .into();
    }

    let q = state.history_search.to_lowercase();

    let entries: Vec<Element<'_, Message>> = state
        .operation_logs
        .iter()
        .filter(|log| {
            q.is_empty()
                || log.result.kind.to_string().to_lowercase().contains(&q)
                || log.result.per_project.iter().any(|p| {
                    p.project_id.to_string().contains(&q)
                        || p.stdout.to_lowercase().contains(&q)
                        || p.stderr.to_lowercase().contains(&q)
                })
        })
        .map(|log| view_log_entry(state, log))
        .collect();

    if entries.is_empty() {
        return container(text(state.t("history.no_match")).size(14))
            .width(Length::Fill)
            .height(250)
            .center_x(Length::Fill)
            .center_y(250)
            .into();
    }

    column(entries).spacing(6).padding(12).into()
}

// ---------------------------------------------------------------------------
// Single log entry
// ---------------------------------------------------------------------------

fn view_log_entry<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let result   = &log.result;
    let expanded = state.history_expanded.contains(&result.operation_id);

    let status_label = summarise_status(result);
    let timestamp    = result.started_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let project_count = result.per_project.len();

    let toggle_label = if expanded {
        state.t("history.collapse")
    } else {
        state.t("history.expand")
    };

    let op_id_toggle = result.operation_id.clone();
    let _op_id_copy  = result.operation_id.clone();

    let summary_row = row![
        text(result.kind.to_string()).size(13),
        text(format!("  {timestamp}")).size(11),
        text(format!("  {project_count} project(s)")).size(11),
        Space::new().width(Length::Fill),
        text(status_label).size(12),
        button(text(toggle_label).size(11))
            .on_press(Message::History(HistoryMessage::EntryToggled(op_id_toggle))),
        button(text(state.t("history.copy_log")).size(11))
            .on_press({
                // Build a text representation of the log entry for clipboard.
                let kind   = result.kind.to_string();
                let ts     = result.started_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let status = summarise_status(result);
                let mut text_parts = vec![
                    format!("# {} — {} — {}", kind, ts, status),
                ];
                for pr in &result.per_project {
                    let ok = if pr.success { "ok" } else { "FAILED" };
                    text_parts.push(format!("  {} [{}]", pr.project_id, ok));
                    for cmd in &pr.commands_executed {
                        text_parts.push(format!("    $ {}", cmd));
                    }
                    if !pr.stderr.is_empty() {
                        for line in pr.stderr.lines().take(5) {
                            text_parts.push(format!("    {}", line));
                        }
                    }
                }
                Message::CopyToClipboard(text_parts.join("\n"))
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut col = column![summary_row].spacing(4);

    if expanded {
        col = col.push(view_log_detail(state, log));
    }

    container(col)
        .width(Length::Fill)
        .padding([8, 12])
        .into()
}

// ---------------------------------------------------------------------------
// Expanded detail
// ---------------------------------------------------------------------------

fn view_log_detail<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let result = &log.result;
    let mut rows: Vec<Element<'a, Message>> = Vec::new();

    // Rollback status.
    if result.rollback_attempted {
        let rb_text = format!(
            "{}  {}",
            state.t("history.rollback_note"),
            if result.rollback_succeeded == Some(true) { "succeeded" } else { "FAILED" }
        );
        rows.push(text(rb_text).size(11).into());
    }

    // Per-project results.
    for pr in &result.per_project {
        let icon = if pr.success { "✓" } else { "✗" };
        rows.push(text(format!("  {icon} {}", pr.project_id)).size(12).into());

        // Commands (transparency).
        if !pr.commands_executed.is_empty() {
            rows.push(text(state.t("history.commands_header")).size(10).into());
            for cmd in &pr.commands_executed {
                rows.push(text(format!("    $ {cmd}")).size(10).into());
            }
        }

        // Stderr excerpt on failure.
        if !pr.success && !pr.stderr.is_empty() {
            let preview: String = pr.stderr.lines().take(3).collect::<Vec<_>>().join("\n");
            rows.push(text(format!("    {preview}")).size(10).into());
        }
    }

    // Recovery hints.
    if !log.recovery_hints.is_empty() {
        rows.push(text(state.t("history.recovery_header")).size(11).into());
        for hint in &log.recovery_hints {
            rows.push(text(format!("  {}", hint.situation)).size(11).into());
            for cmd in &hint.suggested_commands {
                rows.push(text(format!("    $ {cmd}")).size(10).into());
            }
        }
    }

    column(rows).spacing(2).padding([4, 12]).into()
}

// ---------------------------------------------------------------------------
// Status label helper
// ---------------------------------------------------------------------------

fn summarise_status(result: &OperationResult) -> &'static str {
    if result.rollback_attempted {
        if result.rollback_succeeded == Some(true) {
            "↩ Rolled back"
        } else {
            "✗ Rollback failed"
        }
    } else if result.per_project.iter().all(|p| p.success) {
        "✓ Success"
    } else if result.per_project.iter().any(|p| p.success) {
        "⚠ Partial"
    } else {
        "✗ Failed"
    }
}
