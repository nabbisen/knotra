//! 4. Conflict resolve panel (right-docked sheet) — RFC-037 Stage 2.
//!
//! Migrated off the hand-rolled `container`/`row`/`button(text("✕"))` chrome
//! onto RFC-034's overlay-host primitives (D1/D4). `modal_shell` was never
//! this file's shell — `resolve_panel` built its own — so nothing here
//! called it and nothing here needs to stop calling it.
//!
//! **`view.rs:170`'s `Sheet::new(el).at(SheetEdge::End).with_size(SheetSize::Half)`
//! is untouched** (R7/D4: conflict resolution stays a sheet, not a dialog) —
//! this file only changes what `resolve_panel` returns *into* that mount
//! point, not how it is mounted.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    BUTTON_HEIGHT, NoticeTone, Tokens, notice,
    overlay::{OverlayWidth, surface},
    reasoned, style,
};
use knotra_vcs::{ProjectId, VcsKind};

use crate::{
    message::{ConflictOpsMessage, Message},
    state::AppState,
};

pub fn resolve_panel<'a>(state: &'a AppState, project_id: &'a ProjectId) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let name = project_name_for(state, project_id);
    let ops = &state.conflict_ops;
    let vcs_kind = conflict_vcs_kind_for_project(state, project_id);
    let git_actions_supported = vcs_kind == Some(VcsKind::Git);
    let abort_supported = git_actions_supported && project_has_git_merge_state(state, project_id);
    let editor_configured = state.config.external_editor.is_some();
    let merge_tool_configured = state.config.external_merge_tool.is_some();

    // R5: the one invariant this migration must not drop. `close_msg` is
    // `None` while `Operating` — both the header close (via `surface`'s own
    // `on_close`) and the footer Close button below key off this same
    // value, so neither can ever be pressable during a non-cancellable
    // phase. See the review request for how this was checked.
    let close_msg = (!matches!(
        ops.phase,
        crate::state::conflict_ops::ConflictPhase::Operating { .. }
    ))
    .then_some(Message::ConflictOps(ConflictOpsMessage::PanelClosed));

    let content: Element<'_, Message> = match &ops.phase {
        crate::state::conflict_ops::ConflictPhase::Loading(id) if id == project_id => {
            text(state.t("plain.resolve.loading"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
                .into()
        }
        crate::state::conflict_ops::ConflictPhase::Operating {
            project_id: id,
            action,
        } if id == project_id => column![
            text(action)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            text(state.t("plain.resolve.working_hint"))
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        ]
        .spacing(8)
        .into(),
        crate::state::conflict_ops::ConflictPhase::Done {
            project_id: id,
            success,
            message,
            result,
        } if id == project_id => {
            // `title` and `message` were the same text rendered twice (both
            // derive from `*success`); the notice primitive's tone now
            // carries that distinction, so only `message` (state-provided)
            // is shown, once.
            let tone = if *success {
                NoticeTone::Success
            } else {
                NoticeTone::Danger
            };
            let banner = notice(tokens, tone, None, message.as_str(), None);

            let details_label = if state.show_op_details {
                state.t("plain.hide_details")
            } else {
                state.t("plain.show_details")
            };
            let mut result_col = column![
                banner,
                styled_button(
                    tokens,
                    details_label,
                    Some(Message::ToggleOpDetails),
                    style::ghost,
                ),
            ]
            .spacing(8);

            if state.show_op_details
                && let Some(result) = result
            {
                for command in &result.commands_executed {
                    result_col = result_col.push(
                        text(format!("command: {command}"))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                }
                if !result.stdout.is_empty() {
                    result_col = result_col.push(
                        text(format!("stdout: {}", result.stdout))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                }
                if !result.stderr.is_empty() {
                    result_col = result_col.push(
                        text(format!("stderr: {}", result.stderr))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                }
                if let Some(error) = &result.error_message {
                    result_col = result_col.push(
                        text(format!("error: {error}"))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens,
                            )),
                    );
                }
            }

            result_col.into()
        }
        _ => {
            let detail = match &ops.phase {
                crate::state::conflict_ops::ConflictPhase::Browsing {
                    project_id: id,
                    detail,
                } if id == project_id => Some(detail),
                _ => ops.cached.get(project_id),
            };

            if let Some(detail) = detail {
                if detail.conflicted_files.is_empty() {
                    text(state.t("plain.resolve.no_files"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens))
                        .into()
                } else {
                    let file_rows: Vec<Element<'_, Message>> = detail
                        .conflicted_files
                        .iter()
                        .map(|f| {
                            let editor_reason = (!editor_configured)
                                .then_some(state.t("plain.resolve.editor_not_configured"));
                            let open_editor_msg =
                                editor_configured.then_some(Message::ConflictOps(
                                    ConflictOpsMessage::OpenInEditorRequested(f.path.clone()),
                                ));
                            let merge_tool_reason = (!merge_tool_configured)
                                .then_some(state.t("plain.resolve.merge_tool_not_configured"));
                            let open_merge_tool_msg =
                                merge_tool_configured.then_some(Message::ConflictOps(
                                    ConflictOpsMessage::OpenInMergeToolRequested(f.path.clone()),
                                ));

                            // RFC-054 D1/D4: line 1 always has the icon,
                            // path, spacer and the two launch controls — a
                            // button-kind third slot joins them inline
                            // (R1/R3: Git rows are otherwise unchanged);
                            // prose gets its own full-width line instead
                            // (R2), because a row sized for buttons cannot
                            // hold a sentence up to four times as wide.
                            let mut line1 = row![
                                text("!")
                                    .size(snora::design::style::text::body_size(tokens))
                                    .line_height(snora::design::style::text::body_line_height(
                                        tokens
                                    ))
                                    .width(Length::Fixed(22.0)),
                                text(&f.path)
                                    .size(snora::design::style::text::body_size(tokens))
                                    .line_height(snora::design::style::text::body_line_height(
                                        tokens
                                    ))
                                    .width(Length::Fill),
                                Space::new().width(Length::Fixed(8.0)),
                                // RFC-037 Stage 6: migrated onto the shared
                                // `reasoned` primitive. `style::secondary`
                                // matches the third slot's own weight when
                                // it is a button — both are peripheral row
                                // actions, not this phase's one completing
                                // action.
                                reasoned(
                                    tokens,
                                    state.t("plain.resolve.open_editor"),
                                    open_editor_msg,
                                    editor_reason,
                                    false,
                                    style::secondary,
                                ),
                                // Handoff 058: same control, same weight,
                                // same reasoned-disabled shape as the editor
                                // button beside it — an editor and a merge
                                // tool are different jobs, always shown,
                                // each with its own reason when
                                // unconfigured, rather than the row's shape
                                // depending on Settings.
                                reasoned(
                                    tokens,
                                    state.t("plain.resolve.open_merge_tool"),
                                    open_merge_tool_msg,
                                    merge_tool_reason,
                                    false,
                                    style::secondary,
                                ),
                            ]
                            .align_y(Alignment::Center)
                            .spacing(6);

                            let mut file_col = column![].spacing(4);
                            match third_slot_for(state, tokens, vcs_kind, project_id, &f.path) {
                                ThirdSlot::Button(control) => {
                                    line1 = line1.push(control);
                                    file_col = file_col.push(line1);
                                }
                                ThirdSlot::Prose(sentence) => {
                                    file_col = file_col.push(line1);
                                    // RFC-054 D3: indented 28px — the 22px
                                    // glyph plus line 1's own 6px
                                    // `.spacing(6)` before the path — so
                                    // this reads as belonging to that file,
                                    // not as a new list item. Wraps rather
                                    // than truncates (§4): the sentence is
                                    // the thing the user needs to read, and
                                    // this line exists to give it room —
                                    // `text`'s default behaviour under a
                                    // constrained `Fill` width already
                                    // wraps, so no extra configuration is
                                    // needed to get that.
                                    file_col = file_col.push(row![
                                        Space::new().width(Length::Fixed(28.0)),
                                        text(sentence)
                                            .size(snora::design::style::text::body_small_size(
                                                tokens
                                            ))
                                            .line_height(
                                                snora::design::style::text::body_small_line_height(
                                                    tokens
                                                )
                                            )
                                            .width(Length::Fill),
                                    ]);
                                }
                            }
                            file_col.into()
                        })
                        .collect();
                    // No inner `scrollable` here (unlike the original) —
                    // `surface`'s own body scrollable covers the whole
                    // `content` slot; nesting a second independently
                    // scrollable region inside it would be the same
                    // accessibility anti-pattern RFC-035 avoided elsewhere.
                    column(file_rows).spacing(8).into()
                }
            } else {
                text(state.t("plain.resolve.loading"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .into()
            }
        }
    };

    let stop_control: Element<'_, Message> = if abort_supported {
        styled_button(
            tokens,
            state.t("plain.resolve.stop_attempt"),
            Some(Message::ConflictOps(
                ConflictOpsMessage::AbortMergeRequested(project_id.clone()),
            )),
            style::secondary,
        )
    } else {
        Space::new().width(Length::Fixed(0.0)).into()
    };

    let footer = row![
        stop_control,
        Space::new().width(Length::Fill),
        styled_button(
            tokens,
            state.t("action.close"),
            close_msg.clone(),
            style::ghost
        ),
    ]
    .align_y(Alignment::Center);

    let body = column![
        text(state.t("plain.resolve.instruction"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        content,
    ]
    .spacing(14);

    surface(
        tokens,
        // RFC-051 D4: was Small — 400px, chosen when each file row held one
        // control. It now holds up to three (editor, comparison tool,
        // mark-done or the Jujutsu hint), which need roughly 300px between
        // them; see the Handoff 070 review request for the row arithmetic
        // this floor was set against.
        OverlayWidth::Large.resolve(state.window_width),
        format!("{} — {}", state.t("plain.resolve.title"), name),
        close_msg,
        // No real focus-order exists for this overlay's controls yet — R3
        // forbids `app/`/`state/` changes this stage, and RFC-036 Stage 3
        // scoped only the three workspace-manager dialogs into a real
        // `FocusTarget` order, naming this file explicitly as later work.
        // `false` here is honest: no ring can show for a target that is
        // never tracked, the same as before this migration (the original
        // hand-rolled buttons had no ring capability at all).
        false,
        body,
        footer,
    )
}

/// A button styled with one of `knotra_ui::widget::style`'s semantic
/// functions plus a focus ring — the same shape `workspace_manager.rs`
/// (RFC-034 R9's validating migration) uses for every dialog button.
/// `is_focused` is always `false` here (see `resolve_panel`'s own note);
/// kept as a real parameter position, not hardcoded into the ring call, so
/// a later stage that wires real focus tracking only has to change what is
/// passed in, not this helper's shape.
fn styled_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'a, Message> {
    let t = tokens.clone();
    iced::widget::button(
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    )
    .height(BUTTON_HEIGHT)
    .padding([0, 18])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, false, style_fn(&t, status)))
    .into()
}

/// RFC-054 D4: the conflicted-file row's third slot, as a value rather than
/// something implied by which match arm happened to build which widget.
/// Neither this project nor its reviewer can render this panel, so without
/// this enum "prose and buttons are laid out differently" would only be
/// checkable by looking at it — with it, R7's test asserts the
/// classification directly and only the pixels stay unverifiable.
enum ThirdSlot<'a> {
    /// Git: a real completing action, styled like `plain.resolve.open_editor`
    /// and `plain.resolve.open_merge_tool` beside it — stays inline on the
    /// row (D1/R1).
    Button(Element<'a, Message>),
    /// Jujutsu's hint or the no-evidence `None` case — a sentence, not a
    /// control. Owned rather than borrowed: `jj_finish_hint` builds it with
    /// `format!`, so there is no existing `&str` of the right lifetime to
    /// borrow (D4's "shape is yours" — this is the shape that avoids
    /// threading a lifetime through a value that is not always a borrow to
    /// begin with).
    Prose(String),
}

/// RFC-054 D4/R6/R7: the same exhaustive `Option<VcsKind>` match Handoff 065
/// built for `mark_control`, now returning the row's third-slot *kind*
/// rather than a pre-laid-out `Element` — `resolve_panel` decides where each
/// kind goes (D1-D3), this function only decides which kind a project gets.
/// Still exhaustive, still wildcard-free: a third `VcsKind` is a compile
/// error here exactly as it was before this handoff.
fn third_slot_for<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    vcs_kind: Option<VcsKind>,
    project_id: &ProjectId,
    file_path: &str,
) -> ThirdSlot<'a> {
    match vcs_kind {
        Some(VcsKind::Git) => {
            let t = tokens.clone();
            ThirdSlot::Button(
                iced::widget::button(
                    text(state.t("plain.resolve.mark_done"))
                        .size(snora::design::style::text::label_size(tokens)),
                )
                .height(36.0)
                .padding([0, 10])
                .on_press(Message::ConflictOps(
                    ConflictOpsMessage::MarkResolvedRequested {
                        project_id: project_id.clone(),
                        file_path: file_path.to_owned(),
                    },
                ))
                .style(move |_theme, status| {
                    style::with_focus_ring(&t, false, style::secondary(&t, status))
                })
                .into(),
            )
        }
        // D1 deleted `ProjectConflictDetail.note`, the VCS layer's discarded
        // English sentence for this same slot — the completing action is
        // computed here instead, from `VcsKind` plus this row's own file
        // path, so it is never English-only prose baked into a record
        // (RFC-046 D1's contract).
        Some(VcsKind::Jujutsu) => ThirdSlot::Prose(jj_finish_hint(state, file_path)),
        // Handoff 065: no evidence either way — the pre-RFC-045 message,
        // generic and therefore not asserting anything that could be false,
        // rather than guessing a specific command that might be wrong.
        None => ThirdSlot::Prose(state.t("plain.resolve.unsupported").to_owned()),
    }
}

