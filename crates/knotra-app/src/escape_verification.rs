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
//!
//! **Task 111:** every window whose entry focus is a text input — the
//! command palette, Add project (both steps), Rename workspace and New
//! workspace — gets a second test starting from that field focused
//! (clicked into first, the same way `click_then_tap_escape` below and
//! `tab_verification.rs`'s Rename test do), proving `escape_first` sees
//! Escape before the field does, not only the unfocused case every row
//! already covered. Before Task 111's fix, `tab_and_escape_in_rename_
//! workspace_with_the_field_focused` (`tab_verification.rs`) showed the
//! defect directly: `Captured`, zero messages — the dialog needed a
//! second press. The focused-field rows here are this file's own
//! equivalent demonstration, one per window, now passing.
//!
//! An input-method composition keeps its own Escape — the module doc on
//! `knotra_ui::widget::escape_first` explains why — exercised at the end
//! of this file.

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

    /// Clicks into the field whose current value renders as `field_text`
    /// (giving it iced's own text-input focus, the way a real pointer
    /// click would), then taps Escape over the same `app_view` — Task
    /// 111's own scenario: the first Escape press while the cursor is in
    /// a field.
    fn click_then_tap_escape(state: &AppState, field_text: &str) -> Vec<Message> {
        let mut ui =
            crate::headless::simulator(crate::view::app_view(state), Size::new(1024.0, 768.0));
        ui.click(field_text).unwrap_or_else(|e| {
            panic!("the field showing {field_text:?} must be clickable: {e:?}")
        });
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

    /// Task 111: the same window, but the first Escape press comes while
    /// the search field itself has focus — the state a real user opening
    /// the palette is actually in.
    #[test]
    fn escape_closes_the_command_palette_with_the_field_focused() {
        let mut state = make_state();
        state.palette.open = true;
        state.palette.query = "palette-escape-focus".to_owned();

        let messages = click_then_tap_escape(&state, "palette-escape-focus");
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

    /// Task 111: the same step, but the first Escape press comes while
    /// the path field itself has focus.
    #[test]
    fn escape_closes_add_project_at_step_1_with_the_field_focused() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::ChooseFolder,
            path: "/tmp/add-project-escape-focus".to_owned(),
            ..Default::default()
        });

        let messages = click_then_tap_escape(&state, "/tmp/add-project-escape-focus");
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

    /// Task 111: step 2, but the first Escape press comes while the name
    /// field itself has focus.
    #[test]
    fn escape_closes_add_project_at_step_2_with_the_field_focused() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::NameProject,
            path: "/tmp/some-project".to_owned(),
            name: "add-project-escape-focus-name".to_owned(),
            ..Default::default()
        });

        let messages = click_then_tap_escape(&state, "add-project-escape-focus-name");
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
    // A dialog with a text field (Rename workspace): Escape closes it,
    // whether or not the field has focus (Task 111 — the focused case is
    // now distinguishable, and tested, not just asserted away: see this
    // file's own module doc, and `tab_verification.rs`'s own Rename test
    // for the pre-fix failure this exact scenario showed).
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

    /// Task 111: the same dialog, but the first Escape press comes while
    /// the name field itself has focus — the exact scenario Review 281 §2
    /// reported and `tab_verification.rs`'s own Rename test showed failing
    /// before this task's fix.
    #[test]
    fn escape_closes_rename_workspace_dialog_with_the_field_focused() {
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

        let messages = click_then_tap_escape(&state, "Renamed");
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.workspace_mgr.rename_dialog.is_none(),
            "the rename dialog must be closed"
        );
    }

    // -----------------------------------------------------------------------
    // New workspace (the create dialog) — not in Task 107 §4's own table
    // (it did not exist in 0.53 or carry a Tab/Escape row there), but named
    // in Task 111 §1 as a window that opens with the cursor in a field.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_closes_create_workspace_dialog() {
        let mut state = make_state();
        state.workspace_mgr.create_dialog =
            Some(crate::state::workspace_mgr::CreateWorkspaceDialog {
                name: "new-workspace-escape-focus".to_owned(),
                error: None,
            });

        let messages = tap_escape(&state);
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.workspace_mgr.create_dialog.is_none(),
            "the create dialog must be closed"
        );
    }

    /// Task 111: the same dialog, but the first Escape press comes while
    /// the name field itself has focus.
    #[test]
    fn escape_closes_create_workspace_dialog_with_the_field_focused() {
        let mut state = make_state();
        state.workspace_mgr.create_dialog =
            Some(crate::state::workspace_mgr::CreateWorkspaceDialog {
                name: "new-workspace-escape-focus".to_owned(),
                error: None,
            });

        let messages = click_then_tap_escape(&state, "new-workspace-escape-focus");
        assert_exactly_one_close(&messages);

        let _ = crate::app::update(&mut state, messages.into_iter().next().unwrap());
        assert!(
            state.workspace_mgr.create_dialog.is_none(),
            "the create dialog must be closed"
        );
    }

    // -----------------------------------------------------------------------
    // Task 111 §3: an input-method composition keeps its own Escape — the
    // key belongs to the IME, which is what cancels it, not to knotra's
    // close logic. Add project step 1 stands in for "any field"; nothing
    // here is specific to that window.
    // -----------------------------------------------------------------------
    #[test]
    fn escape_during_a_non_empty_preedit_does_not_close_the_window() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::ChooseFolder,
            path: "/tmp/ime-escape-test".to_owned(),
            ..Default::default()
        });

        let mut ui =
            crate::headless::simulator(crate::view::app_view(&state), Size::new(1024.0, 768.0));
        ui.click("/tmp/ime-escape-test")
            .expect("the path field must be clickable");

        // A composition is in progress (e.g. typing Japanese) — the
        // pre-edit is non-empty.
        let _ = ui.simulate([iced_test::core::Event::InputMethod(
            iced_test::core::input_method::Event::Preedit("あ".to_owned(), None),
        )]);

        let _ = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "Escape must not close the window while a composition is in progress; \
             got {messages:?}"
        );
    }

    /// Clears the pre-edit with an empty `Preedit`, not a `Commit` —
    /// `escape_first` tracks both the same way (its own doc comment), but
    /// a real `Commit` also inserts the committed text into the field's
    /// value, publishing an `on_input` message of the field's own that
    /// would be noise here; an empty `Preedit` isolates the one thing
    /// this test checks — that clearing the pre-edit, by either route,
    /// gives Escape back to this wrapper.
    #[test]
    fn escape_after_the_preedit_clears_closes_the_window() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::ChooseFolder,
            path: "/tmp/ime-escape-test".to_owned(),
            ..Default::default()
        });

        let mut ui =
            crate::headless::simulator(crate::view::app_view(&state), Size::new(1024.0, 768.0));
        ui.click("/tmp/ime-escape-test")
            .expect("the path field must be clickable");

        let _ = ui.simulate([iced_test::core::Event::InputMethod(
            iced_test::core::input_method::Event::Preedit("あ".to_owned(), None),
        )]);
        let _ = ui.simulate([iced_test::core::Event::InputMethod(
            iced_test::core::input_method::Event::Preedit(String::new(), None),
        )]);

        let _ = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_exactly_one_close(&messages);
    }
}
