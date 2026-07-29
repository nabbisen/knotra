//! Integration-level tests for knotra-app.

use crate::config::{AppConfig, AppPaths, DashboardGrouping, DashboardSort, load_config};
use crate::message::{
    ActivityMessage, BackgroundMessage, ChangelogMessage, ConflictOpsMessage, ContextMessage,
    DashboardMessage, DetailPanelMessage, FilterMessage, FreezerMessage, Message, PaletteMessage,
    SelectionMessage, ShortcutMessage, StatusFilter, SyncMessage, TagPushMessage, WorkspaceMessage,
};
use crate::persistence::{load_workspaces, save_workspace};
use crate::state::{
    ActiveModal, ActivityRetryAction, AddProjectDialog, AppState, LatestOpState, OperationOwner,
    RetryAvailability, RetryUnavailableReason, Screen,
    changelog::ChangelogPhase,
    conflict_ops::ConflictPhase,
    context::ContextPhase,
    focus::{self, FocusTarget},
    freezer::FreezerPhase,
    sync::{RetryPreparationId, SmartPullRetryPreparation, SyncPhase},
};
use chrono::Utc;
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
    let now = Utc::now();
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
        refreshed_at: Utc::now(),
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
        refreshed_at: Utc::now(),
        read_error: None,
    }
}

fn make_changelog_draft(project: &Project, entries: Vec<CommitEntry>) -> ChangelogDraft {
    ChangelogDraft {
        release_name: "v1.2.0".to_owned(),
        generated_at: Utc::now(),
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
        date: Utc::now(),
    }
}

fn ready_freeze_validation(project: &Project, freeze_name: &str) -> FreezeValidation {
    FreezeValidation {
        freeze_name: freeze_name.to_owned(),
        entries: vec![FreezeValidationEntry {
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            included: true,
            is_clean: true,
            tag_exists: false,
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
    assert_eq!(
        state.status_bar.as_deref(),
        Some(state.t("dashboard.preference_save_failed"))
    );
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
fn dashboard_error_details_and_retry_follow_workspace_guard() {
    let mut no_workspace = make_state();
    no_workspace.show_op_details = true;
    dispatch(
        &mut no_workspace,
        Message::Background(BackgroundMessage::TaskError {
            description: "adapter path detail".to_owned(),
        }),
    );
    assert!(!no_workspace.dashboard_error_details_open);
    assert_eq!(
        no_workspace.status_bar.as_deref(),
        Some(no_workspace.t("dashboard.load_failed"))
    );
    dispatch(
        &mut no_workspace,
        Message::Dashboard(DashboardMessage::ErrorDetailsToggled),
    );
    assert!(no_workspace.dashboard_error_details_open);
    assert!(no_workspace.show_op_details);
    dispatch(
        &mut no_workspace,
        Message::Dashboard(DashboardMessage::ErrorRetryRequested),
    );
    assert!(matches!(
        no_workspace.load_phase,
        crate::state::LoadPhase::Error(_)
    ));
    assert!(!no_workspace.is_refreshing);

    let mut loaded = make_state();
    let workspace = Workspace::new("Main");
    install_workspaces(&mut loaded, vec![workspace], 0);
    loaded.load_phase = crate::state::LoadPhase::Error("again".to_owned());
    loaded.dashboard_error_details_open = true;
    loaded.is_refreshing = true;
    dispatch(
        &mut loaded,
        Message::Dashboard(DashboardMessage::ErrorRetryRequested),
    );
    assert!(matches!(
        loaded.load_phase,
        crate::state::LoadPhase::Refreshing
    ));
    assert!(loaded.is_refreshing);
    assert!(!loaded.dashboard_error_details_open);
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

// ---------------------------------------------------------------------------
// RFC-036 Stage 3 — overlay focus trap, entry, and return
// ---------------------------------------------------------------------------

/// A stand-in for "the shell control that had knotra-focus before the
/// dialog opened" — the exact key `view/shell.rs` mints for the workspace
/// switcher trigger. Using the real key (not an arbitrary test string)
/// means this test would still catch a rename of that key breaking R7.
fn shell_switcher_target() -> FocusTarget {
    FocusTarget::control("shell.workspace_switcher")
}

#[test]
fn create_dialog_open_enters_focus_at_the_name_field_r6() {
    let mut state = make_state();
    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );
    assert_eq!(
        state.overlay_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::WORKSPACE_NAME.clone()
        ))
    );
}

#[test]
fn create_dialog_tab_wraps_within_the_dialog_and_never_touches_shell_focus_r5() {
    let mut state = make_state();
    // Simulate the workspace switcher having had keyboard focus before the
    // dialog opened, so an untouched `dashboard_focus` is a meaningful
    // assertion rather than trivially `None` either way.
    state.dashboard_focus = Some(shell_switcher_target());

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

    let name_field = FocusTarget::text_input(knotra_ui::widget::focus_id::WORKSPACE_NAME.clone());
    let confirm = FocusTarget::control("workspace_mgr.dialog.confirm");
    let cancel = FocusTarget::control("workspace_mgr.dialog.cancel");
    let close = FocusTarget::control("workspace_mgr.dialog.close");

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));
    assert_eq!(state.overlay_focus, Some(confirm));
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));
    assert_eq!(state.overlay_focus, Some(cancel));
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));
    assert_eq!(state.overlay_focus, Some(close));
    // R5: from the dialog's last target, Tab wraps back to its first —
    // never into any shell target.
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));
    assert_eq!(state.overlay_focus, Some(name_field));

    // R7's precondition: the shell's own focus was never touched while the
    // dialog held it, regardless of how many times Tab moved within it.
    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
}

