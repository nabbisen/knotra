# RFC-036 - Keyboard Navigation and Focus Traversal

| Field | Value |
|---|---|
| Status | Implemented (main: d20c7be) |
| Priority | High - blocks RFC-035 R22/R23, and RFC-033 D3's focus trap/return for every overlay |
| Effort | Large |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-ui/src/widget/focus.rs`, `crates/knotra-ui/src/widget/buttons.rs`, `crates/knotra-ui/src/widget/overlay.rs`, `crates/knotra-app/src/view.rs`, `crates/knotra-app/src/view/dashboard.rs`, `crates/knotra-app/src/view/workspace_manager.rs` |
| Related RFCs | `rfcs/done/033-ui-ux-foundation-shell-and-overlay-contracts.md` (D3 focus entry/trap/return, D7 `FocusTokens`), `rfcs/done/034-design-foundation-shell-and-overlay-host.md` (overlay host this reconciles with), `rfcs/proposed/035-dashboard-and-selection-migration.md` (R22/R23 depend on this; card arrow-navigation and Enter-to-open are that RFC's own scope, not this one's), `rfcs/done/0016-keyboard-shortcuts.md` (the "complete keyboard scheme" this closes the gap in) |
| Related audit evidence | `.git-exclude/reviewed/073-tab-navigation-gap-and-light-theme-resolution-review.md`, `.git-exclude/reviewed/074-rfc-036-draft-plan-review.md` |

## Implementation Record

The first six-stage RFC in the project. Each hash resolves on `main`:

| Stage | Commit | Delivered |
|---|---|---|
| 1 | `1a9d481` | Focus infrastructure and reconciliation |
| 2 | `21609b8` | Visible focus ring, shell focus order |
| 3 | `e2d2d6c` | Overlay focus trap, entry, and return |
| 4 | `a7c354c` | Dashboard rows, bare `/`, D8 evidence |
| 4a | `429ca4a` | `Ctrl+/` regression test held under the Stage 4 commit freeze |
| 5 | `347f429` | Dialog focus rings |
| 6 | `d20c7be` | High-contrast focus ring on filled controls |

## Summary

knotra has no keyboard focus traversal. Tab does nothing; there is no visible
focus indication anywhere; overlays neither trap nor return focus. This is not
a small gap: `iced` 0.14, the version knotra depends on, does not supply
focus-cycling or focus-aware styling for anything except `text_input` and
`text_editor`. `button` - the widget nearly every knotra control is built on -
has no concept of being focused at all.

This RFC's central decision is therefore **who owns focus**: knotra must build
and own a focus model of its own for non-text-input widgets, because iced does
not provide one. Everything else - Tab/Shift-Tab order, the visible ring,
overlay trap and return, bare `/` for search - is derived from that decision
and from how it reconciles with the focus iced already manages for text
inputs.

**What this RFC does not cover:** dashboard card-to-card arrow-key movement
and Enter-to-open. Those are dashboard-specific interactions that belong to
`rfcs/proposed/035-...md`, which already owns the dashboard and already
carries the requirement (R22) that depends on the mechanism built here.
Bundling them here would put dashboard-specific interaction inside a
cross-cutting RFC - the same scope error in the other direction.

## Background

### The finding that produced this RFC

`.git-exclude/reviewed/073-...md` found, during an unrelated keyboard-evidence
tooling spike, that Tab does nothing in knotra. Confirmed two ways:

- **Source.** `crates/knotra-app/src/app.rs`'s keyboard subscription maps
  exactly five bindings - `Escape`, `Ctrl/Cmd+R`, `Ctrl/Cmd+K`, `Ctrl/Cmd+T`,
  `Ctrl/Cmd+/` - and nothing else. `grep -rn 'Named::' crates/knotra-app/src/`
  returns one line, `Named::Escape`. `grep -rn 'focus_next|focus_previous|focus_cycle'
  crates/` returns nothing.
- **Behaviour.** Typing `x`, pressing Tab, then typing `y` into an
  auto-focused dialog field produced `"xy"` in that same field. Had Tab moved
  focus to a button, `y` could not have landed there - buttons do not accept
  character input.

### `rfcs/done/0016-keyboard-shortcuts.md` specified more than exists

RFC-0016 described "the complete keyboard scheme" for knotra. Comparing it
against what is actually implemented:

| RFC-0016 specified | Implemented today |
|---|---|
| `↑` / `↓` / `j` / `k` - move focus between cards | no |
| `Enter` - open the detail panel for the focused card | no |
| `/` - focus the search input | only as `Ctrl+/` |
| Tab order / focus traversal | no |

`ROADMAP.md:81` currently states `[x] Full keyboard navigation (tab order,
focus visibility)`. That line is false. It is corrected as part of this RFC's
*implementation* landing, not here - editing it now would leave a bare
retraction with nothing built yet to replace it with, per `073` H4.

### The decisive fact about iced 0.14, verified from source

| Fact | Source |
|---|---|
| Only `text_input.rs` and `text_editor.rs` implement `Focusable` | `grep -rln 'Focusable' iced_widget-0.14.2/src/` |
| `button.rs` contains **zero** occurrences of `Focusable` | same |
| `button::Status` = `Active \| Hovered \| Pressed \| Disabled` - no `Focused` variant | `iced_widget-0.14.2/src/button.rs:465-480` |
| `text_input::Status` **does** carry `Focused { is_hovered: bool }` | `iced_widget-0.14.2/src/text_input.rs:1690-1710` |
| `focus_next()` / `focus_previous()` exist | `iced_runtime-0.14/src/widget/operation.rs:50,55` |

`focus_next()`/`focus_previous()` are real, but they only cycle iced's
*focusable* set - which is text inputs and text editors, nothing else. A
button's style closure receives `Status`, which has no way to say "I am
focused." There is no version of "just call `focus_next()` on Tab" that
delivers what knotra needs, because the set it cycles excludes every button,
chip, checkbox, section header, and row action in the application.

`snora_design::FocusTokens` (D7) is a data type - ring width, ring offset,
ring colour - not a rendering mechanism. Its own doc comment states the
limitation plainly: *"in iced 0.14, standard `button`/`container` styling does
not expose focus state, so these tokens apply only where the widget surface
allows it."* Nothing upstream closes this gap; knotra has to.

### What already exists, and is one half of a hazard

`crates/knotra-ui/src/widget/focus.rs`'s `focus_input()` already calls
`iced::widget::operation::focus(id)` to give a dialog's first text field real
iced focus when the dialog opens (RFC-034 D3 item 4, partially satisfied for
text fields only). This is genuine, working iced-owned focus. It is not
removed or replaced by this RFC - but it means iced-owned focus already
exists in the application today, and whatever knotra builds for buttons has
to coexist with it correctly, not quietly conflict with it.

## Motivation

- **N-8** (keyboard-complete navigation) is a stated non-functional
  requirement of the Production Readiness Reset and is unmet across the whole
  application, not one screen.
- **RFC-033 D3** requires focus entry, trap, and return for every overlay.
  Entry is partially built (text fields only); trap and return do not exist
  at all, because there is no focus model to trap or return within.
- **RFC-035 R22/R23** cannot be satisfied or evidenced without this RFC -
  amended in the same pass as this draft to state that dependency explicitly.
- **RFC-037/038/039** (mutating overlays, settings/history, per-project
  history - renumbered from RFC-033's original 036/037/038 sequence; see
  Numbering below) will each need the same focus infrastructure. Building it
  once, here, is cheaper than each later RFC solving it ad hoc.

## Numbering note

`rfcs/done/033-...md`'s migration-sequence table and Deferred Follow-ups list
name "RFC-036" as the mutating-workflow-overlays RFC. This RFC's drafting
takes that number instead, for keyboard navigation, because the Tab-navigation
gap was discovered after RFC-033 was accepted and must land before RFC-035's
evidence pass - earlier than the overlay work RFC-033 anticipated at 036.

The planned sequence shifts: mutating workflow overlays becomes **RFC-037**,
settings and history **RFC-038**, per-project VCS history **RFC-039**. This is
recorded in `ROADMAP.md`'s UI/UX track and here; `rfcs/done/033-...md` itself
is **not** edited - it is an accepted document, and editing it to match a
later renumbering is exactly what the project's lifecycle policy forbids.
Only `rfcs/proposed/035-...md` existed on disk among the shifted numbers, and
it has been amended (this same pass) to use the new numbers.

## Decision

### D1. knotra owns a focus model of its own; iced owns focus only for text inputs

**Decision.** Introduce a knotra-owned notion of "focused control" -
`FocusTarget`, an ordered identifier naming any focusable element in the
current view (buttons, chips, checkboxes, section-header disclosures, row
actions, and text inputs alike). State holds the current target. Tab/Shift-Tab
advance it through a view-declared order. Every control's style closure
receives an explicit `is_focused: bool`, computed by comparing its own
`FocusTarget` against the current one, and renders the `FocusTokens` ring
itself when true - not through `Status`, which cannot express it.

**Text inputs are a special case, not an exception.** A text input is both a
`FocusTarget` in knotra's model *and* an iced-focusable widget. When
knotra-focus lands on a text input's target, this RFC's mechanism issues
`operation::focus(id)` (the same primitive `focus_input()` already uses) so
iced's own focus - and therefore keyboard character delivery - moves with it.
**knotra-focus is authoritative; iced-focus for text inputs is always kept in
lockstep with it, never set independently.** Nothing outside this mechanism
may call `operation::focus`/`focus_input()` without also updating
knotra-focus to match, because the two diverging is the exact hazard below.

**The hazard this closes.** If knotra-focus could point at a button while
iced-focus remained inside a text field (e.g. because something updated one
without the other), typed characters would go to the field while the visible
ring sat on the button - the `"xy"` symptom the spike found, in reverse and
harder to diagnose, since the ring would visibly disagree with where input
actually goes. Keeping knotra-focus as the single source of truth, with
text-input iced-focus as a derived effect of it, prevents the two models from
ever being asked to agree independently.

**Alternative rejected: wait for/patch iced to add button focus.** Rejected
because it blocks the whole Production Readiness Reset on an upstream change
outside this project's control, with no committed timeline, for a widget
(`button`) that is load-bearing for nearly every control in the application.

**Alternative rejected: use `focus_next()`/`focus_previous()` as-is and accept
that Tab only cycles text inputs.** Rejected because it satisfies none of
N-8, RFC-033 D3, or RFC-035 R22 - a user could Tab between the two or three
text fields in a dialog and reach no button, chip, checkbox, or row action by
keyboard at all. This is not a partial win; it is close to the status quo.

**Alternative rejected: give every widget a synthetic `Focusable` wrapper
that reimplements iced's operation protocol.** Considered and rejected as
materially more work than an application-level `FocusTarget` enum/vec with a
comparison in each style closure, for a benefit (interoperating with
`focus_next()`'s existing cycling) that does not matter once knotra owns
Tab handling directly rather than delegating to it.

**Consequence for effort.** This is why Effort is Large rather than the
Medium-Large first assumed: the plan that preceded this document assumed
iced supplied traversal and only visible-ring rendering needed design. It
does not, so a focus-order data structure, an authoritative reconciliation
rule with iced's own focus, and per-widget `is_focused` plumbing must all be
built.

## Requirements

### Functional - traversal

R1. Tab moves knotra-focus to the next `FocusTarget` in the current view's
declared order; Shift-Tab moves to the previous. The order is stable and
matches visual reading order (top-to-bottom, left-to-right) for every screen
and overlay this RFC touches.

R2. Every interactive control on the dashboard toolbar, the dashboard rows
(section headers, checkboxes, row actions - not card-to-card movement, which
is RFC-035's), the shell (workspace switcher, Dashboard/History destinations,
refresh, palette, Settings), and every overlay migrated under RFC-034 (the
workspace-manager dialogs) is reachable via Tab/Shift-Tab.

R3. Enter or Space activates the control currently holding knotra-focus,
dispatching exactly the `Message` a pointer click on that control would.

R3a. **Activation keys are gated on text-input focus, on the same rule as R4.**
When knotra-focus holds a text input, Space types a literal space and must not
activate anything; Enter is delivered to the input (submitting only where that
input's own handler already defines a submit action) and must not activate an
unrelated control. Only when knotra-focus is on a non-text-input target do
Enter and Space act as activation keys. Without this, the workspace-name field
becomes unable to accept a space character - an immediately visible
regression.

R4. Bare `/` (no modifier) focuses the dashboard search field, per RFC-0016's
original specification, when no text input currently holds focus (so `/`
inside an already-focused text field types a literal `/`, not a shortcut).
`Ctrl+/` (RFC-0016's implemented binding) continues to work identically.

### Functional - overlay trap and return (D3)

R5. While any overlay (`AppLayout::dialog`, `AppLayout::sheet`, or
`AppLayout::header_menu`) is open, Tab/Shift-Tab traversal is confined to
that overlay's `FocusTarget`s. Focus cannot Tab into content beneath it.

R6. On overlay open, knotra-focus enters at the first meaningful control
(matching D3 item 4's existing text-input behaviour, generalized to overlays
whose first control is not a text input).

R7. On overlay close - by any of the three phase-aware close routes
(Escape, scrim click, header close control) - knotra-focus returns to the
control that opened the overlay. `close_topmost_layer`'s existing
phase-aware ordering (RFC-029/031 lease semantics) is not modified; this RFC
adds focus return alongside it, not instead of it.

### Functional - visible indication

R8. The currently knotra-focused control renders a visible ring using
`snora_design::FocusTokens`, re-exported through `knotra_ui::widget` (R11)
following the `Tokens` precedent - a consistent 2px high-contrast ring in
both themes, per RFC-033 D7. The ring is drawn by the application passing
`is_focused` explicitly; it does not depend on `Status::Focused`, which does
not exist for the widgets that need it.

R9. Focus is never lost silently: if the control currently holding
knotra-focus is removed from the view (e.g. a filtered-out row, a closed
overlay), knotra-focus falls back to a well-defined target (the first
`FocusTarget` in the current order) rather than pointing at nothing.

### Non-functional

R10. **No regression to `Named::Escape`, existing `Ctrl+`-modified shortcuts,
or `close_topmost_layer`'s phase-aware ordering.** That function is not
rewritten by this RFC; focus return is added as a step invoked alongside its
existing branches, from outside, the same pattern `handle_shortcut` already
uses for the workspace switcher.

R11. `FocusTokens` is re-exported through `knotra_ui::widget`, satisfying R2
of RFC-034 (application view code imports from `knotra_ui`, never
`snora::design` directly). `grep -rn 'snora::design' crates/knotra-app/src/`
continues to return zero.

R12. Text-input focus and knotra-focus never diverge: any code path that
moves knotra-focus onto a text-input `FocusTarget` also issues
`operation::focus` for that input's `Id` in the same `Task`, and vice versa -
no path sets one without the other.

R13. D8 keyboard evidence: focus order, the visible ring in both themes, and
overlay trap/return are captured per `.git-exclude/reference/002-keyboard-evidence-runbook.md`'s
`xdotool windowfocus --sync` method for the shell, the dashboard toolbar, one
dashboard row, and one migrated overlay (a workspace-manager dialog).

R14. `ROADMAP.md:81`'s `[x] Full keyboard navigation (tab order, focus
visibility)` line is corrected to reflect what this RFC actually delivers,
**at implementation time**, not during drafting.

R15. All new or changed strings (any new accessible-name or tooltip text
this RFC's focus indication requires) are localized in English and Japanese.

R16. Existing gates pass: `fmt --all --check`,
`clippy --workspace --all-targets -- -D warnings`, and the three test suites.

## Goals

- Make Tab/Shift-Tab a complete, visible, predictable way to operate every
  control this RFC's scope covers, without a pointer.
- Reconcile knotra-owned and iced-owned focus into one authoritative model
  rather than two that can disagree.
- Give RFC-033 D3's overlay trap and return an actual mechanism to run on.
- Leave dashboard-specific interaction (card arrow-movement, Enter-to-open)
  to RFC-035, which owns that screen.

## Non-goals

- `↑` / `↓` / `j` / `k` card-to-card movement and `Enter`-to-open the detail
  panel - RFC-035's scope, built on this RFC's focus model.
- Migrating the mutating workflow overlays or any other ad hoc overlay layer
  (RFC-037).
- Migrating Settings or History bodies (RFC-038).
- Re-litigating RFC-034's `current_or` pattern or D4's control vocabulary.
- Any change to `state/dashboard.rs`'s display pipeline (RFC-032's, unrelated
  to this RFC).
- Mouse/pointer interaction changes of any kind.
- Animation, motion, or theme customization.

## External Design

### Focus order, shell + dashboard toolbar (standard width)

```text
Tab sequence (visual, top-to-bottom / left-to-right):
  1. Workspace switcher            [shell, left]
  2. Dashboard destination         [shell, centre-left]
  3. History destination           [shell, centre-left]
  4. Refresh                       [shell, right]
  5. Command palette               [shell, right]
  6. Settings                      [shell, right]
  7. Status filter chips           [toolbar, left-to-right]
  8. Group select                  [toolbar]
  9. Sort select                   [toolbar]
 10. Search field                  [toolbar]  ← also reachable via bare `/`
 11. Select (selection mode)       [toolbar, right]
 12. First row's section header / first row's checkbox / first row's action
