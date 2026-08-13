# RFC-039 - Per-project VCS history

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | Medium - the last roadmap feature; everything it depended on now exists |
| Effort | Medium - a new adapter call in three files, a cached load, one panel section |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-vcs/src/vcs/{adapter,git,jj}.rs`, `crates/knotra-vcs/src/model/changelog.rs`, `crates/knotra-app/src/view/detail_panel.rs`, `crates/knotra-app/src/state.rs`, `crates/knotra-app/src/message.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/038-...md` (**D3/R6** - `record_row`, extracted for this), `rfcs/done/048-...md` (which made `detail_panel.rs` a coherent place to add a section), `rfcs/done/046-...md` (**D1** - the field-with-two-meanings lesson D2 applies), `rfcs/done/044-...md` (**D3** - absent data is stated) |
| Blocked on | nothing, since RFC-048 |

## Summary

The project detail panel shows what **knotra** did to a project. It shows nothing about
what happened *in* the project. Add a "Recent commits" section beside "Recent operations".

## Problem

### The panel has one half of the story

`detail_panel.rs` renders Identity, Status, **Recent operations** (the last five knotra
operations touching this project), and Actions. A user asking "what changed here lately?"
gets knotra's own audit trail and nothing from the repository.

### `log_since` cannot answer "the last N commits"

`VcsAdapter::log_since(project, since_ref, until_ref)` builds `git log <since>..<until>`
(`git.rs:346`) and a `-r <revset>` for jj (`jj.rs:246`). **It requires a since-ref**, and
"the most recent five commits" has none. There is no existing call that answers this.

### `record_row` has been waiting for this

RFC-038 D3 extracted `(summary, detail: Option<_>)` into `knotra-ui` **specifically** so
this RFC could consume rather than copy it, and deliberately stopped there — no status
slot, no built-in disclosure — because RFC-039 did not exist yet to say what it needed.
This is where that restraint gets tested.

## Non-goals

- Full history browsing, paging, or search. Five entries, like its neighbour.
- Diffs, file lists, or commit bodies. Subject line only.
- Copying commits to the clipboard. `history.rs` has an export; this is a panel section.
- Any change to `log_since`, `collect_changelog`, or the changelog overlay.
- Configurable count. See D6.

## Decision

### D1. A new adapter call

`VcsAdapter::recent_commits(project, limit)`, dispatching to `git` and `jj` beside
`log_since`. Git is `git log -n <limit>`; **the jj invocation is D7's**.

### D2. It returns a distinct type, not `ProjectCommits`

`ProjectCommits` carries `since_ref: String`, which has no meaning for this query.
Reusing it would put a field with two meanings into the model — the exact defect RFC-046
spent an RFC removing from `skip_reason`, and the reason that one went unnoticed for so
long is that a `String` accommodates anything.

`CommitEntry` itself is right and is reused unchanged.

### D3. It renders as a `record_row` list in the detail panel

A "Recent commits" section beside "Recent operations". Summary line per commit: short
hash, subject, relative or short date. Detail — if the implementer finds one worth having
— is `record_row`'s `Option` slot, and **`None` is a legitimate answer**: RFC-038's
extraction says the caller decides whether a detail exists at all.

### D4. Loaded on panel open, cached per project

Not per render — a view function must not start work. The pattern to mirror is
`conflict_ops`'s: a per-project cache plus a phase, with the load triggered by
`DetailPanelMessage::Opened`.

`DetailPanelState` is currently one field (`open_project_id`). It grows.

### D5. Every non-success state is stated

Per RFC-044 D3, and by now the house rule: **loading**, **no commits yet** (a fresh
repository), and **could not be read** are three different sentences. None of them is an
empty section.

The VCS layer already returns an error string; the panel must not discard it the way
`ProjectConflictDetail.note` was discarded (RFC-045).

### D6. Five entries, a constant, not config

Matching "Recent operations" directly above it. A second knob in Settings for a panel
section is not worth its weight, and two adjacent lists showing different counts is worse
than either count.

### D7. The jj invocation is the implementer's to determine and report

`jj.rs:246` uses `-r <revset>` with a template, not a `-n` flag. **I cannot verify jj's
CLI surface from here**, and RFC-0003 already documents jj's CLI as a deliberate exception
requiring care.

Determine the correct invocation against the installed jj, **report the exact command and
how you verified it**, and if the two VCSs cannot express the same query, say so rather
than approximating one in terms of the other.

## Requirements

| # | Requirement |
|---|---|
| R1 | `recent_commits` exists for both VCSs and needs no since-ref |
| R2 | Its return type has no field that is meaningless for this query (D2) |
| R3 | The section uses `knotra_ui::widget::record_row`, not a second copy of that shape |
| R4 | The load is triggered by a message, cached per project, and never started from a view function |
| R5 | Loading, empty, and error are three distinct stated outcomes (D5) |
| R6 | Every new string is in **both** catalogs under `detail.*`; RFC-048's and RFC-049's guards stay green |
| R7 | The suppression map stays at **five** entries - a new lint is reported, not suppressed |
| R8 | The jj invocation is reported with its verification (D7) |
| R9 | `crates/knotra-app/src/tests.rs` is not edited |

## Test Plan

- `recent_commits` against a fixture repository per VCS: the entry count respects `limit`,
  and a repository with fewer commits than `limit` returns what it has rather than erroring.
- The three D5 states, driven from the cache/phase rather than asserted on strings in
  isolation - the coverage-then-content shape established in `062` and applied since.
- `record_row` consumption needs no new test; it has its own.

Expect a rise. This is the first RFC in several to add behaviour rather than correct it.

## Security Considerations

Commit subjects and author names are **repository-controlled text** rendered in the UI.
That is not new — the changelog overlay already renders them — but it is worth stating
that this puts them on a second surface.

Nothing here copies to the clipboard, so RFC-045 D3's shell-injection concern does not
apply. `limit` is an integer this code controls, not user input, so the command line is
not attacker-influenced.

## Migration / rollout

No data, config, or schema change. The panel gains a section; nothing existing moves.
