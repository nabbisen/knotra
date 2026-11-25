//! Workspace management dialogs.
//!
//! RFC-034 R9: these three dialogs are the validating migration for the
//! overlay host. Content is unchanged from RFC-023; only the surrounding
//! surface (opaque, via [`knotra_ui::widget::overlay`]) and its routing
//! (through `AppLayout::dialog` in `view.rs`, giving the scrim and input
//! blocking) changed.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    danger_control_maybe, guided_field_focused,
    overlay::{OverlayWidth, surface},
    primary_control_maybe, secondary_control,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{AppState, workspace_mgr},
};

/// Whether the Confirm button's disabled-reason text should render.
/// Reproduces `guided_button`'s own rule verbatim: the reason shows only
/// while the button is genuinely disabled. `pub(crate)` so it can be
/// tested directly rather than by inspecting rendered output, which this
/// codebase's test suite has no existing way to do.
pub(crate) fn confirm_shows_reason(has_on_press: bool, reason: Option<&str>) -> bool {
    !has_on_press && reason.is_some()
}

/// RFC-075 stage 3 §3: the entry for whichever workspace-manager dialog
/// is open, or `None` if none is. The name field for create/rename; the
/// safe action, `Cancel`, for delete, which has no field (R6: a
/// destructive dialog should not default focus onto its destructive
/// action).
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    if state.workspace_mgr.create_dialog.is_some() || state.workspace_mgr.rename_dialog.is_some() {
        return Some(knotra_ui::widget::focus_id::WORKSPACE_NAME.clone());
    }
    if state.workspace_mgr.confirm_delete.is_some() {
        return Some(crate::view::ids::workspace_manager::delete_cancel());
    }
    None
}

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    if let Some(dialog) = &state.workspace_mgr.create_dialog {
        return Some(workspace_name_dialog(state, NameDialog::Create(dialog)));
    }

    if let Some(dialog) = &state.workspace_mgr.rename_dialog {
        return Some(workspace_name_dialog(state, NameDialog::Rename(dialog)));
    }

    if let Some(dialog) = &state.workspace_mgr.confirm_delete {
        return Some(delete_dialog(state, dialog));
    }

    None
}

#[derive(Clone, Copy)]
enum NameDialog<'a> {
    Create(&'a workspace_mgr::CreateWorkspaceDialog),
    Rename(&'a workspace_mgr::RenameWorkspaceDialog),
}

fn workspace_name_dialog<'a>(state: &'a AppState, dialog: NameDialog<'a>) -> Element<'a, Message> {
    let (title, confirm_label, value, error, confirm, cancel) = match dialog {
        NameDialog::Create(dialog) => (
            state.t("workspace.create.title"),
            state.t("workspace.create.confirm"),
            dialog.name.as_str(),
            dialog.error.as_deref(),
            WorkspaceMessage::CreateWorkspaceConfirmed,
            WorkspaceMessage::CreateWorkspaceCancelled,
        ),
        NameDialog::Rename(dialog) => (
            state.t("workspace.rename.title"),
            state.t("workspace.rename.confirm"),
            dialog.new_name.as_str(),
            dialog.error.as_deref(),
            WorkspaceMessage::RenameWorkspaceConfirmed,
            WorkspaceMessage::RenameWorkspaceCancelled,
        ),
    };

    let field = match dialog {
        NameDialog::Create(_) => guided_field_focused(
            &state.theme.tokens,
            state.t("workspace.name_label"),
            state.t("workspace.name_hint"),
            value,
            |s| Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(s)),
            error,
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
        ),
        NameDialog::Rename(_) => guided_field_focused(
            &state.theme.tokens,
            state.t("workspace.name_label"),
            state.t("workspace.name_hint"),
            value,
            |s| Message::Workspace(WorkspaceMessage::RenameWorkspaceNameChanged(s)),
            error,
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
        ),
    };

    let reason = if value.trim().is_empty() {
        Some(state.t("workspace.error.empty_name"))
    } else {
        None
    };

    let tokens = &state.theme.tokens;
    let confirm_on_press = (!value.trim().is_empty()).then_some(Message::Workspace(confirm));
    let confirm_btn = primary_control_maybe(
        tokens,
        confirm_label,
        confirm_on_press.clone(),
        crate::view::ids::workspace_manager::confirm(),
    );
    let confirm: Element<'_, Message> = if confirm_shows_reason(confirm_on_press.is_some(), reason)
    {
        column![
            confirm_btn,
            // RFC-071 Amendment A3 D14: a reason — `body`.
            text(reason.unwrap_or_default())
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ]
        .spacing(6)
        .into()
    } else {
        confirm_btn
    };

    let cancel_btn = secondary_control(
        tokens,
        state.t("action.cancel"),
        Message::Workspace(cancel.clone()),
        crate::view::ids::workspace_manager::cancel(),
    );

    let footer =
        row![confirm, Space::new().width(Length::Fill), cancel_btn].align_y(Alignment::Center);

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Standard.resolve(state.window_width),
            title,
            Some(Message::Workspace(cancel)),
            crate::view::ids::workspace_manager::close(),
            field,
            footer,
        ),
        &[],
    )
}

fn delete_dialog<'a>(
    state: &'a AppState,
    dialog: &'a crate::state::workspace_mgr::DeleteWorkspaceDialog,
) -> Element<'a, Message> {
    let body_text = format!(
        "{} \"{}\". {}",
        state.t("workspace.delete.body_prefix"),
        dialog.workspace_name,
        state.t("workspace.delete.body_suffix"),
    );
    let project_count = format!(
        "{} {}",
        dialog.project_count,
        state.t("workspace.delete.project_count_suffix"),
    );

    let can_delete = state.all_workspaces.len() > 1 && dialog.error.is_none();
    let reason = if state.all_workspaces.len() <= 1 {
        Some(state.t("workspace.delete.disabled_last"))
    } else {
        dialog.error.as_deref()
    };

    let tokens = &state.theme.tokens;
    let confirm_on_press = can_delete.then_some(Message::Workspace(
        WorkspaceMessage::DeleteWorkspaceConfirmed,
    ));
    let confirm_btn = danger_control_maybe(
        tokens,
        state.t("workspace.delete.confirm"),
        confirm_on_press.clone(),
        crate::view::ids::workspace_manager::confirm_delete(),
    );
    let confirm: Element<'_, Message> = if confirm_shows_reason(confirm_on_press.is_some(), reason)
    {
        column![
            confirm_btn,
            // RFC-071 Amendment A3 D14: a reason — `body`.
            text(reason.unwrap_or_default())
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ]
        .spacing(6)
        .into()
    } else {
        confirm_btn
    };

    let cancel_btn = secondary_control(
        tokens,
        state.t("action.cancel"),
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceCancelled),
        crate::view::ids::workspace_manager::delete_cancel(),
    );

    let footer =
        row![confirm, Space::new().width(Length::Fill), cancel_btn].align_y(Alignment::Center);

    let body = column![
        text(body_text)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        // RFC-071 Amendment A3 D14: a second sentence of the dialog's own
        // body, warning of a consequence — `body`, not a metadata count.
        text(project_count)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ]
    .spacing(8);

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Standard.resolve(state.window_width),
            state.t("workspace.delete.title"),
            Some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled,
            )),
            crate::view::ids::workspace_manager::close(),
            body,
            footer,
        ),
        &[],
    )
}
