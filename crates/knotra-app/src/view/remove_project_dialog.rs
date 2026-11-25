//! RFC-066 D6 — Remove project, as a dialog over any screen.
//!
//! Previously rendered inline on the Dashboard only (`view/dashboard/mod.rs`
//! pushed it below the dashboard's own body), so confirming a removal
//! requested from the palette while on History or Settings showed nothing.
//! Moved here, into `AppLayout::dialog`'s slot — the same slot and surface
//! shape "Delete workspace" uses — so it now shows over every screen, with
//! the engine's own scrim and input blocking.

use iced::{
    Alignment, Element,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    danger_control, ghost_control,
    overlay::{OverlayWidth, surface},
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::AppState,
};

/// RFC-075 stage 3 §3: Remove project's own entry — "Keep"
/// (`RemoveProjectConfirmed` starts a change, so it is never the entry).
/// `None` while no confirmation is pending.
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    state
        .confirm_remove_dialog
        .as_ref()
        .map(|_| crate::view::ids::remove_project::keep())
}

/// Render the dialog (call only when `state.confirm_remove_dialog.is_some()`
/// — matched by `view.rs` against `layer::top_layer`, not this function).
pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let dialog = state.confirm_remove_dialog.as_ref()?;
    let tokens = &state.theme.tokens;

    let body = column![
        text(dialog.project_name.as_str())
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        // RFC-071 Amendment A3 D14: a dialog body — `body`.
        text(state.t("plain.remove.body"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ]
    .spacing(8);

    // "Keep" is `ghost`-weight; "Remove from list" is `danger` (RFC D6) —
    // an irreversible action, the same reasoning `buttons.rs`'s own
    // `danger`/`danger_maybe` doc comment gives.
    let footer = row![
        ghost_control(
            tokens,
            state.t("confirm.remove_no"),
            Message::Workspace(WorkspaceMessage::RemoveProjectCancelled),
            crate::view::ids::remove_project::keep(),
        ),
        Space::new().width(iced::Length::Fill),
        danger_control(
            tokens,
            state.t("plain.remove.confirm"),
            Message::Workspace(WorkspaceMessage::RemoveProjectConfirmed(
                dialog.project_id.clone(),
            )),
            crate::view::ids::remove_project::remove(),
        ),
    ]
    .align_y(Alignment::Center);

    Some(knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Small.resolve(state.window_width),
            state.t("plain.remove.title"),
            Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
            crate::view::ids::remove_project::close(),
            body,
            footer,
        ),
        &[],
    ))
}
