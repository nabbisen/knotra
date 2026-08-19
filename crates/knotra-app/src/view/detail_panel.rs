// RFC-052 A1: `unused_imports`/`unused_variables` masked nothing in any
// target and are gone. `dead_code` is narrowed to the test build only —
// `view()`, and `field_row`/`IDENTITY_LABEL_WIDTH`/`STATUS_LABEL_WIDTH`
// (only reachable from inside it), are called/used from `view.rs` in the
// real binary, but no `#[test]` in this crate calls into the render tree,
// so the test compilation's call graph never reaches them and flags all
// four as dead. The binary build carries no suppression at all.
#![cfg_attr(test, allow(dead_code))]
//! RFC-0014 — Project detail side panel.
//!
//! Opens as a right-docked panel when the user clicks a project name.
//! Showing all status fields, recent operations, and available actions.
//!
//! RFC-048: every user-facing string here now routes through `state.t()`
//! under the `detail.*` prefix — the panel made zero `t()` calls before
//! this and was entirely English for every Japanese user. `detail.*` is
//! deliberately not a first-level prefix (RFC-048 D1): this is the expert
//! surface RFC-021's plain-language layer defers *to*, so `Branch`,
//! `Fetch`, `Conflict` etc. are translated, not re-worded.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text},
};

use knotra_ui::widget::{Tokens, record_row};

use crate::{
    message::{DetailPanelMessage, Message, ProjectMessage, WorkspaceMessage},
    state::{AppState, detail_panel::RecentCommitsPhase},
};

/// RFC-048 D3: label/value pairs laid out as two columns sized by the
/// layout engine, not by space-padding the label string to a fixed
/// character count (the pre-RFC-048 shape — `format!("Branch:     {}",
/// branch)`). No catalog value carries alignment whitespace in either
/// locale; `label_width` is chosen per section to fit that section's
/// longest label, matching the old padded columns' per-section grouping
/// rather than one width shared across the whole panel.
fn field_row<'a>(
    tokens: &Tokens,
    label: &'a str,
    value: impl std::fmt::Display,
    label_width: f32,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .width(Length::Fixed(label_width)),
        text(value.to_string())
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    ]
    .into()
}

/// Fits `detail.label_remote` ("Remote:"), the longest label in the
/// Identity section, at the `body_small` role (RFC-056 Stage 2; was a raw
/// `.size(11)`).
const IDENTITY_LABEL_WIDTH: f32 = 56.0;
/// Fits `detail.label_untracked` ("Untracked:"), the longest label in the
/// Status section, at the `body_small` role (RFC-056 Stage 2; was a raw
/// `.size(11)`).
const STATUS_LABEL_WIDTH: f32 = 72.0;

