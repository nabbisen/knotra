# RFC-071 handoff — Amendment A7: every setting is saved when it changes

**Adds to stage 4** (zoom). Approved by the owner on Review 258 §5, with the refinements of
Review 259 §1. Everything else in stage 4 stands.

## Why

Today:

- Settings applies a language or theme at once, but **writes it to disk only on "Save
  settings"** (`app/misc.rs:114-190`, `view/settings.rs:360`). Change the theme, close
  knotra, and the old theme returns.
- The dashboard's Group, Sort and collapse save themselves (`persist_dashboard_preferences`).
- Stage 4's zoom shortcuts save each change.

Three save behaviours, and the main one silently loses a choice the user saw take effect.

## The change

1. **Every setting is saved when it changes:**
   - language, theme and zoom (the Settings select and the shortcuts);
   - the file-monitoring switch;
   - each typed field (refresh interval, concurrent reads, log entries, detection interval,
     editor path, comparison tool path), **when its value is valid**.

   An invalid value is neither applied nor saved, and shows its existing error, as today.
   One function saves, called from every one of these paths. Name it in the report.
2. **The "Save settings" button goes** (`view/settings.rs:360`), with:
   - its success message ("Settings saved.");
   - `settings.save` and `settings.saved_ok`, from both catalogs.

   The failure message (`settings.save_error`) stays.
3. **A failed save is always shown,** wherever the change came from:
   - in the activity line (`status_bar`);
   - also in Settings when Settings is open.

   A zoom shortcut on the dashboard that fails to save must not fail silently.
4. **The restart hint stays as it is.** RFC-073 will say which settings need a restart.
5. **Tests:**
   - each kind of setting persists without any Save step (a select, the switch, a valid typed
     number, a typed path);
   - an invalid number is not persisted, and its error shows;
   - a failed save (an unwritable settings path) shows the error in the activity line, for a
     change made from Settings and for one made by a zoom shortcut;
   - the unused-key test passes after the two keys go.

## CHANGELOG `[Unreleased]`, exactly

```markdown
### Fixed — a theme or language you chose could be forgotten

Settings applied a new theme or language at once, but kept it only if you also pressed
"Save settings"; closing knotra without it brought the old choice back. Every setting is now
saved as soon as you change it, and the "Save settings" button is gone.
```

## Evidence

Folded into stage 4's capture set, with one more question: *after changing the theme in
Settings and restarting knotra with no other step, is the new theme still there?*

1. Change the theme in Settings.
2. Quit.
3. Relaunch with the same isolated XDG folders.
4. Capture the dashboard.

Report the `config.toml` line before and after.

## Report (added to stage 4's)

- the saving function, and every path that calls it;
- the removed button and keys (catalog diff);
- the failure path, with its tests;
- the restart capture and the `config.toml` lines;
- the CHANGELOG entry as committed.
