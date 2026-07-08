# knotra RFCs

This directory holds all Request for Comments documents for knotra.
Documents are organised by lifecycle state following
[RFC 000](./done/000-rfc-lifecycle-policy.md).

**Folder is the source of truth for state.**
A file's location determines its state; the Status field inside the file
must be kept consistent with its folder.

```
rfcs/
  README.md       ← this index
  proposed/       ← open for review; implementation should not yet start
  done/           ← shipped; historical record only
  archive/        ← withdrawn or superseded
```

---

## Proposed

Open for review.  Design may still change.  Do not start implementation
until the RFC moves to `done/`.

| ID   | Title                                           | Target | Priority |
|------|-------------------------------------------------|--------|----------|
| [017](./proposed/017-screen-removal.md) | Remove deprecated screens (Sync Center, Freezer, ContextOps, ConflictResolution, Changelog) | v0.16 | Medium |

---

## Done

Implemented and shipped.  These are historical records; the design decisions
they contain remain authoritative.

### v0.15.0 — Crate migration

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [018](./done/018-published-crate-migration.md) | Re-layer onto published `endringer` 0.19.2; rename in-tree `snora` → `knotra-ui` | v0.15.0 |

### v0.12.0 — UI/UX Redesign

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [009](./done/009-selection-model.md)        | Selection model and selection bar               | v0.12.0 |
| [010](./done/010-attention-tiers.md)        | Three-tier attention grouping                   | v0.12.0 |
| [011](./done/011-activity-strip.md)         | Activity strip at bottom of window              | v0.12.0 |
| [012](./done/012-command-palette.md)        | Command palette (⌘K)                            | v0.12.0 |
| [013](./done/013-bulk-action-modals.md)     | Bulk action modals (replaces 5 screens)         | v0.12.0 |
| [014](./done/014-project-detail-panel.md)   | Project detail side panel                       | v0.12.0 |
| [015](./done/015-workspace-tabs.md)         | Workspace tabs                                  | v0.12.0 |
| [016](./done/016-keyboard-shortcuts.md)     | Keyboard shortcuts and cheat sheet              | v0.12.0 |

### v0.11.0 — Technical correctness

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [001](./done/001-history-log-copy.md)        | Complete `HistoryMessage::LogCopyRequested`    | v0.11.0 |
| [002](./done/002-stash-entry-commit-id.md)   | Add `commit_id` to `StashEntry`               | v0.11.0 |
| [003](./done/003-jj-conflict-detection.md)   | jj conflict detection — documented CLI exception | v0.11.0 |
| [004](./done/004-ahead-behind-gix.md)        | Ahead/Behind counts via gix                   | v0.11.0 |
| [005](./done/005-annotated-tag-freezer.md)   | Annotated tag support in the Freezer          | v0.11.0 |
| [006](./done/006-jj-log-since-range.md)      | Accurate `log_since` range for jj             | v0.11.0 |
| [007](./done/007-topology-multi-manifest.md) | Topology scan: Cargo.toml-only scope          | v0.11.0 |
| [008](./done/008-fspoller-prune-on-switch.md)| Prune `FsPoller` snapshots on workspace switch | v0.11.0 |

### Policy

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [00](./done/00-rfc-lifecycle-policy.md) | RFC lifecycle policy (this directory's own rules) | v0.12.1 |

---

## Archive

No RFCs have been withdrawn or superseded yet.
See [archive/](.//archive/.gitkeep) for the placeholder.

---

## RFC template

### Lightweight (default — small changes)

```markdown
# RFC-NNNN — Title

| Field    | Value                              |
|----------|------------------------------------|
| Status   | Proposed                           |
| Priority | High / Medium / Low                |
| Effort   | Trivial / Small / Medium / Large   |
| Related  | file paths or prior RFC numbers    |

## Summary
One paragraph.

## Problem
What is wrong or missing today.

## Design
What to build and how.

## Test Plan
What tests to add or change.

## Security Considerations
Impact on security, or "None."
```

### Extended (medium-to-large changes)

Add as needed:

- **Background** — context for readers unfamiliar with the area
- **Requirements** — numbered R1…Rn must-have properties
- **External Design** — what the user sees (ASCII mockups, interaction flows)
- **Internal Design** — state shape, message variants, file boundaries
- **Alternatives considered** — options weighed and why they lost
- **Migration / rollout** — how existing users and data are affected

---

## Lifecycle reference

```
Draft → Proposed → [Implemented → done/]
                 → [Withdrawn   → archive/]
                 → [Superseded  → archive/]
```

For Implemented RFCs the Status field carries the release tag:
`Implemented (v1.2.3)`.  
For Superseded RFCs: `Superseded by RFC NNNN`.  
For Withdrawn RFCs: `Withdrawn — <one-line reason>`.

Full rules: [RFC 00](./done/00-rfc-lifecycle-policy.md).
