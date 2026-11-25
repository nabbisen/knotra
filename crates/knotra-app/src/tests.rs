//! Integration-level tests for knotra-app.

use crate::config::{AppConfig, AppPaths, DashboardGrouping, DashboardSort, load_config};
use crate::message::{
    ActivityMessage, BackgroundMessage, ChangelogMessage, ConflictOpsMessage, ContextMessage,
    DashboardMessage, DetailPanelMessage, FilterMessage, FreezerMessage, Message, PaletteMessage,
    SelectionMessage, SettingsMessage, ShortcutMessage, StatusFilter, SyncMessage, TagPushMessage,
    WorkspaceMessage,
};
use crate::persistence::{delete_workspace_file, load_workspaces, save_workspace};
use crate::state::{
    ActiveModal, ActivityRetryAction, AddProjectDialog, AddProjectStep, AppState, LatestOpState,
    OperationOwner, RetryAvailability, RetryExclusion, RetryUnavailableReason, Screen,
    changelog::ChangelogPhase,
    conflict_ops::ConflictPhase,
    context::ContextPhase,
    freezer::FreezerPhase,
    sync::{RetryPreparationId, SmartPullRetryPreparation, SyncPhase},
};
use jiff::Timestamp;
use knotra_vcs::{
    ChangelogDraft, CommitEntry, ConflictMarker, ConflictedFile, ContextTarget, OperationId,
    Project, ProjectCommits, ProjectConflictDetail, ProjectId, Workspace, WorkspaceStatus,
    model::{
        operation::{
            FreezeOutcome, FreezeProjectResult, FreezeResult, FreezeValidation,
            FreezeValidationEntry, OperationKind, OperationLog, OperationResult,
            ProjectOperationOutcome, ProjectOperationResult, RetryExclusionReason,
            SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry, SmartPullProgress,
            SmartPullSkipReason,
        },
        status::{ConflictStatus, RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus},
    },
};

fn make_state() -> AppState {
    AppState::new(AppConfig::default())
}

fn make_state_with_paths(paths: AppPaths) -> AppState {
    AppState::new_with_paths(AppConfig::default(), paths)
}

fn blocked_workspace_paths(tmp: &tempfile::TempDir) -> AppPaths {
    let workspaces_dir = tmp.path().join("not-a-directory");
    std::fs::write(&workspaces_dir, "file").expect("create blocking file");
    AppPaths {
        config_file: tmp.path().join("config.toml"),
        workspaces_dir,
        history_dir: tmp.path().join("history"),
    }
}

fn install_workspaces(state: &mut AppState, workspaces: Vec<Workspace>, active_idx: usize) {
    state.all_workspaces = workspaces;
    state.active_workspace_idx = active_idx;
    state.workspace = state.all_workspaces.get(active_idx).cloned();
}

fn make_project(name: &str) -> Project {
    Project::new(name, "/tmp")
}

fn make_project_at(name: &str, path: impl Into<String>) -> Project {
    Project::new(name, path)
}

fn make_operation_result(
    project_id: ProjectId,
    outcome: ProjectOperationOutcome,
) -> ProjectOperationResult {
    let success = outcome != ProjectOperationOutcome::Failed;
    ProjectOperationResult {
        project_id,
        // Task 103: this generic fixture is shared by tests that are not
        // about project naming at all — `None` (not an empty string)
        // honestly says "this synthetic result never recorded one", the
        // same shape a genuinely pre-Task-103 log carries.
        project_name: None,
        outcome,
        success,
        skip_reason: None,
        commands_executed: Vec::new(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: Some(if success { 0 } else { 1 }),
        error_message: (!success).then(|| "failed".to_owned()),
    }
}

fn make_operation_log(kind: OperationKind, results: Vec<ProjectOperationResult>) -> OperationLog {
    let now = Timestamp::now();
    OperationLog {
        result: OperationResult {
            operation_id: OperationId::new(),
            kind,
            started_at: now,
            finished_at: now,
            per_project: results,
            rollback_attempted: false,
            rollback_succeeded: None,
        },
        recovery_hints: Vec::new(),
    }
}

fn make_project_status(project_id: ProjectId, upstream: Option<&str>) -> knotra_vcs::ProjectStatus {
    knotra_vcs::ProjectStatus {
        project_id,
        identity: RepositoryIdentity {
            path: "/tmp".into(),
            vcs_kind: VcsKind::Git,
        },
        context: None,
        remote: RemoteStatus {
            upstream: upstream.map(str::to_owned),
            ..RemoteStatus::default()
        },
        working_tree: WorkingTreeStatus::default(),
        conflict: ConflictStatus::default(),
        refreshed_at: Timestamp::now(),
        read_error: None,
    }
}

fn make_project_status_with_kind(
    project_id: ProjectId,
    vcs_kind: VcsKind,
) -> knotra_vcs::ProjectStatus {
    knotra_vcs::ProjectStatus {
        project_id,
        identity: RepositoryIdentity {
            path: "/tmp".into(),
            vcs_kind,
        },
        context: None,
        remote: RemoteStatus::default(),
        working_tree: WorkingTreeStatus::default(),
        conflict: ConflictStatus::default(),
        refreshed_at: Timestamp::now(),
        read_error: None,
    }
}

fn make_changelog_draft(project: &Project, entries: Vec<CommitEntry>) -> ChangelogDraft {
    ChangelogDraft {
        release_name: "v1.2.0".to_owned(),
        generated_at: Timestamp::now(),
        projects: vec![ProjectCommits {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            since_ref: "v1.1.0".to_owned(),
            entries,
            error: None,
        }],
    }
}

fn make_commit(hash: &str, subject: &str) -> CommitEntry {
    CommitEntry {
        hash: hash.to_owned(),
        subject: subject.to_owned(),
        author: "Maintainer".to_owned(),
        date: Timestamp::now(),
    }
}

fn ready_freeze_validation(project: &Project, freeze_name: &str) -> FreezeValidation {
    FreezeValidation {
        freeze_name: freeze_name.to_owned(),
        entries: vec![FreezeValidationEntry {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            notes: Vec::new(),
            blockers: Vec::new(),
        }],
    }
}

fn install_pending_push(state: &mut AppState, freeze_name: &str, project_id: ProjectId) {
    state.pending_tag_push = Some(crate::state::PendingTagPush {
        freeze_name: freeze_name.to_owned(),
        project_ids: vec![project_id],
        is_pushing: false,
    });
}

fn dispatch(state: &mut AppState, message: Message) {
    let _ = crate::app::update(state, message);
}

#[test]
fn initial_screen_is_dashboard() {
    let state = make_state();
    assert_eq!(state.screen, Screen::Dashboard);
}

#[test]
fn filter_search_updates_state() {
    let mut state = make_state();
    state.apply_filter(FilterMessage::SearchChanged("api".to_owned()));
    assert_eq!(state.filter.search_text, "api");
}

#[test]
fn filter_toggle_adds_and_removes() {
    use crate::message::StatusFilter;
    let mut state = make_state();

    state.apply_filter(FilterMessage::StatusFilterToggled(StatusFilter::Behind));
    assert_eq!(state.filter.status_filters.len(), 1);

    state.apply_filter(FilterMessage::StatusFilterToggled(StatusFilter::Behind));
    assert_eq!(state.filter.status_filters.len(), 0);
}

#[test]
fn default_config_has_sensible_values() {
    let cfg = AppConfig::default();
    assert!(cfg.max_concurrent_reads > 0);
    assert!(cfg.max_log_entries > 0);
}

#[test]
fn legacy_config_uses_dashboard_serde_defaults() {
    let config: AppConfig = toml::from_str("").expect("empty legacy config");
    assert_eq!(config.dashboard_grouping, DashboardGrouping::Attention);
    assert_eq!(config.dashboard_sort, DashboardSort::Recommended);
    assert!(!config.dashboard_in_progress_collapsed);
    assert!(config.dashboard_all_set_collapsed);
}

#[test]
fn dashboard_preferences_persist_and_round_trip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());

    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::GroupingChanged(
            DashboardGrouping::ProjectGroup,
        )),
    );
    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::SortChanged(DashboardSort::NameAscending)),
    );
    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::GroupingChanged(
            DashboardGrouping::Attention,
        )),
    );
    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::TierToggled(
            crate::state::dashboard::DashboardTier::InProgress,
        )),
    );
    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::TierToggled(
            crate::state::dashboard::DashboardTier::AllSet,
        )),
    );

    let (loaded, error) = load_config(&paths);
    assert!(error.is_none(), "{error:?}");
    assert_eq!(loaded.dashboard_grouping, DashboardGrouping::Attention);
    assert_eq!(loaded.dashboard_sort, DashboardSort::NameAscending);
    assert!(loaded.dashboard_in_progress_collapsed);
    assert!(!loaded.dashboard_all_set_collapsed);
}

#[test]
fn dashboard_preference_save_failure_keeps_session_choice_and_warns() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let blocked_parent = tmp.path().join("blocked");
    std::fs::write(&blocked_parent, "not a directory").expect("blocking file");
    let paths = AppPaths {
        config_file: blocked_parent.join("config.toml"),
        workspaces_dir: tmp.path().join("workspaces"),
        history_dir: tmp.path().join("history"),
    };
    let mut state = make_state_with_paths(paths);

    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::GroupingChanged(DashboardGrouping::None)),
    );

    assert_eq!(state.config.dashboard_grouping, DashboardGrouping::None);
    // RFC-071 Amendment A7: `persist_config` is now the one function every
    // setting-changing path saves through, including this dashboard one —
    // its failure message is `settings.save_error`, not the removed
    // `dashboard.preference_save_failed`, and it shows in both places
    // `persist_config` writes on failure.
    let expected = format!("{} ", state.t("settings.save_error"));
    assert!(
        state
            .status_bar
            .as_deref()
            .is_some_and(|s| s.starts_with(&expected)),
        "{:?}",
        state.status_bar
    );
    assert_eq!(state.status_bar, state.settings_save_msg);
}

// ---------------------------------------------------------------------------
// RFC-071 Amendment A7: every setting is saved when it changes.
// ---------------------------------------------------------------------------

/// A7 item 1: a select (theme), the file-monitoring switch, and a valid
/// typed field all reach disk with no "Save settings" step — there is no
/// such message any more, only the change itself.
#[test]
fn every_kind_of_setting_persists_without_a_save_step() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());

    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::ThemeChanged(
            knotra_ui::ThemeChoice::HighContrastDark,
        )),
    );
    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::ZoomChanged(130)),
    );
    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::FsWatchEnabledChanged(true)),
    );
    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::RefreshAutoChanged(900)),
    );
    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::ExternalEditorChanged(
            "/usr/bin/nvim".to_owned(),
        )),
    );

    assert_eq!(state.settings_save_msg, None, "no failure occurred");

    let (loaded, warning) = load_config(&paths);
    assert_eq!(warning, None, "{warning:?}");
    assert_eq!(loaded.theme, knotra_ui::ThemeChoice::HighContrastDark);
    assert_eq!(loaded.zoom_percent, 130);
    assert!(loaded.fs_watch_enabled);
    assert_eq!(loaded.refresh_interval_secs, 900);
    assert_eq!(loaded.external_editor.as_deref(), Some("/usr/bin/nvim"));
}

/// A7's refinement (Review 259 §1): a failed save must show in the
/// activity line *and* in Settings, for a change made from Settings
/// itself — the case the un-amended handoff already covered.
#[test]
fn a_failed_save_from_settings_shows_in_both_places() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let blocked_parent = tmp.path().join("blocked");
    std::fs::write(&blocked_parent, "not a directory").expect("blocking file");
    let paths = AppPaths {
        config_file: blocked_parent.join("config.toml"),
        workspaces_dir: tmp.path().join("workspaces"),
        history_dir: tmp.path().join("history"),
    };
    let mut state = make_state_with_paths(paths);

    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::ThemeChanged(knotra_ui::ThemeChoice::Dark)),
    );

    // The change still takes effect live — only the save failed.
    assert_eq!(state.config.theme, knotra_ui::ThemeChoice::Dark);
    let expected_prefix = format!("{} ", state.t("settings.save_error"));
    assert!(
        state
            .status_bar
            .as_deref()
            .unwrap()
            .starts_with(&expected_prefix)
    );
    assert_eq!(state.status_bar, state.settings_save_msg);
}

