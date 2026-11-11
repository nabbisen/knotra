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
    widget::{Space, button, column, container, row, scrollable, text},
};
use knotra_ui::widget::{
    SMALL_BUTTON_HEIGHT, Tokens, Tone, WIDE_CONTENT_WIDTH, body_badge_diameter,
    centered_button_label, centered_button_row, icon, list, record_row, status_badge,
    status_symbol, strong, style, styled_text_input,
};
use knotra_vcs::model::operation::{OperationLog, OperationResult, ProjectOperationOutcome};

use crate::{
    message::{HistoryMessage, Message},
    state::{AppState, dashboard::badge_for_operation_outcome},
    view::dashboard::WidthMode,
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
    let tokens = &state.theme.tokens;
    // RFC-034 R13: per-screen back navigation removed — Dashboard/History are
    // reached through the persistent shell now, not a screen-owned button.
    row![strong(
        text(state.t("history.title")).size(snora::design::style::text::heading_size(tokens))
    )]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

// ---------------------------------------------------------------------------
// Search toolbar
// ---------------------------------------------------------------------------

fn view_toolbar(state: &AppState) -> Element<'_, Message> {
    let tokens = state.theme.tokens.clone();
    // RFC-073 handoff Amendment A3 §2: `BUTTON_HEIGHT` through the shared
    // builder.
    let input = styled_text_input(
        &tokens,
        state.t("history.search_hint"),
        &state.history_search,
    )
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
    let tokens = &state.theme.tokens;
    // RFC-047 D3: a directory that could not be read is its own state, not
    // "no history yet" — checked before the empty-list branch below, since
    // an unreadable directory always yields zero logs and would otherwise
    // be indistinguishable from a genuine first run.
    if state.history_directory_unreadable {
        return empty_state(tokens, state.t("history.directory_unreadable"));
    }

    if state.operation_logs.is_empty() {
        // RFC-047 D2: "N entries could not be read" is the more important
        // message when the list is otherwise empty — it replaces
        // `history.empty` rather than sitting beside it, since there is
        // nothing else on screen for it to sit above.
        return if state.history_unreadable_count > 0 {
            empty_state(tokens, unreadable_count_message(state))
        } else {
            empty_state(tokens, state.t("history.empty"))
        };
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
                    project_name_matches_search(state, p, &q)
                        || p.stdout.to_lowercase().contains(&q)
                        || p.stderr.to_lowercase().contains(&q)
                })
        })
        .map(|log| view_log_entry(state, log))
        .collect();

    if entries.is_empty() {
        return empty_state(tokens, state.t("history.no_match"));
    }

    // RFC-071 D8a: the history entries are the list; a notice above a
    // populated list (RFC-047 D2) is not one of its entries, so it sits
    // outside `list(...)` the same way the dashboard's section header
    // sits outside its own row-list, rather than picking up a separator
    // line of its own.
    let notice = (state.history_unreadable_count > 0).then(|| {
        // RFC-071 Amendment A3 D14: a status message, read to understand
        // what happened — `body`.
        Element::from(
            text(unreadable_count_message(state))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        )
    });

    let mut list_body = column![].spacing(6).padding(12);
    if let Some(notice) = notice {
        list_body = list_body.push(notice);
    }
    let list_column = list_body.push(list(tokens, entries));

    // RFC-071 D7: the same wide-band ceiling Dashboard and Settings use —
    // an entry's result and actions then sit within `WIDE_CONTENT_WIDTH` of
    // its title, left-aligned at the toolbar's own edge rather than
    // centred. Standard/Compact are already narrower than the ceiling, so
    // they are left as plain fill, same as Dashboard's own match.
    match state.width_mode {
        WidthMode::Wide => container(list_column.width(Length::Fixed(WIDE_CONTENT_WIDTH)))
            .align_left(Length::Fill)
            .into(),
        WidthMode::Compact | WidthMode::Standard => list_column.into(),
    }
}

