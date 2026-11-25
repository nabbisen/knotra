//! RFC-034 R12/R13 — the persistent application shell.
//!
//! Replaces `view/workspace_tabs.rs`: a workspace switcher (name + attention
//! count, whose menu owns switch/create/rename/delete), Dashboard/History
//! navigation with an unambiguous active state, and a right cluster
//! (operation status, refresh, command palette, settings). Rendered into
//! `snora::AppLayout::header`; the switcher's dropdown renders into
//! `AppLayout::header_menu`, a separate engine layer positioned below this
//! bar (see `switcher_menu` below).
//!
//! **RFC-075 stage 2 §3:** every control in the bar itself (the switcher's
//! own trigger, Dashboard/History, Check now, the command palette, and
//! Settings) is now a `snora::design::control` — a real Tab stop with its
//! own ring, Enter/Space, and `.id(...)` (`view::ids::shell`). **The
//! switcher's dropdown stays knotra's own model until stage 3** (the
//! handoff's own carve-out): [`switcher_menu_layer_focus`] and
//! [`switcher_menu`] are unchanged below.

use iced::{
    Alignment, Element, Length,
    widget::{Space, column, container, row, text, tooltip},
};
use knotra_ui::widget::{self, BUTTON_HEIGHT, Tokens, centered_button_row, icon};

use crate::{
    message::{Message, WorkspaceMessage},
    state::{AppState, Screen},
    view::ids,
};

/// RFC-075 stage 3 §4: the workspace menu's own entry — the first
/// workspace row that is not the active one (`WorkspaceSwitched` starts a
/// change there, so the active row disables itself and is never the
/// entry); "Add workspace" if every workspace is somehow the active one
/// (a single-workspace menu).
pub fn switcher_menu_entry_id(state: &AppState) -> Option<iced::widget::Id> {
    if !state.workspace_mgr.switcher_open {
        return None;
    }
    let active_id = state
        .all_workspaces
        .get(state.active_workspace_idx)
        .map(|ws| ws.id);
    Some(
        state
            .all_workspaces
            .iter()
            .find(|ws| Some(&ws.id) != active_id.as_ref())
            .map(|ws| ids::switcher::row(&ws.id))
            .unwrap_or_else(ids::switcher::add_workspace),
    )
}

