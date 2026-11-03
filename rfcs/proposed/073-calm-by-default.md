# RFC-073 - Calm by default

| Field | Value |
|---|---|
| Status | Proposed |
| Priority | High - older users meet about twenty controls, four meanings of "check", and developer settings before they meet their projects |
| Effort | Medium - five stages: fewer things shown, hands and eyes, calm Settings, one word per action, your data |
| Target | 0.34.0 (`.git-exclude/reviewed/257-older-users-assessment-and-0.34.0-direction.md` §6, `258`, `259`) |
| Related files | `crates/knotra-app/src/view/{shell,settings,detail_panel,history,command_palette,selection_bar}.rs`, `crates/knotra-app/src/view/dashboard/{mod,toolbar,row,section,empty}.rs`, `crates/knotra-app/src/view/overlays/{freezer,smart_pull}.rs`, `crates/knotra-app/src/{config,state/palette}.rs`, `crates/knotra-ui/src/{i18n,widget/chip,widget/status}.rs`, `docs/src/` |
| Decided by | - |
| Related | RFC-071 (type, colour, layout, zoom), RFC-070 (high contrast), RFC-072 (its "your data" half moves here, Review 259 §2), RFC-074 (0.35.0, the flow) |

## Summary

RFC-071 made knotra readable. This RFC makes it **calm**: it shows less at once, uses one word
for one action, and keeps developer settings out of the way. Nothing is taken away. What most
people rarely need moves one step further out of sight.

## Problem

Measured at `df04f4a`. Reviews 257, 258 and 260 have the full findings.

1. **Too much at once.**
   - About twenty controls come before the first project.
   - With **no projects at all**, the dashboard still shows six chips at "0", Group, Sort,
     Search and an unavailable Select (`stage-4/zoom/en-100.png`).
   - "Git" stands under every name, even when every project is Git.
   - The workspace menu shows an unlabelled number (`shell.rs:140-158`: the count of projects
     needing help), and looks like a settings menu.
   - "Needs help" and "Needs a choice" are two chips with the same red symbol and
     overlapping counts.
2. **Status is drawn three ways.**
   - Tone badges on the dashboard;
   - "!" / "✓" in the ready check;
   - "✓ Success" text on History and in results.
3. **Small targets and symbols.**
   - Chips are about 30 px tall, and the header links, group toggles and names about the
     height of their text.
   - The badge symbol is about 10 px.
4. **Settings shows developer controls to everyone**, as typed numbers: "Max concurrent
   reads", "Max operation log entries", "Change detection interval", experimental file
   monitoring. "Some changes take effect on next launch" does not say which. A typed value is
   applied, and saved, on every valid keystroke (Review 259 §1).
5. **One word, four actions:** "Check now", "Check for updates", "Check all projects", "Check
   only", "Check readiness"; and "Refresh", "Refresh dashboard", "Fetch".
   - **In Japanese**, "Refresh" is 「更新」, the same word as "Updates available" (「更新あり」).
   - "Work area" is both 「作業エリア」 and 「作業場所」.
   - "Unsaved work" is both 「作業中の変更」 and 「未保存の作業」.
   - The detail panel speaks Git: "Remote", "Ahead", "Dirty", "Untracked", "Fetch".
6. **knotra keeps the user's project paths and history on disk, and says nowhere what or
   where** (Review 258 §2).
7. **A dead entry:** the command palette's "Toggle theme" is always hidden and does nothing
   (`state/palette.rs:142`).

## Design

### Stage 1 - show less

**D1. No toolbar on an empty dashboard.** With no projects in the workspace, the chips, Group,
Sort, Search and Select are not drawn. "Welcome to knotra" and "Add project folder" stand
alone.

**D2. Group and Sort move into one "Arrange" menu**, beside Search. Opening it shows the two
existing selects, unchanged.

- Chips, Search and Select stay where they are.
- RFC-066's dashboard focus order gains "Arrange" in place of the two selects.

**D3. The project kind shows only when it tells something.** "Git" / "Jujutsu" appears under a
name only when the workspace holds both kinds.

**D4. Workspaces stay out of the way until there are two.**

- **With one workspace,** its name is plain text in the header, not a menu.
- **With two or more,** the menu appears as today, for switching.
- **Creating, renaming and removing workspaces** move to a new **"Workspaces"** section in
  Settings, built from the existing dialogs and texts. The palette's actions stay.
- **The unlabelled count goes** from the workspace name. The "Needs help" chip already says it.

**D5. One chip per meaning.** "Needs a choice" is a kind of "Needs help": every project needing
a choice also needs help. **Proposed: the "Needs a choice" chip goes.** Each row in "Needs
help" still says why ("A resolution choice is needed").

- Owner input 2 offers the alternative, which keeps both chips under clearer names.

**D6. The command palette keeps its label**, with a neutral icon in place of "⌘", a macOS
symbol. Owner input 3 asks whether it belongs in the header at all.

**D7. Remove the dead "Toggle theme"** entry and its catalog key.

### Stage 2 - hands and eyes

