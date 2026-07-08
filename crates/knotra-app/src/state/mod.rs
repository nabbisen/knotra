//! Application state — single source of truth.

pub mod dashboard;
pub mod sync;

use std::collections::HashSet;

use endringer::{
    model::{
        operation::OperationLog,
        project::ProjectId,
        workspace::Workspace,
    },
    WorkspaceStatus,
};
use snora::i18n::{Catalog, Locale};
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
}

impl Screen {
    pub fn nav_key(&self) -> &'static str {
        match self {
            Screen::Dashboard  => "nav.dashboard",
            Screen::SyncCenter => "nav.sync",
            Screen::ContextOps => "nav.context",
            Screen::Freezer    => "nav.freezer",
            Screen::History    => "nav.history",
            Screen::Settings   => "nav.settings",
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
    pub freezer_name: String,
    pub status_bar: Option<String>,
    pub add_project_dialog: Option<AddProjectDialog>,
    pub confirm_remove_dialog: Option<ConfirmRemoveDialog>,
    pub fetching_projects: HashSet<ProjectId>,
    pub is_refreshing: bool,
    /// Sync Center state.
    pub sync: sync::SyncCenterState,
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
            freezer_name: String::new(),
            status_bar: None,
            add_project_dialog: None,
            confirm_remove_dialog: None,
            fetching_projects: HashSet::new(),
            is_refreshing: false,
            sync: sync::SyncCenterState::default(),
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