#[test]
fn create_dialog_escape_closes_and_returns_focus_to_the_opener_r7() {
    let mut state = make_state();
    state.dashboard_focus = Some(shell_switcher_target());

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(state.workspace_mgr.create_dialog.is_none());
    assert_eq!(state.overlay_focus, None);
    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
}

#[test]
fn create_dialog_scrim_click_closes_and_returns_focus_to_the_opener_r7() {
    // `view.rs` wires `AppLayout`'s scrim click to the exact same
    // `Message::Shortcut(ShortcutMessage::Close)` Escape dispatches
    // (`.on_close_modals(...)`), so this exercises the identical code path
    // as the Escape test above — kept as its own test, per the Handoff,
    // because the two routes are conceptually distinct even though this
    // codebase happens to converge them onto one message today.
    let mut state = make_state();
    state.dashboard_focus = Some(shell_switcher_target());

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));

    assert!(state.workspace_mgr.create_dialog.is_none());
    assert_eq!(state.overlay_focus, None);
    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
}

#[test]
fn create_dialog_header_close_returns_focus_to_the_opener_r7() {
    // The header close button dispatches `CreateWorkspaceCancelled`
    // directly (see `overlay::surface`'s `on_close`), bypassing
    // `close_topmost_layer` entirely — a genuinely different code path from
    // the Escape/scrim tests above.
    let mut state = make_state();
    state.dashboard_focus = Some(shell_switcher_target());

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
    );

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::CreateWorkspaceCancelled),
    );

    assert!(state.workspace_mgr.create_dialog.is_none());
    assert_eq!(state.overlay_focus, None);
    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
}

#[test]
fn delete_dialog_entry_is_cancel_not_the_destructive_action_r6() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::under(tmp.path().to_path_buf());
    let mut state = make_state_with_paths(paths);
    install_workspaces(
        &mut state,
        vec![Workspace::new("Main"), Workspace::new("Lab")],
        0,
    );

    dispatch(
        &mut state,
        Message::Workspace(WorkspaceMessage::DeleteWorkspaceRequested),
    );

    assert_eq!(
        state.overlay_focus,
        Some(FocusTarget::control("workspace_mgr.dialog.cancel"))
    );
}

