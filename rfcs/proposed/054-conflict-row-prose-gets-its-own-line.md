# RFC-054 - The conflict row's prose gets its own line

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | Medium - one of three states is unreadable at the minimum window width |
| Effort | Small - one row split, one classification made explicit |
| Target | Production Readiness Reset - UI/UX foundation |
| Related files | `crates/knotra-app/src/view/overlays/conflict.rs` |
| Related RFCs | `rfcs/done/051-...md` (**D4**, whose widening this completes), `rfcs/done/045-...md` (the jj hint), `rfcs/done/043-...md` (the comparison-tool control) |
| Owner decisions | Approved 2026-08-13 |

## Summary

The conflicted-file row puts three controls beside a filename. Two of the three possible
third-slot contents are **prose**, not buttons — one of them nearly twice the width of the
widest button. Widening the overlay helped and did not fix it.

Give prose its own line. Keep buttons inline.

## Problem

### It is a content-kind problem, and RFC-051 D4 treated it as a width problem

Handoff 070's arithmetic, at the 800px minimum window where `Large` resolves to 640px:

| Third slot | Kind | Est. width | Path share |
|---|---|---|---|
| `"Mark done"` (Git) | button | ≈74px | 244px |
| jj hint — `` Finish with: `jj resolve <path>` `` | **prose** | ≈222px | 96px |
| `"This action is available for Git projects only."` | **prose** | ≈288px | **30px** |

D4 changed the overlay from `Small` to `Large` on my recommendation. That was a real
improvement — the same third row overflowed the surface entirely at the old 400px — but no
width fixes a column whose content varies fourfold by state. **A row laid out for buttons
is being handed sentences.**

The estimates are the implementer's, flagged as estimates, and the conclusion does not
depend on their precision: the widest case is ~4× the narrowest whatever the constant.

### The `None` case is reachable, not hypothetical

Handoff 065 established it: a project in neither `workspace_status` nor
`workspace.projects`, or present with neither a `.git` nor `.jj` marker — which is what
`missing_projects` tracks.

## Non-goals

- Changing any wording. The three strings stay exactly as they are.
- Changing `OverlayWidth` or RFC-051's arithmetic.
- Two-line rows unconditionally. See D2.
- Revisiting which VCS gets which third slot - that is Handoff 065's three-way match and
  it is correct.

## Decision

### D1. The split is by content kind, not by VCS

A **button** stays inline on the first line. **Prose** renders on a second line beneath,
full width.

Today that means Git keeps one line and the two prose cases gain one — but the rule is
about what the content *is*, so a future third slot lands correctly without anyone
rederiving the reasoning.

### D2. No extra height in the common case

Git projects — the ordinary case — keep the single-line row they have now. Only the cases
that need a line get one, and they get it because prose needs width, not because rows
should be taller.

### D3. The prose line aligns with the path, not the icon

Indented past the 22px status glyph so it reads as belonging to that file rather than as a
new item in the list.

### D4. The classification is explicit, so it can be asserted

The third slot's kind — button or prose — must be visible in the code as a value, not
implied by which branch built which widget. Something like:

```rust
enum ThirdSlot<'a> { Button(Element<'a, Message>), Prose(&'a str) }
```

Then a test asserts `Some(Git) => Button`, `Some(Jujutsu) => Prose`, `None => Prose`, and
the layout follows from the kind mechanically.

**This is the point of the RFC, not decoration.** Without it the central claim — that
prose and buttons are laid out differently — is only observable by rendering, which
nobody here can do. With it, the classification is a unit test and only the pixels are
unverifiable.

## Requirements

| # | Requirement |
|---|---|
| R1 | A button-kind third slot renders inline on the first line |
| R2 | A prose-kind third slot renders on its own full-width line beneath, aligned with the path (D3) |
| R3 | Git rows are otherwise visually unchanged - same controls, same order, same single line |
| R4 | The 800px arithmetic is reported for both shapes: the path's share on line one, with and without a button in slot three |
| R5 | No catalog key is added, removed, or reworded |
| R6 | The three-way match over `Option<VcsKind>` stays exhaustive and wildcard-free (Handoff 065's R6) |
| R7 | D4's classification is a value a test can assert, and there is such a test |
| R8 | `crates/knotra-vcs` and `crates/knotra-app/src/tests.rs` are not modified |

## Test Plan

- R7's classification test: each of the three `Option<VcsKind>` cases maps to the expected
  kind. This is the only part of the change that is testable without rendering, which is
  exactly why D4 requires the kind to exist as a value.
- Handoff 065's three existing `conflict_vcs_kind_for_project` tests pass **unmodified** -
  this RFC does not touch that lookup.

No pixel assertions. The layout itself remains unverifiable here, and the report should
say so rather than imply otherwise.

## Security Considerations

None.

## Migration / rollout

No data or config change. Git users see no difference. Jujutsu users and users with an
unidentifiable project see the message on its own line instead of crushed into a column
beside a filename.
