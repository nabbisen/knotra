# RFC-046 - Operation logs store codes, not rendered text

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - it corrupts persisted user data, and every release shipped before it lands adds more |
| Effort | Small - one write site, one catalog removal, one guard |
| Target | Production Readiness Reset - data correctness |
| Related files | `crates/knotra-app/src/app/sync.rs`, `crates/knotra-vcs/src/model/operation.rs`, `crates/knotra-app/src/view/history.rs`, `crates/knotra-ui/src/i18n.rs` |
| Related RFCs | `rfcs/done/038-settings-and-history.md` (**A1** - this is the gap A1's guarantee stops at), `rfcs/done/042-catalog-integrity.md` (the guard pattern D5 mirrors) |
| Found by | the dev team, out of scope, in Review Request 060 §3 |

## Summary

`ProjectOperationResult.skip_reason` is a `String` that holds a **stable code** from three
of its four writers and **rendered UI text in the user's language** from the fourth. The
field is serialised to disk and reloaded at startup, so the fourth writer bakes a locale
into persisted history permanently.

Fix the one writer, document the contract, and guard it.

## Problem

### One field, two incompatible meanings

`ProjectOperationResult.skip_reason: Option<String>` (`knotra-vcs/src/model/operation.rs:140`).
Four sites in non-test source write a non-`None` value:

| Site | Writes |
|---|---|
| `app/background/smart_pull.rs:158` | `exclusion.reason.code()` |
| `app/background/mod.rs:115` | `exclusion.reason.code()` |
| `knotra-vcs/src/model/operation.rs:481` | `reason.code()` |
| **`app/sync.rs:309`** | **`state.t("plain.fetch.skipped_unavailable")`** |

The first three store `retry:project_path_missing` and friends. The fourth stores
`"This project cannot be checked right now."` — or `"このプロジェクトは今は確認できません。"`,
depending on what language knotra happened to be in at the moment the operation ran.

### It is persisted, so the locale is permanent

`persistence.rs:91` writes each `OperationLog` to its own timestamped JSON file;
`persistence.rs:105` reads them back at startup (`app.rs:79`). The rendered sentence is
serialised verbatim and survives restarts.

Consequences:

- **A user who switches knotra to English still sees Japanese** in those entries. The
  render path is `RetryExclusionReason::from_code(reason).map(|r| state.t(r.i18n_key())).unwrap_or(reason)`
  (`view/history.rs:288`) — `from_code` returns `None` for a prose value, so the fallback
  emits the stored string unchanged, forever.
- **The export is affected too.** RFC-038 A1 made `export_text` a pure function of the
  operation record so the export could not depend on the sender's locale. It succeeds at
  the function boundary and is defeated at the write: the record itself carries locale.
- **Nothing notices.** `from_code`'s `unwrap_or` fallback is exactly what stops this from
  ever surfacing as an error.

### The vague message is also the wrong message

`app/sync.rs:300` collapses two genuinely different situations into one sentence:

```rust
if !project_map.contains_key(&id) || state.missing_projects.contains(&id) {
```

- `!project_map.contains_key(&id)` — the project is not in the active workspace.
- `state.missing_projects.contains(&id)` — populated at `app/background/status.rs:20-24`
  from `!VcsAdapter::repo_exists(p)`: the project's folder is gone.

`RetryExclusionReason` already has a variant for each — `NotInActiveWorkspace` and
`ProjectPathMissing`, with catalog text "Not in the active workspace" and "Project folder
is missing". The user is currently told "This project cannot be checked right now" when
knotra knows which of the two it is.

So fixing the storage defect also replaces a vague message with a precise one. **No new
enum variant is required.**

## Non-goals

- Retyping `skip_reason`. See D3.
- Migrating existing log files. See D4.
- `SmartPullPlanEntry.skip_reason` (`Option<SmartPullSkipReason>`) — already typed,
  in-memory, not persisted in that form. Correct as built.
- The `note` field in `ProjectConflictDetail` — that is RFC-045.

## Decision

### D1. `skip_reason` holds a stable code, and the field says so

Document the contract at the declaration (`knotra-vcs/src/model/operation.rs:140`): a
`RetryExclusionReason` code, never rendered text, because the value is persisted and
outlives the locale that produced it.

### D2. Split `app/sync.rs`'s collapsed condition into its two existing codes

`!project_map.contains_key(&id)` → `NotInActiveWorkspace::code()`.
`state.missing_projects.contains(&id)` → `ProjectPathMissing::code()`.

Order matters where both hold: a project absent from the active workspace is not in
`project_map` at all, so test workspace membership first and folder existence second.

`plain.fetch.skipped_unavailable` then has no referent — it is used at exactly one site
today — and comes out of both catalogs.

### D3. The type stays `Option<String>`

`Option<RetryExclusionReason>` would be better typing and is the wrong call here.

The field is serialised. Retyping changes the on-disk JSON shape, and `load_recent_logs`
discards any record that fails to parse (`serde_json::from_str::<OperationLog>(&text).ok()?`,
`persistence.rs:120`) — **silently**. A retype would delete users' existing history on
upgrade, with no error and no way back.

A documented contract plus D5's guard buys the same protection at the write, which is
where the defect actually is.

### D4. No migration pass; the fallback becomes an intentional property

Existing log files keep their prose. That is acceptable and should be stated rather than
fixed:

- `load_recent_logs` reads only the most recent `max_log_entries`, so pre-fix entries
  **age out of view** as the user performs new operations. Nothing accumulates in the UI.
- Reverse-mapping rendered prose back to codes would mean shipping a table of every past
  translation of one string in every locale — more fragile than what it repairs.
- `from_code(...).unwrap_or(reason)` already renders unknown values verbatim. Re-cast that
  fallback in its doc comment as **deliberate forward and backward compatibility** — an
  older knotra reading a newer log's code, or a newer one reading a pre-fix record — not
  as the accident that hid this.

### D5. A guard, mirroring the one RFC-042 already ships

`i18n.rs:1870`'s `status_bar_and_settings_save_msg_always_route_through_t` scans source
text to assert certain assignments **do** route through `t()`. This is its mirror: no
`skip_reason` assignment in non-test source may route through `t()`.

Same file-scanning helpers, same `files.len() > 50` sanity assertion, opposite polarity.

## Requirements

| # | Requirement |
|---|---|
| R1 | No `skip_reason` write in non-test source produces localised text |
| R2 | `app/sync.rs`'s two conditions produce two **distinct** codes, workspace membership tested first |
| R3 | `plain.fetch.skipped_unavailable` is removed from **both** catalogs, after confirming it has no other referent |
| R4 | The contract is documented at the field declaration (D1) and the fallback's doc comment states its compatibility purpose (D4) |
| R5 | D5's guard exists **and has been seen to fail on a planted violation** before it is trusted |
| R6 | `skip_reason`'s type and the on-disk JSON shape are unchanged |
| R7 | A log file containing pre-fix prose still loads, renders, and exports verbatim - no panic, no dropped record |
| R8 | `tests.rs:2021`'s fixture changes from prose to a code. That test asserts round-trip persistence and currently documents prose as an expected value shape |
| R9 | `all_keys_are_localised_in_both_catalogs` and `every_literal_t_call_names_an_existing_key` stay green |

**`tests.rs` is editable under this RFC** (R8). RFC-038's zero-lines rule was RFC-038's,
and R8 names the single edit intended - the fixture value, not the assertions around it.

## Test Plan

Co-located, as throughout:

- Each of D2's two branches produces its own code — two tests, distinct fixtures.
- R7's compatibility case: an `OperationResult` whose `skip_reason` is prose survives a
  `save_operation_log` / `load_recent_logs` round trip and renders verbatim. This pins
  D4's decision so a later change cannot quietly start discarding old records.
- D5's guard, with its planted violation reported.

Expect a rise. `tests.rs` gains no test; its one fixture value changes.

## Security Considerations

No new attack surface. One integrity improvement: the operation log is the record a user
pastes into a bug report, and it stops varying by the sender's UI language, so two users
reporting the same failure produce comparable text.

`load_recent_logs`' silent `.ok()?` on parse failure is noted in D3 as the reason not to
retype, and is otherwise unchanged by this RFC. **It is worth its own look later** — a
corrupted or truncated history file currently disappears from the UI with no indication,
which is the same class of defect as the one RFC-044 D3 fixed for topology data.

## Migration / rollout

Two user-visible changes, both wanted, both worth a changelog line:

- Projects skipped during a fetch now say **"Not in the active workspace"** or **"Project
  folder is missing"** instead of "This project cannot be checked right now."
- Entries logged before this lands keep their original wording, in the language they were
  written in, until they age out of the most-recent-`max_log_entries` window.