```

Shift-Tab reverses this exactly. The visible ring (2px, high-contrast, both
themes) appears on whichever of these currently holds knotra-focus.

### Overlay trap (a workspace-manager dialog)

```text
┌─ Create workspace ──────────────────── [x] ┐  ← header close, in the order
│                                             │
│  Name  [ my-workspace            ]         │  ← focus enters here (R6)
│                                             │
│              [ Cancel ]  [ Create ]        │
└─────────────────────────────────────────────┘
  Tab from [Create] wraps back to [x], not to
  anything beneath the scrim (R5). Closing by
  any route returns focus to the button that
  opened this dialog (R7).
```

## Internal Design

### `FocusTarget` and where it lives

A `FocusTarget` is a small, view-scoped identifier - concretely, an ordered
index or stable key naming a position in the current screen/overlay's
declared focus order (the exact representation, e.g. an enum per screen
versus a flat `Vec<Id>` built at view time, is a decision for the developer
handoff to work through with a spike; both satisfy R1-R9). It lives in
`AppState`, alongside a per-context field such as `dashboard_focus` and
`overlay_focus` (an overlay's focus order is scoped to that overlay per R5,
so it needs its own slot, not the dashboard's). It is presentation state, not
domain state - it does not enter `AppConfig` or get persisted.

### Traversal and activation

Tab/Shift-Tab are added as new `Named` arms in `app.rs`'s existing keyboard
subscription (alongside `Named::Escape`), advancing the current context's
`FocusTarget` by one position, with wraparound. Enter/Space activation reads
the current `FocusTarget` and dispatches whatever `Message` a pointer click
on that same control would produce - the same message, not a parallel path,
so behaviour cannot diverge between pointer and keyboard.

**R3 constrains the representation, and this is not a free choice.** A bare
index or key held in `AppState` cannot satisfy R3 on its own, because nothing
maps position 3 back to the `Message` that control would dispatch. The focus
order must therefore be built **at view time, carrying each target's
activation message alongside it** - concretely a `Vec<(FocusTarget, Option<Message>)>`
or equivalent, produced by the same view code that renders the controls, so a
control and its keyboard activation cannot drift apart. `AppState` holds only
the current *position* within that order; the order itself is derived per
frame from the view, exactly as `DashboardDisplay` is (RFC-032).

`Option<Message>` rather than `Message`: a disabled control is still a
traversal stop (the user must be able to Tab to it and see why it is
unavailable, per RFC-033 D6), but activating it must do nothing.

### Reconciling with iced's text-input focus (R12)

Whenever a `FocusTarget` transition lands on (or leaves) a text input, the
same `Task` that updates knotra-focus also calls
`knotra_ui::widget::focus_input()` (existing) for that input's `Id`. This is
additive to `focus_input()`'s current call sites (dialog-open auto-focus) -
those continue to work exactly as today; this RFC adds the reverse direction
(Tab-driven arrival at a text input) through the same primitive, so there is
exactly one function that ever calls `operation::focus`.

### Visible ring (R8)

`knotra-ui`'s button/chip/checkbox/select style functions gain an
`is_focused: bool` parameter (or an additional wrapper, following the
`current_or` precedent of feeding a fixed input rather than reading it from
`Status`) and draw the `FocusTokens` ring when true, composed with whatever
the underlying `Status`-driven style already produces. `FocusTokens` is
re-exported from `knotra_ui::widget` (R11), following `Tokens`'s existing
re-export pattern in `widget/mod.rs`.

> **Correction to review `080` (recorded per review `083` Finding 1).** Review
> `080` stated that only `primary` controls collided with the focus ring and that
> `danger` "takes a light-blue ring with strong contrast." That was wrong.
> Measured WCAG ratios are `ring_color` vs `danger` = **1.27:1** (dark) and
> **1.03:1** (light) — as poor as `primary`'s 1.27:1 and 1.00:1. WCAG contrast is
> a luminance ratio and ignores hue, so the dark preset's red `danger` and blue
> `accent`, which look very different, have almost identical luminance. Stage 6's
> background-driven mechanism therefore corrected `danger` as well as `primary`.
> `.git-exclude/reviewed/` artifacts are immutable, so `080` is not amended; the
> correction lives in `083` and here.

### Overlay trap and return (R5-R7)

The overlay host (`AppLayout::dialog`/`sheet`/`header_menu`) already
constrains *rendering* to one slot; this RFC adds a *focus* constraint next
to it - when an overlay's `FocusTarget` context is active, Tab/Shift-Tab
operate only within it. Return-to-opener (R7) is implemented as a value
captured at the moment the overlay opens (which control had knotra-focus) and
restored when it closes, invoked from the same phase-aware close paths
`close_topmost_layer` and `handle_shortcut`'s switcher branch already use -
added as a step around those functions, not a rewrite of them (R10).

### Bare `/` (R4)

Added to the keyboard subscription's match arm, gated on no text input
currently holding knotra-focus, so it does not shadow typing a literal `/`
into an already-focused field. `Ctrl+/`'s existing binding is unchanged.

## Security Considerations

**Focus trap must never produce an inescapable state.** Every overlay this
RFC touches already has Escape, scrim click, or a header close control per
RFC-034 D3/R8.4; this RFC's trap (R5) confines Tab within an overlay but does
not remove or gate any of those three existing exits. No new command
construction, lease acquisition, or VCS task is introduced; this is
presentation and input-routing only.

## Test Plan

### Unit tests

- Tab from the last `FocusTarget` in an order wraps to the first; Shift-Tab
  from the first wraps to the last.
- Enter/Space on the current `FocusTarget` dispatches the same `Message` a
  pointer click on that control would.
- **Space with knotra-focus on a text input types a space and activates
  nothing; Enter with focus on a text input does not activate an unrelated
  control (R3a).** This is the gating twin of the bare-`/` test below.
- A disabled control is still a Tab stop but activating it dispatches nothing
  (the `Option<Message>` case in the view-built order).
- Opening an overlay sets knotra-focus to its first control (R6); closing it
  by each of Escape, scrim click, and header close returns focus to the
  opener (R7), and `close_topmost_layer`'s existing phase-aware branch order
  is unchanged by a source diff, not just by behaviour.
- Tab from an overlay's last control does not move focus to anything outside
  the overlay (R5).
- A `FocusTarget` transition onto or off of a text input issues
  `operation::focus` for that input's `Id` in the same `Task` (R12) -
  regression-tests the reconciliation rule directly, since silent divergence
  is exactly the failure mode D1 exists to prevent.
- Bare `/` focuses search when no text input holds focus, and types a literal
  `/` when one does.

### Behavioural / evidence (D8)

Per `.git-exclude/reference/002-keyboard-evidence-runbook.md`'s
`xdotool windowfocus --sync` + bare `xdotool key` method:

- **The spike's own regression check**: the `"xy"` test, repeated - typing
  `x`, Tab, typing `y` into a dialog's Name field must now show focus move
  off the field (character `y` does not append to the same field, or is not
  typed at all if focus lands on a button) rather than reproducing `"xy"`.
- Tab/Shift-Tab order captured across the shell, dashboard toolbar, and one
  row, light and dark, English and Japanese.
- Visible focus ring captured on a button, a chip, a select menu, and a
  checkbox, light and dark.
- Overlay trap and return captured for one workspace-manager dialog: Tab
  wraps within it; closing by each of the three routes returns focus to the
  opener.
- Bare `/` focusing search captured once; typing `/` into an already-focused
  field captured once, to show both branches of R4.

### Commands

```sh
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
env TMPDIR="$PWD/.git-exclude/tmp" \
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
  GIT_EDITOR=true VISUAL=true EDITOR=true \
  cargo +1.91 test -p knotra-vcs
