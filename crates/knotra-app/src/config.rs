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
    /// Resolves OS-standard config/data directories, degrading — never
    /// failing — to the current working directory when either cannot be
    /// determined (Handoff 033 Task B). The fallback used to be silent: a
    /// user whose environment could not resolve `config_dir()` had their
    /// settings written to `./knotra/config.toml` in whatever directory
    /// knotra happened to be launched from, with no indication anything
    /// unusual had happened. It still falls back — knotra's documented
    /// contract is defaults-plus-warning, never a failed start — but now
    /// says so, and where, mirroring [`crate::config::load_config`]'s own
    /// `(value, Option<String>)` shape rather than introducing a second one.
    pub fn resolve() -> (Self, Option<String>) {
        Self::resolve_from(dirs::config_dir(), dirs::data_local_dir())
    }

    /// The pure decision `resolve` makes, with `dirs::config_dir()`/
    /// `dirs::data_local_dir()`'s results passed in rather than read
    /// directly — a private seam so the warning-producing branch is
    /// testable without `dirs::` being injectable itself (Handoff 034 Item
    /// 2: `resolve` alone could exercise only the "both resolve" case on
    /// any real machine, leaving the actual deliverable of Task B —
    /// producing a correct warning — with no coverage at all).
    fn resolve_from(
        config_root: Option<PathBuf>,
        data_root: Option<PathBuf>,
    ) -> (Self, Option<String>) {
        let mut warnings = Vec::new();

        let config_root = config_root.unwrap_or_else(|| {
            let fallback = PathBuf::from(".");
            warnings.push(format!(
                "Could not determine the OS config directory; using \"{}\" \
                 (the current directory) instead. Settings will be written \
                 to a different place depending on where knotra is launched \
                 from.",
                fallback.display()
            ));
            fallback
        });
        let config_base = config_root.join("knotra");

        let data_root = data_root.unwrap_or_else(|| {
            let fallback = PathBuf::from(".");
            warnings.push(format!(
                "Could not determine the OS data directory; using \"{}\" \
                 (the current directory) instead for operation history.",
                fallback.display()
            ));
            fallback
        });
        let data_base = data_root.join("knotra");

        let paths = AppPaths {
            config_file: config_base.join("config.toml"),
            workspaces_dir: config_base.join("workspaces"),
            history_dir: data_base.join("history"),
        };

        // Joined with a bare newline — distinct from `app::init`'s "\n\n"
        // join of *its* two independent startup warnings (path resolution
        // and config parsing). Both are defensible; this is the one this
        // function commits to, pinned by the tests below rather than left
        // to whichever separator happened to be typed (Handoff 034 §2).
        let warning = (!warnings.is_empty()).then(|| warnings.join("\n"));
        (paths, warning)
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
///
/// A missing `config.toml` is "no config yet" only once the directory that
/// should contain it is confirmed genuinely absent (or `config_file` has no
/// parent to blame at all) — `NotFound` alone cannot carry that distinction
/// on every platform, for the same reason `persistence::delete_workspace_file`
/// cannot trust it either (Task 081/082): a config directory blocked by a
/// plain file can report the same `NotFound` kind reading `config.toml`
/// would report if the directory had simply never been created. A blocked
/// directory instead takes the "cannot read" path below, message and all.
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
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound
                && paths
                    .config_file
                    .parent()
                    .is_none_or(crate::persistence::treat_missing_directory_as_absent) =>
        {
            (AppConfig::default(), None)
        }
        Err(e) => {
            let msg = format!(
                "Cannot read config file (using defaults): {e}\nPath: {}",
                paths.config_file.display()
            );
            (AppConfig::default(), Some(msg))
        }
    }
}