/// A7's refinement (Review 259 §1): zoom's shortcuts work *outside*
/// Settings, so a failed save from one must still reach the activity line
/// — not only `settings_save_msg`, which nothing would be on screen to
/// read at the moment a dashboard-bound `Ctrl+Plus` fires.
#[test]
fn a_failed_save_from_a_zoom_shortcut_shows_in_the_activity_line() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let blocked_parent = tmp.path().join("blocked");
    std::fs::write(&blocked_parent, "not a directory").expect("blocking file");
    let paths = AppPaths {
        config_file: blocked_parent.join("config.toml"),
        workspaces_dir: tmp.path().join("workspaces"),
        history_dir: tmp.path().join("history"),
    };
    let mut state = make_state_with_paths(paths);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::ZoomIn));

    // The zoom change still takes effect live — only the save failed.
    assert_eq!(state.config.zoom_percent, 115);
    let expected_prefix = format!("{} ", state.t("settings.save_error"));
    assert!(
        state
            .status_bar
            .as_deref()
            .unwrap()
            .starts_with(&expected_prefix)
    );
    assert_eq!(state.status_bar, state.settings_save_msg);
}

#[test]
fn attention_collapse_prunes_hidden_selection_but_needs_help_cannot_collapse() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = make_state_with_paths(AppPaths::under(tmp.path().to_path_buf()));
    let project = make_project("work");
    let mut workspace = Workspace::new("Main");
    workspace.projects.push(project.clone());
    install_workspaces(&mut state, vec![workspace], 0);
    let mut project_status = make_project_status(project.id.clone(), Some("origin/main"));
    project_status.working_tree.uncommitted_count = 1;
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![project_status],
        last_refresh: None,
    });
    state.selection_mode = true;
    state.selection.toggle(project.id.clone());

    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::TierToggled(
            crate::state::dashboard::DashboardTier::InProgress,
        )),
    );
    assert!(state.selection.selected_ids.is_empty());
    assert!(state.selection_mode);

    dispatch(
        &mut state,
        Message::Dashboard(DashboardMessage::TierToggled(
            crate::state::dashboard::DashboardTier::NeedsHelp,
        )),
    );
    assert!(state.config.dashboard_in_progress_collapsed);
    assert!(state.config.dashboard_all_set_collapsed);
}

#[test]
fn filter_changes_prune_selection_and_summary_defensively_intersects_visibility() {
    let mut state = make_state();
    state.config.dashboard_all_set_collapsed = false;
    let project = make_project("api");
    let mut workspace = Workspace::new("Main");
    workspace.projects.push(project.clone());
    install_workspaces(&mut state, vec![workspace], 0);
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![make_project_status(project.id.clone(), None)],
        last_refresh: None,
    });
    state.selection.toggle(project.id.clone());

    dispatch(
        &mut state,
        Message::Filter(FilterMessage::StatusFilterToggled(StatusFilter::NeedsHelp)),
    );
    assert!(state.selection.selected_ids.is_empty());

    state.selection.selected_ids.insert(project.id.clone());
    let summary = state.selection_summary();
    assert_eq!(summary.selected_count, 0);
    assert!(summary.selected_ids.is_empty());
}

#[test]
fn create_workspace_confirm_persists_and_switches() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(
            "Lab".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 2);
    assert_eq!(state.active_workspace_idx, 1);
    assert!(state.is_refreshing);
    assert_eq!(
        state.workspace.as_ref().map(|ws| ws.name.as_str()),
        Some("Lab")
    );
    assert!(state.workspace_mgr.create_dialog.is_none());

    let (loaded, errors) = load_workspaces(&paths);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "Lab");
}

#[test]
fn create_workspace_save_failure_keeps_dialog_and_state() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = blocked_workspace_paths(&tmp);
    let mut state = make_state_with_paths(paths);
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(
            "Lab".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 1);
    assert_eq!(state.active_workspace_idx, 0);
    assert!(!state.is_refreshing);
    assert!(
        state
            .workspace_mgr
            .create_dialog
            .as_ref()
            .and_then(|d| d.error.as_deref())
            .is_some_and(|error| error.starts_with("We could not save this workspace."))
    );
}

#[test]
fn create_workspace_rejects_duplicate_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths);
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(
            " main ".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 1);
    assert_eq!(
        state
            .workspace_mgr
            .create_dialog
            .as_ref()
            .and_then(|d| d.error.as_deref()),
        Some("That workspace already exists.")
    );
}

#[test]
fn rename_workspace_confirm_persists_and_updates_active_workspace() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());
    let workspace = Workspace::new("Main");
    save_workspace(&workspace, &paths).expect("save initial workspace");
    install_workspaces(&mut state, vec![workspace], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceNameChanged(
            "Renamed".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceConfirmed),
    );

    assert_eq!(
        state.workspace.as_ref().map(|ws| ws.name.as_str()),
        Some("Renamed")
    );
    assert_eq!(state.all_workspaces[0].name, "Renamed");
    assert!(state.workspace_mgr.rename_dialog.is_none());

    let (loaded, errors) = load_workspaces(&paths);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "Renamed");
}

#[test]
fn rename_workspace_save_failure_keeps_original_name_and_dialog() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = blocked_workspace_paths(&tmp);
    let mut state = make_state_with_paths(paths);
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceNameChanged(
            "Renamed".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RenameWorkspaceConfirmed),
    );

    assert_eq!(
        state.workspace.as_ref().map(|ws| ws.name.as_str()),
        Some("Main")
    );
    assert_eq!(state.all_workspaces[0].name, "Main");
    assert!(
        state
            .workspace_mgr
            .rename_dialog
            .as_ref()
            .and_then(|d| d.error.as_deref())
            .is_some_and(|error| error.starts_with("We could not save this workspace."))
    );
}

#[test]
fn delete_workspace_confirm_removes_file_and_selects_nearest_workspace() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());
    let first = Workspace::new("Main");
    let second = Workspace::new("Lab");
    let deleted_id = second.id.clone();
    save_workspace(&first, &paths).expect("save first");
    save_workspace(&second, &paths).expect("save second");
    install_workspaces(&mut state, vec![first, second], 1);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceRequested),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 1);
    assert_eq!(state.active_workspace_idx, 0);
    assert!(state.is_refreshing);
    assert_eq!(
        state.workspace.as_ref().map(|ws| ws.name.as_str()),
        Some("Main")
    );
    assert!(state.workspace_mgr.confirm_delete.is_none());

    let deleted_path = paths.workspaces_dir.join(format!("{deleted_id}.toml"));
    assert!(!deleted_path.exists());
}

#[test]
fn delete_workspace_failure_keeps_state_and_dialog() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = blocked_workspace_paths(&tmp);
    let mut state = make_state_with_paths(paths);
    let first = Workspace::new("Main");
    let second = Workspace::new("Lab");
    install_workspaces(&mut state, vec![first, second], 1);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceRequested),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 2);
    assert_eq!(state.active_workspace_idx, 1);
    assert_eq!(
        state.workspace.as_ref().map(|ws| ws.name.as_str()),
        Some("Lab")
    );
    assert!(!state.is_refreshing);
    assert!(
        state
            .workspace_mgr
            .confirm_delete
            .as_ref()
            .and_then(|d| d.error.as_deref())
            .is_some_and(|error| error.starts_with("We could not remove this workspace."))
    );
}

// Task 081: `delete_workspace_file` on its own, without going through
// dispatch — a blocked workspaces directory must error on every platform,
// while a workspaces directory that never existed at all is not an error.

#[test]
fn delete_workspace_file_blocked_by_a_plain_file_is_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = blocked_workspace_paths(&tmp);
    let workspace = Workspace::new("Main");

    assert!(delete_workspace_file(&workspace, &paths).is_err());
}

#[test]
fn delete_workspace_file_with_no_workspaces_directory_is_ok() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths {
        config_file: tmp.path().join("config.toml"),
        workspaces_dir: tmp.path().join("workspaces"),
        history_dir: tmp.path().join("history"),
    };
    let workspace = Workspace::new("Main");

    assert_eq!(delete_workspace_file(&workspace, &paths), Ok(()));
}

// Task 082: `load_workspaces` must not read a blocked workspaces directory
// as "nothing here yet" — the same ambiguity Task 081 removed from
// `delete_workspace_file`, one directory up.

#[test]
fn load_workspaces_blocked_by_a_plain_file_reports_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = blocked_workspace_paths(&tmp);

    let (loaded, errors) = load_workspaces(&paths);

    assert!(loaded.is_empty());
    assert!(!errors.is_empty(), "a blocked directory must be reported");
}

// RFC-075 stage 3 §5: retired. `create_dialog_open_enters_focus_at_the_
// name_field_r6` and `create_dialog_tab_wraps_within_the_dialog_and_
// never_touches_shell_focus_r5` both exercised the retired declared-order
// model directly (`overlay_focus` via `FocusNext`) — every window is real
// snora controls now (stage 3 §2), with no declared order left to walk.
// Stage 5 replaces this coverage.

/// RFC-066 D4/R5: Tab sets the outline visible; a mouse press clears it.
#[test]
fn focus_visible_is_set_by_tab_and_cleared_by_a_mouse_press() {
    let mut state = make_state();
    assert!(!state.focus_visible, "hidden by default");

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));
    assert!(state.focus_visible);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::MousePressed));
    assert!(!state.focus_visible);
}

// RFC-066 D4/R5's "reveal the ring instead of pressing" tests for the page
// (`activate_focused_with_hidden_ring_reveals_it_without_pressing`,
// `activate_focused_with_visible_ring_would_press_the_target`) are retired
// with the page's own knotra-focus model (RFC-075 §2): `ActivateFocused`
// is a flat no-op on the page now, never reaching this logic at all (see
// `app.rs`'s own comment on that match arm). The rule still holds for a
// window, which is what `handle_shortcut`'s unit-level coverage for it now
// exercises.
// RFC-075 stage 3 §5: retired. `assert_entry_never_starts_a_change`,
// `every_layer_and_phase_entry_never_starts_a_change_r2`,
// `settle_layer_focus_moves_off_a_stale_none_onto_a_newly_real_entry`,
// `tab_all_the_way_around_a_windows_order_returns_to_the_entry_r4` and
// `tab_and_shift_tab_stay_inside_five_more_windows_task_110` all exercised
// the retired declared-order model directly (`layer_focus`, `overlay_focus`,
// `advance_focus` via `FocusNext`/`FocusPrevious`) — every window is real
// snora controls now (stage 3 §2), with no declared order left to walk.
// Stage 5 replaces this coverage.

/// RFC-066 D6: Remove project is a dialog over any screen now, not a
/// Dashboard-only inline block — `top_layer` never reads `state.screen`
/// at all, so this is also a regression guard against a future change
/// that makes it read-gated on the Dashboard again.
#[test]
fn remove_project_is_the_top_layer_on_the_history_screen() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.screen = Screen::History;

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RemoveProjectRequested(project.id)),
    );

    assert_eq!(state.screen, Screen::History);
    assert_eq!(
        crate::state::layer::top_layer(&state),
        Some(crate::state::layer::Layer::RemoveProject)
    );
    assert!(crate::view::remove_project_dialog::view(&state).is_some());
}

// `arrow_down_matches_j_on_the_dashboard_with_no_window_open` (RFC-066 D5)
// read `state.dashboard_focus` after dispatching, which RFC-075 retires —
// the page's real focus lives in the live widget tree now, not a value
// this test file's single-`update()` `dispatch` helper can read
// synchronously afterward. `knotra_ui::widget::keyboard`'s own tests cover
// `↓`/`↑`/`j`/`k` landing on the same real id through the harness instead
// (RFC-075 stage 2 §5's acceptance tests, `keyboard_acceptance.rs`).

#[test]
fn confirm_reason_shown_only_while_genuinely_disabled() {
    // Reproduces `guided_button`'s own rule directly (RFC-036 Stage 5):
    // this codebase's tests have no way to inspect rendered output, so the
    // extracted decision is what's asserted, matching how Stage 3/4 tested
    // `Task`-producing decisions they also couldn't observe by side effect.
    assert!(crate::view::workspace_manager::confirm_shows_reason(
        false,
        Some("empty name")
    ));
    assert!(!crate::view::workspace_manager::confirm_shows_reason(
        false, None
    ));
    assert!(!crate::view::workspace_manager::confirm_shows_reason(
        true,
        Some("empty name")
    ));
    assert!(!crate::view::workspace_manager::confirm_shows_reason(
        true, None
    ));
}

// RFC-075 stage 3 §5: `delete_dialog_open_does_not_affect_focus_traversal_
// pure_function_tests` retired along with it — `state::focus` itself is
// gone (nothing in production called any of it once stage 3 converted
// every window's own controls to real snora focus targets); its own
// pure-function tests (`resolve`/`advance`/`reconcile`, deleted with the
// module) covered this directly, and there is nothing left to be a
// sanity check "on top of".

