# RFC-075 - One keyboard model: snora's

| Field | Value |
|---|---|
| Status | Implemented (main: 6373d73, 1144026, 279c0ee, c913ffa, 8ed8815, fb0282f, e534683, c97883b, 43c6646, 3d73e79, e5c1ce2), with Amendment A1 and handoff amendments across stages 2-4 |
| Priority | High - Settings and History cannot be used from the keyboard at all, and a select cannot be opened from it anywhere |
| Effort | Large - every control knotra draws, every window's focus, five stages |
| Target | 0.35.0 (after 0.34.0, which ships snora 0.65 through Task 112) |
| Related files | `crates/knotra-app/src/app/focus_ops.rs`, `crates/knotra-app/src/state/focus.rs`, `crates/knotra-app/src/app.rs` (the key subscription), every `crates/knotra-app/src/view/**` module, `crates/knotra-ui/src/widget/{buttons,chip,select,checkbox,field,focus,ring}.rs` |
| Decided by | project owner, 2026-11-18 - the direction (adopt snora's model, not extend knotra's own); then accepted, with D4 a-e as recommended, and D4 d to be decided by comparing both behaviours in stage 0 |
| Related | RFC-033 D3 item 4 (the contract: focus entry, trap, return); RFC-036 (knotra's own focus model); RFC-066 (keyboard use stays inside the open window); RFC-067; Task 095; Task 111; snora 0.55-0.65 (RFC-116, -119, -120, -121, -122, -127, -134) |

## Summary

knotra has had its own keyboard model since RFC-036, when snora had none. It is a declared
focus order per window, knotra's own focus target, and knotra's own ring. It covers the header,
the dashboard and every dialog and sheet. It does not cover the Settings and History screens.
And where it reaches a select, Enter cannot open it, because iced 0.14's `pick_list` has no
keyboard route.

snora now has a complete keyboard model:

- Tab through `snora::keyboard::traverse`;
- every control on `snora::control`, with a ring, a minimum target and Enter and Space;
- keyboard-reachable checkboxes, selects and sliders (0.65);
- an id to choose where focus starts (0.61);
- overlays that keep the keyboard inside them (0.55).

After this RFC, knotra uses that model alone, and its own model retires. Every screen and every
control can be used from the keyboard, by construction. RFC-066's rules stay the acceptance
tests, except where the owner decides otherwise below.

## Problem

Measured at `605a4a6` (Review 282 §3).

1. **Two screens have no keyboard order.**
   - `shell_and_dashboard_focus_order` (`focus_ops.rs:155`) adds the dashboard's order only on
     the Dashboard screen.
   - On Settings and History, Tab moves through the header and nothing else.
   - In Settings that leaves out: language, theme, zoom, the four choices of RFC-073 D12, the
     tool fields, the workspace buttons and "Advanced".
   - In History it leaves out "Show details" and "Copy log".
2. **A select cannot be opened from the keyboard,** anywhere.
   - iced 0.14's `pick_list`, `checkbox`, `toggler` and `slider` register no focus (snora's
     keyboard guide, "Form controls", measured against iced's source).
   - knotra's selects and checkboxes are iced's.
3. **Two keyboard systems would meet in one window.** Every snora control knotra adopts would
   answer snora's Tab while knotra's answers its own. The owner's philosophy rules that out,
   so the choice is one model or the other, and the owner has chosen snora's.

## Design

### D1. One model

- **Tab and Shift+Tab** go through `snora::keyboard::traverse`, from knotra's one key
  subscription.
- **Every control knotra draws** is a snora control:
  - buttons: `snora::design::control` with the `style::button::*` functions knotra already
    uses, so each can take an id;
  - chips: `snora::design::control` with `style::chip::*`;
  - name links and row actions: the same;
  - text fields stay iced's `text_input`, which is a Tab stop already.
- **Form controls:**
  - selects become `snora::design::select`;
  - checkboxes become `snora::design::checkbox`, including stage 4's file-change box (RFC-073
    A5 §5).
- **Retired:** `overlay_focus`, `FocusOrder`, `LayerFocus.order`, knotra's ring helper, and
  `focus_visible`.
