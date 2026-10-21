# Developer Handoff — RFC-066: keyboard use stays inside the open window

RFC: `rfcs/accepted/066-keyboard-use-stays-inside-the-open-window.md`, **accepted 2026-10-21
(project owner), with the Japanese of D7.** Read it in full first, and RFC-036 D1, R5–R7, R12
and its Guardrail 2, which this builds on.

**Start after RFC-067 is approved.** Baseline: `origin/main` at that point; report the commit
and its test count.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard

Every window can be used with the keyboard alone, and nothing the keyboard does is
invisible or surprising. Concretely: focus is always inside the top window; it never lands
first on a control that changes something; nothing is pressed while the outline is hidden;
and the shortcuts sheet cannot list a key that does not work.

**Three stages, three review requests.** Each stage ends with every gate green and its own
review request. Do not start a stage before the previous one is approved.

## Stage 1 — one list of windows; focus moves in one place (D1, D3)

No user-visible change except the workspace dialogs' Tab order (reading order).

**`Layer` and `top_layer`** (new, `state/layer.rs` or beside `focus.rs`):

- `Layer` has the 13 variants of RFC D1. `Resolve` carries the project id; nothing else
  carries data.
- `top_layer(&AppState) -> Option<Layer>` returns the window **drawn on top**. Derive the
  order from `view.rs` (the `AppLayout` dialog/sheet/menu and the `stack` layers drawn after
  it). Write the order in its doc comment, with the `view.rs` lines it follows. Test every
  pair of windows that can be open at once.
- The Remove project confirmation is a `Layer` (`RemoveProject`) from this stage on, even
  though it is still drawn inline until Stage 3.

**Replace, using `top_layer`:**

- `overlay_focus_order` and `any_other_overlay_is_open`: an exhaustive
  `match top_layer(state)`, calling each layer's `layer_focus`. In this stage, layers other
  than the three workspace dialogs return an empty order with `entry: None`, as today. Stage 2
  fills them.
