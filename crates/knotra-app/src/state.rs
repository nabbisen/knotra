//! Application state — single source of truth.

pub mod changelog;
pub mod conflict_ops;
pub mod context;
pub mod dashboard;
pub mod detail_panel;
pub mod focus;
pub mod freezer;
pub mod palette;
pub mod sync;
pub mod topology;
pub mod workspace_mgr;

use std::collections::HashSet;

use knotra_ui::KnotraTheme;
use knotra_ui::i18n::Catalog;
use knotra_vcs::{
    OperationId, WorkspaceStatus,
    model::{
        operation::{OperationLog, ProjectOperationResult, RetryExclusionReason},
        project::ProjectId,
        workspace::Workspace,
    },
};

use crate::{
    config::{AppConfig, AppPaths},
    message::{FilterMessage, StatusFilter},
    view::dashboard::WidthMode,
};

// ---------------------------------------------------------------------------
// Window size
// ---------------------------------------------------------------------------

/// The window size `main.rs` configures at startup. A single named constant
/// so `main.rs`'s `window::Settings` and `AppState`'s initial `width_mode`
/// (no resize event fires before the first frame) can never silently drift
/// apart (RFC-035 R8/Handoff 029 §2.2).
pub const INITIAL_WINDOW_SIZE: iced::Size = iced::Size::new(1100.0, 720.0);

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    History,
    Settings,
}

// ---------------------------------------------------------------------------
// Filter state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub search_text: String,
    pub active_group: Option<String>,
    pub status_filters: Vec<StatusFilter>,
}

impl FilterState {
    pub fn is_active(&self) -> bool {
        !self.search_text.is_empty()
            || self.active_group.is_some()
            || !self.status_filters.is_empty()
    }
    pub fn has_status_filter(&self, sf: &StatusFilter) -> bool {
        self.status_filters.contains(sf)
    }
}

// ---------------------------------------------------------------------------
// Dialog states
// ---------------------------------------------------------------------------

/// Which step of the 2-step Add Project flow is active.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AddProjectStep {
    /// Step 1: choose a project folder.
    #[default]
    ChooseFolder,
    /// Step 2: give it a display name.
    NameProject,
}

#[derive(Debug, Clone, Default)]
pub struct AddProjectDialog {
    pub step: AddProjectStep,
    pub name: String,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfirmRemoveDialog {
    pub project_id: ProjectId,
    pub project_name: String,
}

/// A recently removed project that can be undone before the snackbar expires.
#[derive(Debug, Clone)]
pub struct UndoableRemoval {
    pub project: knotra_vcs::Project,
    /// Project status snapshot so re-adding restores display immediately.
    pub status_snapshot: Option<knotra_vcs::ProjectStatus>,
}

// ---------------------------------------------------------------------------
// Load phase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPhase {
    Startup,
    Refreshing,
    Ready,
}

// ---------------------------------------------------------------------------
// Settings edit buffer
// ---------------------------------------------------------------------------

/// Temporary edit buffer for the Settings screen.
/// Mirrors `AppConfig` but stores text fields as raw Strings for input widgets.
#[derive(Debug, Clone)]
pub struct SettingsEdit {
    pub refresh_interval_secs: String,
    pub max_concurrent_reads: String,
    pub external_editor: String,
    pub external_merge_tool: String,
    pub max_log_entries: String,
    pub fs_debounce_secs: String,
}

