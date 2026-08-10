//! 5. Generate notes modal (Changelog) — RFC-037 Stage 1.
//!
//! `ChangelogResultCounts`, `changelog_result_counts`, and
//! `changelog_markdown_preview` are not in Handoff 041 §1's own function
//! table, which lists only `changelog_modal`, `changelog_project_picker`,
//! `changelog_summary_text`, `changelog_result_notice`, and
//! `changelog_project_results` for this file. They are moved here anyway —
//! flagged in the Stage 1 review request, not silently added — because they
//! belong to no other domain, `changelog_modal`'s own body calls both
//! functions directly, and leaving them in `mod.rs` would fail the
//! handoff's own criterion for what stays there ("used by more than one
//! overlay" — `modal_shell`/`project_name_for`'s test), since both are used
//! only within this one modal's flow.
//!
//! The two functions are `pub(crate)`, unchanged from `bulk_modals.rs`:
//! `tests.rs` calls both via `crate::view::bulk_modals::...` and R8 forbids
//! editing `tests.rs` this stage, so `overlays/mod.rs` re-exports both at
//! its own top level to keep that path resolving through the `bulk_modals`
//! alias (see `view.rs`).

use iced::{
    Element, Length,
    widget::{Space, button, column, row, scrollable, text},
};

use iced::Alignment;
use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, guided_button, guided_field};

use super::modal_shell;
use crate::{
    message::{ChangelogMessage, Message},
    state::AppState,
};

pub fn changelog_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::changelog::ChangelogPhase;

    let cl = &state.changelog;
    let is_collecting = matches!(cl.phase, ChangelogPhase::Collecting);

    let since_field = guided_field(
        state.t("plain.changelog.since_label"),
        state.t("plain.changelog.since_hint"),
        &cl.since_ref,
        |s| Message::Changelog(ChangelogMessage::SinceRefChanged(s)),
        None,
    );

    let project_picker = changelog_project_picker(state, is_collecting);

    let content: Element<'_, Message> = match &cl.phase {
        ChangelogPhase::Idle => {
            let reason = if cl.since_ref.trim().is_empty() {
                Some(state.t("plain.changelog.reason_empty"))
            } else if cl.selected_ids().is_empty() {
                Some(state.t("plain.disabled.choose_one"))
            } else {
                None
            };
            guided_button(
                state.t("plain.changelog.generate"),
                cl.is_ready_to_collect()
                    .then_some(Message::Changelog(ChangelogMessage::CollectRequested)),
                reason,
            )
        }

        ChangelogPhase::Collecting => text(state.t("plain.changelog.collecting"))
            .size(FONT_BODY)
            .into(),

        ChangelogPhase::Ready(draft) => {
            let counts = changelog_result_counts(draft);
            let preview_text = changelog_markdown_preview(draft);
            let mut result_col = column![
                text(changelog_summary_text(state, counts)).size(FONT_BODY),
                changelog_result_notice(state, draft, counts),
                changelog_project_results(state, draft, counts),
                scrollable(text(preview_text).size(FONT_SMALL)).height(Length::Fixed(240.0)),
            ]
            .spacing(8);

            if draft.projects.is_empty() {
                result_col = result_col.push(text(state.t("plain.changelog.no_projects")));
            }

            column![
                result_col,
                row![
                    button(text(state.t("plain.changelog.copy")).size(FONT_BODY))
                        .height(BUTTON_HEIGHT)
                        .padding([0, 18])
                        .on_press(Message::Changelog(ChangelogMessage::CopyRequested)),
                    Space::new().width(Length::Fill),
                    button(text(state.t("action.close")).size(FONT_BODY))
                        .height(BUTTON_HEIGHT)
                        .padding([0, 18])
                        .on_press(Message::Changelog(ChangelogMessage::ModalClosed)),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            ]
            .spacing(10)
            .into()
        }
    };

    let inner = column![since_field, project_picker, content].spacing(14);

    modal_shell(
        state.t("plain.changelog.title"),
        Some(Message::Changelog(ChangelogMessage::ModalClosed)),
        inner.into(),
    )
}

