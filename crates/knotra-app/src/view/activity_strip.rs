#![allow(unused_imports, unused_variables, dead_code)]
//! RFC-0011 — Activity strip view.
//!
//! A single-line bar at the very bottom of the window.  Hidden when idle.
//! Shows progress during an operation and a summary when done.

use iced::{
    widget::{button, container, row, text, Space},
    Alignment, Element, Length,
};

use crate::{
    message::{ActivityMessage, Message},
    state::{ActivityStripState, AppState, LatestOpState},
};

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
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
                button(text("Retry").size(11))
                    .on_press(Message::Activity(ActivityMessage::RetryRequested)),
            );
            Some(strip_row(label, retry, strip))
        }
        LatestOpState::TotalFailure { summary } => {
            let label = format!("✗  {}", summary);
            let retry = Some(
                button(text("Retry").size(11))
                    .on_press(Message::Activity(ActivityMessage::RetryRequested)),
            );
            Some(strip_row(label, retry, strip))
        }
    }
}

fn strip_row_no_extra<'a>(label: String, state: &'a ActivityStripState) -> Element<'a, Message> {
    let details_btn = button(text("›").size(11))
        .on_press(Message::Activity(ActivityMessage::PopoverToggled));
    container(
        row![text(label).size(12), Space::new().width(Length::Fill), details_btn]
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
    let details_btn = button(text("›").size(11))
        .on_press(Message::Activity(ActivityMessage::PopoverToggled));

    let mut r = row![
        text(label).size(12),
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