- **Kept:** `Layer` and `top_layer` (RFC-066 D1), which still decide Escape, shortcuts and
  entries.

### D2. Order is reading order, by construction

- snora's Tab follows the view tree, start to end.
- RFC-066 D2 already required each window's order to be reading order, so the declared orders
  become the layout itself.
- **Acceptance:** a test per window walks Tab from its entry and asserts the sequence RFC-066's
  handoff tables list. Each difference is either a layout fix or an owner decision (D4).

### D3. Entries by id, decided in one place

- RFC-066 D2's entry rule stands. A control that starts a change is never the entry.
- RFC-066 D3's single place stands: one function, after every update, compares the top layer
  before and after.
- What changes is the mechanism. That function now returns `iced::widget::operation::focus(id)`
  for the entry's id (snora 0.61), in place of setting knotra's own target.

### D4. Where knotra's accepted rules and snora's differ

The owner decided these on 2026-11-18: each as recommended. **d** waits for a comparison of both behaviours in stage 0; until then the recommendation is not taken.

| # | knotra today (accepted) | snora | Recommended |
|---|---|---|---|
| a | **Tab wraps** inside a window (RFC-066 D5) | Tab bounded to the overlay; "past the last, focus leaves the group: it does not wrap" | **Stage 0 measures** what "leaves" means inside a modal. If focus goes nowhere, keep wrapping by routing the key; otherwise take snora's |
| b | **Disabled controls are Tab stops** (RFC-036 R3) | a disabled control is no stop | **snora's.** The common convention. The reason for an unavailable control is printed beside it (RFC-073), so it need not be focused to be read |
| c | **No ring after a window opens by mouse;** Enter then shows the ring before pressing (RFC-066 D4) | focus set by the application shows its ring, and Enter presses at once | **Set the entry only when the window was opened from the keyboard** (a shortcut, or Enter on a control). Opened by a click, nothing is focused, and the first Tab reaches the window's first control: its header close, which is safe. D4's purpose, no ring the user did not ask for, then holds by construction |
| d | **↓ / ↑ act as Tab** inside a window (RFC-066 D5) | arrows only inside menus, radio groups, selects and sliders | **Recommended:** keep them in the list-like windows (palette results, workspace menu, "Find work area") by routing them to `traverse`, and only while no text field holds focus; drop them elsewhere, and update the shortcuts sheet. **The owner decides after comparing both behaviours** (stage 0) |
| e | **One Escape closes,** even from a field (Task 095, Task 111) | "a focused text field in a dialog takes the first Escape; the second closes" | **Keep knotra's,** through Task 111's layer, which sits inside knotra's own content and does not compete with snora's |

### D5. Settings and History join by construction

With every control a snora control, both screens are reachable with no order to maintain. Their
first Tab reaches their first control. No entry is set on a screen change; RFC-066's entries are
for windows.

### D6. The dashboard's own keys stay knotra's

knotra's subscription keeps these, each moving focus with `operation::focus(id)`:

- ↓ ↑ j k between projects;
- `/` and Ctrl+/ to the search field;
- Ctrl+R, Ctrl+K, Ctrl+T.

The shortcuts sheet's test (RFC-066 D7) keeps it true by construction.

## Stages

Each stage has its own handoff, evidence and review, under the owner's GUI test rules.

0. **Measure,** with no product change.
   - For every window and screen: every control in reading order, and RFC-066's declared
     order beside it.
   - snora's answers to D4 a-d, measured in a spike branch that is not merged: what Tab does
     past a modal's last control; whether a focused text field keeps ↓ ↑; the ring on
     programmatic focus.
   - **D4 d, compared:** two spike builds, not merged, that differ only in ↓ ↑ in the
     list-like windows (one with the arrows routed to Tab there, one without). The owner tries
     both, and the decision is recorded in an amendment.
   - Whether the headless level can run `traverse`'s tasks. `iced_test`'s `Emulator` runs a
     program's tasks, and `Simulator` does not.
   - **Output:** a table, and a short amendment to this RFC with any decision the
     measurements change.
