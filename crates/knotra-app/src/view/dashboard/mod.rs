//! Dashboard view module: grouping, sorting, filtering, and bulk selection.
//!
//! Split from a single `dashboard.rs` in RFC-035 Stage 2 commit 1 (move
//! only, byte-identical) into `toolbar`, `section`, `row`, and `empty`
//! submodules by render responsibility. This file keeps the module's
//! external entry point, `view`, plus the two functions (`view_header`,
//! `view_body`) that compose across submodules.
//!
//! **RFC-075 stage 2 §4**: `focus_order` retired — every control here is
//! a real Tab stop through its own `.id(...)` now (`view::ids`), reading
//! order by construction (D2), with nothing left to declare.

use iced::widget::{column, container, scrollable, text};
use iced::{Alignment, Element, Length, Padding};

use knotra_ui::widget::{WIDE_CONTENT_WIDTH, strong};

use crate::{
    message::Message,
    state::{AppState, LoadPhase},
};

mod empty;
mod row;
mod section;
mod toolbar;
mod width_mode;

use empty::{empty_workspace, no_matches, view_without_workspace};
use section::view_section;
use toolbar::view_toolbar;
pub(crate) use width_mode::WidthMode;

/// RFC-035 R22's card arrow-navigation (Handoff 032), RFC-075 D6: a coarser
/// traversal than Tab's own — row-name ids only, skipping section headers,
/// row checkboxes, and row actions, which is what makes arrow movement
/// worth having alongside Tab rather than a redundant copy of it. A
/// parallel walk of the same `state.dashboard_display().sections`
/// iteration every row's real id (`view::ids::row::name`) comes from.
///
/// Sections still respect `collapsed` for the same reason the old,
/// retired `focus_order` did: a collapsed section's rows were never in
/// `state.dashboard_display`'s visible entries to begin with, so they are
/// excluded for free, not by a separate check.
pub fn card_focus_ids(state: &AppState) -> Vec<iced::widget::Id> {
    let display = state.dashboard_display();
    let mut ids = Vec::new();

    for section in &display.sections {
        if section.collapsed {
            continue;
        }
        for entry in &section.entries {
            ids.push(crate::view::ids::row::name(&entry.project.id));
        }
    }

    ids
}

/// `mode` is `state.width_mode`, read once in `view.rs` and passed to both
/// `dashboard::view` and `selection_bar::view` (Handoff 027 Ruling 6.2;
/// reversed from a `responsive` measurement to a state field by Handoff 029
/// — see `width_mode.rs`'s module doc for the full history).
pub fn view(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    let mut body = column![view_header(state)].height(Length::Fill).spacing(4);
    // RFC-073 D1: with no projects in the active workspace, neither the
    // toolbar (chips, Arrange, Search, Select) nor the selection bar (whose
    // only entry point is the toolbar's Select button) is drawn — only the
    // welcome block (`view_body`'s own `empty_workspace`/
    // `view_without_workspace`) remains.
    if !has_no_projects(state) {
        body = body.push(view_toolbar(state, mode));
    }
    body = body.push(scrollable(view_body(state, mode)).height(Length::Fill));

    // RFC-071 Amendment A3 D14: a status message — `body`.
    if let Some(message) = &state.status_bar {
        body = body.push(
            container(
                text(message)
                    .size(snora::design::style::text::body_size(&state.theme.tokens))
                    .line_height(snora::design::style::text::body_line_height(
                        &state.theme.tokens,
                    )),
            )
            .width(Length::Fill)
            .padding([3, 12]),
        );
    }

    body.into()
}

