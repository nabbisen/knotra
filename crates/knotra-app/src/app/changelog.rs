//! The changelog domain: `handle_changelog` (RFC-040 Stage 3 commit 1).

use iced::{Task, clipboard};
use knotra_vcs::VcsAdapter;

use crate::{
    message::{BackgroundMessage, ChangelogMessage, Message},
    state::{AppState, changelog::ChangelogPhase},
};

pub(super) fn handle_changelog(state: &mut AppState, msg: ChangelogMessage) -> Task<Message> {
    match msg {
        ChangelogMessage::BulkOpenRequested => {
            let selected = state.selection_summary().selected_ids;
            if selected.is_empty() {
                return Task::none();
            }
            state.changelog.invalidate_collection();
            state.changelog.project_selection = selected.into_iter().map(|id| (id, true)).collect();
            state.changelog.phase = ChangelogPhase::Idle;
            state.active_modal = crate::state::ActiveModal::Changelog;
            Task::none()
        }

        ChangelogMessage::SinceRefChanged(s) => {
            state.changelog.since_ref = s;
            if matches!(
                state.changelog.phase,
                ChangelogPhase::Ready(_) | ChangelogPhase::Collecting
            ) {
                state.changelog.phase = ChangelogPhase::Idle;
            }
            state.changelog.invalidate_collection();
            Task::none()
        }

        ChangelogMessage::ProjectToggled(id, v) => {
            state.changelog.project_selection.insert(id, v);
            if matches!(state.changelog.phase, ChangelogPhase::Ready(_)) {
                state.changelog.phase = ChangelogPhase::Idle;
            }
            state.changelog.invalidate_collection();
            Task::none()
        }

        ChangelogMessage::GenerateRequested => {
            if !state.changelog.is_ready_to_collect() {
                return Task::none();
            }
            let selected_ids = state.changelog.selected_ids();
            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .filter(|p| selected_ids.contains(&p.id))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if projects.is_empty() {
                return Task::none();
            }
            let since = state.changelog.since_ref.clone();
            let max_cl = state.config.max_concurrent_reads;
            let request_id = state.changelog.begin_collection();

            Task::perform(
                async move { VcsAdapter::collect_changelog(&projects, &since, max_cl).await },
                move |draft| {
                    Message::Background(BackgroundMessage::ChangelogDraftReady {
                        request_id,
                        draft,
                    })
                },
            )
        }

        ChangelogMessage::CopyRequested => {
            if let ChangelogPhase::Ready(ref draft) = state.changelog.phase {
                let md = draft.to_markdown();
                state.status_bar = Some(format!(
                    "{} {} {}",
                    state.t("plain.changelog.copied_prefix"),
                    md.len(),
                    state.t("plain.changelog.copied_suffix")
                ));
                return clipboard::write(md);
            }
            Task::none()
        }

        ChangelogMessage::CollectRequested => {
            Task::done(Message::Changelog(ChangelogMessage::GenerateRequested))
        }
        ChangelogMessage::ModalClosed => {
            state.changelog.invalidate_collection();
            state.active_modal = crate::state::ActiveModal::None;
            Task::none()
        }
    }
}
