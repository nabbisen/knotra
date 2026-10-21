# RFC-066 - Keyboard use stays inside the open window

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - most windows cannot be used from the keyboard, and the shortcuts sheet describes keys that do not exist |
| Effort | Medium-large - one layer model, ten focus orders, one subscription, the shortcuts sheet |
| Target | 0.32.0 (`.git-exclude/reviewed/226-release-plan-0.31.1-and-0.32.0.md` §3) |
| Related files | `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/app/focus_ops.rs`, `crates/knotra-app/src/state/focus.rs`, `crates/knotra-app/src/view/{overlays/*,command_palette,add_project_modal,shortcuts_overlay,workspace_manager,shell,dashboard/empty}.rs`, `crates/knotra-ui/src/{i18n,widget/focus}.rs` |
| Related | RFC-033 D3 item 4 (the contract: focus entry, trap, return); RFC-036 (knotra's own focus model; did the three workspace dialogs); RFC-037 (migrated the workflow windows visually, not for focus); Task 095 (defects fixed first, below) |

## Summary

knotra promised in RFC-033 that every window (dialog, sheet, menu) takes keyboard focus when
it opens, keeps Tab inside itself, and gives focus back when it closes. Only the three
workspace dialogs do that. In the other ten, Tab does nothing, and Enter reaches nothing.
In several, pressing Tab inside a text field even drops focus with no way back.

After this RFC, every window can be used from the keyboard alone: focus enters at a safe
control, Tab and the arrow keys move inside the window only, Enter presses the highlighted
control, and Escape closes it. The focus outline appears for keyboard users only. The
shortcuts sheet lists exactly the keys that work.

## Problem

Measured at `316a3bb`.

**1. Ten windows have no keyboard order.** `focus_ops::overlay_focus_order` gives a real order
only to the workspace create, rename and delete dialogs. Every other window gets an empty
order, a deliberate safety net from RFC-036 Stage 3 so that Tab cannot reach the screen
beneath. The effect, per window:

| Window | Today |
|---|---|
| Get latest safely, Change work area, Resolve, Generate notes, Save release point (Ctrl+T) | focus does not enter at all |
| Command palette, Add project, Save release point (from the selection bar) | the text field is focused; **Tab clears it** and nothing can be reached |
| Shortcuts sheet, workspace menu | focus does not enter |
| Remove project confirmation | not a window: drawn at the bottom of the Dashboard, so **opened from History or Settings it is invisible**, yet it still swallows Tab and Enter |

Every workflow window draws its buttons with `is_focused = false`, hard-coded, with a
comment pointing to later work.

**2. Focus state goes stale.** Nothing clears the window focus when these ten close. The
next window inherits it. The bare `/` key then checks a text field that no longer exists.

**3. Where focus should enter is unspecified.** In four windows the button that starts a
change comes first in the footer: "Start getting latest", "Save release point", "Change work
area", "Add project". A rule of "focus the first control" would put Enter one keystroke away
from changing repositories.

**4. The shortcuts sheet is wrong.** Of its 19 rows, 11 describe keys that do nothing
(`?`, `⌘1…9`, `Shift+Space`, `Ctrl+A`, `f`, `p`, `t`, `b`, `g h`, `g s`, and the palette's
`↑ / ↓` and `Enter`). `Ctrl+K` is listed as "Open command palette"; it opens "Change work
area". `Tab`, `Enter`, `Ctrl+T` and `Ctrl+/` are missing. Its Japanese names do not match the
screens ("安全に最新化" for 「安全に最新を取得」, "リリース地点を保存" for 「リリースポイントを保存」).

**5. The palette cannot be driven from the keyboard.** Its results have no keyboard path; the
`highlighted` styling hook is never drawn and nothing moves it.

Fixed first, separately, in **Task 095** because they are defects needing no design: global
shortcuts acting underneath an open window (`Ctrl+T`/`Ctrl+K` replaced a running "Save
release point" or "Get latest safely" view; `/` switched the screen behind a dialog);
Escape skipping cleanup the window's own Close performs; "Back" in Add project closing the
whole dialog.

## Design

### D1. One list of windows, in one stacking order

A `Layer` enum names every window: `ShortcutsSheet`, `Palette`, `AddProject`,
`WorkspaceCreate`, `WorkspaceRename`, `WorkspaceDelete`, `RemoveProject`, `WorkspaceMenu`,
`GetLatest`, `SaveReleasePoint`, `GenerateNotes`, `ChangeWorkArea`, `Resolve`.
`fn top_layer(&AppState) -> Option<Layer>` decides which is on top, in **one** order that
matches what is drawn on top. Keyboard routing, focus, the outline and Escape all read
`top_layer`, through **exhaustive** `match`es, so a new window cannot be added without a
focus order and a close path. This replaces `any_other_overlay_is_open`, the empty-order
safety net, and the workspace menu's separate Escape branch in `app.rs`.

### D2. Every window declares its order and its entry

Each window's view module provides
`fn layer_focus(state) -> LayerFocus { order: FocusOrder<Message>, entry: Option<FocusTarget> }`.

- **`order` follows reading order**, as WCAG 2.4.3 requires: header close first, then body,
  then footer, left to right. Disabled controls stay in it (RFC-036 R3); a control that is
  not drawn is not in it.
- **`entry` is where focus lands**, by one rule, applied to whatever the window shows now:
  1. while the user is choosing: the first text field, if there is one; otherwise the first
     control **below the window's header** that does **not** start a change; otherwise the
     header's close;
  2. once the window shows a result: "Close". Generate notes is the one exception: its
     result is the notes, so "Copy to clipboard".

  **A control that starts a change is never the entry.** "Starts a change" means it changes
  a repository, a remote, or the list of projects or workspaces. Enter pressed straight
  after a window opens never does any of those.
- While an operation runs and the window shows no controls, `order` is empty and `entry`
  is `None`. Tab and Enter do nothing, as now.

The entries this gives, for the record (the handoff lists every phase):

| Window | Entry |
|---|---|
| Get latest safely, choosing | first "Check only / Get anyway" choice; if no project needs a choice, "Cancel" |
| Save release point | "Release name"; once checked, "Show details" when shown, else "Cancel" |
| Generate notes | "Since"; with notes ready, "Copy to clipboard" |
| Change work area | "Find work area"; on the confirm step, "Cancel" |
| Resolve | the first file's "Open in editor"; with no files left, "Close" |
| Command palette | the search field |
| Add project | "Project folder"; on the name step, "Project name" |
| Workspace create / rename | the name field (unchanged) |
| Workspace delete, Remove project | "Cancel" / "Keep" |
| Workspace menu | the first workspace in the list |
| Shortcuts sheet | "Close" |
| Any result | "Close" |

The three workspace dialogs' orders change to reading order (header close first, not last;
delete's Cancel no longer first), with their entries unchanged. What the user sees on
opening does not change.

### D3. Entry, change and return happen in one place

Today each place that opens a window calls a focus function, and three of those calls are
wrong (Add project focuses its name field even when validation failed and the field is not
drawn). After this RFC, **no open or close site touches focus.** One function, run after
every update, compares the window on top before and after:

- a window opened, or a different one is now on top: focus moves to its `entry`;
- the same window, but the focused control is gone (its phase changed): focus moves to
  `entry`;
- no window is open any more: window focus is cleared.

Each move reconciles iced's text-field focus in the same `Task` (RFC-036 R12, Guardrail 2).
The screen's own focus is never written while a window is open, so it is exactly where the
user left it when the window closes: return by construction, as RFC-036 R7 already does for
the workspace dialogs. The calls to `open_overlay_focus`, `enter_overlay_focus` and
`close_overlay_focus` at open and close sites are removed.

### D4. The outline is for keyboard users

Focus enters every window on open, so Tab and Enter work at once. But an outline appearing
on a button the user did not choose, right after a mouse click, looks like a mistake. So the
**outline is drawn only while the keyboard is in use**:

- `AppState::focus_visible` turns on with Tab, Shift+Tab, or an arrow key, and off with
  any mouse button press (a new `iced::event::listen_with` subscription; mouse events reach
  it whatever widget handled them).
- Every `is_focused` used for an outline is `focus_visible && the control is the resolved
  focus target`, through one helper. The per-module copies in `shell.rs`,
  `selection_bar.rs` and `workspace_manager.rs` go.

This is the browser's `:focus-visible` behaviour. It applies to the whole app, so the
Dashboard's outline also disappears after a mouse click, instead of lingering on the last
keyboard target. Focus itself is never lost; the next Tab shows the outline where focus is.

**Nothing is pressed that the user cannot see.** While the outline is hidden, Enter or Space
on a control does not press it; it turns the outline on, showing where focus is, and the
next Enter or Space presses. Typing into a text field is unaffected: the field receives
keys itself.

### D5. Keys inside a window

| Key | In a window |
|---|---|
| Tab / Shift+Tab | next / previous control, wrapping, never leaving the window |
| ↓ / ↑ | the same as Tab / Shift+Tab, so the palette, the workspace menu and "Find work area" work as lists |
| Enter / Space | press the focused control; in a text field, typing as usual (RFC-036 R3a) |
| Escape | close, by the window's own Close (Task 095) |

`j` and `k` do nothing in a window; they are letters a user may be typing. The palette's
unused `highlighted` field goes: its results draw the focus outline like every other control.

### D6. Remove project asks in a window

The confirmation becomes a small dialog like "Delete workspace", over any screen, with the
same text and buttons: "Keep" (entry) and "Remove from list", in danger style. It no longer
appears inline on the Dashboard, so it works from History and Settings too.

### D7. The shortcuts sheet lists what works, by construction

Key handling moves into one pure function, `shortcut_for(key, modifiers) -> Option<ShortcutMessage>`,
called by the subscription. The sheet's table holds, for each row, an example key press. A
test asserts every row's example maps through `shortcut_for` to the row's action, and that
every `ShortcutMessage` has a row. A sheet that lies fails the build.

The rows, in English and Japanese. **The Japanese needs the owner's approval.** Names are the
ones the screens use.

| Keys | Where (EN / JA) | Action (EN / JA) |
|---|---|---|
| Tab / Shift+Tab | Everywhere / どこでも | Move to the next or previous control / 次または前の項目へ移動 |
| Enter / Space | Everywhere / どこでも | Press the highlighted control / 強調表示中の項目を押す |
| Esc | Everywhere / どこでも | Close the open window, or clear the selection / 開いている画面を閉じる、または選択を解除 |
| ↑ / ↓ | In a window / 画面内 | Move to the next or previous control / 次または前の項目へ移動 |
| ↑ / ↓ / j / k | Dashboard / ダッシュボード | Move between projects / プロジェクト間を移動 |
| / or Ctrl+/ | Dashboard / ダッシュボード | Go to the search field / 検索欄へ移動 |
| Ctrl+R | Main screens / メイン画面 | Check now / 今すぐ確認 |
| Ctrl+K | Main screens / メイン画面 | Change work area / 作業エリアを変更 |
| Ctrl+T | Main screens / メイン画面 | Save release point / リリースポイントを保存 |

"Main screens" means no window is open, which Task 095 enforces. `Ctrl+R` sends the same
message as the "Check now" button, so the row's name is literally true. Unused `shortcut.*`
keys are removed from both catalogs; the column header "Context" / 「コンテキスト」 becomes
"Where" / 「場所」.

## Requirements

- R1. `top_layer` and `Layer` exist; focus routing, outline and Escape read them only
  through exhaustive matches.
- R2. Each of the 13 layers provides `layer_focus`; `order` is in reading order; `entry`
  follows D2's rule; no `entry` ever sends a message that starts a change (tested per layer
  and phase).
- R3. No open or close site calls a focus function; D3's single post-update step does
  entry, re-entry and clearing (tested: open, phase change, close, for every layer).
- R4. Tab, Shift+Tab, ↓ and ↑ never move focus outside the top window (tested per layer).
- R5. The outline is drawn only when `focus_visible`; a mouse press clears it, a navigation
  key sets it; Enter or Space with the outline hidden shows it and presses nothing (tested).
- R6. Remove project is a dialog over every screen; "Keep" is its entry.
- R7. The shortcuts sheet test of D7 exists and passes; the nine rows above, with the
  approved Japanese.
- R8. Every hard-coded `is_focused = false` and its "later work" comment is gone.
- R9. CHANGELOG `[Unreleased]`: one entry for the user-visible effect (text in the handoff).
- R10. Store screenshots: none should change, since the outline is hidden after mouse use.
  The handoff recaptures `03-smart-pull` and `04-freezer` (both languages) and diffs them
  against the committed set to prove it.

## Out of scope

- F6 between screen regions, and high-contrast themes: their own RFCs, in this cycle.
- What any window does; its texts, apart from the sheet and D6's move.
- New shortcuts. The sheet becomes true; it does not grow.

## Risks

- **A wrong entry could start a change.** D2's rule and R2's per-phase test are the guard.
- **Focus moving after every update could fight the user.** D3 moves focus only when the
  top window changes or the focused control disappears, never otherwise.
- **An invisible press.** Focus enters on open even after a mouse click; D4's rule that Enter
  and Space first show the outline means a mouse user's stray Enter presses nothing.
- **The mouse subscription.** `listen_with` receives events already captured by widgets;
  if iced 0.14 does not deliver them, the handoff stops and reports before D4 is built.
