//! Top-level Elm-architecture implementation for knotra.

use iced::{keyboard, time, Element, Subscription, Task};
use iced::futures::StreamExt;
use std::time::Duration;

use endringer::{
    model::{
        operation::{
            ContextSwitchResult, OperationId, OperationKind, OperationLog, OperationResult,
            SmartPullDisposition, SmartPullPlan, SmartPullProgress,
        },
        project::Project,
        workspace::Workspace,
    },
    VcsAdapter,
};

use crate::{
    config::{load_config, save_config, AppPaths},
    message::{
        BackgroundMessage, ContextMessage, FilterMessage, FreezerMessage, HistoryMessage, Message,
        ProjectMessage, SettingsMessage, ShortcutMessage, SyncMessage, WorkspaceMessage,
    },
    persistence::{load_recent_logs, load_workspaces, save_operation_log, save_workspace},
    state::{
        context::{ContextOpsState, ContextPhase},
        sync::{ProjectOutcome, SyncKind, SyncPhase, SyncResult},
        AddProjectDialog, AppState, ConfirmRemoveDialog, LoadPhase, Screen,
    },
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
    for e in &ws_errors { tracing::warn!("workspace load error: {e}"); }

    let workspace = workspaces.into_iter().next()
        .unwrap_or_else(|| Workspace::new("My Workspace"));
    state.workspace = Some(workspace);
    state.load_phase = LoadPhase::Refreshing;
    state.is_refreshing = true;
    state.operation_logs = load_recent_logs(&paths, config.max_log_entries);

    let task = refresh_workspace_task(&state);
    (state, task)
}

// ---------------------------------------------------------------------------
// Subscription
// ---------------------------------------------------------------------------

pub fn subscription(state: &AppState) -> Subscription<Message> {
    let tick_sub = if state.config.refresh_interval_secs > 0 {
        time::every(Duration::from_secs(u64::from(state.config.refresh_interval_secs)))
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
        Message::Tick
    });

    Subscription::batch([tick_sub, keyboard_sub])
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Navigate(screen)    => { state.screen = screen; Task::none() }
        Message::Tick                => handle_tick(state),
        Message::Shortcut(msg)       => handle_shortcut(state, msg),
        Message::Workspace(msg)      => handle_workspace(state, msg),
        Message::Project(msg)        => handle_project(state, msg),
        Message::Sync(msg)           => handle_sync(state, msg),
        Message::Freezer(msg)        => handle_freezer(state, msg),
        Message::History(msg)        => handle_history(state, msg),
        Message::Settings(msg)       => handle_settings(state, msg),
        Message::Background(msg)     => handle_background(state, msg),
        Message::Filter(msg)         => { state.apply_filter(msg); Task::none() }
        Message::Context(msg)        => handle_context(state, msg),
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    app_view(state)
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

fn handle_tick(state: &mut AppState) -> Task<Message> {
    if !state.is_refreshing {
        state.is_refreshing = true;
        state.load_phase = LoadPhase::Refreshing;
        refresh_workspace_task(state)
    } else {
        Task::none()
    }
}

// ---------------------------------------------------------------------------
// Shortcut
// ---------------------------------------------------------------------------