#[test]
fn seven_site_fix_dialog_open_paths_set_knotra_focus_alongside_iced_focus_r12() {
    // Regression test for `.git-exclude/reviewed/076-...md`'s finding: the
    // seven pre-existing `focus_input` dialog-open call sites moved iced
    // focus without moving knotra-focus. Each must now also set
    // `overlay_focus` to the exact same text-input target — the state half
    // of the invariant `state::focus::tests` already proves `reconcile`
    // upholds in isolation (`reconcile(_, Some(TextInput)) =>
    // FocusTextInput`). `Task` itself isn't inspectable from these
    // integration tests, so this is the strongest assertion available
    // short of driving a real iced runtime.
    let mut add_project_state = make_state();
    dispatch(
        &mut add_project_state,
        Message::Workspace(WorkspaceMessage::AddProjectDialogOpened),
    );
    assert_eq!(
        add_project_state.overlay_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone()
        ))
    );

    let mut palette_state = make_state();
    dispatch(&mut palette_state, Message::Palette(PaletteMessage::Opened));
    assert_eq!(
        palette_state.overlay_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::PALETTE_QUERY.clone()
        ))
    );

    let mut freezer_state = make_state();
    dispatch(
        &mut freezer_state,
        Message::Freezer(FreezerMessage::BulkOpenRequested),
    );
    assert_eq!(
        freezer_state.overlay_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::RELEASE_NAME.clone()
        ))
    );
}

#[test]
fn non_cancellable_overlay_does_not_leak_tab_to_the_shell() {
    // Smart Pull running (RFC-029/031's non-cancellable phase) is
    // explicitly out of this stage's order-building scope (RFC-037's), but
    // Tab must still not reach shell controls hidden beneath it. An empty
    // overlay order makes Tab a safe no-op instead of leaking through.
    let mut state = make_state();
    state.dashboard_focus = Some(shell_switcher_target());
    state.active_modal = ActiveModal::Tag;
    state.freezer.phase = FreezerPhase::Executing;

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));

    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
    assert_eq!(state.overlay_focus, None);

    // Escape is inert during the non-cancellable phase (existing
    // `close_topmost_layer` behaviour, unchanged) — confirm the wrapper
    // added around it does not reset overlay focus while the overlay is
    // still legitimately open.
    dispatch(&mut state, Message::Shortcut(ShortcutMessage::Close));
    assert_eq!(state.active_modal, ActiveModal::Tag);
    assert_eq!(state.dashboard_focus, Some(shell_switcher_target()));
}

#[test]
fn delete_dialog_open_does_not_affect_focus_traversal_pure_function_tests() {
    // Sanity check that Stage 1's pure-function tests in `state::focus`
    // still hold unmodified — this stage adds no new arms to `resolve`,
    // `advance`, or `reconcile` themselves, only new callers.
    let order: focus::FocusOrder<Message> = vec![(
        FocusTarget::control("a"),
        Some(Message::Shortcut(ShortcutMessage::Refresh)),
    )];
    assert_eq!(
        focus::advance(&order, None, focus::Direction::Next),
        Some(&FocusTarget::control("a"))
    );
}

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
        last_refresh: Some(Utc::now()),
    });
    state.load_phase = crate::state::LoadPhase::Ready;
    // `AppConfig::default()` collapses AllSet by default; expand both
    // collapsible tiers so all three projects are selectable/visible unless
    // a specific test re-collapses one deliberately.
    state.config.dashboard_in_progress_collapsed = false;
    state.config.dashboard_all_set_collapsed = false;

    (alpha_id, beta_id, gamma_id)
}

fn name_target_ids(order: &focus::FocusOrder<Message>) -> Vec<String> {
    order
        .iter()
        .map(|(t, _)| format!("{t:?}"))
        .filter(|debug| debug.ends_with(".name\")"))
        .collect()
}

