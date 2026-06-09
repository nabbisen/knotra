//! All `Message` variants for the knotra GUI.
//!
//! Messages are grouped by domain and named for the *intent* they express,
//! not for the widget that produced them.

use endringer::{
    WorkspaceStatus,
    model::{
        operation::{OperationId, OperationLog},
        project::{Project, ProjectId},
        workspace::WorkspaceId,
    },
};

use crate::state::Screen;

// ---------------------------------------------------------------------------
// Top-level message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Message {
    /// Navigation to a different screen.
    Navigate(Screen),

    /// Workspace-level messages.
    Workspace(WorkspaceMessage),

    /// Per-project messages.
    Project(ProjectMessage),

    /// Bulk synchronisation messages.
    Sync(SyncMessage),

    /// Context-switch messages.
    Context(ContextMessage),

    /// Freezer (static-point creation) messages.
    Freezer(FreezerMessage),

    /// History screen messages.
    History(HistoryMessage),

    /// Application settings messages.
    Settings(SettingsMessage),

    /// Background task completion messages.
    Background(BackgroundMessage),

    /// Search / filter toolbar messages.
    Filter(FilterMessage),
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    /// The user requested a full status refresh of the active workspace.
    RefreshRequested,
    /// User switched to a different named workspace.
    WorkspaceSwitched(WorkspaceId),
    /// User opened the "add project" dialog.
    AddProjectDialogOpened,
    /// User confirmed adding a new project.
    ProjectAdded(Project),
    /// User confirmed removing a project.
    ProjectRemoved(ProjectId),
}

// ---------------------------------------------------------------------------
// Project (single-repo actions from the dashboard card)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ProjectMessage {
    /// Request a status refresh for a single project.
    StatusRefreshRequested(ProjectId),
    /// User clicked the card to expand / inspect it.
    CardExpanded(ProjectId),
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SyncMessage {
    /// User requested a bulk fetch for the selected projects.
    BulkFetchRequested(Vec<ProjectId>),
    /// User requested a Smart Pull (with dirty-state check).
    SmartPullRequested(Vec<ProjectId>),
    /// User toggled inclusion of a project in the current bulk operation.
    ProjectToggled(ProjectId, bool),
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ContextMessage {
    /// User selected a project to operate on.
    ProjectSelected(ProjectId),
    /// User requested a context switch to the given target.
    SwitchRequested {
        project_id: ProjectId,
        target: String,
    },
    /// User confirmed a pending context switch.
    SwitchConfirmed,
    /// User cancelled a pending context switch.
    SwitchCancelled,
}

// ---------------------------------------------------------------------------
// Freezer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum FreezerMessage {
    /// User typed in the freeze-point name field.
    NameChanged(String),
    /// User requested pre-execution validation.
    ValidationRequested,
    /// User confirmed and started the freeze.
    ExecutionConfirmed,
    /// User cancelled before execution.
    ExecutionCancelled,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum HistoryMessage {
    /// User typed in the history search field.
    SearchChanged(String),
    /// User requested to copy a log entry.
    LogCopyRequested(OperationId),
    /// User expanded / collapsed a log entry.
    EntryToggled(OperationId),
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    /// User changed the UI locale.
    LocaleChanged(snora::i18n::Locale),
    /// User changed the theme preference.
    ThemeChanged(bool), // true = dark
    /// User changed the refresh interval (seconds).
    RefreshIntervalChanged(u32),
    /// User changed the maximum concurrent status-read tasks.
    MaxConcurrentChanged(usize),
    /// User confirmed saving settings.
    SaveRequested,
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    /// A workspace-wide status refresh has completed.
    WorkspaceStatusRefreshed(WorkspaceStatus),
    /// A bulk fetch operation completed.
    BulkFetchCompleted(OperationLog),
    /// A Smart Pull operation completed.
    SmartPullCompleted(OperationLog),
    /// A context switch completed.
    ContextSwitchCompleted(OperationLog),
    /// A freeze operation completed.
    FreezeCompleted(OperationLog),
    /// A background task produced an error that should surface in the UI.
    TaskError { description: String },
}

// ---------------------------------------------------------------------------
// Filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum FilterMessage {
    /// User typed in the search box.
    SearchChanged(String),
    /// User changed the active group filter.
    GroupChanged(Option<String>),
    /// User toggled a status filter (e.g. "show only Behind").
    StatusFilterToggled(StatusFilter),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    Healthy,
    Behind,
    Ahead,
    Dirty,
    Conflict,
    Error,
}
