//! Top-level Elm-architecture implementation for knotra.
//!
//! `update` maps `Message → Task<Message>` and mutates `AppState`.
//! `view` maps `&AppState → Element<Message>`.
//! Neither function performs I/O directly.

use iced::{Element, Task};

use endringer::{VcsAdapter, model::workspace::Workspace};

use crate::{
    config::{AppPaths, load_config, save_config},
    message::{
        BackgroundMessage, FreezerMessage, HistoryMessage, Message, ProjectMessage,
        SettingsMessage, SyncMessage, WorkspaceMessage,
    },
    persistence::{load_recent_logs, load_workspaces, save_operation_log},
    state::{AppState, LoadPhase, Screen},
    view::app_view,
};

// ---------------------------------------------------------------------------
// Application bootstrap
// ---------------------------------------------------------------------------

/// Initialise application state and issue the first startup tasks.
pub fn init() -> (AppState, Task<Message>) {
    let paths = AppPaths::resolve();
    let (config, config_err) = load_config(&paths);

    let mut state = AppState::new(config.clone());

    if let Some(err) = config_err {
        state.status_bar = Some(err);
    }

    // Load workspaces.
    let (workspaces, ws_errors) = load_workspaces(&paths);
    for e in &ws_errors {
        tracing::warn!("workspace load error: {e}");
    }

    // Use the first workspace (or create a default empty one).
    let workspace = workspaces
        .into_iter()
        .next()
        .unwrap_or_else(|| Workspace::new("My Workspace"));
    state.workspace = Some(workspace);
    state.load_phase = LoadPhase::Refreshing;

    // Load recent history.
    state.operation_logs = load_recent_logs(&paths, config.max_log_entries);

    // Kick off the initial workspace status refresh.
    let task = refresh_workspace_task(&state);

    (state, task)
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

        Message::Workspace(msg) => handle_workspace(state, msg),
        Message::Project(msg) => handle_project(state, msg),
        Message::Sync(msg) => handle_sync(state, msg),
        Message::Freezer(msg) => handle_freezer(state, msg),
        Message::History(msg) => handle_history(state, msg),
        Message::Settings(msg) => handle_settings(state, msg),
        Message::Background(msg) => handle_background(state, msg),
        Message::Filter(msg) => {
            state.apply_filter(msg);
            Task::none()
        }
        Message::Context(msg) => {
            // Context operations handled in Phase 4.
            tracing::debug!("context message: {:?}", msg);
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &AppState) -> Element<Message> {
    app_view(state)
}

// ---------------------------------------------------------------------------
// Message handlers
// ---------------------------------------------------------------------------

fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            state.load_phase = LoadPhase::Refreshing;
            state.status_bar = Some(state.t("status.refreshing").to_owned());
            refresh_workspace_task(state)
        }
        WorkspaceMessage::WorkspaceSwitched(id) => {
            tracing::info!("workspace switched to {id}");
            Task::none()
        }
        WorkspaceMessage::AddProjectDialogOpened => {
            // Phase 2+: show add-project dialog.
            tracing::info!("add project dialog requested");
            Task::none()
        }
        WorkspaceMessage::ProjectAdded(project) => {
            if let Some(ws) = &mut state.workspace {
                ws.add_project(project);
            }
            refresh_workspace_task(state)
        }
        WorkspaceMessage::ProjectRemoved(id) => {
            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
            }
            Task::none()
        }
    }
}

fn handle_project(state: &mut AppState, msg: ProjectMessage) -> Task<Message> {
    match msg {
        ProjectMessage::StatusRefreshRequested(id) => {
            let project = state
                .workspace
                .as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id).cloned());
            if let Some(project) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&project).await },
                    |status| {
                        // Wrap single status into a workspace status update.
                        let ws_status = endringer::WorkspaceStatus {
                            projects: vec![status],
                            last_refresh: Some(chrono::Utc::now()),
                        };
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(ws_status))
                    },
                )
            } else {
                Task::none()
            }
        }
        ProjectMessage::CardExpanded(_id) => Task::none(),
    }
}

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
                    use endringer::model::operation::{
                        OperationId, OperationKind, OperationLog, OperationResult,
                        ProjectOperationResult,
                    };
                    use std::sync::Arc;
                    use tokio::sync::Semaphore;

                    let started = chrono::Utc::now();
                    let op_id = OperationId::new();
                    let sem = Arc::new(Semaphore::new(max_concurrent));
                    let mut handles = Vec::new();

                    for project in projects {
                        let sem = Arc::clone(&sem);
                        handles.push(tokio::spawn(async move {
                            let _permit = sem.acquire().await.expect("semaphore open");
                            VcsAdapter::fetch(&project).await
                        }));
                    }

                    let mut per_project = Vec::new();
                    for h in handles {
                        if let Ok(r) = h.await {
                            per_project.push(r);
                        }
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
            // Phase 3 implementation.
            tracing::info!("smart pull requested — Phase 3");
            Task::none()
        }
        SyncMessage::ProjectToggled(_, _) => Task::none(),
    }
}

