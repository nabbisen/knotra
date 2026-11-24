//! Dashboard section rendering: the collapsible tier/group header and the
//! list of rows beneath it.

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use knotra_ui::widget::{Tokens, centered_button_row, icon, list, strong};

use crate::{
    message::{DashboardMessage, Message},
    state::{
        AppState,
        dashboard::{DashboardSection, DashboardSectionKey, DashboardTier},
    },
    view::ids,
};

use super::row::view_project_row;
use super::width_mode::WidthMode;

pub(super) fn view_section<'a>(
    state: &'a AppState,
    section: DashboardSection<'a>,
    mode: WidthMode,
) -> Element<'a, Message> {
    let header = section_header(state, section.key, section.entries.len(), section.collapsed);
    if section.collapsed {
        return header;
    }
    // RFC-071 D8a: the rows share one border line between adjacent rows,
    // never above the first or below the last — the header sits outside
    // `list(...)` rather than as one of its items, so it keeps its own
    // larger gap (D8) instead of picking up a separator line of its own.
    let rows: Vec<Element<'a, Message>> = section
        .entries
        .into_iter()
        .map(|entry| view_project_row(state, entry, mode))
        .collect();
    // RFC-071 D8: `tokens.spacing.sm` between the header and its own rows
    // — smaller than `dashboard/mod.rs`'s inter-section `tokens.spacing.lg`,
    // so a header binds visually to the rows beneath it rather than
    // floating an equal distance from both neighbours.
    column![header, list(&state.theme.tokens, rows)]
        .spacing(state.theme.tokens.spacing.sm)
        .into()
}

/// RFC-071 D4: a group toggle is `text_secondary`, not `accent` — the
/// chevron beside it, not its colour, says "this opens and closes". Unlike
/// `name_link`'s style (`row.rs`), this has no hover colour change to
/// make: D4 names only the one colour for toggles, and this control is
/// never disabled, so there is no second branch to keep either.
fn group_toggle_style(
    tokens: &Tokens,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let text_secondary = snora::design::style::color::to_iced_color(tokens.palette.text_secondary);
    iced::widget::button::Style {
        background: None,
        text_color: text_secondary,
        ..iced::widget::button::Style::default()
    }
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
    let label = format!("{label} ({entry_count})");

    if let Some(tier) = toggle {
        let tokens = state.theme.tokens.clone();
        // Right reads as "click to open"; down reads as "already open,
        // contents below" — the disclosure idiom, and the direction
        // `chevron_down` already carries at the workspace switcher
        // (Handoff 026 §7.2).
        let chevron = if collapsed {
            icon::chevron_right()
        } else {
            icon::chevron_down()
        };
        // RFC-073 D8: at least `SMALL_BUTTON_HEIGHT`, here `BUTTON_HEIGHT`
        // — a group toggle is a frequent, full-width target, grouped with
        // chips/project names/Arrange under the same rule. RFC-075 stage 2
        // §3: on `snora::design::control`.
        snora::design::control(
            &tokens,
            centered_button_row(
                row![
                    strong(text(label).size(snora::design::style::text::title_size(&tokens))),
                    icon::icon_element(&chevron)
                ]
                .spacing(6),
            ),
        )
        .height(Length::Fixed(knotra_ui::widget::BUTTON_HEIGHT))
        .width(Length::Fill)
        .id(ids::section::toggle(tier))
        .on_press(Message::Dashboard(DashboardMessage::TierToggled(tier)))
        .style(move |_theme, status| group_toggle_style(&tokens, status))
        .into()
    } else {
        container(strong(text(label).size(
            snora::design::style::text::title_size(&state.theme.tokens),
        )))
        .width(Length::Fill)
        .padding([5, 8])
        .into()
    }
}
