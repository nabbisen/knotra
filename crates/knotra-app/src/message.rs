//! All `Message` variants for the knotra GUI.
//!
//! Messages are grouped by domain and named for the *intent* they express,
//! not for the widget that produced them.

use endringer::model::{
    operation::{OperationId, OperationLog},
    project::{Project, ProjectId},
    workspace::WorkspaceId,
};
use endringer::WorkspaceStatus;

use crate::state::Screen;

// ---------------------------------------------------------------------------
// Top-level message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Screen),
    Workspace(WorkspaceMessage),
    Project(ProjectMessage),
    Sync(SyncMessage),
    Context(ContextMessage),
    Freezer(FreezerMessage),
    History(HistoryMessage),
    Settings(SettingsMessage),
    Background(BackgroundMessage),
    Filter(FilterMessage),
    /// Keyboard shortcut activated.
    Shortcut(ShortcutMessage),
    /// Periodic tick from the background subscription.
    Tick,
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    RefreshRequested,
    WorkspaceSwitched(WorkspaceId),
    /// Open the add-project dialog.
    AddProjectDialogOpened,
    /// User typed in the dialog name field.
    AddProjectNameChanged(String),
    /// User typed in the dialog path field.
    AddProjectPathChanged(String),
    /// User confirmed the add-project dialog.
    AddProjectConfirmed,
    /// User cancelled the add-project dialog.
    AddProjectCancelled,
    /// Remove a project after confirmation.
    RemoveProjectRequested(ProjectId),
    /// Remove confirmed.
    RemoveProjectConfirmed(ProjectId),
    /// Remove cancelled.
    RemoveProjectCancelled,
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ProjectMessage {
    StatusRefreshRequested(ProjectId),
    FetchRequested(ProjectId),
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SyncMessage {
    BulkFetchRequested(Vec<ProjectId>),
    SmartPullRequested(Vec<ProjectId>),
    ProjectToggled(ProjectId, bool),
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ContextMessage {
    ProjectSelected(ProjectId),
    SwitchRequested { project_id: ProjectId, target: String },
    SwitchConfirmed,
    SwitchCancelled,
}

// ---------------------------------------------------------------------------
// Freezer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum FreezerMessage {
    NameChanged(String),
    ValidationRequested,
    ExecutionConfirmed,
    ExecutionCancelled,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum HistoryMessage {
    SearchChanged(String),
    LogCopyRequested(OperationId),
    EntryToggled(OperationId),
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    LocaleChanged(snora::i18n::Locale),
    ThemeChanged(bool),
    RefreshIntervalChanged(u32),
    MaxConcurrentChanged(usize),
    SaveRequested,
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    WorkspaceStatusRefreshed(WorkspaceStatus),
    BulkFetchCompleted(OperationLog),
    SmartPullCompleted(OperationLog),
    ContextSwitchCompleted(OperationLog),
    FreezeCompleted(OperationLog),
    SingleFetchCompleted(OperationLog),
    TaskError { description: String },
}

// ---------------------------------------------------------------------------
// Filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum FilterMessage {
    SearchChanged(String),
    GroupChanged(Option<String>),
    StatusFilterToggled(StatusFilter),
    AllFiltersCleared,
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

impl StatusFilter {
    pub fn label_key(&self) -> &'static str {
        match self {
            StatusFilter::Healthy  => "filter.healthy",
            StatusFilter::Behind   => "filter.behind",
            StatusFilter::Ahead    => "filter.ahead",
            StatusFilter::Dirty    => "filter.dirty",
            StatusFilter::Conflict => "filter.conflict",
            StatusFilter::Error    => "filter.error",
        }
    }
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ShortcutMessage {
    Refresh,
    OpenContextOps,
    OpenFreezer,
    FocusSearch,
    Close,
}
