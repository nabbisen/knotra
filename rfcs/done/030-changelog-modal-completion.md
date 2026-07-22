# RFC-030 - Changelog Modal Completion

| Field | Value |
|---|---|
| Status | Implemented (working tree; pending commit) |
| Priority | High - the modal reaches a ready state but shows debug output and bypasses its intended copy route |
| Effort | Small |
| Target | Production Readiness Reset |
| Related files | `crates/knotra-app/src/state/changelog.rs`, `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/app.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-app/src/state/palette.rs`, `crates/knotra-vcs/src/model/changelog.rs`, `crates/knotra-vcs/src/vcs/adapter.rs`, `crates/knotra-vcs/src/vcs/git.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-ui/src/i18n.rs`, `rfcs/done/013-bulk-action-modals.md`, `rfcs/done/021-plain-language-layer.md`, `rfcs/done/027-selection-mode-and-bulk-selection-completion.md`, `rfcs/done/028-command-palette-action-completion.md` |
| Related audit evidence | `.git-exclude/reviewed/008-basic-function-rfc-overview-amended.md`, `.git-exclude/reviewed/010-reviewed-artifacts-consolidation.md`, `.git-exclude/reviewed/009-architect-001-prepare-review.md` |

## Summary

Complete the changelog modal as an honest production workflow. The app already
has changelog state, messages, a modal, a multi-project collector, Git and jj
`log_since` implementations, and `ChangelogDraft::to_markdown()`. However, the
ready state currently renders `format!("{:?}", draft)` in the modal and the
visible copy button copies that debug string directly through
`Message::CopyToClipboard`, bypassing `ChangelogMessage::CopyRequested`.

This RFC requires the modal to render a user-facing Markdown preview, route
copy through the changelog message path, show per-project errors and empty
states, validate selected projects and the starting ref, and clarify the Git/jj
history source constraints before the changelog action becomes visible from the
command palette again.

## Background

RFC-013 introduced bulk action modals, including a changelog workflow. RFC-021
established plain-language UI and i18n. RFC-027 completed selection semantics
for bulk actions. RFC-028 made the command palette hide `Generate changelog for
selected...` until this workflow becomes production-ready.

Current code has useful pieces:

- `ChangelogState` stores `since_ref`, selected projects, available tags, and
  phase.
- `ChangelogMessage` includes open, field changes, project toggle, generate,
  copy, and close messages.
- `handle_changelog` opens the modal, initializes project selection from the
  workspace, collects a draft through `VcsAdapter::collect_changelog`, and has
  a `CopyRequested` branch that writes `draft.to_markdown()` to the clipboard.
- `ChangelogDraft::to_markdown()` renders Markdown from collected
  `ProjectCommits`.
- Git collection runs `git log <since>..HEAD --format=... --no-merges`.
- jj collection runs `jj log -r <since>..@ --no-graph -T ...`.
- Per-project backend failures are represented as `ProjectCommits.error`.

The production gaps are visible to users:

- The modal ready state displays Rust debug output instead of release notes.
- The ready-state copy button copies the debug string and bypasses the
  changelog copy message path.
- The ready view skips user-facing project selection, per-project error
  presentation, and all-empty result messaging.
- The generate button only validates non-empty `since_ref`; it does not disable
  when no projects are selected.
- `available_tags` and `LoadTagsRequested` exist, but the modal does not expose
  a reviewed tag/loading/error contract.
- The command palette action remains hidden until this RFC defines the
  production-ready behavior.

## Motivation

Users generate changelogs to publish or paste release notes. Debug output is not
usable release content, and copying a different value than the intended
Markdown model breaks trust in the workflow.

Product readiness requires every visible control to work, be disabled with a
clear reason, or be hidden. For changelog generation, that means the modal must
explain what projects are included, why generation is blocked, which projects
had no changes, which projects failed, and what exactly will be copied.

Operationally, changelog collection reads repository history across many local
projects. It must not mutate repositories, must not use shell interpolation,
and must make Git and jj history-source limits explicit so users do not mistake
operation history for repository history.

## Requirements

### Functional

R1. The changelog modal must render `ChangelogDraft` as a user-facing Markdown
preview or a structured release-note preview. It must not render Rust `Debug`
or otherwise expose internal struct syntax.

