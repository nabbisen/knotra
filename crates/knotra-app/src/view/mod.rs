//! View functions for each screen.
//!
//! Each sub-module exports a single `view(state) -> Element<Message>` function.
//! Views are pure — they only read `AppState` and emit `Message`s.
pub mod activity_strip;
pub mod add_project_modal;
pub mod bulk_modals;
pub mod command_palette;
pub mod detail_panel;
pub mod nav_bar;
pub mod selection_bar;
pub mod shortcuts_overlay;
pub mod workspace_tabs;

pub mod changelog_view;
pub mod conflict_ops;
pub mod context_ops;
pub mod dashboard;
pub mod freezer;
pub mod history;
pub mod settings;
pub mod sync_center;

use crate::{message::Message, state::AppState};
use iced::Element;

/// Render the full application layout: sidebar + main content area.
pub fn app_view(state: &AppState) -> Element<'_, Message> {
    use crate::state::ActiveModal;
    #[allow(unused_imports)]
    use crate::state::Screen;
    use iced::Length;
    use iced::widget::{column, container, row, stack};

    // --- Workspace tabs (RFC-015) ---
    let tabs = workspace_tabs::view(state);

    // --- Navigation bar ---
    let nav = nav_bar::view(state);

    // --- Main screen content ---
    let screen_content: Element<'_, Message> = match state.screen {
        crate::state::Screen::Dashboard => dashboard::view(state),
        crate::state::Screen::SyncCenter => sync_center::view(state),
        crate::state::Screen::ContextOps => context_ops::view(state),
        crate::state::Screen::Freezer => freezer::view(state),
        crate::state::Screen::History => history::view(state),
        crate::state::Screen::Settings => settings::view(state),
        crate::state::Screen::ConflictResolution => conflict_ops::view(state),
        crate::state::Screen::Changelog => changelog_view::view(state),
    };

    // --- Selection bar (RFC-009) ---
    let sel_bar = selection_bar::view(state);

    // --- Activity strip (RFC-011) ---
    let activity = activity_strip::view(state);

    // --- Main column ---
    let mut main_col = column![tabs, nav, screen_content].height(Length::Fill);
    if let Some(bar) = sel_bar {
        main_col = main_col.push(bar);
    }
    if let Some(strip) = activity {
        main_col = main_col.push(strip);
    }

    // --- Detail panel (RFC-014) ---
    let base: Element<'_, Message> = if state.detail_panel.open_project_id.is_some() {
        if let Some(panel) = detail_panel::view(state) {
            row![main_col, panel].into()
        } else {
            main_col.into()
        }
    } else {
        main_col.into()
    };

    // --- Modal overlay (RFC-013) ---
    let modal_layer: Option<Element<'_, Message>> = match &state.active_modal {
        ActiveModal::None => None,
        ActiveModal::Pull => Some(
            container(bulk_modals::pull_modal(state))
                .center(Length::Fill)
                .into(),
        ),
        ActiveModal::Tag => Some(
            container(bulk_modals::tag_modal(state))
                .center(Length::Fill)
                .into(),
        ),
        ActiveModal::Switch => Some(
            container(bulk_modals::switch_modal(state))
                .center(Length::Fill)
                .into(),
        ),
        ActiveModal::Resolve(pid) => {
            // resolve panel is right-docked, not centred — wrap in a right-aligned container
            use iced::Alignment;
            use iced::widget::container;
            Some(
                container(bulk_modals::resolve_panel(state, pid))
                    .align_x(Alignment::End)
                    .height(Length::Fill)
                    .into(),
            )
        }
        ActiveModal::Changelog => Some(
            container(bulk_modals::changelog_modal(state))
                .center(Length::Fill)
                .into(),
        ),
    };

    // --- Command palette (RFC-012) ---
    let palette_layer: Option<Element<'_, Message>> = if state.palette.open {
        Some(
            container(command_palette::view(state))
                .center(Length::Fill)
                .into(),
        )
    } else {
        None
    };

    // --- Add project modal ---
    let add_project_layer: Option<Element<'_, Message>> =
        add_project_modal::view(state).map(|m| container(m).center(Length::Fill).into());

    // --- Keyboard cheat sheet (RFC-016) ---
    let shortcuts_layer: Option<Element<'_, Message>> = if state.keyboard.cheat_sheet_open {
        Some(
            container(shortcuts_overlay::view(state))
                .center(Length::Fill)
                .into(),
        )
    } else {
        None
    };

    // Stack base + any overlay.  iced::widget::stack renders last element on top.
    if modal_layer.is_some()
        || palette_layer.is_some()
        || shortcuts_layer.is_some()
        || add_project_layer.is_some()
    {
        let mut layers: Vec<Element<'_, Message>> = vec![base];
        if let Some(a) = add_project_layer {
            layers.push(a);
        }
        if let Some(m) = modal_layer {
            layers.push(m);
        }
        if let Some(p) = palette_layer {
            layers.push(p);
        }
        if let Some(s) = shortcuts_layer {
            layers.push(s);
        }
        stack(layers).into()
    } else {
        base
    }
}
