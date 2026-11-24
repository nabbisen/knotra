# RFC-075 handoff — Stage 2: the header, the dashboard and the selection bar

RFC: `rfcs/accepted/075-one-keyboard-model.md`, with Amendment A1. Read stage 0's inventory first
(`.git-exclude/evidence/rfc-075/stage-0/inventory.md` §1-§3), then stage 1's review,
`.git-exclude/reviewed/298-rfc-075-stage-1-review.md`. Its findings F1, F2 and F4 are carried out
here.

**Start now.** Baseline: `origin/main`. One stage, one review request.

**Windows are not touched in this stage.** While a window is open, Tab, Enter and the arrows
behave exactly as today, through knotra's model. Stage 3 moves the windows.

**No release ships until stage 3 is done** (Review 298 §3). There is no CHANGELOG entry here.

**This handoff is immutable.**

## 0. First: correct stage 1's select test (Review 298 F1)

`select_control`'s keyboard test built the select with nothing above it, so nothing answered ↓.

- snora answers ↓ and ↑ in an open list through `MenuKeys`, which `snora::render` puts at the root
  (`snora-0.65.0/src/render.rs:327`).
- Build the test's view through `snora::render`, as knotra's own view is built. Then check:
  - with the list open, ↓ focuses the first item, ↓ moves, and ↑ on the first item wraps to the
    last;
  - Enter chooses;
  - the caller closes the list.
- If the harness needs a helper for this, add it to `test_support`, so stages 3 and 4 use the
  same one.

## 1. Control text follows knotra's one size (Review 298 F2)

- In `knotra-ui/src/theme.rs`, for every preset, set `typography.label` to the same size and line
  height as `typography.body`. Derive it from `body`, so the two cannot drift apart.
- Say why in one comment: knotra draws all its text in `body` (RFC-056), and snora draws its
  controls' text in `label`.
- **List every place the change shows,** by reading where snora 0.65 reads `label`. These include:
  - `control.rs:229`;
  - `design/button.rs`;
  - `design/notice.rs`;
  - the header menu;
  - `form.rs`'s `Skin`.

  For each place, say what knotra draws through it, if anything. Then capture the most affected
  one (§5, C4).
- Re-run the select height test, and set its expected range to what you now measure.

## 2. Tab on the page goes through snora

While **no window is open**:

- **Tab and Shift+Tab** go through `snora::keyboard::traverse` (RFC-075 D1).
- **The page wraps** (A1): no press leaves nothing focused.
  - Tab past the last control focuses the first.
  - Shift+Tab before the first focuses the last.
  - "The page" is the header, the dashboard and the selection bar, as drawn.
- **Enter and Space** press the focused control once, with snora's own handling. knotra's
  `ActivateFocused` must not press it a second time.
- **The dashboard's own keys (D6)** keep their behaviour, now moving snora's focus with
  `operation::focus(id)`:
  - ↓ ↑ j k between project names, wrapping as today. From focus off the names, the next key lands
    on the first name, as today;
  - `/` and Ctrl+/ to the search field;
  - Ctrl+R, Ctrl+K and Ctrl+T are unchanged.
