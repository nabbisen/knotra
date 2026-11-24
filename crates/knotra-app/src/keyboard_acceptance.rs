//! RFC-075 handoff stage 2 §5 — the page's own keyboard acceptance tests:
//! the real `Message`/`update`/`view` path, driven through `knotra_ui::
//! test_support::Harness`, exactly the shape the handoff's own table asks
//! for ("the key's message through `update`, then every widget operation
//! in the returned task, run on `view(&state)` in the harness").
//!
//! **Not `Harness::tab()`**: that method calls `snora::keyboard::traverse`
//! directly (by design, for primitive-level tests that do not need
//! wrapping). The page wraps (Amendment A1), which only `app::update`'s
//! own `handle_shortcut` — through `knotra_ui::widget::keyboard::
//! traverse_wrapping` — does. [`press`] drives the real message instead,
//! taking its returned `Task` apart the same way `knotra-ui`'s own
//! `widget::keyboard` tests do for the identical reason — and rebuilds the
//! harness's own tree from the *post-`update`* state first, so a press
//! that also changes what `view` draws (`FocusSearchBare`'s screen
//! switch) runs its `Task` against the tree it would really run against,
//! not the one `view` drew before the message arrived.
//!
//! `app_view(state)` borrows from `state`, but `Harness::new` needs a
//! `'static` `Element` — the same mismatch every "whole app view" test
//! this harness was built to eventually support would hit, and `state`
//! here is mutated between presses (`update` itself needs `&mut
//! AppState`), so leaking a single allocation once (as `Box::leak` would)
//! cannot work: the leaked, `'static`-borrowed tree would alias any later
//! `&mut state` the borrow checker must then refuse. [`extend`] is the
//! usual fix for exactly this "a test's data outlives nothing but needs a
//! `'static` view of it" shape — see its own doc comment for why it is
//! sound here and `Box::leak` is not.

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::message::{
        DashboardMessage, FilterMessage, Message, SelectionMessage, ShortcutMessage, StatusFilter,
    };
    use crate::state::{AppState, ArrangeSelect, Screen};
    use crate::view::{app_view, ids};
    use iced::widget::Id;
    use iced_test::core::Size;
    use knotra_ui::test_support::Harness;
    use knotra_vcs::model::status::{RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    /// Breaks the lifetime correlation a plain `&state` reborrow would
    /// keep, the same shape `Box::leak` provides but without leaking a
    /// fresh allocation on every single key press: the function boundary
    /// gives the caller a `&'static AppState` with no tracked connection
    /// back to `state`'s own binding, so `state` can be mutably borrowed
    /// again right after, as every test below does between presses.
    ///
    /// # Safety
    /// Sound only because every `Element`/`Harness` built from the
    /// returned reference is dropped, or simply never read again, before
    /// `state` is next mutated — true by construction in every test
    /// here: each call site immediately consumes the reference to build
    /// one `view()` call's output, never holding it past that.
    fn extend(state: &AppState) -> &'static AppState {
        unsafe { std::mem::transmute::<&AppState, &'static AppState>(state) }
    }

    fn harness(state: &AppState) -> Harness<Message> {
        Harness::new(
            app_view(extend(state)),
            Size::new(1100.0, 900.0),
            Some(knotra_ui::widget::icon::FONT_BYTES),
        )
    }

    /// Drives `ShortcutMessage` `msg` through the real `app::update`
    /// (mutating `state` synchronously, exactly as the live app does),
    /// rebuilds `h`'s tree from the state `update` leaves behind, then
    /// runs every widget operation the returned `Task` yields against
    /// that rebuilt tree — the one path this file's tests use to move or
    /// query real focus.
    fn press(h: Harness<Message>, state: &mut AppState, msg: ShortcutMessage) -> Harness<Message> {
        use iced::futures::StreamExt;
        let task = crate::app::update(state, Message::Shortcut(msg));
        let mut h = h.rebuilt(app_view(extend(state)));
        let Some(mut stream) = iced_test::runtime::task::into_stream(task) else {
            return h;
        };
        loop {
            match iced::futures::executor::block_on(stream.next()) {
                Some(iced_test::runtime::Action::Widget(op)) => h.run_operation(op),
                Some(_) => continue,
                None => break,
            }
        }
        h
    }

    fn tab(h: Harness<Message>, state: &mut AppState) -> Harness<Message> {
        press(h, state, ShortcutMessage::FocusNext)
    }

    fn make_project_status(id: ProjectId) -> ProjectStatus {
        ProjectStatus {
            project_id: id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: Default::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: Default::default(),
            refreshed_at: jiff::Timestamp::now(),
            read_error: None,
        }
    }

    /// One workspace, one project only, in the NeedsHelp tier (no status
    /// registered at all — `classify`'s own `StatusUnknown` fallback) —
    /// small enough that every id in the page's full Tab sequence can be
    /// named and checked, which is this table row's own requirement ("the
    /// full sequence by id").
    fn one_project_state() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let project_id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        (state, project_id)
    }

    #[test]
    fn tab_sequence_follows_reading_order_and_wraps_both_ways() {
        let (mut state, alpha_id) = one_project_state();
        let mut h = harness(&state);

        // Reading order: the shell (one workspace, so no switcher), the
        // toolbar (five chips, Arrange, the search field, Select — Clear
        // filters is not drawn, no filter is active), then the one
        // NeedsHelp row (no section toggle — NeedsHelp never has one —
        // name then action).
        let expected: Vec<Id> = vec![
            // `nav_dashboard` is skipped: it is already the active
            // destination (`AppState::new`'s own default screen), so its
            // `on_press_maybe` is `None` — disabled, not a Tab stop (D4 b).
            ids::shell::nav_history(),
            ids::shell::refresh(),
            ids::shell::palette(),
            ids::shell::settings(),
            ids::toolbar::filter_needs_help(),
            ids::toolbar::filter_dirty(),
            ids::toolbar::filter_behind(),
            ids::toolbar::filter_ahead(),
            ids::toolbar::filter_all_set(),
            ids::toolbar::arrange(),
            knotra_ui::widget::focus_id::SEARCH.clone(),
            ids::toolbar::select_mode(),
            ids::row::name(&alpha_id),
            ids::row::action(&alpha_id),
        ];

        for want in &expected {
            h = tab(h, &mut state);
            assert_eq!(h.focused_id(), Some(want.clone()), "expected {want:?}");
        }

        // Past the last control, Tab wraps to the first (Amendment A1) —
        // not "leaves the group for one press" (snora's own, un-wrapped
        // default).
        h = tab(h, &mut state);
        assert_eq!(
            h.focused_id(),
            Some(expected[0].clone()),
            "Tab past the last control must wrap to the first"
        );

        // Shift+Tab from the first wraps to the last.
        h = press(h, &mut state, ShortcutMessage::FocusPrevious);
        assert_eq!(
            h.focused_id(),
            Some(expected.last().unwrap().clone()),
            "Shift+Tab before the first control must wrap to the last"
        );
    }

    /// RFC-075 stage 0 inventory's two order differences — fixed by
    /// construction once Tab follows the drawn tree. Checked directly:
    /// the selection bar's Exit selection is reachable right after the
    /// toolbar's own last control, before any of the four bulk actions
    /// (it is drawn first).
    #[test]
    fn selection_bar_exit_comes_before_the_bulk_actions_as_drawn() {
        let (mut state, _alpha_id) = one_project_state();
        state.selection_mode = true;
        let mut h = harness(&state);

        // Shell (4) + chips (5) + Arrange + search + Select + the row's
        // checkbox + name + action = 15 presses reaches the row's own
        // action; the 16th reaches Exit selection.
        for _ in 0..16 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::selection_bar::exit()));
    }

    /// Disabled controls are skipped entirely (D4 b) — Select, with
    /// nothing selectable (no visible projects), is never reached.
    #[test]
    fn disabled_select_mode_is_skipped() {
        let (mut state, _alpha_id) = one_project_state();
        state.filter.search_text = "does-not-match-anything".to_owned();
        let mut h = harness(&state);

        // Shell (4) + chips (5) + Arrange + search = 11 presses reaches
        // the search field; the 12th (a filter is active) reaches Clear
        // filters.
        for _ in 0..12 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::toolbar::clear_filters()));

        h = tab(h, &mut state);
        assert_ne!(
            h.focused_id(),
            Some(ids::toolbar::select_mode()),
            "Select must be skipped while disabled"
        );
    }

    /// Enter presses the focused control once — exactly one message,
    /// through the real event path (the widget's own capture, not
    /// `ActivateFocused`).
    #[test]
    fn enter_presses_a_chip_once() {
        let (mut state, _alpha_id) = one_project_state();
        let mut h = harness(&state);

        for _ in 0..5 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::toolbar::filter_needs_help()));
        h.enter();
        assert!(matches!(
            h.messages(),
            [Message::Filter(FilterMessage::StatusFilterToggled(
                StatusFilter::NeedsHelp
            ))]
        ));
    }

    /// A row's checkbox and an Arrange select both press once too — the
    /// handoff's own table names these alongside a button and a chip.
    #[test]
    fn space_presses_a_row_checkbox_once() {
        let (mut state, alpha_id) = one_project_state();
        state.selection_mode = true;
        let mut h = harness(&state);

        // Shell (4) + chips (5) + Arrange + search + Select = 12 presses
        // reaches Select; the 13th reaches the row's own checkbox.
        for _ in 0..13 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::row::checkbox(&alpha_id)));
        h.space();
        assert!(matches!(
            h.messages(),
            [Message::Selection(SelectionMessage::Toggled(id))] if *id == alpha_id
        ));
    }

    /// RFC-075 D6: `/` focuses the search field directly, with no Tab
    /// needed first — `focus_search` switches `state.screen` and issues
    /// `operation::focus(SEARCH)` in the same `Task`; `press`'s own
    /// rebuild-before-draining order is what lets this land on a tree
    /// that actually has the field in it.
    #[test]
    fn bare_slash_focuses_the_search_field() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace {
            projects: vec![Project::new("Alpha", "/tmp")],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.screen = Screen::Settings;
        let h = harness(&state);

        let mut h = press(h, &mut state, ShortcutMessage::FocusSearchBare);
        assert_eq!(state.screen, Screen::Dashboard);
        assert_eq!(
            h.focused_id(),
            Some(knotra_ui::widget::focus_id::SEARCH.clone())
        );
    }

    /// RFC-075 D6: ↓ ↑ j k move among project-name ids, wrapping; off-card
    /// focus (the toolbar) lands on the first name either way.
    #[test]
    fn arrow_and_jk_move_among_project_names_and_wrap() {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        let beta = Project::new("Beta", "/tmp");
        let (alpha_id, beta_id) = (alpha.id.clone(), beta.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![alpha, beta],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![make_project_status(beta_id.clone())],
            last_refresh: None,
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.config.dashboard_all_set_collapsed = false;
        state.config.dashboard_in_progress_collapsed = false;
        let h = harness(&state);

        let mut h = press(h, &mut state, ShortcutMessage::ArrowDown);
        assert_eq!(h.focused_id(), Some(ids::row::name(&alpha_id)));

        h = press(h, &mut state, ShortcutMessage::CardFocusNext); // j
        assert_eq!(h.focused_id(), Some(ids::row::name(&beta_id)));

        h = press(h, &mut state, ShortcutMessage::ArrowDown);
        assert_eq!(
            h.focused_id(),
            Some(ids::row::name(&alpha_id)),
            "↓ past the last project must wrap to the first"
        );

        let mut h = press(h, &mut state, ShortcutMessage::ArrowUp);
        assert_eq!(
            h.focused_id(),
            Some(ids::row::name(&beta_id)),
            "↑ before the first project must wrap to the last"
        );
    }

    /// RFC-075 §2: ↓ on a focused, closed select opens it, and does not
    /// also move card focus between projects — the message published is
    /// the select's own open-change, never a card move.
    #[test]
    fn arrow_down_on_a_closed_select_opens_it_not_card_focus() {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![alpha],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.dashboard_arrange_open = true;
        let mut h = harness(&state);

        // Shell (4) + chips (5) + Arrange + search + Select = 12 presses
        // (the panel's own Group/Sort selects are drawn *below* the
        // Arrange/search/Select row, not right after Arrange); the 13th
        // reaches the open panel's first select (Group).
        for _ in 0..13 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::toolbar::arrange_group()));

        h.arrow_down();
        assert!(matches!(
            h.messages(),
            [Message::Dashboard(
                DashboardMessage::ArrangeSelectOpenChanged(ArrangeSelect::Group, true)
            )]
        ));
    }

    /// RFC-075 §3: the two Arrange selects' open state — at most one
    /// open, choosing/closing the panel/leaving the dashboard close it.
    #[test]
    fn arrange_select_open_state_follows_its_own_rules() {
        let mut state = AppState::new(AppConfig::default());
        state.arrange_select_open = Some(ArrangeSelect::Group);
        state.dashboard_arrange_open = true;

        // Closing the panel closes the select too.
        let _ = crate::app::update(
            &mut state,
            Message::Dashboard(DashboardMessage::ArrangeToggled),
        );
        assert_eq!(state.arrange_select_open, None);
        assert!(!state.dashboard_arrange_open);

        // Opening Sort closes Group, by construction (`Option`, not two
        // booleans).
        state.arrange_select_open = Some(ArrangeSelect::Group);
        let _ = crate::app::update(
            &mut state,
            Message::Dashboard(DashboardMessage::ArrangeSelectOpenChanged(
                ArrangeSelect::Sort,
                true,
            )),
        );
        assert_eq!(state.arrange_select_open, Some(ArrangeSelect::Sort));

        // Leaving the dashboard closes whichever select was open.
        let _ = crate::app::update(&mut state, Message::Navigate(Screen::Settings));
        assert_eq!(state.arrange_select_open, None);
    }

    /// RFC-075 stage 0 inventory §2d: both empty-state buttons are real
    /// Tab stops now.
    #[test]
    fn empty_workspace_add_first_project_is_reachable() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace::new("Main"));
        state.load_phase = crate::state::LoadPhase::Ready;
        let mut h = harness(&state);

        for _ in 0..5 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::empty::add_first_project()));
    }

    #[test]
    fn no_matches_clear_filters_is_reachable() {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![alpha],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.filter.search_text = "does-not-match-anything".to_owned();
        let mut h = harness(&state);

        // Shell (4) + chips (5) + Arrange + search + Clear filters (now
        // active) = 12 presses; Select stays disabled (nothing visible),
        // so the 13th real stop is the content area's own Clear filters.
        for _ in 0..13 {
            h = tab(h, &mut state);
        }
        assert_eq!(h.focused_id(), Some(ids::empty::clear_filters()));
    }
}