**D8. Everything you can press is at least 36 px tall** (`SMALL_BUTTON_HEIGHT`), and 44 px where
the row allows: chips, header links, group toggles, project names, the Arrange menu.

- Text keeps its size. The hit area grows.
- A guard pins it: every pressable widget built by a `knotra-ui` helper has a height of at
  least 36.

**D9. One status sign everywhere.** The ready check, Smart Pull's plan and results, Change work
area's results, and History's entries use RFC-071's `status_badge` in place of "!", "✓", "✗",
"-".

- **Outcomes map exhaustively:** success → `success`, check mark; failure → `danger`, alert;
  skipped → `info`, a minus sign.
- The words beside the badge stay.

**D10. A larger badge symbol:** at least 14 px inside the badge. The badge grows if needed,
never past the text's line height plus 4 px.

### Stage 3 - calm Settings

**D11. Settings shows what people use; the rest is under "Advanced", closed by default.**

| Shown | Under Advanced |
|---|---|
| Language, Theme, Zoom | Simultaneous reads (was "Max concurrent reads") |
| Refresh automatically (D12) | Operation log size |
| Editor, comparison tool | File-system monitoring and its interval |
| Workspaces (D4) | Your data (Stage 5) |

**D12. Choices, not typed numbers.** Each number setting becomes a select.

| Setting | Choices |
|---|---|
| Refresh automatically | Off · Every 30 seconds · Every minute · Every 5 minutes · Every 15 minutes |
| Simultaneous reads | 2 · 4 · 8 · 16 |
| Operation log size | 100 · 200 · 500 · 1000 |
| Change detection interval | 1 · 2 · 5 seconds |

- A saved value that is not a choice loads as the nearest one, like zoom.
- Saving is A7's: at once.
- Nothing can be typed half-way, so no unintended value is ever applied or saved.

**D13. Say which settings need a restart.** Each one that does carries the note "Takes effect
after you restart knotra." under it. The general hint goes.

- The handoff finds which settings these are by reading the code, and reports each one with
  its reason.

### Stage 4 - one word per action

**D14. Each action has one name, in both languages.** "Check" means only "check for updates".

| Action | English | Japanese |
|---|---|---|
| Re-read every project's status (local, quick) | **Refresh** | **再読み込み** |
| Ask the shared source for news, change nothing | **Check for updates** | **更新を確認** |
| Get the news and move forward safely | **Get latest** (already) | **最新を取得** |
| The release ready check | **Ready check** (as a name) | **準備確認** |

The keys this touches are in D16.

**D15. The detail panel speaks knotra's words, not Git's**, matching the dashboard's.

**D16. Texts (owner approval needed).** Changed values only; keys keep their names.

| Key | English, now → proposed | Japanese, now → proposed |
|---|---|---|
| `plain.check_now` | Check now → **Refresh** | 今すぐ確認 → **再読み込み** |
| `shortcut.action_check_now` | Check now → **Refresh** | 今すぐ確認 → **再読み込み** |
| `palette.action.refresh` | Refresh dashboard → **Refresh all projects** | ダッシュボードを更新 → **すべてのプロジェクトを再読み込み** |
| `detail.refresh` | Refresh (same) | 更新 → **再読み込み** |
| `plain.activity.kind_refresh` | Refresh (same) | 更新 → **再読み込み** |
| `status.refreshing` | Refreshing… (same) | 更新中… → **再読み込み中…** |
| `plain.status.checking` (shown while status is re-read: `shell.rs:203`, `dashboard/mod.rs:202`, `dashboard/empty.rs:16`) | Checking… → **Refreshing…** | 確認中… → **再読み込み中…** |
| `plain.activity.retry_refresh_again` | Use Refresh to check again. → **Use Refresh to try again.** | 更新を使ってもう一度確認してください。 → **再読み込みでもう一度お試しください。** |
| `palette.action.check_all` | Check all projects → **Check all projects for updates** | すべてのプロジェクトを確認 → **すべてのプロジェクトの更新を確認** |
| `detail.fetch` | Fetch → **Check for updates** | フェッチ → **更新を確認** |
| `plain.activity.kind_fetch` | Check for updates (same) | 取得 → **更新を確認** |
| `plain.activity.retry_failed_fetches` | Retry failed checks → **Retry failed update checks** | 失敗した取得を再試行 → **失敗した更新確認を再試行** |
| `plain.activity.retrying_fetch` | Retrying failed checks → **Retrying failed update checks** | 失敗した取得を再試行中 → **失敗した更新確認を再試行中** |
| `palette.disabled.no_fetchable_projects` | No projects can be checked right now. → **No projects can be checked for updates right now.** | 今は確認できるプロジェクトがありません。 → **今は更新を確認できるプロジェクトがありません。** |
| `plain.selection.none_fetchable` | None of the selected projects can be checked right now. → **None of the selected projects can be checked for updates right now.** | 選択したプロジェクトは今は確認できません。 → **選択したプロジェクトは今は更新を確認できません。** |
| `plain.get_latest.check_only`, `plain.get_latest.action_check` | Check only → **Only check for updates** | 確認のみ → **更新の確認のみ** |
| `plain.release.check_readiness` | Check readiness → **Run the ready check** | 準備確認 → **準備確認を実行** |
| `plain.release.checking` | Checking… → **Running the ready check…** | 確認中… → **準備確認中…** |
| `plain.activity.kind_smart_pull` | Get latest (same) | 最新版を取得 → **最新を取得** |
| `plain.activity.kind_context_switch` | Change work area (same) | 作業場所を変更 → **作業エリアを変更** |
| `filter.dirty` | Unsaved work (same) | 作業中の変更 → **未保存の作業** |
| `detail.label_vcs` | VCS: → **Kind:** | VCS: → **種類:** |
| `detail.label_remote` | Remote: → **Shared source:** | リモート: → **共有元:** |
| `detail.label_branch` | Branch: → **Work area:** | ブランチ: → **作業エリア:** |
| `detail.label_ahead` | Ahead: → **Local commits:** | 先行: → **ローカルのコミット:** |
| `detail.label_behind` | Behind: → **Updates available:** | 遅れ: → **更新あり:** |
| `detail.label_dirty` | Dirty: → **Unsaved files:** | 未コミット: → **未保存のファイル:** |
| `detail.label_untracked` | Untracked: → **New files:** | 未追跡: → **新しいファイル:** |
| `detail.label_conflict` | Conflict: → **Needs a choice:** | 競合: → **選択が必要:** |

