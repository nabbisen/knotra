# knotra RFCs

This directory contains design specifications (Request for Comments) for
knotra's planned changes.  Each file is a self-contained document addressed
to the implementer.

## Template

### Lightweight (default)

```markdown
# RFC-NNNN — Title

| Field  | Value |
|--------|-------|
| Status | Proposed / Accepted / Implemented / Rejected |
| Priority | High / Medium / Low |
| Effort | Trivial / Small / Medium / Large |
| Related | file paths or issue references |

## Summary
One-paragraph description.

## Background   ← omit when obvious
Why this matters.

## Problem      ← or "Motivation"
What is wrong / missing today.

## Design
What to build and how.

## Test Plan
What tests to add or change.

## Security Considerations
Impact on security, or "None."
```

### Extended (medium-to-large changes)

Add the following sections as needed:

- **Requirements** — numbered list of must-have properties
- **Design** → split into **External Design** and **Internal Design**
- **Alternatives considered**
- **Migration / rollout**

## Index

### v0.11.0 — Implemented (technical correctness)

| RFC  | Title                                                   | Status         | Priority |
|------|---------------------------------------------------------|----------------|----------|
| [001](001-history-log-copy.md)        | Complete `HistoryMessage::LogCopyRequested`             | **Implemented** | High   |
| [002](002-stash-entry-commit-id.md)   | Add `commit_id` to `StashEntry`                         | **Implemented** | Medium |
| [003](003-jj-conflict-detection.md)   | jj conflict detection: documented CLI exception (Option B) | **Implemented** | Medium |
| [004](004-ahead-behind-gix.md)        | Ahead/Behind counts via gix                             | **Implemented** | Low    |
| [005](005-annotated-tag-freezer.md)   | Annotated tag support in the Freezer                    | **Implemented** | Medium |
| [006](006-jj-log-since-range.md)      | Accurate `log_since` range for jj                       | **Implemented** | Medium |
| [007](007-topology-multi-manifest.md) | Topology scan: Cargo.toml-only scope documented (Option A) | **Implemented** | Low    |
| [008](008-fspoller-prune-on-switch.md)| Prune `FsPoller` snapshots on workspace switch          | **Implemented** | Low    |

### v0.12 – v0.16 — UI/UX Redesign (Proposed)

The redesign reorganises the UI from screen-based navigation to selection-driven
bulk actions on a single dashboard view.  See
[knotra-UI-UX-redesign.md](https://github.com/nabbisen/knotra/blob/main/docs/src/contributing/ui-ux-redesign.md)
for the rationale.

| RFC  | Title                                            | Target | Priority | Effort |
|------|--------------------------------------------------|--------|----------|--------|
| [009](009-selection-model.md)        | Selection model and selection bar  | v0.12 | **High**   | Medium |
| [010](010-attention-tiers.md)        | Three-tier attention grouping      | v0.13 | High       | Medium |
| [011](011-activity-strip.md)         | Activity strip (bottom bar)        | v0.12 | Medium     | Small–Medium |
| [012](012-command-palette.md)        | Command palette (⌘K)               | v0.12 stub / v0.13 full | Medium | Medium |
| [013](013-bulk-action-modals.md)     | Bulk action modals (replaces 5 screens) | v0.14 | **High** | **Large** |
| [014](014-project-detail-panel.md)   | Project detail side panel          | v0.15 | Medium     | Medium |
| [015](015-workspace-tabs.md)         | Workspace tabs at top              | v0.15 | Low–Medium | Small–Medium |
| [016](016-keyboard-shortcuts.md)     | Keyboard shortcuts and cheat sheet | v0.13 | Medium     | Medium |
| [017](017-screen-removal.md)         | Removal of deprecated screens      | v0.16 | Medium     | Small  |

## Release roadmap

| Release | Theme                          | RFCs implemented                  |
|---------|--------------------------------|------------------------------------|
| v0.11.0 | Technical correctness          | 001–008                          |
| v0.12.0 | Selection foundation           | 009, 011, 012 (stub)            |
| v0.13.0 | Attention model                | 010, 012 (full), 016            |
| v0.14.0 | Workflow modals                | 013                               |
| v0.15.0 | Detail + multi-workspace       | 014, 015                         |
| v0.16.0 | Cleanup                        | 017                               |
