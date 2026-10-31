//! Dashboard view module: grouping, sorting, filtering, and bulk selection.
//!
//! Split from a single `dashboard.rs` in RFC-035 Stage 2 commit 1 (move
//! only, byte-identical) into `toolbar`, `section`, `row`, and `empty`
//! submodules by render responsibility. This file keeps the module's two
//! external entry points, `view` and `focus_order`, plus the two
//! functions (`view_header`, `view_body`) that compose across submodules.

use iced::widget::{column, container, scrollable, text};
use iced::{Alignment, Element, Length, Padding};

use knotra_ui::widget::{WIDE_CONTENT_WIDTH, strong};

use crate::{
    message::{
        ConflictOpsMessage, DashboardMessage, DetailPanelMessage, Message, SelectionMessage,
    },
    state::{
        AppState, LoadPhase,
        dashboard::{DashboardCause, DashboardSectionKey, DashboardTier},
        focus::{FocusOrder, FocusTarget},
    },
};

mod empty;
mod row;
mod section;
mod toolbar;
mod width_mode;

use empty::{empty_workspace, no_matches, view_without_workspace};
use row::{action_key, checkbox_key, name_key};
use section::{focus_key, view_section};
use toolbar::view_toolbar;
pub(crate) use width_mode::WidthMode;

/// Tab/Shift-Tab focus targets for the dashboard (RFC-036 R2, Stage 4;
/// toolbar targets added RFC-035 Handoff 022 §7.4): the toolbar's filter
/// chips, grouping/sorting selects, search, and bulk-selection entry point
/// (`toolbar::focus_order`) — **before** the rows' own targets, matching
/// visual order — then collapsible section headers, row checkboxes
/// (selection mode only), and row actions. Card-to-card `↑`/`↓`/`j`/`k`
/// movement is not this - that is RFC-035's.
///
/// The row portion iterates `DashboardDisplay::sections` in the exact order
/// and with the exact `!collapsed` filter `build_dashboard_display` used to
/// compute `ordered_selectable_ids` - this is that same computation's row
/// targets, not a second ordering (RFC-036 Stage 4 change scope). A
/// dedicated test asserts the two ID sequences are identical.
pub fn focus_order(state: &AppState) -> FocusOrder<Message> {
    let display = state.dashboard_display();
    let mut order = toolbar::focus_order(state, state.width_mode);

    for section in &display.sections {
        if let DashboardSectionKey::Tier(tier) = section.key
            && tier != DashboardTier::NeedsHelp
        {
            order.push((
                FocusTarget::control_dynamic(focus_key(tier)),
                Some(Message::Dashboard(DashboardMessage::TierToggled(tier))),
            ));
        }

        if section.collapsed {
            continue;
        }

        for entry in &section.entries {
            let id = &entry.project.id;

            if state.selection_mode {
                order.push((
                    FocusTarget::control_dynamic(checkbox_key(id)),
                    Some(Message::Selection(SelectionMessage::Toggled(id.clone()))),
                ));
            }

            // The name/detail-link button - present on every row regardless
            // of tier, and the most common row interaction.
            order.push((
                FocusTarget::control_dynamic(name_key(id)),
                Some(Message::DetailPanel(DetailPanelMessage::Opened(id.clone()))),
            ));

            // The tier-specific action button. Only NeedsHelp rows render
            // one (`view_project_row`'s `action` slot is a plain `Space`,
            // not a button, for InProgress/AllSet).
            if entry.tier == DashboardTier::NeedsHelp {
                let action_message = if entry.cause == Some(DashboardCause::Conflict) {
                    (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                        ConflictOpsMessage::OpenRequested(Some(id.clone())),
                    ))
                } else {
                    Some(Message::DetailPanel(DetailPanelMessage::Opened(id.clone())))
                };
                order.push((FocusTarget::control_dynamic(action_key(id)), action_message));
            }
        }
    }

    order
}

