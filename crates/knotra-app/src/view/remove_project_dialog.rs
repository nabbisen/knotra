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
    widget::{Space, button, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, Tokens, centered_button_label,
    overlay::{OverlayWidth, surface},
    style,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "remove_project.close";
    pub const KEEP: &str = "remove_project.keep";
    pub const REMOVE: &str = "remove_project.remove";
}

/// RFC-066 D2/D6: order and entry for Remove project — `[Close, "Keep",
/// "Remove from list"]`, entry "Keep" (`RemoveProjectConfirmed` starts a
/// change, so it is never the entry). `None` while no confirmation is
/// pending, the general "no controls" rule.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    let Some(dialog) = &state.confirm_remove_dialog else {
        return LayerFocus {
            order: Vec::new(),
            entry: None,
        };
    };
    let close_msg = Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled));
    let keep_target = FocusTarget::control(focus_target::KEEP);
    LayerFocus {
        order: vec![
            (FocusTarget::control(focus_target::CLOSE), close_msg.clone()),
            (keep_target.clone(), close_msg),
            (
                FocusTarget::control(focus_target::REMOVE),
                Some(Message::Workspace(
                    WorkspaceMessage::RemoveProjectConfirmed(dialog.project_id.clone()),
                )),
            ),
        ],
        entry: Some(keep_target),
    }
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring (RFC-066 D4) — the same shape every other
/// dialog's local `styled_button` uses.
fn styled_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    is_focused: bool,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 18])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style_fn(&t, status)))
    .into()
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

    // "Keep" is `style::ghost`; "Remove from list" is `style::danger`
    // (RFC D6) — an irreversible action, the same reasoning
    // `buttons.rs`'s own `danger`/`danger_maybe` doc comment gives.
    let footer = row![
        styled_button(
            tokens,
            state.t("confirm.remove_no"),
            Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
            crate::app::ring(state, &FocusTarget::control(focus_target::KEEP)),
            style::ghost,
        ),
        Space::new().width(iced::Length::Fill),
        styled_button(
            tokens,
            state.t("plain.remove.confirm"),
            Some(Message::Workspace(
                WorkspaceMessage::RemoveProjectConfirmed(dialog.project_id.clone(),)
            )),
            crate::app::ring(state, &FocusTarget::control(focus_target::REMOVE)),
            style::danger,
        ),
    ]
    .align_y(Alignment::Center);

    Some(surface(
        tokens,
        OverlayWidth::Small.resolve(state.window_width),
        state.t("plain.remove.title"),
        Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
        crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE)),
        body,
        footer,
    ))
}
