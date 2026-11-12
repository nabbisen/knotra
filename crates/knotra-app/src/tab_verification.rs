//! Task 110 §4 — Tab, at level 2, not on a real window.
//!
//! iced 0.14 has no accessibility tree (confirmed: neither `iced` nor
//! `iced_winit` name `accesskit`/`a11y` anywhere, not even behind an
//! unselected optional feature), so the owner's GUI rules' AT-SPI path is
//! closed for this application specifically. And knotra moves keyboard
//! focus with its own model — `overlay_focus`, inside each layer's own
//! focus order (RFC-066) — never through a snora/iced widget-tree focus
//! at all. So the snora 0.53 → 0.57 upgrade can change only one thing
//! about Tab: whether the key **reaches** knotra's own
//! `app.rs::subscription`-driven `keyboard::listen()` while a dialog or
//! sheet is open, or whether snora's own overlay machinery now swallows
//! it the way it already swallows Escape (`escape_verification.rs`'s own
//! module doc). The `Simulator` answers that directly: a key tap that
//! reaches no handler in the view tree returns `event::Status::Ignored`
//! from `Simulator::tap_key`, and publishes no message — exactly what a
//! real window's behaviour requires here, since Tab itself is routed
//! entirely by `app.rs`'s subscription, which the `Simulator` never runs
//! at all; snora must not intercept the key before it gets the chance.
//!
//! One test per window in Task 107 §4's own table that takes Tab (Add
//! project at step 1 and step 2, Remove project, Save release point at
//! the ready check, Get latest safely at the plan, the Resolve sheet with
//! Stop's confirmation open, and Rename workspace — the command palette
//! and the workspace menu are excluded: that table's own Tab column
//! reads "—" for both). Each asserts both `Tab` and `Shift+Tab` return
//! `Ignored` and publish nothing.

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::message::Message;
    use crate::state::{
        AppState, conflict_ops::ConflictPhase, freezer::FreezerPhase, sync::SyncPhase,
    };
    use iced_test::core::{Size, event, keyboard};
    use knotra_vcs::model::conflict::ProjectConflictDetail;
    use knotra_vcs::model::operation::{FreezeValidation, OperationId, SmartPullPlan};
    use knotra_vcs::{Project, Workspace};

    fn make_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    /// `Event::Keyboard(KeyPressed { modifiers, .. })` for Tab/Shift+Tab —
    /// built by hand, since `Simulator::tap_key`'s own convenience
    /// wrapper (and the free `tap_key`/`press_key` functions it calls)
    /// hard-code `Modifiers::default()`, with no way to ask for Shift
    /// through it.
    fn tab_event(shift: bool) -> [iced_test::core::Event; 2] {
        let key = keyboard::Key::Named(keyboard::key::Named::Tab);
        let modifiers = if shift {
            keyboard::Modifiers::SHIFT
        } else {
            keyboard::Modifiers::default()
        };
        let physical_key =
            keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified);
        [
            iced_test::core::Event::Keyboard(keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key.clone(),
                physical_key,
                location: keyboard::Location::Standard,
                modifiers,
                repeat: false,
                text: None,
            }),
            iced_test::core::Event::Keyboard(keyboard::Event::KeyReleased {
                key: key.clone(),
                modified_key: key,
                physical_key,
                location: keyboard::Location::Standard,
                modifiers,
            }),
        ]
    }

    /// Taps Tab, then Shift+Tab, over the full `app_view`: both must
    /// return `Ignored` (knotra's own `keyboard::listen()` receives them,
    /// unlike Escape — `escape_verification.rs`'s own on_close_modals
    /// capture), and neither must publish a message of snora's.
    fn assert_tab_reaches_knotra(state: &AppState) {
        let mut ui =
            crate::headless::simulator(crate::view::app_view(state), Size::new(1024.0, 768.0));

        let tab_statuses = ui.simulate(tab_event(false));
        assert!(
            tab_statuses.iter().all(|s| *s == event::Status::Ignored),
            "Tab must be Ignored (reach knotra's own subscription), got {tab_statuses:?}"
        );
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "Tab must publish no message of snora's; got {messages:?}"
        );

        let mut ui =
            crate::headless::simulator(crate::view::app_view(state), Size::new(1024.0, 768.0));
        let shift_tab_statuses = ui.simulate(tab_event(true));
        assert!(
            shift_tab_statuses
                .iter()
                .all(|s| *s == event::Status::Ignored),
            "Shift+Tab must be Ignored, got {shift_tab_statuses:?}"
        );
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "Shift+Tab must publish no message of snora's; got {messages:?}"
        );
    }

    #[test]
    fn tab_reaches_knotra_add_project_step_1() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::ChooseFolder,
            ..Default::default()
        });
        assert_tab_reaches_knotra(&state);
    }

    #[test]
    fn tab_reaches_knotra_add_project_step_2() {
        let mut state = make_state();
        state.add_project_dialog = Some(crate::state::AddProjectDialog {
            step: crate::state::AddProjectStep::NameProject,
            path: "/tmp/some-project".to_owned(),
            ..Default::default()
        });
        assert_tab_reaches_knotra(&state);
    }

    #[test]
    fn tab_reaches_knotra_remove_project() {
        let mut state = make_state();
        state.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "Alpha".to_owned(),
        });
        assert_tab_reaches_knotra(&state);
    }

    #[test]
    fn tab_reaches_knotra_save_release_point_at_the_ready_check() {
        let mut state = make_state();
        state.active_modal = crate::state::ActiveModal::Tag;
        state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
            freeze_name: "v1.0.0".to_owned(),
            entries: vec![],
        });
        assert_tab_reaches_knotra(&state);
    }

    #[test]
    fn tab_reaches_knotra_get_latest_safely_at_the_plan() {
        let mut state = make_state();
        state.active_modal = crate::state::ActiveModal::Pull;
        state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
            id: OperationId::new(),
            entries: vec![],
        });
        assert_tab_reaches_knotra(&state);
    }

    #[test]
    fn tab_reaches_knotra_resolve_with_stop_confirmation_open() {
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
                project_id,
                project_name: "Alpha".to_owned(),
                conflicted_files: vec![],
                read_error: None,
            },
        };
        assert_tab_reaches_knotra(&state);
    }

    /// Rename workspace — the task's own extra question: what Tab and
    /// Escape each do while the dialog's text field has keyboard focus
    /// (simulated here by clicking into it, giving iced's own text-input
    /// state the focus a real window's pointer click would). Found by
    /// running it, not assumed — and the two keys behave differently:
    ///
    /// - **Tab** still leaves the field for the next control: `Ignored`,
    ///   reaching knotra the same as every other row.
    /// - **Escape does not close the dialog while the field is
    ///   focused.** `tap_key` itself reports `event::Status::Captured`,
    ///   and no message is published at all — iced's own `text_input`
    ///   widget, being the focused control, consumes Escape before
    ///   snora's `on_close_modals` (sitting above it in the tree) ever
    ///   sees it. This is unrelated to the snora upgrade (nothing here
    ///   changed by it) and is not this task's to fix, only to report:
    ///   a user editing the field who presses Escape expecting to cancel
    ///   the dialog gets nothing on that first press; a second press,
    ///   once the field itself has lost focus, would close it.
    #[test]
    fn tab_and_escape_in_rename_workspace_with_the_field_focused() {
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

        // Give the field focus, the way a real click would.
        let mut ui =
            crate::headless::simulator(crate::view::app_view(&state), Size::new(1024.0, 768.0));
        ui.click("Renamed")
            .expect("the rename field must be found and clickable");

        let tab_statuses = ui.simulate(tab_event(false));
        assert!(
            tab_statuses.iter().all(|s| *s == event::Status::Ignored),
            "Tab with the rename field focused must still be Ignored (leave the field for the \
             next control, not be consumed by it); got {tab_statuses:?}"
        );
        // One combined drain: the click itself (giving the field focus)
        // must not have published anything either, same as Tab.
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "neither the click nor Tab with the field focused may publish a message; \
             got {messages:?}"
        );

        // Separately: Escape with the field focused — found, not
        // assumed, to be `Captured` by the field itself, publishing
        // nothing; see this test's own doc comment above for why, and
        // what that means for a user.
        let mut ui =
            crate::headless::simulator(crate::view::app_view(&state), Size::new(1024.0, 768.0));
        ui.click("Renamed")
            .expect("the rename field must be found and clickable");
        let escape_status = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));
        assert_eq!(
            escape_status,
            event::Status::Captured,
            "expected the focused text field to capture Escape itself"
        );
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "Escape captured by the focused field must publish no message (the dialog does not \
             close on this press); got {messages:?}"
        );
    }
}
