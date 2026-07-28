# RFC-032 - Dashboard Grouping, Sorting, and Tier Density

| Field | Value |
|---|---|
| Status | Implemented (main: d98374a) |
| Priority | High - the visible grouping control is inert and the active dashboard does not implement its promised information hierarchy |
| Effort | Medium |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/state/dashboard.rs`, `crates/knotra-app/src/state/tier.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/config.rs`, `crates/knotra-app/src/tests.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/010-attention-tiers.md`, `rfcs/done/027-selection-mode-and-bulk-selection-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Replace the dashboard's inert `Group by` button and competing render paths
with one deterministic display pipeline. Users can group projects by attention,
by their configured project group, or not at all. They can sort by recommended
urgency or by name. Grouping and sorting are independent, visible preferences
and persist through the existing application configuration.

Every project is still classified as **Needs help**, **In progress**, or
**All set**, regardless of grouping mode. That classification controls the
information density of its row. Needs-help rows explain the primary problem
and expose one safe next action; in-progress rows show the work area and one
most-relevant count; all-set rows show only identity and work area. The active
render path must use those distinct row types.

This RFC supersedes the unfulfilled external and internal dashboard contracts
in RFC-010 where they conflict with this design. In particular, it does not add
speculative dirty-age tracking, per-project default-branch settings, hover-only
information, or per-workspace UI metadata.

## Background

RFC-010 described automatic attention tiers and distinct information density,
and the roadmap later marked that design implemented. The current repository
contains parts of that design:

- `compute_tier` classifies status into three attention tiers;
- `view_tier_grid` renders collapsible tier headers;
- `GroupingMode` and `TierMessage::GroupingModeChanged` exist in state;
- `build_display_groups` groups projects by `Project.group`;
- the i18n catalog contains tier labels and hints.

The production behavior does not complete the contract:

- the toolbar renders a `Group by` button with no message;
- `GroupingMode` is only `Auto` or `Legacy`, with no usable visible selector;
- automatic tier sections call the same dense `view_project_card` used by the
  legacy grid, so tier-specific density is not active;
- specialized `card_needs_attention`, `card_active`, and `card_clean` functions
  are dead code;
- the dead needs-attention card contains hardcoded English and technical terms;
- the tier render path does not consistently apply the dashboard filter before
  classification;
- refreshing forces the generic grid even when attention grouping is selected;
- the generic grid uses a fixed four-column layout that does not define a safe
  narrow-window behavior;
- project order inside groups is inherited from workspace storage rather than
  being a named, deterministic user choice;
- grouping, sorting, and collapse preferences are not persisted.

The result is false affordance and inconsistent scanning behavior. A visible
control promises a choice but cannot be used, while the default mode groups by
attention without delivering the visual hierarchy that makes attention
grouping useful.

## Motivation

### User trust

A grouping control must change the dashboard immediately and predictably.
Projects must not disappear because filtering, grouping, and tiering apply in a
different order. A project with a conflict or unreadable status must remain
prominent in every grouping mode.

### Product readiness

The dashboard is the primary repeated-use surface. It must support quick
scanning across many repositories without showing every status counter and
action on every row. The current dense generic card makes healthy repositories
as visually expensive as repositories that require intervention.

### Operational safety

Sorting and grouping are presentation operations and must never trigger VCS
commands. Inline actions on needs-help rows must route to already supported,
typed workflows. The dashboard must not invent destructive recovery shortcuts
or infer Git/jj behavior from display strings.

## Requirements

### Functional requirements

R1. The toolbar must expose a working grouping selector with exactly these
initial options:

1. **Needs help** - sections ordered Needs help, In progress, All set.
2. **Project group** - sections from `Project.group`, with Ungrouped last.
3. **No grouping** - one flat project list.

R2. The toolbar must expose a working sorting selector with exactly these
initial options:

1. **Needs help first** - the exact typed per-tier urgency keys in
   **Deterministic recommended ordering**, including progress kind/count, then
   Unicode-lowercased project name and `ProjectId` tie-breaker.
2. **Name A-Z** - Unicode-lowercased project name, then `ProjectId`.

R3. Grouping and sorting are orthogonal. Changing one must not silently change
the other.

R4. Every visible project must be classified into exactly one attention tier
before rendering, in this priority order:

