# RFC-073 handoff — Amendment A1: Settings' workspace buttons, and a cue on "Arrange"

Adds **one commit to stage 1**, on top of `8366458`. Ruling of Review 266. Stage 1 is approved
once this lands. Everything else stands. No new text.

## Why

`after/settings-workspaces.png`:

- **Five filled buttons in one area.** Every Rename and Remove, and New workspace, is a filled
  `primary` button. RFC-071 D5 allows at most one filled button per area.
- **Remove looks the same as Rename,** though one deletes a workspace from knotra. The
  confirmation window already asks, and its own Remove is `danger`.

`after/arrange-menu-open.png`: "Arrange" is a plain `ghost` word. Nothing says that it opens
something until it is pressed. The workspace menu, a similar control, shows a chevron.

## The commit

1. **Settings → Workspaces:**
   - Rename, Remove and New workspace use `style::secondary` (outlined), through
     `with_focus_ring` like every other button.
   - No filled button in this section. The emphasis on removing stays where it is: in the
     confirmation window's `danger` button.
   - Report which `style::` each control used before.
2. **"Arrange":**
   - A `chevron_down` icon follows the label when the panel is closed, and `chevron_up` when it
     is open. Use the same icon helper and size as the workspace switcher's chevron.
   - The label, its focus key and its message do not change.
3. **Tests:** none new required. The existing focus and guard tests must pass unchanged. If
   any changes, report why.

## Evidence

**The question:** does Settings show no filled button among the workspaces, and does
"Arrange" show that it opens?

- Settings → Workspaces with two workspaces;
- the dashboard with Arrange closed, and with it open;
- English, after only. The before images are stage 1's.

Save to `.git-exclude/evidence/rfc-073/stage-1-a1/`.

## Report

- the diff;
- the styles before and after;
- the captures, each named;
- gates, with `git diff --check 8366458..HEAD`.

**Note on the gate:** `cargo deny check advisories` passes again from `c2fc042`. `deny.toml`
now checks only the three targets knotra ships. RUSTSEC-2026-0441 (`pin-utils`) reached the
graph only through a wasm32-only dependency of iced (Review 266 §3). Rebase on it.
