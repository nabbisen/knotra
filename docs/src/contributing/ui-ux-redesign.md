# knotra UI/UX Redesign Proposal

> Critique of the current design (v0.11.0), proposed replacement, and migration path.
> This document is opinionated; it does not propose to keep everything.

---

## Part 1 — Critique of the Current Design

The current UI is functional but follows a structure that reflects the **implementation phases**, not the **user's workflow**.  Each phase added a screen; each screen got a sidebar item.  The result: eight screens, each a place to *go*, when most of what users do is sequences of actions on subsets of projects.

### Problem 1 — Screens reflect engineering, not workflows

| Screen            | What it represents                | What the user actually wants                          |
|-------------------|-----------------------------------|-------------------------------------------------------|
| Dashboard         | Status display                    | "Show me what needs attention"                        |
| Sync Center       | Fetch / pull operations           | "Bring these projects up to date"                     |
| Context Ops       | Branch / changeset switch         | "Get these projects onto feature-x"                   |
| Freezer           | Tag / bookmark creation           | "Tag these projects as v1.2.3"                        |
| Conflict Res.     | Conflict UI                       | "Fix the conflict in project-A"                       |
| Changelog         | Commit collection                 | "Generate release notes for these projects since v1.2" |
| History           | Past operations                   | "What did I just do? Did it work?"                    |
| Settings          | Configuration                     | "Change a setting"                                    |

The user-side column is **action-on-selection**, not "navigate to a place."
The current UI forces the user to:

1. See a problem on the Dashboard
2. Remember which screen handles that problem
3. Navigate there
4. Re-identify the projects involved
5. Perform the action
6. Navigate back to see the result

This is the core friction.  Every separate screen is a context switch.

### Problem 2 — Status and action are dissociated

The Dashboard shows "project-A is behind by 3 commits."  To pull it, the user
navigates to Sync Center, which is a separate view of the same projects with
a different layout.  The same data is presented twice in two different
formats, and the user has to mentally re-link "the card I saw" with "the row
in Sync Center."

A repository's state should be **the place where actions on it begin**.

### Problem 3 — Filter chips presuppose a power-user mental model

The current Dashboard has filter chips: `Synced` / `Behind` / `Ahead` /
`Uncommitted` / `Untracked` / `Conflict`.  This is the **Git data model**
projected directly into the UI.  Real users don't think this way; they think:

- "What's broken and I need to fix?"
- "What am I in the middle of?"
- "What's fine and I can ignore?"

Six filter chips combined with multi-select OR logic produce 63 possible
filter states.  Most of them are nonsensical.  Two or three coarse categories
match the actual mental model.

### Problem 4 — Cards are too dense

Each card currently shows: name, VCS badge, branch, ahead count, behind count,
uncommitted count, untracked count, conflict flag, last refresh time, path
warning.  That's ten data points per card.  For 50 projects, the user is
scanning 500 data points to find the three that matter.

Density should be **inverse to relevance**: the worst projects show the most;
clean projects collapse to a single line.

### Problem 5 — History is invisible until you need it, then hard to find

Operations are logged.  Logs are accessible by clicking "History" in the
sidebar, navigating to a different screen, scrolling, expanding entries.
After a bulk pull, the user wants to immediately see "what happened" — not
navigate to a screen and search.

### Problem 6 — Recovery hints are buried

When an operation fails, the recovery commands go into the operation log.
The user might not look at the log.  The hint should appear **on the card
that failed**, where the user's eyes already are.

### Problem 7 — Multi-workspace switching is heavy

Switching workspaces today: open sidebar, click workspace name in the list.
For users with two workspaces who switch frequently (e.g., personal vs.
work), this is too many steps.  Should be ⌘1, ⌘2 or a tab strip.

### Problem 8 — Discoverability of keyboard shortcuts is zero

Shortcuts exist (`Ctrl+R`, `Ctrl+K`, `Ctrl+T`, `Ctrl+/`) but there's no `?`
overlay listing them.  Users discover the app keyboard-mouse-first and never
learn the faster paths.

### What is good about the current UI and must be preserved

- **The state model is correct.**  ProjectStatus / WorkspaceStatus / etc. are
  the right abstractions.  The redesign is purely presentation.
