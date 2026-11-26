# Glossary & Status Vocabulary

| Term | Meaning |
|---|---|
| **All set** | Local and remote identical; working tree clean |
| **Updates available** | Remote has commits not yet pulled |
| **Local commits** | Local has commits not yet pushed |
| **Unsaved files** | Modified or staged tracked files |
| **Needs a choice** | Repository in a merge/rebase/cherry-pick conflict state |
| **Unknown** | Status read failed |
| **Skipped** | Skipped for the current bulk operation |
| **Rolled back** | Partial failure; previously applied changes undone |
| **Rollback failed** | Partial failure; rollback also failed — manual action needed |

| Generic | Git | jj |
|---|---|---|
| Work area | Branch | Change-ID + bookmark |
| Release point | Tag | Bookmark |
| Change work area | `git switch` | `jj edit` |
| Check for updates | `git fetch --prune` | `jj git fetch` |