/// RFC-045 D2, Handoff 065: `Option<VcsKind>`, not a bare `VcsKind` — there
/// is no value in a two-variant enum that means "I could not tell," and this
/// function sometimes cannot tell. `None` covers exactly the cases the old
/// Git-only bool this replaced had no evidence for: a project in neither
/// `workspace_status` nor `workspace.projects`, and a project present in
/// `workspace.projects` whose path has neither a `.git` nor a `.jj` marker
/// (the state `missing_projects` tracks, `!VcsAdapter::repo_exists`,
/// RFC-046). `Some(Jujutsu)` is returned whenever `.jj` is a directory,
/// including a colocated repo that also has `.git` — a `.jj` marker is
/// still evidence, so this is not a no-evidence case. Every reachable
/// outcome now either matches the pre-RFC-045 boolean exactly (`Some(Git)`
/// where it was `true`, `Some(Jujutsu)`/`None` both where it was `false`,
/// collapsed there because that old bool never distinguished "confirmed
/// Jujutsu" from "unknown") or is the new distinction RFC-045 exists to
/// draw — with no appeal to how rarely the no-evidence path is reached,
/// because `mark_control`'s `None` arm renders the same generic,
/// nothing-asserted message the bool's `false` case always rendered.
fn conflict_vcs_kind_for_project(state: &AppState, project_id: &ProjectId) -> Option<VcsKind> {
    state
        .workspace_status
        .as_ref()
        .and_then(|ws| {
            ws.projects
                .iter()
                .find(|status| &status.project_id == project_id)
        })
        .map(|status| status.identity.vcs_kind)
        .or_else(|| {
            state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|project| &project.id == project_id))
                .and_then(|project| {
                    let path = std::path::Path::new(&project.path);
                    if path.join(".jj").is_dir() {
                        Some(VcsKind::Jujutsu)
                    } else if path.join(".git").exists() {
                        Some(VcsKind::Git)
                    } else {
                        None
                    }
                })
        })
}

