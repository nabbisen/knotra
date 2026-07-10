# knotra — UI/UX Handoff Document

> Authoritative reference for implementing the knotra GUI.
> Derived from the codebase (v0.18.0) and the UI/UX redesign proposal.
> Intended for: AI developers implementing or extending the GUI.

---

## 1. Executive Summary

**App name:** knotra  
**Platform:** Desktop GUI — Linux (primary), cross-platform via iced/winit  
**Framework:** Rust, iced 0.14, snora 0.18 layout engine  

**Purpose:** A local-first dashboard for developers who manage many Git and
Jujutsu (jj) repositories in parallel. knotra shows the health of all
repositories on one screen and lets the user act on multiple repositories
at once — fetch, pull, tag, switch branch — without leaving that screen.

**Target users:** Developers maintaining 5–50+ repositories simultaneously
(microservices, monorepos with sub-repos, shared libraries, workspace
configurations). Both Git and jj users; both novice multi-repo users and
power users who live in the keyboard.

**Core value proposition:** Select projects → act on them. No screen changes.
No re-selecting. Every operation is logged with the exact VCS commands
executed, so nothing is a black box.

### Key user journeys

| Journey | Entry point | Completion |
|---|---|---|
| Morning sync | Dashboard (keyboard: `f`) | Activity strip shows result; cards update in place |
| Cut a release | Select projects → `t` | Tag modal → atomic execution with rollback |
| Fix a conflict | Needs Attention card → Resolve… | Resolve panel → card moves out of tier |
| Switch feature branch | Select projects → `b` | Switch modal → card contexts updated |
| Audit what happened | Activity strip → History | Searchable log with per-project commands |

---

## 2. Business Requirements

### Primary goals

1. Reduce time-to-awareness: user sees all project health at a glance in under 3 seconds from launch.
2. Reduce steps-to-action: any bulk operation completes in ≤7 user steps from Dashboard.
3. Zero silent failures: every failed operation surfaces a recovery path.
4. Keyboard parity: every mouse action has a keyboard equivalent.

### User personas

**Pat — the polyglot maintainer**  
Manages 20–40 repositories across 3 workspaces (work, personal, OSS). Switches
between Git and jj projects daily. Wants to see "what's wrong" and fix it fast.
Uses keyboard shortcuts once they're learned; initially relies on mouse.

**Sam — the release engineer**  
Cuts weekly releases across 8 microservices. Uses Freezer (Tag modal) every
release day. Cares deeply about atomic rollback — must not leave repos in a
half-tagged state. Reads the operation log after every release to audit.

**Jo — the new team member**  
Inherits 12 repositories. Doesn't know which ones are behind. Uses knotra to
get oriented. Does not know keyboard shortcuts. Relies on visible status labels
and inline action buttons.

### Success metrics

- Dashboard to first action ≤ 3 user gestures (click/keypress)
- Zero operations that leave repositories in undefined state
- All status indicators readable without color (WCAG AA text + icon)
- Keyboard-only navigation covers 100% of functionality

---

## 3. Functional Specifications

### Feature list (current v0.18.0)

| Feature | Priority | Status |
|---|---|---|
| Dashboard — three-tier card grid | P0 | Implemented |
| Workspace tab strip with attention badge | P0 | Implemented |
| Project cards (Needs Attention / Active / Clean) | P0 | Implemented |
| Selection bar (bulk actions) | P0 | Implemented |
| Activity strip (last operation summary) | P0 | Implemented |
| Sync & Pull modal (Fetch / Smart Pull) | P0 | Implemented |
| Freezer modal (atomic tag/bookmark creation) | P0 | Implemented |
| Context Switch modal (bulk branch/changeset switch) | P0 | Implemented |
| Conflict Resolve panel (right-docked sheet) | P0 | Implemented |
| Command palette (Ctrl+K) | P0 | Implemented |
| Project detail side panel | P1 | Implemented |
| History screen (searchable operation log) | P1 | Implemented |
| Settings screen | P1 | Implemented |
| Keyboard shortcut overlay (?) | P1 | Implemented |
| Add Project dialog | P1 | Implemented |
| Filesystem watch (config-gated) | P2 | Implemented |
| Japanese localisation | P2 | Implemented |

### Business logic rules

**Attention tier computation** (per project):
- `NeedsAttention` if ANY: conflict, `detection_unavailable`, path missing, recent failed operation, dirty for >7 days, detached HEAD
- `Active` if ANY: uncommitted changes, ahead of upstream, non-default branch
- `Clean` otherwise — synced, clean working tree, on default branch