// ---------------------------------------------------------------------------
// RFC-036 Stage 4 — dashboard-row focus targets and bare `/`
// ---------------------------------------------------------------------------

/// Three projects, one per tier under the default `Attention` grouping:
/// Alpha (NeedsHelp, missing path), Beta (InProgress, one uncommitted
/// change), Gamma (AllSet, clean). Returns their ids in that order.
fn install_dashboard_projects(state: &mut AppState) -> (ProjectId, ProjectId, ProjectId) {
    let alpha = make_project("Alpha");
    let beta = make_project("Beta");
    let gamma = make_project("Gamma");
    let (alpha_id, beta_id, gamma_id) = (alpha.id.clone(), beta.id.clone(), gamma.id.clone());

    let mut workspace = Workspace::new("Main");
    workspace.projects = vec![alpha, beta, gamma];
    install_workspaces(state, vec![workspace], 0);

    state.missing_projects.insert(alpha_id.clone());

    let mut beta_status = make_project_status(beta_id.clone(), None);
    beta_status.working_tree.uncommitted_count = 1;
    let gamma_status = make_project_status(gamma_id.clone(), None);

    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![beta_status, gamma_status],
        last_refresh: Some(Timestamp::now()),
    });
    state.load_phase = crate::state::LoadPhase::Ready;
    // `AppConfig::default()` collapses AllSet by default; expand both
    // collapsible tiers so all three projects are selectable/visible unless
    // a specific test re-collapses one deliberately.
    state.config.dashboard_in_progress_collapsed = false;
    state.config.dashboard_all_set_collapsed = false;

    (alpha_id, beta_id, gamma_id)
}

#[test]
fn delete_last_workspace_is_not_allowed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths);
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceRequested),
    );
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceConfirmed),
    );

    assert_eq!(state.all_workspaces.len(), 1);
    assert_eq!(
        state
            .workspace_mgr
            .confirm_delete
            .as_ref()
            .and_then(|d| d.error.as_deref()),
        Some("Keep at least one workspace.")
    );
}

#[test]
fn close_shortcut_closes_only_topmost_layer() {
    let mut state = make_state();
    state.active_modal = ActiveModal::Changelog;
    state.workspace_mgr.create_dialog = Some(Default::default());
    state.add_project_dialog = Some(AddProjectDialog::default());
    state.palette.open = true;
    state.keyboard.cheat_sheet_open = true;

    // Task 098 Amendment A2: the shortcuts sheet outranks every other
    // layer in `layer::top_layer`'s priority order, so it must be the
    // first thing an Escape closes when every layer is open at once.
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert!(!state.keyboard.cheat_sheet_open);
    assert!(state.palette.open);
    assert!(state.add_project_dialog.is_some());
    assert!(state.workspace_mgr.create_dialog.is_some());
    assert!(matches!(state.active_modal, ActiveModal::Changelog));

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert!(!state.palette.open);
    assert!(state.add_project_dialog.is_some());
    assert!(state.workspace_mgr.create_dialog.is_some());
    assert!(matches!(state.active_modal, ActiveModal::Changelog));

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert!(state.add_project_dialog.is_none());
    assert!(state.workspace_mgr.create_dialog.is_some());
    assert!(matches!(state.active_modal, ActiveModal::Changelog));

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert!(state.workspace_mgr.create_dialog.is_none());
    assert!(matches!(state.active_modal, ActiveModal::Changelog));

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert!(matches!(state.active_modal, ActiveModal::None));
}

#[test]
fn add_project_back_returns_to_folder_step_with_the_path_kept() {
    let mut state = make_state();
    state.add_project_dialog = Some(AddProjectDialog {
        step: AddProjectStep::NameProject,
        path: "/home/user/projects/svc".to_owned(),
        name: "svc".to_owned(),
        error: None,
    });

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::AddProjectBack),
    );

    let dialog = state.add_project_dialog.expect("dialog stays open");
    assert_eq!(dialog.step, AddProjectStep::ChooseFolder);
    assert_eq!(dialog.path, "/home/user/projects/svc");
}

#[test]
fn palette_create_workspace_opens_dialog() {
    let mut state = make_state();
    state.palette.query = "create new workspace".to_owned();
    crate::state::palette::update_results(&mut state);
    let index = state
        .palette
        .results
        .iter()
        .position(|entry| entry.payload == "action.workspace_create")
        .expect("workspace create action visible");

    let message = crate::state::palette::dispatch_entry(&state, index);
    assert!(matches!(
        message,
        crate::state::palette::PaletteDispatch::Dispatched(Message::Workspace(
            WorkspaceMessage::CreateWorkspaceDialogOpened
        ))
    ));
}

/// RFC-066 D4/D5: there is no `highlighted` cursor any more — the index a
/// click or a keyboard activation dispatches is carried in the message
/// itself. Tests find that index by payload instead of setting a field.
fn palette_payload_index(state: &mut AppState, query: &str, payload: &str) -> usize {
    state.palette.query = query.to_owned();
    crate::state::palette::update_results(state);
    state
        .palette
        .results
        .iter()
        .position(|entry| entry.payload == payload)
        .unwrap_or_else(|| panic!("palette payload {payload} visible for query {query:?}"))
}

#[test]
fn palette_visible_action_rows_do_not_noop() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        }],
        0,
    );

    for action_id in crate::state::palette::visible_action_ids(&state) {
        let index = palette_payload_index(&mut state, action_id, action_id);
        assert!(
            !matches!(
                crate::state::palette::dispatch_entry(&state, index),
                crate::state::palette::PaletteDispatch::Noop
            ),
            "visible palette action {action_id} must dispatch or be disabled"
        );
    }
}

#[test]
fn palette_changelog_action_is_disabled_until_projects_are_selected() {
    let mut state = make_state();

    state.palette.query = "changelog".to_owned();
    crate::state::palette::update_results(&mut state);
    let entry = state
        .palette
        .results
        .iter()
        .find(|entry| entry.payload == "action.changelog_selected")
        .expect("changelog palette action is listed");
    assert_eq!(entry.disabled_reason_key, Some("plain.disabled.choose_one"));
}

/// Task 119 — the owner's exact path: switch to a workspace where every
/// project sits in the closed "All set" group, then open the command
/// palette. Debug-only in effect (the suite runs debug, where `Catalog::t`'s
/// `debug_assert!` is live) — this is a regression test for a bug that, on
/// the pre-fix code, failed here: `AppState::visible_projects_unavailable_reason_key`
/// (then named without the `_key` suffix) called `self.t(...)` itself and
/// handed the command palette's "Enter selection" row already-translated
/// text instead of a key, which `view/command_palette.rs` then fed to
/// `state.t()` a second time — `missing i18n key: Open a group to choose
/// its projects.`
#[test]
fn switching_to_a_workspace_with_every_project_in_a_closed_group_gives_the_palette_a_real_key() {
    let mut state = make_state();
    let second_project = make_project("Gamma");
    let second_project_id = second_project.id.clone();
    let mut second_workspace = Workspace::new("Second");
    second_workspace.projects = vec![second_project];
    let second_workspace_id = second_workspace.id.clone();
    install_workspaces(
        &mut state,
        vec![Workspace::new("Main"), second_workspace],
        0,
    );

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::WorkspaceSwitched(second_workspace_id)),
    );
    // `WorkspaceSwitched` clears `workspace_status` and starts the real
    // refresh as a `Task` a unit test never runs; stand in for that refresh
    // completing with one clean project, so the dashboard's only tier is
    // "All set" — collapsed by default (`AppConfig::default()`), the
    // owner's exact "every project sits in the closed 'All set' group".
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![make_project_status(second_project_id, None)],
        last_refresh: Some(Timestamp::now()),
    });
    state.load_phase = crate::state::LoadPhase::Ready;

    dispatch(&mut state, Message::Palette(PaletteMessage::Opened));

    for entry in &state.palette.results {
        if let Some(key) = entry.disabled_reason_key {
            assert!(
                state.catalog.contains_key(key),
                "palette entry {:?} carries {key:?} as its disabled_reason_key, \
                 which is not a catalog key — Task 119: that field must hold \
                 a key the view translates itself, never already-translated text",
                entry.payload
            );
        }
    }
}

/// Task 119 §3's sweep — every state the palette's own availability
/// functions distinguish, in both locales: no workspace, no projects
/// registered, every project in a closed group, a filter matching nothing,
/// and no project selected (the selection-dependent actions). A `Disabled`
/// reason that is accidentally already-translated text (as
/// `visible_projects_unavailable_reason_key` used to hand back) is missing
/// from one or both catalogs — `Catalog::contains_key` is the same check
/// `Catalog::t`'s `debug_assert!` makes, without the panic.
#[test]
fn every_palette_disabled_reason_is_a_real_key_in_both_catalogs() {
    type Scenario = (&'static str, fn() -> AppState);
    let scenarios: Vec<Scenario> = vec![
        ("no workspace registered", || make_state()),
        ("a workspace with no projects", || {
            let mut state = make_state();
            install_workspaces(&mut state, vec![Workspace::new("Empty")], 0);
            state.load_phase = crate::state::LoadPhase::Ready;
            state
        }),
        ("every project in a closed group", || {
            let mut state = make_state();
            let project = make_project("Gamma");
            let project_id = project.id.clone();
            let mut workspace = Workspace::new("Main");
            workspace.projects = vec![project];
            install_workspaces(&mut state, vec![workspace], 0);
            state.workspace_status = Some(WorkspaceStatus {
                projects: vec![make_project_status(project_id, None)],
                last_refresh: Some(Timestamp::now()),
            });
            state.load_phase = crate::state::LoadPhase::Ready;
            state
        }),
        ("a filter matching nothing", || {
            let mut state = make_state();
            let (_, _, _) = install_dashboard_projects(&mut state);
            state.filter.search_text = "no-such-project".to_owned();
            state
        }),
        ("nothing selected", || {
            let mut state = make_state();
            let _ = install_dashboard_projects(&mut state);
            state
        }),
    ];

    for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
        for (name, build) in &scenarios {
            let mut state = build();
            state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
            crate::state::palette::update_results(&mut state);

            for entry in &state.palette.results {
                if let Some(key) = entry.disabled_reason_key {
                    assert!(
                        state.catalog.contains_key(key),
                        "scenario {name:?}, locale {locale:?}: palette entry \
                         {:?} carries {key:?}, missing from this locale's catalog",
                        entry.payload
                    );
                }
            }
        }
    }
}

#[test]
fn palette_changelog_action_dispatches_selected_project_modal() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;
    state.selection.selected_ids.insert(project.id.clone());
    let index = palette_payload_index(&mut state, "changelog", "action.changelog_selected");

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state, index),
        crate::state::palette::PaletteDispatch::Dispatched(Message::Changelog(
            ChangelogMessage::BulkOpenRequested
        ))
    ));
}

#[test]
fn palette_project_row_opens_detail_panel() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    let index = palette_payload_index(&mut state, "svc", &project.id.to_string());

    let message = crate::state::palette::dispatch_entry(&state, index);

    assert!(matches!(
        message,
        crate::state::palette::PaletteDispatch::Dispatched(Message::DetailPanel(
            DetailPanelMessage::Opened(id)
        )) if id == project.id
    ));
}

#[test]
fn palette_active_workspace_row_is_disabled() {
    let mut state = make_state();
    let workspace = Workspace::new("Main");
    let workspace_id = workspace.id.clone();
    install_workspaces(&mut state, vec![workspace], 0);
    let index = palette_payload_index(&mut state, "Main", &workspace_id.to_string());

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state, index),
        crate::state::palette::PaletteDispatch::Disabled("palette.disabled.already_open")
    ));
}

#[test]
fn palette_fetch_all_uses_active_workspace_projects() {
    let mut state = make_state();
    let available = make_project("available");
    let missing = make_project("missing");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![available.clone(), missing.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.missing_projects.insert(missing.id.clone());
    state.selection.selected_ids.insert(missing.id.clone());
    state
        .sync
        .project_selection
        .insert(missing.id.clone(), true);

    dispatch(
        &mut state,
        Message::Sync(SyncMessage::BulkFetchAllRequested),
    );

    assert!(state.sync.selected_project_ids.contains(&available.id));
    assert!(!state.sync.selected_project_ids.contains(&missing.id));
    assert_eq!(state.sync.project_selection.get(&available.id), Some(&true));
    assert_eq!(state.sync.project_selection.get(&missing.id), Some(&false));
    let SyncPhase::FetchRunning {
        total,
        done,
        completed,
        ..
    } = &state.sync.phase
    else {
        panic!("expected active workspace fetch to start");
    };
    assert_eq!((*total, *done), (2, 1));
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].outcome, ProjectOperationOutcome::Skipped);
}

