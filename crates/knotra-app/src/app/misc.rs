//! The short handlers (RFC-040 Stage 3 commit 7): project, history, settings,
//! external-tool launch, tag push, selection, palette, and dashboard-display
//! messages, plus the one helper used only by the last of those. Grouped by
//! size, not by a shared domain - each function here still owns exactly one
//! `Message` variant group end to end, same as every other `app/` module;
//! they are simply too small individually to each warrant their own file
//! (RFC-040 D1).
//!
//! RFC-043 Handoff 054: the documented `misc -> workspace` edge (RFC-040
//! D7, `.git-exclude/reviewed/089-...md`) existed solely for
//! `DashboardMessage::ErrorRetryRequested`'s call into
//! `handle_workspace`. That variant is gone with the rest of the dashboard
//! error path (`LoadPhase::Error` had zero production constructors), so
//! the edge and the `workspace` import it justified are gone too.
//!
//! RFC-044: `handle_topology` (`TopologyMessage::ScanRequested`) is gone —
//! scanning is automatic now (`shared::scan_topology_task`), triggered from
//! workspace load and every place the project set changes, not a message a
//! user sends.

use iced::Task;
use knotra_vcs::{
    VcsAdapter,
    model::operation::{OperationId, OperationKind, OperationLog, OperationResult},
};

use super::shared;
use crate::{
    config::{DashboardGrouping, save_config},
    message::{
        BackgroundMessage, DashboardMessage, HistoryMessage, LaunchMessage, Message,
        PaletteMessage, ProjectMessage, SelectionMessage, SettingsMessage, TagPushMessage,
    },
    state::{AppState, OperationOwner},
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
    }
    Task::none()
}