> **Display wording (RFC-0021):** `NeedsAttention` / `Active` / `Clean` are the
> internal tier names. The first-level UI displays them in plain language as
> **Needs help** / **In progress** / **All set**, and status labels likewise
> use plain wording (e.g. Conflict → "Needs your choice"). Technical terms are
> shown under "Show details". All wording is routed through the i18n catalog
> (`tier.*`, `plain.*`, `status.*`) in English and Japanese — never hardcoded.

**Smart Pull safety rules:**
- Dirty repos: default to fetch-only; user may override to stash-pull-pop
- Conflicted repos: always excluded; cannot be overridden
- Execution order: sequential (prevents stacked conflict states)
- Rollback: `stash pop` attempted on failure; if rollback also fails, manual recovery commands are shown

**Freeze (Tag/Bookmark) safety rules:**
- Topology validation runs before execute button is enabled
- Blockers (dirty, existing tag, conflict): execute button disabled; blocker shown per-project
- Execution: sequential; on failure, already-applied tags are rolled back
- Never overwrites existing tags — user must delete manually

**Conflict detection for jj:**
- Uses `jj log -r @ -T conflict` (CLI), not file inspection
- When `jj` binary absent: `detection_unavailable: true` → UI shows "Unknown" (never "No conflict")

### Integration points

- **Git** via endringer-git 0.33.2 (reads) + `git` CLI (writes: fetch, pull, switch, tag)
- **Jujutsu (jj)** via endringer-jj 0.33.2 (reads) + `jj` CLI (writes: fetch, edit, bookmark, git fetch)
- **Filesystem** via `notify` (optional FS watch) and `endringer-async::FsPoller`
- **Config** at `~/.config/knotra/config.toml`
- **History** at `~/.local/share/knotra/history/<timestamp>_<op-id>.json`
- **External editor / merge tool**: shell-launched from buttons when configured

---

## 4. Screen Layouts & Wireframes

### Screen 1: Dashboard (primary view)

```
┌──────────────────────────────────────────────────────────────────┐
│  [work (3)]  [personal]  [oss]  [+]    ⟳  ⚙  ?   [/ search…]   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  🔴  Needs Attention  (2)                                        │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ ☐  project-alpha    Git   main ↓3   CONFLICT               │  │
│  │     3 conflicted files · last fetch 2h ago                 │  │
│  │     [Resolve…]  [Abort merge]  [Open in editor]            │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ ☐  project-beta     Git   —         PATH NOT FOUND         │  │
│  │     /home/me/code/project-beta does not exist              │  │
│  │     [Remove]  [Update path…]                               │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  🟡  Active  (3)                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ ☑  project-gamma    Git   feature-x   ↑2  3 uncommitted    │  │
│  └────────────────────────────────────────────────────────────┘  │
│  │ ☐  project-delta    jj    @abc123     ↑1                   │  │
│  │ ☐  project-epsilon  Git   bugfix-y    1 uncommitted         │  │
│                                                                  │
│  ⚪  Clean  (19)   ▶ (collapsed)                                  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  ☑ 1 selected    [⤓ Fetch]  [⤒ Pull]  [Tag…]  [Switch…]  [⋯]    │
├──────────────────────────────────────────────────────────────────┤
│  ⓘ Last: Fetched 24 projects · 24 ok                    [›]      │
└──────────────────────────────────────────────────────────────────┘
```

**Component table:**

| Component | Position | Width | Height | Notes |
|---|---|---|---|---|
| Workspace tab strip | Top, full-width | 100% | 36px | Tab per workspace; badge = NeedsAttention count; `+` adds workspace |
| Global toolbar (⟳ ⚙ ?) | Top right | auto | 36px | Refresh, Settings, Shortcut overlay |
| Search box | Top right | 220px | 28px | `Ctrl+/` focuses; clears on Esc |
| Tier header (🔴🟡⚪) | Section divider | 100% | 28px | Label + count; click to collapse Clean tier |
| Needs Attention card | Full-width | 100% | ~88px | 3-line: name+status / problem detail / action buttons |
| Active card | Full-width | 100% | ~44px | 2-line: name+status+counter / (hover expands) |
| Clean card | Full-width | 100% | ~32px | 1-line row when tier is expanded |
| Selection checkbox | Card left edge | 20px | 20px | Click to select; Space to toggle when focused |
| Selection bar | Bottom sticky | 100% | 48px | Visible when ≥1 selected; slides in from below |
| Activity strip | Bottom, below sel. bar | 100% | 28px | Last operation summary; progress bar when running |

---

### Screen 2: Sync & Pull modal

Opens as a centred dialog over the Dashboard.