- **Smart Pull's plan-confirm-execute flow.**  This is the right interaction
  pattern for any potentially destructive bulk operation.
- **Per-project recovery hints.**  The data is there; the placement is wrong.
- **Atomic rollback in the Freezer.**  Critical safety property; cannot be lost.
- **The transparency of operation logs.**  Every command-equivalent is
  recorded.  This is rare among GUIs and must be preserved.

---

## Part 2 — Redesign Principles

### P1 — Selection, then action

The fundamental interaction is: select one or more projects, then pick what
to do with them.  All bulk operations work this way.  No more navigating to a
screen that has its own project list.

### P2 — One main view; everything else overlays

There is one primary view: the project list.  Actions open modals or side
panels that overlay it.  When the action completes, the user is back where
they started, with the new state visible immediately.

### P3 — Group by attention, not by Git state

Three tiers, computed automatically:

- 🔴 **Needs attention** — conflict, detached HEAD, path missing, recent
  failed operation, dirty for >7 days
- 🟡 **Active** — uncommitted changes, ahead of upstream, non-default branch
- ⚪ **Clean** — synced, on default branch, working tree clean

Filtering still possible but secondary.

### P4 — Information density follows criticality

Cards in the Needs Attention tier are tall and show details + a recovery
button.  Cards in Active are medium and show the one relevant counter.  Cards
in Clean collapse to a single line.

### P5 — Outcomes are immediate and visible

After every action, the result is visible without navigation.  An activity
strip at the bottom shows the last operation's summary.  Clicking it opens
the full history.

### P6 — Keyboard parity with mouse

Every action achievable with the mouse must have a keyboard shortcut.  A
command palette (`⌘K` / `Ctrl+K`) covers anything not on a dedicated key.

### P7 — Workflows over screens

A workflow is a sequence of decisions toward a goal.  A screen is a fixed
arrangement of widgets.  Releases, syncs, switches, conflict resolutions are
workflows.  They should be flows, not destinations.

---

## Part 3 — Proposed Information Architecture

### 3.1 — Top-level layout

```
┌─────────────────────────────────────────────────────────────┐
│ knotra   [work] [personal] [+]   ⟳  ⚙  ?       [/ search]  │  ← workspace tabs + global controls
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   🔴  Needs attention (3)                                    │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ☐ project-alpha          conflict on main · ↓3      │   │
│   │   3 conflicted files · last fetch 2h ago            │   │
│   │   [Resolve…]  [Abort merge]  [Open in editor]       │   │
│   └─────────────────────────────────────────────────────┘   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ☐ project-beta           path not found             │   │
│   │   /home/me/code/project-beta does not exist         │   │
│   │   [Remove from workspace]  [Update path…]           │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
│   🟡  Active (4)                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ☑ project-gamma          feature-x · ↑2 · 3 dirty   │   │
│   └─────────────────────────────────────────────────────┘   │
│   ☑ project-delta          feature-x · ↑1                   │
│   ☐ project-epsilon        bugfix-y · 1 dirty                │
│   ☐ project-zeta           main · ↑5                         │
│                                                              │
│   ⚪  Clean (24)   ▶                                          │  ← collapsed
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ ✓ 2 selected   [⤓ Fetch]  [⤒ Pull]  [Tag…]  [Switch…]  [⋯]  │  ← selection bar
├─────────────────────────────────────────────────────────────┤
│ ⓘ Last: Fetched 28 projects · 27 ok, 1 failed (alpha)  ›    │  ← activity strip
└─────────────────────────────────────────────────────────────┘
```

### 3.2 — Tier cards

**Needs Attention** cards are tall (≈80 px) and contain:
- Project name + VCS badge
- One-line state summary
- Specific problem statement on second line
- Inline recovery actions (the buttons that previously lived in dedicated screens)

**Active** cards are medium (≈40 px) and contain:
- Project name + VCS badge
- Branch and the one most relevant counter

**Clean** cards collapse to single-line rows when the tier is expanded.
The tier is collapsed by default; expanding reveals a compact list.

### 3.3 — Workspace tabs

The current sidebar Workspaces section becomes a horizontal tab strip at the
top.  Each tab shows the workspace name and an unread-attention badge:

```
[work (3)]  [personal]  [+]
       ^^^
       count of "Needs attention" projects
```

`⌘1` / `⌘2` / `⌘3` switches between the first three workspaces.

### 3.4 — Project detail side panel

Clicking a project name (not the checkbox) opens a right-side panel showing:

- Full status: every field that's currently on the card, plus path, remote
  URL, last fetch time
- Recent operations on this project (last 5)
- Available actions for this project's specific state
- An "Open in terminal" button (new)

This replaces ContextOps for the single-project case.  The panel does not
prevent interaction with the main view (semi-modal).

### 3.5 — The selection bar

The bottom action bar appears whenever ≥1 project is selected and slides up
from below.  It shows:

- Selection count
- Primary actions: `Fetch`, `Pull`, `Tag…`, `Switch…`
- Overflow menu (`⋯`): Generate changelog…, Open all in terminal, Remove from
  workspace, Export status…

Buttons disable themselves when not applicable to the selection (e.g., `Pull`
disables if no selected project has an upstream).

The bar is keyboard-driven:
- `space` — toggle selection on focused card
- `f` — Fetch
- `p` — Pull
- `t` — Tag (opens modal)
- `b` — Switch branch (opens modal)
- `escape` — clear selection

### 3.6 — Activity strip

A single-line strip at the very bottom showing the most recent operation:

```
ⓘ Last: Fetched 28 projects · 27 ok, 1 failed (alpha)         [details ›]
```

States:
- Operation in progress: progress bar across the strip
- Operation complete: summary, click for details
- Recovery needed: red background, recovery commands inline
- Idle: empty / hidden

Clicking the strip expands a history panel showing the last 20 operations.

### 3.7 — Modals for workflows

Each major workflow becomes a modal that opens over the dashboard:

**Smart Pull modal**:
- Title: "Pull 4 projects"
- Plan section: per-project disposition (fast-forward / stash-pull-pop / skip)
- Per-project checkboxes to override defaults
- "Execute" button → streaming progress in-modal
- Result section: success/failure per project with recovery commands inline
- "Close" returns to dashboard with refreshed state

**Tag modal** (replaces Freezer screen):
- Selected projects shown as a list
- Tag name input
- Optional message input (annotated tag)
- Topology warnings inline (no separate scan button — scan runs automatically)
- "Validate" → shows blockers per project
- "Execute" → atomic with rollback
- Result inline

**Switch Branch modal** (replaces ContextOps screen):
- Selected projects shown
- Branch picker: shows branches that exist in **all** selected projects
- "Switch" → executes; dirty projects shown as blockers

**Generate Changelog modal**:
- Selected projects shown
- Since-ref input + tag picker
- "Generate" → renders Markdown preview
- "Copy to clipboard" / "Save to file"

**Conflict Resolution panel** (replaces ConflictResolution screen):
- Slides out as a right panel when "Resolve…" is clicked on a card
- File list with conflict markers
- Per-file: "Open in editor" / "Open merge tool" / "Mark resolved"
- "Abort merge" at the bottom

### 3.8 — Command palette (⌘K)

Opens a centered input field with fuzzy search over:

- All actions: "Fetch all", "Tag selected as…", "Generate changelog…"
- All projects: typing a name jumps focus to that card
- All workspaces: "Switch to workspace work"
- Settings: "Open settings", "Toggle dark mode"

This solves discoverability: anything you can do in knotra is findable by
typing in the palette.  Power users live in the palette; mouse users never
need to open it.

### 3.9 — Settings (unchanged location)

Settings remains a separate full-screen view, reached via the `⚙` icon or the
command palette.  It is visited rarely and benefits from full-screen layout.

The Settings page itself reorganized into:

- **General**: language, theme, status bar density
- **Refresh**: background interval, max concurrent reads, FS watch
- **External tools**: editor, merge tool, terminal
- **Storage**: where configs and logs are stored (read-only paths +
  "Open in file manager" button)
- **About**: version, license, links

Removed from Settings: nothing.  Just regrouped.

---

## Part 4 — Workflow Walkthroughs

### Workflow A — Morning sync