/// RFC-035 R22's card arrow-navigation (Handoff 032): a coarser traversal
/// than [`focus_order`]'s Tab order — row-name targets only, skipping
/// section headers, row checkboxes, and row actions, which is what makes
/// arrow movement worth having alongside Tab rather than a redundant copy
/// of it. Not a filter over `focus_order`'s own output (that would still
/// need a way to tell a name target apart from the others by string
/// inspection); instead a parallel walk of the same
/// `state.dashboard_display().sections` iteration, keeping only the push
/// `focus_order` makes unconditionally for every row's name button.
///
/// Sections still respect `collapsed` here for the same reason `focus_order`
/// does: a collapsed section's rows were never in `state.dashboard_display`'s
/// visible entries to begin with, so they are excluded for free, not by a
/// separate check.
pub fn card_focus_order(state: &AppState) -> FocusOrder<Message> {
    let display = state.dashboard_display();
    let mut order = Vec::new();

    for section in &display.sections {
        if section.collapsed {
            continue;
        }
        for entry in &section.entries {
            let id = &entry.project.id;
            order.push((
                FocusTarget::control_dynamic(name_key(id)),
                Some(Message::DetailPanel(DetailPanelMessage::Opened(id.clone()))),
            ));
        }
    }

    order
}

/// `mode` is `state.width_mode`, read once in `view.rs` and passed to both
/// `dashboard::view` and `selection_bar::view` (Handoff 027 Ruling 6.2;
/// reversed from a `responsive` measurement to a state field by Handoff 029
/// — see `width_mode.rs`'s module doc for the full history).
pub fn view(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    let mut body = column![view_header(state), view_toolbar(state, mode)]
        .height(Length::Fill)
        .spacing(4);
    // RFC-071 D7: `scrollable`'s own `width` defaults to `Length::Shrink`
    // (`iced_widget::scrollable::Scrollable::new`) — left unset, the
    // scrollable collapses to its content's natural width instead of the
    // window's, so `view_body`'s wide-band `container(...).align_left(
    // Length::Fill)` never receives any real extra space to position
    // within, and centring never visibly worked either before this stage.
    // Found by direct pixel inspection of the before/after evidence
    // (neither showed any margin), not assumed from the code.
    body = body.push(
        scrollable(view_body(state, mode))
            .width(Length::Fill)
            .height(Length::Fill),
    );

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

fn view_body(state: &AppState, mode: WidthMode) -> Element<'_, Message> {
    if state.workspace.is_none() {
        return view_without_workspace(state);
    }

    let projects_empty = state
        .workspace
        .as_ref()
        .is_none_or(|workspace| workspace.projects.is_empty());
    if projects_empty {
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
            refreshed_at: chrono::Utc::now(),
            read_error: None,
        }
    }

    #[test]
    fn card_focus_order_contains_only_row_name_targets_in_display_order() {
        let mut state = AppState::new(AppConfig::default());
        let a = needs_help_project("alpha");
        let b = needs_help_project("beta");
        let (a_id, b_id) = (a.id.clone(), b.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![a, b],
            ..Workspace::new("Test")
        });

        let order = card_focus_order(&state);
        let targets: Vec<_> = order.iter().map(|(target, _)| target.clone()).collect();
        assert_eq!(
            targets,
            vec![
                FocusTarget::control_dynamic(name_key(&a_id)),
                FocusTarget::control_dynamic(name_key(&b_id)),
            ],
            "only the two row-name targets, in display order — no \
             checkboxes, section headers, or row actions"
        );
        for (_, message) in &order {
            assert!(
                matches!(
                    message,
                    Some(Message::DetailPanel(DetailPanelMessage::Opened(_)))
                ),
                "every card target must activate to opening its detail panel"
            );
        }
    }

    #[test]
    fn card_focus_order_skips_rows_in_a_collapsed_all_set_section() {
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

        let order = card_focus_order(&state);
        let targets: Vec<_> = order.iter().map(|(target, _)| target.clone()).collect();
        assert_eq!(
            targets,
            vec![FocusTarget::control_dynamic(name_key(&needs_help_id))],
            "the AllSet-tier project sits in a collapsed section and must \
             be excluded, the same way its checkbox/action targets already \
             are from focus_order"
        );
    }
}
