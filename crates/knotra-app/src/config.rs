//! Application configuration: loading, validation, and persistence.
//!
//! Config is stored as TOML in `~/.config/knotra/config.toml`.
//! Workspace definitions live in `~/.config/knotra/workspaces/`.
//! Operation history lives in `~/.local/share/knotra/history/`.

use knotra_ui::i18n::Locale;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DashboardGrouping {
    #[default]
    Attention,
    ProjectGroup,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DashboardSort {
    #[default]
    Recommended,
    NameAscending,
}

/// Application-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// UI display locale.
    pub locale: Locale,
    /// Use dark theme when true.
    pub dark_theme: bool,
    /// Background refresh interval in seconds (0 = manual only).
    pub refresh_interval_secs: u32,
    /// Maximum concurrent repository status reads.
    pub max_concurrent_reads: usize,
    /// Path to the preferred external editor.
    pub external_editor: Option<String>,
    /// Path to the preferred external merge tool.
    pub external_merge_tool: Option<String>,
    /// Maximum number of operation log entries to keep in memory.
    pub max_log_entries: usize,
    /// Enable file-system event monitoring (off by default).
    pub fs_watch_enabled: bool,
    /// Debounce interval in seconds before a FS event triggers a refresh.
    pub fs_debounce_secs: u32,
    /// Dashboard section grouping.
    pub dashboard_grouping: DashboardGrouping,
    /// Dashboard project ordering.
    pub dashboard_sort: DashboardSort,
    /// Whether the In progress attention section is collapsed.
    pub dashboard_in_progress_collapsed: bool,
    /// Whether the All set attention section is collapsed.
    pub dashboard_all_set_collapsed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            locale: Locale::En,
            dark_theme: true,
            refresh_interval_secs: 60,
            max_concurrent_reads: 8,
            external_editor: None,
            external_merge_tool: None,
            max_log_entries: 200,
            fs_watch_enabled: false,
            fs_debounce_secs: 2,
            dashboard_grouping: DashboardGrouping::default(),
            dashboard_sort: DashboardSort::default(),
            dashboard_in_progress_collapsed: false,
            dashboard_all_set_collapsed: true,
        }
    }
}

/// Paths used by the application.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub workspaces_dir: PathBuf,
    pub history_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        let config_base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("knotra");
        let data_base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("knotra");

        AppPaths {
            config_file: config_base.join("config.toml"),
            workspaces_dir: config_base.join("workspaces"),
            history_dir: data_base.join("history"),
        }
    }

    #[cfg(test)]
    pub fn under(base: PathBuf) -> Self {
        AppPaths {
            config_file: base.join("config").join("config.toml"),
            workspaces_dir: base.join("config").join("workspaces"),
            history_dir: base.join("data").join("history"),
        }
    }
}

/// Load configuration from disk, falling back to defaults on any error.
pub fn load_config(paths: &AppPaths) -> (AppConfig, Option<String>) {
    match std::fs::read_to_string(&paths.config_file) {
        Ok(text) => match toml::from_str::<AppConfig>(&text) {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                let msg = format!(
                    "config.toml parse error (using defaults): {e}\nPath: {}",
                    paths.config_file.display()
                );
                (AppConfig::default(), Some(msg))
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (AppConfig::default(), None),
        Err(e) => {
            let msg = format!(
                "Cannot read config file (using defaults): {e}\nPath: {}",
                paths.config_file.display()
            );
            (AppConfig::default(), Some(msg))
        }
    }
}

/// Persist configuration to disk.
pub fn save_config(config: &AppConfig, paths: &AppPaths) -> Result<(), String> {
    if let Some(parent) = paths.config_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create config dir: {e}"))?;
    }
    let text = toml::to_string_pretty(config).map_err(|e| format!("serialization error: {e}"))?;
    std::fs::write(&paths.config_file, text).map_err(|e| format!("cannot write config.toml: {e}"))
}
