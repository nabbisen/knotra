# Developer Handoff — RFC-067: stopping a fix attempt asks first

RFC: `rfcs/accepted/067-stopping-a-fix-attempt-asks-first.md`, **accepted 2026-10-21 (project
owner), with the Japanese of D3.** Read it in full first.

**Start after Task 095 is approved.** Baseline: `origin/main` at that point; report the commit
and its test count. Task 095 routes Escape and the scrim to each window's own close control
per phase. This handoff relies on that.

**This handoff is immutable.** Corrections after issue go in a new document.

## 0. The standard

One click must never discard a user's work. After this change, every path to
`git merge --abort` passes through a screen that says, in plain words, that edits will be
lost, and offers a way out that is the default.

## 1. One commit

### State — `state/conflict_ops.rs`

Add `ConflictPhase::ConfirmStop { project_id: ProjectId, detail: ProjectConflictDetail }`,
carrying the `Browsing` detail so "Keep fixing" restores the file list **exactly** as it was,
with no reload. Doc comment: what the user sees in this phase, not how it is reached.

### Messages — `message.rs`, `ConflictOpsMessage`

- `StopRequested(ProjectId)`: sent by "Stop this fix attempt". `Browsing` → `ConfirmStop`.
  Runs no command.
- `StopCancelled`: `ConfirmStop` → `Browsing` with the carried detail. Runs no command.
- `AbortMergeRequested(ProjectId)`: unchanged in what it does. **Sent only by "Stop and
  discard edits".** Its handler keeps every existing guard.

A `StopRequested` or `StopCancelled` arriving in any other phase does nothing.

### View — `view/overlays/conflict.rs`

**Browsing**, footer, as now except the stop control:

- "Stop this fix attempt" uses `reasoned(.., style::danger)`, sending
  `StopRequested(project_id)`.
- While `state.operation_interlock.is_busy()`, the message is `None` and the reason is
  `state.t("plain.activity.busy")`, the pattern `smart_pull.rs:234-242` uses.
- Shown only where it is now (Git, `MERGE_HEAD` present). Its place, the left of the footer,
  is unchanged.

**ConfirmStop**, in place of the file list:

- title `plain.resolve.stop_confirm_title`, then `plain.resolve.stop_confirm_body`, in the
  body styles the panel already uses;
- footer: Fill space, then "Keep fixing" (`style::ghost`, `StopCancelled`), then "Stop and
  discard edits" (`style::danger`, `AbortMergeRequested(project_id)`);
- the header close sends `StopCancelled`, **not** `PanelClosed`. `close_msg` becomes
  phase-dependent: `None` while `Operating`, `StopCancelled` in `ConfirmStop`, `PanelClosed`
  otherwise. Through Task 095, Escape and the scrim follow it.

Under RFC-066 (later), "Keep fixing" is this phase's entry. Nothing to do for that here.

### Catalog — `knotra-ui/src/i18n.rs`

The four keys of RFC D3, **exactly** as written there, in both catalogs. The Japanese is
owner-approved; do not reword it. `plain.resolve.stop_attempt` is unchanged.

### CHANGELOG `[Unreleased]`, exactly

```markdown
### Fixed — "Stop this fix attempt" discarded your edits without asking

In the Resolve panel, "Stop this fix attempt" abandoned the merge at once, and every edit
you had made to the conflicted files was lost, with no warning. It now asks first, says
that your edits will be lost, and cannot be pressed while another operation is running.
```

## 2. Tests

1. **R1, the first click runs nothing:** from `Browsing`, `StopRequested` gives `ConfirmStop`
   with the same detail, and returns `Task::none()`. Seen to fail first: point it at the
   baseline's button message (`AbortMergeRequested`) and report that the phase becomes
   `Operating`.
2. **R2, every way out keeps the attempt:** from `ConfirmStop`, `StopCancelled` gives
   `Browsing` with an equal detail. Escape, through `ShortcutMessage::Close`, does the same,
   and the panel stays open.
3. **R3, busy:** with the interlock held, the Browsing footer's stop control has no message,
   and its reason is `plain.activity.busy`. Test this through a view helper you extract, in
   the way existing view tests do, not by rendering.
4. **Stray messages:** `StopRequested` in `Loading` and `StopCancelled` in `Browsing` change
   nothing.
5. **Catalog:** the existing both-catalogs test covers the four keys. Add nothing that only
   restates a string.

## 3. Screenshots — review evidence only, not the Store set

In Task 087's demo environment under `/tmp/knotra-demo/` (`docs-site` is the conflicted
project), light theme, English and Japanese:

- `05-resolve.png`: the Resolve panel on its file list, showing the danger-styled button;
- `05b-resolve-confirm.png`: the confirmation.

Save to `.git-exclude/evidence/rfc-067/{en,ja}/`, 1920×1080. **Task 087's privacy rules
apply in full**: no username, real path, hostname, notification or other window. The panel
lists file paths, so **confirm every path shown is inside the demo project and relative**. If
an absolute path appears anywhere, stop and report before saving. **Never press "Stop and
discard edits"** in the demo; it would end the demo's conflict state. Do not commit the images.

## 4. Out of scope

Anything else in the panel; Jujutsu; RFC-066's focus work; any other window.

## 5. Verification

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <baseline>..HEAD
cargo +1.91 clippy -p knotra --bin knotra -- --force-warn dead_code
cargo +1.91 deny --manifest-path Cargo.toml check advisories
```

Suppression map **five**, dead-code probe **one** line, `advisories ok`. Explain the test
count delta.

## 6. What to report back

- the new phase and messages, as committed; the view's three phase-dependent pieces
  (stop control, `close_msg`, the confirm footer);
- each test, with test 1's seen-to-fail output;
- the CHANGELOG entry as committed;
- the four screenshots: paths, measured dimensions, your privacy inspection, and the
  relative-path check;
- gate output, with the `diff --check` range.
