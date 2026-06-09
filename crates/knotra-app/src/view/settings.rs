//! Settings view (Phase 6 implementation).
use crate::{message::Message, state::AppState};
use iced::{
    Element,
    widget::{column, text},
};
pub fn view(state: &AppState) -> Element<Message> {
    column![
        text(state.t("nav.settings")).size(22),
        text("Settings — coming in Phase 6").size(14)
    ]
    .spacing(12)
    .padding(24)
    .into()
}
