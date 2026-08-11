//! The short handlers (RFC-040 Stage 3 commit 7): project, history, settings,
//! external-tool launch, topology, tag push, selection, palette, and
//! dashboard-display messages, plus the one helper used only by the last of
//! those. Grouped by size, not by a shared domain - each function here still
//! owns exactly one `Message` variant group end to end, same as every other
//! `app/` module; they are simply too small individually to each warrant
//! their own file (RFC-040 D1).
//!
//! `handle_dashboard` calls `handle_workspace` directly - a documented
//! `misc -> workspace` edge per `.git-exclude/reviewed/089-...md`'s ruling
//! (handler modules may call another handler where the domain genuinely
//! requires it, provided the dependency graph stays acyclic) and RFC-040
//! D7. As of RFC-040 Stage 4 this is a real cross-handler-module import -
//! `handle_workspace` calls nothing in this module, so the graph stays
//! acyclic. `workspace.rs` must not import `misc.rs`.

use iced::Task;
use knotra_vcs::{
    VcsAdapter,
    model::operation::{OperationId, OperationKind, OperationLog, OperationResult},
};

// `handle_dashboard` calls `handle_workspace` directly - the documented
// `misc -> workspace` edge (RFC-040 D7). See this module's doc comment.
use super::focus_ops;
use super::shared;
use super::workspace;
use crate::{
    config::{DashboardGrouping, save_config},
    message::{
        BackgroundMessage, DashboardMessage, HistoryMessage, LaunchMessage, Message,
        PaletteMessage, ProjectMessage, SelectionMessage, SettingsMessage, TagPushMessage,
        TopologyMessage, WorkspaceMessage,
    },
    state::{
        AppState, LoadPhase, OperationOwner, PendingTagPush, Screen, focus, topology::TopologyPhase,
    },
};

pub(super) fn handle_project(state: &mut AppState, msg: ProjectMessage) -> Task<Message> {
    match msg {
        ProjectMessage::StatusRefreshRequested(id) => {
            let project = shared::find_project(state, &id);
            if let Some(p) = project {
                Task::perform(
                    async move { VcsAdapter::read_project_status(&p).await },
                    |s| {
                        Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                            knotra_vcs::WorkspaceStatus {
                                projects: vec![s],
                                last_refresh: Some(chrono::Utc::now()),
                            },
                        ))
                    },
                )
            } else {
                Task::none()
            }
        }
        ProjectMessage::FetchRequested(id) => {
            let project = shared::find_project(state, &id);
            if let Some(p) = project {
                let Some(lease_id) = shared::acquire_operation(state, OperationOwner::SingleFetch)
                else {
                    return Task::none();
                };
                state.fetching_projects.insert(id.clone());
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
                    move |log| {
                        Message::Background(BackgroundMessage::SingleFetchCompleted {
                            lease_id,
                            log,
                        })
                    },
                )
            } else {
                state.fetching_projects.remove(&id);
                Task::none()
            }
        }
    }
}

pub(super) fn handle_history(state: &mut AppState, msg: HistoryMessage) -> Task<Message> {
    match msg {
        HistoryMessage::SearchChanged(s) => {
            state.history_search = s;
        }
        HistoryMessage::EntryToggled(id) => {
            if state.history_expanded.contains(&id) {
                state.history_expanded.remove(&id);
            } else {
                state.history_expanded.insert(id);
            }
        }
        HistoryMessage::LogCopyRequested(_id) => {
            // Clipboard access is platform-dependent; Phase 7 can wire iced's clipboard API.
            // For now we record the intent and show a status-bar note.
            // Real clipboard write is handled by Message::CopyToClipboard.
            // This is a fallback status note in case no text was available.
            state.status_bar = Some(state.t("plain.activity.copy_command_sent").to_owned());
        }
        HistoryMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
        }
    }
    Task::none()
}