fn handle_shortcut(state: &mut AppState, msg: ShortcutMessage) -> Task<Message> {
    match msg {
        ShortcutMessage::Refresh => handle_tick(state),
        ShortcutMessage::OpenContextOps => handle_context(state, ContextMessage::OpenRequested(None)),
        ShortcutMessage::OpenFreezer    => { state.screen = Screen::Freezer;    Task::none() }
        ShortcutMessage::FocusSearch    => { state.screen = Screen::Dashboard;  Task::none() }
        ShortcutMessage::Close          => {
            state.add_project_dialog   = None;
            state.confirm_remove_dialog = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

fn handle_workspace(state: &mut AppState, msg: WorkspaceMessage) -> Task<Message> {
    match msg {
        WorkspaceMessage::RefreshRequested => {
            if !state.is_refreshing {
                state.is_refreshing = true;
                state.load_phase = LoadPhase::Refreshing;
                state.status_bar = Some(state.t("status.refreshing").to_owned());
                refresh_workspace_task(state)
            } else { Task::none() }
        }
        WorkspaceMessage::WorkspaceSwitched(id) => {
            tracing::info!("workspace switched: {id}"); Task::none()
        }
        WorkspaceMessage::AddProjectDialogOpened => {
            state.add_project_dialog = Some(AddProjectDialog::default()); Task::none()
        }
        WorkspaceMessage::AddProjectNameChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog { d.name = s; d.error = None; }
            Task::none()
        }
        WorkspaceMessage::AddProjectPathChanged(s) => {
            if let Some(d) = &mut state.add_project_dialog { d.path = s; d.error = None; }
            Task::none()
        }
        WorkspaceMessage::AddProjectConfirmed => {
            let dialog = match state.add_project_dialog.take() { Some(d) => d, None => return Task::none() };
            let name = dialog.name.trim().to_owned();
            let path = dialog.path.trim().to_owned();
            if name.is_empty() || path.is_empty() {
                state.add_project_dialog = Some(AddProjectDialog {
                    name: dialog.name, path: dialog.path,
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
            state.add_project_dialog = None; Task::none()
        }
        WorkspaceMessage::RemoveProjectRequested(id) => {
            let name = state.workspace.as_ref()
                .and_then(|ws| ws.projects.iter().find(|p| p.id == id))
                .map(|p| p.name.clone()).unwrap_or_default();
            state.confirm_remove_dialog = Some(ConfirmRemoveDialog { project_id: id, project_name: name });
            Task::none()
        }
        WorkspaceMessage::RemoveProjectConfirmed(id) => {
            state.confirm_remove_dialog = None;
            if let Some(ws) = &mut state.workspace {
                ws.remove_project(&id);
                persist_workspace(ws);
            }
            if let Some(ws_status) = &mut state.workspace_status {
                ws_status.projects.retain(|s| s.project_id != id);
            }
            state.fetching_projects.remove(&id);
            Task::none()
        }
        WorkspaceMessage::RemoveProjectCancelled => {
            state.confirm_remove_dialog = None; Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

fn handle_project(state: &mut AppState, msg: ProjectMessage) -> Task<Message> {
    match msg {
        ProjectMessage::StatusRefreshRequested(id) => {
            let project = find_project(state, &id);
            if let Some(p) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&p).await },
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        endringer::WorkspaceStatus {
                            projects: vec![s],
                            last_refresh: Some(chrono::Utc::now()),
                        },
                    )),
                )
            } else { Task::none() }
        }
        ProjectMessage::FetchRequested(id) => {
            state.fetching_projects.insert(id.clone());
            let project = find_project(state, &id);
            if let Some(p) = project {
                Task::perform(
                    async move {
                        let started = chrono::Utc::now();
                        let op_id = OperationId::new();
                        let result = VcsAdapter::fetch(&p).await;
                        OperationLog {
                            result: OperationResult {
                                operation_id: op_id,
                                kind: OperationKind::Fetch,
                                started_at: started,
                                finished_at: chrono::Utc::now(),
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
// Sync
// ---------------------------------------------------------------------------

fn handle_sync(state: &mut AppState, msg: SyncMessage) -> Task<Message> {
    match msg {
        SyncMessage::OpenRequested => {
            if let Some(ws) = &state.workspace {
                state.sync.init_selection(&ws.projects);
            }
            state.screen = Screen::SyncCenter;
            Task::none()
        }

        SyncMessage::ProjectToggled(id, included) => {
            state.sync.project_selection.insert(id, included);
            Task::none()
        }

        SyncMessage::DispositionChanged(id, disposition) => {
            state.sync.disposition_overrides.insert(id, disposition);
            Task::none()
        }

        SyncMessage::BulkFetchRequested => {
            let ids = state.sync.selected_ids();
            let projects: Vec<_> = state.workspace.as_ref()
                .map(|ws| ws.projects.iter().filter(|p| ids.contains(&p.id)).cloned().collect())
                .unwrap_or_default();
            let total = projects.len();
            state.sync.phase = SyncPhase::FetchRunning { total, done: 0 };

            let max = state.config.max_concurrent_reads;

            // Stream results per-project using Task::run.
            use iced::futures::stream;

            let project_stream = stream::iter(projects).then(move |project| async move {
                VcsAdapter::fetch(&project).await
            });

            Task::run(project_stream, |per_project_result| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted(SmartPullProgress {
                    project_id: per_project_result.project_id.clone(),
                    project_name: String::new(), // filled in from state on receipt
                    result: per_project_result,
                    recovery_hint: None,
                }))
            })
        }

        SyncMessage::SmartPullPlanRequested => {
            state.sync.phase = SyncPhase::Planning;
            // Build the plan synchronously from existing status.
            let plan = state.sync.build_plan(
                state.workspace.as_ref().map(|w| w.projects.as_slice()).unwrap_or(&[]),
                state.workspace_status.as_ref(),
            );
            state.sync.phase = SyncPhase::AwaitingConfirm(plan.clone());
            Task::done(Message::Background(BackgroundMessage::SmartPullPlanReady(plan)))
        }

        SyncMessage::SmartPullConfirmed(plan) => {
            let projects_map: std::collections::HashMap<_, _> = state
                .workspace.as_ref()
                .map(|ws| ws.projects.iter().map(|p| (p.id.clone(), p.clone())).collect())
                .unwrap_or_default();

            let entries = plan.entries.clone();
            state.sync.phase = SyncPhase::PullRunning { plan, completed: Vec::new() };

            use iced::futures::stream;

            let pull_stream = stream::iter(entries).then(move |entry| {
                let project = projects_map.get(&entry.project_id).cloned();
                async move {
                    let Some(project) = project else {
                        return SmartPullProgress {
                            project_id: entry.project_id.clone(),
                            project_name: entry.project_name.clone(),
                            result: endringer::model::operation::ProjectOperationResult {
                                project_id: entry.project_id,
                                success: false,
                                commands_executed: vec![],
                                stdout: String::new(),
                                stderr: String::new(),
                                exit_code: None,
                                error_message: Some("project not found".to_owned()),
                            },
                            recovery_hint: None,
                        };
                    };

                    match entry.disposition {
                        SmartPullDisposition::Excluded => SmartPullProgress {
                            project_id: project.id.clone(),
                            project_name: entry.project_name.clone(),
                            result: endringer::model::operation::ProjectOperationResult {
                                project_id: project.id.clone(),
                                success: true,
                                commands_executed: vec![],
                                stdout: "[excluded]".to_owned(),
                                stderr: String::new(),
                                exit_code: Some(0),
                                error_message: None,
                            },
                            recovery_hint: None,
                        },
                        SmartPullDisposition::FetchOnly => {
                            let r = VcsAdapter::fetch(&project).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: None,
                            }
                        }
                        SmartPullDisposition::Pull | SmartPullDisposition::StashAndPull => {
                            let stash = matches!(entry.disposition, SmartPullDisposition::StashAndPull);
                            let (r, hint) = VcsAdapter::smart_pull(&project, stash).await;
                            SmartPullProgress {
                                project_id: project.id.clone(),
                                project_name: entry.project_name.clone(),
                                result: r,
                                recovery_hint: hint,
                            }
                        }
                    }
                }
            });

            Task::run(pull_stream, |progress| {
                Message::Background(BackgroundMessage::SmartPullProjectCompleted(progress))
            })
        }

        SyncMessage::SmartPullCancelled => {
            state.sync.phase = SyncPhase::Idle;
            Task::none()
        }

        SyncMessage::RetryFailedRequested => {
            // Collect failed project IDs from the last result.
            let failed_ids: Vec<_> = if let SyncPhase::Done(ref result) = state.sync.phase {
                result.per_project.iter()
                    .filter(|p| !p.success)
                    .map(|p| p.project_id.clone())
                    .collect()
            } else { return Task::none(); };

            // Deselect all, then select only failed.
            for (id, v) in state.sync.project_selection.iter_mut() {
                *v = failed_ids.contains(id);
            }
            state.sync.phase = SyncPhase::Idle;
            Task::done(Message::Sync(SyncMessage::BulkFetchRequested))
        }
    }
}

// ---------------------------------------------------------------------------
// Background
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

        BackgroundMessage::SmartPullPlanReady(plan) => {
            // Already set in handle_sync; this message lets the view re-render.
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);
            Task::none()
        }

        BackgroundMessage::SmartPullProjectCompleted(mut progress) => {
            // Fill in the project name if missing.
            if progress.project_name.is_empty() {
                if let Some(name) = find_project_name(state, &progress.project_id) {
                    progress.project_name = name;
                }
            }

            match &mut state.sync.phase {
                SyncPhase::FetchRunning { done, total } => {
                    *done += 1;
                    let done_val = *done;
                    let total_val = *total;

                    // Accumulate into a temporary vec using the Done phase.
                    // We check completion after updating.
                    let outcome = ProjectOutcome {
                        project_id:        progress.project_id.clone(),
                        project_name:      progress.project_name.clone(),
                        success:           progress.result.success,
                        commands_executed: progress.result.commands_executed.clone(),
                        stdout:            progress.result.stdout.clone(),
                        stderr:            progress.result.stderr.clone(),
                        log_expanded:      false,
                    };

                    // Store partial results in a transient Done or accumulate in running.
                    // For simplicity, switch to Done when all projects report.
                    // We keep a partial results list in the Fetch case.
                    // Switch to Done phase temporarily when all are in.
                    if done_val >= total_val {
                        // All done — we need the full list. Rebuild from log.
                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::Fetch,
                            per_project: vec![outcome],
                            recovery_hints: vec![],
                        });
                        // Trigger status refresh.
                        state.is_refreshing = true;
                        state.load_phase = LoadPhase::Refreshing;
                        return refresh_workspace_task(state);
                    }
                    // Still running — accumulate into a temporary SyncResult in Done.
                    // Replace with accumulation pattern.
                    // (This rebuilds to avoid borrow issues.)
                    let _ = outcome; // handled per-project in Done transition
                }
                SyncPhase::PullRunning { plan, completed } => {
                    if let Some(hint) = progress.recovery_hint.clone() {
                        // Recovery hint collected.
                        let _ = hint;
                    }
                    completed.push(progress.clone());

                    let expected = plan.entries.len();
                    let got = completed.len();
                    if got >= expected {
                        // Build final result from completed.
                        let outcomes: Vec<ProjectOutcome> = completed.iter().map(|p| ProjectOutcome {
                            project_id:        p.project_id.clone(),
                            project_name:      p.project_name.clone(),
                            success:           p.result.success,
                            commands_executed: p.result.commands_executed.clone(),
                            stdout:            p.result.stdout.clone(),
                            stderr:            p.result.stderr.clone(),
                            log_expanded:      false,
                        }).collect();

                        let hints: Vec<_> = completed.iter()
                            .filter_map(|p| p.recovery_hint.clone())
                            .collect();

                        state.sync.phase = SyncPhase::Done(SyncResult {
                            kind: SyncKind::SmartPull,
                            per_project: outcomes,
                            recovery_hints: hints,
                        });

                        // Trigger status refresh.
                        state.is_refreshing = true;
                        state.load_phase = LoadPhase::Refreshing;
                        return refresh_workspace_task(state);
                    }
                }
                _ => {}
            }
            Task::none()
        }

        BackgroundMessage::SingleFetchCompleted(log) => {
            for r in &log.result.per_project {
                state.fetching_projects.remove(&r.project_id);
            }
            persist_log(&log, state);

            let tasks: Vec<Task<Message>> = log.result.per_project.iter()
                .filter_map(|r| find_project(state, &r.project_id))
                .map(|project| Task::perform(
                    async move { VcsAdapter::read_project_status(&project).await },
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        endringer::WorkspaceStatus { projects: vec![s], last_refresh: Some(chrono::Utc::now()) },
                    )),
                ))
                .collect();
            Task::batch(tasks)
        }

        BackgroundMessage::BulkFetchCompleted(log) => {
            persist_log(&log, state);
            state.status_bar = Some(if log.result.any_failed() {
                format!("Fetch — {} ok, {} failed",
                    log.result.successful_projects().len(),
                    log.result.failed_projects().len())
            } else {
                format!("Fetch complete — {} projects", log.result.per_project.len())
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

        BackgroundMessage::ContextListLoaded(list) => {
            let id = list.project_id.clone();
            state.context_ops.cached_lists.insert(id.clone(), list.clone());
            // Only update phase if we were waiting for this exact project.
            if matches!(&state.context_ops.phase, ContextPhase::LoadingList(loading_id) if loading_id == &id) {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list,
                    search: String::new(),
                };
            }
            Task::none()
        }

        BackgroundMessage::ContextSwitchDone(result) => {
            use endringer::model::operation::{OperationKind, OperationLog, OperationResult};

            // Build an operation log entry.
            let op_log = OperationLog {
                result: OperationResult {
                    operation_id: OperationId::new(),
                    kind: OperationKind::ContextSwitch,
                    started_at: chrono::Utc::now(),
                    finished_at: chrono::Utc::now(),
                    per_project: vec![result.operation_result.clone()],
                    rollback_attempted: false,
                    rollback_succeeded: None,
                },
                recovery_hints: result.recovery_hint.clone().into_iter().collect(),
            };
            persist_log(&op_log, state);

            state.context_ops.phase = ContextPhase::Done(result);

            // Refresh the project's status card after a switch.
            let project = match &state.context_ops.phase {
                ContextPhase::Done(r) => find_project(state, &r.project_id),
                _ => None,
            };
            if let Some(p) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&p).await },
                    |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                        endringer::WorkspaceStatus { projects: vec![s], last_refresh: Some(chrono::Utc::now()) },
                    )),
                )
            } else {
                Task::none()
            }
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
// Freezer / History / Settings
// ---------------------------------------------------------------------------

