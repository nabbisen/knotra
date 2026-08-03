//! Dashboard view module: grouping, sorting, filtering, and bulk selection.
//!
//! Split from a single `dashboard.rs` in RFC-035 Stage 2 commit 1 (move
//! only, byte-identical) into `toolbar`, `section`, `row`, and `empty`
//! submodules by render responsibility. This file keeps the module's two
//! external entry points, `view` and `focus_order`, plus the two
//! functions (`view_header`, `view_body`) that compose across submodules.

use iced::widget::{button, column, container, responsive, scrollable, text};
use iced::{Element, Length, Padding};

use crate::{
    message::{
        ConflictOpsMessage, DashboardMessage, DetailPanelMessage, Message, SelectionMessage,
        WorkspaceMessage,
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

use empty::{
    empty_workspace, no_matches, view_confirm_remove_dialog, view_error_notice,
    view_without_workspace,
};
use row::{action_key, checkbox_key, name_key};
use section::{focus_key, view_section};
use toolbar::view_toolbar;
use width_mode::WidthMode;

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
    let mut order = toolbar::focus_order(state);

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

pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut body = column![view_header(state), view_toolbar(state)]
        .height(Length::Fill)
        .spacing(4);
    // RFC-035 R8/Internal Design §Responsive strategy: `responsive` wraps
    // the body region specifically (not the whole window, via `view()`'s
    // own `state` parameter) so its closure's `Size` is the space actually
    // available to the dashboard body, not the window size. `WidthMode` is
    // recomputed fresh every layout pass here and never stored — see
    // `width_mode.rs`'s module doc for why.
    body = body.push(
        responsive(move |size| {
            let mode = WidthMode::from_width(size.width);
            scrollable(view_body(state, mode))
                .height(Length::Fill)
                .into()
        })
        .height(Length::Fill),
    );

    if let Some(message) = &state.status_bar {
        body = body.push(
            container(text(message).size(12))
                .width(Length::Fill)
                .padding([3, 12]),
        );
    }

    if state.confirm_remove_dialog.is_some() {
        return column![body, view_confirm_remove_dialog(state)]
            .height(Length::Fill)
            .into();
    }
    body.into()
}

fn view_header(state: &AppState) -> Element<'_, Message> {
    // RFC-034 R13/R14: the workspace name lives in the shell switcher now,
    // not repeated here. This is the RFC's one migrated page header; the
    // toolbar below (grouping/sorting/filtering/selection) is RFC-035.
    let refresh: Element<'_, Message> = if state.is_refreshing {
        text(state.t("plain.status.checking")).size(13).into()
    } else {
        button(text(state.t("plain.check_now")).size(13))
            .on_press(Message::Workspace(WorkspaceMessage::RefreshRequested))
            .into()
    };

    crate::view::shell::page_header(state.t("nav.dashboard"), refresh)
}

/// `mode` is unused in Stage 4 commit 1 — this signature exists so it does
/// not need to change again in commits 2-4, which are the ones that branch
/// on it (wide centring, compact rows). Commit 1's own claim is only that
/// `responsive` delivers a correct `WidthMode` with no visible change at
/// standard width.
fn view_body(state: &AppState, _mode: WidthMode) -> Element<'_, Message> {
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
        LoadPhase::Startup | LoadPhase::Refreshing => content.push(
            container(text(state.t("plain.status.checking")).size(12))
                .width(Length::Fill)
                .padding([5, 12])
                .into(),
        ),
        LoadPhase::Error(error) => content.push(view_error_notice(
            state,
            error,
            state.t("dashboard.load_failed"),
            true,
        )),
        LoadPhase::Ready => {}
    }

    if display.sections.is_empty() {
        content.push(no_matches(state));
    } else {
        for section in display.sections {
            content.push(view_section(state, section));
        }
    }
    column(content)
        .spacing(8)
        .padding(Padding {
            top: 4.0,
            right: 12.0,
            bottom: 16.0,
            left: 12.0,
        })
        .into()
}
