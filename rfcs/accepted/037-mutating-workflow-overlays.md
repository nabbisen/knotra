# RFC-037 - Mutating Workflow Overlays and Remaining Ad Hoc Layers

| Field | Value |
|---|---|
| Status | Accepted (2026-08-10, project owner) - implementation authorised, not yet shipped. D5 accepted as recommended: the RFC takes all nineteen `guided_*` call sites and deletes the helpers |
| Priority | High - the last large user-visible surface still on pre-RFC-034 primitives, and the only RFC that can close the `guided_button` parallel-systems window |
| Effort | Large - five overlays, ~1,337 ELOC, safety-critical state machines that must not move |
| Target | Production Readiness Reset - UI/UX foundation track |
| Related files | `crates/knotra-app/src/view/bulk_modals.rs`, `crates/knotra-app/src/view/add_project_modal.rs`, `crates/knotra-app/src/view/workspace_manager.rs`, `crates/knotra-app/src/view/dashboard/empty.rs` |
| Related RFCs | `rfcs/done/033-ui-ux-foundation-shell-and-overlay-contracts.md` (**H3**, not H4 - see Background), `rfcs/done/034-design-foundation-shell-and-overlay-host.md` (D3 primitives, R7, R8 overlay host), `rfcs/done/029`, `rfcs/done/030`, `rfcs/done/031` (the invariants that must survive) |

## Summary

`view/bulk_modals.rs` is **1,337 ELOC** and renders all five mutating workflow
overlays - Smart Pull, Freezer, context switch, conflict resolution, changelog -
through a locally defined `modal_shell` that predates RFC-034's overlay host. This
RFC migrates them onto the RFC-034 primitives **without touching their state
machines**, and splits the file while doing it.

It is also the only scheduled RFC that can close the `guided_button` /
`guided_field` parallel-systems window RFC-034 R7 opened - if it takes the eight
call sites currently owned by nobody. See D5; that one is the owner's call.

## Background

### Read RFC-033 H3, not H4

**RFC-033's section headings are stale by one and will send an implementer to the
wrong specification.** RFC-033 labels its child sections:

| Section | Heading in RFC-033 | Actually |
|---|---|---|
| H3 | "RFC-036 - mutating workflow overlays" | **this RFC, 037** |
| H4 | "RFC-037 - settings and history" | RFC-038 |

RFC-036 was taken by keyboard navigation, inserted after the July 2026 spike found
no Tab traversal existed anywhere (`073`), which shifted every later number by one.
`ROADMAP.md` carries the corrected numbering; RFC-033's headings were never updated.

**H3 is this RFC's specification.** Anyone who follows H4's heading will build
Settings and History instead.

### The measured problem

| Overlay | `bulk_modals.rs` lines |
|---|---|
| `pull_modal` - Smart Pull | 75-347 |
| `tag_modal` - Freezer | 528-816 |
| `switch_modal` - context switch | 845-1036 |
| `resolve_panel` - conflict resolution | 1037-1236 |
| `changelog_modal` | 1276-end |
| `modal_shell` - the ad hoc layer | 41-74 |

At 1,337 ELOC it is the largest module in `knotra-app` and the second largest in the
workspace after `knotra-ui/src/i18n.rs`. Five independent overlays share one file
and one hand-rolled shell.

`modal_shell` is "the remaining ad hoc layer" the RFC title names. It builds its own
header and close affordance from `button(text("✕"))` with local padding constants -
exactly what RFC-034 R8's overlay host exists to provide once, with a scrim, bounded
width tokens, a stable header/body/footer, focus entry, and a focus trap.

### What makes this verifiable, and what does not

RFC-040 and RFC-041 established byte-identity as this project's proof that a
restructuring changed nothing. **That technique does not survive a migration** - the
whole point of a migration is that the code changes, so nothing can be diffed
against an original.

The technique still applies to *part* of this work, which is why D2 splits the RFC
in two rather than doing both at once.

## Motivation

1. **The last large surface on old primitives.** RFC-034 shipped the design system;
   RFC-035 migrated the dashboard. These five overlays are what remains, and they
   are where the application's riskiest operations are driven from.
2. **Nothing else can close R7.** `guided_button` and `guided_field` survive because
   their last callers do. RFC-038 owns Settings and History, neither of which calls
   them. If this RFC does not take the orphans, the helpers are undeletable.
3. **1,337 ELOC in one file** is the largest single-file concentration outside the
   i18n catalog, and it is over the threshold RFC-041 just finished enforcing in
   `app/`.