#[test]
fn palette_remove_project_requires_exactly_one_selected_project() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    let index = palette_payload_index(&mut state, "remove selected", "action.remove_project");
    assert!(matches!(
        crate::state::palette::dispatch_entry(&state, index),
        crate::state::palette::PaletteDispatch::Disabled("plain.disabled.choose_one")
    ));

    state.selection.selected_ids.insert(project.id.clone());
    let index = palette_payload_index(&mut state, "remove selected", "action.remove_project");
    assert!(matches!(
        crate::state::palette::dispatch_entry(&state, index),
        crate::state::palette::PaletteDispatch::Dispatched(Message::Workspace(
            WorkspaceMessage::RemoveProjectRequested(id)
        )) if id == project.id
    ));
}

#[test]
fn palette_next_workspace_switches_in_tab_order() {
    let mut state = make_state();
    let first = Workspace::new("Main");
    let second = Workspace::new("Lab");
    let second_id = second.id.clone();
    install_workspaces(&mut state, vec![first, second], 0);
    let index = palette_payload_index(&mut state, "next workspace", "action.workspace_next");

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state, index),
        crate::state::palette::PaletteDispatch::Dispatched(Message::Workspace(
            WorkspaceMessage::WorkspaceSwitched(id)
        )) if id == second_id
    ));
}

#[test]
fn selection_mode_enter_shows_empty_selection_state() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::Selection(SelectionMessage::ModeEntered),
    );

    assert!(state.selection_mode);
    assert_eq!(state.selection.selected_ids.len(), 0);
    assert!(state.selection.anchor_id.is_none());
    assert_eq!(state.selection_summary().selected_count, 0);
}

#[test]
fn selection_mode_entry_label_is_not_select_visible_action() {
    let state = make_state();

    assert_ne!(
        state.t("plain.selection.enter"),
        state.t("plain.select_visible_projects")
    );
}

#[test]
fn selection_last_deselect_keeps_empty_mode_and_clears_anchor() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::Selection(SelectionMessage::Toggled(project.id.clone())),
    );
    dispatch(
        &mut state,
        Message::Selection(SelectionMessage::Toggled(project.id.clone())),
    );

    assert!(state.selection_mode);
    assert_eq!(state.selection.selected_ids.len(), 0);
    assert!(state.selection.anchor_id.is_none());
}

#[test]
fn selection_toggle_rejects_project_outside_active_workspace() {
    let mut state = make_state();
    let project = make_project("svc");
    let stale = ProjectId::new();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::Selection(SelectionMessage::Toggled(stale)),
    );

    assert!(!state.selection_mode);
    assert_eq!(state.selection.selected_ids.len(), 0);
}

#[test]
fn selection_select_all_uses_visible_project_set() {
    let mut state = make_state();
    let visible = make_project("visible");
    let hidden = make_project("hidden");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![visible.clone(), hidden.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.apply_filter(FilterMessage::SearchChanged("visible".to_owned()));

    dispatch(&mut state, Message::Selection(SelectionMessage::SelectAll));

    assert!(state.selection_mode);
    assert!(state.selection.contains(&visible.id));
    assert!(!state.selection.contains(&hidden.id));
}

#[test]
fn workspace_switch_clears_selection_mode() {
    let mut state = make_state();
    let project = make_project("svc");
    let first = Workspace {
        projects: vec![project.clone()],
        ..Workspace::new("Main")
    };
    let second = Workspace::new("Lab");
    let second_id = second.id.clone();
    install_workspaces(&mut state, vec![first, second], 0);
    state.selection_mode = true;
    state.selection.selected_ids.insert(project.id);

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::WorkspaceSwitched(second_id)),
    );

    assert!(!state.selection_mode);
    assert_eq!(state.selection.selected_ids.len(), 0);
}

#[test]
fn project_removal_prunes_selection_and_exits_empty_mode() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;
    state.selection.selected_ids.insert(project.id.clone());
    state.selection.anchor_id = Some(project.id.clone());

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::RemoveProjectConfirmed(project.id)),
    );

    assert!(!state.selection_mode);
    assert_eq!(state.selection.selected_ids.len(), 0);
    assert!(state.selection.anchor_id.is_none());
}

#[test]
fn bulk_fetch_uses_dashboard_selection_exactly() {
    let mut state = make_state();
    let selected = make_project("selected");
    let other = make_project("other");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![selected.clone(), other.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;
    state.selection.selected_ids.insert(selected.id.clone());
    state.sync.project_selection.insert(other.id.clone(), true);

    dispatch(&mut state, Message::Sync(SyncMessage::BulkFetchRequested));

    assert!(state.sync.selected_project_ids.contains(&selected.id));
    assert!(!state.sync.selected_project_ids.contains(&other.id));
    assert_eq!(state.sync.project_selection.get(&selected.id), Some(&true));
    assert_eq!(state.sync.project_selection.get(&other.id), Some(&false));
    let SyncPhase::FetchRunning { total, done, .. } = &state.sync.phase else {
        panic!("expected selected-project fetch to start");
    };
    assert_eq!((*total, *done), (1, 0));
}

#[test]
fn bulk_fetch_empty_dashboard_selection_does_not_start() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;

    dispatch(&mut state, Message::Sync(SyncMessage::BulkFetchRequested));

    assert!(matches!(state.sync.phase, SyncPhase::Idle));
}

#[test]
fn context_bulk_open_uses_exactly_one_selected_project() {
    let mut state = make_state();
    let selected = make_project("selected");
    let other = make_project("other");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![selected.clone(), other],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;
    state.selection.selected_ids.insert(selected.id.clone());

    dispatch(
        &mut state,
        Message::Context(ContextMessage::BulkOpenRequested),
    );

    assert!(matches!(state.active_modal, ActiveModal::Switch));
    assert!(matches!(
        &state.context_ops.phase,
        ContextPhase::LoadingList(project_id) if project_id == &selected.id
    ));
}

#[test]
fn context_bulk_open_rejects_multiple_selected_projects() {
    let mut state = make_state();
    let first = make_project("first");
    let second = make_project("second");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![first.clone(), second.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection_mode = true;
    state.selection.selected_ids.insert(first.id);
    state.selection.selected_ids.insert(second.id);

    dispatch(
        &mut state,
        Message::Context(ContextMessage::BulkOpenRequested),
    );

    assert!(matches!(state.active_modal, ActiveModal::None));
    assert!(matches!(state.context_ops.phase, ContextPhase::Idle));
}

#[test]
fn context_target_choice_preserves_typed_target_and_blocks_dirty_project() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    let mut status = make_project_status(project.id.clone(), Some("origin/main"));
    status.working_tree.uncommitted_count = 1;
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![status],
        last_refresh: Some(Timestamp::now()),
    });
    let target = ContextTarget::GitLocalBranch {
        name: "feature/foo".to_owned(),
    };

    dispatch(
        &mut state,
        Message::Context(ContextMessage::SwitchTargetChosen(
            project.id.clone(),
            target.clone(),
            "feature/foo".to_owned(),
        )),
    );

    assert!(matches!(
        &state.context_ops.phase,
        ContextPhase::ConfirmSwitch {
            target: actual,
            disabled_reason_key: Some("plain.switch.reason_dirty"),
            ..
        } if actual == &target
    ));
}

#[test]
fn context_switch_confirm_does_not_execute_disabled_confirmation() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    let mut status = make_project_status(project.id.clone(), Some("origin/main"));
    status.conflict.has_conflict = true;
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![status],
        last_refresh: Some(Timestamp::now()),
    });

    dispatch(
        &mut state,
        Message::Context(ContextMessage::SwitchTargetChosen(
            project.id,
            ContextTarget::GitLocalBranch {
                name: "main".to_owned(),
            },
            "main".to_owned(),
        )),
    );
    dispatch(
        &mut state,
        Message::Context(ContextMessage::SwitchConfirmed),
    );

    assert!(matches!(
        &state.context_ops.phase,
        ContextPhase::ConfirmSwitch {
            disabled_reason_key: Some("plain.switch.reason_conflict"),
            ..
        }
    ));
}

#[test]
fn smart_pull_bulk_open_enters_planning_for_dashboard_selection() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection.selected_ids.insert(project.id.clone());

    dispatch(&mut state, Message::Sync(SyncMessage::BulkPullRequested));

    assert!(matches!(state.active_modal, ActiveModal::Pull));
    assert!(matches!(state.sync.phase, SyncPhase::Planning));
    assert!(state.sync.selected_project_ids.contains(&project.id));
}

#[test]
fn smart_pull_plan_keeps_mixed_no_upstream_project_skipped() {
    let mut state = make_state();
    let with_upstream = make_project("with-upstream");
    let no_upstream = make_project("no-upstream");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![with_upstream.clone(), no_upstream.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![
            make_project_status(with_upstream.id.clone(), Some("origin/main")),
            make_project_status(no_upstream.id.clone(), None),
        ],
        last_refresh: Some(Timestamp::now()),
    });
    state
        .selection
        .selected_ids
        .insert(with_upstream.id.clone());
    state.selection.selected_ids.insert(no_upstream.id.clone());

    dispatch(&mut state, Message::Sync(SyncMessage::BulkPullRequested));
    dispatch(
        &mut state,
        Message::Sync(SyncMessage::SmartPullPlanRequested),
    );

    let SyncPhase::AwaitingConfirm(plan) = &state.sync.phase else {
        panic!("expected reviewed plan");
    };
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.pull_count(), 1);
    assert_eq!(plan.excluded_count(), 1);
    let skipped = plan
        .entries
        .iter()
        .find(|entry| entry.project_id == no_upstream.id)
        .expect("no-upstream row");
    assert_eq!(skipped.disposition, SmartPullDisposition::Excluded);
    assert_eq!(skipped.skip_reason, Some(SmartPullSkipReason::NoUpstream));
}

#[test]
fn smart_pull_skipped_completion_persists_without_success_or_failure_count() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    let plan = SmartPullPlan {
        id: OperationId::new(),
        entries: vec![SmartPullPlanEntry {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            is_dirty: false,
            has_conflict: false,
            skip_reason: Some(SmartPullSkipReason::NoUpstream),
            disposition: SmartPullDisposition::Excluded,
        }],
    };
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::SmartPullExecution)
        .expect("acquire smart-pull lease");
    state.sync.phase = SyncPhase::PullRunning {
        plan,
        lease_id,
        started_at: Timestamp::now(),
        completed: Vec::new(),
    };

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            progress: Box::new(SmartPullProgress {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
                result: ProjectOperationResult {
                    project_id: project.id.clone(),
                    project_name: Some(project.name.clone()),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some("retry:not_in_active_workspace".to_owned()),
                    commands_executed: vec![],
                    stdout: "[excluded]".to_owned(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    error_message: None,
                },
                recovery_hint: None,
            }),
        }),
    );

    let SyncPhase::Done(result) = &state.sync.phase else {
        panic!("expected result phase");
    };
    assert_eq!(result.success_count(), 0);
    assert_eq!(result.fail_count(), 0);
    assert_eq!(result.skipped_count(), 1);
    assert_eq!(state.operation_logs.len(), 1);
    assert_eq!(
        state.operation_logs[0].result.successful_projects().len(),
        0
    );
    assert_eq!(state.operation_logs[0].result.failed_projects().len(), 0);
    assert_eq!(state.operation_logs[0].result.skipped_projects().len(), 1);

    let loaded = crate::persistence::load_recent_logs(&paths, 10).logs;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].result.skipped_projects().len(), 1);
    assert_eq!(
        loaded[0].result.per_project[0].skip_reason.as_deref(),
        Some("retry:not_in_active_workspace")
    );
}

fn install_running_smart_pull(state: &mut AppState) {
    let project = make_project("svc");
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::SmartPullExecution)
        .expect("acquire smart-pull lease");
    state.active_modal = ActiveModal::Pull;
    state.sync.phase = SyncPhase::PullRunning {
        plan: SmartPullPlan {
            id: OperationId::new(),
            entries: vec![SmartPullPlanEntry {
                project_id: project.id.clone(),
                project_name: project.name,
                is_dirty: false,
                has_conflict: false,
                skip_reason: None,
                disposition: SmartPullDisposition::Pull,
            }],
        },
        lease_id,
        started_at: Timestamp::now(),
        completed: Vec::new(),
    };
}