```
┌─────────────────────────────────────────────────────┐
│  Pull  ·  4 projects                           [✕]  │
├─────────────────────────────────────────────────────┤
│  Disposition                                         │
│  ┌─────────────────────────────────────────────┐    │
│  │ ✓  project-gamma    fast-forward             │    │
│  │ ✓  project-delta    fast-forward             │    │
│  │ ✓  project-zeta     stash → pull → pop ⚠    │    │
│  │ —  project-eta      skip (conflicted)   ⛔   │    │
│  └─────────────────────────────────────────────┘    │
│                                                      │
│  ⚠ project-zeta has uncommitted changes.            │
│    Override: stash, pull, then restore.              │
├─────────────────────────────────────────────────────┤
│                        [Cancel]  [Execute Pull →]    │
└─────────────────────────────────────────────────────┘
```

**States:** Plan view → (Execute) → Streaming progress → Result view (success/failure per project with recovery hints inline)

---

### Screen 3: Freezer (Tag/Bookmark) modal

```
┌─────────────────────────────────────────────────────┐
│  Tag  ·  3 projects                            [✕]  │
├─────────────────────────────────────────────────────┤
│  Tag name    [v1.2.3                          ]      │
│  Message     [Release v1.2.3 (optional)       ]      │
│                                                      │
│  Validation                                          │
│  ┌─────────────────────────────────────────────┐    │
│  │ ✓  api-server       ready                    │    │
│  │ ✓  web-frontend     ready                    │    │
│  │ ⛔  data-pipeline   tag v1.2.3 already exists│    │
│  └─────────────────────────────────────────────┘    │
│                                                      │
│  ⛔ 1 blocker — resolve before executing.           │
├─────────────────────────────────────────────────────┤
│                  [Cancel]  [Execute Tag →]  (blocked)│
└─────────────────────────────────────────────────────┘
```

**Validation runs automatically** on name input change (debounced 400ms). Execute button is disabled while any blocker exists.

---

### Screen 4: Context Switch modal

```
┌─────────────────────────────────────────────────────┐
│  Switch Branch  ·  3 projects                  [✕]  │
├─────────────────────────────────────────────────────┤
│  Branch  [feature-x                   ▼]            │
│                                                      │
│  Available in all 3 selected projects               │
│  ● feature-x      ● main      ● bugfix-y            │
│                                                      │
│  ┌─────────────────────────────────────────────┐    │
│  │ ✓  api-server       main → feature-x         │    │
│  │ ⚠  web-frontend     main → feature-x (dirty) │    │
│  │ ✓  data-pipeline    main → feature-x         │    │
│  └─────────────────────────────────────────────┘    │
│                                                      │
│  ⚠ web-frontend has uncommitted changes.            │
│    Switch will proceed; changes are preserved.       │
├─────────────────────────────────────────────────────┤
│                        [Cancel]  [Switch →]          │
└─────────────────────────────────────────────────────┘
```

---

### Screen 5: Conflict Resolve panel (right-docked sheet)

Slides in from the right edge, covering ~50% of the Dashboard width. Dashboard remains visible and interactive to the left.

```
┌──────────────────────────────┐
│  Resolve — project-alpha [✕] │
├──────────────────────────────┤
│  3 conflicted files          │
│                              │
│  src/main.rs         ⛔     │
│    [Open in editor]          │
│  src/config.rs       ⛔     │
│    [Open in editor]          │
│  tests/lib.rs        ✓ done │
│    [Mark unresolved]         │
│                              │
│  ─────────────────────────── │
│  [Abort merge]               │
└──────────────────────────────┘
```

Panel auto-closes when all conflicts are marked resolved. Card moves out of Needs Attention tier immediately.

---

### Screen 6: History screen

Full-screen view (navigated to via Settings icon area or activity strip).

```
┌──────────────────────────────────────────────────────────────────┐
│  History               [Search…                          ] [✕]   │
├──────────────────────────────────────────────────────────────────┤
│  2026-06-11 14:32  Smart Pull · 4 projects · 4 ok         [▶]   │
│  2026-06-11 14:20  Tag v1.2.3 · 3 projects · 2 ok, 1 rb   [▶]   │
│  2026-06-11 09:05  Fetch · 24 projects · 24 ok             [▶]   │
│                                                                  │
│  ▼ 2026-06-11 14:20  Tag v1.2.3 · expanded                       │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ api-server      ✓   git tag -a v1.2.3 -m "Release v1.2.3"  │  │
│  │                     git push origin v1.2.3                 │  │
│  │ web-frontend    ✓   git tag -a v1.2.3 -m "Release v1.2.3"  │  │
│  │ data-pipeline   ⟲   rolled back (tag existed)             │  │
│  │                     Recovery: git tag -d v1.2.3            │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                              [Copy log entry]     │
└──────────────────────────────────────────────────────────────────┘
```