**Before (current UI):**
1. Open knotra
2. Glance at Dashboard
3. Click "Sync Center" in sidebar
4. Click "Fetch All"
5. Wait for results
6. Navigate back to Dashboard to see updated state
7. Notice one project is behind
8. Open Sync Center again
9. Click "Smart Pull" on that project
10. Confirm plan
11. Execute

**After (proposed):**
1. Open knotra
2. Press `f` (Fetch all — no selection = all)
3. Activity strip shows progress, then "Fetched 28 projects · 27 ok"
4. The one needs-attention project that's behind moves to its tier
5. Click "Pull" button on that card (or select it and press `p`)
6. Modal opens with single-project plan
7. Click Execute

Steps: 11 → 7.  Screen changes: 5 → 0.

### Workflow B — Cutting a release

**Before:**
1. Dashboard → identify projects in the release
2. Sidebar → Freezer
3. Enter freeze name
4. Tick projects
5. Optionally click "Scan Dependencies"
6. Click Validate
7. Review blockers
8. Click Execute
9. See results
10. Sidebar banner suggests pushing tags
11. Click push tags

**After:**
1. Dashboard → select projects (click checkboxes or `shift+click` range)
2. Press `t` (Tag…)
3. Modal opens with selection pre-filled, topology warnings already visible
4. Enter name + optional message
5. Click Execute
6. Modal shows result with "Push tags now?" button
7. Click

Steps: 11 → 7.  And step 2 reuses the dashboard selection — the user never
has to reselect projects.

### Workflow C — Fix a conflict

**Before:**
1. Dashboard shows conflict badge on project-A
2. Sidebar → ConflictResolution
3. Click project-A in the list
4. See files
5. Open editor
6. Resolve
7. Mark resolved
8. Sidebar → Dashboard to verify

**After:**
1. Dashboard shows project-A in the Needs Attention tier with an inline
   "Resolve…" button and 3-file count visible
2. Click "Resolve…"
3. Right panel slides in with the file list
4. Open editor (panel stays open)
5. Resolve and save
6. Click "Mark resolved" in the panel
7. Panel auto-closes when last conflict cleared; card moves out of Needs
   Attention tier

Steps: 8 → 7.  More importantly: zero full-screen navigation.

### Workflow D — Switch a feature branch across repos

**Before:**
1. Dashboard
2. Sidebar → ContextOps
3. Per-project: click "Switch", choose branch, confirm
4. Repeat for each project (no bulk)

**After:**
1. Dashboard → select the 4 projects working on feature-x
2. Press `b` (Switch branch)
3. Modal shows branches available in all selected projects
4. Pick `feature-x` and execute
5. Dirty projects shown as blockers; user resolves before retrying

Steps: per-project linear → single bulk operation.

---

## Part 5 — What Gets Removed

| Removed                     | Replacement                                          |
|-----------------------------|------------------------------------------------------|
| Sync Center screen          | `Fetch` / `Pull` toolbar buttons + Smart Pull modal |
| ContextOps screen           | `Switch…` toolbar button + Switch Branch modal      |
| Freezer screen              | `Tag…` toolbar button + Tag modal                   |
| Conflict Resolution screen  | Inline panel from Needs Attention card              |
| Changelog screen            | Overflow menu → Generate Changelog modal            |
| Sidebar navigation list     | Workspace tabs (top) + command palette              |
| Filter chip OR-grid         | Three-tier auto-grouping (filtering still possible) |
| "Group" field on workspace  | Tier-based grouping + optional manual groups        |
| `repo_exists` warning badge | Card in Needs Attention tier with explicit message  |

## Part 6 — What Gets Added

| Added                           | Purpose                                         |
|---------------------------------|-------------------------------------------------|
| Three-tier auto-grouping        | Match user's mental model                       |
| Selection model + selection bar | Bulk operations from dashboard directly         |
| Workspace tabs                  | Faster switching                                |
| Project detail side panel       | Drill-down without navigation                   |
| Activity strip                  | Outcomes visible without history navigation     |
| Inline action modals            | Workflow-based interaction                      |
| Command palette (⌘K)            | Discoverability + keyboard access               |
| Keyboard shortcut overlay (?)   | Make shortcuts learnable                        |
| Inline recovery on failed cards | Recovery commands where the user is looking     |
| Per-tier collapsed densities    | Information density follows attention           |
| `Open in terminal` action       | Most-requested missing action (anticipated)     |