**New keys:**

| Key | English | Japanese |
|---|---|---|
| `dashboard.arrange` (D2) | Arrange | 並べ方 |
| `settings.section.workspaces` (D4) | Workspaces | ワークスペース |
| `settings.section.advanced` (D11) | Advanced | 詳細設定 |
| `settings.refresh_auto_label` (D12) | Refresh automatically | 自動で再読み込み |
| `settings.choice.off` | Off | オフ |
| `settings.choice.every_30s` | Every 30 seconds | 30秒ごと |
| `settings.choice.every_1m` | Every minute | 1分ごと |
| `settings.choice.every_5m` | Every 5 minutes | 5分ごと |
| `settings.choice.every_15m` | Every 15 minutes | 15分ごと |
| `settings.max_concurrent_label` (changed) | Simultaneous reads | 同時に読み込む数 |
| `settings.max_logs_label` (changed) | Operation log size | 操作ログの保持数 |
| `settings.needs_restart` (D13) | Takes effect after you restart knotra. | knotra を再起動すると反映されます。 |
| `plain.selection.all_in_closed_groups` (W3) | Open a group to choose its projects. | グループを開いてプロジェクトを選んでください。 |

- **W3** is shown when projects exist but every one sits in a closed group, in place of "No
  projects match this view."
- **One reuse corrected:** Change work area's list-loading line (`overlays/context_switch.rs:166`) uses `plain.status.checking`, but it is loading, not refreshing. It switches to the existing `detail.loading` ("Loading…" / 「読み込み中…」, value checked by the handoff). No new text.
- **Removed keys:**
  - `filter.conflict`, with D5;
  - `palette.action.toggle_theme`, with D7;
  - `settings.refresh_interval_label` and `settings.restart_hint`, with D12 and D13.

### Stage 5 - your data

**D17. Measure first** (Review 259 §2). For each way of installing (tar.gz, `cargo install`, the
Microsoft Store package), record:

- where knotra's settings, workspaces and history actually live on disk;
- what an uninstall leaves behind.

The Store package needs the owner's Windows machine. The handoff gives a five-minute
checklist.

**D18. Then tell the user.**

- **A "Your data" page in the docs:** what knotra keeps (settings, the list of project
  folders, operation history), where, per platform and installer, and that it never stores
  anything inside a project.
- **"Your data" under Settings → Advanced:** the same facts for this installation, with the
  real folder of each.
- **"Open folder"** appears only where D17 showed it opens the right place.
- The texts for D18 come to the owner as an amendment, after D17. They cannot be written
  before the facts are known.

## Requirements

- R1. D1–D7: each covered by a view test where the codebase has one, and by the captures.
- R2. D8's guard; D9's exhaustive outcome mapping; D10's measured symbol size.
- R3. D11–D13:
  - nearest-choice loading, tested like zoom;
  - the restart list reported, with reasons.
- R4. D16's texts exact, in both catalogs; the unused-key test passes.
- R5. D17's measurements reported before D18 starts.
- R6. Each stage is its own review request, with its own CHANGELOG entries and before/after
  captures under the GUI rules.
- R7. The Store images are retaken in 0.34.0's release prep.

## Out of scope

- The flow changes (Review 257 F1–F3): RFC-074, 0.35.0.
- Removing knotra's data (RFC-072's other half): 0.35.0.
- F6.
- Issue #4.
- Any colour, size or theme rule from RFC-070 or RFC-071.

## Owner inputs

1. **Accept,** with D16's texts. Each line can be changed; tell me which.
2. **D5:**
   - remove "Needs a choice" (recommended);
   - or keep both chips, renamed so they read as different things.
3. **D6:** keep the labelled command palette in the header (recommended, as today), or move it
   to the Settings screen and the shortcut only.
4. **D17:** five minutes on your Windows machine, when stage 5 asks.