/// RFC-075 stage 3 §4: every enabled row's own id, in reading order — the
/// list ↓/↑ moves real focus among (`knotra_ui::widget::keyboard::
/// focus_among`), skipping the active workspace row (disabled) and
/// "Delete" when it is the workspace's only one.
pub fn switcher_menu_ids(state: &AppState) -> Vec<iced::widget::Id> {
    let active_id = state
        .all_workspaces
        .get(state.active_workspace_idx)
        .map(|ws| ws.id);
    let mut ids: Vec<iced::widget::Id> = state
        .all_workspaces
        .iter()
        .filter(|ws| Some(&ws.id) != active_id.as_ref())
        .map(|ws| ids::switcher::row(&ws.id))
        .collect();
    ids.push(ids::switcher::add_workspace());
    ids.push(ids::switcher::rename());
    if state.all_workspaces.len() > 1 {
        ids.push(ids::switcher::delete());
    }
    ids
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

    let workspace_name = state
        .workspace
        .as_ref()
        .map(|ws| ws.name.as_str())
        .unwrap_or_else(|| state.t("dashboard.no_workspace"));

    // RFC-073 D4: with one workspace, its name is plain text, not a menu;
    // the switcher (switch/create/rename/delete) appears only with two or
    // more.
    let workspace_header: Element<'_, Message> = if state.all_workspaces.len() > 1 {
        let t = tokens.clone();
        // RFC-073 D8: `BUTTON_HEIGHT` — a header control.
        snora::design::control(
            tokens,
            centered_button_row(
                row![
                    text(workspace_name.to_owned())
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                    widget::icon::icon_element(&icon::chevron_down()),
                ]
                .spacing(6),
            ),
        )
        .height(Length::Fixed(BUTTON_HEIGHT))
        .id(ids::shell::workspace_switcher())
        .padding([0.0, 12.0])
        .on_press(Message::Workspace(WorkspaceMessage::SwitcherToggled))
        .style(move |_theme, status| widget::style::ghost(&t, status))
        .into()
    } else {
        text(workspace_name.to_owned())
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into()
    };

    let dashboard_active = matches!(state.screen, Screen::Dashboard);
    let history_active = matches!(state.screen, Screen::History);

    // RFC-012 R12: the current destination is the *most* salient item —
    // `secondary` (Active) when it is, `ghost` otherwise. `Control`'s own
    // ring draws the keyboard-focus indicator now; nothing here keys off
    // it directly any more (RFC-075 D1).
    let nav_control = |label: &str, active: bool, target: Screen, id: iced::widget::Id| {
        let t = tokens.clone();
        snora::design::control(
            tokens,
            centered_button_row(row![
                text(label.to_owned())
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
            ]),
        )
        .height(Length::Fixed(BUTTON_HEIGHT))
        .id(id)
        .padding([0.0, 12.0])
        .on_press_maybe((!active).then_some(Message::Navigate(target)))
        .style(move |_theme, status| {
            if active {
                widget::style::secondary(&t, iced::widget::button::Status::Active)
            } else {
                widget::style::ghost(&t, status)
            }
        })
    };

    let status_text: Element<'_, Message> = if state.is_refreshing {
        text(state.t("plain.status.checking"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into()
    } else {
        Space::new().width(Length::Shrink).into()
    };

    // RFC-071 D10: Refresh, the command palette and Settings show their own
    // name beside the icon in the standard and wide bands; the compact band
    // keeps the icon-only control with today's tooltip, unchanged.
    let compact = matches!(state.width_mode, crate::view::dashboard::WidthMode::Compact);
    let refresh_button = if compact {
        icon_control_with_tooltip(
            tokens,
            &icon::refresh(),
            state.t("plain.check_now"),
            (!state.is_refreshing)
                .then_some(Message::Workspace(WorkspaceMessage::RefreshRequested)),
            ids::shell::refresh(),
        )
    } else {
        header_icon_control(
            tokens,
            &icon::refresh(),
            state.t("plain.check_now"),
            (!state.is_refreshing)
                .then_some(Message::Workspace(WorkspaceMessage::RefreshRequested)),
            ids::shell::refresh(),
        )
    };

    let palette_button = if compact {
        icon_control_with_tooltip(
            tokens,
            &icon::command_palette(),
            state.t("palette.title"),
            Some(Message::Palette(crate::message::PaletteMessage::Opened)),
            ids::shell::palette(),
        )
    } else {
        header_icon_control(
            tokens,
            &icon::command_palette(),
            state.t("palette.title"),
            Some(Message::Palette(crate::message::PaletteMessage::Opened)),
            ids::shell::palette(),
        )
    };

    let settings_button = if compact {
        icon_control_with_tooltip(
            tokens,
            &icon::settings(),
            state.t("nav.settings"),
            Some(Message::Navigate(Screen::Settings)),
            ids::shell::settings(),
        )
    } else {
        header_icon_control(
            tokens,
            &icon::settings(),
            state.t("nav.settings"),
            Some(Message::Navigate(Screen::Settings)),
            ids::shell::settings(),
        )
    };

    let bar = row![
        workspace_header,
        nav_control(
            state.t("nav.dashboard"),
            dashboard_active,
            Screen::Dashboard,
            ids::shell::nav_dashboard(),
        ),
        nav_control(
            state.t("nav.history"),
            history_active,
            Screen::History,
            ids::shell::nav_history(),
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

/// The compact band's icon-only header control, with today's tooltip —
/// [`knotra_ui::widget::icon_button_maybe`]'s own twin on `snora::design::
/// control`.
fn icon_control_with_tooltip<'a>(
    tokens: &Tokens,
    icon: &snora::Icon,
    accessible_label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let btn = snora::design::control(
        tokens,
        centered_button_row(row![widget::icon::icon_element(icon)]),
    )
    .height(Length::Fixed(BUTTON_HEIGHT))
    .width(Length::Fixed(BUTTON_HEIGHT))
    .id(id)
    .on_press_maybe(on_press)
    .style(move |_theme, status| widget::style::ghost(&t, status));

    tooltip(
        btn,
        widget::overlay::raised_card(
            tokens,
            // RFC-071 Amendment A3 D14: a tooltip is a transient hint, not
            // primary content read to act — `body_small`, today's size
            // (`knotra_ui::widget::icon_button_maybe`'s own treatment,
            // this control's own retired twin).
            text(accessible_label.into())
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        ),
        tooltip::Position::Bottom,
    )
    .into()
}

/// The standard/wide bands' icon-and-label header control —
/// [`knotra_ui::widget::header_control`]'s own twin on `snora::design::
/// control`.
fn header_icon_control<'a>(
    tokens: &Tokens,
    icon: &snora::Icon,
    label: impl Into<String>,
    on_press: Option<Message>,
    id: iced::widget::Id,
) -> Element<'a, Message> {
    let t = tokens.clone();
    snora::design::control(
        tokens,
        centered_button_row(
            row![
                widget::icon::icon_element_sized(icon, 16.0),
                text(label.into())
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(6),
        ),
    )
    .height(Length::Fixed(BUTTON_HEIGHT))
    .id(id)
    .padding([0.0, 12.0])
    .on_press_maybe(on_press)
    .style(move |_theme, status| widget::style::ghost(&t, status))
    .into()
}

/// The switcher's dropdown content, rendered through `AppLayout::header_menu`
/// when open. `None` when closed — callers must not set the layout slot at
/// all in that case (an empty `Some` would still opt into the click-outside
/// backdrop per `AppLayout` semantics).
///
/// RFC-075 stage 3 §4: "a list on its own" — every row here is now a real
/// snora control and Tab stop (unlike the command palette's/Change work
/// area's own highlight-only lists); ↓/↑ move real focus between them
/// (`switcher_menu_ids`, driven from `app.rs`), wrapping, skipping
/// disabled rows.
pub fn switcher_menu(state: &AppState) -> Option<Element<'_, Message>> {
    if !state.workspace_mgr.switcher_open {
        return None;
    }

    let tokens = &state.theme.tokens;
    let active_id = state
        .all_workspaces
        .get(state.active_workspace_idx)
        .map(|ws| ws.id);

    let mut items = column![].spacing(2);
    for ws in &state.all_workspaces {
        let is_active = Some(&ws.id) == active_id.as_ref();
        // RFC-073 D8: `SMALL_BUTTON_HEIGHT` — a menu row.
        items = items.push(widget::ghost_control_maybe(
            tokens,
            ws.name.clone(),
            (!is_active).then_some(Message::Workspace(WorkspaceMessage::WorkspaceSwitched(
                ws.id,
            ))),
            ids::switcher::row(&ws.id),
        ));
    }

    let can_delete = state.all_workspaces.len() > 1;
    let content = column![
        items,
        widget::ghost_control(
            tokens,
            state.t("plain.add_workspace"),
            Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
            ids::switcher::add_workspace(),
        ),
        widget::ghost_control(
            tokens,
            state.t("workspace.rename.short"),
            Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened),
            ids::switcher::rename(),
        ),
        widget::danger_control_maybe(
            tokens,
            state.t("workspace.delete.short"),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceRequested
            )),
            ids::switcher::delete(),
        ),
    ]
    .spacing(2)
    .width(Length::Fixed(SWITCHER_WIDTH));

    let surface = widget::overlay::raised_card(tokens, content);

    Some(knotra_ui::widget::focus_by_id_window(
        container(surface).padding(iced::Padding {
            top: SHELL_HEIGHT + 4.0,
            left: 8.0,
            ..iced::Padding::ZERO
        }),
        &[],
    ))
}
