//! History view (Phase 6 implementation).
use iced::{widget::{column, text}, Element};
use crate::{message::Message, state::AppState};
pub fn view(state: &AppState) -> Element<Message> {
    column![text(state.t("nav.history")).size(22), text("History — coming in Phase 6").size(14)]
        .spacing(12).padding(24).into()
}
