//! Multi-workspace management state.

use knotra_vcs::{Workspace, WorkspaceId};

/// State for the workspace-switcher and manager UI.
#[derive(Debug, Default)]
pub struct WorkspaceMgrState {
    /// Dialog for creating a new workspace.
    pub create_dialog: Option<CreateWorkspaceDialog>,
    /// Dialog for renaming the active workspace.
    pub rename_dialog: Option<RenameWorkspaceDialog>,
    /// Dialog for confirming workspace deletion.
    pub confirm_delete: Option<DeleteWorkspaceDialog>,
    /// Whether the shell's workspace-switcher dropdown (RFC-034 R12) is open.
    /// This is a `snora::AppLayout::header_menu`, not a `dialog` — it is
    /// dismissed by `on_close_menus` (click outside) or by choosing an item,
    /// not by `close_topmost_layer`'s Escape-driven stack.
    pub switcher_open: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CreateWorkspaceDialog {
    pub name: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RenameWorkspaceDialog {
    pub new_name: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteWorkspaceDialog {
    pub workspace_id: WorkspaceId,
    pub workspace_name: String,
    pub project_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceNameError {
    Empty,
    Duplicate,
}

impl WorkspaceNameError {
    pub fn i18n_key(self) -> &'static str {
        match self {
            WorkspaceNameError::Empty => "workspace.error.empty_name",
            WorkspaceNameError::Duplicate => "workspace.error.duplicate_name",
        }
    }
}

pub fn validate_workspace_name(
    candidate: &str,
    workspaces: &[Workspace],
    current_id: Option<&WorkspaceId>,
) -> Result<String, WorkspaceNameError> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return Err(WorkspaceNameError::Empty);
    }

    let candidate_folded = trimmed.to_lowercase();
    let duplicate = workspaces.iter().any(|ws| {
        let is_current = current_id.is_some_and(|id| id == &ws.id);
        !is_current && ws.name.trim().to_lowercase() == candidate_folded
    });

    if duplicate {
        Err(WorkspaceNameError::Duplicate)
    } else {
        Ok(trimmed.to_owned())
    }
}

pub fn next_active_index_after_delete(active_idx: usize, len_before_delete: usize) -> usize {
    debug_assert!(len_before_delete > 1);
    if active_idx >= len_before_delete.saturating_sub(1) {
        active_idx.saturating_sub(1)
    } else {
        active_idx
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_dialog_defaults_empty() {
        let d = CreateWorkspaceDialog::default();
        assert!(d.name.is_empty());
        assert!(d.error.is_none());
    }

    #[test]
    fn workspace_name_rejects_empty() {
        let workspaces = Vec::new();
        assert_eq!(
            validate_workspace_name("  ", &workspaces, None),
            Err(WorkspaceNameError::Empty)
        );
    }

    #[test]
    fn workspace_name_rejects_duplicate_case_insensitive() {
        let workspaces = vec![Workspace::new("Work")];
        assert_eq!(
            validate_workspace_name(" work ", &workspaces, None),
            Err(WorkspaceNameError::Duplicate)
        );
    }

    #[test]
    fn workspace_name_allows_current_name_for_rename() {
        let workspace = Workspace::new("Work");
        let id = workspace.id.clone();
        let workspaces = vec![workspace];
        assert_eq!(
            validate_workspace_name(" work ", &workspaces, Some(&id)),
            Ok("work".to_owned())
        );
    }

    #[test]
    fn delete_active_workspace_selects_nearest_remaining_workspace() {
        assert_eq!(next_active_index_after_delete(0, 3), 0);
        assert_eq!(next_active_index_after_delete(1, 3), 1);
        assert_eq!(next_active_index_after_delete(2, 3), 1);
    }
}
