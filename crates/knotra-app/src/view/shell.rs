//! RFC-034 R12/R13 — the persistent application shell.
//!
//! Replaces `view/workspace_tabs.rs`: a workspace switcher (name + attention
//! count, whose menu owns switch/create/rename/delete), Dashboard/History
//! navigation with an unambiguous active state, and a right cluster
//! (operation status, refresh, command palette, settings). Rendered into
//! `snora::AppLayout::header`; the switcher's dropdown renders into
//! `AppLayout::header_menu`, a separate engine layer positioned below this
//! bar (see `switcher_menu` below).

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
use knotra_ui::widget::{self, Tokens, icon, icon_button_maybe};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{
        AppState, Screen,
        focus::{FocusOrder, FocusTarget},
    },
};

/// Stable keys for the shell's `FocusTarget`s (RFC-036), shared between
/// [`focus_order`] (Tab/Shift-Tab + activation) and `view` (which control
/// currently draws the ring). Kept as one list so the two cannot drift.
mod focus_target {
    pub const WORKSPACE_SWITCHER: &str = "shell.workspace_switcher";
    pub const NAV_DASHBOARD: &str = "shell.nav.dashboard";
    pub const NAV_HISTORY: &str = "shell.nav.history";
    pub const REFRESH: &str = "shell.refresh";
    pub const PALETTE: &str = "shell.palette";
    pub const SETTINGS: &str = "shell.settings";
}

/// The shell's Tab/Shift-Tab focus order (RFC-036 R1/R2), each target paired
/// with the `Message` a pointer click on it would dispatch right now —
/// `None` where a click currently does nothing (e.g. the already-active nav
/// destination), so keyboard activation cannot diverge from pointer
/// activation (R3).
pub fn focus_order(state: &AppState) -> FocusOrder<Message> {
    let dashboard_active = matches!(state.screen, Screen::Dashboard);
    let history_active = matches!(state.screen, Screen::History);

    vec![
        (
            FocusTarget::control(focus_target::WORKSPACE_SWITCHER),
            Some(Message::Workspace(WorkspaceMessage::SwitcherToggled)),
        ),
        (
            FocusTarget::control(focus_target::NAV_DASHBOARD),
            (!dashboard_active).then_some(Message::Navigate(Screen::Dashboard)),
        ),
        (
            FocusTarget::control(focus_target::NAV_HISTORY),
            (!history_active).then_some(Message::Navigate(Screen::History)),
        ),
        (
            FocusTarget::control(focus_target::REFRESH),
            (!state.is_refreshing)
                .then_some(Message::Workspace(WorkspaceMessage::RefreshRequested)),
        ),
        (
            FocusTarget::control(focus_target::PALETTE),
            Some(Message::Palette(crate::message::PaletteMessage::Opened)),
        ),
        (
            FocusTarget::control(focus_target::SETTINGS),
            Some(Message::Navigate(Screen::Settings)),
        ),
    ]
}

/// Whether the shell control keyed `key` currently draws the RFC-036 focus
/// ring — plain equality against `state.dashboard_focus`, not
/// `focus::resolve`'s stale-target fallback: rendering shows a ring only
/// where knotra-focus genuinely and currently sits, never a guessed
/// substitute (`resolve`'s fallback is for Tab/activation, not display).
fn is_focused(state: &AppState, key: &'static str) -> bool {
    state.dashboard_focus.as_ref() == Some(&FocusTarget::control(key))
}

/// Height of the persistent shell bar (RFC-033 D7: 48-52px). Also used to
/// position the switcher dropdown (`header_menu`) directly beneath it, since
/// `AppLayout` renders `header_menu` as its own full-window stack layer with
/// no positioning relative to the header — the content must place itself.
const SHELL_HEIGHT: f32 = 52.0;

