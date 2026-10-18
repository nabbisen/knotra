# Developer Handoff — RFC-065: the ready check lists only what is being released, and says why exactly

RFC: `rfcs/accepted/065-the-ready-check-lists-only-what-is-being-released.md`, **accepted
2026-10-19 (project owner), with Amendment A1**. Read the RFC in full first, including A1.

Baseline: the RFC-065 acceptance commit on `main`. **312** tests (231 / 32 / 49).

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard this is held to

The owner's bar: clean, safe, robust, and **never confusing to a user**. For this screen that
means one rule. **Every line the ready check shows is true, and is about the release being
made.** No project that is not being released; no reason guessed from a sentence; no check
that could not run reported as passed; no project silently missing.

## 1. Commit 1 — the model and its producers (`knotra-vcs`)

**`model/operation.rs`** (D1, D2):

- `FreezeBlocker` and `FreezeNote` as in RFC D2, deriving what the surrounding model types
  derive (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`), plus `PartialOrd, Ord` on
  `FreezeBlocker` if you use ordering for D3. Doc comments say what each variant means to a
  user, not how it is detected.
- `FreezeValidationEntry`: remove `included`, `is_clean`, `tag_exists`; `blockers:
  Vec<FreezeBlocker>`, `notes: Vec<FreezeNote>`. `ready()` is `!is_blocked()`.
- `FreezeValidation::{all_ready, blocked_count, ready_count}` lose their `included` filters.
- Export the new types from `lib.rs` beside the existing ones.

**R11 first:** before changing the types, confirm `FreezeValidation` and
`FreezeValidationEntry` are not written to history, config or any file. Report how you
checked. If they are persisted, **stop and report**.

**`vcs/adapter.rs::validate_freeze`** (D1, D4):

- Iterate **only the selected projects**, in workspace order. Unselected projects are never
  opened.
- Factor the "await each task, turn it into an entry" step into a function a test can call
  with handles it builds itself. A task that fails to join becomes an entry for **that
  project** with `Unreadable { reason }` carrying the join error. The function's contract,
  in its doc comment: one entry per project handed to it, in the order handed.
- The "no repository at …" case becomes `Unreadable { reason }`.

**`vcs/git.rs::validate_for_freeze`** and **`vcs/jj.rs::validate_for_freeze`** (D2, A1):

- Drop the `included` parameter.
- Emit typed blockers. Git: `UnresolvedConflict`, `UncommittedChanges`, `NameInUse`, and
  `FreezeNote::UntrackedFiles(n)` when `n > 0`. Jujutsu: `UnresolvedConflict`,
  `UncommittedChanges`, `NameInUse`; no notes.
- **A1.1:** every probe that fails becomes `Unreadable { reason }` naming the probe, never a
  passing value. That covers Git's `worktree_status`, `list_tags` and conflict probe, and
  Jujutsu's backend open, `is_dirty`, bookmark listing, and "detection unavailable". Remove
  every `.ok()` / `.unwrap_or(false)` / `.unwrap_or(0)` that turned a failure into a pass on
  this path. When a probe fails, do not also report blockers from probes whose result is
  unknown.
- **A1.1, `read_conflict_cli`:** return `detection_unavailable: true` when
  `rev-parse --git-dir` fails. Correct the freeze call site's comment ("always false for
  Git").
- **A1.2, Jujutsu names:** exact equality with **local** bookmark names, read from `jj`
  through a template that prints one name per line. Check the template syntax against the
  installed `jj` and report its version. Remote-tracking entries must not count.

## 2. Commit 2 — the view, the catalog, and the CHANGELOG (`knotra-app`, `knotra-ui`)

**`view/overlays/freezer.rs`** (D3, D5, R9):

- Delete `plain_blocker`. Map `FreezeBlocker` and `FreezeNote` with exhaustive `match`es,
  **no `_` arm**, in small functions next to the view.
  - `UnresolvedConflict` → `plain.release.blocker_needs_choice`
  - `UncommittedChanges` → `plain.release.blocker_unsaved`
  - `NameInUse` → `plain.release.blocker_name_used`
  - `Unreadable { .. }` → `plain.release.blocker_unreadable`
  - `UntrackedFiles(n)` → `plain.release.note_untracked` followed by ` {n}`, the
    "label: count" form.
- A blocked row: icon `!`, the project name, then **each** blocker's message on its own line,
  once each, in declaration order. A ready row: `✓`, the name, "Ready". Either kind then lists
  its notes, each on its own line: an untracked-files note is just as true on a blocked row.
- The excluded branch (`—`, "Not included") is gone.
- **"Show details" in the ready check:** only when at least one entry has an `Unreadable`
  blocker, add the same ghost "Show details"/"Hide details" button the result screen uses,
  sending the same message and reading the same `show_op_details`. The footer becomes
  Save, space, the details button, Cancel: Save stays where it is, the details button
  sits just before Cancel, and neither changes size or style. While it is on,
  each `Unreadable` row shows its `reason` beneath its message, in `body_small`. Nothing else
  is affected.
- Save's enabled state and its disabled reasons: **unchanged in behaviour** (R10).

**`state/topology.rs::warnings_for`**: use every entry; there is no `included` left to
filter on. Update its test helpers.

**`knotra-ui/src/i18n.rs`** (R8): remove `plain.release.row_excluded` from both catalogs. Add
the two keys from RFC D5, English and Japanese, **with the exact text in the RFC**. The
Japanese text is owner-accepted; do not reword it.

**`CHANGELOG.md`** `[Unreleased]` (R12), this text exactly:

```markdown
### Fixed — the release check listed projects you had not chosen

"Save release point" checks each project before saving the release point. The check
listed every project in the workspace, and showed problems in projects you had not
chosen as if they stopped the release. It now lists only the projects being saved,
shows every problem with each one rather than only the first, and tells you when new
files will not be part of the release point. A project that could not be checked is
now reported as such, instead of passing. For Jujutsu projects, a release name already
in use was reported as "Needs help", and a name such as v1 counted as taken when only
v10 existed; both are fixed.
```

## 3. Tests

In `knotra-vcs/tests/git_integration.rs` and unit tests where they fit:

1. **Selection only (R1):** a two-project workspace, one selected → exactly one entry, for
   that project.
2. **Typed reasons:** move the existing freeze tests to typed assertions
   (`blockers.contains(&FreezeBlocker::NameInUse)` and so on). A dirty repository with an
   existing tag reports **both** `UncommittedChanges` and `NameInUse` (R5).
3. **Untracked note (R7):** a clean repository with one untracked file is ready, with
   `UntrackedFiles(1)`.
4. **Nothing drops out (R6):** hand the factored collection function one normal handle and
   one that panics; assert two entries, the second `Unreadable` with the join error. **Seen
   to fail first**: point the same test at the old `if let Ok` logic, or a faithful copy of
   it, and report the failure.
5. **A failed probe never passes (R13):** at least one planted Git probe failure. One way is
   a project path that exists but is not a repository; any honest plant works. It must yield
   `Unreadable`, not a ready entry.
6. **`read_conflict_cli` (R14):** a path where `rev-parse --git-dir` fails reports
   `detection_unavailable: true`.
7. **Jujutsu exact names (R4, R15):** with `jj` present, `NameInUse` for an existing
   bookmark `v1`; **free** for `v1` when only `v10` exists. Skipped when `jj` is absent, like
   the suite's other jj tests. **Say in the report whether `jj` was present, so these actually
   ran.**

Any view-level test you add: the exhaustive `match` already enforces completeness at compile
time, so do not add a test that only restates it.

## 4. Screenshots (RFC test plan)

The ready check, light theme, **English and Japanese**, in Task 087's demo environment under
`/tmp/knotra-demo/`, with the Store set's selection (`mobile-app`, `shared-ui`). Then a second
pair with one blocked project selected as well (e.g. add `billing-service`), so a blocked row
with its reason is visible. **Task 087's privacy rules apply in full**: no username, real
path, hostname, notification or other window. Do not turn on "Show details" in any capture.
A real path would show there.

Save to `.git-exclude/evidence/rfc-065/{en,ja}/`, named `04-freezer.png` (the Store
replacement) and `04b-freezer-blocked.png`. 1920×1080, measured.

## 5. Out of scope

- What Freezer does: tag creation, rollback, push offer, impact warnings.
- Any other overlay, the dashboard's view code (A1's dashboard effect comes only through
  `read_conflict_cli`'s truthful flag), the selection model.
- The version bump and release prep.

## 6. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
cargo +1.91 publish --workspace --dry-run --locked
```

Suppression map **five**. The dead-code probe must still print **one** line; if removing the
string blockers changes that, report it. `git grep -n 'row_excluded\|plain_blocker' -- crates`
prints nothing.

## 7. What to report back

- R11's persistence check, and how it was made;
- the new types and the reshaped entry, as committed;
- each producer's diff, with every removed failure-to-pass fallback listed;
- the `jj` version and the bookmark template used;
- the view's mapping functions and the row construction;
- each §3 test, with the "seen to fail" output for test 4, and whether `jj` was present;
- the CHANGELOG entry as committed;
- the four screenshots: paths, measured dimensions, and confirmation of the privacy
  inspection;
- gate output, with the `diff --check` range, and the test count with its delta explained.
