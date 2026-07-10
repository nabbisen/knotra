# RFC-0018 — Migrate onto published `endringer` and `snora`

| Field          | Value                                                                          |
|----------------|--------------------------------------------------------------------------------|
| Status         | Implemented (v0.15.0)                                                          |
| Priority       | High — unblocks consuming the official crates; current in-tree copies are a fork |
| Effort         | Medium — re-layering + read-path adaptation; app logic largely unchanged       |
| Target version | v0.15                                                                          |
| Related        | RFC-0003 (jj conflict detection), RFC-0004 (ahead/behind), development-instructions §2 |

## Summary

The handoff (`development-instructions §2`) says to replace the path
dependencies on the in-tree `endringer`/`snora` with the published
`endringer 0.19.2` / `snora 0.8.0` and that **"the application code does
not change."** That premise does not hold. The published crates share a
name and author with the in-tree copies but occupy a **different layer**
and expose a **different public API**. A literal swap fails to compile:
every `endringer::` and `snora::` import in `knotra-app` resolves to a
symbol the published crates do not provide.

This RFC reframes the work as a **re-layering**, not a dependency swap:
rename the two in-tree facade crates to knotra-owned names, re-point their
internals onto the published low-level crates, and keep `knotra-app`'s
imports stable behind those renamed facades. The published crates become
the foundation they were always meant to be; the knotra-specific facade
and product model stay in-tree where they belong.

## What the published crates actually are

Investigation against crates.io (download + `lib.rs` inspection):

**`endringer` family (0.19.2)** — a low-level, read-first VCS
**introspection** library (the same role as the in-tree
`endringer-backend-*` crates), split as:

- `endringer-core` — read model types (`StatusDigest`, `BranchInfo`,
  `CommitInfo`, `TagInfo`, `StashEntry`, `WorktreeStatus`, `DiffSummary`,
  `CommitId`, …) + the `VcsBackend` trait.
- `endringer-git`, `endringer-jj` — gix-based backends.
- `endringer` — umbrella: re-exports core types + sync `repository()` /
  `jj_repository()` constructors.
- `endringer-async` — `AsyncRepository`: a **single-repo** `spawn_blocking`
  async wrapper. Reads, plus the only writes it has: `create_tag`,
  `create_annotated_tag`, `delete_tag`.

**`snora` family (0.8.0)** — a general-purpose **iced rendering engine +
widget kit**, split as:

- `snora-core` — vocabulary (`AppLayout`, `Toast`, `Dialog`, `Sheet`,
  `Menu`, `TabBar`, `SideBar`, `Icon`, …); zero iced dependency.
- `snora-widgets` — prefab `Element` builders (`app_header`, `app_footer`,
  `app_side_bar`, `app_tab_bar`, `render_menu`, `icon_element`, …).
- `snora` — umbrella: the `render()` engine + toast lifecycle + the
  re-exports above.

Neither published crate is what `knotra-app` consumes today.

## Gap against what `knotra-app` uses

### endringer

`knotra-app` consumes a **high-level async facade + product model** that
the published crate does not contain:

| knotra-app needs (in-tree `endringer`) | In published 0.19.2? |
|----------------------------------------|----------------------|
| `VcsAdapter` async facade (~22 methods) | No — only per-repo `AsyncRepository` |
| Writes: `fetch`, `smart_pull`, `switch_context`, `mark_resolved`, `abort_merge`, `list_conflicted_files` | No — only tag create/delete |
| Writes: `create_tag`, `create_tag_with_message`, `delete_tag` | **Yes** (`AsyncRepository`) |
| Reads: `list_tags`, `log_since`, `stash_entries`, `worktree_status` | **Yes** (different shapes) |
| Multi-repo `read_workspace_status` (bounded concurrency) | No — single-repo only |
| Domain model: `Project`, `Workspace`, `ProjectStatus`, `WorkspaceStatus`, operations log, `Freeze*`, `ChangelogDraft`, topology/`DependencyGraph` | No — knotra-specific orchestration |
| `FsPoller`, `FsChangeEvent` | No |