/// Width of the switcher dropdown.
const SWITCHER_WIDTH: f32 = 260.0;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    let attention = state
        .dashboard_display()
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .filter(|entry| entry.tier == crate::state::dashboard::DashboardTier::NeedsHelp)
        .count();

    let workspace_name = state
        .workspace
        .as_ref()
        .map(|ws| ws.name.as_str())
        .unwrap_or_else(|| state.t("dashboard.no_workspace"));

    let switcher_label = if attention > 0 {
        format!("{workspace_name} ({attention})")
    } else {
        workspace_name.to_owned()
    };

    let switcher_trigger = button(
        row![
            text(switcher_label).size(snora::design::style::text::body_size(tokens)),
            widget::icon::icon_element(&icon::chevron_down()),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(Message::Workspace(WorkspaceMessage::SwitcherToggled))
    .style({
        let t = tokens.clone();
        let focused = is_focused(state, focus_target::WORKSPACE_SWITCHER);
        move |_theme, status| {
            widget::style::with_focus_ring(&t, focused, widget::style::ghost(&t, status))
        }
    });

    let dashboard_active = matches!(state.screen, Screen::Dashboard);
    let history_active = matches!(state.screen, Screen::History);

    let nav_button = |label: &str, active: bool, target: Screen, tokens: &Tokens, focused: bool| {
        let t = tokens.clone();
        button(text(label.to_owned()).size(snora::design::style::text::body_size(tokens)))
            .on_press_maybe((!active).then_some(Message::Navigate(target)))
            .style(move |_theme, status| {
                // R12: the current destination must be the *most* salient
                // item, not the least — see `current_or`'s own doc comment
                // for why a fixed status is fed in (RFC-033 D4; review 066).
                widget::current_or(active, &t, status, focused)
            })
    };

    let status_text: Element<'_, Message> = if state.is_refreshing {
        text(state.t("plain.status.checking"))
            .size(snora::design::style::text::body_small_size(tokens))
            .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    let refresh_button = icon_button_maybe(
        tokens,
        &icon::refresh(),
        state.t("plain.check_now"),
        (!state.is_refreshing).then_some(Message::Workspace(WorkspaceMessage::RefreshRequested)),
        is_focused(state, focus_target::REFRESH),
    );

    let palette_button = icon_button_maybe(
        tokens,
        &icon::command_palette(),
        state.t("palette.title"),
        Some(Message::Palette(crate::message::PaletteMessage::Opened)),
        is_focused(state, focus_target::PALETTE),
    );

    let settings_button = icon_button_maybe(
        tokens,
        &icon::settings(),
        state.t("nav.settings"),
        Some(Message::Navigate(Screen::Settings)),
        is_focused(state, focus_target::SETTINGS),
    );

    let bar = row![
        switcher_trigger,
        nav_button(
            state.t("nav.dashboard"),
            dashboard_active,
            Screen::Dashboard,
            tokens,
            is_focused(state, focus_target::NAV_DASHBOARD)
        ),
        nav_button(
            state.t("nav.history"),
            history_active,
            Screen::History,
            tokens,
            is_focused(state, focus_target::NAV_HISTORY)
        ),
        Space::new().width(Length::Fill),
        status_text,
        refresh_button,
        palette_button,
        settings_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([0, 12]);

    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(SHELL_HEIGHT))
        .into()
}

/// The switcher's dropdown content, rendered through `AppLayout::header_menu`
/// when open. `None` when closed — callers must not set the layout slot at
/// all in that case (an empty `Some` would still opt into the click-outside
/// backdrop per `AppLayout` semantics).
pub fn switcher_menu(state: &AppState) -> Option<Element<'_, Message>> {
    if !state.workspace_mgr.switcher_open {
        return None;
    }

    let tokens = &state.theme.tokens;
    let active_id = state
        .all_workspaces
        .get(state.active_workspace_idx)
        .map(|ws| ws.id.clone());

    let mut items = column![].spacing(2);
    for ws in &state.all_workspaces {
        let is_active = Some(&ws.id) == active_id.as_ref();
        let t = tokens.clone();
        items = items.push(
            button(text(ws.name.clone()).size(snora::design::style::text::body_size(tokens)))
                .width(Length::Fill)
                .on_press_maybe((!is_active).then_some(Message::Workspace(
                    WorkspaceMessage::WorkspaceSwitched(ws.id.clone()),
                )))
                .style(move |_theme, status| widget::style::ghost(&t, status)),
        );
    }

    let can_delete = state.all_workspaces.len() > 1;
    let menu_action = |label: &str, msg: Message, tokens: &Tokens| {
        let t = tokens.clone();
        button(text(label.to_owned()).size(snora::design::style::text::body_size(tokens)))
            .width(Length::Fill)
            .on_press(msg)
            .style(move |_theme, status| widget::style::ghost(&t, status))
    };

    let content = column![
        items,
        menu_action(
            state.t("plain.add_workspace"),
            Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
            tokens
        ),
        menu_action(
            state.t("workspace.rename.short"),
            Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened),
            tokens
        ),
        danger_menu_action(
            state.t("workspace.delete.short"),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceRequested
            )),
            tokens,
        ),
    ]
    .spacing(2)
    .width(Length::Fixed(SWITCHER_WIDTH));

    let surface = widget::overlay::raised_card(tokens, content);

    Some(
        container(surface)
            .padding(iced::Padding {
                top: SHELL_HEIGHT + 4.0,
                left: 8.0,
                ..iced::Padding::ZERO
            })
            .into(),
    )
}

fn danger_menu_action<'a>(
    label: &'a str,
    on_press: Option<Message>,
    tokens: &Tokens,
) -> Element<'a, Message> {
    let t = tokens.clone();
    button(text(label).size(snora::design::style::text::body_size(tokens)))
        .width(Length::Fill)
        .on_press_maybe(on_press)
        .style(move |_theme, status| widget::style::danger(&t, status))
        .into()
}

/// Shared page-header pattern: title on the left, contextual actions on the
/// right. RFC-034 R14 migrates exactly one caller (the dashboard) as
/// validation; other screens keep their own inline header for now.
///
/// RFC-056 Stage 2: the title is `heading` (24) — snora's own description of
/// that role, "page or section heading," is exactly what this is.
pub fn page_header<'a>(
    title: impl Into<String>,
    actions: impl Into<Element<'a, Message>>,
    tokens: &Tokens,
) -> Element<'a, Message> {
    row![
        text(title.into()).size(snora::design::style::text::heading_size(tokens)),
        Space::new().width(Length::Fill),
        actions.into(),
    ]
    .align_y(Alignment::Center)
    .padding([8, 14])
    .into()
}