---

### Screen 7: Settings screen

Full-screen view.

```
┌──────────────────────────────────────────────────────────────────┐
│  Settings                                                  [✕]   │
├──────────────────────────────────────────────────────────────────┤
│  Display                                                         │
│    Language        [English ▼]                                   │
│    Theme           ( ) Light  (●) Dark                           │
│                                                                  │
│  Refresh & Performance                                           │
│    Background interval   [60    ] seconds  (0 = manual only)     │
│    Max concurrent reads  [8     ]                                │
│    Filesystem watch      [ ] enabled  debounce [2] s             │
│                                                                  │
│  External Tools                                                  │
│    Editor          [                              ] (optional)   │
│    Merge tool      [                              ] (optional)   │
│                                                                  │
│  Logs                                                            │
│    Max log entries [200   ]                                      │
│                                                                  │
│                                         [Cancel]  [Save →]       │
└──────────────────────────────────────────────────────────────────┘
```

---

## 5. UI Component Library

### Button

| Property | Value |
|---|---|
| Height (default) | 32px |
| Horizontal padding | 16px |
| Border radius | 6px (`CARD_RADIUS * 0.75`) |
| Typography | System UI, 13px, weight 500 |

| Variant | Background (dark) | Text (dark) | Background (light) | Text (light) |
|---|---|---|---|---|
| Primary | `#4C8BF5` | `#FFFFFF` | `#1565C0` | `#FFFFFF` |
| Secondary | `#3A3A3A` | `#E0E0E0` | `#E8E8E8` | `#1A1A1A` |
| Ghost | transparent | `#A0A0A0` | transparent | `#5A5A5A` |
| Danger | `#C62828` | `#FFFFFF` | `#B7281A` | `#FFFFFF` |
| Disabled | `#2A2A2A` | `#555555` | `#D0D0D0` | `#9A9A9A` |

States: Default / Hover (+8% brightness) / Pressed (-8%) / Disabled (no pointer events)

---

### Status badge (on project card)

Every status indicator renders: **icon + text label**. Color is never the sole signal.

| Status | Icon | Label (EN) | Color (dark) | Color (light) | `StatusColor` enum |
|---|---|---|---|---|---|
| Synced | ✓ | Synced | `#4CAF50` | `#2E7D32` | `Healthy` |
| Behind | ↓ | Behind | `#FFB74D` | `#E65100` | `Behind` |
| Ahead | ↑ | Ahead | `#42A5F5` | `#1565C0` | `Ahead` |
| Uncommitted | ● | Uncommitted | `#FFB74D` | `#E65100` | `Dirty` |
| Conflict | ⛔ | Conflict | `#EF5350` | `#C62828` | `Conflict` |
| Unknown | ? | Unknown | `#757575` | `#616161` | `Unknown` |
| Refreshing | ⟳ | Refreshing… | accent | accent | — |

---

### Project card

Three density tiers. All cards are full-width within the dashboard scroll area.

**Needs Attention card (~88px tall):**
```
┌──[checkbox 20px]──[name bold 14px]──[VCS badge]──[branch]──[status]──┐
│  [problem detail, muted 12px, second line]                            │
│  [action button 1]  [action button 2]  (ghost/danger buttons, 28px)   │
└───────────────────────────────────────────────────────────────────────┘
```
Padding: `CARD_PADDING` (14px top/bottom, 16px left/right)  
Corner radius: `CARD_RADIUS` (8px)  
Left border accent: 3px solid `StatusColor` value

**Active card (~44px tall):**
```
┌──[checkbox]──[name bold 13px]──[VCS badge]──[branch]──[counter]──┐
└───────────────────────────────────────────────────────────────────┘
```
Hover: expand to show secondary counters (+24px)

**Clean card (~32px tall, single line):**
```
  [checkbox]  [name 13px]  [VCS badge]  [branch, muted]
```
No border. Minimal padding (8px vertical).

---

### Tier header

```
  🔴  Needs Attention  (2)
```
Height: 28px. Font: 12px, weight 600, uppercase. Muted foreground (`#888`).
Clicking the Clean tier header toggles its expanded/collapsed state.

---

### Selection bar

Sticky to the bottom of the main scroll area. Slides up with a 150ms ease-out transition when ≥1 project is selected; slides back down when selection is cleared.

