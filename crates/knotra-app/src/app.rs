//! Top-level Elm-architecture implementation for knotra.
//!
//! `init`   → produces initial `AppState` + startup `Task`
//! `update` → `Message → Task<Message>`, mutates `AppState`
//! `view`   → `&AppState → Element<Message>`
//! `subscription` → periodic tick + keyboard shortcuts

use iced::{keyboard, time, Element, Subscription, Task};
use std::time::Duration;

use endringer::{
    model::{
        operation::{OperationId, OperationKind, OperationLog, OperationResult},
        project::Project,
        workspace::Workspace,
    },
    VcsAdapter,
};

use crate::{
    config::{load_config, save_config, AppPaths},
    message::{
        BackgroundMessage, FilterMessage, FreezerMessage, HistoryMessage, Message,
        ProjectMessage, SettingsMessage, ShortcutMessage, SyncMessage, WorkspaceMessage,
    },
    persistence::{load_recent_logs, load_workspaces, save_operation_log, save_workspace},
    state::{AddProjectDialog, AppState, ConfirmRemoveDialog, LoadPhase, Screen},
    view::app_view,
};

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn init() -> (AppState, Task<Message>) {
    let paths = AppPaths::resolve();
    let (config, config_err) = load_config(&paths);
    let mut state = AppState::new(config.clone());

    if let Some(err) = config_err {
        state.status_bar = Some(err);
    }

    let (workspaces, ws_errors) = load_workspaces(&paths);
    for e in &ws_errors {
        tracing::warn!("workspace load error: {e}");
    }

    let workspace = workspaces
        .into_iter()
        .next()
        .unwrap_or_else(|| Workspace::new("My Workspace"));
    state.workspace = Some(workspace);
    state.load_phase = LoadPhase::Refreshing;
    state.is_refreshing = true;

    state.operation_logs = load_recent_logs(&paths, config.max_log_entries);

    let task = refresh_workspace_task(&state);
    (state, task)
}

// ---------------------------------------------------------------------------
// Subscription — periodic tick + keyboard shortcuts
// ---------------------------------------------------------------------------

