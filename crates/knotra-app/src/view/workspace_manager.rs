//! Workspace management dialogs.
//!
//! RFC-034 R9: these three dialogs are the validating migration for the
//! overlay host. Content is unchanged from RFC-023; only the surrounding
//! surface (opaque, via [`knotra_ui::widget::overlay`]) and its routing
//! (through `AppLayout::dialog` in `view.rs`, giving the scrim and input
//! blocking) changed.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button, guided_field_focused,
    overlay::{OverlayWidth, surface},
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{
        AppState,
        focus::{FocusOrder, FocusTarget},
        workspace_mgr,
    },
};

/// Stable keys for the non-text-input `FocusTarget`s these dialogs share
/// (RFC-036 Stage 3), kept alongside [`focus_order`] so the order and the
/// dialog it describes cannot drift apart.
mod focus_target {
    pub const CONFIRM: &str = "workspace_mgr.dialog.confirm";
    pub const CANCEL: &str = "workspace_mgr.dialog.cancel";
    pub const CLOSE: &str = "workspace_mgr.dialog.close";
}

/// The focus order for whichever workspace-manager dialog is open, or
/// `None` if none is (RFC-036 R5/R6/R7). Each target's activation `Message`
/// mirrors the same view's own `on_press`/`on_press_maybe` gating exactly
/// (R3), and the order's first entry is that dialog's entry point (R6): the
/// name field for create/rename, matching `focus_input`'s existing
/// auto-focus; `Cancel` — the safe action — for delete, which has no field.
pub fn focus_order(state: &AppState) -> Option<FocusOrder<Message>> {
    if let Some(dialog) = &state.workspace_mgr.create_dialog {
        return Some(name_dialog_focus_order(
            dialog.name.trim().is_empty(),
            WorkspaceMessage::CreateWorkspaceConfirmed,
            WorkspaceMessage::CreateWorkspaceCancelled,
        ));
    }
    if let Some(dialog) = &state.workspace_mgr.rename_dialog {
        return Some(name_dialog_focus_order(
            dialog.new_name.trim().is_empty(),
            WorkspaceMessage::RenameWorkspaceConfirmed,
            WorkspaceMessage::RenameWorkspaceCancelled,
        ));
    }
    if let Some(dialog) = &state.workspace_mgr.confirm_delete {
        let can_delete = state.all_workspaces.len() > 1 && dialog.error.is_none();
        return Some(delete_dialog_focus_order(can_delete));
    }
    None
}

/// Shared shape for the create and rename dialogs: field (entry), Confirm
/// (gated on a non-empty name, exactly like the view's own
/// `guided_button` call), Cancel, header close — the last two dispatch the
/// same `cancel` message the view wires to both controls.
fn name_dialog_focus_order(
    name_is_empty: bool,
    confirm: WorkspaceMessage,
    cancel: WorkspaceMessage,
) -> FocusOrder<Message> {
    vec![
        (
            FocusTarget::text_input(knotra_ui::widget::focus_id::WORKSPACE_NAME.clone()),
            None,
        ),
        (
            FocusTarget::control(focus_target::CONFIRM),
            (!name_is_empty).then_some(Message::Workspace(confirm)),
        ),
        (
            FocusTarget::control(focus_target::CANCEL),
            Some(Message::Workspace(cancel.clone())),
        ),
        (
            FocusTarget::control(focus_target::CLOSE),
            Some(Message::Workspace(cancel)),
        ),
    ]
}

/// Delete has no text field, so Cancel — not Confirm — is the entry point
/// (R6): a destructive dialog should not default focus onto its destructive
/// action.
fn delete_dialog_focus_order(can_delete: bool) -> FocusOrder<Message> {
    vec![
        (
            FocusTarget::control(focus_target::CANCEL),
            Some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled,
            )),
        ),
        (
            FocusTarget::control(focus_target::CONFIRM),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceConfirmed,
            )),
        ),
        (
            FocusTarget::control(focus_target::CLOSE),
            Some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled,
            )),
        ),
    ]
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
            state.t("workspace.name_label"),
            state.t("workspace.name_hint"),
            value,
            |s| Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(s)),
            error,
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
        ),
        NameDialog::Rename(_) => guided_field_focused(
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

    let footer = row![
        guided_button(
            confirm_label,
            (!value.trim().is_empty()).then_some(Message::Workspace(confirm)),
            reason,
        ),
        Space::new().width(Length::Fill),
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(cancel.clone())),
    ]
    .align_y(Alignment::Center);

    surface(
        &state.theme.tokens,
        OverlayWidth::Standard,
        title,
        Some(Message::Workspace(cancel)),
        field,
        footer,
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

    let footer = row![
        guided_button(
            state.t("workspace.delete.confirm"),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceConfirmed
            )),
            reason,
        ),
        Space::new().width(Length::Fill),
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled
            )),
    ]
    .align_y(Alignment::Center);

    let body = column![
        text(body_text).size(FONT_BODY),
        text(project_count).size(FONT_SMALL),
    ]
    .spacing(8);

    surface(
        &state.theme.tokens,
        OverlayWidth::Standard,
        state.t("workspace.delete.title"),
        Some(Message::Workspace(
            WorkspaceMessage::DeleteWorkspaceCancelled,
        )),
        body,
        footer,
    )
}
