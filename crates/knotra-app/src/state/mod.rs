//! Application state — single source of truth.

pub mod changelog;
pub mod conflict_ops;
pub mod workspace_mgr;
pub mod context;
pub mod dashboard;
pub mod freezer;
pub mod sync;
pub mod topology;

use std::collections::HashSet;

use endringer::{
    model::{
        operation::OperationLog,
        project::ProjectId,
        workspace::Workspace,
    },
    WorkspaceStatus,
};
use snora::i18n::Catalog;
use snora::KnotraTheme;

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
    SyncCenter,
    ContextOps,
    Freezer,
    History,
    Settings,
    ConflictResolution,
    Changelog,
}

impl Screen {
    #[allow(dead_code)]
    pub fn nav_key(&self) -> &'static str {
        match self {
            Screen::Dashboard  => "nav.dashboard",
            Screen::SyncCenter => "nav.sync",
            Screen::ContextOps => "nav.context",
            Screen::Freezer    => "nav.freezer",
            Screen::History            => "nav.history",
            Screen::Settings           => "nav.settings",
            Screen::ConflictResolution => "nav.conflicts",
            Screen::Changelog          => "nav.changelog",
        }
    }
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
    pub project_ids: Vec<endringer::ProjectId>,
    pub is_pushing: bool,
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
    pub history_expanded: std::collections::HashSet<endringer::OperationId>,
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
    pub all_workspaces: Vec<endringer::Workspace>,
    /// Index into `all_workspaces` for the currently active workspace.
    pub active_workspace_idx: usize,
    /// Workspace management dialog state.
    pub workspace_mgr: workspace_mgr::WorkspaceMgrState,
    /// Missing-path projects detected at last refresh.
    pub missing_projects: std::collections::HashSet<endringer::ProjectId>,
    /// Post-freeze: offer to push tags to remote.
    pub pending_tag_push: Option<PendingTagPush>,
    /// File-system change poller (used by the FS-watch Subscription).
    pub fs_poller: endringer::FsPoller,
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
            fs_poller: endringer::FsPoller::default(),
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
