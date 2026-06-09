//! Application state — the single source of truth for the knotra GUI.

pub mod dashboard;

use endringer::{
    WorkspaceStatus,
    model::{
        operation::OperationLog,
        project::ProjectId,
        workspace::{Workspace, WorkspaceId},
    },
};

use snora::KnotraTheme;
use snora::i18n::{Catalog, Locale};

use crate::{
    config::AppConfig,
    message::{FilterMessage, StatusFilter},
};

/// The active screen shown in the main content area.
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
            Screen::Dashboard => "nav.dashboard",
            Screen::SyncCenter => "nav.sync",
            Screen::ContextOps => "nav.context",
            Screen::Freezer => "nav.freezer",
            Screen::History => "nav.history",
            Screen::Settings => "nav.settings",
        }
    }
}

/// Filter / search state shared by the dashboard toolbar.
#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub search_text: String,
    pub active_group: Option<String>,
    pub status_filters: Vec<StatusFilter>,
}

/// Loading phase for the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPhase {
    /// Application just started; nothing loaded yet.
    Startup,
    /// Status refresh in progress.
    Refreshing,
    /// Status is available (may be stale).
    Ready,
    /// A refresh failed entirely.
    Error(String),
}

/// Top-level application state.
pub struct AppState {
    /// Active screen.
    pub screen: Screen,
    /// User configuration.
    pub config: AppConfig,
    /// i18n catalog for the active locale.
    pub catalog: Catalog,
    /// Current theme.
    pub theme: KnotraTheme,
    /// The active workspace definition.
    pub workspace: Option<Workspace>,
    /// Latest known status snapshot for the active workspace.
    pub workspace_status: Option<WorkspaceStatus>,
    /// Loading / refresh phase.
    pub load_phase: LoadPhase,
    /// Dashboard filter state.
    pub filter: FilterState,
    /// Completed operation logs (most recent first).
    pub operation_logs: Vec<OperationLog>,
    /// History search text.
    pub history_search: String,
    /// Freezer: user-entered freeze-point name.
    pub freezer_name: String,
    /// Notification / status bar text.
    pub status_bar: Option<String>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
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
            history_search: String::new(),
            freezer_name: String::new(),
            status_bar: None,
            config,
        }
    }

    /// Translate a key using the active locale.
    pub fn t(&self, key: &'static str) -> &'static str {
        self.catalog.t(key)
    }

    /// Apply a filter message and return whether a re-render is needed.
    pub fn apply_filter(&mut self, msg: FilterMessage) {
        match msg {
            FilterMessage::SearchChanged(s) => self.filter.search_text = s,
            FilterMessage::GroupChanged(g) => self.filter.active_group = g,
            FilterMessage::StatusFilterToggled(sf) => {
                if let Some(pos) = self.filter.status_filters.iter().position(|f| f == &sf) {
                    self.filter.status_filters.remove(pos);
                } else {
                    self.filter.status_filters.push(sf);
                }
            }
        }
    }
}