pub fn subscription(state: &AppState) -> Subscription<Message> {
    let interval_secs = state.config.refresh_interval_secs;

    let tick_sub = if interval_secs > 0 {
        time::every(Duration::from_secs(u64::from(interval_secs)))
            .map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    let keyboard_sub = keyboard::listen().map(|event| {
        use keyboard::key::Named;
        use keyboard::Event;
        if let Event::KeyPressed { key, modifiers, .. } = event {
            let ctrl = modifiers.control() || modifiers.command();
            let shortcut = match &key {
                keyboard::Key::Named(Named::Escape) => Some(ShortcutMessage::Close),
                keyboard::Key::Character(c) => match c.as_str() {
                    "r" | "R" if ctrl => Some(ShortcutMessage::Refresh),
                    "k" | "K" if ctrl => Some(ShortcutMessage::OpenContextOps),
                    "t" | "T" if ctrl => Some(ShortcutMessage::OpenFreezer),
                    "/" if ctrl       => Some(ShortcutMessage::FocusSearch),
                    _ => None,
                },
                _ => None,
            };
            if let Some(s) = shortcut { return Message::Shortcut(s); }
        }
        // Non-shortcut key events — map to a no-op (Tick is ignored when already refreshing).
        Message::Tick
    });

    Subscription::batch([tick_sub, keyboard_sub])
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Navigate(screen) => {
            state.screen = screen;
            Task::none()
        }
        Message::Tick => {
            // Only refresh on tick if not already refreshing.
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                refresh_workspace_task(state)
            } else {
                Task::none()
            }
        }
        Message::Shortcut(msg)   => handle_shortcut(state, msg),
        Message::Workspace(msg)  => handle_workspace(state, msg),
        Message::Project(msg)    => handle_project(state, msg),
        Message::Sync(msg)       => handle_sync(state, msg),
        Message::Freezer(msg)    => handle_freezer(state, msg),
        Message::History(msg)    => handle_history(state, msg),
        Message::Settings(msg)   => handle_settings(state, msg),
        Message::Background(msg) => handle_background(state, msg),
        Message::Filter(msg) => {
            state.apply_filter(msg);
            Task::none()
        }
        Message::Context(msg) => {
            tracing::debug!("context message: {:?}", msg);
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<'_, Message> {
    app_view(state)
}

// ---------------------------------------------------------------------------
// Shortcut handler
// ---------------------------------------------------------------------------

fn handle_shortcut(state: &mut AppState, msg: ShortcutMessage) -> Task<Message> {
    match msg {
        ShortcutMessage::Refresh => {
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                refresh_workspace_task(state)
            } else {
                Task::none()
            }
        }
        ShortcutMessage::OpenContextOps => {
            state.screen = Screen::ContextOps;
            Task::none()
        }
        ShortcutMessage::OpenFreezer => {
            state.screen = Screen::Freezer;
            Task::none()
        }
        ShortcutMessage::FocusSearch => {
            state.screen = Screen::Dashboard;
            // Focus is handled by widget ID in a future phase; for now just navigate.
            Task::none()
        }
        ShortcutMessage::Close => {
            // Close any open dialog.
            state.add_project_dialog = None;
            state.confirm_remove_dialog = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace handler
// ---------------------------------------------------------------------------

fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                state.status_bar = Some(state.t("status.refreshing").to_owned());
                refresh_workspace_task(state)
            } else {
                Task::none()
            }
        }

        WorkspaceMessage::WorkspaceSwitched(id) => {
            tracing::info!("workspace switched to {id}");
            Task::none()
        }

        WorkspaceMessage::AddProjectDialogOpened => {
            state.add_project_dialog = Some(AddProjectDialog::default());
            Task::none()
        }
        WorkspaceMessage::AddProjectNameChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.name = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectPathChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog {
                d.path = s;
                d.error = None;
            }
            Task::none()
        }
        WorkspaceMessage::AddProjectConfirmed => {
            let dialog = match state.add_project_dialog.take() {
                Some(d) => d,
                None => return Task::none(),
            };

            let name = dialog.name.trim().to_owned();
            let path = dialog.path.trim().to_owned();

            if name.is_empty() || path.is_empty() {
                state.add_project_dialog = Some(AddProjectDialog {
                    name: dialog.name,
                    path: dialog.path,
                    error: Some(state.t("dialog.add_project.error_empty").to_owned()),
                });
                return Task::none();
            }

            let project = Project::new(name, path);
            if let Some(ws) = &mut state.workspace {
                ws.add_project(project);
                persist_workspace(ws);
            }

            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }
        WorkspaceMessage::AddProjectCancelled => {
            state.add_project_dialog = None;
            Task::none()
        }

        WorkspaceMessage::RemoveProjectRequested(id) => {
            let name = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            state.confirm_remove_dialog = Some(ConfirmRemoveDialog { project_id: id, project_name: name });
            Task::none()
        }
        WorkspaceMessage::RemoveProjectConfirmed(id) => {
            state.confirm_remove_dialog = None;
            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
                persist_workspace(ws);
            }
            // Remove cached status entry.
            if let Some(ws_status) = &mut state.workspace_status {
                ws_status.projects.retain(|s| s.project_id != id);
            }
            state.fetching_projects.remove(&id);
            Task::none()
        }
        WorkspaceMessage::RemoveProjectCancelled => {
            state.confirm_remove_dialog = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Project handler
// ---------------------------------------------------------------------------

fn handle_project(state: &mut AppState, msg: ProjectMessage) -> Task<Message> {
    match msg {
        ProjectMessage::StatusRefreshRequested(id) => {
            let project = find_project(state, &id);
            if let Some(project) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&project).await },
                    |status| {
                        let ws = endringer::WorkspaceStatus {
                            projects: vec![status],
                            last_refresh: Some(chrono::Utc::now()),
                        };
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(ws))
                    },
                )
            } else {
                Task::none()
            }
        }

        ProjectMessage::FetchRequested(id) => {
            state.fetching_projects.insert(id.clone());
            let project = find_project(state, &id);
            if let Some(project) = project {
                let project_id = id.clone();
                Task::perform(
                    async move {
                        let started = chrono::Utc::now();
                        let op_id = endringer::OperationId::new();
                        let result = VcsAdapter::fetch(&project).await;
                        let finished = chrono::Utc::now();
                        OperationLog {
                            result: OperationResult {
                                operation_id: op_id,
                                kind: OperationKind::Fetch,
                                started_at: started,
                                finished_at: finished,
                                per_project: vec![result],
                                rollback_attempted: false,
                                rollback_succeeded: None,
                            },
                            recovery_hints: vec![],
                        }
                    },
                    |log| Message::Background(BackgroundMessage::SingleFetchCompleted(log)),
                )
            } else {
                state.fetching_projects.remove(&id);
                Task::none()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sync handler
// ---------------------------------------------------------------------------

fn handle_sync(state: &mut AppState, msg: SyncMessage) -> Task<Message> {
    match msg {
        SyncMessage::BulkFetchRequested(ids) => {
            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| {
                    ws.projects
                        .iter()
                        .filter(|p| ids.contains(&p.id))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            let max_concurrent = state.config.max_concurrent_reads;

            Task::perform(
                async move {
                    use std::sync::Arc;
                    use tokio::sync::Semaphore;
                    let started = chrono::Utc::now();
                    let op_id = endringer::OperationId::new();
                    let sem = Arc::new(Semaphore::new(max_concurrent));
                    let mut handles = Vec::new();
                    for project in projects {
                        let sem = Arc::clone(&sem);
                        handles.push(tokio::spawn(async move {
                            let _permit = sem.acquire().await.expect("open");
                            VcsAdapter::fetch(&project).await
                        }));
                    }
                    let mut per_project = Vec::new();
                    for h in handles {
                        if let Ok(r) = h.await { per_project.push(r); }
                    }
                    OperationLog {
                        result: OperationResult {
                            operation_id: op_id,
                            kind: OperationKind::Fetch,
                            started_at: started,
                            finished_at: chrono::Utc::now(),
                            per_project,
                            rollback_attempted: false,
                            rollback_succeeded: None,
                        },
                        recovery_hints: vec![],
                    }
                },
                |log| Message::Background(BackgroundMessage::BulkFetchCompleted(log)),
            )
        }
        SyncMessage::SmartPullRequested(_ids) => {
            tracing::info!("smart pull — Phase 3");
            Task::none()
        }
        SyncMessage::ProjectToggled(_, _) => Task::none(),
    }
}

// ---------------------------------------------------------------------------
// Freezer handler
// ---------------------------------------------------------------------------

fn handle_freezer(state: &mut AppState, msg: FreezerMessage) -> Task<Message> {
    if let FreezerMessage::NameChanged(name) = msg {
        state.freezer_name = name;
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// History handler
// ---------------------------------------------------------------------------

fn handle_history(state: &mut AppState, msg: HistoryMessage) -> Task<Message> {
    match msg {
        HistoryMessage::SearchChanged(s)   => { state.history_search = s; }
        HistoryMessage::LogCopyRequested(_)=> {}
        HistoryMessage::EntryToggled(_)    => {}
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Settings handler
// ---------------------------------------------------------------------------

fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(locale) => {
            state.config.locale = locale;
            state.catalog = snora::i18n::Catalog::for_locale(locale);
        }
        SettingsMessage::ThemeChanged(dark) => {
            state.config.dark_theme = dark;
            state.theme = if dark { snora::KnotraTheme::dark() } else { snora::KnotraTheme::light() };
        }
        SettingsMessage::RefreshIntervalChanged(secs) => { state.config.refresh_interval_secs = secs; }
        SettingsMessage::MaxConcurrentChanged(n)      => { state.config.max_concurrent_reads = n; }
        SettingsMessage::SaveRequested => {
            let paths = AppPaths::resolve();
            match save_config(&state.config, &paths) {
                Ok(()) => state.status_bar = Some("Settings saved.".to_owned()),
                Err(e) => state.status_bar = Some(format!("Error saving config: {e}")),
            }
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Background handler
// ---------------------------------------------------------------------------

fn handle_background(state: &mut AppState, msg: BackgroundMessage) -> Task<Message> {
    match msg {
        BackgroundMessage::WorkspaceStatusRefreshed(new_status) => {
            merge_workspace_status(state, new_status);
            state.load_phase = LoadPhase::Ready;
            state.is_refreshing = false;
            state.status_bar = None;
            Task::none()
        }

        BackgroundMessage::SingleFetchCompleted(log) => {
            // Remove the project from the "fetching" set.
            for r in &log.result.per_project {
                state.fetching_projects.remove(&r.project_id);
            }
            persist_log(&log, state);

            // Refresh status for the fetched project.
            let ids: Vec<_> = log.result.per_project.iter().map(|r| r.project_id.clone()).collect();
            let tasks: Vec<Task<Message>> = ids
                .into_iter()
                .filter_map(|id| find_project(state, &id))
                .map(|project| {
                    Task::perform(
                        async move { VcsAdapter::read_project_status(&project).await },
                        |status| {
                            let ws = endringer::WorkspaceStatus {
                                projects: vec![status],
                                last_refresh: Some(chrono::Utc::now()),
                            };
                            Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(ws))
                        },
                    )
                })
                .collect();
            Task::batch(tasks)
        }

        BackgroundMessage::BulkFetchCompleted(log) => {
            persist_log(&log, state);
            let failed = log.result.any_failed();
            state.status_bar = Some(if failed {
                format!(
                    "Fetch complete — {} succeeded, {} failed.",
                    log.result.successful_projects().len(),
                    log.result.failed_projects().len()
                )
            } else {
                format!("Fetch complete — {} projects updated.", log.result.per_project.len())
            });

            state.is_refreshing = true;
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }

        BackgroundMessage::SmartPullCompleted(log)
        | BackgroundMessage::ContextSwitchCompleted(log)
        | BackgroundMessage::FreezeCompleted(log) => {
            persist_log(&log, state);
            Task::none()
        }

        BackgroundMessage::TaskError { description } => {
            state.load_phase = LoadPhase::Error(description.clone());
            state.is_refreshing = false;
            state.status_bar = Some(description);
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_project(state: &AppState, id: &endringer::ProjectId) -> Option<endringer::Project> {
    state
        .workspace
        .as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id).cloned())
}

fn merge_workspace_status(state: &mut AppState, new_status: endringer::WorkspaceStatus) {
    if let Some(existing) = &mut state.workspace_status {
        for new_ps in new_status.projects {
            if let Some(pos) = existing.projects.iter().position(|p| p.project_id == new_ps.project_id) {
                existing.projects[pos] = new_ps;
            } else {
                existing.projects.push(new_ps);
            }
        }
        existing.last_refresh = new_status.last_refresh;
    } else {
        state.workspace_status = Some(new_status);
    }
}

fn persist_workspace(ws: &Workspace) {
    let paths = AppPaths::resolve();
    if let Err(e) = save_workspace(ws, &paths) {
        tracing::warn!("failed to save workspace: {e}");
    }
}

fn persist_log(log: &OperationLog, state: &mut AppState) {
    let paths = AppPaths::resolve();
    if let Err(e) = save_operation_log(log, &paths) {
        tracing::warn!("failed to save operation log: {e}");
    }
    state.operation_logs.insert(0, log.clone());
    state.operation_logs.truncate(state.config.max_log_entries);
}

fn refresh_workspace_task(state: &AppState) -> Task<Message> {
    let workspace = match &state.workspace {
        Some(ws) => ws.clone(),
        None => return Task::none(),
    };
    let max = state.config.max_concurrent_reads;

    Task::perform(
        async move { VcsAdapter::read_workspace_status(&workspace, max).await },
        |status| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(status)),
    )
}