The read model also differs: published `StatusDigest` carries branch +
last-commit only. `ProjectStatus` additionally needs **ahead/behind**
(derivable from `merge_base` + commit walk, or computed knotra-side per
RFC-0004) and **conflict state** (no published method; stays knotra-side —
jj-via-CLI per RFC-0003, git via index/merge state). Dirty/untracked
counts come from `worktree_status()`; upstream identity from `remote_url()`.

### snora

All five symbols `knotra-app` imports are absent from published `snora`:
`KnotraTheme`, `theme::StatusColor`, `i18n::Catalog`, `i18n::Locale`,
`widget::CARD_GAP` (plus `nav_menu::{nav_bar, NavItem, NAV_BAR_HEIGHT}`
from RFC-0013). These are **knotra-specific application concerns** (a
product theme palette, the knotra UI string catalog, attention-tier color
semantics) — they do not belong in a general-purpose GUI crate. Published
`snora` instead offers the rendering engine and a widget kit knotra can
build *on top of* (e.g. its `TabBar`/`render_menu` supersede the hand-rolled
`nav_menu`).

## Decision

Adopt a **re-layering**. The naming collision (you cannot have an in-tree
crate named `endringer` that itself depends on crates.io `endringer`)
forces the facades to be renamed anyway; we use that to draw the layer
boundary cleanly.

Target architecture:

```
knotra-app
   │  imports only the knotra facades (logic unchanged; import paths renamed)
   ├─────────────────────────────┬───────────────────────────────
   ▼                             ▼
knotra-vcs  (was in-tree         knotra-ui  (was in-tree `snora`)
  `endringer`)                     · KnotraTheme, StatusColor, i18n catalog
  · VcsAdapter + domain model      · layout tokens + (unused) nav primitives
  · reads mapped onto              · renders via iced directly (no snora dep;
    endringer-async                  snora layout-framework adoption = future RFC)
  · writes via VCS CLI (C-1)
   ▼
endringer / -async / -core / -git / -jj  (crates.io)
```

Concretely:

1. **Delete** the in-tree backend crates `endringer-backend-core`,
   `endringer-backend-git`, `endringer-backend-jj`,
   `endringer-backend-async`. Published `endringer-core/-git/-jj` (0.19.2,
   ahead of the in-tree 0.14) replace them.
2. **Rename** in-tree `endringer` → **`knotra-vcs`**. Re-point its read
   path onto `endringer-async::AsyncRepository` + `endringer-core` types
   (map `StatusDigest` + `worktree_status` + `remote_url` + computed
   ahead/behind → `ProjectStatus`). Keep its write path on the VCS CLI
   (matches constraint C-1) except tag create/delete, which may now use
   `AsyncRepository`. Keep the knotra domain model, `FsPoller`, freeze /
   changelog / topology / conflict orchestration in this crate.
3. **Rename** in-tree `snora` → **`knotra-ui`** (resolving the name
   collision with the published `snora` umbrella). Keep `KnotraTheme`,
   `StatusColor`, the i18n string catalog, and the layout tokens — all
   knotra-specific, with no equivalent in published `snora`.
   Implementation finding: `knotra-app` consumes **none** of published
   `snora`'s surface today, and its only would-be overlap — the
   hand-rolled `nav_menu` — is currently **unused**. So this migration
   adds **no** dependency on published `snora`; a hollow dependency would
   be complexity that earns nothing. Adopting snora's layout framework
   (prefab header/sidebar/tabs/menus, the `render()`/`AppLayout` engine,
   and ABDD RTL mirroring) is a separate UI re-architecture — see the
   resolved snora note under Open Questions — and belongs in its own
   future RFC.
4. **`knotra-app`** changes: `endringer = { path = … }` → `knotra-vcs`,
   `snora = { path = … }` → `knotra-ui`, and a mechanical import rename
   `use endringer::` → `use knotra_vcs::`, `use snora::` → `use knotra_ui::`.
   The `VcsAdapter` and foundation **public surfaces are preserved**, so app
   message/state/view logic is unchanged. The hard boundary
   (`knotra-app` never imports `gix`/`jj` directly) is maintained — and
   strengthened: `knotra-vcs` is now the only crate touching the published
   VCS layer.

This keeps knotra-specific product logic in-tree (per "balance" — no
forcing app concerns into general crates), uses the official crates for
the general foundation, and preserves the architectural boundary and the
clean-compile baseline.

## Alternatives considered

