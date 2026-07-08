# RFC-005 — Annotated Tag Support in the Freezer

| Field    | Value                                                             |
|----------|-------------------------------------------------------------------|
| Status      | Implemented                      |
| Priority | Medium — improves release workflow utility                        |
| Effort   | Small–Medium (VcsAdapter change + UI toggle + i18n)               |
| Related  | `crates/endringer/src/vcs/adapter.rs`, `view/freezer.rs`          |

## Summary

The Freezer currently creates lightweight Git tags only.
`endringer-backend-git::GitBackend` already implements `create_annotated_tag(name, message)`.
Expose it through `VcsAdapter` and add an optional message field to the Freezer
UI so users can create annotated tags at freeze time.

## External Design

### User-visible behaviour

The Freezer's freeze-name input gains a secondary optional text area:

```
Freeze point name:  [v1.2.3         ]

Tag message (optional):
[ Leave blank for a lightweight tag.    ]
[                                        ]
```

- When the message field is **empty**, behaviour is unchanged (lightweight tag).
- When the message field is **non-empty**, `create_annotated_tag(name, message)`
  is called instead.

For jj repositories, the message field is ignored (jj bookmarks have no
message concept).  The UI notes this with a small hint below the field.

### Rationale

Annotated tags carry a tagger name, email, date, and GPG-signable message.
Many release workflows require annotated tags (`git describe` uses them by
default).  The feature is additive and backward-compatible: the default path
(empty message → lightweight tag) is unchanged.

## Internal Design

### VcsAdapter

Add one method:

```rust
/// Create a tag or bookmark, optionally annotated.
///
/// `message = None`  → lightweight tag (current behaviour).
/// `message = Some(s)` → annotated tag with message `s` (Git only).
pub async fn create_tag_with_message(
    project: &Project,
    tag_name: &str,
    message: Option<&str>,
) -> ProjectOperationResult {
    let kind = detect_vcs_kind(Path::new(&project.path)).await;
    match (kind, message) {
        (Some(VcsKind::Git), Some(msg)) =>
            git::tag_create_annotated(project, tag_name, msg).await,
        (Some(VcsKind::Git), None) =>
            git::tag_create(project, tag_name).await,
        (Some(VcsKind::Jujutsu), _) =>
            jj::bookmark_create(project, tag_name).await,
        (None, _) => ProjectOperationResult::error(project.id.clone(),
            format!("no repository at {}", project.path)),
    }
}
```

Keep `VcsAdapter::create_tag` as-is (no message → lightweight tag); it is
used in integration tests and by `execute_freeze` internally.

### `git::tag_create_annotated`

```rust
pub(crate) async fn tag_create_annotated(
    project: &Project,
    tag_name: &str,
    message: &str,
) -> ProjectOperationResult {
    let path   = Path::new(&project.path).to_path_buf();
    let name   = tag_name.to_owned();
    let msg    = message.to_owned();
    let pid    = project.id.clone();

    tokio::task::spawn_blocking(move || {
        use endringer_backend_core::backend::VcsBackend;
        match endringer_backend_git::GitBackend::open(&path) {
            Err(e) => ProjectOperationResult::error(pid, e.to_string()),
            Ok(b)  => match b.create_annotated_tag(&name, &msg) {
                Ok(()) => ProjectOperationResult::success(
                    pid, vec![format!("git tag -a {name} -m ...")],
                ),
                Err(e) => ProjectOperationResult::error(pid, e.to_string()),
            },
        }
    }).await.unwrap_or_else(|e| ProjectOperationResult::error(
        project.id.clone(), format!("task join: {e}"),
    ))
}
```

### Freezer state

Add one field to `FreezePlanDraft` (or pass through the existing
`FreezerMessage::NameChanged` pattern):

```rust
// state/freezer.rs
pub struct FreezerPlanDraft {
    pub freeze_name:    String,
    pub tag_message:    String,   // NEW — empty = lightweight
    pub project_ids:    HashSet<ProjectId>,
}
```

Add `FreezerMessage::MessageChanged(String)` (mirrors `NameChanged`).

In `execute_freeze`, dispatch to `create_tag_with_message`:

```rust
let msg_opt = if draft.tag_message.trim().is_empty() {
    None
} else {
    Some(draft.tag_message.trim())
};
VcsAdapter::create_tag_with_message(&project, &draft.freeze_name, msg_opt).await
```

### i18n keys

```
freezer.tag_message_label   = "Tag message (optional, annotated tag)"
freezer.tag_message_hint    = "Leave blank for a lightweight tag"
freezer.tag_message_jj_note = "Message is not used for jj bookmarks"
```

(ja translations follow the same pattern.)

## Requirements

| # | Requirement |
|---|-------------|
| R1 | Empty message → lightweight tag (no behaviour change) |
| R2 | Non-empty message → annotated tag via `GitBackend::create_annotated_tag` |
| R3 | jj bookmarks are created regardless of message; message is silently ignored |
| R4 | Rollback (`delete_tag`) works the same for annotated and lightweight tags |
| R5 | The existing `create_tag` method is not changed |

## Test Plan

Add to `git_integration.rs`:

1. **`annotated_tag_create_and_delete`** — create an annotated tag with a
   known message, verify it appears in `list_tags`, delete it, verify it is
   gone.
2. **`annotated_tag_freeze_validation_blocks`** — annotated tag already exists
   → freeze validation returns a blocker (same as lightweight).

## Security Considerations

The message is a free-text string written as a git tag annotation.  It is
stored in the local repository only and never transmitted.  No sanitisation
is required beyond trimming.