/// Persist configuration to disk, atomically (Handoff 033 Task A) — this is
/// called on every dashboard Group/Sort change and section collapse, not
/// only an explicit Save, so a bare truncating write is a real exposure
/// rather than a theoretical one.
pub fn save_config(config: &AppConfig, paths: &AppPaths) -> Result<(), String> {
    if let Some(parent) = paths.config_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create config dir: {e}"))?;
    }
    let text = toml::to_string_pretty(config).map_err(|e| format!("serialization error: {e}"))?;
    crate::atomic_write::write(&paths.config_file, text)
        .map_err(|e| format!("cannot write config.toml: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression this whole task exists to prevent from becoming
    /// silent: on a normal development machine, `dirs::config_dir()` and
    /// `dirs::data_local_dir()` both resolve, so `resolve()` must produce no
    /// warning at all — the CI/dev environment this test runs in is exactly
    /// such a machine.
    #[test]
    fn resolve_produces_no_warning_on_a_normal_machine() {
        let (_paths, warning) = AppPaths::resolve();
        assert_eq!(warning, None);
    }

    #[test]
    fn resolve_from_both_present_produces_no_warning() {
        let (_paths, warning) =
            AppPaths::resolve_from(Some(PathBuf::from("/cfg")), Some(PathBuf::from("/data")));
        assert_eq!(warning, None);
    }

    /// Handoff 034 Item 2: the warning-producing branch — the entire
    /// deliverable of Task B — had no coverage before this test existed.
    #[test]
    fn resolve_from_missing_config_dir_names_the_config_directory() {
        let (paths, warning) = AppPaths::resolve_from(None, Some(PathBuf::from("/data")));
        let warning = warning.expect("a warning must be produced");
        assert!(
            warning.contains("config directory"),
            "warning must name which directory failed: {warning:?}"
        );
        assert!(!warning.contains("data directory"), "{warning:?}");
        assert_eq!(paths.config_file, PathBuf::from("./knotra/config.toml"));
    }

    #[test]
    fn resolve_from_missing_data_dir_names_the_data_directory() {
        let (paths, warning) = AppPaths::resolve_from(Some(PathBuf::from("/cfg")), None);
        let warning = warning.expect("a warning must be produced");
        assert!(
            warning.contains("data directory"),
            "warning must name which directory failed: {warning:?}"
        );
        assert!(!warning.contains("config directory"), "{warning:?}");
        assert_eq!(paths.history_dir, PathBuf::from("./knotra/history"));
    }

    /// Pins the join form this function commits to — a bare `"\n"` — since
    /// nothing else would catch it changing silently.
    #[test]
    fn resolve_from_both_missing_joins_both_messages_with_a_newline() {
        let (_paths, warning) = AppPaths::resolve_from(None, None);
        let warning = warning.expect("a warning must be produced");
        let lines: Vec<&str> = warning.split('\n').collect();
        assert_eq!(
            lines.len(),
            2,
            "both messages must be present, joined by a single newline: {warning:?}"
        );
        assert!(lines[0].contains("config directory"), "{warning:?}");
        assert!(lines[1].contains("data directory"), "{warning:?}");
    }

    // Task 083: no dedicated `load_config` tests existed before this task —
    // only `resolve`/`resolve_from` above, plus indirect round-trip coverage
    // in `tests.rs`. Both cases below are new, not duplicates of anything
    // pre-existing.

    /// The first-run case: the config directory was never created at all.
    /// `load_config` must still read this as "no config yet" — unchanged
    /// behaviour after Task 083's fix, not a new guarantee.
    #[test]
    fn load_config_with_no_config_directory_uses_defaults_and_no_warning() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = AppPaths::under(tmp.path().to_path_buf()); // config dir never created

        let (loaded, warning) = load_config(&paths);

        assert_eq!(warning, None);
        assert_eq!(
            loaded.refresh_interval_secs,
            AppConfig::default().refresh_interval_secs
        );
        assert_eq!(loaded.dark_theme, AppConfig::default().dark_theme);
    }

    /// Task 083: a config directory blocked by a plain file must not read
    /// as "no config yet" — the same ambiguity Tasks 081/082 removed from
    /// `delete_workspace_file` and the two loaders in `persistence.rs`.
    #[test]
    fn load_config_with_a_blocked_config_directory_reports_and_uses_defaults() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let blocked = tmp.path().join("config");
        std::fs::write(&blocked, "not a directory").expect("blocking file");
        let paths = AppPaths {
            config_file: blocked.join("config.toml"),
            workspaces_dir: blocked.join("workspaces"),
            history_dir: tmp.path().join("data").join("history"),
        };

        let (loaded, warning) = load_config(&paths);

        assert!(warning.is_some(), "a blocked directory must be reported");
        assert_eq!(
            loaded.refresh_interval_secs,
            AppConfig::default().refresh_interval_secs
        );
        assert_eq!(loaded.dark_theme, AppConfig::default().dark_theme);
    }
}
