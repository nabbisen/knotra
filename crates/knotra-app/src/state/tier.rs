//! RFC-010 — Attention tier computation.
//!
//! `compute_tier` maps a `ProjectStatus` to one of three attention buckets
//! so the dashboard can group projects without requiring user-defined filters.

use crate::state::AttentionTier;
use endringer::model::status::ProjectStatus;

/// Causes for a project being in the NeedsAttention tier.
/// Used for inline recovery hints on cards.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AttentionCause {
    PathNotFound,
    Conflict,
    ConflictDetectionUnavailable,
    DetachedHead,
    OperationFailed,
    DirtyForLong,
}

/// Classify one project into an attention tier.
///
/// Priority order (first match wins):
/// 1. Path missing → NeedsAttention
/// 2. Active conflict → NeedsAttention
/// 3. Conflict detection unavailable → NeedsAttention
/// 4. Detached HEAD (context label starts with `(`) → NeedsAttention
/// 5. Persistent read error → NeedsAttention
/// 6. Uncommitted changes OR ahead of upstream → Active
/// 7. On a non-default branch name → Active
/// 8. Behind upstream → Active
/// 9. Otherwise → Clean
pub fn compute_tier(
    status: Option<&ProjectStatus>,
    path_exists: bool,
) -> (AttentionTier, Option<AttentionCause>) {
    if !path_exists {
        return (
            AttentionTier::NeedsAttention,
            Some(AttentionCause::PathNotFound),
        );
    }

    let Some(s) = status else {
        // Unknown state — leave as NeedsAttention until we have data.
        return (AttentionTier::NeedsAttention, None);
    };

    if s.conflict.has_conflict {
        return (
            AttentionTier::NeedsAttention,
            Some(AttentionCause::Conflict),
        );
    }
    if s.conflict.detection_unavailable {
        return (
            AttentionTier::NeedsAttention,
            Some(AttentionCause::ConflictDetectionUnavailable),
        );
    }
    if s.read_error.is_some() {
        return (
            AttentionTier::NeedsAttention,
            Some(AttentionCause::OperationFailed),
        );
    }
    let ctx = s.context.as_ref().map(|c| c.label.as_str()).unwrap_or("");
    if ctx.starts_with('(') {
        // jj "(no branch)" or git "(HEAD detached at …)"
        return (
            AttentionTier::NeedsAttention,
            Some(AttentionCause::DetachedHead),
        );
    }

    // Active conditions
    if s.working_tree.uncommitted_count > 0
        || s.working_tree.untracked_count > 0
        || s.remote.ahead > 0
        || s.remote.behind > 0
    {
        return (AttentionTier::Active, None);
    }
    // Non-default branch (heuristic: not "main", "master", or "trunk")
    if !matches!(ctx, "main" | "master" | "trunk" | "") {
        return (AttentionTier::Active, None);
    }

    (AttentionTier::Clean, None)
}
