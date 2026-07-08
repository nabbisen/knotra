//! All `Message` variants for the knotra GUI.

use endringer::{
    model::{
        operation::{OperationId, OperationLog, SmartPullPlan, SmartPullProgress},
        project::{Project, ProjectId},
        workspace::WorkspaceId,
    },
    WorkspaceStatus,
};

use crate::state::Screen;

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
    Shortcut(ShortcutMessage),
    Tick,
}

// Workspace
#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    RefreshRequested,
    WorkspaceSwitched(WorkspaceId),
    AddProjectDialogOpened,
    AddProjectNameChanged(String),
    AddProjectPathChanged(String),
    AddProjectConfirmed,
    AddProjectCancelled,
    RemoveProjectRequested(ProjectId),
    RemoveProjectConfirmed(ProjectId),
    RemoveProjectCancelled,
}

// Project
#[derive(Debug, Clone)]
pub enum ProjectMessage {
    StatusRefreshRequested(ProjectId),
    FetchRequested(ProjectId),
}

// Sync
#[derive(Debug, Clone)]
pub enum SyncMessage {
    /// Open the Sync Center and build the initial plan.
    OpenRequested,
    /// Toggle whether a project is included in the current operation.
    ProjectToggled(ProjectId, bool),
    /// Set the dirty-project disposition for one entry.
    DispositionChanged(ProjectId, endringer::SmartPullDisposition),
    /// User requested bulk fetch of selected projects.
    BulkFetchRequested,
    /// User requested Smart Pull — begin planning.
    SmartPullPlanRequested,
    /// User confirmed the Smart Pull plan and execution should begin.
    SmartPullConfirmed(SmartPullPlan),
    /// User cancelled the Smart Pull plan.
    SmartPullCancelled,
    /// Retry only the failed projects from the last run.
    RetryFailedRequested,
}

// Context
#[derive(Debug, Clone)]
pub enum ContextMessage {
    ProjectSelected(ProjectId),
    SwitchRequested { project_id: ProjectId, target: String },
    SwitchConfirmed,
    SwitchCancelled,
}

// Freezer
#[derive(Debug, Clone)]
pub enum FreezerMessage {
    NameChanged(String),
    ValidationRequested,
    ExecutionConfirmed,
    ExecutionCancelled,
}

// History
#[derive(Debug, Clone)]
pub enum HistoryMessage {
    SearchChanged(String),
    LogCopyRequested(OperationId),
    EntryToggled(OperationId),
}

// Settings
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    LocaleChanged(snora::i18n::Locale),
    ThemeChanged(bool),
    RefreshIntervalChanged(u32),
    MaxConcurrentChanged(usize),
    SaveRequested,
}

// Background
#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    WorkspaceStatusRefreshed(WorkspaceStatus),
    BulkFetchCompleted(OperationLog),
    SmartPullCompleted(OperationLog),
    ContextSwitchCompleted(OperationLog),
    FreezeCompleted(OperationLog),
    SingleFetchCompleted(OperationLog),
    /// One project finished during a streaming operation.
    SmartPullProjectCompleted(SmartPullProgress),
    /// Planning step finished — ready for user confirmation.
    SmartPullPlanReady(SmartPullPlan),
    TaskError { description: String },
}

// Filter
#[derive(Debug, Clone)]
pub enum FilterMessage {
    SearchChanged(String),
    GroupChanged(Option<String>),
    StatusFilterToggled(StatusFilter),
    AllFiltersCleared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    Healthy, Behind, Ahead, Dirty, Conflict, Error,
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

// Shortcuts
#[derive(Debug, Clone)]
pub enum ShortcutMessage {
    Refresh,
    OpenContextOps,
    OpenFreezer,
    FocusSearch,
    Close,
}