/// RFC-071 D8: the dashboard's own "Check now" (and its "Checking…" status
/// swap) is gone — the shell header's own refresh control (D10) is now the
/// only "Check now" on screen, visible from every screen rather than only
/// here. This leaves the header as a plain title, the same shape History's
/// and Settings' own headers already use (RFC-034 R13/R14: the workspace
/// name lives in the shell switcher, not repeated here).
fn view_header(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    iced::widget::row![strong(
        text(state.t("nav.dashboard")).size(snora::design::style::text::heading_size(tokens))
    )]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

/// RFC-073 D1: true with no active workspace, or a workspace with no
/// projects yet — the two cases `view_body` itself already distinguishes
/// to pick a welcome screen, collapsed here into the single "is there
/// nothing to filter, arrange, or search over" question `view` and
/// `focus_order` ask before drawing the toolbar.
fn has_no_projects(state: &AppState) -> bool {
    state
        .workspace
        .as_ref()
        .is_none_or(|workspace| workspace.projects.is_empty())
}

fn view_body(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    if state.workspace.is_none() {
        return view_without_workspace(state);
    }

    if has_no_projects(state) {
        return empty_workspace(state);
    }

    let display = state.dashboard_display();
    let mut content: Vec<Element<'_, Message>> = Vec::new();
    match &state.load_phase {
        // RFC-071 Amendment A3 D14: a status — `body`.
        LoadPhase::Startup | LoadPhase::Refreshing => content.push(
            container(
                text(state.t("plain.status.checking"))
                    .size(snora::design::style::text::body_size(&state.theme.tokens))
                    .line_height(snora::design::style::text::body_line_height(
                        &state.theme.tokens,
                    )),
            )
            .width(Length::Fill)
            .padding([5, 12])
            .into(),
        ),
        LoadPhase::Ready => {}
    }

    if display.sections.is_empty() {
        content.push(no_matches(state));
    } else {
        for section in display.sections {
            content.push(view_section(state, section, mode));
        }
    }
    // RFC-071 D8: the gap above a group header (between one section and
    // the next) is `tokens.spacing.lg` — "section spacing" by the scale's
    // own naming — deliberately larger than the gap between a header and
    // its own rows (`view_section`'s `tokens.spacing.sm`) and larger still
    // than the ~0px between rows themselves (D8a's hairline, no spacing of
    // its own), so a header reads as a heading rather than just another
    // row.
    let content_column = column(content)
        .spacing(state.theme.tokens.spacing.lg)
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 16.0,
            left: 12.0,
        });

    match mode {
        // RFC-071 D7: content left-aligned, at most `WIDE_CONTENT_WIDTH` —
        // bounding the column itself to a fixed width and aligning that
        // within the full available width to the left edge (the toolbar's
        // own edge above it), rather than centring it. Tracks still do not
        // grow indefinitely, same reasoning as the centred version this
        // replaces; only the alignment changed.
        WidthMode::Wide => container(content_column.width(Length::Fixed(WIDE_CONTENT_WIDTH)))
            .align_left(Length::Fill)
            .into(),
        WidthMode::Compact | WidthMode::Standard => content_column.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use knotra_vcs::{
        ConflictStatus, Project, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsKind,
        WorkingTreeStatus, Workspace, WorkspaceStatus,
    };

    /// A project with no `WorkspaceStatus` entry classifies as
    /// `NeedsHelp`/`StatusUnknown` (`state/dashboard.rs::classify`'s `else`
    /// branch) — the one tier never collapsed regardless of config, so this
    /// alone is enough for a visible card.
    fn needs_help_project(name: &str) -> Project {
        Project::new(name, "/tmp")
    }

    /// A clean status (no conflict, no relevant counts) classifies as
    /// `AllSet` — collapsed by default (`AppConfig::default()`'s
    /// `dashboard_all_set_collapsed: true`).
    fn clean_status(project_id: knotra_vcs::ProjectId) -> ProjectStatus {
        ProjectStatus {
            project_id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: RemoteStatus::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: ConflictStatus::default(),
            refreshed_at: jiff::Timestamp::now(),
            read_error: None,
        }
    }

    #[test]
    fn card_focus_ids_contains_only_row_name_ids_in_display_order() {
        let mut state = AppState::new(AppConfig::default());
        let a = needs_help_project("alpha");
        let b = needs_help_project("beta");
        let (a_id, b_id) = (a.id.clone(), b.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![a, b],
            ..Workspace::new("Test")
        });

        let ids = card_focus_ids(&state);
        assert_eq!(
            ids,
            vec![
                crate::view::ids::row::name(&a_id),
                crate::view::ids::row::name(&b_id),
            ],
            "only the two row-name ids, in display order — no checkboxes, \
             section headers, or row actions"
        );
    }

    #[test]
    fn card_focus_ids_skips_rows_in_a_collapsed_all_set_section() {
        let mut state = AppState::new(AppConfig::default());
        // Asserted explicitly so this test fails loudly if the default ever
        // changes, rather than silently passing for the wrong reason.
        assert!(state.config.dashboard_all_set_collapsed);

        let needs_help = needs_help_project("alpha");
        let all_set = needs_help_project("beta");
        let needs_help_id = needs_help.id.clone();
        let all_set_id = all_set.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![needs_help, all_set],
            ..Workspace::new("Test")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![clean_status(all_set_id)],
            last_refresh: None,
        });

        let ids = card_focus_ids(&state);
        assert_eq!(
            ids,
            vec![crate::view::ids::row::name(&needs_help_id)],
            "the AllSet-tier project sits in a collapsed section and must \
             be excluded"
        );
    }
}
