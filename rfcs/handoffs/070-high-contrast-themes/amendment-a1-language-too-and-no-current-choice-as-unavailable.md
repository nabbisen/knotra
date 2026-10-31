# RFC-070 handoff — Amendment A1: the language choice too; a current choice is never shown as unavailable

Ruling of Review 251, from RFC-071 stage 2's report. **Adds to step 3 (Settings).**
Everything else in the handoff stands. No new text.

## Why

Settings shows the **current** language the same way it shows the current theme, by
**disabling** that button (`view/settings.rs:120`, `:127`), followed by an "Active: …" note.

- A disabled button means "you cannot do this now". A current choice is not unavailable;
  it is chosen. Two different meanings share one look.
- Since RFC-071 A1 and A5, an unavailable button is drawn as grey text or a grey outline. So
  the language you are using looks switched off, and the other one looks like the choice.

RFC-070 step 3 already replaces the theme buttons with a select. The language row must
follow the same rule, or Settings ends with two patterns for the same thing.

## Changes to step 3

1. **Language, like Theme, becomes one select** (`knotra-ui/src/widget/select.rs`).
   - Label: the existing `settings.locale_label` (`settings.rs:116`).
   - Options: "English" and "日本語", as endonyms, exactly as today.
   - The "Active: …" note under it goes.
2. **`settings.active_prefix`.** Once both notes are gone, nothing uses it. Delete the key in
   both languages; the unused-key test then passes. This supersedes RFC-070 D5's note that
   the key stays.
3. **The rule, enforced by enumeration.** Find every place where a button is disabled
   **because** it is the current choice: a `.on_press_maybe(if current { None } …)` shape
   on a pair or group of options. Open each match.
   - Each one becomes a select, or uses `current_or` (`knotra-ui/src/widget/buttons.rs:323`),
     the style made for "current, not pressable".
   - List each one, with what you did. The navigation tabs already use `current_or`; confirm
     it.
4. **Focus order.** Settings' order changes by the language row as by the theme row. Report
   it before and after.

## Evidence

Question 1's Japanese Settings capture shows **both** selects, with the theme select open.
Add one capture with the language select open, in English.

## Report

Added to the handoff's report:

- the language select;
- the key deletion;
- the enumeration, each match and what you did.