## Non-goals

- **No state machine changes.** Not one. D3 states this as a requirement with a
  test-based proof.
- **No `knotra-vcs` changes.** RFC-033 H5: if a UI RFC needs one, that is a scope
  error - stop and raise it.
- **Settings and History** - `view/settings.rs` and `view/history.rs` are RFC-038's,
  per RFC-033 H4. That includes `settings.rs`'s hard-coded English. `ROADMAP.md`
  attributes that to this RFC; the roadmap is wrong and is being corrected alongside
  this draft.
- **No behaviour change to what the overlays do.** Presentation only.
- **`tests.rs` is not edited.** It has survived RFC-040, RFC-035, RFC-041, and
  Handoffs 033-040.

## Decision

### D1. Migrate onto RFC-034's primitives and overlay host

Each overlay's chrome comes from the R8 overlay host - scrim, bounded surface,
header/body/footer, focus entry and trap - and its controls from the D3 primitives.
`modal_shell` is deleted when its last caller migrates.

### D2. Split first as a pure move, then migrate - not both at once

**Stage 1 is a pure move with no other change**, so it can be verified by the
byte-identity technique RFC-040 and RFC-041 proved. `bulk_modals.rs` becomes
`view/overlays/` with one file per overlay plus the shared shell. Every migration
stage then works inside one small file.

Doing the split and the migration together would produce a diff nobody can verify:
the migration justifies every changed line, so a mistake hides in plain sight.
Separating them means Stage 1 is provably inert and each later stage is small enough
to read.

### D3. The state machines do not move, and zero test changes is the proof

RFC-033 H3's recommended proof, adopted as a requirement: run the app suite before
and after each overlay's migration and require **zero test changes**. A test that
needs editing means the migration changed behaviour it was not supposed to change -
stop and report rather than adjust the test.

The invariants at risk, with anchors verified present in the tree today:

| Source | Invariant | Anchor |
|---|---|---|
| RFC-029 | close and Escape are inert while switching | `app/context.rs:238`, `:245` guard on `ContextPhase::Switching` |
| RFC-031 | cancellable preparation releases its exact lease on every close route and ignores late completion | 19 `release_if_matches` call sites |
| RFC-031 | non-cancellable execution and tag push disable every close affordance | `tag_modal`, `switch_modal` |
| RFC-030 | changelog request-id guard and `Collecting`-phase field policy | `state/changelog.rs:25` `active_request_id`, `app/changelog.rs:93` |

**Correction, 2026-08-10, before Stage 2 was handed off.** This paragraph
originally read: *"The overlay host provides Escape handling. That is precisely
where RFC-029's 'Escape is inert while switching' can be silently lost."* Checked
while scoping Stage 2, and **it is wrong.**

Escape does not reach the overlays. `app.rs:105` maps it to
`ShortcutMessage::Close`, which calls `focus_ops::close_topmost_layer`
(`app/focus_ops.rs:257`). That function already holds every phase guard centrally -
`smart_pull_is_running`, `freezer_is_running`, `context_switch_is_running`,
`conflict_is_running` all return early - plus the two cancellable special cases for
`SyncPhase::RetryPreparing` and `FreezerPhase::Validating`. **It lives in `app/`,
which R3 forbids this RFC from touching**, so the Escape guards are structurally out
of reach of a view-layer migration.

The real residual risk is narrower and worth stating accurately: each overlay gates
its own close *affordance* in the view, via `on_press_maybe(close_msg)` where
`close_msg` is `None` during a non-cancellable phase. A migration to a primitive
that always renders a close button would drop that gating. Even then the handler
re-checks - `conflict_ops.rs:268` returns early on `Operating` - so the failure mode
is a clickable button that does nothing, not a cancelled operation.

R5 stands unchanged: re-check each overlay's close routes per overlay. The reason is
now the view-level affordance gating, not Escape.

### D4. Conflict resolution stays a sheet

RFC-033 H3 and RFC-034 D3. `resolve_panel` benefits from remaining visible against
project context. It migrates to the sheet primitive, not the dialog primitive.

### D5. The `guided_*` orphans - owner decision

Measured at `fe09ff9`, **19 live call sites across four files**:

| File | `guided_button` | `guided_field*` | Owner |
|---|---|---|---|
| `view/bulk_modals.rs` | 7 | 4 | **this RFC** |
| `view/add_project_modal.rs` | 2 | 2 | none |
| `view/workspace_manager.rs` | 0 | 2 | none |
| `view/dashboard/empty.rs` | 2 | 0 | RFC-035, which closed without migrating them |

