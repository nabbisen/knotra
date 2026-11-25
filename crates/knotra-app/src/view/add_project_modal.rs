//! RFC-0021 Phase 5 — Guided 2-step "Add project folder" dialog.
//!
//! Step 1: Choose the folder that contains your project.
//! Step 2: Give it a display name.
//!
//! The Browse button auto-advances to Step 2 when a folder is selected.
//! A typed path also advances on "Next" after validation.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, container, row, text},
};

use knotra_ui::widget::{
    ghost_control, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned_control_maybe, style,
};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{AddProjectStep, AppState},
};

/// RFC-075 stage 3 §3: Add project's own entry per step — the folder
/// field for step 1, the name field for step 2. `None` while no dialog is
/// open.
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    let dialog = state.add_project_dialog.as_ref()?;
    Some(match dialog.step {
        AddProjectStep::ChooseFolder => crate::view::ids::add_project::path(),
        AddProjectStep::NameProject => crate::view::ids::add_project::name(),
    })
}

/// Task 098 Amendment A2: content now goes through `surface()`, so this
/// renders into `AppLayout::dialog` (`view.rs`) instead of its own centred
/// `iced::widget::stack` layer — the opaque surface, the full-window dim,
/// and the scrim's click-to-close all come from the engine now. The
/// header's own title and "✕" become `surface()`'s — its text is
/// unchanged (`plain.add_project.title`). `surface()`'s header has no
/// room for a second line, so the step label ("Step 1 of 2" /
/// "Step 2 of 2") moves into the body, as its own small line directly
/// above the step's instruction text — the same text, just relocated,
/// not reworded.
pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let dialog = state.add_project_dialog.as_ref()?;
    let tokens = &state.theme.tokens;

    let step_label = match dialog.step {
        AddProjectStep::ChooseFolder => state.t("plain.add_project.step1_of2"),
        AddProjectStep::NameProject => state.t("plain.add_project.step2_of2"),
    };
    // RFC-071 Amendment A3 D14: kept `body_small` — a wizard-progress
    // indicator ("Step 1 of 2"), ambient orientation rather than content
    // read to act.
    let step_label_line = text(step_label)
        .size(snora::design::style::text::body_small_size(tokens))
        .line_height(snora::design::style::text::body_small_line_height(tokens));

    let (body, footer): (Element<'_, Message>, Element<'_, Message>) = match dialog.step {
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
                crate::view::ids::add_project::path(),
            );

            let browse_btn = ghost_control(
                tokens,
                state.t("plain.add_project.browse"),
                Message::Workspace(WorkspaceMessage::BrowsePathRequested),
                crate::view::ids::add_project::browse(),
            );

            let next_reason: Option<&str> = if dialog.path.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_folder"))
            } else {
                None
            };
            let next = reasoned_control_maybe(
                tokens,
                state.t("plain.add_project.next"),
                (!dialog.path.trim().is_empty())
                    .then_some(Message::Workspace(WorkspaceMessage::AddProjectNextStep)),
                next_reason,
                style::primary,
                crate::view::ids::add_project::next(),
            );

            let footer = row![
                next,
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.cancel"),
                    Message::Workspace(WorkspaceMessage::AddProjectCancelled),
                    crate::view::ids::add_project::cancel(),
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    step_label_line,
                    text(state.t("plain.add_project.step1_instruction"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    path_field,
                    browse_btn,
                ]
                .spacing(14)
                .into(),
                footer.into(),
            )
        }

        // ------------------------------------------------------------------
        // Step 2 — Name the project
        // ------------------------------------------------------------------
        AddProjectStep::NameProject => {
            // Show the chosen folder as a read-only confirmation.
            let folder_display = container(
                column![
                    // RFC-071 Amendment A3 D14: a field label — `body`.
                    text(state.t("plain.add_project.folder_chosen"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
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
                crate::view::ids::add_project::name(),
            );

            let add_reason: Option<&str> = if dialog.name.trim().is_empty() {
                Some(state.t("plain.add_project.reason_no_name"))
            } else {
                None
            };
            let add = reasoned_control_maybe(
                tokens,
                state.t("plain.add_project.add"),
                (!dialog.name.trim().is_empty())
                    .then_some(Message::Workspace(WorkspaceMessage::AddProjectConfirmed)),
                add_reason,
                style::primary,
                crate::view::ids::add_project::add(),
            );

            let footer = row![
                add,
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("plain.add_project.back"),
                    Message::Workspace(WorkspaceMessage::AddProjectBack),
                    crate::view::ids::add_project::back(),
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    step_label_line,
                    text(state.t("plain.add_project.step2_instruction"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    folder_display,
                    name_field,
                ]
                .spacing(14)
                .into(),
                footer.into(),
            )
        }
    };

    Some(knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Standard.resolve(state.window_width),
            state.t("plain.add_project.title"),
            Some(Message::Workspace(WorkspaceMessage::AddProjectCancelled)),
            crate::view::ids::add_project::close(),
            body,
            footer,
        ),
        &[],
    ))
}
