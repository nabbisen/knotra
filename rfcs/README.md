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

| RFC  | Title                                         | Status   | Priority |
|------|-----------------------------------------------|----------|----------|
| [001](001-history-log-copy.md)              | Complete `HistoryMessage::LogCopyRequested`         | Proposed | High   |
| [002](002-stash-entry-commit-id.md)         | Add `commit_id` to `StashEntry`                     | Proposed | Medium |
| [003](003-jj-conflict-detection.md)         | jj conflict detection: gix path or documented exception | Proposed | Medium |
| [004](004-ahead-behind-gix.md)              | Ahead/Behind counts via gix                         | Proposed | Low    |
| [005](005-annotated-tag-freezer.md)         | Annotated tag support in the Freezer                | Proposed | Medium |
| [006](006-jj-log-since-range.md)            | Accurate `log_since` range for jj                   | Proposed | Medium |
| [007](007-topology-multi-manifest.md)       | Topology scan: multi-manifest support               | Proposed | Low    |
| [008](008-fspoller-prune-on-switch.md)      | Prune `FsPoller` snapshots on workspace switch      | Proposed | Low    |
