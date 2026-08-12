# RFC-045 - Jujutsu conflict guidance

| Field | Value |
|---|---|
| Status | Implemented (main: bf7aec9) - amended 2026-08-12 (A1, architect); see Amendments |
| Priority | Medium-high - a dead end for every Jujutsu user who hits a conflicted file |
| Effort | Small |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-app/src/view/overlays/conflict.rs`, `crates/knotra-vcs/src/model/conflict.rs`, `crates/knotra-vcs/src/vcs/jj.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/046-...md` (**D1** - the contract this applies), `rfcs/done/026-...md` (editor-launch hardening), `rfcs/done/0003-...md` (jj CLI exception) |
| Found by | the dev team, out of scope, in Review Request 059 §3 |

## Summary

knotra tells Jujutsu users what it **cannot** do and never what they **should** do. The
sentence that would complete the workflow is computed by the VCS layer, stored in a
field, and never read by anything.

Delete the field, and let the view say the useful thing instead.

## Problem

### The panel works for jj, right up to the last step

`jj.rs:440` runs `jj resolve --list`, so the Resolve panel lists a jj project's
conflicted files correctly. Both launch controls — "Open in editor" and "Open in
comparison tool" — are gated on **configuration**, not on VCS kind, so they work for jj
too.

Only `mark_control` is Git-only (`conflict.rs:34`, `status.identity.vcs_kind == VcsKind::Git`).
For a jj project that slot renders:

> "This action is available for Git projects only."

Which is true, and is the end of the conversation. The user has opened the file, edited
it, and is now told the one remaining button is not for them — with no indication that
`jj resolve <file>` is what finishes the job.

### The answer already exists, one layer down, and is discarded

`jj.rs:468` sets:

```rust
note: Some("Use `jj resolve <file>` or your merge tool.".to_owned()),
```

`ProjectConflictDetail.note` is declared at `model/conflict.rs:45` and constructed at
eleven sites. **Ten set `None`. Nothing anywhere reads it.** No view, no handler, no test.

It is invisible to every gate we run: the field is `pub` and constructed, so `dead_code`
never fires, and no test asserts it reaches a surface.

### Rendering it as-is would be a regression, not a fix

The stored value is an English sentence. RFC-046 D1 established the house contract one
RFC ago: **records store codes, views render text**, because a rendered sentence outlives
the locale that produced it. `note` is the same anti-pattern in a different struct —
plumbing it to the view would put English in front of a Japanese user.

Half of what it says is also now redundant: "or your merge tool" describes a control the
panel has offered since RFC-043's last item shipped.

## Non-goals

- **Running `jj resolve` from knotra.** It is interactive — it launches a resolution tool
  itself — and RFC-0003 already documents jj's CLI as a deliberate exception. A GUI
  spawning an interactive resolver is a separate design with its own failure modes.
- **"Stop this fix attempt"** (`abort_supported`). jj has no equivalent worth naming in
  that slot; leave it.
- Git-side wording. Unchanged.
- Any other `knotra-vcs` change beyond removing the field.

## Decision

### D1. Delete `ProjectConflictDetail.note`

Never read, at any point in its life. A general channel — "non-fatal note from the VCS
layer" — that was never used generally, and whose one use is prose in a record.

If the VCS layer ever does need to pass a note, it passes **a code**, per RFC-046 D1.
That is now the house rule and it should not be rediscovered a third time.

### D2. The view derives the hint from `VcsKind`

`conflict.rs` already reads `status.identity.vcs_kind` to compute `git_actions_supported`.
It has the input; nothing needs to flow from `knotra-vcs`.

For a Jujutsu project, the per-file row states the completing action and names the
command **with that file's path**, in place of the bare "available for Git projects only."

`VcsKind` is `Git | Jujutsu` — closed, two variants. **Match it exhaustively** so that a
third VCS is a compile error rather than a silent fallthrough to Git's wording (R6).

### D3. Offer to copy the command — **only with the path shell-quoted**

`Message::CopyToClipboard(String)` already exists (`message.rs:41`) and the history export
uses it. The command contains a path the user would otherwise retype by hand.

**The security condition is not optional.** The path comes from `jj resolve --list`
output, i.e. from filenames in a repository the user may have cloned. A file named

```
x; rm -rf ~
```

produces a copied string that is a **working shell command** the moment it is pasted into
a terminal. knotra never executes it — but the copy affordance is precisely a mechanism
for getting text into a shell.

So: single-quote the path with embedded quotes escaped, or **do not ship the copy
control** and display the command only. Displaying is safe; copying is what needs the
care. An implementer who cannot do the quoting reliably should drop D3, not approximate
it.

### D4. Wording constraints

The hint lives under `plain.`, so `first_level_wording_has_no_developer_jargon` applies.
**"conflict", "merge", and "execute" are forbidden**; `resolve` and `jj` are not on the
list, so naming the command is allowed.

## Requirements

| # | Requirement |
|---|---|
| R1 | `note` is removed from `ProjectConflictDetail` and from all eleven construction sites; nothing depended on it |
| R2 | A Jujutsu project's per-file row states the completing action and names the command with that file's path |
| R3 | A Git project's rows are **unchanged** |
| R4 | New keys exist in both catalogs; both catalog guards and the jargon guard stay green |
| R5 | `plain.activity.copy_command_sent` is **orphaned today** - present in both catalogs, zero code referents. It is either adopted by D3 or removed. Not left as-is |
| R6 | `VcsKind` is matched exhaustively; a future third variant fails to compile |
| R7 | If D3 ships, the path is shell-quoted, and a test covers a path containing a quote and a semicolon |
| R8 | **Amended - see A1.** Co-located tests. `tests.rs` is edited only to delete the two now-invalid `note: None,` lines |

## Amendments

### A1. R8 as written is unsatisfiable (2026-08-12, architect)

**Recorded before implementation, while scoping the handoff.**

R8 said `tests.rs` is not edited. But `ProjectConflictDetail` is constructed at two sites
in that file - `tests.rs:2740` and `:2785` - each setting `note: None`. **D1 removes the
field, which makes both lines a compile error.** The RFC forbade the edit its own decision
requires.

**R8 becomes**: `tests.rs` is edited only to delete those two `note: None,` lines. No
assertion changes, no fixture semantics change. If anything else in that file needs
touching, that is a signal to stop, not to widen the exception.

Recorded rather than fixed silently because this is the fifth requirement in this stretch
that held in the sentence I wrote and failed against the tree - after RFC-044 D1, Handoff
058 SS8, Handoff 061 SS4, and Handoff 063 SS6. It is the first caught before issue rather
than by the dev team, which is the only thing that distinguishes it.

## Test Plan

- The jj hint renders for a Jujutsu project and the Git wording for a Git project — two
  tests, driven by `VcsKind` rather than asserting a string in isolation.
- R7's quoting test, if D3 ships: `x; rm -rf ~` and a path containing `'` must both
  produce a single safe shell word.
- R1 is covered by the compiler: removing a `pub` field breaks every construction site
  that still sets it.

## Security Considerations

**D3 is the whole of it, and it is real.** The copy affordance moves repository-controlled
text toward a shell. Quoting is the mitigation; dropping the copy control is the
acceptable alternative. Displaying the command carries no such risk, and knotra executes
nothing either way.

Deleting `note` removes a channel that carried unvalidated VCS-layer text toward the UI
without ever being rendered — a latent version of the same exposure.

## Migration / rollout

No data change; `ProjectConflictDetail` is constructed per read and never persisted.

User-visible: Jujutsu users gain an actionable next step where they previously got a dead
end. Git users see nothing different.