#[test]
fn smart_pull_running_modal_close_keeps_progress_visible() {
    let mut state = make_state();
    install_running_smart_pull(&mut state);

    dispatch(&mut state, Message::Sync(SyncMessage::ModalClosed));

    assert!(matches!(state.active_modal, ActiveModal::Pull));
    assert!(matches!(state.sync.phase, SyncPhase::PullRunning { .. }));
}

#[test]
fn smart_pull_running_escape_keeps_progress_visible() {
    let mut state = make_state();
    install_running_smart_pull(&mut state);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(matches!(state.active_modal, ActiveModal::Pull));
    assert!(matches!(state.sync.phase, SyncPhase::PullRunning { .. }));
}

#[test]
fn shortcut_open_context_ops_does_nothing_while_a_window_is_open() {
    let mut state = make_state();
    install_running_smart_pull(&mut state);

    dispatch(
        &mut state,
        Message::Shortcut(ShortcutMessage::OpenContextOps),
    );

    assert!(matches!(state.active_modal, ActiveModal::Pull));
    assert!(matches!(state.sync.phase, SyncPhase::PullRunning { .. }));
}

#[test]
fn freezer_bulk_open_initializes_dashboard_selection() {
    let mut state = make_state();
    let selected = make_project("selected");
    let other = make_project("other");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![selected.clone(), other.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection.selected_ids.insert(selected.id.clone());

    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::BulkOpenRequested),
    );

    assert!(matches!(state.active_modal, ActiveModal::Tag));
    assert!(matches!(state.freezer.phase, FreezerPhase::Idle));
    assert_eq!(state.freezer.project_selection.len(), 1);
    assert_eq!(
        state.freezer.project_selection.get(&selected.id),
        Some(&true)
    );
    assert!(!state.freezer.project_selection.contains_key(&other.id));
}

#[test]
fn changelog_bulk_open_initializes_dashboard_selection() {
    let mut state = make_state();
    let selected = make_project("selected");
    let other = make_project("other");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![selected.clone(), other.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.selection.selected_ids.insert(selected.id.clone());

    dispatch(
        &mut state,
        Message::Changelog(ChangelogMessage::BulkOpenRequested),
    );

    assert!(matches!(state.active_modal, ActiveModal::Changelog));
    assert!(matches!(state.changelog.phase, ChangelogPhase::Idle));
    assert_eq!(state.changelog.project_selection.len(), 1);
    assert_eq!(
        state.changelog.project_selection.get(&selected.id),
        Some(&true)
    );
    assert!(!state.changelog.project_selection.contains_key(&other.id));
}

#[test]
fn changelog_bulk_open_rejects_empty_selection() {
    let mut state = make_state();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![make_project("svc")],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::Changelog(ChangelogMessage::BulkOpenRequested),
    );

    assert!(matches!(state.active_modal, ActiveModal::None));
    assert!(state.changelog.project_selection.is_empty());
}

#[test]
fn changelog_late_background_result_is_ignored_after_close() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state
        .changelog
        .project_selection
        .insert(project.id.clone(), true);
    state.changelog.since_ref = "v1.1.0".to_owned();
    let request_id = state.changelog.begin_collection();

    dispatch(
        &mut state,
        Message::Changelog(ChangelogMessage::ModalClosed),
    );
    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ChangelogDraftReady {
            request_id,
            draft: make_changelog_draft(&project, vec![make_commit("abcdef123456", "Add notes")]),
        }),
    );

    assert!(matches!(state.active_modal, ActiveModal::None));
    assert!(!matches!(state.changelog.phase, ChangelogPhase::Ready(_)));
}

#[test]
fn changelog_since_edit_during_collection_returns_to_idle_and_ignores_late_result() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state
        .changelog
        .project_selection
        .insert(project.id.clone(), true);
    state.changelog.since_ref = "v1.1.0".to_owned();
    let request_id = state.changelog.begin_collection();

    dispatch(
        &mut state,
        Message::Changelog(ChangelogMessage::SinceRefChanged("v1.2.0".to_owned())),
    );
    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ChangelogDraftReady {
            request_id,
            draft: make_changelog_draft(&project, vec![make_commit("abcdef123456", "Add notes")]),
        }),
    );

    assert_eq!(state.changelog.since_ref, "v1.2.0");
    assert!(matches!(state.changelog.phase, ChangelogPhase::Idle));
    assert_eq!(state.changelog.active_request_id, None);
}

#[test]
fn changelog_background_result_sets_ready_for_active_request() {
    let mut state = make_state();
    let project = make_project("svc");
    let request_id = state.changelog.begin_collection();

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ChangelogDraftReady {
            request_id,
            draft: make_changelog_draft(&project, vec![make_commit("abcdef123456", "Add notes")]),
        }),
    );

    assert!(matches!(state.changelog.phase, ChangelogPhase::Ready(_)));
    assert_eq!(state.changelog.active_request_id, None);
}

#[test]
fn changelog_copy_requested_uses_localized_status_feedback() {
    let mut state = make_state();
    let project = make_project("svc");
    state.changelog.phase = ChangelogPhase::Ready(make_changelog_draft(
        &project,
        vec![make_commit("abcdef123456", "Add notes")],
    ));

    dispatch(
        &mut state,
        Message::Changelog(ChangelogMessage::CopyRequested),
    );

    let status = state.status_bar.as_deref().unwrap_or_default();
    assert!(status.starts_with(state.t("plain.changelog.copied_prefix")));
    assert!(status.ends_with(state.t("plain.changelog.copied_suffix")));
}

#[test]
fn changelog_preview_uses_markdown_not_debug_output() {
    let project = make_project("svc");
    let draft = make_changelog_draft(&project, vec![make_commit("abcdef123456", "Add notes")]);

    let preview = crate::view::bulk_modals::changelog_markdown_preview(&draft);

    assert!(preview.contains("# Changelog"));
    assert!(preview.contains("## svc"));
    assert!(preview.contains("Add notes"));
    assert!(!preview.contains("ChangelogDraft"));
    assert!(!preview.contains("ProjectCommits"));
}

#[test]
fn changelog_result_counts_include_no_change_and_error_projects() {
    let ok_project = make_project("ok");
    let empty_project = make_project("empty");
    let failed_project = make_project("failed");
    let draft = ChangelogDraft {
        release_name: "v1.2.0".to_owned(),
        generated_at: Timestamp::now(),
        projects: vec![
            ProjectCommits {
                project_id: ok_project.id,
                project_name: ok_project.name,
                since_ref: "v1.1.0".to_owned(),
                entries: vec![make_commit("abcdef123456", "Add notes")],
                error: None,
            },
            ProjectCommits {
                project_id: empty_project.id,
                project_name: empty_project.name,
                since_ref: "v1.1.0".to_owned(),
                entries: Vec::new(),
                error: None,
            },
            ProjectCommits {
                project_id: failed_project.id,
                project_name: failed_project.name,
                since_ref: "v1.1.0".to_owned(),
                entries: Vec::new(),
                error: Some("bad ref".to_owned()),
            },
        ],
    };

    let counts = crate::view::bulk_modals::changelog_result_counts(&draft);

    assert_eq!(counts.total_commits, 1);
    assert_eq!(counts.projects_with_commits, 1);
    assert_eq!(counts.projects_without_changes, 1);
    assert_eq!(counts.projects_with_errors, 1);
}

#[test]
fn freezer_execute_confirmed_enters_executing_from_ready_validation() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.freezer.phase =
        FreezerPhase::ValidationReady(ready_freeze_validation(&project, "v1.0.0"));

    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::ExecuteConfirmed),
    );

    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
    assert!(state.freezer.execution_started_at.is_some());
}

#[test]
fn freezer_execute_requires_at_least_one_ready_project() {
    let mut state = make_state();
    state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
        freeze_name: "v1.0.0".to_owned(),
        entries: Vec::new(),
    });

    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::ExecuteConfirmed),
    );

    assert!(matches!(
        state.freezer.phase,
        FreezerPhase::ValidationReady(_)
    ));
    assert!(state.freezer.execution_started_at.is_none());
}

fn install_running_freezer(state: &mut AppState) {
    state.active_modal = ActiveModal::Tag;
    state.freezer.phase = FreezerPhase::Executing;
    state.freezer.execution_started_at = Some(Timestamp::now());
}

#[test]
fn freezer_running_modal_close_keeps_progress_visible() {
    let mut state = make_state();
    install_running_freezer(&mut state);

    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::BulkModalClosed),
    );

    assert!(matches!(state.active_modal, ActiveModal::Tag));
    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
}

#[test]
fn freezer_running_escape_keeps_progress_visible() {
    let mut state = make_state();
    install_running_freezer(&mut state);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(matches!(state.active_modal, ActiveModal::Tag));
    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
}

#[test]
fn shortcut_open_freezer_does_nothing_while_a_window_is_open() {
    let mut state = make_state();
    install_running_freezer(&mut state);
    let started_at = state.freezer.execution_started_at;

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::OpenFreezer));

    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
    assert!(matches!(state.active_modal, ActiveModal::Tag));
    assert_eq!(state.freezer.execution_started_at, started_at);
}

#[test]
fn shortcut_focus_search_does_nothing_while_a_window_is_open() {
    let mut state = make_state();
    install_running_freezer(&mut state);
    state.screen = Screen::History;

    dispatch(
        &mut state,
        Message::Shortcut(ShortcutMessage::FocusSearchBare),
    );
    assert_eq!(state.screen, Screen::History);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusSearch));
    assert_eq!(state.screen, Screen::History);
}

#[test]
fn shortcut_refresh_does_nothing_while_a_window_is_open() {
    let mut state = make_state();
    install_running_freezer(&mut state);
    state.is_refreshing = false;

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Refresh));

    assert!(!state.is_refreshing);
}

/// RFC-071 D11: zoom "work[s] only when no window is open" — unlike
/// `Refresh`/`OpenContextOps`/`OpenFreezer` (Task 095's exemption, which
/// let those act on the screen behind an open window), zoom has none, so
/// all three shortcuts must leave `zoom_percent` untouched while a window
/// is open.
#[test]
fn zoom_shortcuts_do_nothing_while_a_window_is_open() {
    let mut state = make_state();
    install_running_freezer(&mut state);
    let before = state.config.zoom_percent;

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::ZoomIn));
    assert_eq!(state.config.zoom_percent, before);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::ZoomOut));
    assert_eq!(state.config.zoom_percent, before);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::ZoomReset));
    assert_eq!(state.config.zoom_percent, before);
}

/// RFC-066 D7: `Ctrl+R` now sends the exact message the "Check now" button
/// sends (`WorkspaceMessage::RefreshRequested`), not `handle_tick` directly
/// — the two were not quite identical: only `RefreshRequested` sets
/// `status_bar`, so Ctrl+R used to refresh silently where the button did
/// not.
#[test]
fn shortcut_refresh_sets_the_status_bar_like_the_check_now_button() {
    let mut state = make_state();
    state.is_refreshing = false;
    state.status_bar = None;

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Refresh));

    assert!(state.is_refreshing);
    assert!(state.status_bar.is_some());
}

#[test]
fn changelog_escape_clears_stale_collection_like_close_does() {
    let mut state = make_state();
    state.active_modal = ActiveModal::Changelog;
    state.changelog.phase = ChangelogPhase::Collecting;
    state.changelog.active_request_id = Some(7);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(matches!(state.active_modal, ActiveModal::None));
    assert_eq!(state.changelog.active_request_id, None);
}

#[test]
fn smart_pull_escape_clears_retry_exclusions_like_close_does() {
    let mut state = make_state();
    state.active_modal = ActiveModal::Pull;
    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
        id: OperationId::new(),
        entries: Vec::new(),
    });
    state.sync.retry_exclusions = vec![RetryExclusion {
        project_id: ProjectId::new(),
        reason: RetryExclusionReason::StatusUnavailable,
    }];

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(matches!(state.active_modal, ActiveModal::None));
    assert!(state.sync.retry_exclusions.is_empty());
}

