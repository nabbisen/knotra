# RFC-0021 — Plain-language layer for non-technical users

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| Status         | Implemented (v0.20.0) — Phases 1–4                                       |
| Priority       | Medium                                                                |
| Effort         | Medium (phased)                                                       |
| Target version | v0.19.0 (Phase 1); v0.20.0 (Phases 2–4); Phases 5–6 TBD                                   |
| Related        | UI/UX handoff; external non-technical UX review                       |

## Summary

An external UX review assessed the v0.18 UI as strong for developers but
unsafe as first-level language for non-technical users: it surfaces terms
like *Fetch*, *Pull*, *Tag*, *Conflict*, *Branch*, *Uncommitted*. This RFC
adopts the review's core recommendation — replace first-level labels with
goal-oriented plain language and keep expert terms behind "Show details" —
implemented in a way that fits knotra's existing architecture.

This matches knotra's own novice→advanced maturity model and "less is more"
principle: the default surface is calm and plain; technical depth is
available on demand.

## What was implemented (Phase 1, v0.19.0)

### Plain-language wording, via the existing i18n catalog

The dashboard tiers, project-card status labels, and selection-bar action
buttons now read in plain language:

| Internal | Was (first-level) | Now (first-level) |
|---|---|---|
| `AttentionTier::NeedsAttention` | "Needs attention" | **Needs help** |
| `AttentionTier::Active` | "Active" | **In progress** |
| `AttentionTier::Clean` | "Clean" | **All set** |
| `StatusColor::Conflict` | "Conflict" | **Needs your choice** |
| `StatusColor::Dirty` | "Uncommitted" | **Unsaved work** |
| `StatusColor::Behind` | "Behind" | **Updates available** |
| `StatusColor::Ahead` | "Ahead" | **Unshared changes** |
| `StatusColor::Unknown` | "Unknown" | **Not sure yet** |
| Selection: Fetch | "Fetch" | **Check for updates** |
| Selection: Pull | "Pull…" | **Get latest safely** |
| Selection: Tag | "Tag…" | **Save release point** |
| Selection: Switch | "Switch…" | **Change work area** |

The technical terms remain in the catalog under their original keys
(`status.*`, `card.*`, `action.*`) for use in the project detail panel and
operation history under "Show details".

### Accessibility tokens

Per the review, clickable controls now meet a 44px minimum touch target
(`widget::BUTTON_HEIGHT`), applied first to the selection-bar actions. Body
text token is 15px (`widget::FONT_BODY`). These were added to the existing
`knotra-ui::widget` module.

### Regression guard

Two unit tests in `knotra-ui::i18n`:
- `first_level_wording_has_no_developer_jargon` — fails if any `plain.*` or
  `tier.*` value contains a forbidden developer term (fetch, pull, tag,
  branch, conflict, uncommitted, detached, upstream, rollback, execute, cli,
  stash, merge, commit, repo).
- `plain_keys_are_localised_in_both_catalogs` — fails if a first-level key
  exists in English but not Japanese.

## Two adaptations from the review's plan

The review's plan is followed in substance. Two of its proposed mechanics
were adapted to avoid forking knotra's established architecture; the
outcomes are identical.

1. **Wording lives in the i18n catalog, not a new `plain_text.rs` enum.**
   The plan proposed an English-only `UserText` enum. knotra already routes
   every user-visible string through a keyed i18n catalog with English and
   Japanese, and the project's hard rule is that all strings are localised in
   both. A parallel English-only system would fragment that and leave the
   plain layer untranslated. New wording was added as `plain.*` / `tier.*`
   keys in both catalogs instead. (The plan's own §5 notes the catalogue
   "makes Japanese localization safer" — using the existing catalogue
   realises that directly.)

2. **Existing `StatusColor` / `KnotraTheme` retained, not renamed to
   `SafeColor` / `tokens.rs`.** The plan's proposed palette has hex values
   identical to the existing `theme.rs`. Renaming working, tested tokens
   would be churn with no user-visible benefit — the kind of complexity
   knotra avoids. The substantive token *changes* the review argues for
   (44px targets, 15px body) were adopted; the type names were not churned.

## Phases not yet implemented

The review proposes further phases. These are deferred and tracked here as
future scope:

- **Phase 2 — safe components.** `SafeButton` (disabled-with-reason),
  `GuidedField`, `ConfirmDialog` with safe-default ordering. knotra's modals
  already validate-before-execute; wiring disabled-reasons through them is
  the main new work.
- **Phase 3 — guided "Get latest safely".** A plan→review→run→result flow
  with per-row friendly messages. knotra's Smart Pull already has
  plan-confirm-execute; this phase is largely re-wording + per-row result
  copy.
- **Phase 4 — guided "Save release point".** Plain wording inside the Tag
  modal + disabled-with-reason on the primary button.
- **Phase 5 — guided setup / empty states + undo for remove-from-list.**
- **Phase 6 — accessibility hardening pass.**

Each later phase is a candidate for its own version bump.

## Non-goals

- No change to the state model, message flow, or VCS layer.
- No second string system; no renamed theme tokens.
- Mobile support is not promised (the review agrees: keep desktop target).

## Open questions

None blocking Phase 1. Later phases will confirm exact friendly-result copy
with the reviewer.
