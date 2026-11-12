//! Task 107 §4 Level 2 — Escape closes each window exactly once, through
//! snora 0.57's own `on_close_modals`/`on_close_menus`, the same way a
//! real window's `app.rs::subscription`-driven Escape already does
//! (`close_topmost_layer`). Each test builds the state with that window
//! open, taps Escape through the headless harness, asserts the message(s)
//! `into_messages()` gives, then feeds that message to `crate::app::update`
//! and asserts the resulting state — the two-step check Task 107 §4 asks
//! for, not just "a message came out".
//!
//! Every row but one asserts exactly one `Message::Shortcut(ShortcutMessage
//! ::Close)`, matching every other window's own close message
//! (`view.rs`'s `on_close_modals`). The workspace menu is the documented
//! exception: it is a `header_menu`, not a `dialog`/`sheet`, dismissed by
//! `on_close_menus`, wired to `Message::Workspace(WorkspaceMessage::
//! SwitcherToggled)` instead (`view.rs`'s own `.on_close_menus(...)` call).

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::message::{Message, ShortcutMessage, WorkspaceMessage};
    use crate::state::{
        AppState, conflict_ops::ConflictPhase, freezer::FreezerPhase, sync::SyncPhase,
    };
    use iced_test::core::{Size, keyboard};
    use knotra_vcs::model::conflict::ProjectConflictDetail;
    use knotra_vcs::model::operation::{FreezeValidation, OperationId, SmartPullPlan};
    use knotra_vcs::{Project, Workspace};

    fn make_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    /// Taps Escape over the full `app_view`, asserts the messages, and
    /// returns them so each test can feed one into `update` and check the
    /// state after — the two assertions Task 107 §4's own Level 2 spec
    /// asks every row to make.
    fn tap_escape(state: &AppState) -> Vec<Message> {
        let mut ui =
            crate::headless::simulator(crate::view::app_view(state), Size::new(1024.0, 768.0));
        let _ = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));
        ui.into_messages().collect()
    }

    fn assert_exactly_one_close(messages: &[Message]) {
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            matches!(messages[0], Message::Shortcut(ShortcutMessage::Close)),
            "expected exactly one Close message; got {messages:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Command palette.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_the_command_palette() {
        let mut state = make_state();
        state.palette.open = true;

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(!state.palette.open, "the palette must be closed");
    }

    // -----------------------------------------------------------------------
    // Add project, step 1 (choose folder) and step 2 (name it) — Task 095's
    // own ruling: Escape at step 2 closes the whole dialog, not a Back to
    // step 1.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_add_project_at_step_1() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::ChooseFolder,
            ..Default::default()
        });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.add_project_dialog.is_none(),
            "the dialog must be closed, not stepped back"
        );
    }

    #[test]
    fn escape_closes_add_project_at_step_2_not_back_to_step_1() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::NameProject,
            path: "/tmp/some-project".to_owned(),
            ..Default::default()
        });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.add_project_dialog.is_none(),
            "Escape at step 2 must close the whole dialog (Task 095), not step back to step 1"
        );
    }

    // -----------------------------------------------------------------------
    // Remove project — already covered by
    // headless_probe.rs::escape_in_a_dialog_produces_exactly_one_close_message.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Save release point, at the ready check (FreezerPhase::ValidationReady).
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_save_release_point_at_the_ready_check() {
        let mut state = make_state();
        state.active_modal = crate::state::ActiveModal::Tag;
        state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
            freeze_name: "v1.0.0".to_owned(),
            entries: vec![],
        });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert_eq!(
            state.active_modal,
            crate::state::ActiveModal::None,
            "the modal must be closed"
        );
    }

    // -----------------------------------------------------------------------
    // Get latest safely, at the plan (SyncPhase::AwaitingConfirm).
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_get_latest_safely_at_the_plan_starting_nothing() {
        let mut state = make_state();
        state.active_modal = crate::state::ActiveModal::Pull;
        state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
            id: OperationId::new(),
            entries: vec![],
        });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert_eq!(
            state.active_modal,
            crate::state::ActiveModal::None,
            "the modal must be closed"
        );
        assert!(
            !matches!(state.sync.phase, SyncPhase::PullRunning { .. }),
            "closing at the plan must not start the pull — got {:?}",
            state.sync.phase
        );
    }

    // -----------------------------------------------------------------------
    // Resolve sheet, with Stop's confirmation open (RFC-067): Escape
    // dismisses the confirmation and keeps the sheet, through the same
    // Close message every other window sends — `conflict::close_message`
    // reads `ConflictPhase::ConfirmStop` and answers `StopCancelled`
    // instead of the ordinary `PanelClosed`.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_on_resolve_with_stop_confirmation_open_dismisses_the_confirmation_and_keeps_the_sheet()
     {
        let mut state = make_state();
        let project = Project::new("Alpha", "/tmp/alpha");
        let project_id = project.id.clone();
        let mut workspace = Workspace::new("Demo");
        workspace.projects.push(project);
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();

        state.active_modal = crate::state::ActiveModal::Resolve(project_id.clone());
        state.conflict_ops.phase = ConflictPhase::ConfirmStop {
            project_id: project_id.clone(),
            detail: ProjectConflictDetail {
                project_id: project_id.clone(),
                project_name: "Alpha".to_owned(),
                conflicted_files: vec![],
                read_error: None,
            },
        };

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert_eq!(
            state.active_modal,
            crate::state::ActiveModal::Resolve(project_id.clone()),
            "the sheet must stay open — cancelling the confirmation means \"keep fixing\", \
             not \"close the whole panel\""
        );
        assert!(
            matches!(state.conflict_ops.phase, ConflictPhase::Browsing { .. }),
            "the confirmation must be dismissed, back to the file list: got {:?}",
            state.conflict_ops.phase
        );
    }

    // -----------------------------------------------------------------------
    // The workspace menu (with two workspaces): new in 0.55, through
    // on_close_menus — a different message than every window above,
    // `WorkspaceMessage::SwitcherToggled` (`view.rs`'s own
    // `.on_close_menus(...)` call), not `ShortcutMessage::Close`.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_the_workspace_menu_through_on_close_menus() {
        let mut state = make_state();
        state.all_workspaces = vec![Workspace::new("Demo"), Workspace::new("Side project")];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_mgr.switcher_open = true;

        let messages = tap_escape(&state);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            matches!(
                messages[0],
                Message::Workspace(WorkspaceMessage::SwitcherToggled)
            ),
            "expected SwitcherToggled (on_close_menus), not Shortcut::Close; got {messages:?}"
        );

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            !state.workspace_mgr.switcher_open,
            "the workspace menu must be closed"
        );
    }

    // -----------------------------------------------------------------------
    // A dialog with a text field (Rename workspace): Escape closes it.
    // What Escape does while the field itself has focus is not
    // distinguishable here — the Simulator drives no subscription, and
    // on_close_modals captures Escape at the view-tree level regardless of
    // which control last had focus; see this file's own module doc.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_rename_workspace_dialog() {
        let mut state = make_state();
        let workspace = Workspace::new("Demo");
        let workspace_id = workspace.id.clone();
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_mgr.rename_dialog =
            Some(crate::state::workspace_mgr::RenameWorkspaceDialog {
                workspace_id,
                new_name: "Renamed".to_owned(),
                error: None,
            });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.workspace_mgr.rename_dialog.is_none(),
            "the rename dialog must be closed"
        );
    }
}
