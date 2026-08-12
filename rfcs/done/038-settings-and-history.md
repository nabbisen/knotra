# RFC-038 - Settings and History

| Field | Value |
|---|---|
| Status | Implemented (main: 4d44366) - amended 2026-08-12 (A1, project owner); see Amendments |
| Priority | High - the last two unmigrated screens, and the RFC that must build the field primitive RFC-034 never did |
| Effort | Medium-to-large - two screens, a new `knotra-ui` primitive, and a localisation gap wider than previously recorded |
| Target | Production Readiness Reset - UI/UX foundation track |
| Related files | `crates/knotra-app/src/view/settings.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-ui/src/widget/field.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/033-...md` (**H4**, mislabelled - see Background), `rfcs/done/034-...md` (D6's unbuilt half), `rfcs/done/037-...md` (D6/D7 - why the field primitive is still missing) |

## Summary

`view/settings.rs` (177 ELOC) and `view/history.rs` (289 ELOC) are the last two
screens on pre-RFC-034 primitives. This RFC migrates both, and in doing so builds
the **validated field primitive** that RFC-034 R7 promised and never delivered -
because Settings is the first thing in the codebase that actually needs one.

It also establishes the **record-list pattern** RFC-039 depends on.

## Background

### Read RFC-033 H4, and know that it is mislabelled

RFC-033's child sections are off by one, because RFC-036 was taken by keyboard
navigation. **H4 is titled "RFC-037 - settings and history" and is this RFC.**
RFC-037 consumed H3. Same trap RFC-037's Background documented; repeated here
because an implementer reading RFC-033 directly will hit it again.

### One of H4's four bullets is already done

H4 asks to localise "the hard-coded back label at `history.rs:32-40`". **That label
no longer exists.** `history.rs:32` reads:

```rust
// RFC-034 R13: per-screen back navigation removed — Dashboard/History are
// reached through the persistent shell now, not a screen-owned button.
row![text(state.t("history.title")).size(20)]
```

RFC-034 removed it and the header is already localised. Verified at `56f85a3`.
Recorded so it is not carried forward as work.

### H4 understates the localisation gap, and the part it missed is user-visible

H4 says the Markdown export "still emits raw reason codes **while the visible path
localizes them**." The second half is wrong.

`summarise_status` (`history.rs:256`) returns hardcoded English `&'static str` -
`"↩ Rolled back"`, `"✗ Rollback failed"`, `"Success"`, `"Partial"`, `"Skipped"`,
`"Failed"` - and it is called at **`history.rs:112` and `:143`**, both inside
`view_log_entry`/`view_log_detail`. Those are the visible path.

So a Japanese user sees English status labels on every history row today. That is a
larger and more visible defect than the Markdown gap H4 did record, and it was
recorded as already-solved.

Verified hardcoded English at `56f85a3`:

| Location | Strings |
|---|---|
| `settings.rs:78-85` | `"Active: English"`, `"Active: 日本語"`, `"Active: Dark"`, `"Active: Light"` |
| `settings.rs:179-206` | FS-watch label, its explanatory sentence, interval label, three topology-phase strings |
| `history.rs:256-266` | six `summarise_status` labels - **visible** |
| `history.rs:307-370` | `log_to_markdown`'s labels - export only |

### The numeric settings do not validate; they silently coerce

`settings.rs` uses six raw `text_input`s and no field helper. Their handlers:

```rust
let n = s.parse::<u32>().unwrap_or(0);                       // :95
s.parse::<usize>().ok().filter(|&n| n > 0)                   // :102, :112
let n = s.parse::<u32>().unwrap_or(2);                       // :191
```

Typing `abc` into the refresh interval sets it to **0**. Typing it into max
concurrent reads is silently dropped. Nothing tells the user either happened. This
is what H4's "validated numeric fields with units and persistent errors" is for.

### Why the field primitive lands here

RFC-034 R7 promised new controls "added alongside `guided_button` and
`guided_field`". RFC-037 D6 found it happened for buttons and **never for fields** -
`field.rs` still holds only `guided_field` and `guided_field_focused`. RFC-037
therefore could not delete `guided_field`, and `ROADMAP.md` now records that closing
that line "needs a field primitive, which nothing schedules yet."

**This RFC schedules it**, not as scope creep but because Settings is the first
consumer that needs more than `guided_field` offers: validation, units, and an error
that persists rather than a silent coercion.

## Motivation

1. **A user in Japanese sees English on every History row.** The visible-path gap
   above is a shipped defect, not a tidy-up.
2. **Settings silently discards input.** A typo becomes `0` with no feedback.
3. **RFC-039 is blocked on the record-list pattern**, which H4 assigns here.
4. **The `guided_field` line cannot close without this.** RFC-037 established that;
   nothing else is scheduled to fix it.

## Non-goals

- **No `AppConfig` schema change.** No new settings, no removed settings.
- **No `knotra-vcs` changes** (RFC-033 H5).
- **No `app/` or `state/` changes** except where a validated field genuinely needs a
  new message variant - and if it does, say so before building it.
- **No `tests.rs` edits.** Untouched across RFC-040, RFC-035, RFC-041, RFC-037 and
  Handoffs 033-046.
- **Not deleting `guided_field`.** This RFC builds its successor; migrating the eight
  existing call sites is a separate exercise once the successor has proven itself
  here.

## Decision

### D1. Build the validated field primitive in `knotra-ui`, and prove it in Settings

A field that carries a label, a value, an optional unit, and a **persistent
validation error** - one that stays visible until the input becomes valid, rather
than a coercion the user never sees.

Scope it to what Settings needs, not to every field knotra might ever want.
`guided_field` stays; this is added alongside it, exactly as R7 intended for buttons
and got right there.

**Do not migrate `guided_field`'s eight existing call sites in this RFC.** Proving a
new primitive on the consumer that motivated it is one thing; sweeping seven other
call sites on the strength of one consumer is another, and RFC-037 D5 shows how that
widens an RFC.

### D2. Localise the visible path first, the export second

`summarise_status` is visible and wrong today. `log_to_markdown` is an export path
and has been wrong since before RFC-033 recorded it.

Fix the visible one first and independently, so it can ship even if the export work
runs long. `log_to_markdown` needs a locale threaded in - it is
`pub(crate) fn log_to_markdown(log: &OperationLog) -> String` with no `AppState`,
which is precisely why it never localised.

Both catalogs, and the jargon guard at `i18n.rs:1564` stays green.

### D3. Extract the record-list pattern where RFC-039 can consume it

`view_log_entry` (`history.rs:108`) and `view_log_detail` (`:183`) are the shape:
a scrolling list of records, each collapsible to a detail view, with a status label
and a disclosure control.

RFC-039 renders per-project VCS history - a different record, the same shape.
Extract the pattern so RFC-039 consumes it rather than copying it. **Where it
lives is a decision for the implementer to propose**: `knotra-ui` if it is
genuinely generic, `view/` if it is knotra-specific. Say which and why.

### D4. Settings becomes a bounded form, not a full-width column

Per H4: two-column grid at standard and wide, stacked at compact. `view_body`
(`settings.rs:33-217`) is one 185-line function; it will not survive the migration
intact and does not need to.

### D5. Stages, smallest and most visible first

| Stage | Content | Why here |
|---|---|---|
| 1 | Localise `summarise_status` and the `settings.rs` strings | Shipped defect, no new primitive needed, independently releasable |
| 2 | The validated field primitive in `knotra-ui`, with tests | Must exist before Settings can use it |
| 3 | Settings migration onto the primitive and the form grid | The primitive's first consumer |
| 4 | History migration and the record-list extraction | The largest, and RFC-039's dependency |
| 5 | **Re-scoped 2026-08-12 (A1).** Make the history export consistently English | Its original subject was deleted by RFC-043; the surviving export path is inconsistent and unguarded |

Stage 1 ships a user-visible fix before any structural work, which is the ordering
0.27.0's two defect fixes argue for.

## Requirements

| # | Requirement |
|---|---|
| R1 | **Amended 2026-08-12 — see Amendment A1.** No user-facing English string remains in the **visible path** of `settings.rs` or `history.rs`. Export text is English by design and is out of scope |
| R2 | Both catalogs carry every new key; `i18n.rs:1564`'s jargon guard stays green |
| R3 | The new field primitive lives in `knotra-ui`, has its own tests, and does not modify `guided_field` |
| R4 | A numeric setting given invalid input shows a persistent error and does **not** silently coerce |
| R5 | `AppConfig`'s schema is unchanged - no field added, removed, or retyped |
| R6 | The record-list pattern is extracted in a form RFC-039 can consume, with its location justified |
| R7 | **Withdrawn 2026-08-12 — see Amendment A1.** `log_to_markdown` was deleted as dead code by RFC-043 R6; this requirement has no subject. Replaced by R7a |
| R7a | The history export closure makes **no catalog lookup**. Its labels are source literals, so the exported artifact is a pure function of the operation record |
| R8 | `tests.rs` is not edited - zero lines |
| R9 | Every touched file stays under 500 ELOC |
| R10 | `knotra-vcs` is not modified |

## Amendments

### A1. Export text is English by design (2026-08-12, project owner)

**What forced this.** R7's subject, `log_to_markdown`, no longer exists — RFC-043 R6
deleted it as dead code, and `view/history.rs:188` records the removal. Stage 5 was
scoped entirely around a function that is gone, so the stage could not be executed as
written.

**What checking it turned up.** R1 required no English string "visible path or export",
but the surviving export — the copy-log closure in `view/history.rs` — is **already
mixed**: it localises the status label (`state.t(status.label_key)`) and the skip reason
(`state.t(reason.i18n_key())`) while emitting `ok`/`FAILED`/`SKIPPED` and the operation
`kind` as raw English. A Japanese user's copied log today reads `✓ 成功` on one line and
`[ok]` on the next. There was no exemption in force to preserve; there was an
inconsistency nobody had looked at, because the stage that would have collided with it
was the one whose subject got deleted.

**The decision: the export is English, and the split is deliberate.** The visible path
and the export are the same information in two media with two audiences. On-screen text
is read by the user, in the app, in their locale — localised, and that work is done. The
export **leaves the app**: it is pasted into issue trackers, chats, and search boxes,
where the reader is frequently not the user. R1 collapsing both media into one
requirement was the original error.

**Why English rather than full localisation**, on safety rather than taste:

- The export's two catalog lookups pass **variable** keys (`status.label_key`,
  `reason.i18n_key()`). `every_literal_t_call_names_an_existing_key` matches the literal
  text `.t("` and cannot see either one. They are the only two lookups in the export, and
  they are exactly the two no guard covers.