R2. The preview source of truth is `ChangelogDraft::to_markdown()` unless the
implementation intentionally replaces it with an equivalent presenter function.
The copied text and preview text must represent the same draft.

R3. The ready-state copy button must dispatch
`Message::Changelog(ChangelogMessage::CopyRequested)` and must not directly
copy view-local text.

R4. `ChangelogMessage::CopyRequested` remains the single changelog copy route.
It writes Markdown through the existing clipboard task and updates status
feedback through localized text.

R5. The modal must disable `Generate notes` when `since_ref.trim()` is empty.
The disabled reason is a localized plain-language string.

R6. The modal must disable `Generate notes` when no project is selected. The
disabled reason is a localized plain-language string such as `Choose at least
one project`.

R7. The modal must render project inclusion controls for all active-workspace
projects that can be part of changelog collection, or it must otherwise show
which project set will be used before generation.

R8. Opening the modal from a selected-project entry point initializes inclusion
from the current selection. Opening it from a workspace-wide entry point
initializes inclusion from all active-workspace projects. The chosen entry-point
scope must be explicit in code and tests.

R9. The first completed implementation may expose only selected-project entry
points. If a workspace-wide entry point is not implemented, it must remain
hidden or disabled.

R10. While collection is running, the modal must show a stable collecting state
and must not let the user start a second concurrent collection for the same
modal state.

R11. When the draft is ready, the modal must show:

- total commit count;
- number of projects with commits;
- number of projects with collection errors;
- the Markdown preview;
- copy and close actions.

R12. A project with commits must appear in the preview under a project heading.

R13. A project with a backend error must appear as a user-facing per-project
error outside or inside the preview. It must not be silently omitted.

R14. A selected project with no commits and no error must be represented in the
modal result as a no-change project. It may be omitted from the copied
Markdown, but the modal must tell the user it was checked and had no changes.

R15. If all selected projects have no commits and no errors, the modal must
show an all-empty state and must not imply that generation failed.

R16. If all selected projects fail collection, the modal must show a failure
summary while preserving per-project errors and details. The copy action may be
disabled or may copy an error-summary draft, but the behavior must be explicit
and tested.

R17. Editing `since_ref` or project inclusion after a ready draft clears the
ready draft and returns the modal to an idle/dirty state so stale notes are not
copied.

R18. Closing the modal cancels no already-running process after collection has
started unless a cancellation mechanism is explicitly implemented. If closed
during collection, late background results must not reopen the modal or replace
state for a newer modal session.

R19. Available tags are optional for this RFC. If shown, tag loading must have
loading, empty, error, and selection behavior. If not shown, dead tag-loading
state must not create a visible incomplete control.

R20. The command palette `Generate changelog for selected...` action may become
visible only after the modal satisfies this RFC. It is enabled only when at
least one project is selected and dispatches the same modal open path as the
visible selection entry point.

R21. If the selection bar exposes a changelog action, it follows the same
availability and dispatch rules as the command palette.

R22. The changelog workflow reads repository history only. It must not write
tags, bookmarks, branches, commits, files, or operation history entries as part
of generation or copy.

### Non-Functional

N1. All new user-facing strings are routed through `knotra-ui` i18n in English
and Japanese.

N2. First-level text uses plain language: `Generate notes`, `Since`, `Copy to
clipboard`, `No changes found`, and `Some projects could not be checked` are
preferred over raw implementation terms.

N3. Technical Git/jj details may appear in details text or preview metadata
where useful, but not as the only explanation of a blocking state.

N4. The modal must remain keyboard accessible: focusable inputs, toggles,
generate, copy, and close actions all work without a pointer.

N5. The ready preview must be scrollable with stable dimensions and must not
resize the modal unpredictably based on draft length.

N6. Tests must prove that production UI does not contain Rust debug struct
syntax for a ready changelog draft.

N7. Changelog collection continues to use structured process execution through
`Command::new(...).args(...)`. No shell-interpolated command path may be added.

### Git and jj Behavior

G1. Git changelog generation reads commits from `git log <since>..HEAD` for
the selected project, excluding merge commits unless a later RFC changes that
policy.

G2. Git `since` accepts any local ref, tag, branch, or commit accepted by
`git log <since>..HEAD`. Invalid refs are reported as per-project errors.

G3. Git collection is per project. A ref that exists in one project and not
another must produce success for the first project and a per-project error for
the second.

