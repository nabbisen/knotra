# RFC-071 handoff — Amendment A5: an unavailable control keeps its own shape

Adds **one commit to stage 2**, on top of `4e571a1`. Ruling of Review 251. It corrects A1
D13 item 1, which is my error. Stage 2 is approved once this lands. Everything else stands.

## Why

A1 D13 item 1 said every unavailable control becomes "no fill, a 1 px outline,
`text_muted`". For filled and outlined buttons that is right. For **ghost** controls it is
wrong: they have no outline when they are available. In stage 2's palette capture
(`after/palette-light.png`):

- "Check all projects", which is available, is plain blue text;
- "Get latest safely", which is not, sits in a box with an outline.

The unavailable commands now look **more** like buttons than the available ones. That is the
opposite of the intent, and confusing.

## The rule, corrected

**An unavailable control never gains a shape its available form lacks.** It keeps its own
shape, drops any fill, and its label becomes `text_muted` at full opacity.

| Variant | Available | Unavailable |
|---|---|---|
| `primary`, `danger` (filled) | fill | no fill, 1 px `border` outline, `text_muted` |
| `secondary` (outlined) | accent outline | 1 px `border` outline, `text_muted` |
| `ghost` (no outline) | no outline | **no outline**, `text_muted` |

- **Where:** `readable_when_disabled` (`knotra-ui/src/widget/buttons.rs`) takes the
  variant, or each variant passes its own unavailable style. No view changes.
- **The test:** `disabled_is_outline_only_with_text_muted_label_for_every_variant` becomes
  per variant. A ghost has border width 0 both available and unavailable; the others as in
  the table.

## The CHANGELOG entry, corrected

Replace the stage 2 entry "buttons you cannot use right now are easier to read" with,
exactly:

```markdown
### Changed — buttons you cannot use right now are easier to read

Their names were drawn so faintly that they were hard to read. They are now shown in
readable grey, without their colour, next to the reason they are unavailable.
```

## Also owed from stage 2's report

A1 D13 item 2 asked for **every other hover-only behaviour** you found. The report gives
the project-name finding, but not this enumeration. List what you checked and what you
found, even if the answer is none.

The project names' missing hover underline is accepted:

- hover still changes the colour;
- keyboard focus gets colour and underline;
- the "›" cue carries the meaning without hover, which is what D13 item 2 required.

## Evidence

**The question:** does an unavailable command now look less like a button than an
available one?

- The command palette with nothing selected: light and dark, English.
- Before (`4e571a1`) and after, same script.
- Save to `.git-exclude/evidence/rfc-071/stage-2-a5/`.

## Report

- the diff of the unavailable styles;
- the per-variant test;
- the hover-only enumeration;
- the CHANGELOG entry as committed;
- captures: paths, sizes, privacy inspection;
- gates, with `git diff --check 4e571a1..HEAD`.