fn unreadable_count_message(state: &AppState) -> String {
    format!(
        "{} {}",
        state.t("history.unreadable_count_label"),
        state.history_unreadable_count
    )
}

/// H4: near the content origin, not vertically centred in a large reserved
/// block. Was `container(...).height(250).center_y(250)` — the message sat
/// 125px down inside a fixed dead area regardless of how little content
/// surrounded it. Now top-of-content, ordinary padding, no reserved height.
///
/// RFC-047: takes `impl Into<String>` rather than `&str` so the two states
/// that carry a count (`unreadable_count_message`) can build an owned
/// message alongside the two that don't (`state.t(...)`), through the same
/// helper rather than a parallel one.
fn empty_state(tokens: &Tokens, message: impl Into<String>) -> Element<'static, Message> {
    container(
        text(message.into())
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    )
    .width(Length::Fill)
    .padding(24)
    .into()
}

// ---------------------------------------------------------------------------
// Single log entry
// ---------------------------------------------------------------------------

fn view_log_entry<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let result = &log.result;
    let expanded = state.history_expanded.contains(&result.operation_id);

    let status = summarise_status(result);
    let timestamp = result
        .started_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let project_count = result.per_project.len();

    // RFC-050 D3: the shared disclosure vocabulary (`plain.show_details`/
    // `plain.hide_details`, already used by five other call sites) rather
    // than this file's own `history.expand`/`history.collapse` — whose
    // Japanese used 閉じる ("close") for a control that closes nothing;
    // `action.close` already owns that word for actual closing.
    let toggle_label = if expanded {
        state.t("plain.hide_details")
    } else {
        state.t("plain.show_details")
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
    //
    // RFC-071 Amendment A3 D14: `status_label`/`toggle_label`/`copy_log`
    // move to `body` here too — a status and a button's own label are both
    // explicitly in D14's "read to act or understand" category, not the
    // incidental metadata the secondary row below still is.
    let primary_row = row![
        // RFC-071 D1: the entry's title — body, strong, same as a dashboard
        // project name.
        strong(
            text(super::operation_kind_label(state, &result.kind))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ),
        Space::new().width(Length::Fill),
        // RFC-073 D9: `status_badge`, not a plain-text glyph.
        row![
            status_badge(
                tokens,
                status.badge.0,
                &status.badge.1,
                body_badge_diameter(tokens)
            ),
            text(state.t(status.label_key))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        // RFC-071 D5: an entry's own actions are not the page's main act —
        // both become `secondary`, not the bare-default (effectively
        // filled) look they had before. Neither is a focus target today
        // (History has no focus-order wiring at all yet), so `false` is
        // passed explicitly rather than left out — the ring ships in case
        // one is ever wired up, same choice Task 100's snackbar made.
        // RFC-073 D8: `SMALL_BUTTON_HEIGHT` — a per-entry control in a
        // potentially long list, the floor rather than `BUTTON_HEIGHT`.
        button(centered_button_row(
            row![
                text(toggle_label)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                icon::icon_element(&chevron)
            ]
            .spacing(4)
        ))
        .height(SMALL_BUTTON_HEIGHT)
        .padding([0, 10])
        .on_press(Message::History(HistoryMessage::EntryToggled(op_id_toggle)))
        .style({
            let t = tokens.clone();
            move |_theme, status| style::with_focus_ring(&t, false, style::secondary(&t, status))
        }),
        button(centered_button_label(
            text(state.t("history.copy_log"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ))
        .height(SMALL_BUTTON_HEIGHT)
        .padding([0, 10])
        .on_press(Message::CopyToClipboard(export_text(
            result,
            &copied_log_names(state, result)
        )))
        .style({
            let t = tokens.clone();
            move |_theme, status| style::with_focus_ring(&t, false, style::secondary(&t, status))
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // RFC-071 Amendment A3 D14: kept `body_small` — a timestamp and a count
    // inside metadata are D14's own named examples of incidental metadata.
    let secondary_row = row![
        text(timestamp)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        text(format!(
            "{} {}",
            state.t("history.project_count_label"),
            project_count
        ))
        .size(snora::design::style::text::body_small_size(tokens))
        .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
// Project name resolution (Task 103)
// ---------------------------------------------------------------------------

/// Task 103: what History calls a project named in a logged
/// `ProjectOperationResult` — one resolver, used by all three display sites
/// (details, the copied log, search) and by nothing else. Each caller
/// decides how to draw its own case; this only decides *which* case.
enum ProjectNameResolution {
    /// The log's own stored name — the common case for any entry logged by
    /// this version of knotra or later.
    Stored(String),
    /// No stored name (an entry logged before this field existed), but a
    /// project with this id is still registered in some workspace — its
    /// name as of right now, which may differ from what it was called when
    /// the operation ran.
    Current(String),
    /// No stored name, and no project with this id in any workspace today.
    Removed,
}

fn resolve_project_name(
    state: &AppState,
    pr: &knotra_vcs::model::operation::ProjectOperationResult,
) -> ProjectNameResolution {
    if let Some(name) = &pr.project_name {
        ProjectNameResolution::Stored(name.clone())
    } else if let Some(name) = state.find_project_name(&pr.project_id) {
        ProjectNameResolution::Current(name)
    } else {
        ProjectNameResolution::Removed
    }
}

/// Task 103 Amendment A1: History search matches **both** the stored name
/// and the project's current name, independently — not
/// [`resolve_project_name`]'s single answer, which picks the stored name
/// over the current one whenever a stored name exists (so that a later
/// rename never rewrites what an entry's own *details* say happened). A
/// search box has no such reason to prefer one: a project renamed after an
/// operation must still be found by the name the dashboard shows today, as
/// well as by the name it had when the operation ran. Both are
/// case-insensitive; the id is never matched (no screen shows it any
/// more). A project with neither a stored name nor a current registration
/// (removed, from an old log) never matches — there is no name text to
/// search by. `q` is already lower-cased by the caller.
fn project_name_matches_search(
    state: &AppState,
    pr: &knotra_vcs::model::operation::ProjectOperationResult,
    q: &str,
) -> bool {
    let stored_matches = pr
        .project_name
        .as_deref()
        .is_some_and(|name| name.to_lowercase().contains(q));
    let current_matches = state
        .find_project_name(&pr.project_id)
        .is_some_and(|name| name.to_lowercase().contains(q));
    stored_matches || current_matches
}

/// Task 103: the details view's own use of [`resolve_project_name`] — a
/// name when there is one, `history.removed_project` (translated) when
/// there is not. Pulled out as its own pure function so this one line of
/// policy (which text each case shows) is directly testable, the same
/// reason `shows_error`/`u32_field_error` exist in other view modules.
fn detail_project_name(
    state: &AppState,
    pr: &knotra_vcs::model::operation::ProjectOperationResult,
) -> String {
    match resolve_project_name(state, pr) {
        ProjectNameResolution::Stored(name) | ProjectNameResolution::Current(name) => name,
        ProjectNameResolution::Removed => state.t("history.removed_project").to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Expanded detail
// ---------------------------------------------------------------------------

/// RFC-073 handoff Amendment A3 §3: `stderr`'s first `take` lines, each
/// with its own four-space indent — not the indent prepended once to the
/// whole joined string, which left every line after the first at the
/// panel's bare left edge (Git's "hint: " continuation lines, for one).
fn indented_stderr_preview(stderr: &str, take: usize) -> String {
    stderr
        .lines()
        .take(take)
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn view_log_detail<'a>(state: &'a AppState, log: &'a OperationLog) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
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
        // RFC-071 Amendment A3 D14: a status message — `body`.
        rows.push(
            text(rb_text)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
                .into(),
        );
    }

    // Per-project results. RFC-071 Amendment A3 D14: this whole expanded
    // block is list content/reasons, read to understand and act on a
    // result — `body`, including the raw command lines (they are the
    // list's own content, not incidental metadata beside something else).
    for pr in &result.per_project {
        // RFC-073 D9: the same badge the dashboard itself uses, not a
        // screen-specific glyph.
        let (tone, glyph) = badge_for_operation_outcome(pr.effective_outcome());
        let name = detail_project_name(state, pr);
        rows.push(
            row![
                status_badge(tokens, tone, &glyph, body_badge_diameter(tokens)),
                text(name)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .padding(Padding {
                left: 14.0,
                ..Padding::ZERO
            })
            .into(),
        );
        if let Some(reason) = &pr.skip_reason {
            rows.push(
                text(format!("    {}", super::skip_reason_display(state, reason)))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .into(),
            );
        }

        // Commands (transparency).
        if !pr.commands_executed.is_empty() {
            rows.push(
                text(state.t("history.commands_header"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .into(),
            );
            for cmd in &pr.commands_executed {
                rows.push(
                    text(format!("    $ {cmd}"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .into(),
                );
            }
        }

        // Stderr excerpt on failure.
        if pr.is_failed() && !pr.stderr.is_empty() {
            rows.push(
                text(indented_stderr_preview(&pr.stderr, 3))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .into(),
            );
        }
    }

    // Recovery hints.
    if !log.recovery_hints.is_empty() {
        rows.push(
            text(state.t("history.recovery_header"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
                .into(),
        );
        for hint in &log.recovery_hints {
            rows.push(
                text(format!("  {}", hint.situation))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .into(),
            );
            for cmd in &hint.suggested_commands {
                rows.push(
                    text(format!("    $ {cmd}"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .into(),
                );
            }
        }
    }

    column(rows).spacing(2).padding([4, 12]).into()
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
/// second English mapping that could drift from the code. RFC-046 D1 made
/// this the only shape `skip_reason` is ever written in going forward
/// (`app/sync.rs`'s prior locale-baked-text writer was fixed at the source,
/// not papered over here); a log entry written before that fix still holds
/// rendered prose, and this export inherits whatever locale was active when
/// such an entry was logged, same as the on-screen path's
/// `skip_reason_display` fallback (`view.rs`) — RFC-046 D4's deliberate
/// compatibility with pre-fix records, not a live defect.
/// Task 103: resolves each `result.per_project` entry's display name for
/// [`export_text`] — computed here, where `state` is available, rather
/// than inside `export_text` itself, which must not gain an `&AppState`
/// parameter (see its own doc comment, R7a). "Removed project" is a literal
/// English string by the same existing exemption the rest of the copied
/// log already relies on — the export is English by design, so this is not
/// routed through `state.t()`. One name per `result.per_project`, same
/// order, same length.
fn copied_log_names(state: &AppState, result: &OperationResult) -> Vec<String> {
    result
        .per_project
        .iter()
        .map(|pr| match resolve_project_name(state, pr) {
            ProjectNameResolution::Stored(name) | ProjectNameResolution::Current(name) => name,
            ProjectNameResolution::Removed => "Removed project".to_owned(),
        })
        .collect()
}

/// `names` is [`copied_log_names`]'s output — one entry per
/// `result.per_project`, same order — kept as a parameter rather than
/// resolved inside this function because `export_text` itself must not
/// gain an `&AppState` parameter (see this function's own doc comment
/// above, R7a); the caller resolves names where `state` is available and
/// hands the plain strings in.
fn export_text(result: &OperationResult, names: &[String]) -> String {
    let kind = result.kind.to_string();
    let ts = result
        .started_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let status = summarise_status(result);
    let status_text = format!("{} {}", status.glyph, status.label_en);
    let mut text_parts = vec![format!("# {} — {} — {}", kind, ts, status_text)];
    for (pr, name) in result.per_project.iter().zip(names) {
        let ok = match pr.effective_outcome() {
            ProjectOperationOutcome::Succeeded => "ok",
            ProjectOperationOutcome::Failed => "FAILED",
            ProjectOperationOutcome::Skipped => "SKIPPED",
        };
        text_parts.push(format!("  {name} ({}) [{}]", pr.project_id, ok));
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
    // RFC-073 D9: the on-screen badge — `glyph`/`label_en` stay as they
    // were, since `export_text` below needs a plain-text indicator, not a
    // coloured `Element`. "Rolled back" and "Partial" are aggregates
    // across many projects, not one of `ProjectOperationOutcome`'s three
    // values, so they are not read from `badge_for_operation_outcome`:
    // `Rolled back` (the attempt failed, but recovery succeeded) and
    // `Rollback failed`/`Failed` all get `danger`/alert — the words beside
    // them, not the symbol, carry which one happened; `Partial` gets its
    // own `warning` tone, since it is genuinely neither a clean success
    // nor a clean failure.
    badge: (Tone, snora::Icon),
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
                badge: (Tone::Danger, status_symbol::alert()),
            }
        } else {
            StatusSummary {
                glyph: "✗",
                label_key: "history.status_rollback_failed",
                label_en: "Rollback failed",
                badge: (Tone::Danger, status_symbol::alert()),
            }
        }
    } else if succeeded > 0 && failed == 0 && skipped == 0 {
        StatusSummary {
            glyph: "✓",
            label_key: "history.status_success",
            label_en: "Success",
            badge: (Tone::Success, status_symbol::all_set()),
        }
    } else if failed > 0 && (succeeded > 0 || skipped > 0) {
        StatusSummary {
            glyph: "⚠",
            label_key: "history.status_partial",
            label_en: "Partial",
            badge: (Tone::Warning, status_symbol::alert()),
        }
    } else if skipped > 0 && failed == 0 {
        StatusSummary {
            glyph: "-",
            label_key: "history.status_skipped",
            label_en: "Skipped",
            badge: (Tone::Info, status_symbol::skipped()),
        }
    } else {
        StatusSummary {
            glyph: "✗",
            label_key: "history.status_failed",
            label_en: "Failed",
            badge: (Tone::Danger, status_symbol::alert()),
        }
    }
}

/// RFC-073 handoff Amendment A3 §1: the same success/partial/failure tone
/// and icon decision this file already makes for its own entries
/// (including the rollback cases, which a simple success/failed/skipped
/// count does not know about), exposed for the activity strip's aggregate
/// summary — one source of truth for what "partial" means, rather than a
/// second, narrower guess living beside it.
pub(crate) fn aggregate_badge(result: &OperationResult) -> (Tone, snora::Icon) {
    summarise_status(result).badge
}

#[cfg(test)]
mod tests {
    use knotra_vcs::model::{
        operation::{OperationId, OperationKind, ProjectOperationResult},
        project::ProjectId,
    };
    use knotra_vcs::{Project, Workspace};

    use super::*;

    /// RFC-073 handoff Amendment A3 §3: a two-line stderr — every line
    /// gets the same four-space indent, not only the first.
    #[test]
    fn indented_stderr_preview_indents_every_line() {
        let preview = indented_stderr_preview("error: failed to push\nhint: try fetch first", 3);
        assert_eq!(
            preview,
            "    error: failed to push\n    hint: try fetch first"
        );
    }

    /// `take` still caps how many lines are shown, each one indented.
    #[test]
    fn indented_stderr_preview_respects_take() {
        let preview = indented_stderr_preview("one\ntwo\nthree\nfour", 2);
        assert_eq!(preview, "    one\n    two");
    }

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

    fn succeeded_project(id: ProjectId, name: &str) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            project_name: Some(name.to_owned()),
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

    fn failed_project(id: ProjectId, name: &str) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            project_name: Some(name.to_owned()),
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

    fn skipped_project(
        id: ProjectId,
        name: &str,
        skip_reason: Option<String>,
    ) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id: id,
            project_name: Some(name.to_owned()),
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
        let result = sample_result(vec![succeeded_project(id.clone(), "dirty-1")], false, None);

        let text = export_text(&result, &["dirty-1".to_owned()]);

        assert!(
            text.starts_with("# Fetch — "),
            "expected an English kind label, got: {text}"
        );
        assert!(
            text.contains("✓ Success"),
            "expected the fixed English status label, got: {text}"
        );
        assert!(
            text.contains(&format!("dirty-1 ({id}) [ok]")),
            "expected the project's name, then its id in parens, then its outcome tag, got: {text}"
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
        let skipped = skipped_project(
            id,
            "clean-1",
            Some("retry:not_in_active_workspace".to_owned()),
        );
        let result = sample_result(vec![skipped], false, None);

        let text = export_text(&result, &["clean-1".to_owned()]);

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
            project_name: Some("ahead-1".to_owned()),
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

        let text = export_text(&result, &["ahead-1".to_owned()]);

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

    /// Handoff 061/062: nothing keeps `StatusSummary::label_en` agreeing
    /// with the English catalog entry `label_key` names — if they drift, the
    /// export and the on-screen UI disagree about what a status is called,
    /// silently. Driving `summarise_status` for each of its six arms (rather
    /// than asserting a table of six literal pairs) is the point only if
    /// each fixture is also checked to land on the arm it claims to —
    /// otherwise a fixture that fell through to a neighbouring arm would
    /// still pass, since that arm's own pairing holds too. Each case
    /// therefore carries its own expected `label_key` and is checked twice:
    /// coverage (the fixture reached the arm it names) before pairing
    /// (`label_en` agrees with the catalog for whichever arm that was).
    #[test]
    fn label_en_matches_the_english_catalog_for_every_status_arm() {
        let en = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::En);
        let id = ProjectId::new();

        let cases: Vec<(&str, OperationResult, &str)> = vec![
            (
                "rolled back",
                sample_result(vec![succeeded_project(id.clone(), "p")], true, Some(true)),
                "history.rollback_note",
            ),
            (
                "rollback failed",
                sample_result(vec![succeeded_project(id.clone(), "p")], true, Some(false)),
                "history.status_rollback_failed",
            ),
            (
                "success",
                sample_result(vec![succeeded_project(id.clone(), "p")], false, None),
                "history.status_success",
            ),
            (
                "partial",
                sample_result(
                    vec![
                        succeeded_project(id.clone(), "p"),
                        failed_project(id.clone(), "p"),
                    ],
                    false,
                    None,
                ),
                "history.status_partial",
            ),
            (
                "skipped",
                sample_result(vec![skipped_project(id.clone(), "p", None)], false, None),
                "history.status_skipped",
            ),
            (
                "failed",
                sample_result(vec![failed_project(id.clone(), "p")], false, None),
                "history.status_failed",
            ),
        ];

        for (case, result, expected_key) in cases {
            let summary = summarise_status(&result);
            assert_eq!(
                summary.label_key, expected_key,
                "the `{case}` fixture no longer reaches the `{expected_key}` arm of \
                 summarise_status — it landed on `{}` instead",
                summary.label_key
            );
            assert_eq!(
                summary.label_en,
                en.t(summary.label_key),
                "label_en drifted from the English catalog for the `{case}` status arm \
                 (label_key = `{}`)",
                summary.label_key
            );
        }
    }

    // -------------------------------------------------------------------
    // Task 103: project name resolution.
    // -------------------------------------------------------------------

    fn state_with_registered_project(project: &Project) -> AppState {
        let mut state = AppState::new(crate::config::AppConfig::default());
        let mut workspace = Workspace::new("Demo");
        workspace.projects.push(project.clone());
        state.all_workspaces = vec![workspace];
        state
    }

    fn result_with_name(
        project_id: ProjectId,
        project_name: Option<String>,
    ) -> ProjectOperationResult {
        ProjectOperationResult {
            project_id,
            project_name,
            outcome: ProjectOperationOutcome::Succeeded,
            success: true,
            skip_reason: None,
            commands_executed: Vec::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            error_message: None,
        }
    }

    /// The resolver's three cases: a stored name wins even when the
    /// project is also still registered (so a rename since the operation
    /// ran does not retroactively rewrite history); no stored name falls
    /// back to the current registered name; neither falls back to removed.
    #[test]
    fn resolve_project_name_covers_all_three_cases() {
        let project = Project::new("current-name", "/tmp/p");
        let state = state_with_registered_project(&project);

        let stored = result_with_name(project.id.clone(), Some("name-as-logged".to_owned()));
        assert!(matches!(
            resolve_project_name(&state, &stored),
            ProjectNameResolution::Stored(name) if name == "name-as-logged"
        ));

        let no_stored_name_but_registered = result_with_name(project.id.clone(), None);
        assert!(matches!(
            resolve_project_name(&state, &no_stored_name_but_registered),
            ProjectNameResolution::Current(name) if name == "current-name"
        ));

        let removed = result_with_name(ProjectId::new(), None);
        assert!(matches!(
            resolve_project_name(&state, &removed),
            ProjectNameResolution::Removed
        ));
    }

    /// Details: a removed project shows the translated label, not a blank
    /// or a raw id.
    #[test]
    fn detail_project_name_shows_the_removed_label_when_nothing_resolves() {
        let state = AppState::new(crate::config::AppConfig::default());
        let removed = result_with_name(ProjectId::new(), None);

        assert_eq!(
            detail_project_name(&state, &removed),
            state.t("history.removed_project")
        );
    }

    /// Details: a stored name is shown verbatim.
    #[test]
    fn detail_project_name_shows_the_stored_name() {
        let state = AppState::new(crate::config::AppConfig::default());
        let result = result_with_name(ProjectId::new(), Some("dirty-1".to_owned()));

        assert_eq!(detail_project_name(&state, &result), "dirty-1");
    }

    /// Search: matches the stored name case-insensitively, and a removed
    /// project (no name to search by) never matches.
    #[test]
    fn project_name_matches_search_is_case_insensitive_and_never_matches_removed() {
        let state = AppState::new(crate::config::AppConfig::default());
        let named = result_with_name(ProjectId::new(), Some("Dirty-1".to_owned()));
        let removed = result_with_name(ProjectId::new(), None);

        assert!(project_name_matches_search(&state, &named, "dirty"));
        assert!(!project_name_matches_search(&state, &named, "clean"));
        assert!(!project_name_matches_search(&state, &removed, "dirty"));
    }

    /// Task 103 Amendment A1: a project renamed since the operation ran is
    /// found by *either* name — the one the entry stored, and the one the
    /// dashboard shows today — not only whichever `resolve_project_name`
    /// would pick for display.
    #[test]
    fn project_name_matches_search_finds_both_the_stored_and_the_current_name_after_a_rename() {
        let project = Project::new("new-name", "/tmp/p");
        let state = state_with_registered_project(&project);
        let result = result_with_name(project.id.clone(), Some("old-name".to_owned()));

        assert!(project_name_matches_search(&state, &result, "old"));
        assert!(project_name_matches_search(&state, &result, "new"));
        assert!(!project_name_matches_search(&state, &result, "other"));
    }

    /// The copied log: a removed project is named in English, by its own
    /// exemption (`export_text`'s own doc comment) — not the translated
    /// catalog key `detail_project_name` uses.
    #[test]
    fn copied_log_names_uses_removed_project_in_english() {
        let state = AppState::new(crate::config::AppConfig::default());
        let result = sample_result(vec![result_with_name(ProjectId::new(), None)], false, None);

        assert_eq!(
            copied_log_names(&state, &result),
            vec!["Removed project".to_owned()]
        );
    }
}
