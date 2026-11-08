# RFC-073 handoff — Amendment A2: every button is styled, and the label guard has no gap

**Adds to stage 2** (hands and eyes: D8–D10). Ruling of Review 270. Everything else in stage 2
stands. No new text.

## Why

Stage 1 A1 found two gaps in how buttons are kept consistent.

1. **A button with no `.style(...)` falls back to iced's default**, which this theme draws as
   a filled accent button. That is how Settings showed five filled buttons, though no code
   named `primary`.
   - `button(` appears at many sites in `crates/knotra-app/src/view/`; for example
     `toolbar.rs` 11, `smart_pull.rs` 8, `selection_bar.rs` 7, `shell.rs` 7.
   - Nothing checks that each one chose a style.
2. **`button_label_guard` checks only a bare `.height(BUTTON_HEIGHT)` /
   `.height(SMALL_BUTTON_HEIGHT)`.** Written as `.height(Length::Fixed(BUTTON_HEIGHT))`, a
   fixed-height button escapes it. Three sites do so now:
   - `view/dashboard/toolbar.rs:392` ("Arrange", icon and text);
   - `knotra-ui/src/widget/buttons.rs:389` (an icon button);
   - `knotra-ui/src/widget/buttons.rs:437` (`header_control`).

   Each centres its content by hand. The rule ("a fixed-height button's content is centred")
   holds today by care, not by check.

**Also,** in `stage-1-a1/arrange-closed.png`, when "Select" shows its reason line below it,
"Arrange" and Search drop to the middle of the taller column and no longer line up with
"Select" by their tops.

## Changes to stage 2

1. **One helper for icon-and-text content.** `knotra-ui` gains `centered_button_row(row)`:
   height `Fill`, `align_y(Center)`, the shape `header_control` uses today.
   - `header_control`, the icon button and "Arrange" use it.
   - `centered_button_label` stays for text alone.
2. **The guard closes its gap.** `button_label_guard` treats `.height(Length::Fixed(BUTTON_HEIGHT))`
   and `.height(Length::Fixed(SMALL_BUTTON_HEIGHT))` like the bare forms. It accepts content
   built by `centered_button_label` **or** `centered_button_row`.
   - **Seen to fail first:** extend the guard before step 1, and report the three sites it
     names.
3. **Every button chooses its style.** Enumerate every `button(` in `crates/knotra-app/src/view/`
   and `crates/knotra-ui/src/widget/` that reaches the screen with no `.style(...)` and no
   knotra helper that sets one. Open each match.
   - Each gets the style its role calls for (`primary`, `secondary`, `ghost`, `danger`,
     `current_or`, through `with_focus_ring`), by RFC-071 D5's rule of one filled button per
     area.
   - **A guard** then fails on a `button(` in those folders whose builder chain has no
     `.style(` and is not a knotra helper. It is text-scanning, like the others; report how it
     decides.
   - **List every site with its style before and after.** Any site where the right style is
     unclear: **stop and report**.
4. **The toolbar lines up by its top.** The dashboard toolbar's controls row aligns its
   children to the top (`align_y(Start)`), so a reason line under "Select" does not move
   "Arrange" or Search.
   - Check the compact band too.
   - Add one capture of the closed-group case (`arrange-closed.png`'s scene) to stage 2's
     evidence.

## Report (added to stage 2's)

- the helper;
- the guard's failures before the fix;
- the button enumeration, as a table of site, style before and style after;
- the style guard and how it decides;
- the alignment capture.