```
┌──────────────────────────────────────────────────────────────────┐
│  ☑ 3 selected    [⤓ Fetch]  [⤒ Pull]  [Tag…]  [Switch…]  [⋯]    │
└──────────────────────────────────────────────────────────────────┘
```
Height: 48px. Background: elevated surface (card-level).  
Buttons: Secondary variant; disabled when not applicable to selection.  
`⋯` overflow menu: Generate changelog…, Remove from workspace, Export status.

---

### Activity strip

```
┌──────────────────────────────────────────────────────────────────┐
│  ⓘ Last: Fetched 24 projects · 24 ok                      [›]    │
└──────────────────────────────────────────────────────────────────┘
```
Height: 28px. Font: 12px. Click `[›]` opens History screen.  
**States:**
- Idle: hidden (zero height)
- In progress: progress bar spans full width; text updates per-project
- Success: `ⓘ` icon, muted color; fades to lower opacity after 30s
- Failure / recovery needed: `⚠` icon, `StatusColor::Conflict` background tint

---

### Modal dialog

Centred overlay over the full window. Background dim: `rgba(0,0,0,0.5)`. Click-outside closes (via `on_close_modals`).

```
┌────────────────────────────────────────────────┐
│  [Title]                                  [✕]  │
├────────────────────────────────────────────────┤
│  [Content area — scrollable if needed]         │
├────────────────────────────────────────────────┤
│                       [Cancel]  [Primary →]    │
└────────────────────────────────────────────────┘
```
Max width: 580px. Min width: 400px. Padding: 24px.  
Corner radius: 12px. Shadow: `0 8px 32px rgba(0,0,0,0.4)`.

---

### Right-docked sheet (Resolve panel)

Slides in from the right edge. Width: ~50% of window width (`SheetSize::Half`).  
Height: 100% of window. Background: surface-elevated.  
Dim covers the left half. Click-dim closes (via `on_close_modals`).

---

### Workspace tabs

Horizontal tab strip at the top of the window. Each tab:
- Width: auto (label + badge), min 80px
- Height: 36px
- Active tab: accent underline 2px, foreground primary
- Badge: small pill `(N)` showing NeedsAttention count; hidden when 0
- `[+]` rightmost tab: opens new workspace dialog

---

### Text input

Height: 32px. Border: 1px `#3A3A3A` (dark) / `#C8C8C8` (light).  
Focus ring: 2px offset `#4C8BF5`. Padding: 0 10px.  
Error state: border `#EF5350`; error message 11px below input.  
Placeholder: muted 40% opacity.

---

### VCS badge

Inline pill next to project name.

| VCS | Label | Background (dark) | Background (light) |
|---|---|---|---|
| Git | `Git` | `#2D3A2D` | `#D4EED4` |
| jj | `jj` | `#2D2D3A` | `#D4D4EE` |

Size: 10px font, 4px vertical / 8px horizontal padding, 4px radius.

---

## 6. Interaction Patterns

### Modal open/close

- **Open:** fade-in 120ms ease-out; scale from 0.96 → 1.0
- **Close:** fade-out 80ms ease-in
- **Sheet slide-in:** translate-x from 100% → 0 over 180ms ease-out
- **Selection bar slide-up:** translate-y 48px → 0 over 150ms ease-out

### Card state transitions

- Status badge update: fade through 100ms (new status fades in while old fades out)
- Card moving between tiers: no animation in v0.18; reflow on next refresh
- Checkbox selection: immediate; no animation

### Loading / progress

- **Dashboard refresh:** each card shows `⟳ Refreshing…` inline (status badge replaced); card does not dim — stale data is visible while refresh runs
- **Modal operation streaming:** per-project row updates in real-time as results arrive; progress is a counter `(3/8 done)` in the modal header; no spinner
- **Activity strip progress bar:** thin 2px bar spanning full width; advances based on completed/total ratio

### Escape / dismiss

`Esc` dismisses in priority order:
1. Keyboard shortcut overlay
2. Command palette
3. Add project dialog
4. Active modal (Pull / Tag / Switch / Changelog)
5. Resolve sheet
6. Clears selection (if bar visible)

### Error handling UI

- **Operation failure:** result view shown in-modal; per-project outcome with recovery commands in a `code` block; "Close" returns to dashboard with card in NeedsAttention tier
- **Config read failure:** status bar error message at top; app starts with defaults
- **Repository path not found:** card in NeedsAttention tier with explicit message and "Update path…" button

### Empty states

