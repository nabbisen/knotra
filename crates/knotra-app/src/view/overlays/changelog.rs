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
    BUTTON_HEIGHT, Tokens, centered_button_label, guided_field_focused,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};

use crate::{
    message::{ChangelogMessage, Message},
    state::{
        AppState,
        focus::{FocusTarget, LayerFocus},
    },
};

mod focus_target {
    pub const CLOSE: &str = "changelog.close";
    pub const GENERATE: &str = "changelog.generate";
    pub const COPY: &str = "changelog.copy";
    pub const FOOTER_CLOSE: &str = "changelog.footer_close";
}

/// RFC-066 D2: order and entry for Generate notes. `close_msg` is sent
/// unconditionally in every phase (R2/§2 in `changelog_modal`'s own
/// comment — closing during `Collecting` is explicitly allowed), so both
/// the header close and the footer-less Done phase's "Close" share one
/// message throughout, unlike `conflict.rs`'s phase-dependent one.
pub fn layer_focus(state: &AppState) -> LayerFocus<Message> {
    use crate::state::changelog::ChangelogPhase;

    let cl = &state.changelog;
    let close_msg = Some(Message::Changelog(ChangelogMessage::ModalClosed));
    let since_field = FocusTarget::text_input(knotra_ui::widget::focus_id::CHANGELOG_SINCE.clone());

    let mut order = vec![
        (FocusTarget::control(focus_target::CLOSE), close_msg.clone()),
        (since_field.clone(), None),
    ];

    if let Some(workspace) = &state.workspace {
        for project in &workspace.projects {
            let included = cl
                .project_selection
                .get(&project.id)
                .copied()
                .unwrap_or(false);
            let is_collecting = matches!(cl.phase, ChangelogPhase::Collecting);
            let msg = (!is_collecting).then_some(Message::Changelog(
                ChangelogMessage::ProjectToggled(project.id.clone(), !included),
            ));
            order.push((
                FocusTarget::control_dynamic(format!("changelog.project.{}", project.id)),
                msg,
            ));
        }
    }

    // D2's rule: the first text field is the entry while the user is
    // choosing. Applies to `Idle` (the table's own row) and `Collecting`
    // (not in the table — no phase-specific entry is named there, and
    // nothing about this phase's own field disables it, so the general
    // rule is what applies).
    let mut entry = Some(since_field);

    match &cl.phase {
        ChangelogPhase::Idle => {
            let reason_blocks = cl.since_ref.trim().is_empty() || cl.selected_ids().is_empty();
            order.push((
                FocusTarget::control(focus_target::GENERATE),
                (!reason_blocks).then_some(Message::Changelog(ChangelogMessage::CollectRequested)),
            ));
        }
        ChangelogPhase::Collecting => {}
        ChangelogPhase::Ready(_) => {
            let copy = FocusTarget::control(focus_target::COPY);
            order.push((
                copy.clone(),
                Some(Message::Changelog(ChangelogMessage::CopyRequested)),
            ));
            order.push((FocusTarget::control(focus_target::FOOTER_CLOSE), close_msg));
            // D2's named exception: a result phase's entry is normally
            // "Close", but this window's result *is* the notes, so "Copy
            // to clipboard".
            entry = Some(copy);
        }
    }

    LayerFocus { order, entry }
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
        knotra_ui::widget::focus_id::CHANGELOG_SINCE.clone(),
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
            reasoned(
                tokens,
                state.t("plain.changelog.generate"),
                cl.is_ready_to_collect()
                    .then_some(Message::Changelog(ChangelogMessage::CollectRequested)),
                reason,
                crate::app::ring(state, &FocusTarget::control(focus_target::GENERATE)),
                style::primary,
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
                text(preview_text)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
            ]
            .spacing(8);

            if draft.projects.is_empty() {
                result_col = result_col.push(text(state.t("plain.changelog.no_projects")));
            }

            column![
                result_col,
                row![
                    styled_button(
                        tokens,
                        state.t("plain.changelog.copy"),
                        Some(Message::Changelog(ChangelogMessage::CopyRequested)),
                        crate::app::ring(state, &FocusTarget::control(focus_target::COPY)),
                        style::primary,
                    ),
                    Space::new().width(Length::Fill),
                    styled_button(
                        tokens,
                        state.t("action.close"),
                        Some(Message::Changelog(ChangelogMessage::ModalClosed)),
                        crate::app::ring(state, &FocusTarget::control(focus_target::FOOTER_CLOSE)),
                        style::ghost,
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
    surface(
        tokens,
        OverlayWidth::Large.resolve(state.window_width),
        state.t("plain.changelog.title"),
        close_msg,
        crate::app::ring(state, &FocusTarget::control(focus_target::CLOSE)),
        body,
        Space::new(),
    )
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `conflict.rs` (RFC-037
/// Stage 2) and `workspace_manager.rs` (RFC-034 R9) use.
fn styled_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    is_focused: bool,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    iced::widget::button(centered_button_label(
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

fn changelog_project_picker<'a>(
    tokens: &Tokens,
    state: &'a AppState,
    disabled: bool,
) -> Element<'a, Message> {
    let Some(workspace) = &state.workspace else {
        return text(state.t("plain.changelog.no_projects"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into();
    };

    if workspace.projects.is_empty() {
        return text(state.t("plain.changelog.no_projects"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into();
    }

    let mut rows = column![
        text(state.t("plain.changelog.projects_label"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
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
        let t = tokens.clone();
        let focused = crate::app::ring(
            state,
            &FocusTarget::control_dynamic(format!("changelog.project.{}", project.id)),
        );
        rows = rows.push(
            iced::widget::button(centered_button_label(
                text(label)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
            ))
            .height(BUTTON_HEIGHT)
            .padding([0, 12])
            .on_press_maybe(msg)
            .style(move |_theme, status| {
                style::with_focus_ring(&t, focused, style::ghost(&t, status))
            }),
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

    text(notice)
        .size(snora::design::style::text::body_small_size(tokens))
        .line_height(snora::design::style::text::body_small_line_height(tokens))
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
        rows = rows.push(
            text(format!(
                "{} {}",
                state.t("plain.changelog.no_change_projects"),
                names
            ))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }

    rows.into()
}