/// RFC-045 D2/D4: names the completing action with this row's own file
/// path — `jj resolve <path>`, not a generic sentence — in place of the
/// discarded `ProjectConflictDetail.note` (D1). `resolve` and `jj` are not
/// in `FORBIDDEN_EN`; extracted from the match arm that calls it so its
/// wording is a plain string comparison in tests rather than something only
/// checkable by rendering an `Element`.
fn jj_finish_hint(state: &AppState, file_path: &str) -> String {
    format!(
        "{} `jj resolve {file_path}`",
        state.t("plain.resolve.jj_finish_hint")
    )
}

fn project_has_git_merge_state(state: &AppState, project_id: &ProjectId) -> bool {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|project| &project.id == project_id))
        .map(|project| {
            let path = std::path::Path::new(&project.path);
            path.join(".git").join("MERGE_HEAD").exists()
        })
        .unwrap_or(false)
}

/// Moved from `overlays/mod.rs` (RFC-037 Stage 6, `131` §5) — its only
/// caller is `resolve_panel`, above, in this same file. `mod.rs`'s own
/// Stage 1 doc comment mistakenly justified keeping it there as "used by
/// more than one overlay"; that was corrected in review `131` and the move
/// deferred to this stage so it wouldn't bury a relocation inside an
/// unverifiable migration diff.
fn project_name_for(state: &AppState, id: &ProjectId) -> String {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use knotra_vcs::{
        ConflictStatus, RemoteStatus, RepositoryIdentity, WorkingTreeStatus, WorkspaceStatus,
    };

    fn status_for(project_id: ProjectId, vcs_kind: VcsKind) -> knotra_vcs::ProjectStatus {
        knotra_vcs::ProjectStatus {
            project_id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind,
            },
            context: None,
            remote: RemoteStatus::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: ConflictStatus::default(),
            refreshed_at: chrono::Utc::now(),
            read_error: None,
        }
    }

    fn state_with_status(vcs_kind: VcsKind) -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project_id = ProjectId::new();
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![status_for(project_id.clone(), vcs_kind)],
            last_refresh: None,
        });
        (state, project_id)
    }

    /// RFC-045 D2/R6: this is the coverage half — a Jujutsu project's own
    /// recorded `VcsKind` must actually reach `conflict_vcs_kind_for_project`
    /// as `Jujutsu`, not merely happen to pair correctly with whichever arm
    /// a broken lookup fell into (the same coverage-vs-pairing distinction
    /// `062` drew for `label_en`).
    #[test]
    fn conflict_vcs_kind_for_project_reports_jujutsu_for_a_jj_project() {
        let (state, project_id) = state_with_status(VcsKind::Jujutsu);
        assert_eq!(
            conflict_vcs_kind_for_project(&state, &project_id),
            Some(VcsKind::Jujutsu)
        );
    }

    /// The Git-side counterpart — R3 requires Git rows stay unchanged, which
    /// starts with this refactored lookup still resolving Git projects to
    /// `VcsKind::Git` exactly as the bool it replaced did.
    #[test]
    fn conflict_vcs_kind_for_project_reports_git_for_a_git_project() {
        let (state, project_id) = state_with_status(VcsKind::Git);
        assert_eq!(
            conflict_vcs_kind_for_project(&state, &project_id),
            Some(VcsKind::Git)
        );
    }

    /// Handoff 065: a project the workspace knows about, but whose on-disk
    /// path has neither a `.git` nor a `.jj` marker — exactly what
    /// `missing_projects` tracks (RFC-046, `!VcsAdapter::repo_exists`), not
    /// a hypothetical. No `workspace_status` entry either, so this drives
    /// the fallback's on-disk check through a real empty directory rather
    /// than asserting the `None` branch in isolation.
    #[test]
    fn conflict_vcs_kind_for_project_reports_none_with_neither_marker_present() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let mut state = AppState::new(AppConfig::default());
        let project = knotra_vcs::Project::new("svc", tmp.path().to_string_lossy());
        let project_id = project.id.clone();
        state.workspace = Some(knotra_vcs::Workspace {
            projects: vec![project],
            ..knotra_vcs::Workspace::new("Test")
        });

        assert_eq!(conflict_vcs_kind_for_project(&state, &project_id), None);
    }

    /// Handoff 065: a project absent from both `workspace_status` and
    /// `workspace.projects` — the other no-evidence case named in `154` §3.
    #[test]
    fn conflict_vcs_kind_for_project_reports_none_when_absent_from_both_sources() {
        let state = AppState::new(AppConfig::default());
        let project_id = ProjectId::new();

        assert_eq!(conflict_vcs_kind_for_project(&state, &project_id), None);
    }

    /// The content half: the jj hint names the completing action and this
    /// row's own file path, not a generic sentence — the thing `note`
    /// (RFC-046 D1, deleted here) used to say and nothing since replaced.
    #[test]
    fn jj_finish_hint_names_the_command_with_the_files_path() {
        let state = AppState::new(AppConfig::default());
        let hint = jj_finish_hint(&state, "src/lib.rs");
        assert_eq!(hint, "Finish with: `jj resolve src/lib.rs`");
    }

    /// RFC-054 D4/R7: the classification the whole RFC exists to make
    /// assertable. Without `ThirdSlot` as a value, "Git gets a button, jj
    /// and `None` get prose" is only checkable by rendering the panel,
    /// which nobody working on this project can do. `matches!` only checks
    /// the variant, not the inner content — the content itself
    /// (`plain.resolve.mark_done`'s label, the jj hint's exact text) is
    /// already covered by other tests; this one is purely about which kind
    /// each `Option<VcsKind>` produces.
    #[test]
    fn third_slot_is_a_button_for_git_and_prose_for_jujutsu_and_none() {
        let state = AppState::new(AppConfig::default());
        let tokens = &state.theme.tokens;
        let project_id = ProjectId::new();

        assert!(matches!(
            third_slot_for(
                &state,
                tokens,
                Some(VcsKind::Git),
                &project_id,
                "src/lib.rs"
            ),
            ThirdSlot::Button(_)
        ));
        assert!(matches!(
            third_slot_for(
                &state,
                tokens,
                Some(VcsKind::Jujutsu),
                &project_id,
                "src/lib.rs"
            ),
            ThirdSlot::Prose(_)
        ));
        assert!(matches!(
            third_slot_for(&state, tokens, None, &project_id, "src/lib.rs"),
            ThirdSlot::Prose(_)
        ));
    }
}