impl SettingsEdit {
    pub fn from_config(cfg: &AppConfig) -> Self {
        SettingsEdit {
            refresh_interval_secs: cfg.refresh_interval_secs.to_string(),
            max_concurrent_reads: cfg.max_concurrent_reads.to_string(),
            external_editor: cfg.external_editor.clone().unwrap_or_default(),
            external_merge_tool: cfg.external_merge_tool.clone().unwrap_or_default(),
            max_log_entries: cfg.max_log_entries.to_string(),
            fs_debounce_secs: cfg.fs_debounce_secs.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tag push pending state
// ---------------------------------------------------------------------------

/// Tracks a pending offer to push freeze tags to the remote after success.
#[derive(Debug, Clone)]
pub struct PendingTagPush {
    pub freeze_name: String,
    pub project_ids: Vec<knotra_vcs::ProjectId>,
    pub is_pushing: bool,
}

// ---------------------------------------------------------------------------
// RFC-0009 — Selection model
// ---------------------------------------------------------------------------

/// Which projects the user has selected (checkboxes).
/// Drives the selection bar and bulk-action modals.
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selected_ids: HashSet<knotra_vcs::ProjectId>,
    /// Last card clicked without Shift — for range-select anchor.
    pub anchor_id: Option<knotra_vcs::ProjectId>,
}

impl SelectionState {
    pub fn toggle(&mut self, id: knotra_vcs::ProjectId) {
        if self.selected_ids.contains(&id) {
            self.selected_ids.remove(&id);
            if self.selected_ids.is_empty() {
                self.anchor_id = None;
            } else if self.anchor_id.as_ref() == Some(&id) {
                self.anchor_id = self.selected_ids.iter().next().cloned();
            }
        } else {
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id);
        }
    }

    pub fn clear(&mut self) {
        self.selected_ids.clear();
        self.anchor_id = None;
    }
    pub fn contains(&self, id: &knotra_vcs::ProjectId) -> bool {
        self.selected_ids.contains(id)
    }

    /// Select all projects.
    pub fn select_all(&mut self, ids: &[knotra_vcs::ProjectId]) {
        for id in ids {
            self.selected_ids.insert(id.clone());
        }
        if self.anchor_id.is_none() {
            self.anchor_id = ids.first().cloned();
        }
    }

    pub fn retain_ids(&mut self, ids: &HashSet<knotra_vcs::ProjectId>) {
        self.selected_ids.retain(|id| ids.contains(id));
        if self
            .anchor_id
            .as_ref()
            .is_some_and(|anchor| !self.selected_ids.contains(anchor))
        {
            self.anchor_id = self.selected_ids.iter().next().cloned();
        }
        if self.selected_ids.is_empty() {
            self.anchor_id = None;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectionSummary {
    pub selected_count: usize,
    pub selected_ids: Vec<knotra_vcs::ProjectId>,
    pub visible_ids: Vec<knotra_vcs::ProjectId>,
    pub fetchable_ids: Vec<knotra_vcs::ProjectId>,
    pub has_upstream: bool,
}

// ---------------------------------------------------------------------------
// RFC-0011 — Activity strip
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub enum LatestOpState {
    #[default]
    Idle,
    Running {
        operation_id: OperationId,
        label: String,
        done: usize,
        total: usize,
    },
    Completed {
        log: OperationLog,
        retry: RetryAvailability,
    },
}

#[derive(Debug, Clone)]
pub enum ActivityRetryAction {
    FetchFailed {
        source_operation_id: OperationId,
        project_ids: Vec<ProjectId>,
    },
    ReviewSmartPull {
        source_operation_id: OperationId,
        project_ids: Vec<ProjectId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryUnavailableReason {
    NoEligibleTargets,
    ContextSwitch,
    Freeze,
    FreezeRollback,
    StatusRefresh,
}

impl RetryUnavailableReason {
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::NoEligibleTargets => "plain.activity.none_available",
            Self::ContextSwitch => "plain.activity.retry_context_again",
            Self::Freeze | Self::FreezeRollback => "plain.activity.retry_freeze_again",
            Self::StatusRefresh => "plain.activity.retry_refresh_again",
        }
    }
}

#[derive(Debug, Clone)]
pub enum RetryAvailability {
    Available(ActivityRetryAction),
    Unavailable(RetryUnavailableReason),
    NotApplicable,
}

#[derive(Debug, Clone)]
pub struct RetryExclusion {
    pub project_id: ProjectId,
    pub reason: RetryExclusionReason,
}

#[derive(Debug, Clone)]
pub struct FetchRetryRun {
    pub operation_id: OperationId,
    pub lease_id: OperationLeaseId,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub total: usize,
    pub completed: Vec<ProjectOperationResult>,
    pub exclusions: Vec<RetryExclusion>,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityStripState {
    pub latest: LatestOpState,
    /// Seconds since the last operation completed (for auto-fade).
    pub completed_secs: u32,
    pub fetch_retry: Option<FetchRetryRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationLeaseId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationOwner {
    SingleFetch,
    BulkFetch,
    SmartPullPreparation,
    SmartPullExecution,
    ContextSwitch,
    FreezeValidation,
    FreezeExecution,
    ConflictMutation,
    TagPush,
    ActivityFetchRetry,
    ActivitySmartPullPreparation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationLease {
    pub id: OperationLeaseId,
    pub owner: OperationOwner,
}

#[derive(Debug, Default)]
pub struct OperationInterlock {
    active: Option<OperationLease>,
    next_id: u64,
}

impl OperationInterlock {
    pub fn try_acquire(&mut self, owner: OperationOwner) -> Option<OperationLeaseId> {
        if self.active.is_some() {
            return None;
        }
        self.next_id = self.next_id.checked_add(1)?;
        let id = OperationLeaseId(self.next_id);
        self.active = Some(OperationLease { id, owner });
        Some(id)
    }

    pub fn release_if_matches(&mut self, id: OperationLeaseId) -> bool {
        if self.active.is_some_and(|lease| lease.id == id) {
            self.active = None;
            true
        } else {
            false
        }
    }

    pub fn is_busy(&self) -> bool {
        self.active.is_some()
    }
}

// ---------------------------------------------------------------------------
// RFC-0012 — Command palette
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteEntryKind {
    Project,
    Workspace,
    Action,
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub kind: PaletteEntryKind,
    pub label: String,
    /// Machine-readable id for dispatching (e.g. project id, action key).
    pub payload: String,
    /// i18n key for why this row cannot currently run.
    pub disabled_reason_key: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    pub open: bool,
    pub query: String,
    pub results: Vec<PaletteEntry>,
    /// Index of the highlighted result.
    pub highlighted: usize,
    /// Last disabled/no-op reason shown in the palette.
    pub notice_key: Option<&'static str>,
}

impl PaletteState {
    pub fn open_palette(&mut self) {
        self.open = true;
        self.query.clear();
        self.highlighted = 0;
        self.notice_key = None;
    }
    pub fn close(&mut self) {
        self.open = false;
        self.notice_key = None;
    }
}

// ---------------------------------------------------------------------------
// RFC-0013 — Active modal discriminant
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveModal {
    #[default]
    None,
    Pull,
    Tag,
    Switch,
    Resolve(knotra_vcs::ProjectId),
    Changelog,
}

/// Whether the keyboard cheat-sheet overlay is open.
#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    pub cheat_sheet_open: bool,
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

pub struct AppState {
    pub screen: Screen,
    pub config: AppConfig,
    /// Filesystem paths used for config, workspace, and history persistence.
    pub paths: AppPaths,
    pub catalog: Catalog,
    pub theme: KnotraTheme,
    pub workspace: Option<Workspace>,
    pub workspace_status: Option<WorkspaceStatus>,
    pub load_phase: LoadPhase,
    pub filter: FilterState,
    pub operation_logs: Vec<OperationLog>,
    /// RFC-047 D2: entries `load_recent_logs` could not read or parse,
    /// within the most-recent-`max_log_entries` window loaded at startup.
    /// Deliberately not folded into `operation_logs` — this is a startup
    /// fact about the history directory, not a property of the in-memory
    /// list, and five other sites read `operation_logs` (including
    /// `background/mod.rs`, which appends to it at runtime as operations
    /// complete) without needing to know or preserve this count.
    pub history_unreadable_count: usize,
    /// RFC-047 D3: the history directory itself could not be read, distinct
    /// from a directory that has simply never been created (genuinely "no
    /// history yet" — see `persistence::LoadedLogs`'s own doc comment).
    pub history_directory_unreadable: bool,
    pub history_search: String,
    pub status_bar: Option<String>,
    pub add_project_dialog: Option<AddProjectDialog>,
    pub confirm_remove_dialog: Option<ConfirmRemoveDialog>,
    pub fetching_projects: HashSet<ProjectId>,
    pub is_refreshing: bool,
    /// History: which log entry IDs are currently expanded.
    pub history_expanded: std::collections::HashSet<knotra_vcs::OperationId>,
    /// Settings: in-progress edit buffer (mirrors config until saved).
    pub settings_edit: SettingsEdit,
    /// Settings: last save result message.
    pub settings_save_msg: Option<String>,
    /// Sync Center state.
    pub sync: sync::SyncCenterState,
    /// Context Operations state.
    pub context_ops: context::ContextOpsState,
    /// Freezer state.
    pub freezer: freezer::FreezerState,
    /// Conflict resolution state.
    pub conflict_ops: conflict_ops::ConflictOpsState,
    /// Changelog aggregation state.
    pub changelog: changelog::ChangelogState,
    /// Dependency topology state.
    pub topology: topology::TopologyState,
    /// All loaded workspaces (active index = active_workspace_idx).
    pub all_workspaces: Vec<knotra_vcs::Workspace>,
    /// Index into `all_workspaces` for the currently active workspace.
    pub active_workspace_idx: usize,
    /// Workspace management dialog state.
    pub workspace_mgr: workspace_mgr::WorkspaceMgrState,
    /// Missing-path projects detected at last refresh.
    pub missing_projects: std::collections::HashSet<knotra_vcs::ProjectId>,
    /// Post-freeze: offer to push tags to remote.
    pub pending_tag_push: Option<PendingTagPush>,
    /// Global ownership guard for VCS launch paths.
    pub operation_interlock: OperationInterlock,
    /// File-system change poller (used by the FS-watch Subscription).
    pub fs_poller: knotra_vcs::FsPoller,
    // ------------------------------------------------------------------
    // RFC-0009 selection model
    // ------------------------------------------------------------------
    pub selection: SelectionState,
    // ------------------------------------------------------------------
    // RFC-0011 activity strip
    // ------------------------------------------------------------------
    pub activity: ActivityStripState,
    // ------------------------------------------------------------------
    // RFC-0012 command palette
    // ------------------------------------------------------------------
    pub palette: PaletteState,
    // RFC-0016 keyboard state
    // ------------------------------------------------------------------
    pub keyboard: KeyboardState,
    // ------------------------------------------------------------------
    // RFC-0014 project detail panel
    // ------------------------------------------------------------------
    pub detail_panel: detail_panel::DetailPanelState,
    // ------------------------------------------------------------------
    // RFC-0013 active modal
    // ------------------------------------------------------------------
    pub active_modal: ActiveModal,
    /// True = selection mode active (checkboxes visible, selection bar shown).
    /// Off by default — cards are clean until the user explicitly selects.
    pub selection_mode: bool,
    /// Whether to show technical command details in modal result views.
    /// Toggled by the "Show details" / "Hide details" button in result screens.
    pub show_op_details: bool,
    /// RFC-035 R8/Handoff 028 Ruling 6.1: whether the compact toolbar's chip
    /// overflow (`⋯`) is open. Ignored outside `WidthMode::Compact` — not
    /// cleared on resize (a stale `true` behind a hidden control is
    /// harmless, and clearing it would need a resize-triggered message,
    /// which is exactly what `width_mode` moving into `AppState` was about
    /// keyboard/render parity, not about growing further).
    pub dashboard_toolbar_overflow_open: bool,
    /// RFC-035 R8/Handoff 028 Ruling 6.1: whether the compact toolbar's
    /// grouping/sorting selector disclosure (`▾`) is open. Same
    /// ignored-outside-compact, not-cleared-on-resize shape as
    /// `dashboard_toolbar_overflow_open`.
    pub dashboard_toolbar_selectors_open: bool,
    /// A recently removed project eligible for undo (cleared by next action or timeout).
    pub recent_removal: Option<UndoableRemoval>,
    // ------------------------------------------------------------------
    // RFC-036 keyboard focus traversal
    // ------------------------------------------------------------------
    /// Current knotra-focus target for the dashboard/shell context.
    /// Presentation state only — not persisted, not in `AppConfig`.
    pub dashboard_focus: Option<focus::FocusTarget>,
    /// Current knotra-focus target while an overlay is open. Kept separate
    /// from `dashboard_focus` because R5 confines Tab/Shift-Tab to the
    /// overlay while it is open, and R7 needs `dashboard_focus` untouched
    /// underneath so focus can return to it when the overlay closes.
    pub overlay_focus: Option<focus::FocusTarget>,
    // ------------------------------------------------------------------
    // RFC-035 R8 responsive mode
    // ------------------------------------------------------------------
    /// The dashboard's width-derived presentation mode (RFC-035 R8).
    /// **Reversed from a `responsive`-computed, `AppState`-excluded value
    /// (RFC-035's original Internal Design §Responsive strategy) to a real
    /// state field, fed by `Message::WindowResized`** (Handoff 028's
    /// finding, Handoff 029): `focus_order` runs inside `update()`, where
    /// no `responsive` closure's `Size` is reachable, so keyboard order and
    /// rendering could not derive from the same value under the original
    /// mechanism. See `width_mode.rs`'s module doc for the full history.
    pub width_mode: WidthMode,
    /// The window's current width in logical pixels (RFC-051 D1) — kept
    /// alongside `width_mode` rather than only the derived band, so overlay
    /// sizing (`OverlayWidth::pixels`) can scale continuously with the
    /// window instead of only in `WidthMode`'s coarse steps. Set on the same
    /// line as `width_mode` in `Message::WindowResized`'s handler, and
    /// seeded here from the same `INITIAL_WINDOW_SIZE` — the two must never
    /// be set independently, which is what R2's pairing test checks.
    pub window_width: f32,
}

impl AppState {
    #[cfg(test)]
    pub fn new(config: AppConfig) -> Self {
        Self::new_with_paths(config, AppPaths::resolve().0)
    }

    pub fn new_with_paths(config: AppConfig, paths: AppPaths) -> Self {
        let locale = config.locale;
        let dark = config.dark_theme;
        AppState {
            screen: Screen::Dashboard,
            catalog: Catalog::for_locale(locale),
            theme: if dark {
                KnotraTheme::dark()
            } else {
                KnotraTheme::light()
            },
            workspace: None,
            workspace_status: None,
            load_phase: LoadPhase::Startup,
            filter: FilterState::default(),
            operation_logs: Vec::new(),
            history_unreadable_count: 0,
            history_directory_unreadable: false,
            history_search: String::new(),
            status_bar: None,
            add_project_dialog: None,
            confirm_remove_dialog: None,
            fetching_projects: HashSet::new(),
            is_refreshing: false,
            history_expanded: std::collections::HashSet::new(),
            settings_edit: SettingsEdit::from_config(&config),
            settings_save_msg: None,
            sync: sync::SyncCenterState::default(),
            context_ops: context::ContextOpsState::default(),
            freezer: freezer::FreezerState::default(),
            conflict_ops: conflict_ops::ConflictOpsState::default(),
            changelog: changelog::ChangelogState::default(),
            topology: topology::TopologyState::default(),
            all_workspaces: Vec::new(),
            active_workspace_idx: 0,
            workspace_mgr: workspace_mgr::WorkspaceMgrState::default(),
            missing_projects: std::collections::HashSet::new(),
            pending_tag_push: None,
            operation_interlock: OperationInterlock::default(),
            fs_poller: knotra_vcs::FsPoller::default(),
            selection: SelectionState::default(),
            activity: ActivityStripState::default(),
            palette: PaletteState::default(),
            keyboard: KeyboardState::default(),
            detail_panel: detail_panel::DetailPanelState::default(),
            active_modal: ActiveModal::default(),
            selection_mode: false,
            show_op_details: false,
            dashboard_toolbar_overflow_open: false,
            dashboard_toolbar_selectors_open: false,
            recent_removal: None,
            dashboard_focus: None,
            overlay_focus: None,
            width_mode: WidthMode::from_width(INITIAL_WINDOW_SIZE.width),
            window_width: INITIAL_WINDOW_SIZE.width,
            paths,
            config,
        }
    }

    pub fn t(&self, key: &'static str) -> &'static str {
        self.catalog.t(key)
    }

    pub fn apply_filter(&mut self, msg: FilterMessage) {
        match msg {
            FilterMessage::SearchChanged(s) => self.filter.search_text = s,
            FilterMessage::StatusFilterToggled(sf) => {
                if let Some(pos) = self.filter.status_filters.iter().position(|f| f == &sf) {
                    self.filter.status_filters.remove(pos);
                } else {
                    self.filter.status_filters.push(sf);
                }
            }
            FilterMessage::AllFiltersCleared => self.filter = FilterState::default(),
        }
    }

    pub fn visible_project_ids(&self) -> Vec<knotra_vcs::ProjectId> {
        self.dashboard_display().ordered_selectable_ids
    }

    pub fn dashboard_display(&self) -> dashboard::DashboardDisplay<'_> {
        let projects = self
            .workspace
            .as_ref()
            .map(|ws| ws.projects.as_slice())
            .unwrap_or(&[]);
        dashboard::build_dashboard_display(
            projects,
            self.workspace_status.as_ref(),
            &self.missing_projects,
            &self.filter,
            dashboard::DashboardDisplayOptions {
                grouping: self.config.dashboard_grouping,
                sort: self.config.dashboard_sort,
                in_progress_collapsed: self.config.dashboard_in_progress_collapsed,
                all_set_collapsed: self.config.dashboard_all_set_collapsed,
            },
        )
    }

    pub fn selection_summary(&self) -> SelectionSummary {
        let visible_ids = self.visible_project_ids();
        let selected_ids: Vec<_> = visible_ids
            .iter()
            .filter(|id| self.selection.selected_ids.contains(*id))
            .cloned()
            .collect();

        let fetchable_ids: Vec<_> = selected_ids
            .iter()
            .filter(|id| !self.missing_projects.contains(*id))
            .cloned()
            .collect();

        let has_upstream = self.workspace_status.as_ref().is_some_and(|status| {
            status.projects.iter().any(|project_status| {
                selected_ids.contains(&project_status.project_id)
                    && project_status.remote.upstream.is_some()
                    && !self.missing_projects.contains(&project_status.project_id)
            })
        });

        SelectionSummary {
            selected_count: selected_ids.len(),
            selected_ids,
            visible_ids,
            fetchable_ids,
            has_upstream,
        }
    }

    pub fn reconcile_selection_with_display(&mut self) {
        let visible_ids: HashSet<_> = self.visible_project_ids().into_iter().collect();
        self.selection.retain_ids(&visible_ids);
    }

    pub fn clear_selection_mode(&mut self) {
        self.selection.clear();
        self.selection_mode = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC-035 R8/Handoff 029 §2.2: the initial `width_mode` must be
    /// *derived* from `INITIAL_WINDOW_SIZE`, not a hardcoded default that
    /// happens to match today's configured size by accident of enum
    /// ordering — this would silently go wrong if `INITIAL_WINDOW_SIZE`
    /// (or `main.rs`'s `window::Settings`, which reads the same constant)
    /// ever changed without this staying in sync.
    #[test]
    fn initial_width_mode_is_derived_from_initial_window_size() {
        let state = AppState::new(AppConfig::default());
        assert_eq!(
            state.width_mode,
            WidthMode::from_width(INITIAL_WINDOW_SIZE.width)
        );
    }
}
