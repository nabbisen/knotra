# RFC-071 handoff — Amendment A3: text you read to act is never small

**Changes D1's scope, D9's density limit, and stage 3.** **Valid only once the owner approves
it**, including the density trade-off below. The owner handing this path to the dev team is
that approval. Everything else in the handoff, A1 and A2 stands.

## Why

The owner (2026-10-31): the main view's text is still too small for older people.

**Measured after stage 1** (`77f05c4`):

- `body_small` (14 px) is used at **137** sites in `view/` and `knotra-ui/src/widget/`;
  `body` (16 px) at 88.
- D1 enlarged only six kinds of text: titles, group headers, names, status lines and
  history titles.
- Most of what a user reads in order to act is still 14 px:
  - row action labels ("Choose resolution", `row.rs:339`, `:351`);
  - reasons under buttons (`buttons.rs:91`);
  - chip labels (`chip.rs:52`, `label` role, 14 px);
  - notices, field labels, the activity strip, dialog bodies.

For comparison, desktop operating systems set their own interface text smaller still:
about 12 px on Windows (9 pt), 13 px on macOS. So knotra is not small *for a desktop app*.
It is small *for its users*: the owner names older people, and the common advice for older
readers is 16 px or more for anything that must be read. A sound rule is better than
another per-site list.

## D14. Two sizes, by purpose

| Text | Role | Size |
|---|---|---|
| **anything read in order to act or understand**: statuses, reasons, button and chip labels, messages, notices, field labels, dialog bodies, list content | `body` | **16 px** |
| incidental metadata only: timestamps, the VCS kind, the branch beside a name, counts inside metadata, keyboard hints | `body_small` | 14 px |
| titles and headings | D1 | 18 / 24 px |

- **The default stays a rule, not a guess.** Every `body_small` site is classified once:
  - keep: incidental, with a one-line reason;
  - move: read to act.
- **The `label` role (14 px)** becomes `body` for buttons and chips, through the
  `knotra-ui` helpers, so no view chooses it.
- **A guard** (`text_size_guard`'s shape) pins the remaining `body_small` sites by file and
  count. A new small site then needs a deliberate change to the map. That is the drift this
  amendment exists to stop.

## D9, revised: readability over density

**The trade-off for the owner.** 16 px everywhere makes rows taller.

- **Today's D9:** at most one row lost at 1920×1080. That limit made size give way to
  density.
- **Revised:** stage 3's layout still removes every waste it can: one-line status, no extra
  padding, lines instead of panels. But **text size does not give way**. The row count is
  reported, and the count no longer stops the work.
- **My recommendation: accept it.** A project list a user can read beats a longer one they
  have to squint at. Zoom (D11) and the window's own size remain for anyone who wants more
  rows.

## Zoom is easy to find

Older users are the least likely to look for a setting.

- **Command palette:** "Zoom in", "Zoom out" and "Reset zoom" are listed, using D12's
  existing texts.
- **Settings:** the "Zoom" choice sits at the top of the Display section, not after the
  others.

No new text.

## Where it lands

**Stage 3**, together with layout and density. The zoom placement lands in stage 4.

- **Stage 3's question grows:** "…and is everything you read to act at least 16 px?"
- **Stage 3's evidence adds** the Dashboard and one dialog in Japanese at 1920 px, before
  and after.

## Report (added to stage 3's)

- The classification table: every `body_small` and `label` site, keep or move, with the
  reason for each keep.
- The guard and its map, seen to fail on a planted site.
- The row count at 1920×1080, before stage 1, after stage 1, and after stage 3.
