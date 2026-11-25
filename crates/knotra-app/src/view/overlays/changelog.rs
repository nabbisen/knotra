//! 5. Generate notes modal (Changelog) — RFC-037 Stage 3.
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
//!
//! **RFC-037 Stage 3**: `modal_shell` is replaced with
//! `knotra_ui::widget::overlay::surface`, and the `guided_button` call site
//! is migrated onto a hand-built control styled through `buttons::style`.
//! Stage 3 had nothing to migrate the reason-carrying composition onto, so
//! it built a local `reasoned_button` helper for it.
//!
//! **RFC-037 Stage 4, commit 1**: Stage 3 also inlined the `guided_field`
//! call site into a local `since_field` helper, on the assumption that an
//! RFC-034 field replacement existed to migrate onto. It does not — D6/R11
//! (added after Stage 3's review, `133` §4) settled that `guided_field` is
//! the field vocabulary, not a legacy helper, since `field.rs` has nothing
//! else and `workspace_manager.rs` (RFC-034 R9's own validating migration)
//! still calls `guided_field_focused`. Reverted back to a direct
//! `guided_field` call here; the local inline composition it replaced is
//! gone, since a second copy of the same composition is exactly what R11
//! now forbids.
//!
//! **RFC-037 Stage 6**: `knotra-ui` grew `reasoned` (D7) — the shared form
//! Stage 3's local `reasoned_button` existed only because there was nothing
//! to call. The local helper is deleted; this file's one `guided_button`
//! call site now calls the shared primitive directly.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    Tokens, ghost_control, ghost_control_maybe, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned_control_maybe, style,
};

use crate::{
    message::{ChangelogMessage, Message},
    state::AppState,
};

/// RFC-075 stage 3 §3: Generate notes' own entry per phase. `Idle`/
/// `Collecting` both enter on the since-field (D2's general rule — neither
/// names its own, more specific entry). `Ready`'s result *is* the notes,
/// so its entry is "Copy to clipboard", not "Close" (D2's named
/// exception).
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    use crate::state::changelog::ChangelogPhase;

    Some(match &state.changelog.phase {
        ChangelogPhase::Idle | ChangelogPhase::Collecting => crate::view::ids::changelog::since(),
        ChangelogPhase::Ready(_) => crate::view::ids::changelog::copy(),
    })
}

