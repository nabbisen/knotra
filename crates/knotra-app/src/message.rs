//! All `Message` variants for the knotra GUI.

use endringer::{
    ContextList, ContextSwitchResult, FreezeResult, FreezeValidation,
    Project, ProjectId,
    model::operation::{OperationId, OperationLog, SmartPullPlan, SmartPullProgress},
    model::workspace::WorkspaceId,
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
    Launch(LaunchMessage),
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
    OpenRequested(Option<ProjectId>),
    ProjectSelected(ProjectId),
    SearchChanged(String),
    SwitchTargetChosen(ProjectId, String),
    SwitchConfirmed,
    SwitchCancelled,
    BackToDashboard,
}

// --- Freezer ---
#[derive(Debug, Clone)]
pub enum FreezerMessage {
    /// User navigated to the Freezer screen.
    OpenRequested,
    /// User typed in the freeze-point name field.
    NameChanged(String),
    /// User toggled a project's inclusion.
    ProjectToggled(ProjectId, bool),
    /// User requested pre-execution validation.
    ValidateRequested,
    /// User confirmed execution after seeing the validation results.
    ExecuteConfirmed,
    /// User cancelled (returns to validation or idle).
    Cancelled,
    /// User wants to re-run validation after fixing issues.
    RevalidateRequested,
    /// Navigate back to Dashboard after completion.
    BackToDashboard,
}

// --- History ---
#[derive(Debug, Clone)]
pub enum HistoryMessage {
    SearchChanged(String),
    LogCopyRequested(OperationId),
    EntryToggled(OperationId),
    BackToDashboard,
}

// --- Settings ---
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    LocaleChanged(snora::i18n::Locale),
    ThemeChanged(bool),
    RefreshIntervalChanged(u32),
    MaxConcurrentChanged(usize),
    ExternalEditorChanged(String),
    ExternalMergeToolChanged(String),
    MaxLogEntriesChanged(usize),
    SaveRequested,
    /// Navigate back to Dashboard.
    BackToDashboard,
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
    ContextListLoaded(ContextList),
    ContextSwitchDone(ContextSwitchResult),
    /// Validation phase completed.
    FreezeValidationDone(FreezeValidation),
    /// Execution phase completed.
    FreezeExecutionDone(FreezeResult),
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

// --- External tool launch ---
#[derive(Debug, Clone)]
pub enum LaunchMessage {
    /// Open a file path in the configured external editor.
    OpenInEditor(String),
    /// Open a file path in the configured merge tool.
    OpenInMergeTool(String),
}

// --- Shortcuts ---
#[derive(Debug, Clone)]
pub enum ShortcutMessage {
    Refresh, OpenContextOps, OpenFreezer, FocusSearch, Close,
}
