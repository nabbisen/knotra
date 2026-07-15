//! Integration-level tests for knotra-app.

use crate::config::AppConfig;
use crate::config::AppPaths;
use crate::message::{
    BackgroundMessage, FilterMessage, FreezerMessage, Message, ShortcutMessage, SyncMessage,
    TagPushMessage, WorkspaceMessage,
};
use crate::persistence::{load_workspaces, save_workspace};
use crate::state::{
    ActiveModal, AddProjectDialog, AppState, Screen, freezer::FreezerPhase, sync::SyncPhase,
};
use chrono::Utc;
use knotra_vcs::{
    OperationId, Project, ProjectId, Workspace, WorkspaceStatus,
    model::{
        operation::{
            FreezeOutcome, FreezeProjectResult, FreezeResult, FreezeValidation,
            FreezeValidationEntry, OperationKind, ProjectOperationOutcome, ProjectOperationResult,
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
        Some(Message::Workspace(
            WorkspaceMessage::CreateWorkspaceDialogOpened
        ))
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
    state.sync.phase = SyncPhase::PullRunning {
        plan,
        started_at: Utc::now(),
        completed: Vec::new(),
    };

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::SmartPullProjectCompleted(
            SmartPullProgress {
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
        )),
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

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone(FreezeResult {
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
        })),
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

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone(FreezeResult {
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
        })),
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

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::FreezeExecutionDone(FreezeResult {
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
        })),
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
