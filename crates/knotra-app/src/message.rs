//! All `Message` variants for the knotra GUI.

use endringer::{
    model::operation::{OperationId, OperationLog, SmartPullPlan, SmartPullProgress},
    ContextList, ContextSwitchResult, ProjectId, WorkspaceStatus,
};
use endringer::model::workspace::WorkspaceId;
use endringer::Project;

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

// --- Workspace ---
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

// --- Project ---
#[derive(Debug, Clone)]
pub enum ProjectMessage {
    StatusRefreshRequested(ProjectId),
    FetchRequested(ProjectId),
}

// --- Sync ---
#[derive(Debug, Clone)]
pub enum SyncMessage {
    OpenRequested,
    ProjectToggled(ProjectId, bool),
    DispositionChanged(ProjectId, endringer::SmartPullDisposition),
    BulkFetchRequested,
    SmartPullPlanRequested,
    SmartPullConfirmed(SmartPullPlan),
    SmartPullCancelled,
    RetryFailedRequested,
}

// --- Context ---
#[derive(Debug, Clone)]
pub enum ContextMessage {
    /// Open the Context Ops screen; optionally pre-select a project.
    OpenRequested(Option<ProjectId>),
    /// User selected a project to operate on.
    ProjectSelected(ProjectId),
    /// User typed in the context search box.
    SearchChanged(String),
    /// User clicked a candidate to switch to (requests confirmation dialog).
    SwitchTargetChosen(ProjectId, String),
    /// User confirmed the pending switch.
    SwitchConfirmed,
    /// User cancelled the pending switch dialog.
    SwitchCancelled,
    /// Navigate back to Dashboard.
    BackToDashboard,
}

// --- Freezer ---
#[derive(Debug, Clone)]
pub enum FreezerMessage {
    NameChanged(String),
    ValidationRequested,
    ExecutionConfirmed,
    ExecutionCancelled,
}

// --- History ---
#[derive(Debug, Clone)]
pub enum HistoryMessage {
    SearchChanged(String),
    LogCopyRequested(OperationId),
    EntryToggled(OperationId),
}

// --- Settings ---
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    LocaleChanged(snora::i18n::Locale),
    ThemeChanged(bool),
    RefreshIntervalChanged(u32),
    MaxConcurrentChanged(usize),
    SaveRequested,
}

// --- Background ---
#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    WorkspaceStatusRefreshed(WorkspaceStatus),
    BulkFetchCompleted(OperationLog),
    SmartPullCompleted(OperationLog),
    ContextSwitchCompleted(OperationLog),
    FreezeCompleted(OperationLog),
    SingleFetchCompleted(OperationLog),
    SmartPullProjectCompleted(SmartPullProgress),
    SmartPullPlanReady(SmartPullPlan),
    /// Context list loaded for a project.
    ContextListLoaded(ContextList),
    /// A context switch finished.
    ContextSwitchDone(ContextSwitchResult),
    TaskError { description: String },
}

// --- Filter ---
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

// --- Shortcuts ---
#[derive(Debug, Clone)]
pub enum ShortcutMessage {
    Refresh,
    OpenContextOps,
    OpenFreezer,
    FocusSearch,
    Close,
}
