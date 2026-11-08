# RFC-073 handoff — Amendment A3: three loose ends of stage 2, done with stage 3; a guard for setup scripts

**Adds to stage 3.** Ruling of Review 271. Stage 2 is approved. These are its loose ends,
folded into stage 3 to save a round. Do them as their own commit, ahead of stage 3's.
Everything else stands. No new text.

## 1. One status sign, everywhere (finishing D9)

The stage 2 report found three sites still drawing status as text. D9's principle is one sign
everywhere, so they join it:

- **The Resolve panel's "!"** beside an unresolved file (`overlays/conflict.rs`);
- **the activity line's "ⓘ" / "✗" / "⚠"** summary (`activity_strip.rs`; seen as "⚠ Get latest:
  4 succeeded, 1 failed, 1 skipped" in `stage-2/after/history-entry-open.png`);
- **the detail panel's recent operations "✓" / "✗"** (`detail_panel.rs`).

Each uses `status_badge` through the existing exhaustive functions:

- `badge_for_operation_outcome` where the value is an outcome;
- History's aggregate mapping for the activity line's summary: success, partial (`warning`),
  failure;
- an unresolved file is `danger` / alert.

Keep the words. If a site's states do not map cleanly, **stop and report**.

## 2. A text field is as tall as the buttons beside it (finishing D8)

In `stage-2/after/dashboard-light.png`, the search field is about 30 px tall beside 44 px
controls. With the row aligned by the top (A2 §4), it now sits visibly higher than "Arrange"
and "Select". A field you click into is a target like any other.

- Every `text_input` built through knotra's helpers or styled by `text_input_style` gets a
  height of `BUTTON_HEIGHT`, by padding, with the text vertically centred.
- **Extend the `button_height_guard` idea to fields:** every `text_input(` in `view/` and
  `widget/` names that height or goes through a helper that does. Seen to fail first.
- **Check every form:** Settings, Add project, the dialogs and the palette. Report any layout a
  taller field breaks.

## 3. Multi-line command output keeps its indent

In the same capture, a Git error's second and later lines ("hint: …") start at the left edge,
not under the first line. The indent is added to the start of the string only
(`format!("    {}", …)`), not to each line.

- History details, the Resolve panel, and any other place that prints command output indent
  **every line** the same: split on line breaks, or one `continuation_line`-style column.
- The copied log (plain text) indents each line too.
- Test it with a two-line stderr.

## 4. Setup scripts never touch the repository (a standing rule from now on)

The stage 2 capture run had a demo-setup script whose working directory was this repository
for one loop. It overwrote `README.md`, created `notes.txt`, and ran `git branch -M main` five
times on this repository (the reflog shows "renamed refs/heads/main to refs/heads/main" ×5 at
21:49). It was restored before anything was committed, and I verified the repository
afterwards (Review 271 §4).

**From now on, every evidence or fixture script:**

- starts with `set -euo pipefail`;
- works only under one absolute root it creates under `/tmp` or `/var/tmp`;
- **checks before every write** that the working directory is under that root, and stops
  otherwise. For example:
  `case "$(pwd -P)" in "$ROOT"/*) ;; *) echo "refusing: $(pwd -P)"; exit 1;; esac`;
- never `cd`s by a relative path;
- after the run, shows `git -C <repo> status --short` and `git -C <repo> reflog -3` in the
  report, proving the repository was not touched.

## Report (added to stage 3's)

- the three status sites, before and after;
- the field height, the guard seen to fail, and the forms checked;
- the indent fix and its test;
- for every script run: the guard line, and the repository's status and reflog afterwards.