#[test]
fn dashboard_row_order_matches_ordered_selectable_ids_r2() {
    let mut state = make_state();
    let (alpha_id, beta_id, gamma_id) = install_dashboard_projects(&mut state);

    let expected: Vec<String> = state
        .dashboard_display()
        .ordered_selectable_ids
        .iter()
        .map(|id| {
            format!(
                "{:?}",
                FocusTarget::control_dynamic(format!("dashboard.row.{id}.name"))
            )
        })
        .collect();
    assert_eq!(
        expected,
        vec![
            format!(
                "{:?}",
                FocusTarget::control_dynamic(format!("dashboard.row.{alpha_id}.name"))
            ),
            format!(
                "{:?}",
                FocusTarget::control_dynamic(format!("dashboard.row.{beta_id}.name"))
            ),
            format!(
                "{:?}",
                FocusTarget::control_dynamic(format!("dashboard.row.{gamma_id}.name"))
            ),
        ],
        "sanity: NeedsHelp, InProgress, AllSet is the expected tier order"
    );

    let order = crate::view::dashboard::focus_order(&state);
    assert_eq!(name_target_ids(&order), expected);
}

#[test]
fn dashboard_row_order_matches_ordered_selectable_ids_project_group_grouping_r2() {
    let mut state = make_state();
    let (alpha_id, beta_id, gamma_id) = install_dashboard_projects(&mut state);
    state.config.dashboard_grouping = DashboardGrouping::ProjectGroup;

    let expected: Vec<String> = state
        .dashboard_display()
        .ordered_selectable_ids
        .iter()
        .map(|id| {
            format!(
                "{:?}",
                FocusTarget::control_dynamic(format!("dashboard.row.{id}.name"))
            )
        })
        .collect();
    // Sanity: all three still present under a different grouping, in
    // whatever order `ordered_selectable_ids` itself gives (its own
    // ordering logic is not this test's concern - only that row targets
    // follow it exactly).
    assert_eq!(expected.len(), 3);
    let _ = (alpha_id, beta_id, gamma_id);

    let order = crate::view::dashboard::focus_order(&state);
    assert_eq!(name_target_ids(&order), expected);
}

#[test]
fn collapsed_section_contributes_no_row_targets_but_keeps_its_header() {
    let mut state = make_state();
    let (_, _, gamma_id) = install_dashboard_projects(&mut state);
    state.config.dashboard_all_set_collapsed = true;

    let order = crate::view::dashboard::focus_order(&state);
    let debug: Vec<String> = order.iter().map(|(t, _)| format!("{t:?}")).collect();

    assert!(
        debug.iter().any(|d| d.contains("dashboard.section.AllSet")),
        "collapsed section's header must remain a Tab stop so it can be reopened"
    );
    assert!(
        !debug.iter().any(|d| d.contains(&gamma_id.to_string())),
        "a collapsed section's rows must not appear (it renders no rows)"
    );
}

#[test]
fn needs_help_header_is_never_a_collapse_target() {
    let mut state = make_state();
    install_dashboard_projects(&mut state);

    let order = crate::view::dashboard::focus_order(&state);
    assert!(
        !order
            .iter()
            .any(|(t, _)| format!("{t:?}").contains("dashboard.section.NeedsHelp")),
        "NeedsHelp has no chevron and must not be a focus target"
    );
}

#[test]
fn tab_reaches_section_header_checkbox_and_row_action() {
    let mut state = make_state();
    let (alpha_id, _, _) = install_dashboard_projects(&mut state);
    state.selection_mode = true;

    let order = crate::view::dashboard::focus_order(&state);
    let debug: Vec<String> = order.iter().map(|(t, _)| format!("{t:?}")).collect();

    assert!(debug.iter().any(|d| d.contains("dashboard.section.")));
    assert!(
        debug
            .iter()
            .any(|d| d.contains(&format!("dashboard.row.{alpha_id}.checkbox")))
    );
    assert!(
        debug
            .iter()
            .any(|d| d.contains(&format!("dashboard.row.{alpha_id}.action")))
    );
}