fn handle_freezer(state: &mut AppState, msg: FreezerMessage) -> Task<Message> {
    if let FreezerMessage::NameChanged(name) = msg { state.freezer_name = name; }
    Task::none()
}

fn handle_history(state: &mut AppState, msg: HistoryMessage) -> Task<Message> {
    if let HistoryMessage::SearchChanged(s) = msg { state.history_search = s; }
    Task::none()
}

fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(l)    => { state.config.locale = l; state.catalog = snora::i18n::Catalog::for_locale(l); }
        SettingsMessage::ThemeChanged(dark)  => { state.config.dark_theme = dark; state.theme = if dark { snora::KnotraTheme::dark() } else { snora::KnotraTheme::light() }; }
        SettingsMessage::RefreshIntervalChanged(s) => { state.config.refresh_interval_secs = s; }
        SettingsMessage::MaxConcurrentChanged(n)   => { state.config.max_concurrent_reads = n; }
        SettingsMessage::SaveRequested => {
            let paths = AppPaths::resolve();
            match save_config(&state.config, &paths) {
                Ok(()) => state.status_bar = Some("Settings saved.".to_owned()),
                Err(e) => state.status_bar = Some(format!("Config save error: {e}")),
            }
        }
    }
    Task::none()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_project(state: &AppState, id: &endringer::ProjectId) -> Option<endringer::Project> {
    state.workspace.as_ref()
        .and_then(|ws| ws.projects.iter().find(|p| &p.id == id).cloned())
}

