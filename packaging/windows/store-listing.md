# Microsoft Store listing

A field-by-field guide to knotra's Partner Center listing, with draft copy.

Limits and rules are quoted from Microsoft's documentation as read on 2026-10-18:
[Add and edit Store listing info](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/add-and-edit-store-listing-info)
and
[App screenshots, images, and trailers](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images).
Check them again before each submission; they change.

**Every claim in the copy below was checked against the code.** Before adding a
feature to it, check the same way. A listing that promises more than the app does
fails users, and may fail certification.

## Before you submit

- **The package.** Download `release-Windows-msix` from the release workflow run
  for the version you are submitting. It is not attached to the GitHub release,
  because an unsigned package cannot be installed; Partner Center signs it.
- **The identity must match.** `AppxManifest.xml`'s `Identity Name` and
  `Publisher` must be exactly what Partner Center reserved. A mismatch fails at
  submission.
- **Each submission needs a higher version** than the last one. The pipeline
  derives it from the crate version, so a new knotra release is a new version.
- **`AppxManifest.xml` carries the real version**, the crate version plus `.0`
  (0.30.0 is 0.30.0.0). Release prep bumps it together with `Cargo.toml`; a test
  fails if they differ, and the release job checks it again before packing.
- **Privacy policy URL**:
  `https://github.com/nabbisen/knotra/blob/main/PRIVACY.md`

## Required

The minimum to complete the Store listings step is **a description and one
screenshot**.

### Description — required, plain text, up to 10,000 characters

No HTML, code or URLs; links go in their own fields.

> knotra is a desktop dashboard for people who work across many Git and Jujutsu
> (jj) repositories at once.
>
> Instead of opening a terminal in each project to check its state, knotra shows
> every repository in your workspace on one screen and groups them by what needs
> your attention: projects that need help, unsaved work, available updates, local
> commits not yet shared, and decisions waiting for you.
>
> From there you can act on many repositories together, with the care that bulk
> changes need. Smart Pull only fast-forwards, so it never creates a merge commit,
> and it never merges over uncommitted work: those repositories are fetched but
> not merged, unless you choose to set the work aside and restore it afterwards.
> Freezer tags a release across several repositories
> at once, checks them first, and rolls back if any step fails. History keeps a
> per-project record of every operation, so you can see what happened and why. A
> conflict panel guides you through resolving merge conflicts, and knotra can
> draft release notes from your recent commits.
>
> knotra is local-first. It has no accounts, no telemetry and no advertising, and
> it never sends your data anywhere. It works through the Git and Jujutsu already
> installed on your computer, so your existing settings and credentials keep
> working as they are.
>
> knotra is available in English and Japanese, offers light and dark themes, and
> has keyboard shortcuts for common actions. It is free and open source software,
> licensed under the Apache License 2.0.

### Screenshots — at least 1, up to 10 for Desktop

- **PNG only**, landscape or portrait, **50 MB** maximum each.
- **Desktop: 1366 × 768 pixels or larger**; 3840 × 2160 is supported.
- Microsoft suggests **at least four**, and 5–8 in its FAQ.
- Keep key content in the **top two-thirds**; text overlays may cover the bottom
  third.
- **No added logos, icons or marketing text** on the images.
- Optional caption per screenshot, **200 characters** or fewer.
- Each listing language needs its own upload, even if the images are the same.

Captured per `.git-exclude/tasks/developer/087-screenshots.md`. Suggested
captions:

1. Every repository in your workspace, grouped by what needs your attention.
2. The same dashboard in the dark theme.
3. Smart Pull fast-forwards only, and never merges over uncommitted work.
4. Freezer checks every repository before tagging, and rolls back on failure.
5. A per-project history of every operation.

## Recommended

### Short description — up to 1,000 characters; keep it under 270

Shown at the top of the listing. If left empty, the first paragraph of the
description is used, so write something different to avoid repetition.

> See every Git and Jujutsu repository you work with on one dashboard, and
> update, tag and review them together — safely, and entirely on your own
> computer.

### Product features — up to 20, 200 characters each

Displayed as a bulleted list; do not add your own bullets.