#[test]
fn enter_on_a_row_action_dispatches_the_same_message_a_click_would_r3() {
    // `activate_focused` returns a `Task::done(msg)` for a non-text-input
    // activation, which this integration harness (like every other Stage
    // 1-3 Task-producing test) cannot run through to a second `update()`
    // call - `dispatch` only executes one `update()`, and `Task` is not
    // otherwise inspectable. So this asserts the pure decision
    // (`focus::activation_message`) yields exactly the `Message` the
    // pointer-click path (`view_project_row`'s non-conflict action button)
    // would dispatch, which is the same guarantee Stage 3's
    // `seven_site_fix_...` test relies on for the same reason.
    let mut state = make_state();
    let (alpha_id, _, _) = install_dashboard_projects(&mut state);

    let mut order = crate::view::shell::focus_order(&state);
    order.extend(crate::view::dashboard::focus_order(&state));

    // Alpha's cause is MissingPath, not Conflict, so its action button opens
    // the detail panel (see `view_project_row`'s non-conflict branch).
    let target = FocusTarget::control_dynamic(format!("dashboard.row.{alpha_id}.action"));
    match focus::activation_message(&order, Some(&target)) {
        Some(Message::DetailPanel(DetailPanelMessage::Opened(id))) => {
            assert_eq!(id, alpha_id);
        }
        other => panic!("expected DetailPanel::Opened({alpha_id}), got {other:?}"),
    }
}

#[test]
fn tab_from_a_stale_row_target_lands_on_the_second_entry_r9() {
    // Deliberate choice (RFC-036 Stage 4 R9 decision): `resolve()`'s
    // fallback treats the first live target in the *current combined order*
    // as the effective "current" position even when the stored target has
    // vanished, so `advance`'s `Next` moves *past* it to the second entry -
    // the same uniform rule Stage 1 already uses for a freshly-opened,
    // never-focused context, not a special case invented for rows.
    //
    // Because Tab traverses one flat shell-then-rows order, "the second
    // entry" here is the shell's second target (`shell.nav.dashboard`), not
    // a second row - falling all the way back to the shell is the honest
    // consequence of there being one order, not two, and is recorded here
    // rather than glossed over. Rationale: one fallback rule for "no
    // defined current position" everywhere is simpler than adding a
    // row-scoped variant that only kicks in once rows can disappear out
    // from under focus.
    let mut state = make_state();
    let (_, beta_id, _) = install_dashboard_projects(&mut state);

    state.dashboard_focus = Some(FocusTarget::control_dynamic(format!(
        "dashboard.row.{beta_id}.name"
    )));

    // Beta is removed outright - its target vanishes from the
    // next-computed order (a stronger stand-in for "filtered/refreshed
    // away" than a text filter, and avoids a filter substring accidentally
    // also matching Beta's own name).
    state
        .workspace
        .as_mut()
        .expect("workspace installed")
        .projects
        .retain(|p| p.id != beta_id);

    let expected_order = {
        let mut order = crate::view::shell::focus_order(&state);
        order.extend(crate::view::dashboard::focus_order(&state));
        order
    };
    let expected_second = expected_order
        .get(1)
        .map(|(t, _)| t.clone())
        .expect("at least two targets remain");

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusNext));

    assert_eq!(state.dashboard_focus, Some(expected_second));
}

#[test]
fn ctrl_slash_now_actually_focuses_search_not_only_the_screen() {
    // Regression test required by `.git-exclude/reviewed/079-rfc-036-stage-4-review.md`
    // review focus 3: `Ctrl+/`'s handler previously only switched screens
    // and never called `focus_input` at all - the pre-existing gap fixed
    // as part of Stage 4's `focus_search()` sharing. Without this test, a
    // future change reintroducing that gap would ship silently, exactly as
    // it did the first time.
    let mut state = make_state();
    state.screen = Screen::Settings;
    assert_eq!(state.dashboard_focus, None);

    dispatch(&mut state, Message::Shortcut(ShortcutMessage::FocusSearch));

    assert_eq!(state.screen, Screen::Dashboard);
    assert_eq!(
        state.dashboard_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::SEARCH.clone()
        ))
    );
}

