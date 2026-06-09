//! Context Operations view (Phase 4 implementation).
use crate::{message::Message, state::AppState};
use iced::{
    Element,
    widget::{column, text},
};
pub fn view(state: &AppState) -> Element<Message> {
    column![
        text(state.t("nav.context")).size(22),
        text("Context Ops — coming in Phase 4").size(14)
    ]
    .spacing(12)
    .padding(24)
    .into()
}