RFC-034 R7 says the helpers are deleted when their last caller migrates. **After this
RFC migrates its eleven, eight remain in three files that no scheduled RFC owns** -
RFC-038 is Settings and History, RFC-039 is per-project VCS history, and neither
touches them.

**Recommendation: this RFC takes all nineteen**, as a final stage, and deletes
`guided_button` and `guided_field`. The three orphan files total 529 ELOC and eight
call sites; absorbing them is small, and the alternative is a roadmap item that can
never close because nothing owns it.

**Against**: `add_project_modal.rs` and `workspace_manager.rs` are workspace
management, not mutating workflow overlays, so this widens the RFC's stated scope.

Recorded as a decision for the owner because it is a scope question, not a technical
one. If declined, R9 drops and the orphans need their own scheduled work.

Also noted rather than buried: **RFC-035's scope included the dashboard, and it
closed leaving two call sites in `view/dashboard/empty.rs`.** Not a defect in
anything shipped - the helpers still work - but it is why the count is four files
rather than three.

### D6. `guided_field` is not deletable - RFC-034 never built its replacement

**Added 2026-08-10, after Stage 3 surfaced it.** D5 and R9 originally assumed both
legacy helpers could be retired together. Checked at `9fb823b`:

| Module | Contents |
|---|---|
| `knotra-ui/src/widget/buttons.rs` | `guided_button` **plus** `primary`, `secondary`, `ghost`, `danger`, each with a `_maybe` variant, plus a `style` module |
| `knotra-ui/src/widget/field.rs` | `guided_field`, `guided_field_focused`. **Nothing else.** |

RFC-034 R7 said new controls are "added alongside `guided_button` and
`guided_field`". **That happened for buttons and never happened for fields.** There
is no primitive to migrate a text field *to*.

The consequence is that "migrating" a field can only mean inlining the same
`column![text(label), text_input(...)]` composition at the call site, which
duplicates what the helper centralises and entrenches a third pattern - the
opposite of what R7 exists to achieve. Corroborating evidence: `workspace_manager.rs`,
which was RFC-034 R9's own validating migration, still calls `guided_field_focused`.

**Therefore:**

- `guided_button` is deleted in Stage 6 as planned. Its replacement vocabulary is
  complete, and six of its eleven call sites are in this RFC's own files.
- **`guided_field` stays.** It is the field vocabulary, not a legacy helper, and the
  "legacy" label applied to it was wrong. Renaming it, or building a richer field
  primitive, is a separate concern and out of scope here.
- Stage 3 inlined one field composition in `changelog.rs` on a handoff instruction
  that assumed a target existed. It is reverted at the start of Stage 4 (`134` §4).

`guided_field*` call sites at `9fb823b`, all of which now stay: `add_project_modal.rs`
2, `workspace_manager.rs` 2, `overlays/freezer.rs` 2, `overlays/context_switch.rs` 1.

### D7. `guided_button` migration is deferred to Stage 6, where its missing half is built once

**Added 2026-08-10, following D6.** `guided_field` is not the only helper whose
replacement is incomplete - `guided_button`'s is too, in a narrower way.

`buttons.rs` fully replaces `guided_button`'s *styling*: `primary_maybe`,
`ghost_maybe` and friends all take `Option<Message>` and render a disabled state.
What none of them carries is `guided_button`'s other half - the **reason text
rendered beneath the button when it is disabled**:

```rust
match reason {
    Some(r) if show_reason => column![btn, text(r).size(FONT_SMALL)].spacing(6).into(),
    _ => btn,
}
```

So migrating a `guided_button` that passes a reason means re-implementing that
composition locally. Stage 3 did exactly that in `changelog.rs`, as
`reasoned_button`. Four overlays still hold `guided_button` call sites - `freezer`
2, `smart_pull` 2, `conflict` 1, `context_switch` 1 - so migrating them stage by
stage produces up to four more local copies of the same nine lines.

**This differs from D6 in one way that matters.** For fields there is no gain at all
- inlining buys nothing. For buttons the gain is real: token-aware styling and a
focus ring, which is what RFC-034 exists to deliver. The problem is only the
duplication of the reason composition.

**Therefore:**

- **Stages 4 and 5 migrate chrome and styling only, and leave `guided_button` call
  sites alone**, as Stage 2 already did for `conflict.rs`.