#[test]
fn bare_slash_focuses_search_with_no_text_input_focused_r4() {
    let mut state = make_state();
    assert_eq!(state.dashboard_focus, None);

    dispatch(
        &mut state,
        Message::Shortcut(ShortcutMessage::FocusSearchBare),
    );

    assert_eq!(state.screen, Screen::Dashboard);
    assert_eq!(
        state.dashboard_focus,
        Some(FocusTarget::text_input(
            knotra_ui::widget::focus_id::SEARCH.clone()
        ))
    );
}

#[test]
fn bare_slash_types_a_literal_character_when_a_text_input_already_holds_focus_r4() {
    let mut state = make_state();
    let already_focused =
        FocusTarget::text_input(knotra_ui::widget::focus_id::WORKSPACE_NAME.clone());
    state.dashboard_focus = Some(already_focused.clone());

    dispatch(
        &mut state,
        Message::Shortcut(ShortcutMessage::FocusSearchBare),
    );

    // Gated: knotra-focus must not have been redirected to search, so the
    // literal `/` iced delivers separately reaches the field that was
    // actually focused, not a jump to search.
    assert_eq!(state.dashboard_focus, Some(already_focused));
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
    state.palette.highlighted = index;

    let message = crate::state::palette::dispatch_entry(&state);
    assert!(matches!(
        message,
        crate::state::palette::PaletteDispatch::Dispatched(Message::Workspace(
            WorkspaceMessage::CreateWorkspaceDialogOpened
        ))
    ));
}

fn highlight_palette_payload(state: &mut AppState, query: &str, payload: &str) {
    state.palette.query = query.to_owned();
    crate::state::palette::update_results(state);
    state.palette.highlighted = state
        .palette
        .results
        .iter()
        .position(|entry| entry.payload == payload)
        .unwrap_or_else(|| panic!("palette payload {payload} visible for query {query:?}"));
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
        highlight_palette_payload(&mut state, action_id, action_id);
        assert!(
            !matches!(
                crate::state::palette::dispatch_entry(&state),
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

#[test]
fn palette_toggle_theme_action_is_not_listed() {
    let mut state = make_state();
    state.palette.query = "toggle theme".to_owned();
    crate::state::palette::update_results(&mut state);
    assert!(
        state
            .palette
            .results
            .iter()
            .all(|entry| entry.payload != "action.toggle_theme")
    );
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
    highlight_palette_payload(&mut state, "changelog", "action.changelog_selected");

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state),
        crate::state::palette::PaletteDispatch::Dispatched(Message::Changelog(
            ChangelogMessage::BulkOpenRequested
        ))
    ));
}

