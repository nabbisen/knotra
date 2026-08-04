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
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY, FONT_SMALL, Tokens, style};

use crate::{
    message::{ContextMessage, FreezerMessage, Message, SelectionMessage, SyncMessage},
    state::{
        AppState,
        focus::{FocusOrder, FocusTarget},
    },
    view::dashboard::WidthMode,
};

/// Stable keys for the selection bar's `FocusTarget`s (RFC-036/Handoff 031),
/// shared between [`focus_order`] and [`view`] — same discipline as
/// `toolbar.rs`'s own `focus_target` module.
mod focus_target {
    pub const FETCH: &str = "dashboard.selection_bar.fetch";
    pub const PULL: &str = "dashboard.selection_bar.pull";
    pub const TAG: &str = "dashboard.selection_bar.tag";
    pub const SWITCH: &str = "dashboard.selection_bar.switch";
    pub const EXIT: &str = "dashboard.selection_bar.exit";
}

fn is_focused(state: &AppState, key: &'static str) -> bool {
    state.dashboard_focus.as_ref() == Some(&FocusTarget::control(key))
}

/// Handoff 031 Finding 1: the selection bar's five controls (four bulk
/// actions plus Exit selection) had no `focus_order` entry at all — RFC-035
/// R22 requires every control the dashboard renders be keyboard-reachable,
/// and the RFC's Related files list names this file explicitly.
///
/// Not part of `dashboard::focus_order`'s tree (`selection_bar::view` is
/// composed alongside `dashboard::view` in `view.rs`, not inside it), so
/// this gets its own entry point, appended **after** the dashboard's targets
/// in `app/focus_ops.rs::shell_and_dashboard_focus_order` — the bar renders
/// beneath the content, matching visual order.
///
/// **Guarded on `state.selection_mode`** the same shape as the row checkbox
/// target and Clear filters: when the bar is not rendered, none of its
/// targets may be in the order (Stage 4's no-focus-black-hole invariant).
///
/// Duplicates the four buttons' enablement computation from [`view`] rather
/// than sharing it — the same choice `toolbar.rs`'s `standard_focus_order`/
/// `compact_focus_order`/`view_standard_toolbar`/`view_compact_toolbar` each
/// make for `select_message`, so `focus_order` and `view` stay independent
/// pure functions of `state`.
pub fn focus_order(state: &AppState) -> FocusOrder<Message> {
    if !state.selection_mode {
        return Vec::new();
    }

    let summary = state.selection_summary();
    let count = summary.selected_count;
    let busy = state.operation_interlock.is_busy();
    let can_act = count > 0 && !busy;

    let fetch_msg = (can_act && !summary.fetchable_ids.is_empty())
        .then_some(Message::Sync(SyncMessage::BulkFetchRequested));
    let pull_msg =
        (can_act && summary.has_upstream).then_some(Message::Sync(SyncMessage::BulkPullRequested));
    let tag_msg = can_act.then_some(Message::Freezer(FreezerMessage::BulkOpenRequested));
    let switch_msg =
        (count == 1 && !busy).then_some(Message::Context(ContextMessage::BulkOpenRequested));

    vec![
        (FocusTarget::control(focus_target::FETCH), fetch_msg),
        (FocusTarget::control(focus_target::PULL), pull_msg),
        (FocusTarget::control(focus_target::TAG), tag_msg),
        (FocusTarget::control(focus_target::SWITCH), switch_msg),
        (
            FocusTarget::control(focus_target::EXIT),
            Some(Message::Selection(SelectionMessage::ModeExited)),
        ),
    ]
}

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
    let fetch_btn = action_button(
        tokens,
        state.t("plain.check_for_updates"),
        fetch_msg,
        is_focused(state, focus_target::FETCH),
    );
    let pull_btn = action_button(
        tokens,
        state.t("plain.get_latest"),
        pull_msg,
        is_focused(state, focus_target::PULL),
    );
    let tag_btn = action_button(
        tokens,
        state.t("plain.save_release_point"),
        tag_msg,
        is_focused(state, focus_target::TAG),
    );
    let switch_btn = action_button(
        tokens,
        state.t("plain.change_work_area"),
        switch_msg,
        is_focused(state, focus_target::SWITCH),
    );

    let clear_btn = exit_button(
        tokens,
        state.t("plain.exit_selection"),
        is_focused(state, focus_target::EXIT),
    );

    let command_row = row![
        text(label).size(13),
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

    let mut content = column![command_row].spacing(6);
    if let Some(reason) = group_reason {
        content = content.push(text(reason).size(FONT_SMALL));
    }
    content = content.push(actions);
    for reason in contextual_reasons {
        content = content.push(text(reason).size(FONT_SMALL));
    }

    let bar = container(content.padding([6, 12])).width(Length::Fill);

    Some(bar.into())
}

/// A selection-bar action button, without `guided_button`'s
/// reason-beneath-itself composition — R13/R14 already render the reason
/// (shared or action-specific) in a slot outside the button, so a copy
/// here would just reintroduce the duplication this migration removes.
///
/// Handoff 031 §2.3: checked before building whether the reason-composition
/// wrapper `select_mode_button`/`row_action_button` use was still needed
/// here. **It is not** — Stage 5 commit 3 already moved every reason this
/// button might have shown into `group_reason`/`contextual_reasons`, so
/// `action_button` never had a `reason` parameter to begin with. Only the
/// ring needed adding: `style::secondary` + `style::with_focus_ring`, the
/// same base `select_mode_button`/row.rs's Conflict action use for a
/// similarly-weighted mutating action.
fn action_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<Message>,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(text(label).size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 18])
        .on_press_maybe(on_press)
        .style(move |_theme, status| {
            style::with_focus_ring(&t, is_focused, style::secondary(&t, status))
        })
        .into()
}

/// Exit selection: `ghost`-styled, matching `row.rs`'s "Show details" —
/// a non-mutating, lower-weight action, not a fourth `secondary` button.
fn exit_button<'a>(tokens: &Tokens, label: &'a str, is_focused: bool) -> Element<'a, Message> {
    let t = tokens.clone();
    button(text(label).size(13))
        .height(BUTTON_HEIGHT)
        .on_press(Message::Selection(SelectionMessage::ModeExited))
        .style(move |_theme, status| {
            style::with_focus_ring(&t, is_focused, style::ghost(&t, status))
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::AppState;

    /// Handoff 031's no-focus-black-hole invariant (Stage 4's own rule,
    /// applied here): the bar is not rendered while `selection_mode` is
    /// false, so none of its five controls may be a Tab stop then either.
    #[test]
    fn focus_order_is_empty_when_not_in_selection_mode() {
        let state = AppState::new(AppConfig::default());
        assert!(!state.selection_mode);
        assert!(focus_order(&state).is_empty());
    }

    /// Selection mode with nothing selected: `can_act` is false for all
    /// four bulk actions, but Exit selection stays a live target — the same
    /// "still a Tab stop, activation is a no-op" shape `focus::FocusOrder`'s
    /// own doc comment describes for a disabled control.
    #[test]
    fn focus_order_has_five_targets_with_only_exit_active_when_nothing_selected() {
        let mut state = AppState::new(AppConfig::default());
        state.selection_mode = true;

        let order = focus_order(&state);
        assert_eq!(order.len(), 5);

        let live: Vec<_> = order
            .iter()
            .filter(|(_, message)| message.is_some())
            .collect();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].0, FocusTarget::control(focus_target::EXIT));
    }
}
