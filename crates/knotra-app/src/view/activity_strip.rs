#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0011 — Activity strip view.
//! RFC-0021 Phase 5 — Undo snackbar for project removal.
//!
//! A single-line bar at the very bottom of the window.  Hidden when idle.
//! Shows progress during an operation and a summary when done.
//! When a project was just removed, shows an "Undo" snackbar instead.

use iced::{
    widget::{button, container, row, text, Space},
    Alignment, Element, Length,
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL};

use crate::{
    message::{ActivityMessage, Message, WorkspaceMessage},
    state::{ActivityStripState, AppState, LatestOpState},
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    // Undo snackbar takes priority over the regular activity strip.
    if let Some(removal) = &state.recent_removal {
        let msg = format!(
            "{} {}.",
            state.t("plain.undo.removed_prefix"),
            removal.project.name,
        );
        let snackbar = container(
            row![
                text(msg).size(FONT_SMALL),
                Space::new().width(Length::Fill),
                button(text(state.t("plain.undo.undo")).size(FONT_SMALL))
                    .height(30.0)
                    .padding([0, 12])
                    .on_press(Message::Workspace(WorkspaceMessage::UndoRemoval)),
                button(text(state.t("plain.undo.dismiss")).size(FONT_SMALL))
                    .height(30.0)
                    .padding([0, 12])
                    .on_press(Message::Workspace(WorkspaceMessage::DismissUndoSnackbar)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([4, 12]),
        )
        .width(Length::Fill);
        return Some(snackbar.into());
    }

    let strip = &state.activity;
    match &strip.latest {
        LatestOpState::Idle => None,
        LatestOpState::Running { label, done, total } => {
            let progress_label = if *total > 0 {
                format!("⟳  {}  {}/{}", label, done, total)
            } else {
                format!("⟳  {}", label)
            };
            Some(strip_row_no_extra(progress_label, strip))
        }
        LatestOpState::Success { summary, .. } => {
            let label = format!("ⓘ  {}", summary);
            Some(strip_row_no_extra(label, strip))
        }
        LatestOpState::PartialFailure { summary, failed_names } => {
            let names = failed_names.join(", ");
            let label = format!("⚠  {}  (failed: {})", summary, names);
            let retry = Some(
                button(text("Retry").size(FONT_SMALL))
                    .on_press(Message::Activity(ActivityMessage::RetryRequested)),
            );
            Some(strip_row(label, retry, strip))
        }
        LatestOpState::TotalFailure { summary } => {
            let label = format!("✗  {}", summary);
            let retry = Some(
                button(text("Retry").size(FONT_SMALL))
                    .on_press(Message::Activity(ActivityMessage::RetryRequested)),
            );
            Some(strip_row(label, retry, strip))
        }
    }
}

fn strip_row_no_extra<'a>(label: String, state: &'a ActivityStripState) -> Element<'a, Message> {
    let details_btn = button(text("›").size(FONT_SMALL))
        .on_press(Message::Activity(ActivityMessage::PopoverToggled));
    container(
        row![text(label).size(FONT_SMALL), Space::new().width(Length::Fill), details_btn]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([4, 12])
    )
    .width(Length::Fill)
    .into()
}

fn strip_row<'a>(
    label:  String,
    extra:  Option<impl Into<Element<'a, Message>>>,
    state:  &'a ActivityStripState,
) -> Element<'a, Message> {
    let details_btn = button(text("›").size(FONT_SMALL))
        .on_press(Message::Activity(ActivityMessage::PopoverToggled));

    let mut r = row![
        text(label).size(FONT_SMALL),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some(e) = extra {
        r = r.push(e);
    }
    r = r.push(details_btn);

    container(r.padding([4, 12]))
        .width(Length::Fill)
        .into()
}