pub(super) fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(l) => {
            state.config.locale = l;
            state.catalog = knotra_ui::i18n::Catalog::for_locale(l);
        }
        SettingsMessage::ThemeChanged(dark) => {
            state.config.dark_theme = dark;
            state.theme = if dark {
                knotra_ui::KnotraTheme::dark()
            } else {
                knotra_ui::KnotraTheme::light()
            };
        }
        SettingsMessage::RefreshIntervalChanged(s) => {
            // 0 is a valid value here (means manual refresh only) — any
            // parseable u32 is accepted. On invalid input, the edit buffer
            // still shows exactly what the user typed; `config` keeps its
            // last valid value rather than silently coercing.
            if let Ok(n) = s.parse::<u32>() {
                state.config.refresh_interval_secs = n;
            }
            state.settings_edit.refresh_interval_secs = s;
        }
        SettingsMessage::MaxConcurrentChanged(s) => {
            // Must be > 0 — a concurrency limit of 0 would mean nothing
            // could ever read, so it's rejected the same as unparseable text.
            if let Ok(n) = s.parse::<usize>()
                && n > 0
            {
                state.config.max_concurrent_reads = n;
            }
            state.settings_edit.max_concurrent_reads = s;
        }
        SettingsMessage::ExternalEditorChanged(s) => {
            state.settings_edit.external_editor = s.clone();
            state.config.external_editor = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
        }
        SettingsMessage::ExternalMergeToolChanged(s) => {
            state.settings_edit.external_merge_tool = s.clone();
            state.config.external_merge_tool = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
        }
        SettingsMessage::MaxLogEntriesChanged(s) => {
            // Must be > 0 — zero retained log entries isn't a meaningful
            // setting, same reasoning as max concurrent reads above.
            if let Ok(n) = s.parse::<usize>()
                && n > 0
            {
                state.config.max_log_entries = n;
            }
            state.settings_edit.max_log_entries = s;
        }
        SettingsMessage::FsWatchEnabledChanged(v) => {
            state.config.fs_watch_enabled = v;
            if !v {
                state.settings_save_msg =
                    Some(state.t("plain.activity.fs_watch_disabled").to_owned());
            }
        }
        SettingsMessage::FsDebounceSecs(s) => {
            // Like the refresh interval, 0 is a legitimate value here.
            if let Ok(n) = s.parse::<u32>() {
                state.config.fs_debounce_secs = n;
            }
            state.settings_edit.fs_debounce_secs = s;
        }
        SettingsMessage::SaveRequested => match save_config(&state.config, &state.paths) {
            Ok(()) => {
                state.settings_save_msg = Some(state.t("settings.saved_ok").to_owned());
                state.status_bar = Some(state.t("settings.saved_ok").to_owned());
            }
            Err(e) => {
                state.settings_save_msg = Some(format!("{} {e}", state.t("settings.save_error")));
            }
        },
        SettingsMessage::BackToDashboard => {
            state.screen = Screen::Dashboard;
        }
    }
    Task::none()
}

pub(super) fn handle_launch(state: &mut AppState, msg: LaunchMessage) -> Task<Message> {
    let (tool_path, file_path) = match msg {
        LaunchMessage::OpenInEditor(path) => (state.config.external_editor.clone(), path),
        LaunchMessage::OpenInMergeTool(path) => (state.config.external_merge_tool.clone(), path),
    };

    let Some(tool) = tool_path else {
        state.status_bar = Some(state.t("tool.not_configured").to_owned());
        return Task::none();
    };

    match std::process::Command::new(&tool).arg(&file_path).spawn() {
        Ok(_) => {
            state.status_bar = Some(format!(
                "{} {} {:?}",
                state.t("plain.activity.launched"),
                tool,
                file_path
            ));
        }
        Err(e) => {
            state.status_bar = Some(format!("{} {}: {e}", state.t("tool.launch_failed"), tool));
        }
    }
    Task::none()
}