---

## Part 7 — Information: What's Shown Where

### On a Needs Attention card

| Item                       | Always | Hover/expand | Side panel |
|----------------------------|--------|--------------|------------|
| Project name               | ✓      |              |            |
| VCS badge                  | ✓      |              |            |
| Problem statement          | ✓      |              |            |
| Most critical counter      | ✓      |              |            |
| Inline recovery button(s)  | ✓      |              |            |
| Branch name                |        | ✓            | ✓          |
| Ahead / behind             |        | ✓            | ✓          |
| Untracked count            |        |              | ✓          |
| Last refresh time          |        | ✓            | ✓          |
| Path                       |        |              | ✓          |
| Remote URL                 |        |              | ✓          |
| Recent operations          |        |              | ✓          |

### On an Active card

| Item                       | Always | Hover/expand | Side panel |
|----------------------------|--------|--------------|------------|
| Project name               | ✓      |              |            |
| VCS badge                  | ✓      |              |            |
| Branch name                | ✓      |              |            |
| One counter (ahead OR dirty) | ✓    |              |            |
| Other counters             |        | ✓            | ✓          |
| Untracked count            |        |              | ✓          |
| Path                       |        |              | ✓          |

### On a Clean card

| Item                | Always | Hover/expand | Side panel |
|---------------------|--------|--------------|------------|
| Project name        | ✓      |              |            |
| VCS badge           | ✓      |              |            |
| Branch name         | ✓      |              |            |
| Everything else     |        |              | ✓          |

### What is NOT shown anywhere by default

These are reachable via Side panel or command palette only:

- jj change-id (technical detail; jj users who need it can open the panel)
- Exact "last refreshed" seconds (rounded to "2m ago" granularity)
- Repository path (long; clutter on card)
- Detection-unavailable flag (only relevant when user is investigating;
  shown in side panel)
- Submodule count (rarely actionable)

---

## Part 8 — Visual / Accessibility Design

### Color use

Colour is **never the only signal**.  Every state has icon + label + position.

| State            | Icon | Color (Dark) | Color (Light)  | Position |
|------------------|------|--------------|----------------|----------|
| Needs attention  | 🔴   | #E54B4B      | #B7281A        | Top tier |
| Active           | 🟡   | #E6B450      | #B0780B        | Mid tier |
| Clean            | ⚪   | #5F6368      | #4A4A4A        | Btm tier |
| Selected         | ☑    | accent       | accent         | inline   |
| Loading          | ⟳    | accent       | accent         | inline   |

All foreground/background combinations meet WCAG AA contrast.

### Density modes

In Settings → General:

- **Comfortable** (default): generous padding, large icons
- **Compact**: reduced padding (≈50% vertical)
- **Dense**: monospace-style row layout for power users

### Empty states

- No workspaces: full-screen "Welcome — Create your first workspace" with a
  guided flow
- Workspace with zero projects: "Add a project" card with an empty-state
  illustration, dropdown for paste-path or browse
- All clean (no attention items, no active items): "🎉 All projects synced
  and clean" message above the collapsed Clean tier

### Loading states

The current "Refreshing…" badge stays on the card while a refresh is in
progress, but the card content does **not** dim or hide — stale-but-known is
more useful than blank-and-loading.

### Errors

When a fetch fails, the project moves to Needs Attention with the failure as
its problem statement.  When dismissed (or the next successful fetch), it
moves back.

---

## Part 9 — Migration Path

A redesign this size needs phased rollout to avoid disrupting current users.

### v0.12 — Foundations

- Add the selection model (`AppState::selected_project_ids: HashSet<ProjectId>`)
- Add the selection bar UI (sticky bottom panel)
- Add `Fetch` / `Pull` / `Tag…` / `Switch…` toolbar buttons that route to
  existing screens with the selection pre-filled (no behaviour change yet)
- Add the activity strip at the bottom
- Add the command palette (⌘K) — initially just a project name jumper

Existing screens remain accessible via the sidebar (deprecated badge).

### v0.13 — Tier-based grouping