1. missing registered path;
2. active conflict;
3. conflict detection unavailable;
4. repository status read error;
5. typed detached context (`VcsContext::is_detached`);
6. uncommitted or untracked work;
7. local commits ahead of upstream;
8. remote commits behind upstream;
9. otherwise all set.

Current Git branch names, jj bookmarks, and jj change labels are identity only.
They do not change tier without another typed status fact. The repository does
not expose a typed default-branch fact, so this RFC removes the current
`main` / `master` / `trunk` heuristic instead of inferring a baseline from a
display label.

R5. Missing status during startup or refresh is **Unknown**, displayed within
Needs help with neutral unavailable wording. It must not be described as a
failed operation or a missing path unless structured state proves that cause.

R6. The filter result is finalized before grouping, section counts, empty
states, and sorting. Classification may be computed first as a pure input to
the All set and Needs help chip predicates, but filtered projects do not enter
later display stages. Status chips use typed predicates, not the current
single-priority `StatusColor`:

| Displayed chip | Typed predicate |
|---|---|
| All set | classified tier is `AllSet` |
| Updates available | usable status facts and `remote.behind > 0` |
| Local commits | usable status facts and `remote.ahead > 0` |
| Unsaved work | usable status facts and `working_tree.is_dirty()` |
| Needs a choice | status exists and `conflict.has_conflict` |
| Needs help | classified tier is `NeedsHelp` |

Status facts are not usable for MissingPath, ReadUnavailable, or StatusUnknown;
their counters must not enter fact-chip predicates. Conflict,
ConflictDetectionUnavailable, and DetachedContext retain other observed status
facts. The resulting truth table is:

| Classified case | All set | Updates | Local commits | Unsaved | Choice | Needs help |
|---|---:|---:|---:|---:|---:|---:|
| MissingPath | no | no | no | no | no | yes |
| Conflict | no | if behind | if ahead | if dirty | yes | yes |
| ConflictDetectionUnavailable | no | if behind | if ahead | if dirty | no | yes |
| ReadUnavailable | no | no | no | no | no | yes |
| DetachedContext | no | if behind | if ahead | if dirty | no | yes |
| StatusUnknown | no | no | no | no | no | yes |
| InProgress: uncommitted/untracked | no | if behind | if ahead | yes | no | no |
| InProgress: ahead | no | if behind | yes | no | no | no |
| InProgress: behind | no | yes | if ahead | no | no | no |
| AllSet | yes | no | no | no | no | no |

Multiple active status chips are OR. Search and project-group filtering are AND
constraints around that OR result. A project may truthfully match more than one
fact chip; for example, a conflicted project that is behind may appear under
either Updates available or Needs a choice, while its row still presents the
higher-priority conflict problem. `StatusFilter::Healthy` and
`StatusFilter::Error` should be renamed or replaced by typed `AllSet` and
`NeedsHelp` variants so internal names match the product contract.

R7. Every `LoadPhase` follows the phase matrix below. Whenever an active
workspace has projects, Startup, Refreshing, Ready, and Error use the same
display pipeline and row renderers. Refreshing must not switch to a generic card
layout.

| Load phase | Workspace/status state | First-level behavior |
|---|---|---|
| Startup | no workspace loaded | localized checking placeholder |
| Startup | workspace loaded | common pipeline; missing snapshots are `StatusUnknown`; checking notice |
| Refreshing | stale snapshots available | common pipeline with stale snapshots; checking notice |
| Refreshing | no snapshots available | common pipeline with `StatusUnknown` rows; checking notice |
| Ready | any active workspace | common pipeline and normal empty states |
| Error | workspace loaded | common pipeline with latest snapshots/unknown rows; localized check-failed notice and Retry |
| Error | no workspace loaded | localized no-workspace recovery surface; Create workspace and Show details; no Retry |

`LoadPhase::Error` raw text is available only after `Show details`; it must not
render in the first-level notice or row problem text.

Dashboard load-error disclosure has dedicated state and does not reuse the
modal-result `show_op_details` flag. Entering any new Error resets dashboard
details to hidden and clears the refresh-in-progress guard. With a loaded
workspace, Retry resets details, guarantees the guard is clear, and starts the
existing workspace refresh path. Without a workspace, Retry is absent because
there is no status task to run; Create workspace dispatches the existing
`WorkspaceMessage::CreateWorkspaceDialogOpened` flow. Successful recovery,
workspace switch, and a new refresh also reset dashboard error details.
Repeated errors therefore never inherit an earlier disclosure choice, and a
visible Retry after `TaskError` cannot be a no-op.

