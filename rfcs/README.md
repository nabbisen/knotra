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

---

## Done

Implemented and shipped.  These are historical records; the design decisions
they contain remain authoritative.

### main — Production Readiness Reset checkpoint

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [031](./done/031-activity-retry-semantics.md) | Activity retry semantics | working tree; pending commit |
| [030](./done/030-changelog-modal-completion.md) | Changelog modal completion | main: f22dc5e |
| [029](./done/029-typed-context-switching-and-context-switch-modal-completion.md) | Typed context switching and context switch modal completion | main: 9821bef |
| [028](./done/028-command-palette-action-completion.md) | Command palette action completion | main: 3699bad |
| [027](./done/027-selection-mode-and-bulk-selection-completion.md) | Selection mode and bulk-selection completion | main: 0fd1e22 |
| [026](./done/026-conflict-resolution-action-completion-and-editor-launch-hardening.md) | Conflict Resolution Action Completion and Editor-Launch Hardening | main: 1cde97d |
| [025](./done/025-freezer-release-point-execution-completion.md) | Freezer / Release Point Execution Completion | main: d9f687a |
| [024](./done/024-smart-pull-modal-execution-completion.md) | Smart Pull modal execution completion | main: 4362a2e |
| [023](./done/023-workspace-management-completion.md) | Workspace management completion | main: 02e1481 |

### v0.23.0 — snora 0.25.0 migration

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0022](./done/022-snora-0.25.0-migration.md) | Migrate to snora 0.25.0; evaluate + defer the Snora Design System | v0.23.0 |

### v0.22.0 — RFC-0021 Phase 6 (complete)

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0021](./done/021-plain-language-layer.md) | Accessibility hardening: contrast, focus, labels, modal width (Phase 6 — RFC complete) | v0.22.0 |

### v0.21.0 — RFC-0021 Phase 5

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0021](./done/021-plain-language-layer.md) | Guided setup, empty states, undo for removal (Phase 5) | v0.21.0 |

### v0.20.0 — RFC-0021 Phases 2–4

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0021](./done/021-plain-language-layer.md) | Guided modal flows, safe components, 72 i18n keys (Phases 2–4) | v0.20.0 |

### v0.19.0 — Plain-language layer

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0021](./done/021-plain-language-layer.md) | Plain-language first-level wording (Phase 1); expert terms behind "Show details" | v0.19.0 |

### v0.18.0 — endringer migration

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0020](./done/020-endringer-0.33.1-migration.md) | Migrate to endringer 0.33.1 (stable version; zero code changes) | v0.18.0 |

### v0.17.0 — Screen removal

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0017](./done/017-screen-removal.md) | Remove five legacy full-screen views; `Screen` enum trimmed to Dashboard/History/Settings | v0.17.0 |

### v0.16.0 — snora layout adoption

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0019](./done/019-snora-layout-adoption.md) | Adopt snora 0.18 layout engine: overlay re-layer, `app_tab_bar`, remove dead `nav_menu` | v0.16.0 |

### v0.15.0 — Crate migration

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0018](./done/018-published-crate-migration.md) | Re-layer onto published `endringer` 0.19.2; rename in-tree `snora` → `knotra-ui` | v0.15.0 |

### v0.12.0 — UI/UX Redesign

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0009](./done/0009-selection-model.md)        | Selection model and selection bar               | v0.12.0 |
| [0010](./done/0010-attention-tiers.md)        | Three-tier attention grouping                   | v0.12.0 |
| [0011](./done/0011-activity-strip.md)         | Activity strip at bottom of window              | v0.12.0 |
| [0012](./done/0012-command-palette.md)        | Command palette (⌘K)                            | v0.12.0 |
| [0013](./done/0013-bulk-action-modals.md)     | Bulk action modals (replaces 5 screens)         | v0.12.0 |
| [0014](./done/0014-project-detail-panel.md)   | Project detail side panel                       | v0.12.0 |
| [0015](./done/0015-workspace-tabs.md)         | Workspace tabs                                  | v0.12.0 |
| [0016](./done/0016-keyboard-shortcuts.md)     | Keyboard shortcuts and cheat sheet              | v0.12.0 |

### v0.11.0 — Technical correctness

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [0001](./done/0001-history-log-copy.md)        | Complete `HistoryMessage::LogCopyRequested`    | v0.11.0 |
| [0002](./done/0002-stash-entry-commit-id.md)   | Add `commit_id` to `StashEntry`               | v0.11.0 |
| [0003](./done/0003-jj-conflict-detection.md)   | jj conflict detection — documented CLI exception | v0.11.0 |
| [0004](./done/0004-ahead-behind-gix.md)        | Ahead/Behind counts via gix                   | v0.11.0 |
| [0005](./done/0005-annotated-tag-freezer.md)   | Annotated tag support in the Freezer          | v0.11.0 |
| [0006](./done/0006-jj-log-since-range.md)      | Accurate `log_since` range for jj             | v0.11.0 |
| [0007](./done/0007-topology-multi-manifest.md) | Topology scan: Cargo.toml-only scope          | v0.11.0 |
| [0008](./done/0008-fspoller-prune-on-switch.md)| Prune `FsPoller` snapshots on workspace switch | v0.11.0 |

### Policy

| ID   | Title                                           | Shipped    |
|------|-------------------------------------------------|------------|
| [000](./done/000-rfc-lifecycle-policy.md) | RFC lifecycle policy (this directory's own rules) | v0.12.1 |

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

Full rules: [RFC 000](./done/000-rfc-lifecycle-policy.md).
