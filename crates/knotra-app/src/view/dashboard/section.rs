//! Dashboard section rendering: the collapsible tier/group header and the
//! list of rows beneath it.

use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::{
    message::{DashboardMessage, Message},
    state::{
        AppState,
        dashboard::{DashboardSection, DashboardSectionKey, DashboardTier},
    },
};

use super::row::view_project_row;

pub(super) fn view_section<'a>(
    state: &'a AppState,
    section: DashboardSection<'a>,
) -> Element<'a, Message> {
    let mut elements = vec![section_header(
        state,
        section.key,
        section.entries.len(),
        section.collapsed,
    )];
    if !section.collapsed {
        elements.extend(
            section
                .entries
                .into_iter()
                .map(|entry| view_project_row(state, entry)),
        );
    }
    column(elements).spacing(3).into()
}

fn section_header<'a>(
    state: &'a AppState,
    key: DashboardSectionKey,
    entry_count: usize,
    collapsed: bool,
) -> Element<'a, Message> {
    let (label, toggle) = match key {
        DashboardSectionKey::Tier(tier) => {
            let label = match tier {
                DashboardTier::NeedsHelp => state.t("tier.needs_attention").to_owned(),
                DashboardTier::InProgress => state.t("tier.active").to_owned(),
                DashboardTier::AllSet => state.t("tier.clean").to_owned(),
            };
            let toggle = (tier != DashboardTier::NeedsHelp).then_some(tier);
            (label, toggle)
        }
        DashboardSectionKey::ProjectGroup(Some(group)) => (group, None),
        DashboardSectionKey::ProjectGroup(None) => (state.t("group.ungrouped").to_owned(), None),
        DashboardSectionKey::Flat => (state.t("dashboard.all_projects").to_owned(), None),
    };
    let label = format!(
        "{} ({}){}",
        label,
        entry_count,
        if toggle.is_some() {
            if collapsed { " +" } else { " -" }
        } else {
            ""
        }
    );
    if let Some(tier) = toggle {
        button(text(label).size(13))
            .on_press(Message::Dashboard(DashboardMessage::TierToggled(tier)))
            .width(Length::Fill)
            .into()
    } else {
        container(text(label).size(13))
            .width(Length::Fill)
            .padding([5, 8])
            .into()
    }
}
