//! RFC-0013 — Bulk action modal views.
//! RFC-0021 Phase 3+4 — Plain-language, guided flows with per-step views.
//!
//! Five modals replacing the dedicated screens for Pull, Tag, Switch,
//! Resolve (conflict), and Changelog workflows. Each modal opens over the
//! dashboard and closes on completion or Esc.
//!
//! # Language policy
//! First-level wording uses goal-oriented plain language (see RFC-0021).
//! Technical terms (fetch, pull, tag, branch, conflict, stash, rollback …)
//! appear only inside the "Show details" sections — never as primary labels,
//! titles, or button text. All user-visible strings are routed through
//! `state.t()` so they are available in English and Japanese.
//!
//! Split from a single 1,337-ELOC `bulk_modals.rs` into this directory by
//! RFC-037 Stage 1 (D2) — a pure move, no behaviour or rendering change.
//! Stages 2-6 migrate each overlay onto RFC-034 primitives one at a time.

use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use knotra_ui::widget::{BUTTON_HEIGHT, FONT_BODY};
use knotra_vcs::ProjectId;

use crate::{message::Message, state::AppState};

mod changelog;
mod conflict;
mod context_switch;
mod freezer;
mod smart_pull;

pub use changelog::changelog_modal;
pub use conflict::resolve_panel;
pub use context_switch::switch_modal;
pub use freezer::tag_modal;
pub use smart_pull::pull_modal;

// `tests.rs` calls these two `pub(crate)` changelog helpers via the old
// `crate::view::bulk_modals::...` path (see `view.rs`'s `bulk_modals` alias)
// and R8 forbids editing `tests.rs` this stage, so both are re-exported here
// to keep that path resolving. `#[cfg(test)]`-gated for the same reason the
// alias itself is: nothing outside `tests.rs` uses this path. Not in
// Handoff 041 §1's own function table — see `changelog.rs`'s module doc for
// why they moved there anyway.
#[cfg(test)]
pub(crate) use changelog::{changelog_markdown_preview, changelog_result_counts};

// ---------------------------------------------------------------------------
// Modal shell
// ---------------------------------------------------------------------------

/// Shared shell with title bar used by all modals.
fn modal_shell<'a>(
    title: &'a str,
    close_msg: Option<Message>,
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    let close_btn = button(text("✕").size(FONT_BODY))
        .height(BUTTON_HEIGHT)
        .padding([0, 12])
        .on_press_maybe(close_msg);

    let header = row![
        text(title).size(FONT_BODY + 2.0),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center);

    container(
        column![header, iced::widget::rule::horizontal(1), inner]
            .spacing(16)
            .padding(24),
    )
    .width(Length::Fill)
    .max_width(580.0)
    .into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Used by exactly one overlay (`conflict::resolve_panel`), not "more than
/// one" as Handoff 041 §4 states — checked by grep, not assumed. Kept in
/// `mod.rs` per the handoff's explicit instruction anyway ("do not improve
/// anything here... note it and leave it"); flagged in the Stage 1 review
/// request for the architect to decide whether it moves to `conflict.rs` in
/// a later stage.
fn project_name_for(state: &AppState, id: &ProjectId) -> String {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| id.to_string())
}