/// RFC-071 Amendment A7: every setting is saved when it changes — no
/// separate "Save settings" step. Each arm below applies the change to
/// `state.config` and then calls [`persist_config`], the one function every
/// setting-changing path in this app saves through — this one, zoom's
/// shortcuts (`app.rs`'s `ShortcutMessage::ZoomIn`/`ZoomOut`/`ZoomReset`,
/// which dispatch back into this function's `ZoomChanged` arm), and the
/// dashboard's Group/Sort/collapse (`handle_dashboard` below).
///
/// RFC-073 D12: `RefreshAutoChanged`/`MaxConcurrentChanged`/
/// `MaxLogEntriesChanged`/`FsDebounceSecs` now carry an already-valid
/// value chosen from a fixed list (one of `config.rs`'s own `*_STEPS`
/// arrays), the same shape `ZoomChanged` already had — RFC-038 Stage 2's
/// "parse the typed text, keep the edit buffer on failure" dance these four
/// used to do is gone with the typed fields themselves.
pub(super) fn handle_settings(state: &mut AppState, msg: SettingsMessage) -> Task<Message> {
    match msg {
        SettingsMessage::LocaleChanged(l) => {
            state.config.locale = l;
            state.catalog = knotra_ui::i18n::Catalog::for_locale(l);
            persist_config(state);
        }
        SettingsMessage::ThemeChanged(choice) => {
            state.config.theme = choice;
            state.theme = knotra_ui::KnotraTheme::from_choice(choice);
            persist_config(state);
        }
        SettingsMessage::ZoomChanged(percent) => {
            state.config.zoom_percent = percent;
            persist_config(state);
        }
        SettingsMessage::RefreshAutoChanged(secs) => {
            state.config.refresh_interval_secs = secs;
            persist_config(state);
        }
        SettingsMessage::MaxConcurrentChanged(n) => {
            state.config.max_concurrent_reads = n;
            persist_config(state);
        }
        SettingsMessage::ExternalEditorChanged(s) => {
            state.settings_edit.external_editor = s.clone();
            state.config.external_editor = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
            persist_config(state);
        }
        SettingsMessage::ExternalMergeToolChanged(s) => {
            state.settings_edit.external_merge_tool = s.clone();
            state.config.external_merge_tool = if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_owned())
            };
            persist_config(state);
        }
        SettingsMessage::MaxLogEntriesChanged(n) => {
            state.config.max_log_entries = n;
            persist_config(state);
        }
        SettingsMessage::FsWatchEnabledChanged(v) => {
            state.config.fs_watch_enabled = v;
            if !v {
                state.status_bar = Some(state.t("plain.activity.fs_watch_disabled").to_owned());
            }
            persist_config(state);
        }
        SettingsMessage::FsDebounceSecs(secs) => {
            state.config.fs_debounce_secs = secs;
            persist_config(state);
        }
        SettingsMessage::AdvancedToggled => {
            state.settings_advanced_open = !state.settings_advanced_open;
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

pub(super) fn handle_tag_push(state: &mut AppState, msg: TagPushMessage) -> Task<Message> {
    match msg {
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
        SelectionMessage::SelectAll => {
            state.selection_mode = true;
            let ids = state.visible_project_ids();
            state.selection.clear();
            state.selection.select_all(&ids);
        }
        SelectionMessage::Clear => state.clear_selection_mode(),
    }
    Task::none()
}

pub(super) fn handle_palette(state: &mut AppState, msg: PaletteMessage) -> Task<Message> {
    match msg {
        PaletteMessage::Opened => {
            state.palette.open_palette();
            crate::state::palette::update_results(state);
        }
        PaletteMessage::Closed => state.palette.close(),
        PaletteMessage::QueryChanged(q) => {
            state.palette.query = q;
            state.palette.notice_key = None;
            crate::state::palette::update_results(state);
        }
        PaletteMessage::EntryClicked(i) => match crate::state::palette::dispatch_entry(state, i) {
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
        },
    }
    Task::none()
}

pub(super) fn handle_dashboard(state: &mut AppState, msg: DashboardMessage) -> Task<Message> {
    match msg {
        DashboardMessage::GroupingChanged(grouping) => {
            state.config.dashboard_grouping = grouping;
            persist_config(state);
            state.reconcile_selection_with_display();
        }
        DashboardMessage::SortChanged(sort) => {
            state.config.dashboard_sort = sort;
            persist_config(state);
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
                persist_config(state);
                state.reconcile_selection_with_display();
            }
        }
        DashboardMessage::ToolbarOverflowToggled => {
            state.dashboard_toolbar_overflow_open = !state.dashboard_toolbar_overflow_open;
        }
        DashboardMessage::ArrangeToggled => {
            state.dashboard_arrange_open = !state.dashboard_arrange_open;
        }
    }
    Task::none()
}

/// RFC-071 Amendment A7: the one function every setting-changing path in
/// this app calls to persist `state.config` — originally just dashboard
/// Group/Sort/collapse (RFC-032, `handle_dashboard` above, under its old
/// name `persist_dashboard_preferences`), now generalized to Settings'
/// selects and typed fields (`handle_settings` above) and zoom (stage 4's
/// Settings select and shortcuts) as well.
///
/// A failed save is never silent, and never shown only where the change
/// happened to come from (A7's refinement, Review 259 §1: zoom's shortcuts
/// work outside Settings): it replaces both `state.status_bar` (the
/// activity line, read everywhere) and `state.settings_save_msg`
/// (`view/settings.rs`, read only while Settings is open) with the same
/// `settings.save_error`-prefixed text, so a zoom shortcut's failure is
/// exactly as visible as a Settings field's. A *successful* save shows
/// nothing in either place — A7's other refinement: a "Settings saved."
/// flash on every keystroke would be noise, not news — but does clear any
/// stale `settings_save_msg` a previous failure left behind, so a retry
/// that works removes the old error rather than leaving it stuck.
fn persist_config(state: &mut AppState) {
    match save_config(&state.config, &state.paths) {
        Ok(()) => state.settings_save_msg = None,
        Err(error) => {
            tracing::warn!("failed to persist settings: {error}");
            let msg = format!("{} {error}", state.t("settings.save_error"));
            state.settings_save_msg = Some(msg.clone());
            state.status_bar = Some(msg);
        }
    }
}