- `close_topmost_layer` and the workspace menu's own Escape branch in `app.rs`: one
  exhaustive `match top_layer(state)` that sends that window's close message in its current
  phase (Task 095's table), keeping the running-phase guards. The selection-clearing fallback
  stays for `None`.
- Task 095's "a window is open" predicate becomes `top_layer(state).is_some()`.

**`LayerFocus { order, entry }`** in `state/focus.rs`. The three workspace dialogs provide
`layer_focus` in reading order: header close first. Their entries are unchanged: the name
field; Cancel for delete.

**One focus step (D3).** In `app.rs`'s `update`:

1. note `top_layer` before handling the message;
2. handle it;
3. call `focus_ops::settle_layer_focus(state, before)`.

`settle_layer_focus` does exactly D3's three cases. Each move goes through
`focus::reconcile` and returns its `Task`, chained with the handler's `Task` (Guardrail 2).
"The focused control is gone" means `overlay_focus` is not a target in the current `order`.

**Remove every focus call at open and close sites:** `open_overlay_focus`,
`enter_overlay_focus` and `close_overlay_focus` in `app/{workspace,misc,freezer}.rs` and
`app.rs`. Task 095's `AddProjectBack` call goes too. List each removal in the report.
`focus_search` stays: it is not a window.

**Tests (Stage 1):**

- the existing RFC-036 dialog tests pass. Where one asserts the old Tab order (close last,
  delete's Cancel first), change it to reading order and say which;
- open, phase change and close for the three workspace dialogs and for the palette (whose
  entry is its field, through `settle_layer_focus` now, not `open_overlay_focus`);
- **the Add project defect:** a failed "Next" (empty path) leaves focus on the folder
  field, not on the name field that is not drawn. Seen to fail on the baseline.

## Stage 2 — every window's order and entry; keys; the outline (D2, D4, D5)

**Orders.** Each window module provides `layer_focus`, in reading order. Disabled controls
are included; controls not drawn are not. New `focus_id` statics for the two text fields
without one: the freezer note and the changelog "Since".

**Entries**, by RFC D2's rule. The inventory below is what the code showed at `316a3bb`. Where
the code differs, follow the rule and report the difference.

| Window, phase | Entry |
|---|---|
| Get latest safely, AwaitingConfirm | first row's first enabled choice; with no choices, "Cancel" |
| Get latest safely, RetryPreparationFailed | "Review retry" if enabled, else "Close" |
| Get latest safely, Done | "Close" |
| Save release point, Idle | "Release name" |
| Save release point, ValidationReady | "Show details" if shown, else "Cancel" |
| Save release point, Done | "Not now" if the share offer is shown, else "Close" |
| Generate notes, Idle | "Since" |
| Generate notes, Ready | "Copy to clipboard" |
| Change work area, BrowsingList | "Find work area" |
| Change work area, ConfirmSwitch | "Cancel" |
| Change work area, Done | "Close" |
| Resolve, Browsing | first file's "Open in editor"; with no files, "Close" |
| Resolve, ConfirmStop (RFC-067) | "Keep fixing" |
| Resolve, Done | "Close" |
| Command palette | the search field |
| Add project | the folder field; on the name step, the name field |
| Workspace menu | the first workspace |
| Shortcuts sheet | "Close" |
| Remove project | "Keep" |
| Any phase with no controls | `None` |

**The guard test (R2):** for every layer and every phase you can construct, assert the
entry's activation message is not one of the change-starting messages:

- `ExecuteRequested` (Get latest safely), `ExecuteConfirmed` (Save release point),
  `PushConfirmed`, `SwitchConfirmed`;
- `MarkResolvedRequested`, `AbortMergeRequested`;
- `AddProjectConfirmed`, `RemoveProjectConfirmed`, workspace create, rename and delete
  confirms.

List them in one place in the test, with a comment that a new change-starting message must
be added.

**Keys (D5).** In `handle_shortcut`, when `top_layer` is `Some`: ↓ and ↑ advance like Tab
and Shift+Tab; `j` and `k` do nothing. Command palette: remove `highlighted`; its results
draw the outline like every other control.

**The outline (D4):**

- `AppState::focus_visible: bool`, default `false`.
- Set to `true` by Tab, Shift+Tab, ↑, ↓, `j`, `k`. Set to `false` by a mouse button press,
  from a new subscription built with `iced::event::listen_with`. **First, verify** that
  iced 0.14 delivers mouse presses there even when a widget captured them. Do it with a
  test or a measured run, and report how. If it does not, **stop and report**.
- `ActivateFocused` with `focus_visible == false` sets it `true` and presses nothing. A
  focused text field is unaffected: it receives keys itself, as RFC-036 R3a already
  ensures.
- One helper, `focus_ops::ring(state, target) -> bool`, returns
  `focus_visible && resolved target == target`, against the current context's order. Use it
  for every outline in the app. Delete the `is_focused` copies in `shell.rs`,
  `selection_bar.rs` and `workspace_manager.rs`, and every hard-coded `false` with its
  "later work" comment (R8). `git grep -n 'is_focused = false\|with_focus_ring(&t, false'`
  must print nothing.
- Buttons drawn with snora's `*_maybe` helpers or bare iced `button` cannot draw the
  outline. That covers the palette's "✕ Esc" and results, Add project's ✕, "Choose
  folder", "Cancel" and "Back", the shortcuts sheet's Close, the workspace menu items, and
  Remove project's buttons. Move each to knotra's `styled_button` / `reasoned` /
  `with_focus_ring` path with the same look. Report any whose look changes.

**Tests (Stage 2):**

- R2 as above;
- R4: for each layer, Tab, Shift+Tab, ↓ and ↑ from every target stay inside its order;
- R5: a mouse press clears `focus_visible`; Tab sets it; Enter with it clear presses
  nothing and sets it;
- the palette: ↓ from the field reaches the first result; Enter there runs it.

## Stage 3 — Remove project in a window; the shortcuts sheet; CHANGELOG (D6, D7, R9, R10)

**D6.** Draw the Remove project confirmation as a snora dialog over any screen: small width,
the same title, body and buttons. "Keep" is `style::ghost`; "Remove from list" is
`style::danger`. Delete the inline rendering in `view/dashboard/{mod,empty}.rs`. Test: it is
the top layer on the History screen.

**D7.**

- Move the subscription's key mapping into a pure
  `shortcut_for(&keyboard::Key, keyboard::Modifiers) -> Option<ShortcutMessage>`. The
  subscription calls it.
- `Ctrl+R` sends `WorkspaceMessage::RefreshRequested`, the "Check now" button's message.
  Remove `handle_tick`'s shortcut path, if nothing else uses it.
- The sheet's table becomes the nine rows of RFC D7, each with an example `(Key, Modifiers)`
  and the `ShortcutMessage` it must produce.
- Test: every row's example maps through `shortcut_for` to its message, and every
  `ShortcutMessage` variant appears in some row. The variants handled only inside a window
  (Escape, Tab, ↓, ↑) appear in the window rows.
- Catalog: the RFC's English and Japanese **exactly**. Remove every `shortcut.*` key no
  longer read, in both catalogs, and confirm with your Task 092 enumeration script that it
  prints 0 unused keys.

**CHANGELOG `[Unreleased]`, exactly:**

```markdown
### Changed — every window can be used from the keyboard

Opening a window now moves keyboard focus into it, onto a control that changes nothing.
Tab and the arrow keys move only inside the window, Enter presses the highlighted control,
and Escape closes it. The focus outline appears only while you use the keyboard.
"Remove project" now asks in a window on every screen, and the keyboard shortcuts list
shows exactly the keys that work.
```

**R10, screenshots.**

1. In Task 087's demo environment, recapture `03-smart-pull` and `04-freezer`, English and
   Japanese, opening each window **with the mouse**.
2. Compare each against `packaging/windows/screenshots/` pixel by pixel and report the
   differing region, if any. Expected: none.
3. Also capture the new Remove project dialog over History, English and Japanese, and the
   shortcuts sheet, English and Japanese.
4. Save everything under `.git-exclude/evidence/rfc-066/{en,ja}/`. Task 087's privacy rules
   apply in full. Do not commit the images.

## Out of scope

F6 and regions; high contrast; new shortcuts; any window's behaviour or text beyond D6 and D7.

## Verification, every stage

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <stage baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
```

Suppression map **five**, dead-code probe **one** line, `advisories ok`. Explain each stage's
test count delta.

## What to report back, per stage

- Stage 1:
  - `top_layer`'s order and its `view.rs` evidence;
  - the removed focus calls;
  - the changed RFC-036 tests;
  - the Add project seen-to-fail output.
- Stage 2:
  - each layer's order and entry per phase, and any difference from the table above;
  - the `listen_with` verification;
  - each control moved to an outline-capable style, and any change in its look.
- Stage 3:
  - the dialog;
  - `shortcut_for` and the sheet test;
  - the catalog before and after (unused keys: 0);
  - the CHANGELOG entry as committed;
  - the screenshot diffs and the privacy inspection.
- Every stage: gate output, with the `diff --check` range.
