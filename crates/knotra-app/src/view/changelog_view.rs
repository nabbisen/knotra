//! Changelog generation view.

use endringer::ChangelogDraft;
use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length, Padding,
};

use crate::{
    message::{ChangelogMessage, Message},
    state::{changelog::ChangelogPhase, AppState},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = row![
        button(text("← Dashboard"))
            .on_press(Message::Changelog(ChangelogMessage::BackToDashboard)),
        text(state.t("changelog.title")).size(20),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0));

    let body: Element<'_, Message> = match &state.changelog.phase {
        ChangelogPhase::Idle       => view_form(state),
        ChangelogPhase::Collecting => spinner(state.t("changelog.generating")),
        ChangelogPhase::Ready(d)   => view_draft(state, d.clone()),
    };

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

fn view_form(state: &AppState) -> Element<'_, Message> {
    let projects = state.workspace.as_ref()
        .map(|w| w.projects.as_slice())
        .unwrap_or(&[]);

    let tag_rows: Vec<Element<'_, Message>> = if state.changelog.available_tags.is_empty() {
        vec![
            button(text(state.t("changelog.load_tags")).size(12))
                .on_press(Message::Changelog(ChangelogMessage::LoadTagsRequested))
                .into()
        ]
    } else {
        state.changelog.available_tags.iter().map(|tag| {
            let t = tag.clone();
            button(text(tag.as_str()).size(11))
                .on_press(Message::Changelog(ChangelogMessage::SinceRefChanged(t)))
                .into()
        }).collect()
    };

    let project_rows: Vec<Element<'_, Message>> = projects.iter().map(|p| {
        let included = *state.changelog.project_selection.get(&p.id).unwrap_or(&true);
        let id = p.id.clone();
        row![
            checkbox(included)
                .label(p.name.as_str())
                .on_toggle(move |v| Message::Changelog(ChangelogMessage::ProjectToggled(id.clone(), v)))
        ]
        .padding([2, 0])
        .into()
    }).collect();

    let generate_btn = button(text(state.t("changelog.generate")))
        .on_press_maybe(
            if state.changelog.is_ready_to_collect() {
                Some(Message::Changelog(ChangelogMessage::GenerateRequested))
            } else { None }
        );

    column![
        text(state.t("changelog.since_label")).size(13),
        text_input(state.t("changelog.since_hint"), &state.changelog.since_ref)
            .on_input(|s| Message::Changelog(ChangelogMessage::SinceRefChanged(s)))
            .width(250),
        text("Recent tags:").size(11),
        row(tag_rows).spacing(4),
        text(state.t("changelog.projects_label")).size(13),
        column(project_rows).spacing(2),
        generate_btn,
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn view_draft(state: &AppState, draft: ChangelogDraft) -> Element<'_, Message> {
    let md = draft.to_markdown();
    let total = draft.total_commits();

    let summary = format!("{} {}",
        state.t("changelog.total"),
        total
    );

    let preview_lines: Vec<Element<'_, Message>> = md
        .lines()
        .take(50)
        .map(|l| text(l.to_owned()).size(11).into())
        .collect();

    column![
        row![
            text(summary).size(13),
            Space::new().width(Length::Fill),
            button(text(state.t("changelog.copy")))
                .on_press(Message::Changelog(ChangelogMessage::CopyRequested)),
            button(text("← Back / Regenerate"))
                .on_press(Message::Changelog(ChangelogMessage::BackToDashboard)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        scrollable(column(preview_lines).spacing(0)).height(500),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn spinner(msg: &str) -> Element<'_, Message> {
    container(text(msg.to_owned()).size(14))
        .width(Length::Fill)
        .height(250)
        .center_x(Length::Fill)
        .center_y(250)
        .into()
}
