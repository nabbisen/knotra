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
//! Stages 2-5 migrated each overlay onto RFC-034 primitives one at a time;
//! `modal_shell` (the pre-RFC-034 shared shell) was deleted at its last
//! caller (`smart_pull.rs`, Stage 5, R6). Stage 6 closed the RFC: the
//! `guided_button` sweep, `knotra-ui`'s `reasoned` primitive, and
//! `project_name_for`'s relocation to `conflict.rs` (its only caller),
//! leaving this file a near-pure declaration module.

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
