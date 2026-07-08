# RFC-010 — Attention Tiers and Tiered Card Density

| Field          | Value                                                                |
|----------------|----------------------------------------------------------------------|
| Status      | **Implemented** (v0.12.0)         |
| Priority       | **High** — defines the new mental model of the dashboard             |
| Effort         | Medium — tier classification, layout changes, settings toggle        |
| Target version | v0.13                                                                |
| Related        | `view/dashboard.rs`, `state/dashboard.rs`, RFC-009 (selection bar)  |

## Summary

Group projects on the Dashboard into three automatically computed tiers —
**Needs Attention**, **Active**, **Clean** — and render each tier's cards
with a different density and information content.  Replaces the current
filter-chip-OR-grid as the primary navigation tool for finding projects
that need work.

## Background

The current Dashboard has filter chips (Synced / Behind / Ahead /
Uncommitted / Untracked / Conflict) plus an optional grouping by an
arbitrary string field on each project.  This is the **Git data model**
exposed directly to the user.  The user's actual mental model has three
buckets:

1. "What's broken?"  →  conflicts, missing paths, failed recent operations
2. "What am I working on?"  →  uncommitted, ahead, non-default branch
3. "What's fine?"  →  synced, clean, default branch

The redesign auto-classifies every project into one of these buckets and
renders the buckets distinctly, with the most attention-worthy bucket
shown first and most prominently.

## Requirements

| #   | Requirement |
|-----|-------------|
| R1  | Every project is classified into exactly one of three tiers              |
| R2  | Tier classification is deterministic from `ProjectStatus`               |
| R3  | Tiers are visually distinct: header label, icon, default color           |
| R4  | The Clean tier is collapsed by default; user can expand                  |
| R5  | Cards in Needs Attention show problem statement + inline recovery button |
| R6  | Cards in Active show branch + the single most relevant counter           |
| R7  | Cards in Clean show name + branch only (single-line rows)                |
| R8  | Tier collapse state persists across restarts (per-workspace)             |
| R9  | A "legacy grouping" toggle in Settings reverts to v0.11.0 behaviour      |
| R10 | An empty tier shows "🎉 No projects need attention" or similar           |

## External Design

### Tier classification rules

Computed from `ProjectStatus`:

```
TIER = NeedsAttention if any of:
  - status.read_error is Some (path missing, repo broken)
  - status.conflict.has_conflict == true
  - status.conflict.detection_unavailable == true and vcs_kind == Jujutsu
  - status.context.is_detached == true
  - last operation on this project failed and no successful op since
  - status.working_tree.uncommitted_count > 0 AND project has been dirty for
    >7 days (timestamp tracked separately; future enhancement)

TIER = Active if any of:
  - status.working_tree.uncommitted_count > 0
  - status.working_tree.untracked_count > 0
  - status.remote.ahead > 0
  - status.context.branch is Some and branch != default branch
    (where "default" is configurable per-project, defaulting to "main" or "master")

TIER = Clean otherwise
```

`untracked > 0` alone is a borderline case (some workflows leave untracked
files indefinitely).  Resolution: include in Active unless a per-project
setting `treat_untracked_as_clean: true` opts out.  Default off.

### Visual layout

```
┌────────────────────────────────────────────────────────────────────┐
│ 🔴  Needs attention (2)                                            │
│ ──────────────────────────────────────────────────────────────────│
│ ┌────────────────────────────────────────────────────────────────┐│
│ │ ☐ alpha             [git]                                      ││
│ │   Conflict on main · 3 conflicted files · ↓3 behind            ││
│ │   [Resolve…]  [Abort merge]  [Open in editor]                  ││
│ └────────────────────────────────────────────────────────────────┘│
│ ┌────────────────────────────────────────────────────────────────┐│
│ │ ☐ beta              [git]                                      ││
│ │   Path not found: /home/me/code/beta                           ││
│ │   [Update path…]  [Remove from workspace]                      ││
│ └────────────────────────────────────────────────────────────────┘│
│                                                                    │
│ 🟡  Active (4)                                                     │
│ ──────────────────────────────────────────────────────────────────│
│ ☐ gamma             feature-x  ·  ↑2                              │
│ ☑ delta             feature-x  ·  3 dirty                         │
│ ☐ epsilon           bugfix-y   ·  1 dirty                         │
│ ☐ zeta              main       ·  ↑5                              │
│                                                                    │
│ ⚪  Clean (24)   ▶                                                  │
└────────────────────────────────────────────────────────────────────┘
```

### Tier header

```
🔴  Needs attention (2)
─────────────────────────────────────
```