pub fn changelog_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::changelog::ChangelogPhase;

    let tokens = &state.theme.tokens;
    let cl = &state.changelog;
    let is_collecting = matches!(cl.phase, ChangelogPhase::Collecting);

    let since_field = guided_field_focused(
        tokens,
        state.t("plain.changelog.since_label"),
        state.t("plain.changelog.since_hint"),
        &cl.since_ref,
        |s| Message::Changelog(ChangelogMessage::SinceRefChanged(s)),
        None,
        crate::view::ids::changelog::since(),
    );

    let project_picker = changelog_project_picker(tokens, state, is_collecting);

    let content: Element<'_, Message> = match &cl.phase {
        ChangelogPhase::Idle => {
            let reason = if cl.since_ref.trim().is_empty() {
                Some(state.t("plain.changelog.reason_empty"))
            } else if cl.selected_ids().is_empty() {
                Some(state.t("plain.disabled.choose_one"))
            } else {
                None
            };
            reasoned_control_maybe(
                tokens,
                state.t("plain.changelog.generate"),
                cl.is_ready_to_collect()
                    .then_some(Message::Changelog(ChangelogMessage::CollectRequested)),
                reason,
                style::primary,
                crate::view::ids::changelog::generate(),
            )
        }

        ChangelogPhase::Collecting => text(state.t("plain.changelog.collecting"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into(),

        ChangelogPhase::Ready(draft) => {
            let counts = changelog_result_counts(draft);
            // No inner `scrollable` around the preview text (unlike the
            // pre-migration version's `.height(Length::Fixed(240.0))` box) —
            // `surface()`'s own body scrollable now covers the whole body,
            // the same reasoning Stage 2 used to drop `conflict.rs`'s inner
            // scrollable (review `132` §4 confirmed that call).
            let preview_text = changelog_markdown_preview(draft);
            let mut result_col = column![
                text(changelog_summary_text(state, counts))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                changelog_result_notice(tokens, state, draft, counts),
                changelog_project_results(tokens, state, draft, counts),
                // RFC-071 Amendment A3 D14: the generated notes themselves
                // — dialog body content the user reviews before copying.
                text(preview_text)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8);

            if draft.projects.is_empty() {
                result_col = result_col.push(text(state.t("plain.changelog.no_projects")));
            }

            column![
                result_col,
                row![
                    knotra_ui::widget::primary_control(
                        tokens,
                        state.t("plain.changelog.copy"),
                        Message::Changelog(ChangelogMessage::CopyRequested),
                        crate::view::ids::changelog::copy(),
                    ),
                    Space::new().width(Length::Fill),
                    ghost_control(
                        tokens,
                        state.t("action.close"),
                        Message::Changelog(ChangelogMessage::ModalClosed),
                        crate::view::ids::changelog::done_close(),
                    ),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            ]
            .spacing(10)
            .into()
        }
    };

    let body = column![since_field, project_picker, content].spacing(14);

    // R2/§2: unconditional, exactly as `modal_shell` received it before this
    // migration — closing during `Collecting` is explicitly allowed
    // (`app/changelog.rs`'s `ModalClosed` handler invalidates the in-flight
    // request via RFC-030's request-id guard), so this must never be gated
    // by phase the way `conflict.rs`'s `close_msg` is.
    let close_msg = Some(Message::Changelog(ChangelogMessage::ModalClosed));

    // This file has no single pre-existing "footer row" the way
    // `conflict.rs` did (Copy/Close lived inside the Ready-phase `content`
    // branch, not a page-level footer) — so rather than relocate them into
    // `surface()`'s footer slot, an empty `Space` is passed here and Copy/
    // Close stay exactly where they were, preserving the original layout.
    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Large.resolve(state.window_width),
            state.t("plain.changelog.title"),
            close_msg,
            crate::view::ids::changelog::close(),
            body,
            Space::new(),
        ),
        &[],
    )
}

fn changelog_project_picker<'a>(
    tokens: &Tokens,
    state: &'a AppState,
    disabled: bool,
) -> Element<'a, Message> {
    // RFC-071 Amendment A3 D14: a notice/message and a list label — both
    // read to understand what the picker shows.
    let Some(workspace) = &state.workspace else {
        return text(state.t("plain.changelog.no_projects"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into();
    };

    if workspace.projects.is_empty() {
        return text(state.t("plain.changelog.no_projects"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into();
    }

    let mut rows = column![
        text(state.t("plain.changelog.projects_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
    ]
    .spacing(6);
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
        // RFC-071 Amendment A3 D14: a project-selection row label — list
        // content read to decide which projects to include.
        rows = rows.push(ghost_control_maybe(
            tokens,
            label,
            msg,
            crate::view::ids::changelog::project_toggle(&project.id),
        ));
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
    tokens: &Tokens,
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

    // RFC-071 Amendment A3 D14: a notice — explicit move category.
    text(notice)
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens))
        .into()
}

fn changelog_project_results<'a>(
    tokens: &Tokens,
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
        // RFC-071 Amendment A3 D14: result list content, read to see which
        // projects had nothing new.
        rows = rows.push(
            text(format!(
                "{} {}",
                state.t("plain.changelog.no_change_projects"),
                names
            ))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }

    for project in draft
        .projects
        .iter()
        .filter(|project| project.error.is_some())
    {
        let error = project.error.as_deref().unwrap_or_default();
        // RFC-071 Amendment A3 D14: an error/reason, read to understand why
        // a project failed.
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
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }

    rows.into()
}
