//! 3. "Change work area" modal (Context Switch) — RFC-037 Stage 4.
//!
//! `context_switch.rs`, not `context.rs` (RFC-041 D5 naming, applied here) —
//! `app/context.rs` already exists one level up. Different module tree, so
//! no collision, but the name avoids reading ambiguously.
//!
//! `modal_shell` replaced with `knotra_ui::widget::overlay::surface`.
//! `guided_button`/`guided_field_focused` call sites are untouched (D6/R11,
//! D7/R12). Each phase's own local `footer` row (`BrowsingList`,
//! `ConfirmSwitch`, `Done`) maps directly onto `surface()`'s `footer`
//! parameter, the same mapping Stage 2 (`conflict.rs`) and Stage 4
//! (`freezer.rs`) used; phases with no such row (`Idle`, `LoadingList`,
//! `Switching`) pass an empty `Space` instead.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, row, text},
};

use knotra_ui::widget::{
    Tone, body_badge_diameter, ghost_control, guided_field_entry, list,
    overlay::{OverlayWidth, surface},
    reasoned_control_maybe, status_badge, status_symbol, style,
};
use knotra_vcs::ContextTarget;

use crate::{
    message::{ContextMessage, Message},
    state::AppState,
};

/// RFC-075 stage 3 §3: Change work area's own entry per phase — the
/// search field while browsing; `Cancel` for the confirmation
/// (`SwitchConfirmed` starts a change, never the entry); "Close" when
/// done. `Idle`/`LoadingList`/`Switching` have no controls.
pub fn entry_id(state: &AppState) -> Option<iced::widget::Id> {
    use crate::state::context::ContextPhase;

    if !matches!(state.active_modal, crate::state::ActiveModal::Switch) {
        return None;
    }
    match &state.context_ops.phase {
        ContextPhase::Idle | ContextPhase::LoadingList(_) | ContextPhase::Switching { .. } => None,
        ContextPhase::BrowsingList { .. } => {
            Some(knotra_ui::widget::focus_id::SWITCH_TARGET.clone())
        }
        ContextPhase::ConfirmSwitch { .. } => Some(crate::view::ids::context_switch::cancel()),
        ContextPhase::Done(_) => Some(crate::view::ids::context_switch::done_close()),
    }
}

fn context_target_kind_key(target: &ContextTarget) -> &'static str {
    match target {
        ContextTarget::GitLocalBranch { .. } | ContextTarget::Manual { .. } => {
            "plain.switch.kind_local"
        }
        ContextTarget::GitRemoteBranch { .. } => "plain.switch.kind_shared",
        ContextTarget::JjBookmark { .. } => "plain.switch.kind_saved_name",
        ContextTarget::JjChange { .. } => "plain.switch.kind_change",
    }
}

