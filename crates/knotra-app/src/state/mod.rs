//! Application state — single source of truth.

pub mod changelog;
pub mod conflict_ops;
pub mod workspace_mgr;
pub mod context;
pub mod dashboard;
pub mod freezer;
pub mod sync;
pub mod topology;
pub mod tier;
pub mod palette;

use std::collections::HashSet;

use knotra_vcs::{
    model::{
        operation::OperationLog,
        project::ProjectId,
        workspace::Workspace,
    },
    WorkspaceStatus,
};
use knotra_ui::i18n::Catalog;
use knotra_ui::KnotraTheme;

use crate::{
    config::AppConfig,
    message::{FilterMessage, StatusFilter},
};

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

#[derive(Debug, Clone, Default)]
pub struct AddProjectDialog {
    pub name: String,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfirmRemoveDialog {
    pub project_id: ProjectId,
    pub project_name: String,
}

// ---------------------------------------------------------------------------
// Load phase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPhase {
    Startup,
    Refreshing,
    Ready,
    Error(String),
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
            max_concurrent_reads:  cfg.max_concurrent_reads.to_string(),
            external_editor:       cfg.external_editor.clone().unwrap_or_default(),
            external_merge_tool:   cfg.external_merge_tool.clone().unwrap_or_default(),
            max_log_entries:       cfg.max_log_entries.to_string(),
            fs_debounce_secs:      cfg.fs_debounce_secs.to_string(),
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
    pub selected_ids:  HashSet<knotra_vcs::ProjectId>,
    /// Last card clicked without Shift — for range-select anchor.
    pub anchor_id:     Option<knotra_vcs::ProjectId>,
}

impl SelectionState {
    pub fn toggle(&mut self, id: knotra_vcs::ProjectId) {
        if self.selected_ids.contains(&id) {
            self.selected_ids.remove(&id);
        } else {
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id);
        }
    }

    pub fn select_range(
        &mut self,
        ordered: &[knotra_vcs::ProjectId],
        to: &knotra_vcs::ProjectId,
    ) {
        if let Some(anchor) = &self.anchor_id.clone() {
            let ai = ordered.iter().position(|x| x == anchor).unwrap_or(0);
            let bi = ordered.iter().position(|x| x == to).unwrap_or(0);
            let (lo, hi) = if ai <= bi { (ai, bi) } else { (bi, ai) };
            for id in &ordered[lo..=hi] {
                self.selected_ids.insert(id.clone());
            }
        } else {
            self.selected_ids.insert(to.clone());
            self.anchor_id = Some(to.clone());
        }
    }

    pub fn clear(&mut self) { self.selected_ids.clear(); self.anchor_id = None; }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool { self.selected_ids.is_empty() }
    pub fn len(&self) -> usize { self.selected_ids.len() }
    pub fn contains(&self, id: &knotra_vcs::ProjectId) -> bool { self.selected_ids.contains(id) }

    /// Select all projects.
    pub fn select_all(&mut self, ids: &[knotra_vcs::ProjectId]) {
        for id in ids { self.selected_ids.insert(id.clone()); }
    }
}

// ---------------------------------------------------------------------------
// RFC-0011 — Activity strip
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[derive(Default)]
pub enum LatestOpState {
    #[default]
    Idle,
    Running { label: String, done: usize, total: usize },
    Success { summary: String, #[allow(dead_code)] elapsed_secs: u32 },
    PartialFailure { summary: String, failed_names: Vec<String> },
    TotalFailure { summary: String },
}


#[derive(Debug, Clone, Default)]
pub struct ActivityStripState {
    pub latest:          LatestOpState,
    /// When true, the full-history popover is shown.
    pub popover_open:    bool,
    /// Seconds since the last operation completed (for auto-fade).
    pub completed_secs:  u32,
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
    pub kind:    PaletteEntryKind,
    pub label:   String,
    /// Machine-readable id for dispatching (e.g. project id, action key).
    pub payload: String,
}

#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    pub open:            bool,
    pub query:           String,
    pub results:         Vec<PaletteEntry>,
    /// Index of the highlighted result.
    pub highlighted:     usize,
}

impl PaletteState {
    pub fn open_palette(&mut self) { self.open = true; self.query.clear(); self.highlighted = 0; }
    pub fn close(&mut self) { self.open = false; }
}

