# RFC-0001 — Complete `HistoryMessage::LogCopyRequested`

| Field       | Value                                      |
|-------------|--------------------------------------------|
| Status      | Implemented (v0.11.0) |
| Priority    | High — user-visible regression             |
| Effort      | Small (≈ 1 function, ≈ 20 lines)           |
| Related     | `crates/knotra-app/src/app.rs`, `view/history.rs` |

## Summary

`HistoryMessage::LogCopyRequested(id)` currently **does not write anything to
the clipboard**.  The handler updates the status bar with a placeholder message
and returns without calling `Message::CopyToClipboard`.  The Copy button on the
History screen appears to work but silently does nothing.

## Background

The clipboard write path is fully wired:
`Message::CopyToClipboard(text) => clipboard::write(text)` in the iced update
dispatcher (line 132 of `app.rs`).  Only the bridge from
`LogCopyRequested(id)` to `CopyToClipboard(text)` is missing.

## Problem

```rust
// app.rs  ← current (broken)
HistoryMessage::LogCopyRequested(_id) => {
    // Real clipboard write is handled by Message::CopyToClipboard.
    state.status_bar = Some("Log copied to clipboard...".to_owned());
    Task::none()
}
```

`_id` is never used: the log text is never generated and
`Message::CopyToClipboard` is never returned.

## Design

### Markdown format for a log entry

```
# Operation: <OperationKind>
Started:  <started_at RFC-3339>
Finished: <finished_at RFC-3339>
Status:   Success | Partial | Failed [| Rolled back]

## Projects

### <project_name> — ✓ Success | ✗ Failed
Commands:
  $ <cmd1>
  $ <cmd2>
Stdout:
  <first 20 lines>
Stderr:
  <first 10 lines>

## Recovery Hints
### <situation>
  $ <suggested_command>
  See also: <url>
```

### Implementation

Add a free function in `view/history.rs` (or a module-private helper in
`app.rs`):

```rust
/// Render one `OperationLog` as a Markdown string suitable for clipboard.
pub fn log_to_markdown(log: &OperationLog) -> String {
    let result   = &log.result;
    let status   = if result.rollback_attempted {
        if result.rollback_succeeded == Some(true) { "Rolled back" } else { "Rollback failed" }
    } else if result.all_succeeded() { "Success" }
      else if result.any_failed()    { "Partial" }
      else                           { "Failed" };

    let mut md = format!(
        "# Operation: {}\nStarted:  {}\nFinished: {}\nStatus:   {}\n\n## Projects\n\n",
        result.kind,
        result.started_at.to_rfc3339(),
        result.finished_at.to_rfc3339(),
        status,
    );

    for pr in &result.per_project {
        let icon = if pr.success { "✓" } else { "✗" };
        md.push_str(&format!("### {} — {}\n", pr.project_id, icon));
        if !pr.commands_executed.is_empty() {
            md.push_str("Commands:\n");
            for cmd in &pr.commands_executed {
                md.push_str(&format!("  $ {cmd}\n"));
            }
        }
        if !pr.stdout.is_empty() {
            let preview = pr.stdout.lines().take(20).collect::<Vec<_>>().join("\n");
            md.push_str(&format!("Stdout:\n  {preview}\n"));
        }
        if !pr.stderr.is_empty() {
            let preview = pr.stderr.lines().take(10).collect::<Vec<_>>().join("\n");
            md.push_str(&format!("Stderr:\n  {preview}\n"));
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
        }
    }
    md
}
```

Update the handler in `app.rs`:

```rust
HistoryMessage::LogCopyRequested(id) => {
    if let Some(log) = state.operation_logs.iter().find(|l| l.result.operation_id == id) {
        let text = log_to_markdown(log);
        let char_count = text.chars().count();
        state.status_bar = Some(format!("✓ Copied {char_count} characters to clipboard."));
        Task::done(Message::CopyToClipboard(text))
    } else {
        state.status_bar = Some("Log entry not found.".to_owned());
        Task::none()
    }
}
```

Also remove the misleading comment `// Real clipboard write is handled by
Message::CopyToClipboard.` — it will no longer be accurate.

### Status bar message

| Outcome | Message |
|---------|---------|
| Success | `✓ Copied N characters to clipboard.` |
| ID not found | `Log entry not found.` |

The i18n keys `history.copy_log` (button label) already exist; add:

```
history.copy_ok     = "✓ Copied {n} characters to clipboard."
history.copy_miss   = "Log entry not found."
```

## Test Plan

Add two unit tests in `crates/knotra-app/src/tests.rs`:

1. **`log_to_markdown_contains_kind_and_status`** — construct a minimal
   `OperationLog` with a known `OperationKind` and one successful
   `ProjectOperationResult`; assert the returned string contains the kind name
   and `✓`.

2. **`log_to_markdown_includes_recovery_hints`** — construct a log with one
   `RecoveryHint` that has a `suggested_command`; assert the string contains
   `## Recovery Hints` and the command text.

No integration test needed: `Message::CopyToClipboard` is already tested at
the iced level.

## Security Considerations

None.  Log text is generated from in-process data and written only to the local
clipboard on explicit user request.
