# RFC-075 handoff — Stage 5: the retirement

RFC: `rfcs/accepted/075-one-keyboard-model.md` (Stages §5, and Acceptance). Read Reviews 302-306
first.

**Start now.** Baseline: `origin/main` (`3d73e79` or later). One stage, one review request.

**Nothing a user sees changes.** There is no CHANGELOG entry, no version change and no release
preparation.

**This handoff is immutable.**

## 1. What remains

Stage 3 already removed most of knotra's old model (`state/focus.rs`, `ring()`,
`ActivateFocused`). What is left:

- **knotra's focus state:** RFC-075's acceptance says `git grep` finds no `overlay_focus` and no
  `focus_visible`. Both still appear in code (`app.rs`, `message.rs`, `state.rs`, `tests.rs`).
  - Remove both from code.
  - A doc comment may still name them as history, but no identifier may remain.
  - If `focus_visible` still decides something, say what. snora's ring already appears only for
    keyboard focus, so the decision should now be snora's alone.
- **knotra-ui's old primitives,** now beside the new ones since stage 1:
  - the iced-based buttons and their `_maybe` forms;
  - `styled_pick_list`;
  - the old `checkbox`;
  - `chip::filter`;
  - `with_focus_ring`;
  - the ring helper;
  - anything else of the old model.
- **Text-field ids** outside `view/ids.rs` (for example `knotra_ui::widget::focus_id`). Bring every
  id into the one place, or say why one cannot move.

## 2. How to find them: with an instrument, not a list

The names above are a starting point, not an enumeration.

- For **every public item** in `crates/knotra-ui/src/widget/`, count its callers outside its own
  file and outside tests. Do it with a script, or with clippy and `dead_code` run against a build
  that does not export the items.
- List the items with no remaining caller, and delete them.
- **Report the table:** item, callers before, verdict.
- `knotra-ui` is published. Removing public items is a breaking change, which is acceptable at
  0.x. The release notes will say it when 0.35.0 is prepared, not now.

## 3. The guards

A guard that scans for a name nothing uses any more passes vacuously.

- `pick_list_height_guard` scans `pick_list(`.
- `text_input_height_guard`, `button_style_guard`, `button_height_guard` and `button_label_guard`
  may scan old names.

For each guard:

- **retarget** it to the new primitives, and show it failing first on a deliberate break; or
- **delete** it, if another guard already covers the same question. Say which guard does.

## 4. Names

**Keep the `_control` names.** Renaming them back to the old names would touch hundreds of call
sites for no user's benefit, and the names stay unambiguous.

Where a doc comment explains that the suffix "tells old from new", rewrite it to say what the
function is.

## 5. Tests

| Question | Level | How |
|---|---|---|
| No knotra focus state remains | 1 | `git grep -nw 'overlay_focus\|focus_visible' -- crates`, comments excluded, finds nothing. Paste the command and its output |
| No unused old primitive remains | 1 | §2's table |
| Every guard still bites | 1 | §3, each failing first or removed with a reason |
| Nothing a user sees changed | 2 | Render the whole page, a window and Settings before and after, in all four themes, and compare by pixels. Expect 0 differing pixels |
| Everything else | 1 | the full suite |

## 6. Report

- §1-§4, with §2's table.
- §5's grep and the pixel comparison.
- Gates, with each crate's printed counts and their sum.
- **CI's result once the run has finished, with its run id.**
- knotra's status, reflog and `git worktree list`.