pub fn view<'a>(state: &'a AppState) -> Option<Element<'a, Message>> {
    let tokens = &state.theme.tokens;
    let id = state.detail_panel.open_project_id.as_ref()?;

    let project = state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))?;

    let status = state
        .workspace_status
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|ps| &ps.project_id == id));

    // --- Header ---
    let close_btn = button(
        text("✕")
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::DetailPanel(DetailPanelMessage::Closed));

    let header = row![
        text(project.name.clone())
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    // --- Identity section ---
    let vcs = status
        .map(|s| s.identity.vcs_kind.to_string())
        .unwrap_or_else(|| "—".into());
    let path = project.path.clone();
    let remote = status
        .and_then(|s| s.remote.upstream.clone())
        .unwrap_or_else(|| "—".into());

    let identity = column![
        text(state.t("detail.section_identity"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        field_row(
            tokens,
            state.t("detail.label_vcs"),
            vcs,
            IDENTITY_LABEL_WIDTH
        ),
        field_row(
            tokens,
            state.t("detail.label_path"),
            path,
            IDENTITY_LABEL_WIDTH
        ),
        field_row(
            tokens,
            state.t("detail.label_remote"),
            remote,
            IDENTITY_LABEL_WIDTH
        ),
    ]
    .spacing(3);

    // --- Status section ---
    let status_col = if let Some(s) = status {
        let branch = s.context.as_ref().map(|c| c.label.as_str()).unwrap_or("—");
        let ahead = s.remote.ahead;
        let behind = s.remote.behind;
        let dirty = s.working_tree.uncommitted_count;
        let untracked = s.working_tree.untracked_count;
        let conflict = if s.conflict.has_conflict {
            state.t("detail.conflict_yes")
        } else if s.conflict.detection_unavailable {
            state.t("detail.conflict_unknown")
        } else {
            state.t("detail.conflict_no")
        };

        column![
            text(state.t("detail.section_status"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
            field_row(
                tokens,
                state.t("detail.label_branch"),
                branch,
                STATUS_LABEL_WIDTH
            ),
            field_row(
                tokens,
                state.t("detail.label_ahead"),
                ahead,
                STATUS_LABEL_WIDTH
            ),
            field_row(
                tokens,
                state.t("detail.label_behind"),
                behind,
                STATUS_LABEL_WIDTH
            ),
            field_row(
                tokens,
                state.t("detail.label_dirty"),
                dirty,
                STATUS_LABEL_WIDTH
            ),
            field_row(
                tokens,
                state.t("detail.label_untracked"),
                untracked,
                STATUS_LABEL_WIDTH
            ),
            field_row(
                tokens,
                state.t("detail.label_conflict"),
                conflict,
                STATUS_LABEL_WIDTH
            ),
        ]
        .spacing(3)
    } else {
        column![
            text(state.t("detail.section_status"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
            text(state.t("detail.loading"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
        ]
    };

    // --- Recent operations section (last 5 involving this project) ---
    let recent_ops: Vec<Element<'_, Message>> = state
        .operation_logs
        .iter()
        .rev()
        .filter(|log| log.result.per_project.iter().any(|pp| &pp.project_id == id))
        .take(5)
        .map(|log| {
            let ok = log
                .result
                .per_project
                .iter()
                .find(|pp| &pp.project_id == id)
                .map(|pp| pp.success)
                .unwrap_or(false);
            let icon = if ok { "✓" } else { "✗" };
            // RFC-048 §2: `operation_kind_label` (`view.rs`), not
            // `log.result.kind`'s raw English `Display` — the fourth
            // consumer, after `activity_strip.rs` and `history.rs`'s two.
            text(format!(
                "{} {} — {}",
                icon,
                super::operation_kind_label(state, &log.result.kind),
                log.result.started_at.format("%m/%d %H:%M")
            ))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into()
        })
        .collect();

    let recent = column(
        std::iter::once(
            text(state.t("detail.section_recent_operations"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
                .into(),
        )
        .chain(if recent_ops.is_empty() {
            vec![
                text(state.t("detail.none"))
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
                    .into(),
            ]
        } else {
            recent_ops
        }),
    )
    .spacing(3);

    // --- Recent commits section (RFC-039 D3/D5) ---
    // Three states, three sentences (D5/R5) — none of them an empty section
    // (RFC-044 D3). The VCS layer's `error` string is shown, not discarded
    // (§5's explicit anti-pattern: `ProjectConflictDetail.note`, deleted
    // unread by RFC-045).
    let commits_body: Vec<Element<'_, Message>> = match &state.detail_panel.commits_phase {
        RecentCommitsPhase::Loading(loading_id) if loading_id == id => {
            vec![
                text(state.t("detail.commits_loading"))
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens))
                    .into(),
            ]
        }
        RecentCommitsPhase::Loaded {
            project_id,
            commits,
        } if project_id == id => {
            if let Some(err) = &commits.error {
                vec![
                    text(format!("{} {}", state.t("detail.commits_error"), err))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens))
                        .into(),
                ]
            } else if commits.entries.is_empty() {
                vec![
                    text(state.t("detail.commits_empty"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens))
                        .into(),
                ]
            } else {
                commits
                    .entries
                    .iter()
                    .map(|entry| {
                        let short_hash = &entry.hash[..entry.hash.len().min(7)];
                        let summary = text(format!("{short_hash}  {}", entry.subject))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            ));
                        let detail = text(format!(
                            "{} — {}",
                            entry.author,
                            entry.date.format("%Y-%m-%d %H:%M")
                        ))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens));
                        record_row(summary.into(), Some(detail.into()))
                    })
                    .collect()
            }
        }
        _ => vec![
            text(state.t("detail.commits_loading"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
                .into(),
        ],
    };

    let commits = column(
        std::iter::once(
            text(state.t("detail.section_recent_commits"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
                .into(),
        )
        .chain(commits_body),
    )
    .spacing(3);

    // --- Actions ---
    let refresh_btn = button(
        text(state.t("detail.refresh"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Project(ProjectMessage::StatusRefreshRequested(
        id.clone(),
    )));

    let fetch_btn = button(
        text(state.t("detail.fetch"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press_maybe(
        (!state.operation_interlock.is_busy())
            .then_some(Message::Project(ProjectMessage::FetchRequested(id.clone()))),
    );

    let remove_btn = button(
        text(state.t("detail.remove_from_workspace"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Workspace(
        WorkspaceMessage::RemoveProjectRequested(id.clone()),
    ));

    let actions = column![
        text(state.t("detail.section_actions"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        row![refresh_btn, fetch_btn].spacing(6),
        remove_btn,
    ]
    .spacing(6);

    let content = column![header, identity, status_col, recent, commits, actions,]
        .spacing(16)
        .padding(16);

    Some(
        container(scrollable(content))
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
            .into(),
    )
}