- **Stage 6 adds the reason-carrying button to `knotra-ui` once**, migrates every
  remaining `guided_button` call site onto it, replaces `changelog.rs`'s local
  `reasoned_button` with it, and only then deletes `guided_button`.

This concentrates the one `knotra-ui` addition in a single stage instead of
scattering four private copies and reconciling them afterwards, and it keeps Stages
4 and 5 to pure view-layer work.

## Requirements

| # | Requirement |
|---|---|
| R1 | Every file in `view/overlays/` is under 500 ELOC |
| R2 | Stage 1 is a pure move - byte-identity evidence per moved item, per RFC-041 R5 |
| R3 | No state machine is modified. No file under `app/` or `state/` changes except where a message variant is genuinely unused after migration |
| R4 | **Zero test changes.** If a test needs editing, stop and report before editing it |
| R5 | Each overlay's close routes are re-verified against its phase guards after migration, and the review request states the result per overlay - not as a blanket claim |
| R6 | `modal_shell` is deleted when its last caller migrates |
| R7 | Conflict resolution renders as a sheet, not a dialog |
| R8 | `tests.rs` is not edited - zero lines |
| R9 | `guided_button` is deleted, with no remaining caller. **`guided_field` is not** - see D6 |
| R11 | No overlay inlines a text-field composition locally. `guided_field` / `guided_field_focused` remain the field vocabulary until a replacement exists (D6) |
| R12 | No overlay inlines a disabled-reason composition locally. `guided_button` call sites stay put until Stage 6 builds the shared form (D7) |
| R10 | `knotra-vcs` is not modified |

## Verification

Per stage, all five gates, with gate five in the form CI runs:

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <stage-base>..HEAD
```

**Bare `git diff --check` is not the gate.** With everything committed it compares
the working tree to the index and verifies nothing - `129` A4.

Baseline at `fe09ff9`: **255 tests**. The count must not change: this RFC adds no
behaviour, and R4 forbids editing tests. A changed count is a signal to stop.

Stage 1 additionally reports byte-identity per moved overlay. Later stages cannot -
that is the point of D2 - and are verified by the unchanged test suite plus R5's
per-overlay close-route check.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| A migrated overlay drops its view-level `close_msg` gating | A close affordance is clickable during a non-cancellable phase | D3; R5's per-overlay check. Downgraded from the original framing: Escape is guarded in `app/focus_ops.rs`, outside this RFC's reach - see D3's correction |
| A lease is released on a close route the migration introduced | Stuck or double-acquired operation lease | 19 `release_if_matches` sites; R4's zero-test-change rule catches behavioural drift |
| Migration and split entangled, so nothing is verifiable | A mistake hides among justified changes | D2 separates them; Stage 1 is provably inert |
| Scope creep via D5 | An overlay RFC quietly becomes a workspace-management RFC | D5 is an explicit owner decision, not an assumption |
| `bulk_modals.rs` at 1,337 ELOC is simply large | Stages run long | One overlay per stage; five stages after the split |

## Alternatives considered

**Migrate in place without splitting.** Leaves the largest module in the crate at
1,337 ELOC and makes every migration diff land in one file, so stages cannot be
reviewed independently. Rejected.

**Split in a separate RFC first.** Cleaner in principle, and rejected as ceremony:
the split has no value on its own, and RFC-041 has just demonstrated that a
staged pure-move is reviewable inside a single RFC.

**Migrate the state machines too, while the overlays are open.** Tempting - the
phase enums have accumulated - and firmly rejected. These are the application's
riskiest operations, the invariants are spread across three prior RFCs, and the
zero-test-change proof only works if behaviour genuinely does not change.

**Leave `guided_button` alone.** Defensible if the owner prefers a narrower RFC.
The cost is explicit: the helpers become undeletable and the roadmap item stays open
with no owner. D5.

## Stages

| Stage | Content |
|---|---|
| 1 | Split `bulk_modals.rs` into `view/overlays/` - pure move, byte-identity verified |
| 2 | Conflict resolution onto the sheet primitive (D4) - smallest, and the only sheet |
| 3 | Changelog - has the request-id guard, self-contained |
| 4 | Freezer, then context switch - the two with non-cancellable phases |
| 5 | Smart Pull - largest, most phases |
| 6 | *(if D5 accepted)* the three orphan files, then delete `guided_button` and `guided_field` |

Smallest and least dangerous first, matching RFC-041 D6, which worked.
