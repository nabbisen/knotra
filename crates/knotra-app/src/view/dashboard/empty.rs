//! Dashboard empty/error/confirmation states: no workspace, no projects, a
//! load error, no filter matches, and the remove-project confirmation.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{BUTTON_HEIGHT, Tokens, centered_button_label, style};

use crate::{
    message::{FilterMessage, Message, WorkspaceMessage},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const KEEP: &str = "remove_project.keep";
    pub const REMOVE: &str = "remove_project.remove";
}

/// RFC-066 D2: order and entry for Remove project — `["Keep", "Remove from
/// list"]`, entry "Keep" (`RemoveProjectConfirmed` starts a change). Still
/// rendered inline (Stage 3's D6 makes it a real dialog over any screen);
/// `None` while no confirmation is pending, the general "no controls" rule.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    let Some(dialog) = &state.confirm_remove_dialog else {
        return LayerFocus {
            order: Vec::new(),
            entry: None,
        };
    };
    let keep_target = FocusTarget::control(focus_target::KEEP);
    LayerFocus {
        order: vec![
            (
                keep_target.clone(),
                Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
            ),
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
/// overlay's local `styled_button` uses. Replaces `ghost_maybe`/
/// `danger_maybe`, neither of which can draw the outline.
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

pub(super) fn view_without_workspace(state: &AppState) -> Element<'_, Message> {
    placeholder(&state.theme.tokens, state.t("plain.status.checking"))
}

pub(super) fn empty_workspace(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.empty.welcome_title"))
                .size(snora::design::style::text::heading_size(tokens)),
            text(state.t("plain.empty.welcome_body"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            button(centered_button_label(text(
                state.t("plain.empty.add_first")
            )))
            .height(BUTTON_HEIGHT)
            .on_press(Message::Workspace(WorkspaceMessage::AddProjectDialogOpened,)),
        ]
        .spacing(14)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(260.0))
    .center(Length::Fill)
    .into()
}

pub(super) fn no_matches(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.empty.no_match"))
                .size(snora::design::style::text::title_size(tokens)),
            button(text(state.t("dashboard.clear_filters")))
                .on_press(Message::Filter(FilterMessage::AllFiltersCleared)),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(220.0))
    .center(Length::Fill)
    .into()
}

pub(super) fn view_confirm_remove_dialog(state: &AppState) -> Element<'_, Message> {
    let Some(dialog) = &state.confirm_remove_dialog else {
        return Space::new().into();
    };
    let tokens = &state.theme.tokens;
    container(
        column![
            text(state.t("plain.remove.title"))
                .size(snora::design::style::text::title_size(tokens)),
            text(dialog.project_name.as_str())
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            text(state.t("plain.remove.body"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
            row![
                // RFC-066 D4: moved off `ghost_maybe`/`danger_maybe` onto
                // `styled_button` — neither `*_maybe` helper can draw the
                // focus outline. `danger` for the confirm button is still
                // the semantically-correct choice for this irreversible
                // action (removes a project) — see `buttons.rs`'s
                // `danger`/`danger_maybe` doc comment.
                styled_button(
                    tokens,
                    state.t("confirm.remove_no"),
                    Some(Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::KEEP)),
                    style::ghost,
                ),
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
            .spacing(12),
        ]
        .spacing(14)
        .padding(24),
    )
    .width(Length::Fixed(380.0))
    .into()
}

fn placeholder<'a>(tokens: &Tokens, message: &'a str) -> Element<'a, Message> {
    container(text(message).size(snora::design::style::text::label_size(tokens)))
        .width(Length::Fill)
        .height(Length::Fixed(250.0))
        .center(Length::Fill)
        .into()
}