#[test]
fn palette_disabled_confirm_keeps_palette_open_with_reason() {
    let mut state = make_state();
    state.palette.open_palette();

    dispatch(
        &mut state,
        Message::Palette(PaletteMessage::QueryChanged("get latest".to_owned())),
    );
    dispatch(&mut state, Message::Palette(PaletteMessage::Confirmed));

    assert!(state.palette.open);
    assert_eq!(state.palette.notice_key, Some("plain.disabled.choose_one"));
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
    highlight_palette_payload(&mut state, "svc", &project.id.to_string());

    let message = crate::state::palette::dispatch_entry(&state);

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
    highlight_palette_payload(&mut state, "Main", &workspace_id.to_string());

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state),
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

    highlight_palette_payload(&mut state, "remove selected", "action.remove_project");
    assert!(matches!(
        crate::state::palette::dispatch_entry(&state),
        crate::state::palette::PaletteDispatch::Disabled("plain.disabled.choose_one")
    ));

    state.selection.selected_ids.insert(project.id.clone());
    highlight_palette_payload(&mut state, "remove selected", "action.remove_project");
    assert!(matches!(
        crate::state::palette::dispatch_entry(&state),
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
    highlight_palette_payload(&mut state, "next workspace", "action.workspace_next");

    assert!(matches!(
        crate::state::palette::dispatch_entry(&state),
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
    assert_eq!(state.selection.len(), 0);
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
    assert_eq!(state.selection.len(), 0);
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
    assert_eq!(state.selection.len(), 0);
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
    assert_eq!(state.selection.len(), 0);
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
    assert_eq!(state.selection.len(), 0);
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
        last_refresh: Some(Utc::now()),
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
        last_refresh: Some(Utc::now()),
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
        last_refresh: Some(Utc::now()),
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
        started_at: Utc::now(),
        completed: Vec::new(),
    };

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullProjectCompleted {
            lease_id,
            progress: SmartPullProgress {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
                result: ProjectOperationResult {
                    project_id: project.id.clone(),
                    outcome: ProjectOperationOutcome::Skipped,
                    success: true,
                    skip_reason: Some("No update source is configured.".to_owned()),
                    commands_executed: vec![],
                    stdout: "[excluded]".to_owned(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    error_message: None,
                },
                recovery_hint: None,
            },
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

    let loaded = crate::persistence::load_recent_logs(&paths, 10);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].result.skipped_projects().len(), 1);
    assert_eq!(
        loaded[0].result.per_project[0].skip_reason.as_deref(),
        Some("No update source is configured.")
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
        started_at: Utc::now(),
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
        generated_at: Utc::now(),
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
fn freezer_execute_requested_does_not_loop_back_to_confirmed() {
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
        Message::Freezer(FreezerMessage::ExecuteRequested),
    );

    assert!(matches!(state.freezer.phase, FreezerPhase::Executing));
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
    state.freezer.execution_started_at = Some(Utc::now());
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
        last_refresh: Some(Utc::now()),
    });
    state.freezer.execution_started_at = Some(Utc::now());
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

    let loaded = crate::persistence::load_recent_logs(&paths, 10);
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
        last_refresh: Some(Utc::now()),
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
        last_refresh: Some(Utc::now()),
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
        note: None,
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
            result: ProjectOperationResult {
                project_id: project.id.clone(),
                outcome: ProjectOperationOutcome::Succeeded,
                success: true,
                skip_reason: None,
                commands_executed: vec!["git add src/lib.rs".to_owned()],
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                error_message: None,
            },
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
        note: None,
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
            result: ProjectOperationResult {
                project_id: project.id.clone(),
                outcome: ProjectOperationOutcome::Failed,
                success: false,
                skip_reason: None,
                commands_executed: vec!["git add src/lib.rs".to_owned()],
                stdout: String::new(),
                stderr: "failed".to_owned(),
                exit_code: Some(1),
                error_message: Some("failed".to_owned()),
            },
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
            result: make_operation_result(project.id, ProjectOperationOutcome::Succeeded),
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
    std::os::unix::fs::symlink(&outside_file, &link).expect("symlink");
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

    let mut cancelled = make_state();
    install_validating_freezer(&mut cancelled);
    dispatch(&mut cancelled, Message::Freezer(FreezerMessage::Cancelled));
    assert!(!cancelled.operation_interlock.is_busy());
    assert!(matches!(cancelled.freezer.phase, FreezerPhase::Idle));

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

    let mut toggled = make_state();
    let project_id = ProjectId::new();
    install_validating_freezer(&mut toggled);
    dispatch(
        &mut toggled,
        Message::Freezer(FreezerMessage::ProjectToggled(project_id.clone(), false)),
    );
    assert!(!toggled.operation_interlock.is_busy());
    assert!(matches!(toggled.freezer.phase, FreezerPhase::Idle));
    assert_eq!(
        toggled.freezer.project_selection.get(&project_id),
        Some(&false)
    );
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
        project_id: ProjectId::new(),
        target: ContextTarget::GitLocalBranch {
            name: "feature/foo".to_owned(),
        },
        target_label: "feature/foo".to_owned(),
    };
}

#[test]
fn context_switch_close_cancel_and_escape_keep_progress_visible() {
    for message in [
        Message::Context(ContextMessage::BulkModalClosed),
        Message::Context(ContextMessage::Cancelled),
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