- Icon: ⛔ filled circle in red, 🟡 filled circle in yellow, ⚪ filled circle in muted gray.
- Label: localized.
- Count in parentheses.
- Right-aligned: collapse toggle (▼ expanded, ▶ collapsed).
- Tap/click on the header toggles collapse.

### Per-tier card heights

| Tier            | Height (comfortable) | Content                                              |
|-----------------|---------------------|------------------------------------------------------|
| Needs Attention | 80–100 px           | Name + badge / problem statement / inline actions    |
| Active          | 40 px               | Name + badge / branch / one counter                  |
| Clean           | 28 px (row)         | Name + badge / branch  (no counters)                 |

In **compact** density mode (Settings):

| Tier            | Height (compact) |
|-----------------|------------------|
| Needs Attention | 60 px            |
| Active          | 30 px            |
| Clean           | 22 px            |

### Default expansion state

| Tier            | Default expanded? |
|-----------------|-------------------|
| Needs Attention | ✓ always; cannot be collapsed |
| Active          | ✓ by default; user can collapse |
| Clean           | ✗ by default; user can expand   |

Persisted in `~/.config/knotra/workspaces/<uuid>.toml` as:

```toml
[ui.tier_collapse]
needs_attention = false  # always; ignored on load
active = false
clean = true
```

### Empty tiers

| Tier            | Empty message |
|-----------------|---------------|
| Needs Attention | (hidden entirely when empty) |
| Active          | "Nothing in progress." (small, muted, only shown if expanded) |
| Clean           | "All projects need attention or are in progress." (only on expand) |

When all three tiers are non-empty, no empty-state messages appear.

### Most-relevant counter selection

For Active tier cards, only the single most pressing counter is shown.
The priority is:

```
uncommitted > 0   →  "N dirty"
ahead > 0         →  "↑N"
untracked > 0     →  "N untracked"
behind > 0        →  "↓N"  (rare in Active tier; more likely Needs Attention)
on non-default branch → branch name shown without counter
```

Active tier cards reveal all counters on hover (compact tooltip) or in the
side panel (RFC-014).

### Problem statement language

For Needs Attention cards, the problem statement is one short sentence in
the user's language.  Examples:

| Cause                          | English statement                                            |
|--------------------------------|--------------------------------------------------------------|
| `read_error: Some(...)`        | "Path not found: {path}"                                     |
| `conflict.has_conflict`        | "Conflict on {branch} · {n} conflicted files"                |
| `conflict.detection_unavailable` (jj) | "Conflict status unknown (`jj` binary not found)"     |
| `context.is_detached`          | "Detached HEAD at {short_commit}"                            |
| Last operation failed          | "Last {op}: failed ({short_reason})"                         |
| Dirty for >7 days              | "Working tree dirty for {days} days"                         |

### Inline recovery actions per Needs Attention cause

| Cause                              | Buttons shown                                                  |
|------------------------------------|----------------------------------------------------------------|
| Path not found                     | [Update path…]  [Remove from workspace]                        |
| Conflict (Git)                     | [Resolve…]  [Abort merge]  [Open in editor]                    |
| Conflict (jj)                      | [Resolve…]  [Open in terminal]                                 |
| Conflict detection unavailable (jj)| [Install jj]  [Hide warning]                                   |
| Detached HEAD                      | [Switch to default branch]  [Create branch here…]              |
| Last operation failed              | [Retry…]  [See details]                                        |
| Dirty for >7 days                  | [Open in editor]  [Stash…]  [Discard…] (with confirmation)     |

## Internal Design

### New types

```rust
// state/dashboard.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tier {
    NeedsAttention,
    Active,
    Clean,
}

impl Tier {
    pub fn icon(self) -> &'static str {
        match self {
            Tier::NeedsAttention => "🔴",
            Tier::Active         => "🟡",
            Tier::Clean          => "⚪",
        }
    }
    pub fn i18n_key(self) -> &'static str {
        match self {
            Tier::NeedsAttention => "tier.needs_attention",
            Tier::Active         => "tier.active",
            Tier::Clean          => "tier.clean",
        }
    }
}

/// Result of classifying a single project.
#[derive(Clone, Debug)]
pub struct Classified {
    pub project_id: ProjectId,
    pub tier:       Tier,
    /// For NeedsAttention only — the most pressing cause.
    pub cause:      Option<AttentionCause>,
}

#[derive(Clone, Debug)]
pub enum AttentionCause {
    PathNotFound { path: String },
    Conflict { branch: String, files: u32 },
    ConflictDetectionUnavailable,
    DetachedHead { short_commit: String },
    OperationFailed { op: String, reason: String },
    DirtyForLong { days: u32 },
}
```

