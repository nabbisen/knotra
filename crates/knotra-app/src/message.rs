//! All `Message` variants for the knotra GUI.

use knotra_vcs::{
    ContextList, ContextSwitchResult, ContextTarget, FreezeResult, FreezeValidation, ProjectId,
    WorkspaceStatus,
    model::operation::{OperationId, OperationLog, SmartPullPlan, SmartPullProgress},
    model::workspace::WorkspaceId,
};

use crate::state::sync::RetryPreparationId;
use crate::state::{OperationLeaseId, Screen};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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
    Dashboard(DashboardMessage),
    KeyEvent(KeyboardMessage),
    DetailPanel(DetailPanelMessage),
    /// Periodic tick from the FS-watch subscription.
    FsWatchTick,
    /// Request to write text to the system clipboard.
    CopyToClipboard(String),
    /// Toggle "Show details / Hide details" in modal result screens (RFC-0021 Phase 2).
    ToggleOpDetails,
    Tick,
    /// RFC-035 R8/Handoff 029: the window resized. Recomputes
    /// `state.width_mode` — iced's own documented subscription shape
    /// (`iced-0.14.0/src/lib.rs:358`), not a knotra-invented mechanism.
    WindowResized(iced::Size),
}

// --- Workspace ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WorkspaceMessage {
    RefreshRequested,
    WorkspaceSwitched(WorkspaceId),
    /// Open/close the shell's workspace-switcher dropdown (RFC-034 R12).
    SwitcherToggled,
    // Project management
    AddProjectDialogOpened,
    AddProjectNameChanged(String),
    AddProjectPathChanged(String),
    /// Advance from Step 1 (folder) to Step 2 (name) in the guided flow.
    AddProjectNextStep,
    AddProjectConfirmed,
    AddProjectCancelled,
    /// User clicked the folder browse button.
    BrowsePathRequested,
    /// Native folder dialog returned (None = cancelled).
    BrowsePathSelected(Option<String>),
    RemoveProjectRequested(ProjectId),
    RemoveProjectConfirmed(ProjectId),
    RemoveProjectCancelled,
    /// Undo the most recent project removal (RFC-0021 Phase 5).
    UndoRemoval,
    /// Dismiss the undo snackbar without undoing.
    DismissUndoSnackbar,
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
    BulkFetchAllRequested,
    SmartPullPlanRequested,
    SmartPullConfirmed(SmartPullPlan),
    SmartPullCancelled,
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
    SwitchTargetChosen(ProjectId, ContextTarget, String),
    SwitchConfirmed,
    SwitchCancelled,
    BackToDashboard,
    BulkOpenRequested,
    BulkModalClosed,
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
    SingleFetchCompleted {
        lease_id: OperationLeaseId,
        log: OperationLog,
    },
    SmartPullProjectCompleted {
        lease_id: OperationLeaseId,
        progress: SmartPullProgress,
    },
    ActivityFetchRetryProjectCompleted {
        lease_id: OperationLeaseId,
        operation_id: OperationId,
        result: knotra_vcs::model::operation::ProjectOperationResult,
    },
    SmartPullRetryStatusReady {
        request_id: RetryPreparationId,
        workspace_id: WorkspaceId,
        lease_id: OperationLeaseId,
        statuses: Vec<knotra_vcs::ProjectStatus>,
    },
    SmartPullPlanReady(SmartPullPlan),
    ContextListLoaded(ContextList),
    /// Conflict file list loaded.
    ConflictFilesLoaded(knotra_vcs::ProjectConflictDetail),
    /// Conflict mutation completed, with refreshed conflict state.
    ConflictOperationCompleted {
        lease_id: OperationLeaseId,
        result: knotra_vcs::model::operation::ProjectOperationResult,
        detail: knotra_vcs::ProjectConflictDetail,
    },
    /// Changelog draft generated.
    ChangelogDraftReady {
        request_id: u64,
        draft: knotra_vcs::ChangelogDraft,
    },
    /// Available tags loaded for changelog since-selector.
    TagsLoaded(Vec<String>),
    /// Topology graph scanned.
    TopologyScanned(knotra_vcs::DependencyGraph),
    /// Tag push completed for all offered projects.
    TagPushCompleted {
        lease_id: OperationLeaseId,
        success_count: usize,
        fail_count: usize,
    },
    /// Missing repository paths detected at refresh.
    MissingProjectsDetected(Vec<ProjectId>),
    /// Validation phase completed.
    FreezeValidationDone {
        lease_id: OperationLeaseId,
        validation: FreezeValidation,
    },
    /// Execution phase completed.
    FreezeExecutionDone {
        lease_id: OperationLeaseId,
        result: FreezeResult,
    },
    ContextSwitchDone {
        lease_id: OperationLeaseId,
        result: ContextSwitchResult,
    },
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
    AllSet,
    Behind,
    Ahead,
    Dirty,
    Conflict,
    NeedsHelp,
}
#[allow(dead_code)]
impl StatusFilter {
    pub fn label_key(&self) -> &'static str {
        match self {
            StatusFilter::AllSet => "filter.all_set",
            StatusFilter::Behind => "filter.behind",
            StatusFilter::Ahead => "filter.ahead",
            StatusFilter::Dirty => "filter.dirty",
            StatusFilter::Conflict => "filter.conflict",
            StatusFilter::NeedsHelp => "filter.needs_help",
        }
    }
}

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    GroupingChanged(crate::config::DashboardGrouping),
    SortChanged(crate::config::DashboardSort),
    TierToggled(crate::state::dashboard::DashboardTier),
    ErrorDetailsToggled,
    ErrorRetryRequested,
    /// RFC-035 R8/Handoff 028 Ruling 6.1: compact toolbar's chip overflow (`⋯`).
    ToolbarOverflowToggled,
    /// RFC-035 R8/Handoff 028 Ruling 6.1: compact toolbar's selector disclosure (`▾`).
    ToolbarSelectorsToggled,
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
    /// RFC-0013: mark a file resolved in the resolve panel.
    FileMarkedResolved(String),
    /// RFC-0021 Phase 3: open a conflicted file in the configured external editor.
    OpenInEditorRequested(String),
    /// RFC-0013: abort merge in the resolve panel.
    AbortRequested,
    /// RFC-0013: close the resolve panel.
    PanelClosed,
}

// --- Changelog ---
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ChangelogMessage {
    OpenRequested,
    BulkOpenRequested,
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
    /// `Ctrl`/`Cmd`+`/` — unconditional, unchanged from before RFC-036.
    FocusSearch,
    Close,
    /// RFC-036 R1: Tab.
    FocusNext,
    /// RFC-036 R1: Shift-Tab.
    FocusPrevious,
    /// RFC-036 R3/R3a: Enter or Space, gated on text-input focus.
    ActivateFocused,
    /// RFC-036 R4: bare `/`, gated on text-input focus so a literal `/`
    /// typed into a field is never intercepted.
    FocusSearchBare,
}

// ---------------------------------------------------------------------------
// RFC-0009 — Selection messages
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
// RFC-0011 — Activity strip messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ActivityMessage {
    RetryRequested { source_operation_id: OperationId },
    DetailsRequested { operation_id: OperationId },
    Tick,
}

// ---------------------------------------------------------------------------
// RFC-0012 — Command palette messages
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
// RFC-0016 — Keyboard messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeyboardMessage {
    CheatSheetToggled,
    LeaderGPressed,
    LeaderCancelled,
}

// ---------------------------------------------------------------------------
// RFC-0014 — Detail panel messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum DetailPanelMessage {
    Opened(ProjectId),
    Closed,
}
