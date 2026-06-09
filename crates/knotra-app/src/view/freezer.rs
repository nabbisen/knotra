//! Freezer view (Phase 5 implementation).
use crate::{message::Message, state::AppState};
use iced::{
    Element,
    widget::{column, text},
};
pub fn view(state: &AppState) -> Element<Message> {
    column![
        text(state.t("nav.freezer")).size(22),
        text("Freezer — coming in Phase 5").size(14)
    ]
    .spacing(12)
    .padding(24)
    .into()
}
