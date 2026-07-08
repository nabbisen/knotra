//! All `Message` variants for the knotra GUI.

use knotra_vcs::{
    ContextList, ContextSwitchResult, FreezeResult, FreezeValidation, ProjectId, WorkspaceStatus,
    model::operation::{OperationId, OperationLog, SmartPullPlan, SmartPullProgress},
    model::workspace::WorkspaceId,
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
    /// Open an external tool (editor or merge tool).
    #[allow(dead_code)]
    Launch(LaunchMessage),
    TagPush(TagPushMessage),
    ConflictOps(ConflictOpsMessage),
    Changelog(ChangelogMessage),
    Topology(TopologyMessage),
    Shortcut(ShortcutMessage),
    Selection(SelectionMessage),
    Activity(ActivityMessage),
    Palette(PaletteMessage),
    Tier(TierMessage),
    KeyEvent(KeyboardMessage),
    DetailPanel(DetailPanelMessage),
    /// Periodic tick from the FS-watch subscription.
    FsWatchTick,
    /// Request to write text to the system clipboard.
    CopyToClipboard(String),
    Tick,
}

// --- Workspace ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WorkspaceMessage {
    RefreshRequested,
    WorkspaceSwitched(WorkspaceId),
    // Project management
    AddProjectDialogOpened,
    AddProjectNameChanged(String),
    AddProjectPathChanged(String),
    AddProjectConfirmed,
    AddProjectCancelled,
    /// User clicked the folder browse button.
    BrowsePathRequested,
    /// Native folder dialog returned (None = cancelled).
    BrowsePathSelected(Option<String>),
    RemoveProjectRequested(ProjectId),
    RemoveProjectConfirmed(ProjectId),
    RemoveProjectCancelled,
    // Multi-workspace management
    CreateWorkspaceDialogOpened,
    CreateWorkspaceNameChanged(String),
    CreateWorkspaceConfirmed,
    CreateWorkspaceCancelled,
    RenameWorkspaceDialogOpened,
    RenameWorkspaceNameChanged(String),
    RenameWorkspaceConfirmed,
    RenameWorkspaceCancelled,
    DeleteWorkspaceRequested,
    DeleteWorkspaceConfirmed,
    DeleteWorkspaceCancelled,
}

// --- Project ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ProjectMessage {
    StatusRefreshRequested(ProjectId),
    FetchRequested(ProjectId),
}

// --- Sync ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SyncMessage {
    OpenRequested,
    ProjectToggled(ProjectId, bool),
    DispositionChanged(ProjectId, knotra_vcs::SmartPullDisposition),
    BulkFetchRequested,
    SmartPullPlanRequested,
    SmartPullConfirmed(SmartPullPlan),
    SmartPullCancelled,
    RetryFailedRequested,
    BulkPullRequested,
    PlanRequested,
    ExecuteRequested,
    ModalClosed,
    Cancelled,
}

// --- Context ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ContextMessage {
    OpenRequested(Option<ProjectId>),
    ProjectSelected(ProjectId),
    SearchChanged(String),
    SwitchTargetChosen(ProjectId, String),
    SwitchConfirmed,
    SwitchCancelled,
    BackToDashboard,
    BulkOpenRequested,
    BulkSwitchRequested,
    BulkModalClosed,
    TargetChanged(String),
    Cancelled,
}

// --- Freezer ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FreezerMessage {
    /// User navigated to the Freezer screen.
    OpenRequested,
    /// User typed in the freeze-point name field.
    NameChanged(String),
    /// User typed in the optional tag annotation message field.
    TagMessageChanged(String),
    BulkOpenRequested,
    BulkModalClosed,
    /// Alias for ExecuteConfirmed — used by bulk modal Execute button.
    ExecuteRequested,
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
#[allow(dead_code)]
pub enum HistoryMessage {
    SearchChanged(String),
    LogCopyRequested(OperationId),
    EntryToggled(OperationId),
    BackToDashboard,
}

// --- Settings ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SettingsMessage {
    LocaleChanged(knotra_ui::i18n::Locale),
    ThemeChanged(bool),
    RefreshIntervalChanged(u32),
    MaxConcurrentChanged(usize),
    ExternalEditorChanged(String),
    ExternalMergeToolChanged(String),
    MaxLogEntriesChanged(usize),
    FsWatchEnabledChanged(bool),
    FsDebounceSecs(u32),
    SaveRequested,
    /// Navigate back to Dashboard.
    BackToDashboard,
}

