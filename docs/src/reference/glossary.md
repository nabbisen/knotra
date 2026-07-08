# Glossary & Status Vocabulary

| Term | Meaning |
|---|---|
| **Synced** | Local and remote identical; working tree clean |
| **Behind** | Remote has commits not yet pulled |
| **Ahead** | Local has commits not yet pushed |
| **Uncommitted** | Modified or staged tracked files |
| **Conflict** | Repository in a merge/rebase/cherry-pick conflict state |
| **Unknown** | Status read failed |
| **Excluded** | Skipped for the current bulk operation |
| **Rolled back** | Partial failure; previously applied changes undone |
| **Rollback failed** | Partial failure; rollback also failed — manual action needed |

| Generic | Git | jj |
|---|---|---|
| Context | Branch | Change-ID + bookmark |
| Static point | Tag | Bookmark |
| Switch context | `git switch` | `jj edit` |
| Fetch | `git fetch --prune` | `jj git fetch` |
