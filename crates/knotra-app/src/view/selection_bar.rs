//! RFC-0009 — Selection bar view.
//!
//! Rendered as a sticky row at the bottom of the main content area whenever
//! ≥ 1 project is selected. Displays the count and primary action buttons.
//!
//! RFC-035 R13/R14/Handoff 030 §6: the four actions used to each carry
//! their own `guided_button` reason cascade, so "Busy" or "Choose at least
//! one project" appeared up to four times in one viewport (Stage 4's own
//! 2x2 capture showed exactly this — `060` finding 5). R13's shared
//! reasons (busy, and choose-at-least-one) now render once, in a single
//! group-level slot below the count; R14's action-specific reasons (fetch
//! has nothing checkable, pull has no upstream, switch needs exactly one
//! project) render in one contextual slot beneath the actions, each
//! labelled by its own action, rather than per-button. Neither is dropped —
//! R14 requires they "remain available", just consolidated rather than
//! repeated.
//!
//! This moves the four actions off `guided_button` (whose
//! reason-beneath-itself composition is exactly what created the
//! duplication) onto a plain button — the same narrow migration
//! `select_mode_button` made in Stage 2 for the same reason.

use iced::{
    Alignment, Element, Length,
    advanced::text::Wrapping,
    widget::{Space, column, container, row, text},
};

use knotra_ui::widget::{ghost_control, secondary_control_maybe};

use crate::{
    message::{ContextMessage, FreezerMessage, Message, SelectionMessage, SyncMessage},
    state::AppState,
    view::{dashboard::WidthMode, ids},
};

