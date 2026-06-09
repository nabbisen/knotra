//! Sync Center view (Phase 3 implementation).

use iced::{widget::{column, text}, Element, Length};
use crate::{message::Message, state::AppState};

pub fn view(state: &AppState) -> Element<Message> {
    column![
        text(state.t("nav.sync")).size(22),
        text("Sync Center — coming in Phase 3").size(14),
    ]
    .spacing(12)
    .padding(24)
    .into()
}
