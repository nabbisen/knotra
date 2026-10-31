# RFC-071 handoff — Amendment A1: kind to older eyes

**Adds D13 to RFC-071, done in stage 2.** **Valid only once the owner approves it.** The owner
handing this path to the dev team is that approval. Everything in the handoff stands.

## Why

The owner's request (2026-10-31): readability and ease of use, **colours kind to elders**,
and text size and weight that give the screen **intonation**, a clear rise and fall between
what matters and what does not. RFC-071 D1, D2, D8 and D11 cover size, weight, rhythm and
zoom. Measured at `37f463e`, two things it does not cover make knotra hard on older eyes.

| What | Contrast now (light / dark) | Source |
|---|---|---|
| main text (`text_primary`) | 17.8 / 16.9 : 1 | snora presets: already excellent |
| secondary text (`text_secondary`) | 8.8 / 9.2 : 1 | already above AAA (7:1) |
| links and button labels (`accent`) | 6.7 / 6.8 : 1 | above AA |
| **a button that cannot be pressed** (label and outline at 45 % opacity, `snora-style` `button.rs:57`) | **≈2.2 / 2.3 : 1** | below every WCAG level for text |

**Unavailable controls are the faintest thing on screen.** Examples:

- "Change work area" in the selection bar;
- most of the command palette's list when nothing is selected.

WCAG exempts disabled controls, but an older user still has to read them to learn what the
app can do and why it will not do it now. RFC-071 cannot fix it by token, because the fading
is snora's.

**Hover-only cues** (RFC-071 D4: a project name shows the accent only on hover) are hard
for anyone with an unsteady hand or reduced sensitivity to small changes.

## D13. Kind to older eyes

1. **Unavailable controls stay readable.**
   - `knotra-ui`'s style wrappers (`widget/buttons.rs` `style::{primary, secondary, ghost,
     danger}`) replace snora's `Disabled` result with knotra's own:
     - no fill;
     - a 1 px `border` outline;
     - the label in `text_muted`, at full opacity: 4.9 / 5.4 : 1.
   - The shape (outline only, no fill) and the reason line beside it (already shown by
     `reasoned`) say "not now". Faintness no longer has to.
   - The command palette's unavailable commands use the same rule.
2. **No meaning by hover alone.** A project name that opens details stays recognisable
   without hovering:
   - a small "›" from the existing icon set follows the name, in `text_secondary`;
   - hover and keyboard focus still add D4's accent and underline.

   Any other control found to reveal what it does only on hover goes in the report.
3. **One contrast suite covers every colour knotra draws text or symbols with**, in both
   themes. The theme tests extend it to RFC-070's presets the same way. Floors:

   | Use | At least |
   |---|---|
   | content text (names, statuses, messages, headings) | **7 : 1** |
   | labels of controls you can press (`accent`, `*_text` on fills) | 4.5 : 1 |
   | labels of unavailable controls (item 1) | 4.5 : 1 |
   | placeholders (RFC-071 D6) | 4.5 : 1 |
   | lines, outlines, the focus ring, a badge against the page (non-text) | 3 : 1 |

   - Colour roles that today fall short **stop the test with their name**. They are not
     adjusted silently. If any default-theme role is short, **stop and report**: changing
     snora's palette is an upstream matter, which I raise with the snora team. The numbers
     above say none should be.
4. **CHANGELOG.** Stage 2 adds one entry to `[Unreleased]`, exactly:

```markdown
### Changed — buttons you cannot use right now are easier to read

They were drawn so faintly that their names were hard to read. They are now shown as an
outline with readable text, next to the reason they are unavailable.
```

## Evidence

Folded into stage 2's set, no new capture set. Its question gains one clause: *"…and can
an unavailable control's name be read?"* Add the command palette with nothing selected:
light and dark, English, before and after.

## Report (added to stage 2's)

- the `Disabled` override and its outline;
- the "›" cue;
- any other hover-only behaviour found;
- the contrast suite's table of measured values per role and theme;
- the CHANGELOG entry as committed.