#[test]
fn freezer_completion_persists_log_and_offers_git_push() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths.clone());
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![make_project_status_with_kind(
            project.id.clone(),
            VcsKind::Git,
        )],
        last_refresh: Some(Timestamp::now()),
    });
    state.freezer.execution_started_at = Some(Timestamp::now());
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::FreezeExecution)
        .expect("acquire freezer lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone {
            lease_id,
            result: FreezeResult {
                freeze_name: "v1.0.0".to_owned(),
                project_results: vec![FreezeProjectResult {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    success: true,
                    commands_executed: vec!["git tag v1.0.0".to_owned()],
                    stdout: String::new(),
                    stderr: String::new(),
                    rollback_attempted: false,
                    rollback_succeeded: None,
                    recovery_hint: None,
                }],
                outcome: FreezeOutcome::Success,
            },
        }),
    );

    assert!(matches!(state.freezer.phase, FreezerPhase::Done(_)));
    assert_eq!(state.operation_logs.len(), 1);
    assert_eq!(state.operation_logs[0].result.kind, OperationKind::Freeze);
    let pending = state.pending_tag_push.as_ref().expect("pending push offer");
    assert_eq!(pending.freeze_name, "v1.0.0");
    assert_eq!(pending.project_ids, vec![project.id.clone()]);

    let loaded = crate::persistence::load_recent_logs(&paths, 10).logs;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].result.kind, OperationKind::Freeze);
}

#[test]
fn freezer_completion_does_not_offer_jj_push() {
    let mut state = make_state();
    let project = make_project("svc");
    install_pending_push(&mut state, "v1.0.0", ProjectId::new());
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![make_project_status_with_kind(
            project.id.clone(),
            VcsKind::Jujutsu,
        )],
        last_refresh: Some(Timestamp::now()),
    });
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::FreezeExecution)
        .expect("acquire freezer lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone {
            lease_id,
            result: FreezeResult {
                freeze_name: "v1.0.0".to_owned(),
                project_results: vec![FreezeProjectResult {
                    project_id: project.id.clone(),
                    project_name: project.name,
                    success: true,
                    commands_executed: vec!["jj bookmark create v1.0.0 -r @".to_owned()],
                    stdout: String::new(),
                    stderr: String::new(),
                    rollback_attempted: false,
                    rollback_succeeded: None,
                    recovery_hint: None,
                }],
                outcome: FreezeOutcome::Success,
            },
        }),
    );

    assert!(state.pending_tag_push.is_none());
}

#[test]
fn freezer_non_success_result_clears_stale_push_offer() {
    let mut state = make_state();
    let project = make_project("svc");
    install_pending_push(&mut state, "v1.0.0", ProjectId::new());
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.workspace_status = Some(WorkspaceStatus {
        projects: vec![make_project_status_with_kind(
            project.id.clone(),
            VcsKind::Git,
        )],
        last_refresh: Some(Timestamp::now()),
    });
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::FreezeExecution)
        .expect("acquire freezer lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone {
            lease_id,
            result: FreezeResult {
                freeze_name: "v1.0.0".to_owned(),
                project_results: vec![FreezeProjectResult {
                    project_id: project.id.clone(),
                    project_name: project.name,
                    success: false,
                    commands_executed: vec!["git tag v1.0.0".to_owned()],
                    stdout: String::new(),
                    stderr: "failed".to_owned(),
                    rollback_attempted: false,
                    rollback_succeeded: None,
                    recovery_hint: None,
                }],
                outcome: FreezeOutcome::RolledBack,
            },
        }),
    );

    assert!(state.pending_tag_push.is_none());
}

#[test]
fn freezer_start_execution_clears_stale_push_offer() {
    let mut state = make_state();
    let project = make_project("svc");
    install_pending_push(&mut state, "v0.9.0", project.id.clone());
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state.freezer.phase =
        FreezerPhase::ValidationReady(ready_freeze_validation(&project, "v1.0.0"));

    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::ExecuteConfirmed),
    );

    assert!(state.pending_tag_push.is_none());
    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
}

#[test]
fn tag_push_decline_clears_pending_offer() {
    let mut state = make_state();
    let project = make_project("svc");
    install_pending_push(&mut state, "v1.0.0", project.id);

    dispatch(&mut state, Message::TagPush(TagPushMessage::PushDeclined));

    assert!(state.pending_tag_push.is_none());
}

#[test]
fn conflict_mark_resolved_request_enters_operating_for_git_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("git marker");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());
    let mut state = make_state();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::MarkResolvedRequested {
            project_id: project.id.clone(),
            file_path: "src/lib.rs".to_owned(),
        }),
    );

    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Operating { .. }
    ));
}

#[test]
fn conflict_mark_resolved_request_rejects_jj_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".jj")).expect("jj marker");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());
    let mut state = make_state();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::MarkResolvedRequested {
            project_id: project.id.clone(),
            file_path: "src/lib.rs".to_owned(),
        }),
    );

    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Done { success: false, .. }
    ));
}

#[test]
fn conflict_abort_request_requires_git_merge_state() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let git_dir = tmp.path().join(".git");
    std::fs::create_dir(&git_dir).expect("git marker");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());
    let mut state = make_state();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );

    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::AbortMergeRequested(project.id.clone())),
    );
    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Done { success: false, .. }
    ));

    std::fs::write(git_dir.join("MERGE_HEAD"), "merge").expect("merge marker");
    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::AbortMergeRequested(project.id.clone())),
    );
    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Operating { .. }
    ));
}

#[test]
fn conflict_stop_requested_enters_confirm_stop_without_running_a_command() {
    let project = make_project("svc");
    let mut state = make_state();
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: Vec::new(),
        read_error: None,
    };
    state.active_modal = ActiveModal::Resolve(project.id.clone());
    state.conflict_ops.phase = ConflictPhase::Browsing {
        project_id: project.id.clone(),
        detail: detail.clone(),
    };

    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::StopRequested(project.id.clone())),
    );

    match &state.conflict_ops.phase {
        ConflictPhase::ConfirmStop {
            project_id,
            detail: carried,
        } => {
            assert_eq!(*project_id, project.id);
            assert_eq!(carried.project_id, detail.project_id);
            assert_eq!(
                carried.conflicted_files.len(),
                detail.conflicted_files.len()
            );
        }
        other => panic!("expected ConfirmStop, got {other:?}"),
    }
    // No operation lease was acquired — the lock is free for a real
    // operation to take, unlike `AbortMergeRequested`'s own baseline
    // behaviour from the same starting state (seen to fail: dispatching
    // `AbortMergeRequested` directly from `Browsing` — the pre-RFC-067
    // button's message — enters `Operating` at once, with no confirmation;
    // confirmed via a temporary scratch test before this one was written,
    // output recorded in the review request).
    assert!(!state.operation_interlock.is_busy());
}

fn confirm_stop_state(project: &Project, detail: ProjectConflictDetail) -> AppState {
    let mut state = make_state();
    state.active_modal = ActiveModal::Resolve(project.id.clone());
    state.conflict_ops.phase = ConflictPhase::ConfirmStop {
        project_id: project.id.clone(),
        detail,
    };
    state
}

#[test]
fn conflict_stop_cancelled_returns_to_browsing_with_the_same_detail() {
    let project = make_project("svc");
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: Vec::new(),
        read_error: None,
    };
    let mut state = confirm_stop_state(&project, detail.clone());

    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::StopCancelled),
    );

    assert!(matches!(state.active_modal, ActiveModal::Resolve(_)));
    match &state.conflict_ops.phase {
        ConflictPhase::Browsing {
            project_id,
            detail: carried,
        } => {
            assert_eq!(*project_id, project.id);
            assert_eq!(carried.project_id, detail.project_id);
        }
        other => panic!("expected Browsing, got {other:?}"),
    }
}

#[test]
fn conflict_confirm_stop_escape_cancels_like_keep_fixing_does() {
    let project = make_project("svc");
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: Vec::new(),
        read_error: None,
    };
    let mut state = confirm_stop_state(&project, detail);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    // The panel stays open — Escape from `ConfirmStop` must not close it,
    // only cancel the confirmation, the same as "Keep fixing".
    assert!(matches!(state.active_modal, ActiveModal::Resolve(_)));
    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Browsing { .. }
    ));
}

#[test]
fn conflict_stray_stop_messages_change_nothing() {
    let project = make_project("svc");

    // StopRequested while Loading: not Browsing, so nothing happens.
    let mut state = make_state();
    state.conflict_ops.phase = ConflictPhase::Loading(project.id.clone());
    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::StopRequested(project.id.clone())),
    );
    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Loading(_)
    ));

    // StopCancelled while Browsing: not ConfirmStop, so nothing happens.
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: Vec::new(),
        read_error: None,
    };
    let mut state = make_state();
    state.conflict_ops.phase = ConflictPhase::Browsing {
        project_id: project.id.clone(),
        detail: detail.clone(),
    };
    dispatch(
        &mut state,
        Message::ConflictOps(ConflictOpsMessage::StopCancelled),
    );
    match &state.conflict_ops.phase {
        ConflictPhase::Browsing { project_id, .. } => assert_eq!(*project_id, project.id),
        other => panic!("expected Browsing, got {other:?}"),
    }
}

#[test]
fn conflict_running_escape_keeps_panel_visible() {
    let mut state = make_state();
    let project = make_project("svc");
    state.active_modal = ActiveModal::Resolve(project.id.clone());
    state.conflict_ops.phase = ConflictPhase::Operating {
        project_id: project.id,
        action: "Working".to_owned(),
    };

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(matches!(state.active_modal, ActiveModal::Resolve(_)));
    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Operating { .. }
    ));
}

#[test]
fn conflict_operation_completion_success_refreshes_browsing_detail() {
    let mut state = make_state();
    let project = make_project("svc");
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: Vec::new(),
        read_error: None,
    };
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::ConflictMutation)
        .expect("acquire conflict lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ConflictOperationCompleted {
            lease_id,
            result: Box::new(ProjectOperationResult {
                project_id: project.id.clone(),
                project_name: Some(project.name.clone()),
                outcome: ProjectOperationOutcome::Succeeded,
                success: true,
                skip_reason: None,
                commands_executed: vec!["git add src/lib.rs".to_owned()],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                error_message: None,
            }),
            detail,
        }),
    );

    assert!(matches!(
        state.conflict_ops.phase,
        ConflictPhase::Browsing { .. }
    ));
    assert!(state.conflict_ops.cached.contains_key(&project.id));
}

#[test]
fn conflict_operation_completion_failure_keeps_panel_error_visible() {
    let mut state = make_state();
    let project = make_project("svc");
    let detail = ProjectConflictDetail {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        conflicted_files: vec![ConflictedFile {
            path: "src/lib.rs".to_owned(),
            marker: ConflictMarker::BothModified,
        }],
        read_error: None,
    };
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::ConflictMutation)
        .expect("acquire conflict lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ConflictOperationCompleted {
            lease_id,
            result: Box::new(ProjectOperationResult {
                project_id: project.id.clone(),
                project_name: Some(project.name.clone()),
                outcome: ProjectOperationOutcome::Failed,
                success: false,
                skip_reason: None,
                commands_executed: vec!["git add src/lib.rs".to_owned()],
                stdout: String::new(),
                stderr: "failed".to_owned(),
                exit_code: Some(1),
                error_message: Some("failed".to_owned()),
            }),
            detail,
        }),
    );

    let ConflictPhase::Done {
        success,
        message,
        result,
        ..
    } = &state.conflict_ops.phase
    else {
        panic!("expected conflict error state");
    };
    assert!(!success);
    assert_eq!(message, "We could not finish that action.");
    let result = result.as_ref().expect("operation result details");
    assert_eq!(
        result.commands_executed,
        vec!["git add src/lib.rs".to_owned()]
    );
    assert_eq!(result.stderr, "failed");
    assert_eq!(result.error_message.as_deref(), Some("failed"));
    assert!(state.conflict_ops.cached.contains_key(&project.id));
}

#[test]
fn operation_interlock_rejects_overlap_and_stale_release() {
    let owners = [
        OperationOwner::SingleFetch,
        OperationOwner::BulkFetch,
        OperationOwner::SmartPullPreparation,
        OperationOwner::SmartPullExecution,
        OperationOwner::ContextSwitch,
        OperationOwner::FreezeValidation,
        OperationOwner::FreezeExecution,
        OperationOwner::ConflictMutation,
        OperationOwner::TagPush,
        OperationOwner::ActivitySmartPullPreparation,
    ];
    for owner in owners {
        let mut state = make_state();
        let retry = state
            .operation_interlock
            .try_acquire(OperationOwner::ActivityFetchRetry)
            .expect("retry lease");
        assert!(state.operation_interlock.try_acquire(owner).is_none());
        assert!(state.operation_interlock.release_if_matches(retry));

        let ordinary = state
            .operation_interlock
            .try_acquire(owner)
            .expect("ordinary lease");
        assert!(
            state
                .operation_interlock
                .try_acquire(OperationOwner::ActivityFetchRetry)
                .is_none()
        );
        assert!(state.operation_interlock.release_if_matches(ordinary));
    }

    let mut state = make_state();
    let first = state
        .operation_interlock
        .try_acquire(OperationOwner::ActivityFetchRetry)
        .expect("first lease");
    assert!(state.operation_interlock.release_if_matches(first));
    let second = state
        .operation_interlock
        .try_acquire(OperationOwner::SingleFetch)
        .expect("second lease");
    assert!(!state.operation_interlock.release_if_matches(first));
    assert!(state.operation_interlock.is_busy());
    assert!(state.operation_interlock.release_if_matches(second));
}