pub fn view(state: &AppState, mode: WidthMode) -> Option<Element<'_, Message>> {
    // Selection bar only shown while in selection mode.
    if !state.selection_mode {
        return None;
    }

    let summary = state.selection_summary();
    let count = summary.selected_count;
    let label = if count == 0 {
        state.t("plain.selection.none").to_owned()
    } else {
        format!(
            "✓  {}  {}",
            count,
            state.t("plain.selection.selected_suffix")
        )
    };

    // RFC-074 Amendment A1 (D6): the one place allowed to read the raw
    // selection directly (`selection_raw_read_guard.rs`'s own allowlist) —
    // only to report the gap between it and what's shown, never to act on
    // it. A tick for a project that no longer exists cannot appear here:
    // `prune_selection_to_existing_projects` already removes those.
    let hidden_count = state
        .selection
        .selected_ids
        .len()
        .saturating_sub(summary.selected_count);

    let busy = state.operation_interlock.is_busy();

    // R13: one shared reason for the whole group — busy takes precedence
    // over count-zero, matching the cascade order every action already
    // used individually before this change.
    let group_reason = if busy {
        Some(state.t("plain.activity.busy"))
    } else if count == 0 {
        Some(state.t("plain.disabled.choose_one"))
    } else {
        None
    };

    let can_act = count > 0 && !busy;
    let fetch_msg = (can_act && !summary.fetchable_ids.is_empty())
        .then_some(Message::Sync(SyncMessage::BulkFetchRequested));
    let pull_msg =
        (can_act && summary.has_upstream).then_some(Message::Sync(SyncMessage::BulkPullRequested));
    let tag_msg = can_act.then_some(Message::Freezer(FreezerMessage::BulkOpenRequested));
    let switch_msg =
        (count == 1 && !busy).then_some(Message::Context(ContextMessage::BulkOpenRequested));

    // R14: action-specific reasons, shown only when the shared (group)
    // reason above does not already explain the disablement. Each is
    // labelled by its own action so more than one can be listed at once
    // without ambiguity — one contextual slot, not a fourth `guided_button`
    // repeating the busy/choose-one text it would otherwise still carry.
    let mut contextual_reasons: Vec<String> = Vec::new();
    if can_act {
        if summary.fetchable_ids.is_empty() {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.check_for_updates"),
                state.t("plain.selection.none_fetchable")
            ));
        }
        if !summary.has_upstream {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.get_latest"),
                state.t("plain.disabled.no_upstream")
            ));
        }
        if count > 1 {
            contextual_reasons.push(format!(
                "{}: {}",
                state.t("plain.change_work_area"),
                state.t("plain.selection.choose_one_work_area")
            ));
        }
    }

    let tokens = &state.theme.tokens;
    let fetch_btn = secondary_control_maybe(
        tokens,
        state.t("plain.check_for_updates"),
        fetch_msg,
        ids::selection_bar::fetch(),
    );
    let pull_btn = secondary_control_maybe(
        tokens,
        state.t("plain.get_latest"),
        pull_msg,
        ids::selection_bar::pull(),
    );
    let tag_btn = secondary_control_maybe(
        tokens,
        state.t("plain.save_release_point"),
        tag_msg,
        ids::selection_bar::tag(),
    );
    let switch_btn = secondary_control_maybe(
        tokens,
        state.t("plain.change_work_area"),
        switch_msg,
        ids::selection_bar::switch(),
    );

    // Exit selection: `ghost`-styled, matching `row.rs`'s "Show details" —
    // a non-mutating, lower-weight action, not a fourth `secondary` button.
    let clear_btn = ghost_control(
        tokens,
        state.t("plain.exit_selection"),
        Message::Selection(SelectionMessage::ModeExited),
        ids::selection_bar::exit(),
    );

    // RFC-071 Amendment A3 D14: the selection count is this bar's own
    // status — read to confirm what the four actions below will act on,
    // not a count incidental to something else — `body`.
    let command_row = row![
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        clear_btn,
    ]
    .align_y(Alignment::Center);

    // RFC-035 R8/Handoff 028 §4: a 2x2 grid at compact width, chosen over an
    // action menu — these four actions are already always-visible in
    // standard mode, so hiding them behind a menu would cost an extra click
    // for something users already expect at a glance; two rows of two keeps
    // every action visible and reachable, just narrower. Standard/wide keep
    // the single four-wide row unchanged.
    let actions: Element<'_, Message> = match mode {
        WidthMode::Compact => column![
            row![
                container(fetch_btn).width(Length::FillPortion(1)),
                container(pull_btn).width(Length::FillPortion(1)),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
            row![
                container(tag_btn).width(Length::FillPortion(1)),
                container(switch_btn).width(Length::FillPortion(1)),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        ]
        .spacing(8)
        .into(),
        WidthMode::Standard | WidthMode::Wide => row![
            container(fetch_btn).width(Length::FillPortion(1)),
            container(pull_btn).width(Length::FillPortion(1)),
            container(tag_btn).width(Length::FillPortion(1)),
            container(switch_btn).width(Length::FillPortion(1)),
        ]
        .spacing(8)
        .align_y(Alignment::Start)
        .into(),
    };

    // RFC-071 Amendment A3 D14: reasons — `body`.
    let mut content = column![command_row].spacing(6);
    if hidden_count > 0 {
        // RFC-074 Amendment A1 (D6) §3: "Selected but not shown: n (not
        // included)" / 「選択中で非表示: n 件（対象外）」 — the owner's own
        // approved text, exact in both languages for any count.
        // `Wrapping::None`, as `plan_summary_line` already settled on for
        // the same "never wraps inside a word" requirement: Japanese has
        // no spaces for cosmic text's CJK line-breaking to respect.
        let hidden_text = format!(
            "{}: {hidden_count} {}",
            state.t("plain.selection.hidden_label"),
            state.t("plain.selection.hidden_suffix")
        );
        content = content.push(
            text(hidden_text)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
                .wrapping(Wrapping::None),
        );
    }
    if let Some(reason) = group_reason {
        content = content.push(
            text(reason)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }
    content = content.push(actions);
    for reason in contextual_reasons {
        content = content.push(
            text(reason)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        );
    }

    let bar = container(content.padding([6, 12])).width(Length::Fill);

    Some(bar.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::AppState;

    /// The bar is not rendered while `selection_mode` is false, so none of
    /// its five controls can be a Tab stop then either — true by
    /// construction now (RFC-075: nothing declares an order any more),
    /// checked here the same way every other "not drawn" case is: `view`
    /// returns `None`.
    #[test]
    fn view_is_none_when_not_in_selection_mode() {
        let state = AppState::new(AppConfig::default());
        assert!(!state.selection_mode);
        assert!(view(&state, WidthMode::Standard).is_none());
    }

    /// Selection mode with nothing selected still draws the bar — Exit
    /// selection is reachable either way; the four bulk actions become
    /// disabled controls (RFC-075 D4 b: not Tab stops at all, unlike
    /// knotra's own retired "still a Tab stop, activation is a no-op"
    /// rule) rather than being removed from the tree. The disabled-is-not-
    /// a-Tab-stop behaviour itself is `*_control`'s own, covered by
    /// `knotra-ui`'s primitive tests, not re-tested per view here.
    #[test]
    fn view_still_renders_with_nothing_selected() {
        let mut state = AppState::new(AppConfig::default());
        state.selection_mode = true;
        assert!(view(&state, WidthMode::Standard).is_some());
    }
}