| Situation | UI |
|---|---|
| No workspaces | Full-screen welcome: "Add your first workspace" with guided input |
| Workspace with no projects | Large "Add Project" button centred in card area |
| All projects clean | "🎉 All clean" message above collapsed Clean tier header |
| History empty | "No operations recorded yet" centred in History screen |
| Search returns nothing | "No projects match '[query]'" inline |

---

## 7. Responsive Design

knotra is a desktop application. The minimum supported window width is **800px**; the minimum height is **500px**. No mobile/tablet target.

| Window width | Layout adaptation |
|---|---|
| 800–1100px | Single-column cards; selection bar compact (icon-only buttons) |
| 1100–1400px | Single-column cards; selection bar full labels |
| >1400px | Cards may optionally use a two-column grid (future; not in v0.18) |

The Resolve panel (`SheetSize::Half`) adapts: minimum 340px, maximum 50% of window width.

---

## 8. Accessibility Requirements

**Target:** WCAG 2.1 AA.

| Requirement | Implementation |
|---|---|
| Color not sole indicator | Every status: icon + text label + color |
| Contrast ratio ≥ 4.5:1 | All `StatusColor` values verified for both themes |
| Keyboard navigation | Tab order follows visual reading order; all interactive elements reachable |
| Focus indicators | iced default focus ring; 2px offset on all interactive elements |
| Screen reader labels | All icon-only buttons have text labels; VCS badges are labelled "Git" / "jj" |
| Motion | No mandatory animation; all transitions ≤200ms; no loops |
| Error identification | Error messages state what failed, what state to expect, and how to recover |

**Keyboard coverage map:**

| Action | Shortcut |
|---|---|
| Refresh all | `Ctrl+R` / `⌘R` |
| Open Context Switch | `Ctrl+K` / `⌘K` |
| Open Freezer | `Ctrl+T` / `⌘T` |
| Focus search | `Ctrl+/` / `⌘/` |
| Close modal / clear | `Esc` |
| Toggle card selection | `Space` |
| Fetch selected | `f` |
| Pull selected | `p` |
| Tag selected | `t` |
| Switch selected | `b` |
| Show shortcuts | `?` |

---

## 9. Design Tokens

### Spacing & layout

```rust
// crates/knotra-ui/src/widget.rs
CARD_RADIUS:   f32 = 8.0    // card corner radius
CARD_GAP:      f32 = 12.0   // gap between cards
CARD_PADDING:  Padding = { top: 14, right: 16, bottom: 14, left: 16 }
SIDEBAR_WIDTH: f32 = 180.0  // (legacy; not used in current nav)
CARD_MIN_WIDTH:f32 = 240.0  // minimum card width
```

### Status colors

```rust
// crates/knotra-ui/src/theme.rs — StatusColor::to_color()

// Dark theme
Healthy:  #4CAF50   // green-500
Behind:   #FFB74D   // amber-300
Ahead:    #42A5F5   // blue-400
Dirty:    #FFB74D   // amber-300 (same as Behind)
Conflict: #EF5350   // red-400
Unknown:  #757575   // grey-600

// Light theme
Healthy:  #2E7D32   // green-800
Behind:   #E65100   // orange-900
Ahead:    #1565C0   // blue-800
Dirty:    #E65100   // orange-900 (same as Behind)
Conflict: #C62828   // red-800
Unknown:  #616161   // grey-700
```

### Attention tier colors

```
// Not in StatusColor — tier header and card accent
NeedsAttention:  #E54B4B (dark) / #B7281A (light)
Active:          #E6B450 (dark) / #B0780B (light)
Clean:           #5F6368 (dark) / #4A4A4A (light)
```

### Semantic surface tokens

```
// Derived from iced::Theme — not hardcoded; pulled via theme.extended_palette()
surface-base:      iced background
surface-card:      iced card / slightly elevated
surface-elevated:  modal / sheet background
border-default:    iced border
text-primary:      iced foreground
text-muted:        iced foreground at ~60% opacity
accent:            iced primary (#4C8BF5 dark / #1565C0 light)
```

### Typography

```
font-family:   system-ui, -apple-system, sans-serif  (iced default)
font-size-xs:  11px   (error messages, timestamps, badges)
font-size-sm:  12px   (card metadata, tier headers, activity strip)
font-size-md:  13px   (card body, modal content, buttons)
font-size-lg:  14px   (project names, modal titles)
font-weight-normal: 400
font-weight-medium: 500  (buttons, labels)
font-weight-bold:   600  (project name on card, tier headers)
```

### Animation

```
duration-fast:   80ms    (modal close, badge fade)
duration-normal: 120ms   (modal open)
duration-sheet:  180ms   (resolve panel slide-in)
duration-bar:    150ms   (selection bar slide-up)
easing-in:       ease-in
easing-out:      ease-out
```