G4. jj changelog generation reads changes from `jj log -r <since>..@` for the
selected project using the current workspace/current change as the upper bound.

G5. jj `since` accepts revsets, bookmarks, or change IDs accepted by the
current jj command. Invalid expressions are reported as per-project errors.

G6. The modal must label the starting point generically as `Since` or
`Starting point`, because the accepted value can be a Git tag/ref/commit or a
jj revset/bookmark/change ID depending on project type.

G7. Mixed Git and jj selections are allowed. The same `since` text is passed to
each backend, and each project reports success, no changes, or error
independently.

G8. This RFC does not replace the later per-project VCS history RFC. The
changelog modal is a generated release-note workflow, not a general repository
history browser.

## Goals

- Replace debug output with a user-facing preview.
- Make the preview and copied Markdown consistent.
- Route copy through `ChangelogMessage::CopyRequested`.
- Show project inclusion, no-change results, per-project errors, and all-empty
  results clearly.
- Keep command-palette and selection-bar changelog actions hidden or disabled
  until they dispatch this completed workflow.
- Preserve VCS-neutral UI language while documenting Git and jj source
  behavior.

## Non-Goals

- This RFC does not implement rich Markdown rendering with headings, bold, or
  inline code styling. A scrollable, user-facing Markdown text preview is
  sufficient for the first implementation.
- This RFC does not add automatic release-note grouping by labels, pull
  requests, conventional commits, or issue IDs.
- This RFC does not design templates for different release-note formats.
- This RFC does not add changelog persistence or file export.
- This RFC does not complete the later per-project VCS history workflow.
- This RFC does not change Smart Pull, Freezer, context switching, or conflict
  resolution behavior.

## External Design

### Entry Points

The first production entry point is selected-project changelog generation:

- selection bar `Generate notes`, if exposed;
- command palette `Generate changelog for selected...`, once this RFC is
  implemented.

Both entry points are enabled only when at least one project is selected. With
no selection, the action is disabled with a localized reason such as `Choose at
least one project`. If no entry point is exposed yet, the command palette row
remains hidden as defined by RFC-028.

### Idle State

The modal opens with:

- title: `Generate notes`;
- `Since` input with placeholder such as `v1.2.0`;
- selected project list with inclusion toggles;
- disabled or enabled `Generate notes` button;
- close action.

If `Since` is empty, `Generate notes` is disabled with the existing localized
reason. If every project is deselected, it is disabled with the no-project
reason. If both are invalid, the modal should show the most immediately useful
reason; tests only need to prove that the action is disabled.

### Collecting State

After `Generate notes`, the modal shows a collecting state with stable layout.
The `Since` field and project toggles are disabled or treated as editing a new
draft only after collection completes. The implementation must avoid copying a
previous draft while a new collection is in progress.

### Ready State

The ready state shows a summary and preview:

```text
8 commits from 3 projects
1 project could not be checked

# Changelog - v1.2.0
...
```

The preview can be plain text containing Markdown. It must be readable as
release notes and must not contain struct names such as `ChangelogDraft`,
`ProjectCommits`, or Rust field-debug syntax.

Copy copies the generated Markdown and leaves the modal open with status
feedback. Close dismisses the modal.

### Empty And Error States

If no selected project has commits and no selected project errors, the modal
shows `No changes found since <since>` or equivalent. Copy may be disabled or
may copy a minimal empty changelog; the chosen behavior must be tested.

If some projects fail, the modal shows `Some projects could not be checked`
with per-project details and still lets the user copy notes for successful
projects.

If all projects fail, the modal shows a failure state with per-project details.
The app must not present an empty changelog as success.

## Internal Design

### State

`ChangelogPhase` may remain `Idle`, `Collecting`, and `Ready(ChangelogDraft)`
if the view can derive all required summaries from the draft and selection
state. If stale background results become possible, add a request/session ID so
late results can be ignored.

`ChangelogState::is_ready_to_collect()` should become part of the production
contract instead of being dead-code-suppressed. It validates non-empty `since`
and at least one selected project.

### Messages

`ChangelogMessage::CopyRequested` remains the only copy request for changelog
content. The view must use it from the ready-state copy button.

If project selection is visible, `ProjectToggled(ProjectId, bool)` must reset a
ready draft to idle or mark it stale so the app cannot copy notes for a
different project set.

### View