R8. Attention grouping uses fixed section order. Needs help is always expanded.
In progress is expanded by default and may be collapsed. All set is collapsed
by default and may be expanded. Collapse preferences apply only to Attention
grouping; Project group and No grouping render every filtered row because they
have no tier sections to collapse.

R9. Project-group sections sort by Unicode-lowercased group name;
Ungrouped appears last. No-grouping mode renders no empty section header.

R10. Tier classification controls row density in every grouping mode:

- Needs help: project name, VCS identity, short localized problem, and one safe
  primary action.
- In progress: project name, work-area label, and one most-relevant count.
- All set: project name and work-area label only.

R11. The most-relevant in-progress count and its sort kind use this fixed
`ProgressKind` priority:

1. uncommitted files;
2. untracked files;
3. commits ahead;
4. commits behind;

There is no work-area-only In-progress condition in this RFC. Larger values
sort first only within the same `ProgressKind`; kind priority sorts before
numeric value, so one uncommitted file sorts before one hundred behind commits.

R12. Needs-help actions are limited to existing supported workflows:

- active conflict: open Conflict Resolution;
- missing path: open project Details, where removal remains confirmed;
- unavailable conflict detection, read error, detached context, or unknown
  status: open project Details;
- no direct discard, reset, abort, branch creation, path editing, installation,
  or command replay is introduced by this RFC.

R13. Every row variant must preserve RFC-027 selection semantics. The checkbox
slot appears only in selection mode, uses the same stable project ID, and must
not change section height or reorder projects when toggled. One pure display
result must expose the exact ordered IDs of rendered, selectable rows. The view,
range selection, Select visible projects, palette selection actions, and
`selection_summary.visible_ids` must consume that same ordered list.

Rows inside collapsed In-progress or All-set sections are not rendered and are
not selectable-visible. Filter or collapse changes that hide selected rows must
prune those IDs immediately before a later bulk action can use them. Grouping
changes also reconcile selection because switching into Attention grouping may
activate persisted collapse. Sorting alone preserves selection because it only
reorders rows. Tests must prove the rule for every grouping/sort/collapse
combination.

The same reconciliation invariant applies whenever any display input changes:
merged or replaced workspace status, missing-path detection, load-phase
snapshot changes, workspace/project membership changes, filters, grouping, and
Attention collapse. A single `reconcile_selection_with_display` helper performs
the pruning after those mutations. As a defensive boundary, selection summaries
and every bulk-operation entry point also intersect selected IDs with the
current `ordered_selectable_ids` before enabling or starting work. A missed
reconciliation call therefore cannot mutate a hidden project.

R14. Clicking a project name continues to open the existing project Details
panel. Group headers and selector controls must not enter selection mode.

R15. Grouping mode, sorting mode, In-progress collapse, and All-set collapse
must persist in `AppConfig` with serde defaults for existing configuration
files. Preference changes apply immediately. Save failure keeps the session
choice and shows a localized warning that it will not survive restart.

R16. The old `Auto` / `Legacy` names, dead tier-card functions, and inert group
button must be removed after migration. There must be one display pipeline and
one active set of row renderers.

### Non-functional requirements

R17. All new or touched first-level text must use the English and Japanese i18n
catalogs. Raw VCS error text may appear only behind Details, not in a primary
dashboard row. Catalog coverage tests must include all touched `dashboard.*`
and `filter.*` keys; Japanese filter values must not retain literal English
Behind/Ahead labels.

R18. Grouping and sorting selectors must expose their selected value, keyboard
focus, and option labels through standard Iced controls. They must not be
implemented as unlabeled cycling buttons.

R19. Rows and section headers must use stable responsive constraints. The
dashboard must not rely on a fixed four-column count. At the minimum supported
800 x 600 window, text must wrap or truncate without overlapping actions,
checkboxes, or adjacent rows.

R20. Section expand/collapse controls require a visible label, count, and state;
keyboard activation must use the same message path as pointer activation.

R21. Git and jj share the same grouping, sorting, and density pipeline. VCS
kind may change the typed problem/action selection, but must not create a
separate dashboard layout.

R22. No grouping, sorting, filtering, or collapse action may acquire the global
operation interlock or run a VCS task.

R23. Existing mutation controls rendered in dashboard rows remain disabled
with the established localized busy reason while another operation owns the
interlock.

## Goals