- One dashboard for every repository in your workspace
- Repositories grouped by what needs your attention
- Organize repositories into named workspaces
- Smart Pull: fast-forward only, and never merges over uncommitted work
- Tag a release across many repositories, with checks first and rollback on failure
- A per-project history of every operation
- Guided merge conflict resolution
- Release note drafts from recent commits
- Works with Git and Jujutsu (jj)
- Local-first: no accounts, no telemetry, nothing sent anywhere
- English and Japanese
- Light and dark themes
- Keyboard shortcuts for common actions

### Additional system requirements — up to 11 items each, 200 characters each

**Required here, not optional in practice.** knotra runs the `git` and `jj`
programs installed on the computer. Without Git, a Store user installs knotra and
nothing works.

- Git must be installed and available on the PATH
- Jujutsu (jj) is needed only for Jujutsu repositories

### 1:1 App tile icon — 300 × 300 PNG

Strongly recommended. If absent, the Store uses the image from the package.

### What's new in this version — up to 1,500 characters

**Leave it blank for the first submission**, as Microsoft instructs. From the
second submission on, summarise that version's `CHANGELOG.md` section.

## Not applicable to knotra

Short title, sort title and voice title are for Xbox and voice experiences.
Trailers, 16:9 super hero art, poster art and box art are aimed at games or
featured placement; Microsoft says to avoid showing the app's UI in the hero art.

## Japanese listing — draft for review

Japanese is one of the two languages in the package, so it gets its own listing,
with its own screenshot uploads. **This draft needs a native reader's review**,
including that its terms match the ones the app itself uses.

### 説明

> knotra は、多数の Git / Jujutsu（jj）リポジトリを並行して扱う人のための
> デスクトップダッシュボードです。
>
> プロジェクトごとにターミナルを開いて状態を確かめる代わりに、ワークスペース内の
> すべてのリポジトリを一つの画面に表示し、対応が必要なものごとにまとめます。
> 手助けが必要なもの、未保存の作業、取り込める更新、まだ共有していないローカル
> コミット、判断を待っているもの、などです。
>
> そこから、複数のリポジトリをまとめて操作できます。一括操作に必要な慎重さを
> 保ったままです。スマートプルは早送り（fast-forward）でのみ更新するため、
> マージコミットを作りません。未コミットの作業がある上にマージすることも
> なく、そうしたリポジトリは取得だけを行います。作業を一時退避して更新後に
> 戻すこともでき、それはあなたが選んだ場合に限られます。フリーザーは複数の
> リポジトリにまとめてリリース
> タグを付けます。事前に確認を行い、途中で失敗すれば元に戻します。履歴は
> プロジェクトごとにすべての操作を記録するので、何が起きたのか、なぜなのかを
> 確かめられます。競合パネルはマージ競合の解決を手順に沿って案内し、最近の
> コミットからリリースノートの下書きも作れます。
>
> knotra はローカルファーストです。アカウントもテレメトリーも広告もなく、
> あなたのデータをどこにも送信しません。コンピューターにすでにある Git と
> Jujutsu を通じて動くため、今の設定や認証情報をそのまま使えます。
>
> 英語と日本語に対応し、ライトテーマとダークテーマ、よく使う操作のキーボード
> ショートカットを備えています。Apache License 2.0 で提供される、無料の
> オープンソースソフトウェアです。

### 短い説明

> 作業中のすべての Git / Jujutsu リポジトリを一つのダッシュボードで確認し、
> まとめて安全に更新・タグ付け・振り返り。すべてお使いのコンピューター上で
> 完結します。

### 追加のシステム要件

- Git がインストールされ、PATH から利用できること
- Jujutsu（jj）は Jujutsu リポジトリを扱う場合のみ必要

## Other fields

| Field | Value |
|---|---|
| Copyright and trademark info | Copyright 2026 nabbisen |
| Website | `https://github.com/nabbisen/knotra` |
| Support contact | `https://github.com/nabbisen/knotra/issues` |
| Privacy policy URL | `https://github.com/nabbisen/knotra/blob/main/PRIVACY.md` |
| Additional license terms | Apache License 2.0 — `https://github.com/nabbisen/knotra/blob/main/LICENSE` |
