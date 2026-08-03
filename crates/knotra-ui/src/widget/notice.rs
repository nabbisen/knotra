//! Toned status-banner primitive (RFC-032), deferred from Stage 1
//! (`098`) until Stage 5 gave it a real consumer.
//!
//! Checked first, per Handoff 030 §5, whether this could be a thin
//! pass-through over `snora::design::notice::Notice` the way `chip`
//! could not be (`094`: the ring needed a `Style` `snora`'s function did
//! not expose). **It can** — `Notice`'s builder (tone/title/body/action/
//! dismiss) has no focus-ring requirement at all (RFC-032's own doc
//! comment: "Focus rings follow the iced 0.14 limitation... documented,
//! not a regression"), so nothing here needed KnotraTheme-level access
//! the way `chip`/`checkbox`/`select` did. This wrapper exists only so
//! call sites go through `knotra_ui::widget` rather than `snora::design`
//! directly (RFC-034 R2/RFC-035 R19), and so `Tone` is nameable from
//! `knotra-app` without a direct `snora` import.
//!
//! **One real constraint did surface**: `Notice`'s own `.action()` is a
//! single slot, and `.dismiss()` is a fixed `"×"` glyph, not a second
//! labelled action. RFC-032 R7's load-error notice needs *two*
//! independent actions — Retry, and a Show/Hide-details toggle — so this
//! wrapper renders only the tone/title/body/one-action part; the
//! details-disclosure affordance and the conditionally-shown raw text are
//! composed by the caller *around* this `Element`, not through it. That
//! is not a workaround for a missing capability (unlike `chip`'s ring) —
//! a details-reveal toggle was never part of what `Notice` models, and
//! composing a second control beside a primitive's output is the same
//! shape `row.rs`'s row actions and `toolbar.rs`'s disclosures already
//! use throughout this RFC.

use snora::design::{Tokens, Tone};

use super::layout::Element;

/// The notice's single optional action button (Retry, in R7's case).
pub struct NoticeAction<Message> {
    pub label: String,
    pub on_press: Message,
}

/// A toned status banner: optional title, body, and at most one action.
#[must_use]
pub fn notice<'a, Message: Clone + 'a>(
    tokens: &'a Tokens,
    tone: Tone,
    title: Option<String>,
    body: impl Into<String>,
    action: Option<NoticeAction<Message>>,
) -> Element<'a, Message> {
    let mut builder = snora::design::notice::Notice::new(tokens, tone, body);
    if let Some(title) = title {
        builder = builder.title(title);
    }
    if let Some(action) = action {
        builder = builder.action(action.label, action.on_press);
    }
    builder.render()
}