- Make both dashboard selectors truthful and persistent.
- Make urgent repositories visually dominant without hiding healthy ones.
- Preserve deterministic project membership and order through filter, refresh,
  workspace switch, and selection-mode transitions.
- Replace generic dense cards with three genuinely different row densities.
- Remove dead dashboard card paths and hardcoded card copy.
- Keep the implementation within existing project/status/config boundaries.

## Non-goals

- Editing a project's group assignment from the dashboard.
- User-defined sorting formulas or drag-and-drop ordering.
- Multiple simultaneous group dimensions or nested attention/project groups.
- Per-workspace display preferences.
- Persisting scroll position or search/filter chips across restarts.
- Dirty-age tracking, default-branch configuration, or
  `treat_untracked_as_clean` project metadata from RFC-010.
- Hover-only counter disclosure.
- New recovery operations, path editing, terminal launch, branch creation,
  discard/reset, or automatic retry.
- Replacing operation History or implementing per-project VCS History.
- A broad redesign of the project Details panel.

## External Design

### Toolbar

The existing filter chips remain first. The right side contains:

```text
[Select]  Group: [Needs help v]  Sort: [Needs help first v]  [Search...]
```

On narrow windows the controls may wrap to another toolbar row. Each selector
shows its current value; opening it shows the complete option set. There is no
generic `Group by` button after implementation.

### Attention grouping

```text
Needs help (2)
  api       Git   Changes need your choice                [Resolve]
  website   Git   Project folder is unavailable           [Details]

In progress (3) v
  worker    feature/jobs                         4 changed
  cli       main                                 2 untracked
  docs      docs-refresh                         1 ahead

All set (18) >
```

Needs help cannot be collapsed. A zero-count Needs-help section is omitted.
In-progress and All-set headers remain visible when their visible count is zero
only when that is necessary to explain a filtered empty result; otherwise empty
sections are omitted.

### Project-group grouping

```text
Backend (3)
  [tier-specific rows sorted by the selected sort]

Frontend (2)
  [tier-specific rows sorted by the selected sort]

Ungrouped (1)
  [tier-specific row]
```

Tier density remains visible inside project groups. Recommended sorting puts
Needs-help rows before In-progress and All-set rows within each project group.

### No grouping

One unframed list is rendered. Recommended sorting puts Needs-help rows first;
Name A-Z ignores tier rank but retains each row's tier-specific density.

### Load phases

The dashboard does not change composition when a status read starts. Startup
or Refreshing with a loaded workspace renders its projects through the common
pipeline; projects without snapshots use the neutral Status unavailable row.
Stale snapshots remain visible during Refreshing.

When loading fails with a workspace, the dashboard retains its rows and shows a
localized first-level notice with **Try again** and **Show details**. Retry
dispatches dashboard `ErrorRetryRequested`, which clears stale refresh state
before routing through workspace refresh and returning to Refreshing.

With no loaded workspace, the surface explains that there is no workspace to
check and offers **Create workspace** plus **Show details**. It does not render
Retry. Create workspace dispatches the existing working workspace-dialog
message; Show details remains dashboard-specific and initially closed for every
new error. Raw error text appears only after Show details in either case.

### Empty and unavailable states

- No projects registered: retain the existing guided add-project state.
- Filters match nothing: localized no-match state with Clear filters action.
- Status not loaded: show a neutral Needs-help row such as “Status is not
  available yet”; do not display zero counters as if they were observed.
- Load error with workspace: retain rows, show generic localized failure copy
  and Retry, and keep raw adapter text behind Show details.
- Load error without workspace: show localized unavailable/recovery copy,
  Create workspace, and Show details; do not show Retry.
- Preference save failure: non-modal status feedback; the selected display
  remains active for the session.

### Accessibility and layout

Rows are full-width list rows rather than a manually packed four-column grid.
This produces stable scan columns and avoids cards becoming too narrow. The
checkbox/action columns have fixed minimum widths; the name/problem area may
wrap. Controls retain the project's existing 44px target policy where they are
interactive.

## Internal Design

### Types

