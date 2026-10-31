// RFC-052 A1: `unused_imports`/`unused_variables` masked nothing in any
// target and are gone. `dead_code` is narrowed to the test build only —
// `view()` and `field_row` (only reachable from inside it) are called from
// `view.rs` in the real binary, but no `#[test]` in this crate calls into
// the render tree, so the test compilation's call graph never reaches them
// and flags both as dead. The binary build carries no suppression at all.
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

use knotra_ui::widget::{Tokens, icon, record_row, style};

use crate::{
    message::{DetailPanelMessage, Message, ProjectMessage, WorkspaceMessage},
    state::{AppState, detail_panel::RecentCommitsPhase},
};

/// RFC-071 Amendment A2 commit 2: the label sits on its own line, the value
/// directly below it — replaces RFC-048 D3's two-column layout, whose fixed
/// `label_width` was always a guessed fit for one language at one font
/// size (Review 248 §4: at 14 px, "Remote:"/"Untracked:" already exceeded
/// their own guessed widths). A stacked layout needs nothing measured: no
/// font, language or zoom level can make a label not fit its own line, and
/// a long value (a path) wraps under its label instead of colliding with
/// it. The label stays `body_small`, now in `text_secondary` so it reads as
/// a caption rather than competing with its own value; the value keeps its
/// role unchanged.
fn field_row<'a>(
    tokens: &Tokens,
    label: &'a str,
    value: impl std::fmt::Display,
) -> Element<'a, Message> {
    column![
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .color(snora::design::style::color::to_iced_color(
                tokens.palette.text_secondary
            )),
        text(value.to_string())
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    ]
    .spacing(1)
    .into()
}

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
    // Review 249 §2: the close control is not the panel's main action — it
    // takes `overlay::surface`'s own close button whole (the lucide `X`
    // icon, `ghost` style, plus a focus ring that never lights, since this
    // panel has no focus-order wiring), not only its colour.
    let close_btn = button(icon::icon_element(&icon::close()))
        .on_press(Message::DetailPanel(DetailPanelMessage::Closed))
        .style({
            let t = tokens.clone();
            move |_theme, status| style::with_focus_ring(&t, false, style::ghost(&t, status))
        });

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
        field_row(tokens, state.t("detail.label_vcs"), vcs),
        field_row(tokens, state.t("detail.label_path"), path),
        field_row(tokens, state.t("detail.label_remote"), remote),
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
            field_row(tokens, state.t("detail.label_branch"), branch),
            field_row(tokens, state.t("detail.label_ahead"), ahead),
            field_row(tokens, state.t("detail.label_behind"), behind),
            field_row(tokens, state.t("detail.label_dirty"), dirty),
            field_row(tokens, state.t("detail.label_untracked"), untracked),
            field_row(tokens, state.t("detail.label_conflict"), conflict),
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
    // Review 249 §2: Refresh and Fetch become `secondary` — neither is the
    // panel's one mutating action. None of the three is a focus target
    // today (no focus-order wiring here), so `false` is passed explicitly.
    let refresh_btn = button(
        text(state.t("detail.refresh"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Project(ProjectMessage::StatusRefreshRequested(
        id.clone(),
    )))
    .style({
        let t = tokens.clone();
        move |_theme, status| style::with_focus_ring(&t, false, style::secondary(&t, status))
    });

    let fetch_btn = button(
        text(state.t("detail.fetch"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press_maybe(
        (!state.operation_interlock.is_busy())
            .then_some(Message::Project(ProjectMessage::FetchRequested(id.clone()))),
    )
    .style({
        let t = tokens.clone();
        move |_theme, status| style::with_focus_ring(&t, false, style::secondary(&t, status))
    });

    // Review 249 §2: "Remove from workspace" becomes `danger` — the one
    // mutating action in this area, styled so it looks like what it does.
    // Its confirmation window (RFC-066 D6) is unchanged.
    let remove_btn = button(
        text(state.t("detail.remove_from_workspace"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    )
    .on_press(Message::Workspace(
        WorkspaceMessage::RemoveProjectRequested(id.clone()),
    ))
    .style({
        let t = tokens.clone();
        move |_theme, status| style::with_focus_ring(&t, false, style::danger(&t, status))
    });

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