- **↓ on a focused, closed select opens it** (snora's). It must not also move between projects.
  While a select is open, its list has the arrows (§0).

While a window is open, nothing changes.

## 3. Every control on the page becomes a snora control

Use stage 1's primitives. Each control gets an id, and **all the ids are defined in one place.**
A row's ids are built from its project's id.

| Where | Controls |
|---|---|
| Header (`view/shell.rs`) | workspace switcher, Dashboard, History, Check now, command palette, Settings. The switcher's menu stays knotra's until stage 3 |
| Toolbar (`view/dashboard/toolbar.rs`), both bands | the five chips, Arrange, Clear filters, Select. The search field is unchanged |
| **The Arrange panel** | **the two selects (Group by, Sort by), on `select_control`.** This closes RFC-075's problem 2 on the page |
| Rows and sections | the section toggles, the row checkbox, the name link, the row action |
| Empty states (`empty.rs`) | "Add your first project", and the no-matches "Clear filters". **Both become reachable** (inventory §2d) |
| Selection bar | Exit selection, and the four actions |

**The rules:**

- **The same look at rest,** as in stage 1. Stage 1 has no primitive for the name link or the row
  action. Add them to knotra-ui the same way, with their guards, and name them.
- **The name link (RFC-071 D4)** keeps its look at rest and on hover.
  - On keyboard focus, snora's ring marks it. That meets RFC-071's "never only on hover".
  - If the underline on focus cannot also be kept, say so. Do not work around it.
- **The row checkbox draws no label** (Review 298 F4). The project's name beside it already says
  which row it is. Keep today's tooltip text.
- **Each Arrange select's open state lives in `AppState`:**
  - at most one is open;
  - choosing closes it (snora's rule);
  - closing the Arrange panel, or leaving the dashboard, closes it too;
  - Escape with a select open closes only the select (snora captures it). The panel stays open.
- **Order is reading order, by construction.** The two differences in stage 0's inventory
  disappear because Tab now follows what is drawn:
  - the selection bar's Exit selection comes first, as drawn;
  - in Compact, Clear filters comes after Select, as drawn.

  Do not reorder the layout to match the old declared order.

## 4. What retires now

- The page's declared orders: `shell::focus_order`, `dashboard::focus_order`,
  `selection_bar::focus_order`, `shell_and_dashboard_focus_order`.
- The page views' calls to knotra's `ring()`.
- Replace their tests with the acceptance tests in §5. **These assert the same sequences, except
  the two differences in §3.**
- knotra's model itself (`FocusOrder`, `LayerFocus`, `overlay_focus`, `focus_visible`) stays
  until stage 5, because the windows still use it. Remove only what nothing uses any more, and
  list it.
- The suppression map stays at 7. Do not add `allow(dead_code)` to keep anything alive.

## 5. Tests and captures

| Question | Level | How |
|---|---|---|
| The select's list answers ↓ ↑ (§0) | 1 | through `snora::render`, in the harness |
| The page's Tab sequence is reading order, everywhere, and wraps both ways | 1 | **Drive the real key path:** the key's message through `update`, then every widget operation in the returned task, run on `view(&state)` in the harness. One test per layout: wide and compact, with and without selection mode, an empty workspace, and no matches. Each test asserts the full sequence by id |
| Disabled controls are skipped | 1 | Check now while refreshing; Select with nothing selectable; Change work area with two selected |
| Enter and Space press once | 1 | one message per press, for a button, a chip, the row checkbox and an Arrange select |
| D6 keys move snora's focus | 1 | ↓ ↑ j k between names, with wrap; `/` focuses the search field; ↓ on a focused, closed select opens it and does not move between projects |
| A window still behaves as today | 1 | Tab and Enter inside the palette and in Get latest safely, unchanged |
| Escape with an Arrange select open closes only the select | 1 | through the harness |
| The look at rest is unchanged (except §1) | 2 | stage 1's pixel comparison, extended to the name link and the row action |
| The guards cover the new primitives | 1 | each one **seen failing first** |

**Captures** follow the owner's GUI rules (`.git-exclude/rules/project-instructions-gui-common.md`):

- use Task 108's session (`--scene demo`), the same script for each before and after;
- the before is built from `origin/main`;
- isolate every XDG folder;
- read every path, name and time on screen, and audit the PNG chunks.

Each capture answers one question:

| # | Question | Worst case | Baseline |
|---|---|---|---|
| C1 | **Does the new select sit well beside its neighbours?** (Review 298 F3: the owner decides from this) | Arrange panel open, Japanese, zoom 150 %, Light, Sort by focused by Tab | the same panel from `origin/main` |
| C2 | Is snora's ring clear on the page? | Dark, English, 100 %, a filter chip focused by Tab | the same presses from `origin/main` |
| C3 | Does the row checkbox read well without a label? | selection mode, one row checked, Japanese, 150 %, Light | the same from `origin/main` |
| C4 | Does §1's text change read well where it shows most? | the place §1's list names | the same from `origin/main` |

## 6. Report

- §0: the corrected test, with its failure before the fix.
- §1: the list of places, and what each one draws.
- The ids' one place, and the new primitives' names.
- What retired (§4), and what stays until stage 5.
- The tests, with failing-first output for the guards.
- The four capture pairs, with the privacy inspection.
- Gates, with the counts summed.
- **CI's result once the run has finished, with its run id.**
- knotra's status and reflog.