- `Catalog::t()` is `debug_assert!` then `unwrap_or(key)`. A **release** build with a
  missing key emits `history.status_success` as raw text into the user's clipboard — on
  the one surface where the user sends the artifact onward before reading it.
- Under localisation the same failure produces different text depending on the *sender's*
  locale, so two users' logs do not match one grep pattern.

Removing the lookups therefore removes an existing exposure rather than merely declining
to add one. The export stops depending on catalog completeness and on the sender's
locale, and depends only on the operation record — R7a.

**Scope of the change**: the labels only. No data is added or dropped either way;
commands, stderr, project ids and timestamps were never localised and are untouched.

**Behaviour change to note in the changelog**: a Japanese user's copied logs will show
`Success` where they showed `成功`.

## Verification

Per stage, all five gates, gate five in the range form (`129` A4):

```
cargo +1.91 fmt --all --check
cargo +1.91 clippy --workspace --all-targets -- -D warnings
cargo +1.91 test -p knotra
cargo +1.91 test -p knotra-ui
cargo +1.91 test -p knotra-vcs
git diff --check <stage-base>..HEAD
```

Baseline at `56f85a3`: **256 tests**.

**Unlike RFC-037, the test count is expected to rise.** R3's primitive and R4's
validation are new logic and must be tested; R2's catalog additions are covered by
the existing coverage guard. A stage that adds behaviour and no test is the thing to
question here - the inverse of RFC-037, where any change to the count was a signal
to stop.

