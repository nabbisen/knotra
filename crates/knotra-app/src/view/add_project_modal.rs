//! RFC-0021 Phase 5 — Guided 2-step "Add project folder" dialog.
//!
//! Step 1: Choose the folder that contains your project.
//! Step 2: Give it a display name.
//!
//! The Browse button auto-advances to Step 2 when a folder is selected.
//! A typed path also advances on "Next" after validation.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, Tokens, centered_button_label, guided_field_focused, reasoned, style,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{
        AddProjectStep, AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "add_project.close";
    pub const BROWSE: &str = "add_project.browse";
    pub const NEXT: &str = "add_project.next";
    pub const CANCEL: &str = "add_project.cancel";
    pub const ADD: &str = "add_project.add";
    pub const BACK: &str = "add_project.back";
}

/// RFC-066 D2: order and entry for Add project. `None` while no dialog is
/// open, the general "no controls" rule.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    let Some(dialog) = state.add_project_dialog.as_ref() else {
        return LayerFocus {
            order: Vec::new(),
            entry: None,
        };
    };
    let close_target = FocusTarget::control(focus_target::CLOSE);
    let close_msg = Some(Message::Workspace(WorkspaceMessage::AddProjectCancelled));

    match dialog.step {
        AddProjectStep::ChooseFolder => {
            let path_field =
                FocusTarget::text_input(knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone());
            let has_path = !dialog.path.trim().is_empty();
            LayerFocus {
                order: vec![
                    (close_target, close_msg),
                    (path_field.clone(), None),
                    (
                        FocusTarget::control(focus_target::BROWSE),
                        Some(Message::Workspace(WorkspaceMessage::BrowsePathRequested)),
                    ),
                    (
                        FocusTarget::control(focus_target::NEXT),
                        has_path
                            .then_some(Message::Workspace(WorkspaceMessage::AddProjectNextStep)),
                    ),
                    (
                        FocusTarget::control(focus_target::CANCEL),
                        Some(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
                    ),
                ],
                entry: Some(path_field),
            }
        }
        AddProjectStep::NameProject => {
            let name_field =
                FocusTarget::text_input(knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone());
            let has_name = !dialog.name.trim().is_empty();
            LayerFocus {
                order: vec![
                    (close_target, close_msg),
                    (name_field.clone(), None),
                    (
                        FocusTarget::control(focus_target::ADD),
                        has_name
                            .then_some(Message::Workspace(WorkspaceMessage::AddProjectConfirmed)),
                    ),
                    (
                        FocusTarget::control(focus_target::BACK),
                        Some(Message::Workspace(WorkspaceMessage::AddProjectBack)),
                    ),
                ],
                entry: Some(name_field),
            }
        }
    }
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring (RFC-066 D4) — the same shape every other
/// overlay's local `styled_button` uses.
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

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let dialog = state.add_project_dialog.as_ref()?;
    let tokens = &state.theme.tokens;

    let (step_label, _total) = match dialog.step {
        AddProjectStep::ChooseFolder => (state.t("plain.add_project.step1_of2"), "1"),
        AddProjectStep::NameProject => (state.t("plain.add_project.step2_of2"), "2"),
    };

    let close_t = tokens.clone();
    let close_is_focused = crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE));
    let close_btn = button(centered_button_label(
        text("✕")
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
    .on_press(Message::Workspace(WorkspaceMessage::AddProjectCancelled))
    .style(move |_theme, status| {
        style::with_focus_ring(&close_t, close_is_focused, style::ghost(&close_t, status))
    });

    let header = row![
        column![
            text(state.t("plain.add_project.title"))
                .size(snora::design::style::text::title_size(tokens)),
            text(step_label)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        ]
        .spacing(2),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = match dialog.step {
        // ------------------------------------------------------------------
        // Step 1 — Choose the folder
        // ------------------------------------------------------------------
        AddProjectStep::ChooseFolder => {
            let path_field = guided_field_focused(
                tokens,
                state.t("plain.add_project.folder_label"),
                state.t("plain.add_project.folder_hint"),
                &dialog.path,
                |s| Message::Workspace(WorkspaceMessage::AddProjectPathChanged(s)),
                dialog.error.as_deref(),
                knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone(),
            );

            let browse_btn = styled_button(
                tokens,
                state.t("plain.add_project.browse"),
                Some(Message::Workspace(WorkspaceMessage::BrowsePathRequested)),
                crate::app::ring(state, &FocusTarget::control(focus_target::BROWSE)),
                style::ghost,
            );

            let next_reason: Option<&str> = if dialog.path.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_folder"))
            } else {
                None
            };

            let footer = row![
                reasoned(
                    tokens,
                    state.t("plain.add_project.next"),
                    (!dialog.path.trim().is_empty())
                        .then_some(Message::Workspace(WorkspaceMessage::AddProjectNextStep)),
                    next_reason,
                    crate::app::ring(state, &FocusTarget::control(focus_target::NEXT)),
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("action.cancel"),
                    Some(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::CANCEL)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.add_project.step1_instruction"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                path_field,
                browse_btn,
                footer,
            ]
            .spacing(14)
            .into()
        }

        // ------------------------------------------------------------------
        // Step 2 — Name the project
        // ------------------------------------------------------------------
        AddProjectStep::NameProject => {
            // Show the chosen folder as a read-only confirmation.
            let folder_display = container(
                column![
                    text(state.t("plain.add_project.folder_chosen"))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                    text(&dialog.path)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                ]
                .spacing(4),
            )
            .padding([10, 14]);

            let name_field = guided_field_focused(
                tokens,
                state.t("plain.add_project.name_label"),
                state.t("dialog.add_project.name_hint"),
                &dialog.name,
                |s| Message::Workspace(WorkspaceMessage::AddProjectNameChanged(s)),
                dialog.error.as_deref(),
                knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
            );

            let add_reason: Option<&str> = if dialog.name.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_name"))
            } else {
                None
            };

            let footer = row![
                reasoned(
                    tokens,
                    state.t("plain.add_project.add"),
                    (!dialog.name.trim().is_empty())
                        .then_some(Message::Workspace(WorkspaceMessage::AddProjectConfirmed)),
                    add_reason,
                    crate::app::ring(state, &FocusTarget::control(focus_target::ADD)),
                    style::primary,
                ),
                Space::new().width(Length::Fill),
                styled_button(
                    tokens,
                    state.t("plain.add_project.back"),
                    Some(Message::Workspace(WorkspaceMessage::AddProjectBack)),
                    crate::app::ring(state, &FocusTarget::control(focus_target::BACK)),
                    style::ghost,
                ),
            ]
            .align_y(Alignment::Center);

            column![
                text(state.t("plain.add_project.step2_instruction"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                folder_display,
                name_field,
                footer,
            ]
            .spacing(14)
            .into()
        }
    };

    Some(
        container(column![header, body].spacing(16).padding(24))
            .width(Length::Fixed(480.0))
            .into(),
    )
}
