//! Integration-level tests for knotra-app.

use crate::config::AppConfig;
use crate::config::AppPaths;
use crate::message::{
    BackgroundMessage, ChangelogMessage, ConflictOpsMessage, ContextMessage, DetailPanelMessage,
    FilterMessage, FreezerMessage, Message, PaletteMessage, SelectionMessage, ShortcutMessage,
    SyncMessage, TagPushMessage, WorkspaceMessage,
};
use crate::persistence::{load_workspaces, save_workspace};
use crate::state::{
    ActiveModal, AddProjectDialog, AppState, Screen, changelog::ChangelogPhase,
    conflict_ops::ConflictPhase, context::ContextPhase, freezer::FreezerPhase, sync::SyncPhase,
};
use chrono::Utc;
use knotra_vcs::{
    ChangelogDraft, CommitEntry, ConflictMarker, ConflictedFile, ContextTarget, OperationId,
    Project, ProjectCommits, ProjectConflictDetail, ProjectId, Workspace, WorkspaceStatus,
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

fn make_project_at(name: &str, path: impl Into<String>) -> Project {
    Project::new(name, path)
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

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ConflictOperationCompleted {
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

    dispatch(
        &mut state,
        Message::Background(BackgroundMessage::ConflictOperationCompleted {
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
