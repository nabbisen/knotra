//! History view — searchable, expandable operation log.
//!
//! RFC-038 Stage 4: operation kinds now route through
//! `super::operation_kind_label` (lifted from `activity_strip.rs` so both
//! files share one mapping, §1a) instead of `OperationKind`'s raw English
//! `Display`; the project count uses a "label: count" phrasing that needs
//! no singular/plural branching in either language (§1b); the search
//! toolbar is width-bounded, each entry's summary is a two-line hierarchy
//! (kind/status/actions, then timestamp/count) with a chevron on the
//! disclosure toggle, and both empty states sit near the content origin
//! rather than centred in a 250px dead block (H4, §2); and each entry's
//! summary/detail composition now goes through
//! `knotra_ui::widget::record_row` (D3/R6) instead of building its own
//! `column`/`container`, so RFC-039 can reuse the same collapsible-record
//! shape for per-project history rather than copying it.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};
use knotra_ui::widget::{icon, record_row};
use knotra_vcs::model::operation::{OperationLog, OperationResult, ProjectOperationOutcome};

use crate::{
    message::{HistoryMessage, Message},
    state::AppState,
};

/// Matches `view/settings.rs`'s own bounded-form width (`FORM_MAX_WIDTH`) —
/// the same "large bounded content" convention (H4's "bounded search/filter
/// toolbar" ask), not a new number invented for this screen.
const TOOLBAR_MAX_WIDTH: f32 = 680.0;

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
    let input = text_input(state.t("history.search_hint"), &state.history_search)
        .on_input(|s| Message::History(HistoryMessage::SearchChanged(s)))
        .width(Length::Fill);

    container(input)
        .width(Length::Fill)
        .max_width(TOOLBAR_MAX_WIDTH)
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
        return empty_state(state.t("history.empty"));
    }

    let q = state.history_search.to_lowercase();

    let entries: Vec<Element<'_, Message>> = state
        .operation_logs
        .iter()
        .filter(|log| {
            // Matches against what is now on screen (the plain-language
            // kind label, §1a) rather than `OperationKind`'s raw English
            // `Display` — a user typing part of what they see should find
            // it; matching the technical name they no longer see would not.
            q.is_empty()
                || super::operation_kind_label(state, &log.result.kind)
                    .to_lowercase()
                    .contains(&q)
                || log.result.per_project.iter().any(|p| {
                    p.project_id.to_string().contains(&q)
                        || p.stdout.to_lowercase().contains(&q)
                        || p.stderr.to_lowercase().contains(&q)
                })
        })
        .map(|log| view_log_entry(state, log))
        .collect();

    if entries.is_empty() {
        return empty_state(state.t("history.no_match"));
    }

    column(entries).spacing(6).padding(12).into()
}

/// H4: near the content origin, not vertically centred in a large reserved
/// block. Was `container(...).height(250).center_y(250)` — the message sat
/// 125px down inside a fixed dead area regardless of how little content
/// surrounded it. Now top-of-content, ordinary padding, no reserved height.
fn empty_state(message: &str) -> Element<'_, Message> {
    container(text(message).size(14))
        .width(Length::Fill)
        .padding(24)
        .into()
}

// ---------------------------------------------------------------------------
// Single log entry
// ---------------------------------------------------------------------------

