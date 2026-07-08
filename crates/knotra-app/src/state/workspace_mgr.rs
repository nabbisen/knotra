//! Multi-workspace management state.

/// State for the workspace-switcher and manager UI.
#[derive(Debug, Default)]
pub struct WorkspaceMgrState {
    /// Dialog for creating a new workspace.
    pub create_dialog: Option<CreateWorkspaceDialog>,
    /// Dialog for renaming the active workspace.
    pub rename_dialog: Option<RenameWorkspaceDialog>,
    /// True when the delete-confirmation prompt is open.
    pub confirm_delete: bool,
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
}