- Compute attention tier per project: `Tier::NeedsAttention | Active | Clean`
- Add tier headers in the dashboard
- Per-card density follows tier
- Keep filter chips visible but de-emphasize them
- Add a Settings toggle: "Group by status (legacy)" / "Group by attention
  (default)"

### v0.14 — Inline workflows

- Convert Sync Center to a modal triggered from the toolbar `Pull` button
- Convert Freezer to a modal triggered from the toolbar `Tag…` button
- Convert ContextOps to a modal triggered from the toolbar `Switch…` button
- Remove sidebar entries for these three; they remain accessible via command
  palette
- Conflict Resolution becomes the right-side panel

### v0.15 — Detail panel

- Right side panel for project detail
- Workspace tabs replace sidebar workspace list

### v0.16 — Removal

- Drop the deprecated full-screen versions of Sync Center / Freezer /
  ContextOps / Conflict Resolution / Changelog
- Sidebar becomes minimal: just Settings and Help links (or removed
  entirely)

### Backward-compatible config

Existing config files continue to load.  New UI preferences (density, tier
collapse state, etc.) default sensibly when missing.

---

## Part 10 — Risks and Open Questions

### R1 — Discoverability of removed screens

Power users may rely on the explicit screens.  Mitigation: the command
palette and direct keyboard shortcuts cover all functionality.

### R2 — Visual density vs. simplicity

Three tiers + cards of varying heights is visually busy compared to a flat
list.  Mitigation: tested density modes; collapsible Clean tier; clean tier
single-line rows.

### R3 — Bulk action UX with mixed-state selection

What does "Pull" do when half the selection has upstream and half doesn't?
**Answer**: Modal shows the plan with disabled rows for projects that can't
be pulled, plus a one-line explanation.  Execute proceeds with the eligible
subset.  This is consistent with current Smart Pull behaviour.

### R4 — Multi-workspace + selection

Selection is per-workspace; switching workspaces clears selection.  Need to
preview behaviour: should there be a quick "select all" shortcut per
workspace?  **Tentative yes**: `⌘A` selects all in current workspace.

### R5 — Notification fatigue

If the activity strip is always changing, it becomes noise.  Mitigation: it
shows only the most recent operation, only updates on user-initiated
actions, never auto-refreshes itself, and fades to muted color after 30s.

### R6 — iced 0.14 implementation effort

Some features may strain iced's current widget set:
- Sticky bottom bar — feasible (containers + alignment)
- Modal overlays — feasible (stacking)
- Slide-out side panel — feasible (animated width transition)
- Command palette with fuzzy search — moderate (new widget, custom search)
- Workspace tabs with badges — feasible (existing button widget + custom layout)

Estimated effort across v0.12–v0.16: 8–12 weeks of focused work.

### R7 — i18n key explosion

New widgets and modals will need new keys.  Estimate: +40 keys for the
redesign, mostly straightforward strings.

---

## Part 11 — Anti-goals (Things This Redesign Will NOT Do)

These are tempting and explicitly out of scope:

- **Custom commit graph visualization** — out of scope; users who need this
  have specialized tools.
- **In-app diff viewer** — `Open in editor` covers it; building a diff
  viewer competes with VS Code etc.
- **Pull request integration** — would require external API connections;
  contrary to the local-first principle.
- **Real-time collaboration** — single-user tool.
- **Mobile / web version** — desktop-only.
- **AI assistant for commit messages / conflict resolution** — out of scope.
- **Embedded terminal** — `Open in terminal` launches the system terminal.

---

## Summary

The current UI is correct in its data model and safety properties, but its
information architecture is organised around how the code is structured
rather than around how users think.  Eight screens encode an
implementation-driven hierarchy; users want to **select projects and act on
them**.

The redesign collapses the eight screens into one main view (dashboard with
auto-tiered project list), one detail panel (right side, on demand), and
modal workflows for each action.  Discoverability is preserved through a
command palette; speed is preserved through keyboard shortcuts; safety is
preserved by keeping all existing plan-confirm-execute patterns.

A five-version migration path lets users see the new patterns alongside the
old before the old is removed.

**Single biggest win**: the user never has to navigate to a different screen
to act on the projects they're already looking at.
