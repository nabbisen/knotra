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
    BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_field_focused,
    overlay::{OverlayWidth, surface},
    style,
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

/// Whether the control keyed `key` currently draws the RFC-036 focus ring —
/// plain equality against `state.overlay_focus`, not `focus::resolve`'s
/// stale-target fallback (RFC-036 Stage 5): rendering shows a ring only
/// where knotra-focus genuinely and currently sits, matching `shell.rs`'s
/// `is_focused` precedent from Stage 2. `pub(crate)` so tests can assert
/// against the exact function the view uses, not a re-implementation of it.
pub(crate) fn is_focused(state: &AppState, key: &'static str) -> bool {
    state.overlay_focus.as_ref() == Some(&FocusTarget::control(key))
}

/// Whether the Confirm button's disabled-reason text should render.
/// Reproduces `guided_button`'s own rule verbatim: the reason shows only
/// while the button is genuinely disabled. Neither dialog builder calls
/// `guided_button` any more (RFC-036 Stage 5), so this behaviour has to be
/// preserved explicitly rather than inherited — `pub(crate)` so it can be
/// tested directly rather than by inspecting rendered output, which this
/// codebase's test suite has no existing way to do.
pub(crate) fn confirm_shows_reason(has_on_press: bool, reason: Option<&str>) -> bool {
    !has_on_press && reason.is_some()
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

    let tokens = &state.theme.tokens;
    let confirm_on_press = (!value.trim().is_empty()).then_some(Message::Workspace(confirm));
    let confirm_focused = is_focused(state, focus_target::CONFIRM);
    let confirm_btn = {
        let t = tokens.clone();
        button(text(confirm_label).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press_maybe(confirm_on_press.clone())
            .style(move |_theme, status| {
                style::with_focus_ring(&t, confirm_focused, style::primary(&t, status))
            })
    };
    let confirm: Element<'_, Message> = if confirm_shows_reason(confirm_on_press.is_some(), reason)
    {
        column![
            confirm_btn,
            text(reason.unwrap_or_default()).size(FONT_SMALL)
        ]
        .spacing(6)
        .into()
    } else {
        confirm_btn.into()
    };

    let cancel_focused = is_focused(state, focus_target::CANCEL);
    let cancel_btn = {
        let t = tokens.clone();
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(cancel.clone()))
            .style(move |_theme, status| {
                style::with_focus_ring(&t, cancel_focused, style::secondary(&t, status))
            })
    };

    let footer =
        row![confirm, Space::new().width(Length::Fill), cancel_btn].align_y(Alignment::Center);

    surface(
        tokens,
        OverlayWidth::Standard,
        title,
        Some(Message::Workspace(cancel)),
        is_focused(state, focus_target::CLOSE),
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

    let tokens = &state.theme.tokens;
    let confirm_on_press = can_delete.then_some(Message::Workspace(
        WorkspaceMessage::DeleteWorkspaceConfirmed,
    ));
    let confirm_focused = is_focused(state, focus_target::CONFIRM);
    let confirm_btn = {
        let t = tokens.clone();
        button(text(state.t("workspace.delete.confirm")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press_maybe(confirm_on_press.clone())
            .style(move |_theme, status| {
                style::with_focus_ring(&t, confirm_focused, style::danger(&t, status))
            })
    };
    let confirm: Element<'_, Message> = if confirm_shows_reason(confirm_on_press.is_some(), reason)
    {
        column![
            confirm_btn,
            text(reason.unwrap_or_default()).size(FONT_SMALL)
        ]
        .spacing(6)
        .into()
    } else {
        confirm_btn.into()
    };

    let cancel_focused = is_focused(state, focus_target::CANCEL);
    let cancel_btn = {
        let t = tokens.clone();
        button(text(state.t("action.cancel")).size(FONT_BODY))
            .height(BUTTON_HEIGHT)
            .padding([0, 18])
            .on_press(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceCancelled,
            ))
            .style(move |_theme, status| {
                style::with_focus_ring(&t, cancel_focused, style::secondary(&t, status))
            })
    };

    let footer =
        row![confirm, Space::new().width(Length::Fill), cancel_btn].align_y(Alignment::Center);

    let body = column![
        text(body_text).size(FONT_BODY),
        text(project_count).size(FONT_SMALL),
    ]
    .spacing(8);

    surface(
        tokens,
        OverlayWidth::Standard,
        state.t("workspace.delete.title"),
        Some(Message::Workspace(
            WorkspaceMessage::DeleteWorkspaceCancelled,
        )),
        is_focused(state, focus_target::CLOSE),
        body,
        footer,
    )
}