`changelog_modal` replaces `format!("{:?}", draft)` with a preview string from
`draft.to_markdown()` or a dedicated presentation helper.

The modal derives summary counts from `draft.projects`:

- total commits from `draft.total_commits()`;
- projects with commits;
- projects with no commits and no error;
- projects with `error.is_some()`.

Per-project errors and no-change projects must be visible in the ready state,
even if `to_markdown()` omits no-change projects from copied notes.

### VCS

`VcsAdapter::collect_changelog` can remain the multi-project collector. It
must preserve one `ProjectCommits` result per selected project whenever
possible, including no-repository and backend-error cases.

The Git and jj adapters must continue to return errors as data in
`ProjectCommits.error` instead of panicking or collapsing the full multi-project
draft.

### Palette And Selection

When implemented, palette and selection-bar changelog actions dispatch into the
same `ChangelogMessage::OpenRequested` flow with selection initialized from
`state.selection.selected_ids`. They must not invent a second changelog state
or call the VCS adapter directly.

If the implementation cannot safely initialize from selection yet, the palette
action remains hidden and the selection-bar action remains absent.

## Security Considerations

Changelog generation is read-only. It must not mutate repository state or write
files.

The `since` value is user input passed as a structured argument to Git or jj.
The implementation must keep using argument vectors and must not pass it
through a shell.

Backend stderr may include repository paths or technical ref names. First-level
UI should summarize errors plainly and keep raw stderr in details where
practical.

The copied Markdown may include commit subjects and author names from local
repositories. That is expected, but the UI should make it clear that copy uses
generated release notes from selected repositories.

## Test Plan

### Unit And Domain Tests

- `ChangelogState::is_ready_to_collect()` returns false for empty `since`.
- `ChangelogState::is_ready_to_collect()` returns false when all projects are
  deselected.
- Editing `since_ref` or project inclusion clears or stales a ready draft.
- `ChangelogDraft::to_markdown()` includes projects with commits.
- The chosen empty-draft behavior is tested.
- Per-project errors are represented in the ready-state presenter.

### UI Contract Tests

- Ready modal text uses Markdown preview output and does not contain
  `ChangelogDraft`, `ProjectCommits`, or Rust field-debug syntax.
- Ready copy button dispatches `ChangelogMessage::CopyRequested`, not
  `Message::CopyToClipboard` with view-local content.
- Generate is disabled with empty `since`.
- Generate is disabled with zero selected projects.
- Per-project no-change and error states are visible in the ready modal.
- Command-palette changelog action is either hidden or dispatches the completed
  modal open path with selected projects.

### VCS Integration Tests

- Git project with commits after a tag returns commit entries.
- Git project with an invalid `since` ref returns a per-project error.
- Git project with no commits after `since` returns no entries and no error.
- jj project with a valid revset/bookmark/change ID returns entries where the
  local test environment supports jj.
- jj project with an invalid expression returns a per-project error.
- Mixed Git/jj collection preserves one result per selected project.

### i18n Tests

- English and Japanese catalogs contain every new changelog key.
- No new first-level changelog strings are hardcoded in `bulk_modals.rs` or
  `app.rs`.

### Commands

Run at least:

```sh
cargo fmt --all --check
cargo test -p knotra-vcs
cargo test -p knotra-ui
cargo test -p knotra
```

If the repository's active release gate changes before implementation, use the
current gate and record exact command output in the implementation review
package.

## Acceptance Criteria

- The proposed RFC is reviewed and moved to `rfcs/done/` before
  implementation begins.
- The changelog modal no longer renders debug output in ready state.
- Preview text and copied text come from the same Markdown draft.
- The ready-state copy button routes through `ChangelogMessage::CopyRequested`.
- Generate is disabled for empty `since` and for zero selected projects.
- Project inclusion is visible or otherwise explicitly summarized before
  generation.
- Per-project errors are shown.
- No-change projects and all-empty results are shown.
- Git and jj invalid starting points are reported as per-project errors.
- The command palette changelog action remains hidden until this completed
  modal can be opened honestly; after it becomes visible, it dispatches the
  completed modal path.
- All new user-facing strings are localized in English and Japanese.
- No shell-interpolated command path is added.
- Tests or review evidence prove visible control to message to handler to task
  behavior for generate and copy.
- Current gate evidence is recorded before the RFC is marked implemented.