#[test]
fn ordinary_fetch_is_rejected_while_activity_retry_owns_interlock() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    state
        .operation_interlock
        .try_acquire(OperationOwner::ActivityFetchRetry)
        .expect("activity retry lease");

    dispatch(
        &mut state,
        Message::Project(crate::message::ProjectMessage::FetchRequested(
            project.id.clone(),
        )),
    );

    assert!(!state.fetching_projects.contains(&project.id));
    assert_eq!(
        state.status_bar.as_deref(),
        Some("Wait for the current operation to finish.")
    );
}

#[test]
fn palette_mutating_actions_show_busy_reason_during_activity_retry() {
    let mut state = make_state();
    let project = make_project("svc");
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        }],
        0,
    );
    state
        .operation_interlock
        .try_acquire(OperationOwner::ActivityFetchRetry)
        .expect("activity retry lease");

    crate::state::palette::update_results(&mut state);

    let fetch = state
        .palette
        .results
        .iter()
        .find(|entry| entry.payload == "action.fetch_all")
        .expect("fetch action");
    assert_eq!(fetch.disabled_reason_key, Some("plain.activity.busy"));
}

#[test]
fn legacy_failed_result_enables_typed_fetch_retry() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut state = make_state_with_paths(AppPaths::under(tmp.path().to_path_buf()));
    let project_id = ProjectId::new();
    let mut legacy = make_operation_result(project_id.clone(), ProjectOperationOutcome::Succeeded);
    legacy.success = false;
    let log = make_operation_log(OperationKind::Fetch, vec![legacy]);
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::SingleFetch)
        .expect("single fetch lease");

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SingleFetchCompleted { lease_id, log }),
    );

    let LatestOpState::Completed {
        retry: RetryAvailability::Available(ActivityRetryAction::FetchFailed { project_ids, .. }),
        ..
    } = &state.activity.latest
    else {
        panic!("expected typed fetch retry");
    };
    assert_eq!(project_ids, &vec![project_id]);
}

#[test]
fn activity_details_opens_and_expands_source_history_entry() {
    let mut state = make_state();
    let operation_id = OperationId::new();

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::DetailsRequested {
            operation_id: operation_id.clone(),
        }),
    );

    assert_eq!(state.screen, Screen::History);
    assert!(state.history_expanded.contains(&operation_id));
}

#[test]
fn fetch_retry_records_ineligible_source_target_as_skipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("git marker");
    let paths = AppPaths::under(tmp.path().join("state"));
    let mut state = make_state_with_paths(paths);
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());
    let removed_id = ProjectId::new();
    install_workspaces(
        &mut state,
        vec![Workspace {
            projects: vec![project.clone()],
            ..Workspace::new("Main")
        }],
        0,
    );
    let source = make_operation_log(
        OperationKind::Fetch,
        vec![
            make_operation_result(project.id.clone(), ProjectOperationOutcome::Failed),
            make_operation_result(removed_id.clone(), ProjectOperationOutcome::Failed),
        ],
    );
    let source_id = source.result.operation_id.clone();
    state.activity.latest = LatestOpState::Completed {
        log: source,
        retry: RetryAvailability::Available(ActivityRetryAction::FetchFailed {
            source_operation_id: source_id.clone(),
            project_ids: vec![project.id.clone(), removed_id.clone()],
        }),
    };

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id.clone(),
        }),
    );
    let run = state.activity.fetch_retry.clone().expect("fetch retry run");
    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ActivityFetchRetryProjectCompleted {
            lease_id: run.lease_id,
            operation_id: run.operation_id,
            result: Box::new(make_operation_result(
                project.id,
                ProjectOperationOutcome::Succeeded,
            )),
        }),
    );

    assert_eq!(state.operation_logs.len(), 1);
    let result = &state.operation_logs[0].result;
    assert_eq!(result.successful_projects().len(), 1);
    assert_eq!(result.failed_projects().len(), 0);
    assert_eq!(result.skipped_projects().len(), 1);
    let skipped = result.skipped_projects()[0];
    assert_eq!(skipped.project_id, removed_id);
    assert_eq!(
        skipped
            .skip_reason
            .as_deref()
            .and_then(RetryExclusionReason::from_code),
        Some(RetryExclusionReason::NotInActiveWorkspace)
    );
}

#[test]
fn closing_smart_pull_retry_ignores_late_status_result_and_releases_lease() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("git marker");
    let mut state = make_state();
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());
    let workspace = Workspace {
        projects: vec![project.clone()],
        ..Workspace::new("Main")
    };
    let workspace_id = workspace.id.clone();
    install_workspaces(&mut state, vec![workspace], 0);
    let source = make_operation_log(
        OperationKind::SmartPull,
        vec![make_operation_result(
            project.id.clone(),
            ProjectOperationOutcome::Failed,
        )],
    );
    let source_id = source.result.operation_id.clone();
    state.activity.latest = LatestOpState::Completed {
        log: source,
        retry: RetryAvailability::Available(ActivityRetryAction::ReviewSmartPull {
            source_operation_id: source_id.clone(),
            project_ids: vec![project.id.clone()],
        }),
    };

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id.clone(),
        }),
    );
    let preparation = state
        .sync
        .retry_preparation
        .clone()
        .expect("retry preparation");
    assert!(state.operation_interlock.is_busy());

    dispatch(&mut state, Message::Sync(SyncMessage::ModalClosed));
    assert!(!state.operation_interlock.is_busy());
    assert_eq!(state.active_modal, ActiveModal::None);
    assert!(matches!(state.sync.phase, SyncPhase::Idle));

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
            request_id: preparation.id,
            workspace_id: workspace_id.clone(),
            lease_id: preparation.lease_id,
            statuses: vec![make_project_status(project.id.clone(), Some("origin/main"))],
        }),
    );
    assert!(matches!(state.sync.phase, SyncPhase::Idle));
    assert!(state.sync.retry_preparation.is_none());

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id.clone(),
        }),
    );
    let superseded = state
        .sync
        .retry_preparation
        .clone()
        .expect("superseded preparation");
    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id,
        }),
    );
    let current = state
        .sync
        .retry_preparation
        .clone()
        .expect("current preparation");
    assert_ne!(superseded.id, current.id);
    assert_ne!(superseded.lease_id, current.lease_id);

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
            request_id: superseded.id,
            workspace_id: workspace_id.clone(),
            lease_id: superseded.lease_id,
            statuses: vec![make_project_status(project.id.clone(), Some("origin/main"))],
        }),
    );
    assert_eq!(
        state.sync.retry_preparation.as_ref().map(|prep| prep.id),
        Some(current.id)
    );
    assert!(matches!(state.sync.phase, SyncPhase::RetryPreparing));

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
            request_id: current.id,
            workspace_id,
            lease_id: current.lease_id,
            statuses: vec![make_project_status(project.id, Some("origin/main"))],
        }),
    );
    assert!(matches!(state.sync.phase, SyncPhase::AwaitingConfirm(_)));
    assert!(!state.operation_interlock.is_busy());
}

#[test]
fn resolve_project_file_path_accepts_spaces_and_metacharacters_as_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let nested = tmp.path().join("a dir");
    std::fs::create_dir(&nested).expect("nested dir");
    let file = nested.join("name; still-file.txt");
    std::fs::write(&file, "content").expect("file");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());

    let resolved = crate::app::resolve_project_file_path(&project, "a dir/name; still-file.txt")
        .expect("resolved");

    assert_eq!(resolved, std::fs::canonicalize(file).expect("canonical"));
}

#[test]
fn resolve_project_file_path_rejects_parent_traversal_and_outside_absolute() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::NamedTempFile::new().expect("outside file");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());

    assert_eq!(
        crate::app::resolve_project_file_path(&project, "../outside").unwrap_err(),
        "plain.resolve.file_outside_project"
    );
    assert_eq!(
        crate::app::resolve_project_file_path(&project, outside.path().to_str().unwrap())
            .unwrap_err(),
        "plain.resolve.file_outside_project"
    );
}

#[test]
fn resolve_project_file_path_rejects_symlink_escape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("outside dir");
    let outside_file = outside_dir.path().join("outside.txt");
    std::fs::write(&outside_file, "outside").expect("outside file");
    let link = tmp.path().join("link.txt");
    // RFC-060 D3/Stage 2 R7: this test guards a security property (a
    // project file path must not escape the project through a symlink),
    // so it is ported to Windows rather than gated to Unix-only, the one
    // platform where path handling differs most.
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_file, &link).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&outside_file, &link).expect("symlink");
    let project = make_project_at("svc", tmp.path().to_string_lossy().into_owned());

    assert_eq!(
        crate::app::resolve_project_file_path(&project, "link.txt").unwrap_err(),
        "plain.resolve.file_outside_project"
    );
}

#[test]
fn zero_eligible_fetch_retry_becomes_unavailable_without_task_or_log() {
    let mut state = make_state();
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);
    let missing_id = ProjectId::new();
    let source = make_operation_log(
        OperationKind::Fetch,
        vec![make_operation_result(
            missing_id.clone(),
            ProjectOperationOutcome::Failed,
        )],
    );
    let source_id = source.result.operation_id.clone();
    state.activity.latest = LatestOpState::Completed {
        log: source,
        retry: RetryAvailability::Available(ActivityRetryAction::FetchFailed {
            source_operation_id: source_id.clone(),
            project_ids: vec![missing_id],
        }),
    };

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id,
        }),
    );

    assert!(!state.operation_interlock.is_busy());
    assert!(state.activity.fetch_retry.is_none());
    assert!(state.operation_logs.is_empty());
    assert!(matches!(
        state.activity.latest,
        LatestOpState::Completed {
            retry: RetryAvailability::Unavailable(RetryUnavailableReason::NoEligibleTargets),
            ..
        }
    ));
    assert_eq!(
        state.status_bar.as_deref(),
        Some(state.t("plain.activity.none_available"))
    );
}

#[test]
fn zero_eligible_smart_pull_retry_becomes_unavailable_without_task_or_log() {
    let mut state = make_state();
    install_workspaces(&mut state, vec![Workspace::new("Main")], 0);
    let missing_id = ProjectId::new();
    let source = make_operation_log(
        OperationKind::SmartPull,
        vec![make_operation_result(
            missing_id.clone(),
            ProjectOperationOutcome::Failed,
        )],
    );
    let source_id = source.result.operation_id.clone();
    state.activity.latest = LatestOpState::Completed {
        log: source,
        retry: RetryAvailability::Available(ActivityRetryAction::ReviewSmartPull {
            source_operation_id: source_id.clone(),
            project_ids: vec![missing_id],
        }),
    };

    dispatch(
        &mut state,
        Message::Activity(ActivityMessage::RetryRequested {
            source_operation_id: source_id,
        }),
    );

    assert!(!state.operation_interlock.is_busy());
    assert!(state.sync.retry_preparation.is_none());
    assert!(state.operation_logs.is_empty());
    assert!(matches!(
        state.activity.latest,
        LatestOpState::Completed {
            retry: RetryAvailability::Unavailable(RetryUnavailableReason::NoEligibleTargets),
            ..
        }
    ));
    assert_eq!(state.active_modal, ActiveModal::None);
}

fn install_validating_freezer(state: &mut AppState) -> crate::state::OperationLeaseId {
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::FreezeValidation)
        .expect("acquire validation lease");
    state.active_modal = ActiveModal::Tag;
    state.freezer.phase = FreezerPhase::Validating { lease_id };
    lease_id
}

#[test]
fn freezer_validation_close_cancel_and_escape_release_the_lease() {
    let mut closed = make_state();
    install_validating_freezer(&mut closed);
    dispatch(
        &mut closed,
        Message::Freezer(FreezerMessage::BulkModalClosed),
    );
    assert!(!closed.operation_interlock.is_busy());
    assert_eq!(closed.active_modal, ActiveModal::None);
    assert!(matches!(closed.freezer.phase, FreezerPhase::Idle));

    let mut escaped = make_state();
    install_validating_freezer(&mut escaped);
    dispatch(&mut escaped, Message::Shortcut(ShortcutMessage::Close));
    assert!(!escaped.operation_interlock.is_busy());
    assert_eq!(escaped.active_modal, ActiveModal::None);
    assert!(matches!(escaped.freezer.phase, FreezerPhase::Idle));
}