---

## 10. Content Guidelines

**Tone:** Direct, professional, terse. knotra speaks like a senior developer
writing a commit message: no filler, no apology, no passive voice. Errors
tell you what happened and what to do.

**Character limits:**

| Field | Limit | Notes |
|---|---|---|
| Project display name | 64 chars | Truncated with `…` on card if overflows |
| Workspace name | 48 chars | Shown in tab; truncated if needed |
| Tag/freeze name | 128 chars | Validated against git tag rules |
| Tag message | 512 chars | Free text; shown in History |
| Search box | — | No limit; filters in real-time |

**Placeholder text:**

| Field | Placeholder |
|---|---|
| Project name | `My Service` |
| Repository path | `/home/user/repos/my-service` |
| Tag name | `v1.2.3` |
| Tag message | (empty — optional) |
| Search | `Search projects…` |

**Error message templates:**

| Situation | Message |
|---|---|
| Path not found | `[path] does not exist. Update the path or remove this project.` |
| Tag already exists | `Tag [name] already exists in [project]. Delete it first.` |
| No upstream | `[project] has no upstream configured. Cannot pull.` |
| Git not found | `git not found in PATH. Install Git and restart.` |
| jj not found | `jj not found in PATH. Conflict detection unavailable.` |
| Required field empty | `Name and path are required.` |

**Status label vocabulary (definitive):**

`Synced` · `Behind` · `Ahead` · `Uncommitted` · `Conflict` · `Unknown` · `Refreshing…` · `Excluded` · `Rolled back` · `Rollback failed`

Do not use: "Up to date", "Modified", "Clean" (as a displayed label), "OK", "Error" for status.

---

## 11. Technical Constraints

**Framework:** Rust 2024 edition, iced 0.14 (Elm architecture), snora 0.18.1 (layout engine).  
**VCS layer:** endringer-* 0.33.2 (reads) + VCS CLI (writes) via knotra-vcs.  
**Async runtime:** tokio (full features).

| Constraint | Value |
|---|---|
| Minimum Rust | 1.85 (tested with 1.91) |
| Target platforms | Linux (primary), macOS, Windows via winit |
| Window minimum | 800 × 500px |
| UI thread | Never blocked — all I/O via `Task`/`Subscription` |
| Max concurrent reads | Configurable; default 8 (tokio Semaphore) |
| Config location | `~/.config/knotra/config.toml` |
| History location | `~/.local/share/knotra/history/` |
| Crate structure | Workspace: knotra-app / knotra-vcs / knotra-ui |
| Hard architectural rule | knotra-app never imports gix or jj directly — all VCS access via knotra-vcs |
| i18n | English + Japanese; keys in `knotra-ui/src/i18n.rs`; add to both locales |
| No telemetry | Local-first; no external network except VCS operations |

**snora integration contract:**
- `render(AppLayout)` is the root composition function (view entry point)
- `AppLayout::dialog(Dialog)` for centred modals; `AppLayout::sheet(Sheet.at(SheetEdge::End))` for the resolve panel
- `AppLayout::on_close_modals(Message::Shortcut(ShortcutMessage::Close))` handles Esc + click-outside
- knotra-app does not implement `VcsBackend`; knotra-ui owns `KnotraTheme` — snora reads iced's theme via `extended_palette()`

---

## 12. User Flow Diagrams

### Flow A — Dashboard to bulk pull

```
[Dashboard loads]
      │
      ▼
[Status refresh runs concurrently]
      │
      ├── Projects show Refreshing… badge
      │
      ▼
[Refresh complete]
      │
      ├── Cards settle into tiers (NeedsAttention / Active / Clean)
      │
      ▼
[User selects projects]  ←── Space / click checkbox
      │
      ├── Selection bar slides up
      │
      ▼
[Press p / click Pull]
      │
      ▼
[Smart Pull modal opens]
      │
      ├── Plan computed (dirty detection)
      │     ├── Dirty projects → stash-pull-pop (overridable)
      │     └── Conflicted projects → excluded (not overridable)
      │
      ▼
[User reviews plan]
      │
      ├── [Cancel] → modal closes, selection preserved
      │
      └── [Execute Pull →]
            │
            ▼
      [Streaming execution — rows update live]
            │
            ├── All succeeded → Result view with ✓ per project
            │                   → [Close] → cards refresh in place
            │
            └── Some failed → Result view with ✗ + recovery commands
                              → [Close] → failed projects in NeedsAttention
```

---

### Flow B — Freeze a release

