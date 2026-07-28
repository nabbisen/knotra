//! History view — searchable, expandable operation log.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};
use knotra_vcs::model::operation::{OperationLog, OperationResult, ProjectOperationOutcome};

use crate::{
    message::{HistoryMessage, Message},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let toolbar = view_toolbar(state);
    let body = view_body(state);

    column![header, toolbar, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn view_header(state: &AppState) -> Element<'_, Message> {
    // RFC-034 R13: per-screen back navigation removed — Dashboard/History are
    // reached through the persistent shell now, not a screen-owned button.
    row![text(state.t("history.title")).size(20)]
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
    .padding(Padding {
        top: 0.0,
        bottom: 8.0,
        left: 12.0,
        right: 12.0,
    })
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
    let result = &log.result;
    let expanded = state.history_expanded.contains(&result.operation_id);

    let status_label = summarise_status(result);
    let timestamp = result
        .started_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let project_count = result.per_project.len();

    let toggle_label = if expanded {
        state.t("history.collapse")
    } else {
        state.t("history.expand")
    };

    let op_id_toggle = result.operation_id.clone();
    let _op_id_copy = result.operation_id.clone();

    let summary_row = row![
        text(result.kind.to_string()).size(13),
        text(format!("  {timestamp}")).size(11),
        text(format!("  {project_count} project(s)")).size(11),
        Space::new().width(Length::Fill),
        text(status_label).size(12),
        button(text(toggle_label).size(11))
            .on_press(Message::History(HistoryMessage::EntryToggled(op_id_toggle))),
        button(text(state.t("history.copy_log")).size(11)).on_press({
            // Build a text representation of the log entry for clipboard.
            let kind = result.kind.to_string();
            let ts = result
                .started_at
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string();
            let status = summarise_status(result);
            let mut text_parts = vec![format!("# {} — {} — {}", kind, ts, status)];
            for pr in &result.per_project {
                let ok = match pr.effective_outcome() {
                    ProjectOperationOutcome::Succeeded => "ok",
                    ProjectOperationOutcome::Failed => "FAILED",
                    ProjectOperationOutcome::Skipped => "SKIPPED",
                };
                text_parts.push(format!("  {} [{}]", pr.project_id, ok));
                if let Some(reason) = &pr.skip_reason {
                    text_parts.push(format!("    {}", skip_reason_text(state, reason)));
                }
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

    container(col).width(Length::Fill).padding([8, 12]).into()
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
            if result.rollback_succeeded == Some(true) {
                "succeeded"
            } else {
                "FAILED"
            }
        );
        rows.push(text(rb_text).size(11).into());
    }

    // Per-project results.
    for pr in &result.per_project {
        let icon = match pr.effective_outcome() {
            ProjectOperationOutcome::Succeeded => "✓",
            ProjectOperationOutcome::Failed => "✗",
            ProjectOperationOutcome::Skipped => "-",
        };
        rows.push(text(format!("  {icon} {}", pr.project_id)).size(12).into());
        if let Some(reason) = &pr.skip_reason {
            rows.push(
                text(format!("    {}", skip_reason_text(state, reason)))
                    .size(10)
                    .into(),
            );
        }

        // Commands (transparency).
        if !pr.commands_executed.is_empty() {
            rows.push(text(state.t("history.commands_header")).size(10).into());
            for cmd in &pr.commands_executed {
                rows.push(text(format!("    $ {cmd}")).size(10).into());
            }
        }

        // Stderr excerpt on failure.
        if pr.is_failed() && !pr.stderr.is_empty() {
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

fn skip_reason_text<'a>(state: &'a AppState, reason: &'a str) -> &'a str {
    knotra_vcs::model::operation::RetryExclusionReason::from_code(reason)
        .map(|reason| state.t(reason.i18n_key()))
        .unwrap_or(reason)
}

// ---------------------------------------------------------------------------
// Status label helper
// ---------------------------------------------------------------------------

fn summarise_status(result: &OperationResult) -> &'static str {
    let succeeded = result.successful_projects().len();
    let failed = result.failed_projects().len();
    let skipped = result.skipped_projects().len();

    if result.rollback_attempted {
        if result.rollback_succeeded == Some(true) {
            "↩ Rolled back"
        } else {
            "✗ Rollback failed"
        }
    } else if succeeded > 0 && failed == 0 && skipped == 0 {
        "✓ Success"
    } else if failed > 0 && (succeeded > 0 || skipped > 0) {
        "⚠ Partial"
    } else if skipped > 0 && failed == 0 {
        "- Skipped"
    } else {
        "✗ Failed"
    }
}

// ---------------------------------------------------------------------------
// Log-to-Markdown rendering (used by LogCopyRequested handler)
// ---------------------------------------------------------------------------

/// Render one [`OperationLog`] as a Markdown string suitable for the clipboard.
///
/// Format:
/// ```
/// # Operation: <kind>
/// Started:  <RFC-3339>
/// Finished: <RFC-3339>
/// Status:   Success | Partial | Failed | Rolled back
///
/// ## Projects
///
/// ### <project_id> — ✓ / ✗
/// Commands:
///   $ <cmd>
/// Stdout:
///   <first 20 lines>
/// Stderr:
///   <first 10 lines>
///
/// ## Recovery Hints
/// ### <situation>
///   $ <cmd>
///   See also: <url>
/// ```
#[allow(dead_code)]
pub(crate) fn log_to_markdown(log: &knotra_vcs::OperationLog) -> String {
    let result = &log.result;
    let succeeded = result.successful_projects().len();
    let failed = result.failed_projects().len();
    let skipped = result.skipped_projects().len();

    let status = if result.rollback_attempted {
        if result.rollback_succeeded == Some(true) {
            "Rolled back"
        } else {
            "Rollback failed"
        }
    } else if succeeded > 0 && failed == 0 && skipped == 0 {
        "Success"
    } else if failed > 0 && (succeeded > 0 || skipped > 0) {
        "Partial"
    } else if skipped > 0 && failed == 0 {
        "Skipped"
    } else {
        "Failed"
    };

    let mut md = format!(
        "# Operation: {}\nStarted:  {}\nFinished: {}\nStatus:   {}\n\n## Projects\n\n",
        result.kind,
        result.started_at.to_rfc3339(),
        result.finished_at.to_rfc3339(),
        status,
    );

    for pr in &result.per_project {
        let icon = match pr.effective_outcome() {
            ProjectOperationOutcome::Succeeded => "✓ Success",
            ProjectOperationOutcome::Failed => "✗ Failed",
            ProjectOperationOutcome::Skipped => "- Skipped",
        };
        md.push_str(&format!("### {} — {}\n", pr.project_id, icon));
        if let Some(reason) = &pr.skip_reason {
            md.push_str(&format!("Reason: {reason}\n"));
        }

        if !pr.commands_executed.is_empty() {
            md.push_str("Commands:\n");
            for cmd in &pr.commands_executed {
                md.push_str(&format!("  $ {cmd}\n"));
            }
        }
        if !pr.stdout.is_empty() {
            let preview: String = pr
                .stdout
                .lines()
                .take(20)
                .map(|l| format!("  {l}\n"))
                .collect();
            md.push_str(&format!("Stdout:\n{preview}"));
        }
        if !pr.stderr.is_empty() {
            let preview: String = pr
                .stderr
                .lines()
                .take(10)
                .map(|l| format!("  {l}\n"))
                .collect();
            md.push_str(&format!("Stderr:\n{preview}"));
        }
        md.push('\n');
    }

    if !log.recovery_hints.is_empty() {
        md.push_str("## Recovery Hints\n\n");
        for hint in &log.recovery_hints {
            md.push_str(&format!("### {}\n", hint.situation));
            for cmd in &hint.suggested_commands {
                md.push_str(&format!("  $ {cmd}\n"));
            }
            if let Some(ref url) = hint.see_also {
                md.push_str(&format!("  See also: {url}\n"));
            }
            md.push('\n');
        }
    }
    md
}
