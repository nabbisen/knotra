# RFC-067 - Stopping a fix attempt asks first

| Field | Value |
|---|---|
| Status | Implemented (main: 70e2447) |
| Priority | High - one click discards the user's conflict work, with no warning |
| Effort | Small - one confirmation step, one button's style and state, four catalog keys |
| Target | 0.32.0 (`.git-exclude/reviewed/226-release-plan-0.31.1-and-0.32.0.md`) |
| Related files | `crates/knotra-app/src/view/overlays/conflict.rs`, `crates/knotra-app/src/app/conflict_ops.rs`, `crates/knotra-app/src/state/conflict_ops.rs`, `crates/knotra-ui/src/i18n.rs` |
| Decided by | project owner, 2026-10-21 - accepted, with the Japanese of D3 |
| Related | RFC-066 (D6 gives Remove project the same kind of confirmation; D2's rule that the entry never starts a change) |

## Summary

In the Resolve panel, "Stop this fix attempt" runs `git merge --abort`. That puts the
project back as it was before the merge **and throws away every edit made to the conflicted
files**. Today it is one click on an ordinary-looking button, the first in the footer, with
no question asked. After this RFC, it looks like what it is, and it asks first.

## Problem

Measured at `316a3bb`.

1. **No confirmation.** `AbortMergeRequested` goes straight to `git merge --abort`
   (`knotra-vcs/src/vcs/git.rs:970`). A user who has resolved four of five files and clicks
   it loses those four.
2. **It does not look destructive.** It is drawn in the `secondary` style, the same as "Open
   in editor".
3. **It looks available when it is not.** While another operation runs (including the
   panel's own "Marking file done…"), it stays enabled. A click then does nothing, silently,
   because the operation lock is held (`app/conflict_ops.rs:183`).

Every other destructive action in knotra asks first: "Delete workspace" and "Remove project"
both do.

## Design

### D1. A confirmation, in the panel

Pressing "Stop this fix attempt" does not stop anything. It shows a confirmation **inside
the Resolve panel**, in place of the file list. That is a new `ConflictPhase::ConfirmStop`, so
the panel's existing close and Escape rules apply to it unchanged:

> **Stop this fix attempt?**
>
> Your edits to the files in this fix will be lost, and the project will go back to how it
> was before.
>
> [ Keep fixing ]  [ Stop and discard edits ]

- "Keep fixing" returns to the file list, exactly as it was. Escape, the scrim and the
  header close do the same as "Keep fixing" on this step: they leave the panel open on the
  file list, never stopping the attempt.
- "Stop and discard edits", in danger style, runs what the button runs today.
- Under RFC-066, "Keep fixing" is this step's entry.

Inside the panel, not a dialog over it, so there is never a window on top of a window.

### D2. The button shows what it does and when it can

- "Stop this fix attempt" is drawn in the **danger** style. It stays at the left of the
  footer, apart from "Close", which is where a destructive side action belongs; with D1 and
  RFC-066 D2, focus never lands on it first.
- While any operation holds the lock, it is **disabled** with the existing reason
  `plain.activity.busy` ("Wait for the current operation to finish." /
  「現在の操作が完了するまでお待ちください。」), like every other action in knotra.

### D3. Texts

Four new keys. **The Japanese needs the owner's approval.** The button label is unchanged.

| Key | English | Japanese |
|---|---|---|
| `plain.resolve.stop_confirm_title` | Stop this fix attempt? | この修正を中断しますか？ |
| `plain.resolve.stop_confirm_body` | Your edits to the files in this fix will be lost, and the project will go back to how it was before. | この修正でファイルに加えた編集は失われ、プロジェクトは修正前の状態に戻ります。 |
| `plain.resolve.stop_confirm_keep` | Keep fixing | 修正を続ける |
| `plain.resolve.stop_confirm_stop` | Stop and discard edits | 中断して編集を破棄 |

## Requirements

- R1. `AbortMergeRequested` is sent only by "Stop and discard edits"; "Stop this fix
  attempt" sends a new message that enters `ConfirmStop` (tested: the first click runs no
  command).
- R2. From `ConfirmStop`, "Keep fixing", Escape, the scrim and the header close all return
  to the file list; none runs a command (tested).
- R3. The button is danger-styled and is disabled with
  `plain.activity.busy` while the operation lock is held (tested).
- R4. The four keys, both languages, with the approved Japanese.
- R5. CHANGELOG `[Unreleased]`, under Fixed (text in the handoff).

## Out of scope

The other conflict actions; Jujutsu, which has no stop action; what `merge --abort` does.