- **Literal swap (the handoff's text).** Rejected: does not compile;
  the published API is a different layer.
- **Push knotra's facade + theme + i18n upstream into `endringer`/`snora`.**
  Rejected as the primary path: most of it (workspace model, operation log,
  freeze validation, the knotra string catalog, attention-tier colors) is
  product-specific and would pollute general-purpose crates. A *narrow*
  subset is a reasonable upstream request (see Questions) but is not a
  blocker — knotra can ship the re-layering against today's published API.
- **Keep the in-tree fork indefinitely.** Rejected: it is a divergent
  duplicate of the author's now-more-advanced published crates (0.19.2 /
  0.8.0 vs in-tree 0.14 / 0.9), and was the original mistake.

## Questions / optional feature requests for the authors

`endringer`, `endringer-async`, `snora` and knotra share an author
(nabbisen), so these are confirmations more than negotiations:

1. **endringer-async writes.** Is a write API (fetch / pull / branch- or
   bookmark-switch / merge-abort / mark-resolved) planned, or is the
   read-first scope deliberate? This decides whether `knotra-vcs`'s write
   path stays CLI-based (the current assumption) or can delegate upstream.
2. **Ahead/behind.** Will `endringer-async` expose ahead/behind relative to
   upstream, or should knotra compute it from `merge_base` + commit walk?
3. **Conflict status.** Any planned `conflict_status()` (esp. jj)? If not,
   knotra keeps jj-via-CLI detection (RFC-0003) and the "unknown, never
   clean" rule (C-2 / R-5).
4. **Multi-repo helper.** Confirm multi-repo orchestration with bounded
   concurrency is intended to be the consumer's job (knotra builds it over
   `AsyncRepository` + a semaphore). Assumed yes.
5. **snora theme/i18n. — RESOLVED (snora author).** The "theme- and
   i18n-agnostic" framing was only half right:
   - **Theming: delegated, not agnostic.** snora's prefab widgets pull
     their chrome from the active iced `Theme`; snora consumes iced's
     theme system rather than defining its own. knotra still owns its
     theme — `KnotraTheme`/`StatusColor` stay in `knotra-ui` — but if
     knotra adopts snora's widgets, that theme should be expressed as the
     iced `Theme` snora reads.
   - **i18n: catalog-agnostic, but snora owns layout direction.** Message
     catalogs, translation, and locale number/date formatting are the
     app's job (so the knotra `Catalog`/`Locale` correctly stay in
     `knotra-ui`), **but** layout direction is snora's first-class domain:
     logical `Edge::Start`/`End`, `LayoutDirection::Ltr`/`Rtl`, and
     automatic mirroring (ABDD). If knotra needs RTL, that is snora's
     ABDD, not a hand-rolled knotra concern.
   This **confirms** the migration's split (knotra-ui owns the theme
   palette + the message catalog) and refines it: a future
   snora-layout-adoption RFC should map `KnotraTheme` onto an iced `Theme`
   and route RTL through snora's ABDD rather than reinventing either.
   (snora is now at **v0.10.0**; "not planned" for the excluded scope
   means no current intent, revisitable given a concrete use case.)

None of these block the re-layering; answers refine where the
`knotra-vcs` seam sits.

## Sequencing

1. Land this RFC (move to `done/` on acceptance).
2. Spike: build `knotra-vcs::read_project_status` over `AsyncRepository`
   for one Git repo and one jj repo; confirm the `StatusDigest` →
   `ProjectStatus` mapping and ahead/behind + conflict derivation.
3. Re-layer `knotra-vcs` (reads adapted, writes CLI, model retained);
   delete in-tree backends.
4. Rename `snora` → `knotra-ui` (**rename only** — no dependency on
   published `snora` added, since `knotra-app` consumes none of its
   surface; see step 3 and the resolved snora Open Question).
5. Rename `knotra-app` deps + imports; restore the 0/0 clean-compile +
   green-test baseline.
6. Update `development-instructions §2` to describe the real layering
   (the "no app changes" line is incorrect and must be corrected).

## Verification

Baseline to restore after each step: `cargo check` + `cargo clippy` clean
(0 errors / 0 warnings) and `cargo test` green. The pre-migration in-tree
workspace already meets this (confirmed: clean check, 0/0).
