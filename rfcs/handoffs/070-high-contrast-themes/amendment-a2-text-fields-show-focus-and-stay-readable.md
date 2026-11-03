# RFC-070 handoff — Amendment A2: a text field shows its focus, and stays readable when unavailable

Adds **one commit** to RFC-070, on top of `b5177ec`. Ruling of Review 255. RFC-070 is
approved once this lands. Everything else stands.

## Why

`text_input_style` (`knotra-ui/src/widget/field.rs`, RFC-071 D6) draws focus by changing
only the border's **colour**, keeping its 1 px width:

| Theme | Unfocused border | Focused border | Difference |
|---|---|---|---|
| light | grey (`border`) | blue (`accent`), 1 px | slight |
| HC light | black | dark blue, 1 px | almost none |
| HC dark | white | white (`accent` vs `border` both near white), 1 px | **none visible** |

The RFC-070 report's own focus captures changed only 64–107 pixels across a 2400×1350 window.

Three things follow:

- In a high-contrast theme, the theme meant for people who need it most, a keyboard user
  cannot see which field has focus. That breaks WCAG 2.4.7 (focus visible), and RFC-066's
  rule that the outline is always visible on the keyboard.
- Buttons use the focus ring (`with_focus_ring`: `tokens.focus.ring_color`, at
  `ring_width` 2 px, or 3 px in HC). Text fields never did.
- RFC-070's CHANGELOG promises "a thicker focus outline". For text fields it is not true
  yet.

**Also:** an unavailable text field fades its value to 50 % opacity. That breaks RFC-071 A1
D13's rule (readable, at full opacity), which A5 restated for buttons.

## The commit

1. **Focused: the focus ring.** A focused text field's border is the focus ring:
   - colour `ring_color_for(tokens, background)`, the same chooser buttons use
     (`widget/ring.rs`);
   - width `tokens.focus.ring_width`.

   Unfocused and hovered stay as they are. Report the four themes' ring colour and width.
2. **Unavailable: readable.**
   - No alpha fade on background or value.
   - The value takes `text_muted` at full opacity.
   - The border keeps its 1 px `border`.

   This is D13's rule, applied to fields.
3. **Tests**, in `field.rs`:
   - in all four themes, the focused border width equals `tokens.focus.ring_width` and is
     greater than the unfocused width;
   - the focused colour differs from the unfocused colour **or** the width differs (it
     always does);
   - the unavailable value is `text_muted`, with alpha 1.0;
   - the ring colour against the field background meets the non-text floor (3 : 1 normal,
     4.5 : 1 HC), added to the contrast suite.

## Evidence

**Question 1:** can you see which field has focus, in every theme?

- The dashboard's search field, focused by keyboard (Tab to it), in light, dark, HC light
  and HC dark, English.
- These are after images only, since this is the fix.

**Question 2:** R6 holds outside a focused field. Re-run RFC-070's own R6 script (dashboard,
ready check, palette; light and dark; nothing focused) on `b5177ec` and on A2's head, and
compare by machine. Every pair must be 0.

- This is R6's second run. It is authorised here.
- The palette opens with its search field focused, so `03-palette` will differ by exactly
  the field's ring. Report its pixel count and crop it, rather than calling it 0.

Save to `.git-exclude/evidence/rfc-070-a2/`.

## CHANGELOG

No new entry. RFC-070's "thicker focus outline" becomes true for fields too. If you judge that
normal-theme users will notice the stronger field focus, propose one sentence for me to
approve. Do not add it unasked.

## Report

- the style diff;
- the four ring colours and widths;
- the tests;
- Question 1's images;
- R6's pixel counts, with the palette crop;
- gates, with `git diff --check b5177ec..HEAD`.

**Image descriptions.** Describe what is in the image. In Review Request 137 §9, the HC
palette's unavailable commands are called "outline-only". `hc-dark/03-palette-empty.png`
shows them with no outline, as A5 requires: grey text with the reason beneath.