// ---------------------------------------------------------------------------
// RFC-0010 — Attention tiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
#[allow(dead_code)]
pub enum AttentionTier {
    NeedsAttention,
    Active,
    Clean,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GroupingMode {
    #[default]
    Auto,       // RFC-0010 tier grouping
    Legacy,     // Original filter-chip grouping
}

/// Whether each tier header is collapsed in the UI.
#[derive(Debug, Clone)]
pub struct TierCollapseState {
    pub needs_attention: bool,
    pub active:          bool,
    pub clean:           bool,  // defaults to collapsed
}

impl Default for TierCollapseState {
    fn default() -> Self {
        TierCollapseState {
            needs_attention: false,
            active:          false,
            clean:           true,
        }
    }
}

// ---------------------------------------------------------------------------
// RFC-0014 — Project detail panel state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DetailPanelState {
    pub open_project_id: Option<knotra_vcs::ProjectId>,
}

// ---------------------------------------------------------------------------
// RFC-0013 — Active modal discriminant
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
#[derive(Default)]
pub enum ActiveModal {
    #[default]
    None,
    Pull,
    Tag,
    Switch,
    Resolve(knotra_vcs::ProjectId),
    Changelog,
}

// ---------------------------------------------------------------------------
// RFC-0016 — Keyboard leader-key state
// ---------------------------------------------------------------------------

/// Pending first key of a two-key sequence (e.g. `g` before `h` / `s`).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LeaderKeyState {
    #[default]
    None,
    /// `g` was pressed; waiting for the second key.
    G,
}

/// Whether the keyboard cheat-sheet overlay is open.
#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    pub cheat_sheet_open: bool,
    pub leader:           LeaderKeyState,
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

pub struct AppState {
    pub screen: Screen,
    pub config: AppConfig,
    pub catalog: Catalog,
    pub theme: KnotraTheme,
    pub workspace: Option<Workspace>,
    pub workspace_status: Option<WorkspaceStatus>,
    pub load_phase: LoadPhase,
    pub filter: FilterState,
    pub operation_logs: Vec<OperationLog>,
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
    // ------------------------------------------------------------------
    // RFC-0010 attention tiers
    // ------------------------------------------------------------------
    pub grouping_mode: GroupingMode,
    pub tier_collapse: TierCollapseState,
    // ------------------------------------------------------------------
    // RFC-0016 keyboard state
    // ------------------------------------------------------------------
    pub keyboard: KeyboardState,
    // ------------------------------------------------------------------
    // RFC-0014 project detail panel
    // ------------------------------------------------------------------
    pub detail_panel: DetailPanelState,
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
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let locale = config.locale;
        let dark = config.dark_theme;
        AppState {
            screen: Screen::Dashboard,
            catalog: Catalog::for_locale(locale),
            theme: if dark { KnotraTheme::dark() } else { KnotraTheme::light() },
            workspace: None,
            workspace_status: None,
            load_phase: LoadPhase::Startup,
            filter: FilterState::default(),
            operation_logs: Vec::new(),
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
            fs_poller: knotra_vcs::FsPoller::default(),
            selection: SelectionState::default(),
            activity:  ActivityStripState::default(),
            palette:   PaletteState::default(),
            grouping_mode: GroupingMode::default(),
            tier_collapse: TierCollapseState::default(),
            keyboard:  KeyboardState::default(),
            detail_panel: DetailPanelState::default(),
            active_modal: ActiveModal::default(),
            selection_mode: false,
            show_op_details: false,
            config,
        }
    }

    pub fn t(&self, key: &'static str) -> &'static str {
        self.catalog.t(key)
    }

    pub fn apply_filter(&mut self, msg: FilterMessage) {
        match msg {
            FilterMessage::SearchChanged(s)            => self.filter.search_text = s,
            FilterMessage::GroupChanged(g)             => self.filter.active_group = g,
            FilterMessage::StatusFilterToggled(sf)     => {
                if let Some(pos) = self.filter.status_filters.iter().position(|f| f == &sf) {
                    self.filter.status_filters.remove(pos);
                } else {
                    self.filter.status_filters.push(sf);
                }
            }
            FilterMessage::AllFiltersCleared => self.filter = FilterState::default(),
        }
    }

    #[allow(dead_code)]
    pub fn all_groups(&self) -> Vec<String> {
        self.workspace.as_ref()
            .map(|ws| {
                ws.projects.iter()
                    .filter_map(|p| p.group.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    }
}