#[test]
fn freezer_validation_parameter_changes_release_the_lease() {
    let mut renamed = make_state();
    install_validating_freezer(&mut renamed);
    dispatch(
        &mut renamed,
        Message::Freezer(FreezerMessage::NameChanged("v2.0.0".to_owned())),
    );
    assert!(!renamed.operation_interlock.is_busy());
    assert!(matches!(renamed.freezer.phase, FreezerPhase::Idle));
}

#[test]
fn cancelled_freezer_validation_ignores_late_completion() {
    let mut state = make_state();
    let project = make_project("svc");
    let lease_id = install_validating_freezer(&mut state);
    dispatch(
        &mut state,
        Message::Freezer(FreezerMessage::BulkModalClosed),
    );

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeValidationDone {
            lease_id,
            validation: ready_freeze_validation(&project, "v1.0.0"),
        }),
    );

    assert!(matches!(state.freezer.phase, FreezerPhase::Idle));
    assert!(!state.operation_interlock.is_busy());
}

fn install_switching_context(state: &mut AppState) {
    state
        .operation_interlock
        .try_acquire(OperationOwner::ContextSwitch)
        .expect("acquire context lease");
    state.active_modal = ActiveModal::Switch;
    state.context_ops.phase = ContextPhase::Switching {
        target_label: "feature/foo".to_owned(),
    };
}

#[test]
fn context_switch_close_cancel_and_escape_keep_progress_visible() {
    for message in [
        Message::Context(ContextMessage::BulkModalClosed),
        Message::Shortcut(ShortcutMessage::Close),
    ] {
        let mut state = make_state();
        install_switching_context(&mut state);
        dispatch(&mut state, message);
        assert!(state.operation_interlock.is_busy());
        assert_eq!(state.active_modal, ActiveModal::Switch);
        assert!(matches!(
            state.context_ops.phase,
            ContextPhase::Switching { .. }
        ));
    }
}

fn install_running_tag_push(state: &mut AppState) {
    state
        .operation_interlock
        .try_acquire(OperationOwner::TagPush)
        .expect("acquire tag-push lease");
    state.active_modal = ActiveModal::Tag;
    state.pending_tag_push = Some(crate::state::PendingTagPush {
        freeze_name: "v1.0.0".to_owned(),
        project_ids: vec![ProjectId::new()],
        is_pushing: true,
    });
}

#[test]
fn tag_push_close_decline_and_escape_keep_progress_visible() {
    for message in [
        Message::Freezer(FreezerMessage::BulkModalClosed),
        Message::TagPush(TagPushMessage::PushDeclined),
        Message::Shortcut(ShortcutMessage::Close),
    ] {
        let mut state = make_state();
        install_running_tag_push(&mut state);
        dispatch(&mut state, message);
        assert!(state.operation_interlock.is_busy());
        assert_eq!(state.active_modal, ActiveModal::Tag);
        assert!(
            state
                .pending_tag_push
                .as_ref()
                .is_some_and(|push| push.is_pushing)
        );
    }
}

fn install_correlated_smart_pull_preparation(
    state: &mut AppState,
) -> (SmartPullRetryPreparation, Project, Project) {
    let first = make_project("first");
    let second = make_project("second");
    let workspace = Workspace {
        projects: vec![first.clone(), second.clone()],
        ..Workspace::new("Main")
    };
    let workspace_id = workspace.id.clone();
    install_workspaces(state, vec![workspace], 0);
    let source = make_operation_log(
        OperationKind::SmartPull,
        vec![
            make_operation_result(first.id.clone(), ProjectOperationOutcome::Failed),
            make_operation_result(second.id.clone(), ProjectOperationOutcome::Failed),
        ],
    );
    let source_operation_id = source.result.operation_id.clone();
    state.activity.latest = LatestOpState::Completed {
        log: source,
        retry: RetryAvailability::Available(ActivityRetryAction::ReviewSmartPull {
            source_operation_id: source_operation_id.clone(),
            project_ids: vec![first.id.clone(), second.id.clone()],
        }),
    };
    let lease_id = state
        .operation_interlock
        .try_acquire(OperationOwner::ActivitySmartPullPreparation)
        .expect("acquire preparation lease");
    let preparation = SmartPullRetryPreparation {
        id: RetryPreparationId(1),
        workspace_id,
        source_operation_id,
        lease_id,
        eligible_ids: vec![first.id.clone(), second.id.clone()],
        exclusions: Vec::new(),
    };
    state.sync.retry_preparation = Some(preparation.clone());
    state.sync.phase = SyncPhase::RetryPreparing;
    state.active_modal = ActiveModal::Pull;
    (preparation, first, second)
}

fn complete_smart_pull_preparation(
    state: &mut AppState,
    preparation: &SmartPullRetryPreparation,
    statuses: Vec<knotra_vcs::ProjectStatus>,
) {
    dispatch(
        state,
        Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
            request_id: preparation.id,
            workspace_id: preparation.workspace_id.clone(),
            lease_id: preparation.lease_id,
            statuses,
        }),
    );
}

#[test]
fn smart_pull_retry_rejects_duplicate_missing_and_unexpected_status_ids() {
    for case in ["duplicate", "missing", "unexpected"] {
        let mut state = make_state();
        let (preparation, first, second) = install_correlated_smart_pull_preparation(&mut state);
        let statuses = match case {
            "duplicate" => vec![
                make_project_status(first.id.clone(), Some("origin/main")),
                make_project_status(first.id.clone(), Some("origin/main")),
            ],
            "missing" => vec![make_project_status(first.id.clone(), Some("origin/main"))],
            "unexpected" => vec![
                make_project_status(first.id.clone(), Some("origin/main")),
                make_project_status(ProjectId::new(), Some("origin/main")),
            ],
            _ => unreachable!(),
        };
        complete_smart_pull_preparation(&mut state, &preparation, statuses);
        assert!(matches!(
            state.sync.phase,
            SyncPhase::RetryPreparationFailed
        ));
        assert!(state.sync.retry_preparation.is_none());
        assert!(!state.operation_interlock.is_busy());
        assert_ne!(first.id, second.id);
    }
}

#[test]
fn smart_pull_retry_accepts_a_reordered_complete_unique_status_set() {
    let mut state = make_state();
    let (preparation, first, second) = install_correlated_smart_pull_preparation(&mut state);
    complete_smart_pull_preparation(
        &mut state,
        &preparation,
        vec![
            make_project_status(second.id, Some("origin/main")),
            make_project_status(first.id, Some("origin/main")),
        ],
    );

    assert!(matches!(state.sync.phase, SyncPhase::AwaitingConfirm(_)));
    assert!(state.sync.retry_preparation.is_none());
    assert!(!state.operation_interlock.is_busy());
}

#[test]
fn smart_pull_retry_escape_releases_lease_and_ignores_late_completion() {
    let mut state = make_state();
    let (preparation, first, second) = install_correlated_smart_pull_preparation(&mut state);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert_eq!(state.active_modal, ActiveModal::None);
    assert!(matches!(state.sync.phase, SyncPhase::Idle));
    assert!(!state.operation_interlock.is_busy());

    complete_smart_pull_preparation(
        &mut state,
        &preparation,
        vec![
            make_project_status(first.id, Some("origin/main")),
            make_project_status(second.id, Some("origin/main")),
        ],
    );
    assert!(matches!(state.sync.phase, SyncPhase::Idle));
    assert_eq!(state.active_modal, ActiveModal::None);
}

/// RFC-068 D4/D5: `main.rs`'s `APP_ID` (the Wayland app ID / X11 class set
/// on Linux) and `packaging/linux/<APP_ID>.desktop`'s own basename are one
/// constant used in two places — proven here by reading the real file from
/// disk, not by repeating the same literal twice.
#[test]
fn app_id_matches_the_desktop_file_name() {
    let desktop_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packaging/linux")
        .join(format!("{}.desktop", crate::APP_ID));
    assert!(
        desktop_file.is_file(),
        "expected {} to exist, named after APP_ID",
        desktop_file.display()
    );
}

// -----------------------------------------------------------------------
// RFC-073 Amendment A5 §2: "Operation log size" takes effect at once.
// -----------------------------------------------------------------------

/// A minimal `OperationLog` at an explicit second — `make_operation_log`'s
/// own `Timestamp::now()` cannot give these tests the distinct, orderable
/// instants a newest-first merge needs to prove anything.
fn make_operation_log_at(seconds: i64) -> OperationLog {
    let ts = Timestamp::from_second(seconds).unwrap();
    OperationLog {
        result: OperationResult {
            operation_id: OperationId::new(),
            kind: OperationKind::Fetch,
            started_at: ts,
            finished_at: ts,
            per_project: Vec::new(),
            rollback_attempted: false,
            rollback_succeeded: None,
        },
        recovery_hints: Vec::new(),
    }
}

/// Lowering truncates `operation_logs` at once — no new operation needs to
/// complete first — and drops the truncated-away ids from
/// `history_expanded`, the one place in `knotra-app` that refers to an
/// entry by id outside the list itself.
#[test]
fn lowering_max_log_entries_truncates_at_once_and_drops_expanded_ids() {
    let mut state = make_state();
    let logs: Vec<OperationLog> = (0..5).map(|i| make_operation_log_at(1000 + i)).collect();
    let kept_id = logs[0].result.operation_id.clone();
    let dropped_id = logs[4].result.operation_id.clone();
    state.history_expanded.insert(kept_id.clone());
    state.history_expanded.insert(dropped_id.clone());
    state.operation_logs = logs;

    dispatch(
        &mut state,
        Message::Settings(SettingsMessage::MaxLogEntriesChanged(2)),
    );

    assert_eq!(state.operation_logs.len(), 2);
    assert!(
        state.history_expanded.contains(&kept_id),
        "an id still present in the list must stay expanded"
    );
    assert!(
        !state.history_expanded.contains(&dropped_id),
        "a truncated-away id must not linger in history_expanded"
    );
}

/// Raising re-reads from disk (`BackgroundMessage::HistoryLogsReloaded`,
/// driven directly — this is the merge logic's own test, not a test of
/// `tokio::task::spawn_blocking` plumbing the code itself already shows is
/// a plain, trusted wrapper around `load_recent_logs`) and merges the
/// result with whatever is already in memory, newest first, by
/// `operation_id`, truncated to the new limit. An operation that finishes
/// while the read is running — modelled here as an entry already in
/// `state.operation_logs` that the simulated disk read does not carry —
/// must not be lost, and must not be duplicated if the disk read also
/// names its id.
#[test]
fn raising_max_log_entries_merges_disk_and_in_memory_losing_nothing() {
    let mut state = make_state();
    state.config.max_log_entries = 5;

    let older_on_disk_only = make_operation_log_at(1000);
    let shared_both_copies_in_memory = make_operation_log_at(2000);
    let shared_both_copies_on_disk = {
        // The disk's own copy of the same operation — same id, so the
        // merge must keep exactly one, not both.
        let mut log = shared_both_copies_in_memory.clone();
        log.result.started_at = Timestamp::from_second(1999).unwrap();
        log
    };
    let finished_during_the_read = make_operation_log_at(3000);

    // In memory: only the entry that finished *during* the simulated read,
    // plus the one copy the two share — the state the previous, smaller
    // `max_log_entries` would have left it in.
    state.operation_logs = vec![
        finished_during_the_read.clone(),
        shared_both_copies_in_memory.clone(),
    ];

    let loaded = crate::persistence::LoadedLogs {
        logs: vec![shared_both_copies_on_disk, older_on_disk_only.clone()],
        unreadable: 0,
        directory_unreadable: false,
    };

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::HistoryLogsReloaded(loaded)),
    );

    let ids: Vec<_> = state
        .operation_logs
        .iter()
        .map(|l| l.result.operation_id.clone())
        .collect();
    assert_eq!(state.operation_logs.len(), 3, "{ids:?}");
    assert_eq!(
        ids,
        vec![
            finished_during_the_read.result.operation_id.clone(),
            shared_both_copies_in_memory.result.operation_id.clone(),
            older_on_disk_only.result.operation_id.clone(),
        ],
        "newest first; the in-memory copy of the shared id wins, no duplicate"
    );
    // The surviving copy of the shared id is the in-memory one (started_at
    // 2000), not the disk one (1999) the read raced against.
    assert_eq!(
        state.operation_logs[1].result.started_at,
        shared_both_copies_in_memory.result.started_at
    );
}