fn changelog_project_picker(state: &AppState, disabled: bool) -> Element<'_, Message> {
    let Some(workspace) = &state.workspace else {
        return text(state.t("plain.changelog.no_projects"))
            .size(FONT_SMALL)
            .into();
    };

    if workspace.projects.is_empty() {
        return text(state.t("plain.changelog.no_projects"))
            .size(FONT_SMALL)
            .into();
    }

    let mut rows =
        column![text(state.t("plain.changelog.projects_label")).size(FONT_SMALL)].spacing(6);
    for project in &workspace.projects {
        let included = state
            .changelog
            .project_selection
            .get(&project.id)
            .copied()
            .unwrap_or(false);
        let marker = if included { "☑" } else { "☐" };
        let label = format!("{marker} {}", project.name);
        let msg = (!disabled).then_some(Message::Changelog(ChangelogMessage::ProjectToggled(
            project.id.clone(),
            !included,
        )));
        rows = rows.push(
            button(text(label).size(FONT_SMALL))
                .height(BUTTON_HEIGHT)
                .padding([0, 12])
                .on_press_maybe(msg),
        );
    }

    rows.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChangelogResultCounts {
    pub total_commits: usize,
    pub projects_with_commits: usize,
    pub projects_without_changes: usize,
    pub projects_with_errors: usize,
}

pub(crate) fn changelog_result_counts(draft: &knotra_vcs::ChangelogDraft) -> ChangelogResultCounts {
    ChangelogResultCounts {
        total_commits: draft.total_commits(),
        projects_with_commits: draft
            .projects
            .iter()
            .filter(|project| !project.entries.is_empty() && project.error.is_none())
            .count(),
        projects_without_changes: draft
            .projects
            .iter()
            .filter(|project| project.entries.is_empty() && project.error.is_none())
            .count(),
        projects_with_errors: draft
            .projects
            .iter()
            .filter(|project| project.error.is_some())
            .count(),
    }
}

pub(crate) fn changelog_markdown_preview(draft: &knotra_vcs::ChangelogDraft) -> String {
    draft.to_markdown()
}

fn changelog_summary_text(state: &AppState, counts: ChangelogResultCounts) -> String {
    format!(
        "{} {} · {} {} · {} {} · {} {}",
        counts.total_commits,
        state.t("plain.changelog.summary_commits"),
        counts.projects_with_commits,
        state.t("plain.changelog.summary_with_notes"),
        counts.projects_without_changes,
        state.t("plain.changelog.summary_no_changes"),
        counts.projects_with_errors,
        state.t("plain.changelog.summary_failed")
    )
}

fn changelog_result_notice<'a>(
    state: &'a AppState,
    draft: &knotra_vcs::ChangelogDraft,
    counts: ChangelogResultCounts,
) -> Element<'a, Message> {
    let all_failed =
        !draft.projects.is_empty() && counts.projects_with_errors == draft.projects.len();
    let notice = if all_failed {
        state.t("plain.changelog.all_failed")
    } else if counts.projects_with_errors > 0 {
        state.t("plain.changelog.some_failed")
    } else if counts.total_commits == 0 {
        state.t("plain.changelog.no_changes_found")
    } else {
        state.t("plain.changelog.ready")
    };

    text(notice).size(FONT_SMALL).into()
}

fn changelog_project_results<'a>(
    state: &'a AppState,
    draft: &knotra_vcs::ChangelogDraft,
    counts: ChangelogResultCounts,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(4);

    if counts.projects_without_changes > 0 {
        let names = draft
            .projects
            .iter()
            .filter(|project| project.entries.is_empty() && project.error.is_none())
            .map(|project| project.project_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        rows = rows.push(
            text(format!(
                "{} {}",
                state.t("plain.changelog.no_change_projects"),
                names
            ))
            .size(FONT_SMALL),
        );
    }

    for project in draft
        .projects
        .iter()
        .filter(|project| project.error.is_some())
    {
        let error = project.error.as_deref().unwrap_or_default();
        rows = rows.push(
            text(format!(
                "{}: {}",
                project.project_name,
                if error.is_empty() {
                    state.t("plain.changelog.project_failed")
                } else {
                    error
                }
            ))
            .size(FONT_SMALL),
        );
    }

    rows.into()
}