### Classification function

```rust
// state/dashboard.rs
pub fn classify(
    status: &ProjectStatus,
    project: &Project,
    recent_failure: Option<&OperationFailure>,
    dirty_since: Option<chrono::DateTime<chrono::Utc>>,
    config: &AppConfig,
) -> Classified {
    // Order matters: first matching rule wins.
    if let Some(err) = &status.read_error {
        return Classified {
            project_id: project.id.clone(),
            tier: Tier::NeedsAttention,
            cause: Some(AttentionCause::PathNotFound { path: project.path.clone() }),
        };
    }
    if status.conflict.has_conflict {
        return needs_attention(project, AttentionCause::Conflict {
            branch: status.context.as_ref().map(|c| c.label.clone()).unwrap_or_default(),
            files:  status.conflict.conflict_count.unwrap_or(0),
        });
    }
    if status.conflict.detection_unavailable {
        return needs_attention(project, AttentionCause::ConflictDetectionUnavailable);
    }
    if status.context.as_ref().is_some_and(|c| c.is_detached) {
        return needs_attention(project, AttentionCause::DetachedHead {
            short_commit: short_hash_from_context(status),
        });
    }
    if let Some(f) = recent_failure {
        return needs_attention(project, AttentionCause::OperationFailed {
            op:     f.operation_kind.clone(),
            reason: f.short_reason.clone(),
        });
    }
    if let Some(since) = dirty_since {
        let days = (chrono::Utc::now() - since).num_days() as u32;
        if days >= config.dirty_warning_days.unwrap_or(7) {
            return needs_attention(project, AttentionCause::DirtyForLong { days });
        }
    }

    // Active checks.
    let wt = &status.working_tree;
    let on_non_default = status.context.as_ref()
        .and_then(|c| c.branch.as_ref())
        .is_some_and(|b| !is_default_branch(b, &project.default_branch));
    let ahead = status.remote.ahead;
    if wt.uncommitted_count > 0 || ahead > 0
        || (wt.untracked_count > 0 && !project.treat_untracked_as_clean)
        || on_non_default
    {
        return Classified { project_id: project.id.clone(), tier: Tier::Active, cause: None };
    }
    Classified { project_id: project.id.clone(), tier: Tier::Clean, cause: None }
}
```

### View structure

```rust
// view/dashboard.rs
pub fn view(state: &AppState) -> Element<'_, Message> {
    let workspace = state.workspace.as_ref().unwrap_or(&empty_ws);
    let classifications = classify_all(state);
    let grouped         = group_by_tier(&classifications);

    let mut col = column![];
    for tier in [Tier::NeedsAttention, Tier::Active, Tier::Clean] {
        let projects = grouped.get(&tier).map(|v| v.as_slice()).unwrap_or(&[]);
        if tier == Tier::NeedsAttention && projects.is_empty() {
            continue; // hide entirely
        }
        col = col.push(tier_header(state, tier, projects.len()));
        if state.tier_expanded(tier) {
            for c in projects {
                col = col.push(card_for_tier(state, c));
            }
        }
    }
    col.into()
}

fn card_for_tier(state: &AppState, c: &Classified) -> Element<'_, Message> {
    match c.tier {
        Tier::NeedsAttention => needs_attention_card(state, c),
        Tier::Active         => active_card(state, c),
        Tier::Clean          => clean_row(state, c),
    }
}
```

### Persistence

Per-workspace tier collapse state goes into the workspace TOML file:

```toml
# ~/.config/knotra/workspaces/<uuid>.toml
[ui]
[ui.tier_collapse]
active = false
clean  = true
```

### AppConfig additions

```rust
pub struct AppConfig {
    // ... existing fields ...

    /// "auto" (default, tier grouping) or "legacy" (filter chips + flat list)
    pub grouping_mode: GroupingMode,

    /// Days of dirty before a project moves to Needs Attention.
    /// None = never automatically promote.
    pub dirty_warning_days: Option<u32>,
}

pub enum GroupingMode {
    Auto,
    Legacy,
}
```

### `dirty_since` tracking

A new in-memory map updated on every status refresh:

```rust
// state/mod.rs
pub struct AppState {
    /// First observation of `uncommitted_count > 0` since last clean refresh.
    /// Cleared when the project becomes clean.
    pub dirty_since: HashMap<ProjectId, chrono::DateTime<chrono::Utc>>,
}
```

On each refresh:

```rust
fn update_dirty_since(state: &mut AppState, statuses: &[ProjectStatus]) {
    for s in statuses {
        let is_dirty = s.working_tree.uncommitted_count > 0;
        if is_dirty {
            state.dirty_since.entry(s.project_id.clone())
                .or_insert_with(chrono::Utc::now);
        } else {
            state.dirty_since.remove(&s.project_id);
        }
    }
}
```

For persistence across restarts, the map is serialised to disk on shutdown
and reloaded on startup, similar to the operation log.  Path:
`~/.local/share/knotra/dirty_since.json`.

### Filter chips (legacy mode)

When `grouping_mode == Legacy`, the existing filter chip UI and flat list
are rendered.  This is the v0.11.0 behaviour.

Even in Auto mode, the search box and the optional manual filter chips
remain available — they filter **within** tiers, hiding non-matching cards
but preserving the tier structure.

## Migration Plan

| Phase | Version | Scope |
|-------|---------|-------|
| 1     | v0.13   | Classification logic, tier headers, tier-specific card layouts.  Both Auto and Legacy modes available; default = Auto |
| 2     | v0.14   | Inline recovery buttons wire to RFC-013 modals |
| 3     | v0.16   | Legacy mode deprecated (still available but hidden behind a Settings toggle) |
| 4     | v0.17 (future) | Legacy mode removed |

## Test Plan

### Unit tests (`crates/knotra-app/src/tests.rs`)

1. **`classify_clean_project_is_clean_tier`** — status with no conflicts,
   no ahead/behind, no dirty → Clean.
2. **`classify_dirty_project_is_active`** — uncommitted_count = 2 → Active.
3. **`classify_conflict_is_needs_attention`** — has_conflict → NeedsAttention
   with `Conflict` cause.
4. **`classify_path_missing_overrides_conflict`** — both flags set; rule
   order picks `PathNotFound`.
5. **`classify_detection_unavailable_is_needs_attention`** — jj
   `detection_unavailable: true` → NeedsAttention.
6. **`classify_detached_head_is_needs_attention`** — is_detached → NeedsAttention.
7. **`classify_recent_failure_is_needs_attention`** — last op failed →
   NeedsAttention with `OperationFailed` cause.
8. **`classify_long_dirty_is_needs_attention`** — dirty_since 30 days ago,
   config = 7 days → NeedsAttention.
9. **`classify_non_default_branch_is_active`** — context.branch = "feature-x",
   default = "main" → Active.
10. **`active_card_picks_uncommitted_over_ahead`** — uncommitted=2, ahead=3 →
    "2 dirty" shown.
11. **`tier_collapse_state_persists`** — set clean tier expanded, save, reload
    → expanded.
12. **`legacy_mode_renders_filter_chips`** — config grouping_mode = Legacy →
    `view::dashboard::renders_filter_chips(state) == true`.

### Manual test plan

1. Workspace with 1 conflict + 3 dirty + 5 clean → tiers show 1 / 3 / 5;
   Needs Attention auto-expanded, Active expanded, Clean collapsed.
2. Toggle Clean tier expansion → state persists across restart.
3. Toggle to Legacy mode in Settings → filter chips reappear, tier headers
   disappear.
4. Click `[Resolve…]` button on a conflict card → routes to existing
   ConflictResolution screen (in v0.13; modal in v0.14).

## Open Questions

### Q1 — What constitutes "default branch"?

`is_default_branch(branch, project.default_branch)` requires per-project
configuration.  **Tentative answer**: Add a `default_branch: Option<String>`
field to `Project`.  When `None`, knotra calls `git symbolic-ref
refs/remotes/origin/HEAD` once when the project is added and stores the
result.  Users can override in project edit dialog.

### Q2 — Detached HEAD as Needs Attention?

Some workflows intentionally use detached HEAD (bisecting, examining
history).  Demoting all detached HEADs to NeedsAttention may be noisy.
**Tentative answer**: yes, but the inline action "Hide warning for this
session" suppresses it until next workspace switch.

### Q3 — Dirty-since persistence cost

Storing a map of `ProjectId → DateTime` for every project on disk on every
shutdown.  **Tentative answer**: fine.  JSON file < 10 KB even for 100
projects.  Update on shutdown only, not on every refresh.

### Q4 — Empty Needs Attention tier confirmation

When the user has zero attention items, should the dashboard show a
celebratory message?  **Tentative answer**: yes, a small dismissible banner
above the Active tier: "🎉 All clear" — but only when transitioning from
having attention items to having none, not on every load.

## Security Considerations

None.  Classification is purely local data.  No I/O.