fn view_log_entry<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let result = &log.result;
    let expanded = state.history_expanded.contains(&result.operation_id);

    let status = summarise_status(result);
    let status_label = format!("{} {}", status.glyph, state.t(status.label_key));
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
    // H4: a text-only toggle read as a generic button, not a disclosure
    // control. `chevron_right`/`chevron_down` are the same idiom already
    // used for the dashboard's section headers and the workspace switcher
    // (`view/dashboard/section.rs`) — right reads as "click to open", down
    // as "already open, contents below".
    let chevron = if expanded {
        icon::chevron_down()
    } else {
        icon::chevron_right()
    };

    let op_id_toggle = result.operation_id.clone();

    // H4: a fixed metadata hierarchy — kind, status, and the two actions on
    // one primary line; timestamp and project count, both secondary, on a
    // smaller line beneath. Was one row interleaving all five with `"  "`
    // string-padding standing in for real spacing.
    let primary_row = row![
        text(super::operation_kind_label(state, &result.kind)).size(13),
        Space::new().width(Length::Fill),
        text(status_label).size(12),
        button(
            row![text(toggle_label).size(11), icon::icon_element(&chevron)]
                .spacing(4)
                .align_y(Alignment::Center)
        )
        .on_press(Message::History(HistoryMessage::EntryToggled(op_id_toggle))),
        button(text(state.t("history.copy_log")).size(11))
            .on_press(Message::CopyToClipboard(export_text(result))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let secondary_row = row![
        text(timestamp).size(11),
        text(format!(
            "{} {}",
            state.t("history.project_count_label"),
            project_count
        ))
        .size(11),
    ]
    .spacing(12);

    let summary = column![primary_row, secondary_row].spacing(2);
    let detail = expanded.then(|| view_log_detail(state, log));

    // D3/R6: the summary-always/detail-when-expanded composition this file
    // and RFC-039's per-project rows both need — `_op_id_copy` (a second,
    // unused clone of `operation_id`) is gone here as a natural consequence
    // of this rewrite, not a separate cleanup.
    record_row(summary.into(), detail)
}

// ---------------------------------------------------------------------------
// Expanded detail
// ---------------------------------------------------------------------------

fn view_log_detail<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let result = &log.result;
    let mut rows: Vec<Element<'a, Message>> = Vec::new();

    // Rollback status.
    //
    // Found and fixed alongside `summarise_status` (RFC-038 Stage 1) rather
    // than left as-is — the handoff named `summarise_status` specifically,
    // but this pair sits on the same on-screen "visible path" (rendered
    // whenever a rolled-back entry is expanded), is the identical class of
    // defect, and both halves already had a same-shaped key to reuse
    // (`plain.activity.succeeded`/`plain.activity.failed`, already used the
    // same way in `activity_strip.rs`), so no new catalog entries were
    // needed for this part. The pre-existing "FAILED" (uppercase) vs.
    // "succeeded" (lowercase) inconsistency is also gone as a side effect of
    // routing both through the same existing key pair — not a rewording
    // decision, since neither string's wording changed, only its casing
    // normalized to match the key it now reuses.
    if result.rollback_attempted {
        let rb_text = format!(
            "{}  {}",
            state.t("history.rollback_note"),
            if result.rollback_succeeded == Some(true) {
                state.t("plain.activity.succeeded")
            } else {
                state.t("plain.activity.failed")
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
// Clipboard/export text — RFC-038 A1
// ---------------------------------------------------------------------------

/// Builds this operation's clipboard/export text. RFC-038 A1: the export is
/// English by design — it leaves the app and lands in issue trackers and
/// search boxes where the reader is frequently not the localised user, and
/// each catalog lookup here would be a *variable*-keyed one, the one shape
/// `every_literal_t_call_names_an_existing_key` cannot check; a missing key
/// in a release build would paste the raw catalog key straight into the
/// user's pasted report (`Catalog::t()` is `debug_assert!` then
/// `unwrap_or(key)`). Taking `&OperationResult` rather than `&AppState`
/// makes that guarantee structural: `t()` is a method on `AppState`, so this
/// function cannot regain a catalog lookup without the parameter coming
/// back — R7a is enforced by the signature, not merely followed by the
/// body.
///
/// `skip_reason` is emitted verbatim, not translated: for the stable
/// `RetryExclusionReason` codes (`retry:not_in_active_workspace` and
/// friends, `knotra-vcs/src/model/operation.rs:93`) this is the canonical,
/// greppable identifier — stable across users and locales, needing no
/// second English mapping that could drift from the code. One skip-reason
/// source outside that enum (`app/sync.rs:309`'s "project cannot be checked
/// right now" path) instead stores already-rendered, locale-baked text at
/// write time; for that source the export inherits whatever locale was
/// active when the entry was logged, exactly as the on-screen path already
/// does via `skip_reason_text`'s same fallback. Pre-existing, not
/// introduced here — see the Handoff 060 review request.
fn export_text(result: &OperationResult) -> String {
    let kind = result.kind.to_string();
    let ts = result
        .started_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let status = summarise_status(result);
    let status_text = format!("{} {}", status.glyph, status.label_en);
    let mut text_parts = vec![format!("# {} — {} — {}", kind, ts, status_text)];
    for pr in &result.per_project {
        let ok = match pr.effective_outcome() {
            ProjectOperationOutcome::Succeeded => "ok",
            ProjectOperationOutcome::Failed => "FAILED",
            ProjectOperationOutcome::Skipped => "SKIPPED",
        };
        text_parts.push(format!("  {} [{}]", pr.project_id, ok));
        if let Some(reason) = &pr.skip_reason {
            text_parts.push(format!("    {}", reason));
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
    text_parts.join("\n")
}

// ---------------------------------------------------------------------------
// Status label helper
// ---------------------------------------------------------------------------

/// A history entry's overall status: a stable glyph, the i18n key for its
/// translated on-screen label, and the fixed English label the export uses
/// (RFC-038 A1) — all three set in the same match arm so they cannot drift
/// apart. The glyph is a status signal, not language — composing it in the
/// view keeps a translator from being able to drop or reorder it, and keeps
/// it from being duplicated across both catalogs. `label_en` exists so the
/// export never has to call `t()` to get English text; it must read the same
/// as the English catalog entry named beside it, but nothing enforces that
/// mechanically — if you retext a `history.status_*`/`history.rollback_note`
/// English value, update the matching `label_en` here too.
struct StatusSummary {
    glyph: &'static str,
    label_key: &'static str,
    label_en: &'static str,
}

fn summarise_status(result: &OperationResult) -> StatusSummary {
    let succeeded = result.successful_projects().len();
    let failed = result.failed_projects().len();
    let skipped = result.skipped_projects().len();

    if result.rollback_attempted {
        if result.rollback_succeeded == Some(true) {
            // Reuses the existing `history.rollback_note` key — the same
            // "Rolled back" text `view_log_detail` already renders below.
            StatusSummary {
                glyph: "↩",
                label_key: "history.rollback_note",
                label_en: "Rolled back",
            }
        } else {
            StatusSummary {
                glyph: "✗",
                label_key: "history.status_rollback_failed",
                label_en: "Rollback failed",
            }
        }
    } else if succeeded > 0 && failed == 0 && skipped == 0 {
        StatusSummary {
            glyph: "✓",
            label_key: "history.status_success",
            label_en: "Success",
        }
    } else if failed > 0 && (succeeded > 0 || skipped > 0) {
        StatusSummary {
            glyph: "⚠",
            label_key: "history.status_partial",
            label_en: "Partial",
        }
    } else if skipped > 0 && failed == 0 {
        StatusSummary {
            glyph: "-",
            label_key: "history.status_skipped",
            label_en: "Skipped",
        }
    } else {
        StatusSummary {
            glyph: "✗",
            label_key: "history.status_failed",
            label_en: "Failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use knotra_vcs::model::{
        operation::{OperationId, OperationKind, ProjectOperationResult},
        project::ProjectId,
    };

    use super::*;

    fn sample_result(
        per_project: Vec<ProjectOperationResult>,
        rollback_attempted: bool,
        rollback_succeeded: Option<bool>,
    ) -> OperationResult {
        let now = chrono::Utc::now();
        OperationResult {
            operation_id: OperationId::new(),
            kind: OperationKind::Fetch,
            started_at: now,
            finished_at: now,
            per_project,
            rollback_attempted,
            rollback_succeeded,
        }
    }

    fn succeeded_project(id: ProjectId) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            outcome: ProjectOperationOutcome::Succeeded,
            success: true,
            skip_reason: None,
            commands_executed: vec!["git fetch".to_owned()],
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            error_message: None,
        }
    }

    fn failed_project(id: ProjectId) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            outcome: ProjectOperationOutcome::Failed,
            success: false,
            skip_reason: None,
            commands_executed: Vec::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(1),
            error_message: None,
        }
    }

    fn skipped_project(id: ProjectId, skip_reason: Option<String>) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            outcome: ProjectOperationOutcome::Skipped,
            success: true,
            skip_reason,
            commands_executed: Vec::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            error_message: None,
        }
    }

    /// RFC-038 A1's core guarantee: the export names the operation and its
    /// status in fixed English, never through `t()` — `export_text` cannot
    /// call it, since it does not take `&AppState`.
    #[test]
    fn export_text_reports_a_successful_entry_in_english() {
        let id = ProjectId::new();
        let result = sample_result(vec![succeeded_project(id.clone())], false, None);

        let text = export_text(&result);

        assert!(
            text.starts_with("# Fetch — "),
            "expected an English kind label, got: {text}"
        );
        assert!(
            text.contains("✓ Success"),
            "expected the fixed English status label, got: {text}"
        );
        assert!(
            text.contains(&format!("{id} [ok]")),
            "expected the project id with its outcome tag, got: {text}"
        );
        assert!(
            text.contains("$ git fetch"),
            "expected the executed command, got: {text}"
        );
    }

    /// A skip reason is emitted verbatim — the stable `RetryExclusionReason`
    /// code, not a translated (or mistranslated) sentence — so a maintainer
    /// reading a pasted log sees the same greppable identifier regardless of
    /// the reporting user's locale.
    #[test]
    fn export_text_emits_the_skip_reason_code_verbatim() {
        let id = ProjectId::new();
        let skipped = skipped_project(id, Some("retry:not_in_active_workspace".to_owned()));
        let result = sample_result(vec![skipped], false, None);

        let text = export_text(&result);

        assert!(
            text.contains("retry:not_in_active_workspace"),
            "expected the raw code, untranslated, got: {text}"
        );
    }

    /// Stderr lines are copied through unchanged (already raw command
    /// output, not catalog-sourced), capped at 5 lines as the on-screen path
    /// caps at 3 — both existing behaviour, unaffected by this stage.
    #[test]
    fn export_text_includes_stderr_lines() {
        let id = ProjectId::new();
        let failed = ProjectOperationResult {
            project_id: id,
            outcome: ProjectOperationOutcome::Failed,
            success: false,
            skip_reason: None,
            commands_executed: Vec::new(),
            stdout: String::new(),
            stderr: "fatal: could not read remote\nauthentication failed".to_owned(),
            exit_code: Some(1),
            error_message: Some("fatal: could not read remote".to_owned()),
        };
        let result = sample_result(vec![failed], false, None);

        let text = export_text(&result);

        assert!(
            text.contains("fatal: could not read remote"),
            "expected the first stderr line, got: {text}"
        );
        assert!(
            text.contains("authentication failed"),
            "expected the second stderr line, got: {text}"
        );
        assert!(
            text.contains("[FAILED]"),
            "expected the failed outcome tag, got: {text}"
        );
    }

    /// Handoff 061: nothing keeps `StatusSummary::label_en` agreeing with
    /// the English catalog entry `label_key` names — if they drift, the
    /// export and the on-screen UI disagree about what a status is called,
    /// silently. Driving `summarise_status` for each of its six arms (rather
    /// than asserting a table of six literal pairs) proves the arm this test
    /// believes it reaches is the arm that actually runs.
    #[test]
    fn label_en_matches_the_english_catalog_for_every_status_arm() {
        let en = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::En);
        let id = ProjectId::new();

        let cases: Vec<(&str, OperationResult)> = vec![
            (
                "rolled back",
                sample_result(vec![succeeded_project(id.clone())], true, Some(true)),
            ),
            (
                "rollback failed",
                sample_result(vec![succeeded_project(id.clone())], true, Some(false)),
            ),
            (
                "success",
                sample_result(vec![succeeded_project(id.clone())], false, None),
            ),
            (
                "partial",
                sample_result(
                    vec![succeeded_project(id.clone()), failed_project(id.clone())],
                    false,
                    None,
                ),
            ),
            (
                "skipped",
                sample_result(vec![skipped_project(id.clone(), None)], false, None),
            ),
            (
                "failed",
                sample_result(vec![failed_project(id.clone())], false, None),
            ),
        ];

        for (case, result) in cases {
            let summary = summarise_status(&result);
            assert_eq!(
                summary.label_en,
                en.t(summary.label_key),
                "label_en drifted from the English catalog for the `{case}` status arm \
                 (label_key = `{}`)",
                summary.label_key
            );
        }
    }
}