```
[Dashboard]
      │
[Select projects via checkbox / Shift+click range]
      │
[Press t / click Tag…]
      │
      ▼
[Tag modal opens — selection pre-filled]
      │
[Type tag name]
      │
      ├── Validation runs automatically (400ms debounce)
      │     ├── All ready → Execute button enabled
      │     └── Blocker found → Execute button disabled, blocker shown per-project
      │
[Optionally type message]
      │
[Click Execute Tag →]
      │
      ▼
[Sequential execution with rollback]
      │
      ├── All tagged → Result: ✓ per project, "Push tags now?" button
      │
      ├── Partial failure → already-applied tags rolled back
      │     → Result: rolled-back rows visible, recovery commands
      │
      └── Rollback also failed → "Rollback failed" row with manual git commands
```

---

### Flow C — Resolve a conflict

```
[Dashboard — project-alpha in NeedsAttention tier]
      │
[Card shows: "3 conflicted files" + [Resolve…] button]
      │
[Click Resolve…]
      │
      ▼
[Resolve panel slides in from right (SheetEdge::End)]
      │
      ├── File list: src/main.rs ⛔, src/config.rs ⛔, tests/lib.rs ⛔
      │
[Click "Open in editor" on src/main.rs]
      │
      ├── External editor launches (configured path)
      │   (Panel stays open)
      │
[User resolves conflict in editor, saves, returns to knotra]
      │
[Click "Mark resolved" on src/main.rs in panel]
      │
      ├── Row shows ✓
      │
[Repeat for remaining files]
      │
[Last file marked resolved]
      │
      ▼
[Panel auto-closes]
[project-alpha card moves from NeedsAttention → Active or Clean]
[Dashboard re-tiers immediately]
```

---

### Flow D — Add a new project

```
[Dashboard — empty workspace]
      │
[Large "Add Project" button visible (empty state)]
      │
[Click Add Project]
      │
      ▼
[Add Project dialog opens (centred modal)]
      │
[Type display name]
[Type or paste repository path]  ──── or ──── [Browse…] native folder picker
      │
      ├── Validation: name empty → "Name and path are required."
      ├── Validation: path not a git/jj repo → "No Git or jj repository found."
      │
[Click Add]
      │
      ▼
[Dialog closes]
[New card appears in correct tier]
[Status refresh starts for the new project]
```

---

### Flow E — Command palette discovery

```
[Any state — Dashboard, History, Settings]
      │
[Press Ctrl+K / ⌘K]
      │
      ▼
[Command palette opens (centred input overlay)]
      │
[Type "tag"]
      │
      ├── Matches: "Tag selected projects…", "Tag all projects…"
      │
[Press ↓ to navigate, Enter to select]
      │
      ▼
[Tag modal opens with current selection pre-filled]
```

```
[Type "fetch a"]
      │
      ├── Matches: "Fetch all projects"
      │
[Enter]
      │
      ▼
[Fetch runs immediately; palette closes; activity strip shows progress]
```

---

## Appendix: i18n key catalogue (v0.18.0)

Add new keys to **both** `en_strings()` and `ja_strings()` in
`crates/knotra-ui/src/i18n.rs`. Access in views via `state.t("key")`.

| Key | English | Japanese |
|---|---|---|
| `nav.dashboard` | Dashboard | ダッシュボード |
| `nav.history` | History | 履歴 |
| `nav.settings` | Settings | 設定 |
| `dashboard.refresh` | Refresh | 更新 |
| `dashboard.search_placeholder` | Search projects… | プロジェクトを検索… |
| `dashboard.add_project` | Add Project | プロジェクトを追加 |
| `status.healthy` | Synced | 同期済み |
| `status.behind` | Behind | Behind |
| `status.ahead` | Ahead | Ahead |
| `status.dirty` | Uncommitted | 未コミットあり |
| `status.conflict` | Conflict | コンフリクトあり |
| `status.unknown` | Unknown | 不明 |
| `action.fetch` | Fetch | フェッチ |
| `action.pull` | Pull | プル |
| `action.switch_context` | Switch Branch | コンテキスト切替 |
| `action.open_freezer` | Tag… | フリーザーを開く |
| `action.confirm` | Confirm | 確認 |
| `action.cancel` | Cancel | キャンセル |
| `action.close` | Close | 閉じる |
| `error.read_failed` | Failed to read repository status. | リポジトリの状態を読み込めませんでした。 |
| `error.no_repo` | No Git or jj repository found. | Git または jj リポジトリが見つかりません。 |
| `confirm.remove_project` | Remove project from workspace? | ワークスペースからプロジェクトを削除しますか？ |