// --- Background ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    /// Conflict file list loaded.
    ConflictFilesLoaded(knotra_vcs::ProjectConflictDetail),
    /// Changelog draft generated.
    ChangelogDraftReady(knotra_vcs::ChangelogDraft),
    /// Available tags loaded for changelog since-selector.
    TagsLoaded(Vec<String>),
    /// Topology graph scanned.
    TopologyScanned(knotra_vcs::DependencyGraph),
    /// Tag push completed for all offered projects.
    TagPushCompleted {
        success_count: usize,
        fail_count: usize,
    },
    /// Missing repository paths detected at refresh.
    MissingProjectsDetected(Vec<ProjectId>),
    /// Validation phase completed.
    FreezeValidationDone(FreezeValidation),
    /// Execution phase completed.
    FreezeExecutionDone(FreezeResult),
    TaskError {
        description: String,
    },
}

// --- Filter ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FilterMessage {
    SearchChanged(String),
    GroupChanged(Option<String>),
    StatusFilterToggled(StatusFilter),
    AllFiltersCleared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StatusFilter {
    Healthy,
    Behind,
    Ahead,
    Dirty,
    Conflict,
    Error,
}
#[allow(dead_code)]
impl StatusFilter {
    pub fn label_key(&self) -> &'static str {
        match self {
            StatusFilter::Healthy => "filter.healthy",
            StatusFilter::Behind => "filter.behind",
            StatusFilter::Ahead => "filter.ahead",
            StatusFilter::Dirty => "filter.dirty",
            StatusFilter::Conflict => "filter.conflict",
            StatusFilter::Error => "filter.error",
        }
    }
}

// --- Conflict resolution ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConflictOpsMessage {
    OpenRequested(Option<ProjectId>),
    ProjectSelected(ProjectId),
    RecheckRequested(ProjectId),
    MarkResolvedRequested {
        project_id: ProjectId,
        file_path: String,
    },
    AbortMergeRequested(ProjectId),
    AbortMergeConfirmed(ProjectId),
    BackToDashboard,
    /// RFC-013: mark a file resolved in the resolve panel.
    FileMarkedResolved(String),
    /// RFC-013: abort merge in the resolve panel.
    AbortRequested,
    /// RFC-013: close the resolve panel.
    PanelClosed,
}

// --- Changelog ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ChangelogMessage {
    OpenRequested,
    SinceRefChanged(String),
    ProjectToggled(ProjectId, bool),
    LoadTagsRequested,
    GenerateRequested,
    CopyRequested,
    BackToDashboard,
    CollectRequested,
    ModalClosed,
}

// --- Topology ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TopologyMessage {
    ScanRequested,
}

// --- Tag push (post-freeze) ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TagPushMessage {
    /// Offer to push after successful freeze.
    OfferShown {
        freeze_name: String,
        project_ids: Vec<ProjectId>,
    },
    /// User accepted the push.
    PushConfirmed,
    /// User declined.
    PushDeclined,
}

// --- External tool launch ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LaunchMessage {
    /// Open a file path in the configured external editor.
    OpenInEditor(String),
    /// Open a file path in the configured merge tool.
    OpenInMergeTool(String),
}

// --- Shortcuts ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ShortcutMessage {
    Refresh,
    OpenContextOps,
    OpenFreezer,
    FocusSearch,
    Close,
}

// ---------------------------------------------------------------------------
// RFC-009 — Selection messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SelectionMessage {
    Toggled(ProjectId),
    RangeTo(ProjectId),
    SelectAll,
    Clear,
    FocusMoved(ProjectId),
    /// Enter selection mode (show checkboxes).
    ModeEntered,
    /// Exit selection mode and clear selection.
    ModeExited,
}

// ---------------------------------------------------------------------------
// RFC-011 — Activity strip messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ActivityMessage {
    Started { label: String, total: usize },
    Progress { done: usize },
    Completed { log: OperationLog },
    PopoverToggled,
    RetryRequested,
    Tick,
}

// ---------------------------------------------------------------------------
// RFC-012 — Command palette messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PaletteMessage {
    Opened,
    Closed,
    QueryChanged(String),
    MoveUp,
    MoveDown,
    Confirmed,
    EntryClicked(usize),
}

// ---------------------------------------------------------------------------
// RFC-010 — Attention tier messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TierMessage {
    Toggled(crate::state::AttentionTier),
    GroupingModeChanged(crate::state::GroupingMode),
}

// ---------------------------------------------------------------------------
// RFC-016 — Keyboard messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeyboardMessage {
    CheatSheetToggled,
    LeaderGPressed,
    LeaderCancelled,
}

// ---------------------------------------------------------------------------
// RFC-014 — Detail panel messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum DetailPanelMessage {
    Opened(ProjectId),
    Closed,
}