1. **Primitives.**
   - knotra-ui's buttons, chips, select and checkbox are rebuilt on snora's controls, with the
     same look.
   - The contrast and target guards hold.
   - Not yet used by the views.
2. **The header, the dashboard and the selection bar.**
3. **Every window,** with entries by id (D3), D4 as decided, and Escape (Task 111) unchanged.
4. **Settings and History.**
5. **Retirement.**
   - knotra's focus model goes.
   - RFC-066's tests are replaced by the acceptance tests of D2 and D3, which assert the
     same orders and entries.

## Acceptance

- Every control on every screen and in every window is reachable by Tab and operable by Enter
  or Space.
- Every select opens from the keyboard.
- Each window's Tab sequence and entry match RFC-066's tables, except as decided in D4.
- Escape closes each window once (Task 095, Task 111).
- The shortcuts sheet's test passes.
- No knotra focus state remains: `git grep` finds no `overlay_focus` or `focus_visible`.

## Out of scope

- F6 zones (snora 0.61). They come later, if wanted, as a row on the shortcuts sheet with the
  owner's text.
- Right-to-left.
- The palette's search behaviour.

## Decided

Owner, 2026-11-18:

- the RFC is accepted;
- D4 a, b, c and e are as recommended (a after measuring);
- **D4 d** is decided after the owner compares both behaviours in stage 0.

## Amendment A1 (2026-11-24): stage 0's answers, and D4 a and D4 d decided

Stage 0 is approved. Its measurements are in `.git-exclude/reviewed/295-rfc-075-stage-0-review.md`.

### What stage 0 measured

- Tab past a window's last control leaves the window for one press, with nothing focused, then
  starts again from its first control.
- A disabled control is skipped.
- `focus(id)` draws the ring. A window opened by a click focuses nothing until the first Tab.
- **knotra's current ↓ ↑ handling desynchronises from snora's focus.** The ring vanishes. It must
  go, whatever is decided.
- snora's select works from the keyboard. The application closes it in `on_select`.
- Acceptance tests can run at the `iced_runtime` `UserInterface` level. `Simulator` cannot run
  `traverse`.

### Decided by the owner, 2026-11-24, after trying both builds

- **D4 d: ↓ ↑ move through lists:**
  - the palette's results;
  - the workspace menu;
  - "Find work area";
  - and, as always, snora's own menus and selects.

  Elsewhere, Tab moves between controls.
- **In the command palette, every typed character goes to the search box,** wherever the
  highlight is. Only Enter runs the highlighted result.
- **D4 a: nothing is ever left focused on nothing:**
  - Tab and Shift+Tab, past the last or before the first control, go to the other end, in a
    window and on the page alike;
  - ↓ ↑ wrap within their list;
  - ↑ from a list's first item returns to the field above it, where there is one (the palette's
    search box).

### Refinements, by the owner's design standard

The owner asked for a review against "finally clean, safe and secure, and robust and
sophisticated design" and "for users not to be confused or misunderstand". The three
refinements above follow from it:

- **"Every typed character" includes Space.** Space would otherwise press the highlighted
  result. Only Enter runs it.
- **Arrows wrap within the list, not the window.** ↓ from the last result never lands on the
  window's Close.
- **The wrap holds on the page too.** One rule everywhere, with no press that shows no ring.

### What this changes in the stages

- **Stage 1:** a `UserInterface`-level helper for Tab and arrow sequences, used by every later
  stage's acceptance tests.
- **Stage 2:**
  - the page wraps;
  - the two empty-state buttons ("Add your first project", and the no-matches "Clear
    filters") become reachable;
  - the two order differences are fixed: the selection bar's "Exit selection", and Compact's
    "Clear filters".
- **Stage 3:**
  - windows wrap;
  - list arrows and the palette's typing rule;
  - a select or menu open inside a window closes on the first Escape, which Task 111's
    `escape_first` must allow.
- **The shortcuts sheet:** "↑ / ↓: in lists". The exact English and Japanese text goes to the
  owner with stage 3.
