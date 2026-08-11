//! A single record in a collapsible list (RFC-038 D3/R6).
//!
//! `history.rs`'s per-operation rows and RFC-039's (not yet built)
//! per-project rows share one shape: a summary that always renders, and
//! detail content that renders only when the record is expanded. This
//! module extracts exactly that composition — nothing else.
//!
//! Deliberately not extracted, because RFC-039 does not exist yet to say it
//! needs them: the summary/detail content itself, the expand/collapse
//! state, and any disclosure control (a button, a chevron, wherever it
//! lives). Those stay the caller's, in `view/`. Building slots for them
//! here would be designing a primitive for a consumer nobody can see yet —
//! the mistake RFC-034 R7 made for fields (RFC-037 D6 had to fix it two
//! RFCs later).

use super::layout::{Element, Length};

/// A collapsible record row: `summary` renders unconditionally; `detail`,
/// when `Some`, renders beneath it. Pass `None` for a collapsed record
/// rather than an empty element — the caller's own "is this expanded"
/// check decides whether to build `detail` at all, so nothing is
/// constructed for a record nobody is looking at.
#[must_use]
pub fn record_row<'a, Message: 'a>(
    summary: Element<'a, Message>,
    detail: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    use iced::widget::{column, container};

    let mut col = column![summary].spacing(4);
    if let Some(detail) = detail {
        col = col.push(detail);
    }

    container(col).width(Length::Fill).padding([8, 12]).into()
}