fn handle_freezer(state: &mut AppState, msg: FreezerMessage) -> Task<Message> {
    match msg {
        FreezerMessage::NameChanged(name) => {
            state.freezer_name = name;
            Task::none()
        }
        _ => {
            // Phase 5 implementation.
            Task::none()
        }
    }
}

fn handle_history(state: &mut AppState, msg: HistoryMessage) -> Task<Message> {
    match msg {
        HistoryMessage::SearchChanged(s) => {
            state.history_search = s;
            Task::none()
        }
        HistoryMessage::LogCopyRequested(_) => Task::none(),
        HistoryMessage::EntryToggled(_) => Task::none(),
    }
}

fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(locale) => {
            state.config.locale = locale;
            state.catalog = snora::i18n::Catalog::for_locale(locale);
            Task::none()
        }
        SettingsMessage::ThemeChanged(dark) => {
            state.config.dark_theme = dark;
            state.theme = if dark {
                snora::KnotraTheme::dark()
            } else {
                snora::KnotraTheme::light()
            };
            Task::none()
        }
        SettingsMessage::RefreshIntervalChanged(secs) => {
            state.config.refresh_interval_secs = secs;
            Task::none()
        }
        SettingsMessage::MaxConcurrentChanged(n) => {
            state.config.max_concurrent_reads = n;
            Task::none()
        }
        SettingsMessage::SaveRequested => {
            let paths = AppPaths::resolve();
            if let Err(e) = save_config(&state.config, &paths) {
                state.status_bar = Some(format!("Error saving config: {e}"));
            } else {
                state.status_bar = Some("Settings saved.".to_owned());
            }
            Task::none()
        }
    }
}

fn handle_background(state: &mut AppState, msg: BackgroundMessage) -> Task<Message> {
    match msg {
        BackgroundMessage::WorkspaceStatusRefreshed(new_status) => {
            if let Some(existing) = &mut state.workspace_status {
                // Merge: update only the projects that came back in new_status.
                for new_ps in new_status.projects {
                    if let Some(pos) = existing
                        .projects
                        .iter()
                        .position(|p| p.project_id == new_ps.project_id)
                    {
                        existing.projects[pos] = new_ps;
                    } else {
                        existing.projects.push(new_ps);
                    }
                }
                existing.last_refresh = new_status.last_refresh;
            } else {
                state.workspace_status = Some(new_status);
            }
            state.load_phase = LoadPhase::Ready;
            state.status_bar = None;
            Task::none()
        }

        BackgroundMessage::BulkFetchCompleted(log) => {
            let paths = AppPaths::resolve();
            if let Err(e) = save_operation_log(&log, &paths) {
                tracing::warn!("failed to save operation log: {e}");
            }
            state.operation_logs.insert(0, log);
            state.operation_logs.truncate(state.config.max_log_entries);
            state.status_bar = Some("Fetch complete.".to_owned());
            // Refresh statuses after fetch.
            state.load_phase = LoadPhase::Refreshing;
            refresh_workspace_task(state)
        }

        BackgroundMessage::SmartPullCompleted(log)
        | BackgroundMessage::ContextSwitchCompleted(log)
        | BackgroundMessage::FreezeCompleted(log) => {
            let paths = AppPaths::resolve();
            if let Err(e) = save_operation_log(&log, &paths) {
                tracing::warn!("failed to save operation log: {e}");
            }
            state.operation_logs.insert(0, log);
            state.operation_logs.truncate(state.config.max_log_entries);
            Task::none()
        }

        BackgroundMessage::TaskError { description } => {
            state.load_phase = LoadPhase::Error(description.clone());
            state.status_bar = Some(description);
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build the workspace refresh task
// ---------------------------------------------------------------------------

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