Replace `GroupingMode` with serialized presentation types owned by the app:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DashboardGrouping {
    #[default]
    Attention,
    ProjectGroup,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DashboardSort {
    #[default]
    Recommended,
    NameAscending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardTier {
    NeedsHelp,
    InProgress,
    AllSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardCause {
    MissingPath,
    Conflict,
    ConflictDetectionUnavailable,
    ReadUnavailable,
    DetachedContext,
    StatusUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgressKind {
    Uncommitted,
    Untracked,
    Ahead,
    Behind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelevantCount {
    pub kind: ProgressKind,
    pub value: u32,
}

pub struct DashboardEntry<'a> {
    pub project: &'a Project,
    pub status: Option<&'a ProjectStatus>,
    pub tier: DashboardTier,
    pub cause: Option<DashboardCause>,
    pub relevant_count: Option<RelevantCount>,
}

pub enum DashboardSectionKey<'a> {
    Tier(DashboardTier),
    ProjectGroup(Option<&'a str>),
    Flat,
}

pub struct DashboardSection<'a> {
    pub key: DashboardSectionKey<'a>,
    pub collapsed: bool,
    /// All filtered entries, retained so collapsed headers keep an exact count.
    /// The view renders these rows only when `collapsed` is false.
    pub entries: Vec<DashboardEntry<'a>>,
}

pub struct DashboardDisplay<'a> {
    pub sections: Vec<DashboardSection<'a>>,
    /// Exact rendered row order after filters and attention collapse.
    pub ordered_selectable_ids: Vec<ProjectId>,
}
```

The exact type names may follow local conventions, but grouping, sorting,
classification cause, progress kind, and relevant count must remain typed.
Display strings must not drive ordering or action routing.

### Configuration

Add serde-defaulted fields to `AppConfig`:

```rust
pub dashboard_grouping: DashboardGrouping,
pub dashboard_sort: DashboardSort,
pub dashboard_in_progress_collapsed: bool,
pub dashboard_all_set_collapsed: bool,
```

Defaults are Attention, Recommended, false, and true. These are global display
preferences. This intentionally supersedes RFC-010's unimplemented
per-workspace collapse persistence and avoids adding UI metadata to the VCS
workspace model.

### Messages and update path

Use explicit dashboard messages:

```rust
pub enum DashboardMessage {
    GroupingChanged(DashboardGrouping),
    SortChanged(DashboardSort),
    TierToggled(DashboardTier),
    ErrorDetailsToggled,
    ErrorRetryRequested,
}
```

The handler updates state/config synchronously and attempts `save_config`.
Failure sets localized status feedback but does not roll back the visible
session preference. Needs-help toggle messages are rejected defensively.

Add `dashboard_error_details_open: bool` to app-owned dashboard state. It is
independent from modal `show_op_details`. `ErrorDetailsToggled` is accepted only
in `LoadPhase::Error`. `ErrorRetryRequested` is accepted only in Error with
`state.workspace.is_some()`, sets dashboard details false and `is_refreshing`
false, then directly routes through the existing
`WorkspaceMessage::RefreshRequested` handler so it always enters Refreshing and
starts a task. The no-workspace view never emits this message; a defensive
no-workspace dispatch is a no-op while the visible Create workspace action uses
`WorkspaceMessage::CreateWorkspaceDialogOpened`.

`BackgroundMessage::TaskError` must set `is_refreshing = false`, enter Error,
and reset dashboard details false. Starting or successfully completing a
refresh and switching workspace also reset it. A later Error resets it again,
regardless of modal details state.

After any mutation listed in R13 changes display membership, the update path
calls `reconcile_selection_with_display`, which rebuilds the pure display
result and retains selected IDs only when they occur in
`ordered_selectable_ids`. If none remain, selection mode may remain active in
its established zero-selection state. Sorting changes preserve selected IDs
while range-selection order immediately follows the reordered display result.

`selection_summary` and bulk handlers must still intersect selected IDs with
the current ordered selectable set. This is a defensive safety check, not a
second ordering implementation; both consume `DashboardDisplay`.

`TierMessage` and `GroupingModeChanged` are removed once callers migrate.

### Display pipeline

`state/dashboard.rs` owns a pure `build_dashboard_display` function with this
sequence:

1. join every workspace project to its optional status by `ProjectId`;
2. classify each project from structured status/path state;
3. apply text and project-group filters plus the R6 typed status predicates;
4. compute its typed relevant count and urgency rank;
5. partition according to `DashboardGrouping`;
6. sort sections and entries according to `DashboardSort`;
7. apply Attention-only collapse visibility;
8. return sections plus `ordered_selectable_ids` in exact rendered order.

The view and `AppState::visible_project_ids` both consume this result; neither
reconstructs project membership or order. Startup, Refreshing, Ready, and Error
use the same result whenever a workspace is loaded. Phase-specific UI adds only
the notice/actions from R7. Refreshing uses stale status where available.

### Deterministic recommended ordering

Recommended ordering has an explicit key per tier:

```text
Needs help:  (0, cause_rank, normalized_name, project_id)
In progress: (1, progress_kind_rank, Reverse(count), normalized_name, project_id)
All set:     (2, normalized_name, project_id)
```

- needs-help cause rank: Conflict, Missing path, Detection unavailable,
  Read unavailable, Detached context, Status unknown;
- progress kind rank: Uncommitted, Untracked, Ahead, Behind;
- values compare descending only inside the same progress kind;
- normalized name and project ID make every order stable.

Therefore one uncommitted file sorts before one hundred behind commits, while
five uncommitted files sort before one uncommitted file. Pairwise tests lock
this policy.

Name A-Z uses `(normalized_name, project_id)` only. Locale-aware collation is
not required in this RFC; normalization is Unicode lowercase with deterministic
fallback ordering.

Project-group section ordering uses `(normalized_group_name,
original_group_name)` where the exact original group string is the stable
section key. Ungrouped is a separate sentinel ordered last. Distinct case
variants therefore never inherit hash-map or workspace input order.

### View composition

`view/dashboard.rs` is split into narrow helpers:

- toolbar selectors;
- section header;
- needs-help row;
- in-progress row;
- all-set row;
- shared identity and selection affordance;
- localized problem/action mapping.

The generic `view_project_card`, dead `card_*` functions, fixed `COLS`, and
`#![allow(unused_imports)]` are removed when no longer needed. Page sections
remain unframed; rows may use a restrained divider/background but must not
become cards nested inside section cards.

### Persistence and history

Display preference changes persist only to application config. They do not
create operation logs or Activity entries because no repository operation
occurred. Workspace/project persistence formats are unchanged.

## Security Considerations

- Classification and action selection use enums and project IDs, never parsed
  status labels, branch text, repository paths, or error strings.
- Group and project names are rendered as text and never interpolated into a
  shell or VCS command.
- Needs-help rows dispatch only existing typed messages. Missing-path handling
  does not delete files; project removal still uses its confirmation flow.
- Grouping, sorting, filtering, and collapse are presentation-only and cannot
  acquire an operation lease or execute commands.
- Raw adapter errors stay out of the first-level dashboard, reducing accidental
  exposure of local paths in the primary view.

## Test Plan

### Unit tests

Add pure-state tests for:

- each classification rule and priority overlap;
- typed detached Git and jj context handling;
- current branch/bookmark labels not changing tier without another typed fact;
- unknown status versus missing path versus read error;
- the complete R6 filter truth table, OR semantics, and overlapping facts;
- filter labels never contradicting the fact/tier predicate they represent;
- relevant-count selection and pairwise progress-kind ordering;
- recommended and name ordering with stable tie-breaks;
- case-variant named project groups and Ungrouped-last behavior;
- every project appearing exactly once in each grouping mode;
- `ordered_selectable_ids` matching exact non-collapsed rendered order for every
  grouping/sort/filter/collapse combination;
- selection reconciliation after merged/replaced status, missing-path updates,
  workspace/project membership changes, and load-phase snapshot changes;
- serde defaults and config round-trip for all dashboard preferences using
  temporary `AppPaths::under` paths only.

### UI contract tests

Prove:

- each selector option dispatches its typed message and changes the active
  display preference;
- selector changes persist and save failure through a deliberately blocked
  temporary `AppPaths::under` path leaves a localized warning;
- tier collapse removes rows from rendered/selectable membership and prunes
  selected hidden IDs;
- refresh moving a selected Needs-help row into collapsed All set prunes it
  before a bulk handler reads selection;
- refresh making a selected row fail an active fact filter prunes it before a
  bulk handler reads selection;
- defensive selection summaries and bulk entry points intersect with the same
  current `ordered_selectable_ids` result;
- Needs help cannot be collapsed;
- Startup with/without a workspace, Refreshing with/without stale status, Ready,
  and Error with/without a workspace follow the R7 matrix;
- Error shows localized retry/details controls and keeps raw text behind
  details when a workspace is loaded;
- Error without a workspace renders no Retry, explains why status cannot be
  checked, and its Create workspace control opens the existing dialog;
- defensive `ErrorRetryRequested` without a workspace does not enter a
  completion-less Refreshing state;
- dashboard error details do not inherit modal `show_op_details`, reset hidden
  on repeated errors, and clear on loaded-workspace Retry and successful
  recovery;
- loaded-workspace Retry after `TaskError` clears a stale `is_refreshing` guard,
  enters Refreshing, and starts the workspace refresh task;
- all three row variants preserve selection toggle behavior;
- range selection, Select visible projects, palette selection, and
  `selection_summary.visible_ids` use `ordered_selectable_ids` exactly;
- conflict and missing-path primary actions route to the correct existing
  workflow;
- no-match state offers a working clear-filter action;
- busy mutation controls remain disabled with a reason.

### i18n tests

- require English and Japanese entries for every touched `dashboard.*`,
  `filter.*`, and tier/problem/action key;
- run first-level wording checks over those prefixes;
- explicitly reject literal English Behind/Ahead values in the Japanese filter
  catalog.

### Rendering evidence

Capture representative English and Japanese dashboard renders at:

- minimum supported window size (800 x 600);
- standard desktop size;
- wide desktop size.

Fixtures must include long project/group names, all three tiers, selection mode,
an expanded All-set section, and a busy operation. Review for overlap, clipped
controls, unstable row heights, and readable wrapping.

### Commands

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p knotra
cargo test -p knotra-ui
env TMPDIR="$PWD/.git-exclude/tmp" \
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
  GIT_EDITOR=true VISUAL=true EDITOR=true \
  cargo test -p knotra-vcs
git diff --check
```

No VCS adapter behavior is expected to change. The VCS suite remains a release
regression gate because dashboard actions continue to enter existing workflows.

## Acceptance Criteria

- [ ] The grouping selector works for Needs help, Project group, and No grouping.
- [ ] The sorting selector works for Needs help first and Name A-Z.
- [ ] Grouping and sorting choices persist across restart with backward-compatible defaults.
- [ ] The R6 typed filter truth table and OR semantics are implemented and localized.
- [ ] Filtering finalizes membership before grouping, counts, and sorting.
- [ ] Every R7 load-phase row uses the common pipeline when a workspace exists.
- [ ] Error surfaces keep raw adapter text behind dashboard Show details.
- [ ] Error with a workspace provides a Retry that starts status refresh.
- [ ] Error without a workspace omits Retry and provides a working Create workspace action.
- [ ] Every visible project appears exactly once in every grouping mode.
- [ ] Needs-help, In-progress, and All-set rows have materially different information density.
- [ ] Needs help is always expanded; other collapse preferences persist.
- [ ] Recommended ordering is deterministic and uses typed state.
- [ ] Branch/bookmark display labels do not infer a default branch or change tier alone.
- [ ] Unlike progress counts follow the explicit kind rank before numeric value.
- [ ] Selection mode works identically in every row variant.
- [ ] View, range selection, Select visible, palette, and summaries consume one ordered selectable-ID result.
- [ ] Every display-membership mutation reconciles selection through one helper.
- [ ] Status refresh and missing-path changes prune newly hidden selected rows before bulk actions.
- [ ] Selection summaries and bulk entry points defensively intersect with current selectable IDs.
- [ ] Collapsed/filtered selected rows are pruned before bulk actions.
- [ ] Needs-help actions route only to supported typed workflows.
- [ ] Git and jj use the same display pipeline with typed context handling.
- [ ] No fixed four-column layout remains in the active dashboard.
- [ ] No inert grouping/sorting control or dead tier-card render path remains.
- [ ] No touched first-level dashboard text is hardcoded or exposes raw adapter errors.
- [ ] Dashboard error disclosure is independent and reset on each error/recovery.
- [ ] Loaded-workspace Retry works after `TaskError` without inheriting a stale refresh guard.
- [ ] Touched dashboard/filter keys have English/Japanese coverage with no literal English Japanese-chip values.
- [ ] Narrow English and Japanese rendering evidence shows no overlap or clipped controls.
- [ ] UI contract tests prove control to message to state/view behavior.
- [ ] Formatting, clippy, app, UI, VCS, and whitespace gates pass with current evidence.

## Deferred Follow-ups

- Per-project VCS history remains the next RFC in the drafting sequence.
- Broad project Details localization remains part of the production-wide i18n
  verification track, not this dashboard layout RFC.
- Locale-aware collation may replace deterministic lowercase ordering later if
  user evidence justifies the dependency and complexity.