pub(super) fn handle_topology(state: &mut AppState, msg: TopologyMessage) -> Task<Message> {
    match msg {
        TopologyMessage::ScanRequested => {
            let projects: Vec<_> = state
                .workspace
                .as_ref()
                .map(|ws| ws.projects.clone())
                .unwrap_or_default();
            state.topology.phase = TopologyPhase::Scanning;

            Task::perform(
                async move { VcsAdapter::scan_topology(&projects).await },
                |graph| Message::Background(BackgroundMessage::TopologyScanned(graph)),
            )
        }
    }
}

pub(super) fn handle_tag_push(state: &mut AppState, msg: TagPushMessage) -> Task<Message> {
    match msg {
        TagPushMessage::OfferShown {
            freeze_name,
            project_ids,
        } => {
            state.pending_tag_push = Some(PendingTagPush {
                freeze_name,
                project_ids,
                is_pushing: false,
            });
            Task::none()
        }

        TagPushMessage::PushConfirmed => {
            let push = match &state.pending_tag_push {
                Some(p) => p.clone(),
                None => return Task::none(),
            };
            let Some(lease_id) = shared::acquire_operation(state, OperationOwner::TagPush) else {
                return Task::none();
            };
            if let Some(ref mut p) = state.pending_tag_push {
                p.is_pushing = true;
            }

            let projects: Vec<_> = push
                .project_ids
                .iter()
                .filter_map(|id| shared::find_project(state, id))
                .collect();
            let tag_name = push.freeze_name.clone();
            let max = state.config.max_concurrent_reads;

            Task::perform(
                async move {
                    use std::sync::Arc;
                    use tokio::sync::Semaphore;

                    let sem = Arc::new(Semaphore::new(max));
                    let mut handles = Vec::new();
                    for project in projects {
                        let sem = Arc::clone(&sem);
                        let tag = tag_name.clone();
                        handles.push(tokio::spawn(async move {
                            let _permit = sem.acquire().await.expect("open");
                            knotra_vcs::VcsAdapter::push_tag(&project, &tag).await
                        }));
                    }
                    let mut results = Vec::new();
                    for h in handles {
                        if let Ok(r) = h.await {
                            results.push(r);
                        }
                    }
                    let success = results.iter().filter(|r| r.success).count();
                    let failed = results.iter().filter(|r| !r.success).count();
                    (success, failed)
                },
                move |(success_count, fail_count)| {
                    Message::Background(BackgroundMessage::TagPushCompleted {
                        lease_id,
                        success_count,
                        fail_count,
                    })
                },
            )
        }

        TagPushMessage::PushDeclined => {
            if state
                .pending_tag_push
                .as_ref()
                .is_some_and(|push| push.is_pushing)
            {
                return Task::none();
            }
            state.pending_tag_push = None;
            Task::none()
        }
    }
}

pub(super) fn handle_selection(state: &mut AppState, msg: SelectionMessage) -> Task<Message> {
    let ordered: Vec<knotra_vcs::ProjectId> = state.visible_project_ids();

    match msg {
        SelectionMessage::ModeEntered => state.selection_mode = true,
        SelectionMessage::ModeExited => state.clear_selection_mode(),
        SelectionMessage::Toggled(id) => {
            let active_ids: std::collections::HashSet<_> = ordered.iter().cloned().collect();
            if !active_ids.contains(&id) {
                return Task::none();
            }
            state.selection_mode = true; // selecting anything enters mode
            state.selection.toggle(id);
        }
        SelectionMessage::RangeTo(id) => {
            if !ordered.contains(&id) {
                return Task::none();
            }
            state.selection_mode = true;
            state.selection.select_range(&ordered, &id);
        }
        SelectionMessage::SelectAll => {
            state.selection_mode = true;
            let ids = state.visible_project_ids();
            state.selection.clear();
            state.selection.select_all(&ids);
        }
        SelectionMessage::Clear => state.clear_selection_mode(),
        SelectionMessage::FocusMoved(_) => {} // focus tracking only
    }
    Task::none()
}