git diff --check
grep -rn 'snora::design' crates/knotra-app/src/    # must return nothing
grep -rn 'FocusTokens' crates/knotra-ui/src/widget/mod.rs   # must show the re-export
```

## Acceptance Criteria

- [ ] Tab/Shift-Tab traverses every control in scope (R1-R2), matching visual
      reading order.
- [ ] Enter/Space activates the currently focused control identically to a
      pointer click (R3).
- [ ] Space types a literal space, and Enter does not activate an unrelated
      control, while knotra-focus holds a text input (R3a).
- [ ] Bare `/` focuses search when no text input holds focus; `Ctrl+/` still
      works (R4).
- [ ] Tab is confined within an open overlay (R5); focus enters at the first
      control (R6) and returns to the opener on close by all three routes (R7).
- [ ] A visible 2px focus ring renders on the current target in both themes
      (R8), using `FocusTokens` re-exported through `knotra_ui::widget` (R11).
- [ ] Focus never points at a removed control; it falls back to the first
      target in the order (R9).
- [ ] `close_topmost_layer` and its phase-aware ordering are unmodified by
      source diff (R10).
- [ ] Every knotra-focus transition onto or off a text input keeps iced's own
      text-input focus in lockstep (R12) - verified by the regression test.
- [ ] The spike's `"xy"` behavioural test now shows Tab moving focus, not
      typing through it.
- [ ] D8 evidence supplied per R13.
- [ ] `ROADMAP.md:81` corrected at implementation time (R14), not before.
- [ ] New strings localized in both catalogs (R15).
- [ ] All gates pass with observed output (R16).

## Developer Handoff

Staged so each leaves a green tree; do not start a stage before the prior one
lands.

### Stage 1 - focus-order infrastructure and reconciliation

Introduce `FocusTarget` and its per-context state fields. Wire Tab/Shift-Tab
and Enter/Space in the keyboard subscription. Implement the text-input
reconciliation rule (R12) and its regression test **before** anything else
consumes it - the same discipline RFC-034 used for its contrast test.

Nothing renders differently yet; this stage is inert without Stage 2.

### Stage 2 - visible ring

Add `is_focused` to `knotra-ui`'s style functions, re-export `FocusTokens`,
and render the ring for the shell. This is the first stage a screenshot can
evidence.

**Amended after Stage 2 (review `077`):** this stage originally also named the
dashboard toolbar. That was an error in this RFC, not an unmet requirement.
`filter_button`/`choice_button` carry no `.style()` at all, so ringing them
would mean styling them for the first time - and *which* styling is RFC-035
R1/R2's decision (`chip::filter`, select menus), on controls RFC-035 replaces
rather than restyles. This RFC's Stage 4 covers dashboard **rows**, not the
toolbar. The toolbar receives its ring from RFC-035. Recorded here rather than
silently dropped, so the amendment is auditable.

### Stage 3 - overlay trap and return

Extend the mechanism to overlay contexts (R5-R7), invoked alongside
`close_topmost_layer`'s existing branches and the switcher's own close path -
without modifying either function's source.

### Stage 4 - bare `/`, dashboard rows, remaining shell/toolbar controls, D8 evidence

Add bare `/` (R4). Extend focus order to dashboard rows' section headers,
checkboxes, and row actions (not card-to-card movement - RFC-035's). Capture
full D8 evidence. Correct `ROADMAP.md:81` (R14) in this stage, once the
capability it describes actually exists.

### Guardrails

1. **Do not modify `close_topmost_layer`'s existing branch logic or
   ordering.** Focus return is added as a step invoked around it, per R10 -
   the same pattern already used for the workspace switcher.
2. **Do not let any code path move knotra-focus onto/off a text input
   without also updating iced's own focus for it (R12).** This is the one
   invariant this entire RFC exists to protect; a single path that skips it
   reproduces the hazard D1 describes.
3. **Do not build `↑`/`↓`/`j`/`k` card movement or Enter-to-open here.**
   RFC-035's, once this RFC's mechanism exists for it to build on.
4. **`grep -rn 'snora::design' crates/knotra-app/src/` must return zero** at
   every stage boundary.
5. **Never modify `crates/knotra-vcs`.**
6. **Do not edit `ROADMAP.md:81` before Stage 4.**

### Leave alone

- `rfcs/done/033-...md` and every other `done/` RFC.
- `state/dashboard.rs`'s display pipeline (RFC-032's).
- `current_or` and RFC-034's D4 control-vocabulary decisions.
- Card-to-card movement and Enter-to-open (RFC-035's).
- The mutating workflow overlays and Settings/History bodies (RFC-037/038).

## Open Questions

1. **Exact `FocusTarget` key type.** The *shape* is settled by R3 (see
   Internal Design): a view-built order carrying each target's activation
   message, with `AppState` holding only the current position. What remains
   open is the key itself - a per-screen enum versus a stable string/`Id` -
   and the developer should pick based on which keeps the order colocated
   with the view that declares it, then report the choice, since RFC-037/038
   inherit it.

2. **Does overlay focus context need its own `AppState` field, or can it
   reuse the dashboard's with a stack discipline?** R5's confinement needs
   *some* separation between "dashboard order" and "this overlay's order";
   the simplest correct shape is a decision to make during Stage 1, not here.

3. **Should the ring composition (Stage 2) be a wrapper style function or a
   parameter threaded through the existing ones?** Either satisfies R8; pick
   based on which produces less call-site churn across the shell and
   toolbar, and record the choice for RFC-037/038 to follow.

## Deferred Follow-ups

- RFC-035: dashboard card-to-card arrow-key movement and Enter-to-open,
  built on this RFC's focus model.
- RFC-037: mutating workflow overlays and the remaining ad hoc layers, using
  this RFC's overlay trap/return mechanism.
- RFC-038: settings form grid and history record-list pattern, same
  dependency.
- RFC-039: per-project VCS history.