pub fn switch_modal(state: &AppState) -> Element<'_, Message> {
    use crate::state::context::ContextPhase;

    let tokens = &state.theme.tokens;
    let ctx = &state.context_ops;

    let (inner, footer): (Element<'_, Message>, Element<'_, Message>) = match &ctx.phase {
        ContextPhase::Idle => (
            column![
                text(state.t("plain.switch.no_project"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::LoadingList(_) => (
            column![
                // RFC-073 D16: this is loading, not refreshing — the one
                // corrected reuse the amendment names (`context_switch.rs`'s
                // own list-loading line borrowed `plain.status.checking`,
                // which Stage 4 repoints at "Refreshing…").
                text(state.t("detail.loading"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                // RFC-071 Amendment A3 D14: instructional guidance, read to
                // understand what to do next — `body`.
                text(state.t("plain.switch.loading_hint"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::BrowsingList {
            project_id, search, ..
        } => {
            let candidates = ctx.filtered_candidates();
            let highlighted_on_submit = ctx.highlighted.and_then(|i| candidates.get(i)).map(|c| {
                Message::Context(ContextMessage::SwitchTargetChosen(
                    project_id.clone(),
                    c.target.clone(),
                    c.label.clone(),
                ))
            });
            let search_field = guided_field_entry(
                tokens,
                state.t("plain.switch.search_label"),
                state.t("plain.switch.search_hint"),
                search,
                |s| Message::Context(ContextMessage::SearchChanged(s)),
                knotra_ui::widget::focus_id::SWITCH_TARGET.clone(),
                highlighted_on_submit,
            );

            let mut rows = column![search_field].spacing(8);
            if candidates.is_empty() {
                // RFC-071 Amendment A3 D14: a status message — `body`.
                rows = rows.push(
                    text(state.t("plain.switch.no_targets"))
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                );
            } else {
                let mut candidate_rows: Vec<Element<'_, Message>> = Vec::new();
                for (i, candidate) in candidates.iter().enumerate() {
                    let reason_key = candidate
                        .is_current
                        .then_some("plain.switch.reason_current");
                    let kind = state.t(context_target_kind_key(&candidate.target));
                    let detail = candidate.target.display_target();
                    let label = column![
                        text(candidate.label.as_str())
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                        // RFC-071 Amendment A3 D14: kept body_small — the
                        // VCS kind and the branch/bookmark detail beside a
                        // candidate's name, both explicitly incidental
                        // metadata.
                        text(format!("{kind} · {detail}"))
                            .size(snora::design::style::text::body_small_size(tokens))
                            .line_height(snora::design::style::text::body_small_line_height(
                                tokens
                            )),
                    ]
                    .spacing(2);
                    let press = reason_key.is_none().then_some(Message::Context(
                        ContextMessage::SwitchTargetChosen(
                            project_id.clone(),
                            candidate.target.clone(),
                            candidate.label.clone(),
                        ),
                    ));
                    let t = tokens.clone();
                    // RFC-075 stage 3 §4: a highlight, not real focus — the
                    // field keeps real focus throughout (§4's one Tab-stop
                    // exception).
                    let is_highlighted = ctx.highlighted == Some(i);
                    let candidate_btn = iced::widget::button(label)
                        .width(Length::Fill)
                        .on_press_maybe(press)
                        .style(move |_theme, status| {
                            style::with_focus_ring(&t, is_highlighted, style::ghost(&t, status))
                        });
                    let mut row_col = column![candidate_btn];
                    if let Some(reason_key) = reason_key {
                        // RFC-071 Amendment A3 D14: a reason — `body`.
                        row_col = row_col.push(
                            text(state.t(reason_key))
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens)),
                        );
                    }
                    candidate_rows.push(row_col.spacing(2).into());
                }
                // No inner `scrollable` around the candidate list (unlike
                // the pre-migration `.height(Length::Fixed(220.0))` box) —
                // `surface()`'s own body scrollable now covers the whole
                // body, same reasoning as Stage 2/3 (review `132` §4).
                //
                // RFC-071 D8a: the candidates are a list of items — one
                // border line between adjacent candidates.
                rows = rows.push(list(tokens, candidate_rows));
            }

            let footer = row![
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.cancel"),
                    Message::Context(ContextMessage::BulkModalClosed),
                    crate::view::ids::context_switch::cancel(),
                ),
            ]
            .align_y(Alignment::Center);
            (rows.into(), footer.into())
        }

        ContextPhase::ConfirmSwitch {
            project_name,
            target_label,
            target,
            is_dirty,
            disabled_reason_key,
            ..
        } => {
            let caution = if let Some(reason_key) = disabled_reason_key {
                state.t(reason_key)
            } else if *is_dirty {
                state.t("plain.switch.dirty_hint")
            } else {
                state.t("plain.no_next_step")
            };
            let switch_reason = if state.operation_interlock.is_busy() {
                Some(state.t("plain.activity.busy"))
            } else {
                disabled_reason_key.map(|key| state.t(key))
            };
            let switch = reasoned_control_maybe(
                tokens,
                state.t("plain.change_work_area"),
                (disabled_reason_key.is_none() && !state.operation_interlock.is_busy())
                    .then_some(Message::Context(ContextMessage::SwitchConfirmed)),
                switch_reason,
                style::primary,
                crate::view::ids::context_switch::confirm(),
            );
            let footer = row![
                switch,
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.cancel"),
                    Message::Context(ContextMessage::SwitchCancelled),
                    crate::view::ids::context_switch::cancel(),
                ),
            ]
            .align_y(Alignment::Center);

            (
                column![
                    text(project_name)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    text(target_label)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    // RFC-071 Amendment A3 D14: kept body_small — the VCS
                    // kind label beside the target's own name.
                    text(state.t(context_target_kind_key(target)))
                        .size(snora::design::style::text::body_small_size(tokens))
                        .line_height(snora::design::style::text::body_small_line_height(tokens)),
                    // A caution/reason the user must read before confirming
                    // — `body`.
                    text(caution)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                ]
                .spacing(12)
                .into(),
                footer.into(),
            )
        }

        ContextPhase::Switching { target_label, .. } => (
            column![
                text(state.t("plain.switch.working"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                // RFC-071 Amendment A3 D14: moved to `body` — this names
                // what is being switched to, the status line's own content,
                // not a detail beside a name (D14's "branch beside a name"
                // exemption needs a name to sit beside; here it is the
                // whole message).
                text(target_label)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8)
            .into(),
            Space::new().into(),
        ),

        ContextPhase::Done(result) => {
            // RFC-073 D9: `status_badge` beside the title — this screen
            // had no status symbol of any kind before (success/failure
            // were title-text-only); success/check and failure/alert, the
            // same two outcomes `badge_for_operation_outcome` gives, named
            // directly here (a single switch has no "skipped" outcome).
            let (title, body, badge) = if result.operation_result.success {
                (
                    state.t("plain.switch.done_title"),
                    state.t("plain.no_next_step"),
                    (Tone::Success, status_symbol::all_set()),
                )
            } else {
                (
                    state.t("plain.switch.failed_title"),
                    state.t("plain.switch.failed_hint"),
                    (Tone::Danger, status_symbol::alert()),
                )
            };

            let mut detail_col = column![
                row![
                    status_badge(tokens, badge.0, &badge.1, body_badge_diameter(tokens)),
                    text(title).size(snora::design::style::text::title_size(tokens)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                // RFC-071 Amendment A3 D14: moved to `body` — same reasoning
                // as the `Switching` phase above: the target name is this
                // line's own content, not a detail beside another name.
                text(result.target.as_str())
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                text(body)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(8);

            // RFC-071 Amendment A3 D14: moved to `body` — this diagnostic
            // detail is read, under "Show details", to understand why the
            // switch failed and how to recover; D14's `body_small` bucket
            // is incidental metadata only (timestamps, VCS kind, branch,
            // counts, keyboard hints), which this is not.
            if !result.operation_result.success && state.show_op_details {
                if let Some(hint) = &result.recovery_hint {
                    detail_col = detail_col.push(
                        text(hint.situation.as_str())
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                    );
                    for cmd in &hint.suggested_commands {
                        detail_col = detail_col.push(
                            text(format!("  {}", cmd))
                                .size(snora::design::style::text::body_size(tokens))
                                .line_height(snora::design::style::text::body_line_height(tokens)),
                        );
                    }
                }
                for cmd in &result.operation_result.commands_executed {
                    detail_col = detail_col.push(
                        text(format!("  {}", cmd))
                            .size(snora::design::style::text::body_size(tokens))
                            .line_height(snora::design::style::text::body_line_height(tokens)),
                    );
                }
            }

            let details_label = if state.show_op_details {
                state.t("plain.hide_details")
            } else {
                state.t("plain.show_details")
            };

            let footer = row![
                ghost_control(
                    tokens,
                    details_label,
                    Message::ToggleOpDetails,
                    crate::view::ids::context_switch::show_details(),
                ),
                Space::new().width(Length::Fill),
                ghost_control(
                    tokens,
                    state.t("action.close"),
                    Message::Context(ContextMessage::BulkModalClosed),
                    crate::view::ids::context_switch::done_close(),
                ),
            ]
            .align_y(Alignment::Center);

            (detail_col.into(), footer.into())
        }
    };

    // R2/§2: unchanged from before this migration.
    let close_msg = (!matches!(ctx.phase, ContextPhase::Switching { .. }))
        .then_some(Message::Context(ContextMessage::BulkModalClosed));

    knotra_ui::widget::focus_by_id_window(
        surface(
            tokens,
            OverlayWidth::Large.resolve(state.window_width),
            state.t("plain.change_work_area"),
            close_msg,
            crate::view::ids::context_switch::close(),
            inner,
            footer,
        ),
        &[],
    )
}