`tests.rs` still stays at zero: new tests are co-located, as they have been
throughout.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| The field primitive is designed for Settings alone and does not fit the eight `guided_field` sites | A second parallel field system - exactly what R7 was meant to prevent | D1 scopes it deliberately and forbids sweeping the other sites; fit is assessed after it has one real consumer |
| Threading a locale into `log_to_markdown` pulls `AppState` into an export helper | Widening a signature across the crate | D2 stages it last and separately; pass the locale, not `AppState` |
| Validation changes what values reach `AppConfig` | A setting that used to coerce to 0 now refuses - visible behaviour change | Intended, and it is R4. Worth a CHANGELOG line, not a silent fix |
| The record-list extraction is shaped around History's record only | RFC-039 copies rather than consumes it | R6 requires the location and shape to be justified, and RFC-039 reviews it |
| Settings' 185-line `view_body` hides a behaviour when split | Silent regression | Stage 3 is its own stage; the suite passes unmodified apart from additions |

## Alternatives considered

**Localise without migrating.** Fixes the shipped defect and leaves both screens on
pre-RFC-034 primitives, with Settings still coercing input silently. Rejected as a
half-measure - though note Stage 1 does exactly this, deliberately, so the fix is not
held hostage to the rest.

**Migrate without building the field primitive** - use `guided_field` and keep the
silent coercion. Rejected: it forecloses R4, and it leaves RFC-034 R7's field half
unbuilt with no other RFC scheduled to build it.

**Build a general field system** covering every field in the application. Rejected as
the mistake RFC-034 arguably made in the other direction - specifying a primitive
before a consumer existed. One consumer first.

**Defer the record-list pattern to RFC-039.** Rejected: RFC-033 H4 assigns it here
precisely so RFC-039 consumes rather than invents, and the shape exists in History
today.