#[cfg(test)]
mod findings {
    use crate::config::AppConfig;
    use crate::message::{DashboardMessage, Message, SelectionMessage, ShortcutMessage};
    use crate::state::AppState;
    use crate::view::{app_view, ids};
    use iced_test::core::Size;
    use knotra_ui::test_support::Harness;
    use knotra_vcs::{Project, Workspace};

    fn extend(state: &AppState) -> &'static AppState {
        unsafe { std::mem::transmute::<&AppState, &'static AppState>(state) }
    }

    fn drive(h: Harness<Message>, state: &mut AppState, msg: Message) -> Harness<Message> {
        use iced::futures::StreamExt;
        let task = crate::app::update(state, msg);
        let mut h = h.rebuilt(app_view(extend(state)));
        let Some(mut stream) = iced_test::runtime::task::into_stream(task) else {
            return h;
        };
        loop {
            match iced::futures::executor::block_on(stream.next()) {
                Some(iced_test::runtime::Action::Widget(op)) => h.run_operation(op),
                Some(_) => continue,
                None => break,
            }
        }
        h
    }

    /// **Found while capturing this stage's GUI evidence (C1), and
    /// fixed**: opening the Arrange panel from the keyboard used to lose
    /// real focus entirely, rather than leaving it on "Arrange" — the
    /// very next Tab wrapped all the way back to the page's first control
    /// instead of continuing into Group/Sort, defeating the behaviour
    /// RFC-075 exists to provide.
    ///
    /// Root cause, read directly rather than guessed: pressing Enter on
    /// "Arrange" (`snora::design::control`'s own built-in press, through
    /// the ordinary event path) published `DashboardMessage::
    /// ArrangeToggled` without moving focus itself — nothing there
    /// differs from any other control. What changes is that the
    /// toolbar's own `column![row![...], arrange_panel]` gains a **new
    /// child** once `state.dashboard_arrange_open` flips, and the
    /// rebuilt widget tree (`Harness::rebuilt`, the same cache-preserving
    /// rebuild the real runtime does, via `UserInterface::build` and
    /// `into_cache()`) reported **nothing** focused immediately
    /// afterward — confirmed directly: `Arrange` was real-focused before
    /// the toggle, and `focused_id()` came back `None` right after the
    /// rebuild that revealed the panel, with no Tab pressed in between.
    ///
    /// **Fix**: `handle_dashboard`'s own `ArrangeToggled` arm now returns
    /// `operation::focus(ids::toolbar::arrange())` in the same `Task` —
    /// re-asserting focus explicitly, the same shape every other
    /// focus-moving path in this app already uses, rather than relying
    /// on the rebuild to preserve it on its own.
    #[test]
    fn opening_the_arrange_panel_keeps_focus_on_arrange() {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;

        let mut h = Harness::new(
            app_view(extend(&state)),
            Size::new(1100.0, 900.0),
            Some(knotra_ui::widget::icon::FONT_BYTES),
        );
        // Shell (4) + chips (5) + Arrange = 10 presses (Standard/Wide
        // order: chips, Arrange, search, Select — see `toolbar.rs`'s
        // `view_standard_toolbar`).
        for _ in 0..10 {
            h = drive(h, &mut state, Message::Shortcut(ShortcutMessage::FocusNext));
        }
        assert_eq!(
            h.focused_id(),
            Some(ids::toolbar::arrange()),
            "sanity: Arrange must be real-focused before this finding's own check means anything"
        );

        h.enter();
        assert!(
            matches!(
                h.messages(),
                [Message::Dashboard(DashboardMessage::ArrangeToggled)]
            ),
            "Enter on Arrange must still press it once, same as any other control"
        );
        h = drive(
            h,
            &mut state,
            Message::Dashboard(DashboardMessage::ArrangeToggled),
        );
        assert!(state.dashboard_arrange_open, "the panel must now be open");

        assert_eq!(
            h.focused_id(),
            Some(ids::toolbar::arrange()),
            "Arrange must still be focused once its own panel is revealed, not lost to the \
             tree gaining a new sibling"
        );
    }