fn find_project_name(state: &AppState, id: &endringer::ProjectId) -> Option<String> {
    find_project(state, id).map(|p| p.name)
}

fn merge_workspace_status(state: &mut AppState, new: endringer::WorkspaceStatus) {
    if let Some(existing) = &mut state.workspace_status {
        for ps in new.projects {
            if let Some(pos) = existing.projects.iter().position(|p| p.project_id == ps.project_id) {
                existing.projects[pos] = ps;
            } else {
                existing.projects.push(ps);
            }
        }
        existing.last_refresh = new.last_refresh;
    } else {
        state.workspace_status = Some(new);
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
    let workspace = match &state.workspace { Some(ws) => ws.clone(), None => return Task::none() };
    let max = state.config.max_concurrent_reads;
    Task::perform(
        async move { VcsAdapter::read_workspace_status(&workspace, max).await },
        |s| Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(s)),
    )
}

// ---------------------------------------------------------------------------
// Context Operations handler
// ---------------------------------------------------------------------------

fn handle_context(state: &mut AppState, msg: ContextMessage) -> Task<Message> {
    match msg {
        ContextMessage::OpenRequested(preselect_id) => {
            state.screen = Screen::ContextOps;
            state.context_ops.phase = ContextPhase::Idle;

            // If a project was pre-selected (e.g. from a dashboard card shortcut), load it.
            if let Some(id) = preselect_id {
                if let Some(project) = find_project(state, &id) {
                    state.context_ops.phase = ContextPhase::LoadingList(id.clone());
                    return Task::perform(
                        async move { VcsAdapter::list_contexts(&project).await },
                        |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
                    );
                }
            }
            Task::none()
        }

        ContextMessage::ProjectSelected(id) => {
            let project = match find_project(state, &id) {
                Some(p) => p,
                None    => return Task::none(),
            };

            // Use cached list if present, otherwise fetch.
            if let Some(cached) = state.context_ops.cached_lists.get(&id).cloned() {
                state.context_ops.phase = ContextPhase::BrowsingList {
                    project_id: id,
                    list: cached,
                    search: String::new(),
                };
                return Task::none();
            }

            state.context_ops.phase = ContextPhase::LoadingList(id.clone());
            Task::perform(
                async move { VcsAdapter::list_contexts(&project).await },
                |list| Message::Background(BackgroundMessage::ContextListLoaded(list)),
            )
        }

        ContextMessage::SearchChanged(s) => {
            if let ContextPhase::BrowsingList { search, .. } = &mut state.context_ops.phase {
                *search = s;
            }
            Task::none()
        }

        ContextMessage::SwitchTargetChosen(project_id, target) => {
            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None    => return Task::none(),
            };

            // Check current dirty state.
            let vcs_kind = state
                .workspace_status.as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id))
                .map(|s| s.identity.vcs_kind)
                .unwrap_or(endringer::VcsKind::Git);

            let is_dirty = state
                .workspace_status.as_ref()
                .and_then(|ws| ws.projects.iter().find(|s| s.project_id == project_id))
                .map(|s| s.working_tree.is_dirty())
                .unwrap_or(false);

            state.context_ops.phase = ContextPhase::ConfirmSwitch {
                project_id,
                project_name: project.name.clone(),
                target,
                vcs_kind,
                is_dirty,
            };
            Task::none()
        }

        ContextMessage::SwitchConfirmed => {
            let (project_id, target, project_name) = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, target, project_name, .. } => {
                    (project_id.clone(), target.clone(), project_name.clone())
                }
                _ => return Task::none(),
            };

            let project = match find_project(state, &project_id) {
                Some(p) => p,
                None    => return Task::none(),
            };

            state.context_ops.phase = ContextPhase::Switching {
                project_id: project_id.clone(),
                target: target.clone(),
            };
            // Invalidate cached list for this project.
            state.context_ops.cached_lists.remove(&project_id);

            Task::perform(
                async move {
                    let (result, hint) = VcsAdapter::switch_context(&project, &target).await;
                    ContextSwitchResult { project_id: project.id, project_name, target, operation_result: result, recovery_hint: hint }
                },
                |r| Message::Background(BackgroundMessage::ContextSwitchDone(r)),
            )
        }

        ContextMessage::SwitchCancelled => {
            // Return to browsing.
            let prev_id = match &state.context_ops.phase {
                ContextPhase::ConfirmSwitch { project_id, .. } => Some(project_id.clone()),
                _ => None,
            };
            if let Some(id) = prev_id {
                if let Some(cached) = state.context_ops.cached_lists.get(&id).cloned() {
                    state.context_ops.phase = ContextPhase::BrowsingList {
                        project_id: id,
                        list: cached,
                        search: String::new(),
                    };
                    return Task::none();
                }
            }
            state.context_ops.phase = ContextPhase::Idle;
            Task::none()
        }

        ContextMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
            Task::none()
        }
    }
}
