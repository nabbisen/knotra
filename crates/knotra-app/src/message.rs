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
pub enum ProjectMessage {
    StatusRefreshRequested(ProjectId),
    FetchRequested(ProjectId),
}

// --- Sync ---
#[derive(Debug, Clone)]
pub enum SyncMessage {
    DispositionChanged(ProjectId, knotra_vcs::SmartPullDisposition),
    BulkFetchRequested,
    BulkFetchAllRequested,
    SmartPullPlanRequested,
    SmartPullConfirmed(SmartPullPlan),
    BulkPullRequested,
    ExecuteRequested,
    ModalClosed,
    Cancelled,
}

// --- Context ---
#[derive(Debug, Clone)]
pub enum ContextMessage {
    OpenRequested(Option<ProjectId>),
    SearchChanged(String),
    SwitchTargetChosen(ProjectId, ContextTarget, String),
    SwitchConfirmed,
    SwitchCancelled,
    BulkOpenRequested,
    BulkModalClosed,
    // RFC-043 Handoff 053: triage error — `tests.rs:3485` constructs this
    // directly. R7 forbids editing `tests.rs`. Restored with its original
    // handler; the Handoff 052 review's uncertainty about whether this
    // duplicates `BulkModalClosed` stands unresolved.
    #[allow(dead_code)]
    Cancelled,
}

// --- Freezer ---
#[derive(Debug, Clone)]
pub enum FreezerMessage {
    /// User navigated to the Freezer screen.
    OpenRequested,
    /// User typed in the freeze-point name field.
    NameChanged(String),
    /// User typed in the optional tag annotation message field.
    TagMessageChanged(String),
    BulkOpenRequested,
    BulkModalClosed,
    /// User requested pre-execution validation.
    ValidateRequested,
    /// User confirmed execution after seeing the validation results.
    ExecuteConfirmed,
    // RFC-043 Handoff 053: triage error — `tests.rs:3434` constructs this
    // directly (`freezer_validation_parameter_changes_release_the_lease`).
    // R7 forbids editing `tests.rs`. Restored with its original handler.
    #[allow(dead_code)]
    ProjectToggled(ProjectId, bool),
    // RFC-043 Handoff 053: triage error — `tests.rs:3406` constructs this
    // directly (part of the freezer bulk-modal-close/cancel/escape group
    // test). R7 forbids editing `tests.rs`. Restored with its original
    // handler.
    #[allow(dead_code)]
    Cancelled,
}

// --- History ---
#[derive(Debug, Clone)]
pub enum HistoryMessage {
    SearchChanged(String),
    EntryToggled(OperationId),
}

// --- Settings ---
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    LocaleChanged(knotra_ui::i18n::Locale),
    ThemeChanged(bool),
    // RFC-038 Stage 2 §1b: these four carry the raw typed text, not a
    // pre-parsed number — the view previously coerced unparseable input to
    // a magic default (0/1/10/2) at the message boundary, which destroyed
    // the user's input before validation could ever see it. Parsing (and
    // deciding what counts as valid) now happens in the handler.
    RefreshIntervalChanged(String),
    MaxConcurrentChanged(String),
    ExternalEditorChanged(String),
    ExternalMergeToolChanged(String),
    MaxLogEntriesChanged(String),
    FsWatchEnabledChanged(bool),
    FsDebounceSecs(String),
    SaveRequested,
}

// --- Background ---
#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    WorkspaceStatusRefreshed(WorkspaceStatus),
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
    /// Topology graph scanned.
    TopologyScanned(knotra_vcs::DependencyGraph),
    /// Tag push completed for all offered projects.
    TagPushCompleted {
        lease_id: OperationLeaseId,
        success_count: usize,
        fail_count: usize,
    },
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
    // RFC-043 Handoff 053 explicitly directed deleting this — it is
    // literally RFC-043's own headline example of Unreached — despite `052`
    // §3 naming its `tests.rs:366` construction
    // (`dashboard_error_details_and_retry_follow_workspace_guard`). But R7
    // forbids editing `tests.rs`, and the two cannot both hold. Restored
    // with its original handler pending the owner's resolution of that
    // conflict — see the Handoff 053 review request. Its downstream effect,
    // `LoadPhase::Error`, is the same discovery noted at that variant's
    // definition (`state.rs`).
    #[allow(dead_code)]
    TaskError {
        description: String,
    },
}

// --- Filter ---
#[derive(Debug, Clone)]
pub enum FilterMessage {
    SearchChanged(String),
    StatusFilterToggled(StatusFilter),
    AllFiltersCleared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    AllSet,
    Behind,
    Ahead,
    Dirty,
    Conflict,
    NeedsHelp,
}
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
pub enum ConflictOpsMessage {
    OpenRequested(Option<ProjectId>),
    ProjectSelected(ProjectId),
    MarkResolvedRequested {
        project_id: ProjectId,
        file_path: String,
    },
    AbortMergeRequested(ProjectId),
    /// RFC-0021 Phase 3: open a conflicted file in the configured external editor.
    OpenInEditorRequested(String),
    /// RFC-0013: close the resolve panel.
    PanelClosed,
}

// --- Changelog ---
#[derive(Debug, Clone)]
pub enum ChangelogMessage {
    BulkOpenRequested,
    SinceRefChanged(String),
    ProjectToggled(ProjectId, bool),
    GenerateRequested,
    CopyRequested,
    CollectRequested,
    ModalClosed,
}

// --- Topology ---
#[derive(Debug, Clone)]
pub enum TopologyMessage {
    ScanRequested,
}

// --- Tag push (post-freeze) ---
#[derive(Debug, Clone)]
pub enum TagPushMessage {
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
    /// RFC-035 R22: `↓`/`j` — card arrow-navigation, coarser than Tab
    /// (moves between dashboard rows only, skipping their internal
    /// controls). Gated on text-input focus in `handle_shortcut`,
    /// consistent with `FocusSearchBare`/`ActivateFocused` even though
    /// iced 0.14's `text_input` does not itself consume arrow keys or
    /// `j`/`k` — the gate is for the user's typing, not to resolve a
    /// widget-level key conflict (Handoff 032 §3/§4).
    CardFocusNext,
    /// RFC-035 R22: `↑`/`k`. See [`ShortcutMessage::CardFocusNext`].
    CardFocusPrevious,
}

// ---------------------------------------------------------------------------
// RFC-0009 — Selection messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum SelectionMessage {
    Toggled(ProjectId),
    SelectAll,
    Clear,
    /// Enter selection mode (show checkboxes).
    ModeEntered,
    /// Exit selection mode and clear selection.
    ModeExited,
}

// ---------------------------------------------------------------------------
// RFC-0011 — Activity strip messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum ActivityMessage {
    RetryRequested { source_operation_id: OperationId },
    DetailsRequested { operation_id: OperationId },
}

// ---------------------------------------------------------------------------
// RFC-0012 — Command palette messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum PaletteMessage {
    Opened,
    Closed,
    QueryChanged(String),
    EntryClicked(usize),
}

// ---------------------------------------------------------------------------
// RFC-0016 — Keyboard messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    CheatSheetToggled,
}

// ---------------------------------------------------------------------------
// RFC-0014 — Detail panel messages
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum DetailPanelMessage {
    Opened(ProjectId),
    Closed,
}