    /// The same finding, the same fix, for "Select" (RFC-075 stage 2 §5):
    /// entering selection mode gives every row a new checkbox and reveals
    /// the selection bar — the identical "this control's own tree gained
    /// new siblings" shape `opening_the_arrange_panel_keeps_focus_on_
    /// arrange` found and fixed for Arrange, confirmed separately here
    /// rather than assumed to be covered by the same fix.
    #[test]
    fn entering_selection_mode_keeps_focus_on_select() {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;

        let mut h = Harness::new(
            app_view(extend(&state)),
            Size::new(1100.0, 900.0),
            Some(knotra_ui::widget::icon::FONT_BYTES),
        );
        // Shell (4) + chips (5) + Arrange + search = 11 presses; the 12th
        // reaches Select.
        for _ in 0..12 {
            h = drive(h, &mut state, Message::Shortcut(ShortcutMessage::FocusNext));
        }
        assert_eq!(h.focused_id(), Some(ids::toolbar::select_mode()));

        h.enter();
        assert!(matches!(
            h.messages(),
            [Message::Selection(SelectionMessage::ModeEntered)]
        ));
        h = drive(
            h,
            &mut state,
            Message::Selection(SelectionMessage::ModeEntered),
        );
        assert!(state.selection_mode, "selection mode must now be on");
        assert_eq!(
            h.focused_id(),
            Some(ids::toolbar::select_mode()),
            "Select must still be focused once the selection bar and every row's \
             checkbox appear, not lost to the tree gaining new siblings"
        );
    }
}