pub(super) fn handle_palette(state: &mut AppState, msg: PaletteMessage) -> Task<Message> {
    match msg {
        PaletteMessage::Opened => {
            state.palette.open_palette();
            crate::state::palette::update_results(state);
            return focus_ops::open_overlay_focus(
                state,
                focus::FocusTarget::text_input(knotra_ui::widget::focus_id::PALETTE_QUERY.clone()),
            );
        }
        PaletteMessage::Closed => state.palette.close(),
        PaletteMessage::QueryChanged(q) => {
            state.palette.query = q;
            state.palette.notice_key = None;
            crate::state::palette::update_results(state);
        }
        PaletteMessage::MoveUp => {
            if state.palette.highlighted > 0 {
                state.palette.highlighted -= 1;
            }
        }
        PaletteMessage::MoveDown => {
            let max = state.palette.results.len().saturating_sub(1);
            if state.palette.highlighted < max {
                state.palette.highlighted += 1;
            }
        }
        PaletteMessage::Confirmed | PaletteMessage::EntryClicked(_) => {
            if let PaletteMessage::EntryClicked(i) = msg {
                state.palette.highlighted = i;
            }
            match crate::state::palette::dispatch_entry(state) {
                crate::state::palette::PaletteDispatch::Dispatched(msg) => {
                    state.palette.close();
                    return Task::done(msg);
                }
                crate::state::palette::PaletteDispatch::Disabled(reason) => {
                    state.palette.notice_key = Some(reason);
                }
                crate::state::palette::PaletteDispatch::Noop => {
                    state.palette.notice_key = Some("palette.disabled.unavailable");
                }
            }
        }
    }
    Task::none()
}

pub(super) fn handle_dashboard(state: &mut AppState, msg: DashboardMessage) -> Task<Message> {
    match msg {
        DashboardMessage::GroupingChanged(grouping) => {
            state.config.dashboard_grouping = grouping;
            persist_dashboard_preferences(state);
            state.reconcile_selection_with_display();
        }
        DashboardMessage::SortChanged(sort) => {
            state.config.dashboard_sort = sort;
            persist_dashboard_preferences(state);
        }
        DashboardMessage::TierToggled(tier) => {
            if state.config.dashboard_grouping == DashboardGrouping::Attention {
                match tier {
                    crate::state::dashboard::DashboardTier::NeedsHelp => {}
                    crate::state::dashboard::DashboardTier::InProgress => {
                        state.config.dashboard_in_progress_collapsed =
                            !state.config.dashboard_in_progress_collapsed;
                    }
                    crate::state::dashboard::DashboardTier::AllSet => {
                        state.config.dashboard_all_set_collapsed =
                            !state.config.dashboard_all_set_collapsed;
                    }
                }
                persist_dashboard_preferences(state);
                state.reconcile_selection_with_display();
            }
        }
        DashboardMessage::ErrorDetailsToggled => {
            if matches!(state.load_phase, LoadPhase::Error(_)) {
                state.dashboard_error_details_open = !state.dashboard_error_details_open;
            }
        }
        DashboardMessage::ErrorRetryRequested => {
            if matches!(state.load_phase, LoadPhase::Error(_)) && state.workspace.is_some() {
                state.is_refreshing = false;
                return workspace::handle_workspace(state, WorkspaceMessage::RefreshRequested);
            }
        }
        DashboardMessage::ToolbarOverflowToggled => {
            state.dashboard_toolbar_overflow_open = !state.dashboard_toolbar_overflow_open;
        }
        DashboardMessage::ToolbarSelectorsToggled => {
            state.dashboard_toolbar_selectors_open = !state.dashboard_toolbar_selectors_open;
        }
    }
    Task::none()
}

fn persist_dashboard_preferences(state: &mut AppState) {
    if let Err(error) = save_config(&state.config, &state.paths) {
        tracing::warn!("failed to persist dashboard preferences: {error}");
        state.status_bar = Some(state.t("dashboard.preference_save_failed").to_owned());
    }
}
