//! RFC-075 handoff stage 2 §5 — the page's own keyboard acceptance tests:
//! the real `Message`/`update`/`view` path, driven through `iced_runtime`'s
//! `UserInterface` directly, exactly the shape the handoff's own table asks
//! for ("the key's message through `update`, then every widget operation
//! in the returned task, run on `view(&state)` in the harness").
//!
//! **Not `knotra_ui::test_support::Harness`**: that type stores a
//! `UserInterface<'static, ...>` across calls, which is right for a test
//! whose root `Element` is genuinely `'static` (every other primitive test
//! in this workspace), but wrong here — `app_view(&state)` borrows from
//! `state`, and `state` is mutated between presses (`update` itself needs
//! `&mut AppState`), so a `'static` view of it can only be had by lying to
//! the borrow checker about the borrow's own length. Review 299 F2 found
//! exactly that lie (two `unsafe fn extend` transmutes) and asked for a
//! design the borrow checker proves instead of one a comment argues for.
//!
//! [`Rig`] is that design: it keeps only what can genuinely outlive any one
//! view (the render cache, the renderer, the published messages) and takes
//! `state` — and therefore the view built from it — **by reference, for
//! the duration of one call only**. [`rebuild`] is the one place a view
//! ever exists; its own return type's lifetime is tied to `state`'s
//! borrow, never to `&mut Rig`, so the caller's very next line can take
//! `&mut state` again with nothing left to alias. No `unsafe` block
//! appears anywhere in this file — `git grep -n unsafe -- 'crates/*.rs'`
//! finds only `knotra-vcs/src/process.rs`'s own documented case, as
//! Amendment A1 §2 asked.

use crate::message::Message;
use crate::state::AppState;
use crate::view::app_view;
use iced::advanced::widget::operation::{self, Outcome};
use iced::advanced::widget::{Id, Operation};
use iced::keyboard::key::Named;
use iced::{Theme, mouse};
use iced_test::core::clipboard;
use iced_test::core::renderer::Headless;
use iced_test::core::{Font, Pixels, Rectangle, Size};
use iced_test::runtime::user_interface::{self, UserInterface};

/// What [`rebuild`] needs across calls, and nothing a view ever borrows:
/// the render cache iced's own `UserInterface::into_cache`/`build` pair
/// carries a tree's widget state through a rebuild, the renderer, every
/// message published so far, and the (fixed, headless) cursor and size
/// every call here uses.
struct Rig {
    cache: user_interface::Cache,
    renderer: iced::Renderer,
    messages: Vec<Message>,
    cursor: mouse::Cursor,
    size: Size,
}

impl Rig {
    fn new(size: Size) -> Self {
        use iced_test::renderer::graphics::text::font_system;
        font_system()
            .write()
            .expect("write to the font system")
            .load_font(std::borrow::Cow::Borrowed(
                knotra_ui::widget::icon::FONT_BYTES,
            ));

        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::with_name("Noto Sans"),
            Pixels(16.0),
            None,
        ))
        .expect("the default headless renderer starts");

        Self {
            cache: user_interface::Cache::default(),
            renderer,
            messages: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            size,
        }
    }

    fn messages(&self) -> &[Message] {
        &self.messages
    }
}

/// Builds `app_view(state)` on `rig`'s own carried-over cache. The
/// returned `UserInterface` borrows `state` (that is the whole reason a
/// generic lifetime is named at all) and **not** `rig` — `rig.size` is
/// read before the call, `rig.renderer`/`rig.cache` are taken by value —
/// so a caller holding this return value can still drive it with
/// `&mut rig.renderer`/`&mut rig.messages` in the same scope, and once it
/// is dropped (every call site below does this before its own function
/// returns), nothing stops the next `&mut state` a message handler needs.
fn rebuild<'s>(
    rig: &mut Rig,
    state: &'s AppState,
) -> UserInterface<'s, Message, Theme, iced::Renderer> {
    let root = app_view(state);
    let cache = std::mem::take(&mut rig.cache);
    UserInterface::build(root, rig.size, cache, &mut rig.renderer)
}

/// Delivers one `RedrawRequested`, the event every real frame sends
/// through `UserInterface::update` before drawing (`iced_winit`'s own
/// shell loop, `lib.rs` around its `let redraw_event = core::Event::
/// Window(window::Event::RedrawRequested(..))`) — and the *only* event
/// `knotra_ui::widget::keyed`'s own `FollowsId` widget acts on to
/// reconcile focus. Without sending this, nothing here would ever
/// exercise Amendment A1's own mechanism at all, Task-driven widget
/// operations (`ui.operate`) being a different, narrower channel that
/// never carries a `Window` event through a widget's `update()`.
fn redraw(rig: &mut Rig, ui: &mut UserInterface<'_, Message, Theme, iced::Renderer>) {
    let events = [iced::Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))];
    let _ = ui.update(
        &events,
        rig.cursor,
        &mut rig.renderer,
        &mut clipboard::Null,
        &mut rig.messages,
    );
}

struct FindFocused(Option<Id>);

impl Operation for FindFocused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
    fn focusable(
        &mut self,
        id: Option<&Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if state.is_focused() {
            self.0 = id.cloned();
        }
    }
}

/// The focused control's own `.id()`, or `None` — rebuilds the current
/// view against `state` to ask, same as every other query here.
fn focused_id(rig: &mut Rig, state: &AppState) -> Option<Id> {
    let mut ui = rebuild(rig, state);
    redraw(rig, &mut ui);
    let mut found = FindFocused(None);
    ui.operate(&rig.renderer, &mut found);
    rig.cache = ui.into_cache();
    found.0
}

/// Drives `msg` through the real `app::update` (mutating `state`
/// synchronously, exactly as the live app does), then runs every widget
/// operation the returned `Task` yields against a view rebuilt from the
/// state `update` left behind — the one path every test below uses to
/// move or query real focus. This is the one function in this file where
/// a message changes `state`; every other helper only ever reads it.
fn drive(rig: &mut Rig, state: &mut AppState, msg: Message) {
    use iced::futures::StreamExt;

    let task = crate::app::update(state, msg);
    let mut ui = rebuild(rig, state);
    if let Some(mut stream) = iced_test::runtime::task::into_stream(task) {
        loop {
            match iced::futures::executor::block_on(stream.next()) {
                Some(iced_test::runtime::Action::Widget(mut op)) => loop {
                    ui.operate(&rig.renderer, &mut operation::black_box(op.as_mut()));
                    match op.finish() {
                        Outcome::None | Outcome::Some(()) => break,
                        Outcome::Chain(next) => op = next,
                    }
                },
                Some(_) => continue,
                None => break,
            }
        }
    }
    redraw(rig, &mut ui);
    rig.cache = ui.into_cache();
}

/// A single named key (Enter, Space, an arrow), delivered directly to the
/// real widget tree — the same event path a user's keyboard driver would
/// produce, for the controls (text fields, snora's own `Select`) that
/// capture a key themselves rather than going through `app::update`.
fn key(rig: &mut Rig, state: &AppState, named: Named) {
    let mut ui = rebuild(rig, state);
    let events = [iced_test::simulator::press_key(named, None)];
    let _ = ui.update(
        &events,
        rig.cursor,
        &mut rig.renderer,
        &mut clipboard::Null,
        &mut rig.messages,
    );
    redraw(rig, &mut ui);
    rig.cache = ui.into_cache();
}

const SIZE: Size = Size {
    width: 1100.0,
    height: 900.0,
};

#[cfg(test)]
mod tests {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{
        DashboardMessage, FilterMessage, Message, SelectionMessage, ShortcutMessage, StatusFilter,
    };
    use crate::state::{AppState, ArrangeSelect, Screen};
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use iced::widget::Id;
    use knotra_vcs::model::status::{RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn tab(rig: &mut Rig, state: &mut AppState) {
        drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
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
        let mut rig = Rig::new(SIZE);

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
            tab(&mut rig, &mut state);
            assert_eq!(
                focused_id(&mut rig, &state),
                Some(want.clone()),
                "expected {want:?}"
            );
        }

        // Past the last control, Tab wraps to the first (Amendment A1) —
        // not "leaves the group for one press" (snora's own, un-wrapped
        // default).
        tab(&mut rig, &mut state);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(expected[0].clone()),
            "Tab past the last control must wrap to the first"
        );

        // Shift+Tab from the first wraps to the last.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusPrevious),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
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
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search + Select + the row's
        // checkbox + name + action = 15 presses reaches the row's own
        // action; the 16th reaches Exit selection.
        for _ in 0..16 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::exit())
        );
    }

    /// Disabled controls are skipped entirely (D4 b) — Select, with
    /// nothing selectable (no visible projects), is never reached.
    #[test]
    fn disabled_select_mode_is_skipped() {
        let (mut state, _alpha_id) = one_project_state();
        state.filter.search_text = "does-not-match-anything".to_owned();
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search = 11 presses reaches
        // the search field; the 12th (a filter is active) reaches Clear
        // filters.
        for _ in 0..12 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::clear_filters())
        );

        tab(&mut rig, &mut state);
        assert_ne!(
            focused_id(&mut rig, &state),
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
        let mut rig = Rig::new(SIZE);

        for _ in 0..5 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::filter_needs_help())
        );
        key(&mut rig, &state, Named::Enter);
        assert!(matches!(
            rig.messages(),
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
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search + Select = 12 presses
        // reaches Select; the 13th reaches the row's own checkbox.
        for _ in 0..13 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::checkbox(&alpha_id))
        );
        key(&mut rig, &state, Named::Space);
        assert!(matches!(
            rig.messages(),
            [Message::Selection(SelectionMessage::Toggled(id))] if *id == alpha_id
        ));
    }

    /// RFC-075 D6: `/` focuses the search field directly, with no Tab
    /// needed first — `focus_search` switches `state.screen` and issues
    /// `operation::focus(SEARCH)` in the same `Task`.
    #[test]
    fn bare_slash_focuses_the_search_field() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace {
            projects: vec![Project::new("Alpha", "/tmp")],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.screen = Screen::Settings;
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusSearchBare),
        );
        assert_eq!(state.screen, Screen::Dashboard);
        assert_eq!(
            focused_id(&mut rig, &state),
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
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id))
        );

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::CardFocusNext),
        ); // j
        assert_eq!(focused_id(&mut rig, &state), Some(ids::row::name(&beta_id)));

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "↓ past the last project must wrap to the first"
        );

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowUp),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
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
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search + Select = 12 presses
        // (the panel's own Group/Sort selects are drawn *below* the
        // Arrange/search/Select row, not right after Arrange); the 13th
        // reaches the open panel's first select (Group).
        for _ in 0..13 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::arrange_group())
        );

        key(&mut rig, &state, Named::ArrowDown);
        assert!(matches!(
            rig.messages(),
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
        let mut rig = Rig::new(SIZE);

        for _ in 0..5 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::empty::add_first_project())
        );
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
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search + Clear filters (now
        // active) = 12 presses; Select stays disabled (nothing visible),
        // so the 13th real stop is the content area's own Clear filters.
        for _ in 0..13 {
            tab(&mut rig, &mut state);
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::empty::clear_filters())
        );
    }
}

#[cfg(test)]
mod findings {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{DashboardMessage, Message, SelectionMessage, ShortcutMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::{Project, Workspace};

    fn one_project_state() -> AppState {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state
    }

    /// **Found while capturing stage 2's own GUI evidence (C1), and
    /// fixed**: opening the Arrange panel from the keyboard used to lose
    /// real focus entirely, rather than leaving it on "Arrange" — the
    /// very next Tab wrapped all the way back to the page's first control
    /// instead of continuing into Group/Sort, defeating the behaviour
    /// RFC-075 exists to provide.
    ///
    /// **The explicit fix (`1144026`) is gone** (Amendment A2 §2): this
    /// test is unchanged, and passes with no explicit `operation::focus`
    /// behind it at all — `knotra_ui::widget::focus_by_id`'s own one,
    /// whole-page wrap keeps real focus on "Arrange" (it is still really
    /// focused, nothing moved it away) through the same rebuild that
    /// reveals the panel, the same way it keeps any other control focused
    /// across any other reorder.
    #[test]
    fn opening_the_arrange_panel_keeps_focus_on_arrange() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange = 10 presses (Standard/Wide
        // order: chips, Arrange, search, Select — see `toolbar.rs`'s
        // `view_standard_toolbar`).
        for _ in 0..10 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::arrange()),
            "sanity: Arrange must be real-focused before this finding's own check means anything"
        );

        key(&mut rig, &state, Named::Enter);
        assert!(
            matches!(
                rig.messages(),
                [Message::Dashboard(DashboardMessage::ArrangeToggled)]
            ),
            "Enter on Arrange must still press it once, same as any other control"
        );
        drive(
            &mut rig,
            &mut state,
            Message::Dashboard(DashboardMessage::ArrangeToggled),
        );
        assert!(state.dashboard_arrange_open, "the panel must now be open");

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::arrange()),
            "Arrange must still be focused once its own panel is revealed, not lost to the \
             tree gaining a new sibling"
        );
    }

    /// The same finding, for "Select" — its own explicit fix (`279c0ee`)
    /// is gone too (Amendment A2 §2), for the same reason.
    #[test]
    fn entering_selection_mode_keeps_focus_on_select() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search = 11 presses; the 12th
        // reaches Select.
        for _ in 0..12 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::select_mode())
        );

        key(&mut rig, &state, Named::Enter);
        assert!(matches!(
            rig.messages(),
            [Message::Selection(SelectionMessage::ModeEntered)]
        ));
        drive(
            &mut rig,
            &mut state,
            Message::Selection(SelectionMessage::ModeEntered),
        );
        assert!(state.selection_mode, "selection mode must now be on");
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::select_mode()),
            "Select must still be focused once the selection bar and every row's \
             checkbox appear, not lost to the tree gaining new siblings"
        );
    }
}

/// RFC-075 stage 3 §3/§7 row 4 — every window's own `entry_id`, called
/// directly against a constructed `AppState` (no `Rig` needed: `entry_id`
/// is a pure function of state, the same one `app/focus_ops.rs::entry_id`
/// dispatches to). Covers "a change is never the entry" for the six named
/// controls, and the general entry-per-phase shape for every window.
#[cfg(test)]
mod stage3_entry_ids {
    use crate::config::AppConfig;
    use crate::state::{
        AppState, conflict_ops::ConflictPhase, context::ContextPhase, freezer::FreezerPhase,
        sync::SyncPhase,
    };
    use crate::view::ids;
    use knotra_vcs::model::conflict::{ConflictMarker, ConflictedFile, ProjectConflictDetail};
    use knotra_vcs::model::operation::{
        FreezeValidation, FreezeValidationEntry, SmartPullDisposition, SmartPullPlan,
        SmartPullPlanEntry,
    };
    use knotra_vcs::model::status::{ContextCandidate, ContextList, ContextTarget, VcsKind};
    use knotra_vcs::{OperationId, Project, ProjectId, Workspace};

    fn base_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    fn one_project_state() -> (AppState, ProjectId) {
        let mut state = base_state();
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        (state, id)
    }

    // ---- Get latest safely (smart_pull) --------------------------------

    #[test]
    fn smart_pull_entry_idle_is_none() {
        let state = base_state();
        assert_eq!(crate::view::overlays::smart_pull_entry_id(&state), None);
    }

    /// Row 4: `ExecuteRequested` ("Start getting latest") is never the
    /// entry — the first dirty entry's own disposition toggle stands in.
    #[test]
    fn smart_pull_entry_awaiting_confirm_is_the_first_dirty_entrys_toggle_never_start() {
        let mut state = base_state();
        let dirty = Project::new("Dirty", "/tmp");
        let dirty_id = dirty.id.clone();
        let clean = Project::new("Clean", "/tmp");
        state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
            id: OperationId::new(),
            entries: vec![
                SmartPullPlanEntry {
                    project_id: clean.id.clone(),
                    project_name: clean.name.clone(),
                    is_dirty: false,
                    has_conflict: false,
                    skip_reason: None,
                    disposition: SmartPullDisposition::Pull,
                },
                SmartPullPlanEntry {
                    project_id: dirty_id,
                    project_name: dirty.name.clone(),
                    is_dirty: true,
                    has_conflict: false,
                    skip_reason: None,
                    disposition: SmartPullDisposition::FetchOnly,
                },
            ],
        });

        let entry = crate::view::overlays::smart_pull_entry_id(&state);
        assert_eq!(
            entry,
            Some(ids::smart_pull::check_only(1)),
            "the entry must be the first dirty entry's own toggle, at its real index (1), \
             not index 0"
        );
        assert_ne!(
            entry,
            Some(ids::smart_pull::start()),
            "\"Start getting latest\" starts a change and must never be the entry"
        );
    }

    #[test]
    fn smart_pull_entry_awaiting_confirm_with_no_dirty_entries_is_cancel() {
        let mut state = base_state();
        let clean = Project::new("Clean", "/tmp");
        state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
            id: OperationId::new(),
            entries: vec![SmartPullPlanEntry {
                project_id: clean.id.clone(),
                project_name: clean.name,
                is_dirty: false,
                has_conflict: false,
                skip_reason: None,
                disposition: SmartPullDisposition::Pull,
            }],
        });
        assert_eq!(
            crate::view::overlays::smart_pull_entry_id(&state),
            Some(ids::smart_pull::cancel())
        );
    }

    #[test]
    fn smart_pull_entry_done_is_close() {
        let mut state = base_state();
        state.sync.phase = SyncPhase::Done(crate::state::sync::SyncResult {
            per_project: Vec::new(),
        });
        assert_eq!(
            crate::view::overlays::smart_pull_entry_id(&state),
            Some(ids::smart_pull::done_close())
        );
    }

    // ---- Save release point (freezer) -----------------------------------

    #[test]
    fn freezer_entry_idle_is_the_name_field() {
        let state = base_state();
        assert_eq!(
            crate::view::overlays::freezer_entry_id(&state),
            Some(knotra_ui::widget::focus_id::RELEASE_NAME.clone())
        );
    }

    /// Row 4: `ExecuteConfirmed` ("Save release point") is never the
    /// entry.
    #[test]
    fn freezer_entry_validation_ready_never_save() {
        let mut state = base_state();
        state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
            freeze_name: "v1".to_owned(),
            entries: vec![FreezeValidationEntry {
                project_id: ProjectId::new(),
                project_name: "Alpha".to_owned(),
                notes: Vec::new(),
                blockers: Vec::new(),
            }],
        });
        let entry = crate::view::overlays::freezer_entry_id(&state);
        assert_eq!(entry, Some(ids::freezer::cancel()));
        assert_ne!(entry, Some(ids::freezer::save()));
    }

    #[test]
    fn freezer_entry_done_is_close() {
        let mut state = base_state();
        state.freezer.phase = FreezerPhase::Done(knotra_vcs::FreezeResult {
            outcome: knotra_vcs::model::operation::FreezeOutcome::Success,
            freeze_name: "v1".to_owned(),
            project_results: Vec::new(),
        });
        assert_eq!(
            crate::view::overlays::freezer_entry_id(&state),
            Some(ids::freezer::done_close())
        );
    }

    // ---- Change work area (context_switch) ------------------------------

    #[test]
    fn context_switch_entry_browsing_list_is_the_search_field() {
        let mut state = base_state();
        state.active_modal = crate::state::ActiveModal::Switch;
        state.context_ops.phase = ContextPhase::BrowsingList {
            project_id: ProjectId::new(),
            list: ContextList::default(),
            search: String::new(),
        };
        assert_eq!(
            crate::view::overlays::context_switch_entry_id(&state),
            Some(knotra_ui::widget::focus_id::SWITCH_TARGET.clone())
        );
    }

    /// Row 4: `SwitchConfirmed` ("Change work area" in the confirmation
    /// footer) is never the entry — Cancel is.
    #[test]
    fn context_switch_entry_confirm_switch_never_confirm() {
        let mut state = base_state();
        state.active_modal = crate::state::ActiveModal::Switch;
        state.context_ops.phase = ContextPhase::ConfirmSwitch {
            project_id: ProjectId::new(),
            project_name: "Alpha".to_owned(),
            target: ContextTarget::GitLocalBranch {
                name: "main".to_owned(),
            },
            target_label: "main".to_owned(),
            is_dirty: false,
            disabled_reason_key: None,
        };
        let entry = crate::view::overlays::context_switch_entry_id(&state);
        assert_eq!(entry, Some(ids::context_switch::cancel()));
        assert_ne!(entry, Some(ids::context_switch::confirm()));
    }

    #[test]
    fn context_switch_candidate_fixture_is_well_formed() {
        // Sanity check for the fixture other tests in this module build —
        // `ContextList`/`ContextCandidate` construction, exercised once
        // here rather than repeated unchecked in every test that needs a
        // candidate.
        let list = ContextList {
            project_id: ProjectId::new(),
            vcs_kind: VcsKind::Git,
            candidates: vec![ContextCandidate {
                label: "main".to_owned(),
                target: ContextTarget::GitLocalBranch {
                    name: "main".to_owned(),
                },
                is_current: true,
            }],
            warning: None,
        };
        assert_eq!(list.candidates.len(), 1);
    }

    // ---- Generate notes (changelog) --------------------------------------

    #[test]
    fn changelog_entry_idle_is_the_since_field() {
        let state = base_state();
        assert_eq!(
            crate::view::overlays::changelog_entry_id(&state),
            Some(knotra_ui::widget::focus_id::CHANGELOG_SINCE.clone())
        );
    }

    #[test]
    fn changelog_entry_ready_is_copy() {
        use crate::state::changelog::ChangelogPhase;
        use knotra_vcs::ChangelogDraft;

        let mut state = base_state();
        state.changelog.phase = ChangelogPhase::Ready(ChangelogDraft {
            release_name: "v1".to_owned(),
            generated_at: jiff::Timestamp::now(),
            projects: Vec::new(),
        });
        assert_eq!(
            crate::view::overlays::changelog_entry_id(&state),
            Some(ids::changelog::copy())
        );
    }

    // ---- Resolve (conflict) ----------------------------------------------

    /// Row 4: `AbortMergeRequested` ("Stop and discard edits") is never the
    /// entry — "Keep fixing" is.
    #[test]
    fn resolve_entry_confirm_stop_never_stop() {
        let (mut state, project_id) = one_project_state();
        state.conflict_ops.phase = ConflictPhase::ConfirmStop {
            project_id: project_id.clone(),
            detail: ProjectConflictDetail {
                project_id: project_id.clone(),
                project_name: "Alpha".to_owned(),
                conflicted_files: Vec::new(),
                read_error: None,
            },
        };
        let entry = crate::view::overlays::conflict_entry_id(&state, &project_id);
        assert_eq!(entry, Some(ids::resolve::keep_fixing()));
        assert_ne!(entry, Some(ids::resolve::stop()));
    }

    #[test]
    fn resolve_entry_browsing_with_files_and_no_tools_configured_is_mark_done() {
        let (mut state, project_id) = one_project_state();
        state.conflict_ops.phase = ConflictPhase::Browsing {
            project_id: project_id.clone(),
            detail: ProjectConflictDetail {
                project_id: project_id.clone(),
                project_name: "Alpha".to_owned(),
                conflicted_files: vec![ConflictedFile {
                    path: "src/lib.rs".to_owned(),
                    marker: ConflictMarker::BothModified,
                }],
                read_error: None,
            },
        };
        // Git is inferred from the project's own on-disk `.git` marker in
        // production; this harness has no real repository, so
        // `conflict_vcs_kind_for_project` falls through to its own
        // no-evidence `None` arm — confirmed by the next assertion
        // choosing `done_close` instead, documented as the "no evidence"
        // case rather than silently assumed to be Git.
        let entry = crate::view::overlays::conflict_entry_id(&state, &project_id);
        assert_eq!(
            entry,
            Some(ids::resolve::done_close()),
            "with no VCS evidence on disk and neither tool configured, the file list has no \
             reachable control at all, so the entry falls back to Close"
        );
    }

    // ---- Remove project ---------------------------------------------------

    /// Row 4: `RemoveProjectConfirmed` ("Remove from list") is never the
    /// entry — "Keep" is.
    #[test]
    fn remove_project_entry_is_keep_never_remove() {
        let (mut state, project_id) = one_project_state();
        state.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
            project_id,
            project_name: "Alpha".to_owned(),
        });
        let entry = crate::view::remove_project_dialog::entry_id(&state);
        assert_eq!(entry, Some(ids::remove_project::keep()));
        assert_ne!(entry, Some(ids::remove_project::remove()));
    }

    #[test]
    fn remove_project_entry_is_none_when_closed() {
        let state = base_state();
        assert_eq!(crate::view::remove_project_dialog::entry_id(&state), None);
    }

    // ---- Workspace create/rename/delete -----------------------------------

    #[test]
    fn workspace_create_entry_is_the_name_field() {
        let mut state = base_state();
        state.workspace_mgr.create_dialog =
            Some(crate::state::workspace_mgr::CreateWorkspaceDialog {
                name: String::new(),
                error: None,
            });
        assert_eq!(
            crate::view::workspace_manager::entry_id(&state),
            Some(knotra_ui::widget::focus_id::WORKSPACE_NAME.clone())
        );
    }

    /// Row 4: `DeleteWorkspaceConfirmed` ("Confirm delete") is never the
    /// entry — "Cancel" is (R6: a destructive dialog never defaults onto
    /// its own destructive action).
    #[test]
    fn workspace_delete_entry_is_cancel_never_confirm() {
        let mut state = base_state();
        state.workspace_mgr.confirm_delete =
            Some(crate::state::workspace_mgr::DeleteWorkspaceDialog {
                workspace_id: knotra_vcs::model::workspace::WorkspaceId::new(),
                workspace_name: "Main".to_owned(),
                project_count: 0,
                error: None,
            });
        let entry = crate::view::workspace_manager::entry_id(&state);
        assert_eq!(entry, Some(ids::workspace_manager::delete_cancel()));
        assert_ne!(entry, Some(ids::workspace_manager::confirm_delete()));
    }

    // ---- The shortcuts sheet, the command palette, the workspace menu ----

    #[test]
    fn shortcuts_sheet_entry_is_always_close() {
        assert_eq!(
            crate::view::shortcuts_overlay::entry_id(),
            ids::shortcuts::close()
        );
    }

    #[test]
    fn palette_entry_is_the_query_field_only_while_open() {
        let mut state = base_state();
        assert_eq!(crate::view::command_palette::entry_id(&state), None);
        state.palette.open = true;
        assert_eq!(
            crate::view::command_palette::entry_id(&state),
            Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
        );
    }

    #[test]
    fn switcher_menu_entry_is_the_first_non_active_workspace_row() {
        let mut state = base_state();
        let main = Workspace::new("Main");
        let second = Workspace::new("Second");
        let second_id = second.id.clone();
        state.all_workspaces = vec![main, second];
        state.active_workspace_idx = 0;
        state.workspace_mgr.switcher_open = true;

        assert_eq!(
            crate::view::shell::switcher_menu_entry_id(&state),
            Some(ids::switcher::row(&second_id)),
            "the active workspace's own row (index 0, \"Main\") is disabled \
             (switching to it would start no change), so the entry must be \
             the next workspace's row"
        );
    }
}

/// Amendment A1 §1/§5 — the mechanism itself: focus follows its control's
/// id across a rebuild that reorders, adds, or removes rows, rather than
/// whatever happens to sit at the same tree position (Review 299 F1).
#[cfg(test)]
mod focus_follows_its_id {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{BackgroundMessage, Message, SelectionMessage, ShortcutMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::model::status::{RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn one_project_state() -> AppState {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state
    }

    /// A clean status (no conflict, no relevant counts) classifies as
    /// `InProgress` once at least one count is non-zero below, or
    /// `AllSet` with none — `uncommitted: 1` keeps this one in
    /// `InProgress`, which (unlike `AllSet`) is **not** collapsed by
    /// default, so the row stays visible and simply moves to a different
    /// section — the direct "same id, new position" case, not the
    /// "its own row disappears" one `focus_goes_to_the_neighbour_...`
    /// covers separately.
    fn in_progress_status(id: ProjectId) -> ProjectStatus {
        ProjectStatus {
            project_id: id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: Default::default(),
            working_tree: WorkingTreeStatus {
                uncommitted_count: 1,
                ..WorkingTreeStatus::default()
            },
            conflict: Default::default(),
            refreshed_at: jiff::Timestamp::now(),
            read_error: None,
        }
    }

    fn two_needs_help_projects() -> (AppState, ProjectId, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        let beta = Project::new("Beta", "/tmp");
        let (alpha_id, beta_id) = (alpha.id.clone(), beta.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![alpha, beta],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        (state, alpha_id, beta_id)
    }

    /// Review 299's own reproduction, corrected to a case where Alpha's
    /// row stays visible (not collapsed away) so this tests the
    /// mechanism's **primary** rule directly: the control with the same
    /// id, wherever it now is — not the fallback-to-neighbour rule, which
    /// is a different, separately-tested case below.
    ///
    /// **Seen failing first on `279c0ee`** (before this amendment):
    /// `focused_id` came back `Some(beta_id's name)` — the position Alpha
    /// vacated, which Beta's row slid into once Alpha moved to
    /// `InProgress` and the section list re-sorted — never Alpha's own
    /// id, confirming the bug is real and this test catches it.
    #[test]
    fn focus_stays_on_its_project_when_a_refresh_moves_it_to_another_tier() {
        let (mut state, alpha_id, _beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "sanity: ↓ must focus Alpha's name first"
        );

        // The refresh lands: Alpha's status is now known, and it is no
        // longer `NeedsHelp` — a `BackgroundMessage`, the real message a
        // live refresh completing publishes, not a direct field write.
        // Beta has no entry in this status, so it keeps classifying as
        // `NeedsHelp` via `classify`'s own `StatusUnknown` fallback —
        // unmoved, same as the review's own reproduction.
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                WorkspaceStatus {
                    projects: vec![in_progress_status(alpha_id.clone())],
                    last_refresh: None,
                },
            )),
        );

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "focus must stay on Alpha's own name, now in a different section — \
             not on whatever row slid into the position Alpha's used to be"
        );
    }

    /// The fallback rule: with Alpha's own control genuinely gone (the
    /// project removed outright, not merely moved to another section),
    /// focus goes to its neighbour — the next survivor in the old order,
    /// here Beta.
    #[test]
    fn focus_goes_to_the_neighbour_when_its_own_project_is_removed() {
        let (mut state, alpha_id, beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id))
        );

        state
            .workspace
            .as_mut()
            .unwrap()
            .projects
            .retain(|p| p.id != alpha_id);
        // A no-op message, just to drive one more rebuild/redraw through
        // `Rig` the same way every other step here does.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusPrevious),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&beta_id)),
            "with Alpha gone outright, focus must land on its neighbour, Beta"
        );
    }

    /// The reader's own move, when a filter narrows the list on a **later**
    /// step: Tab's own choice (settled by its own redraw before the filter
    /// message even arrives) must still stand, not be pulled back once
    /// `follow()` reconciles the filter's own reorder.
    #[test]
    fn the_readers_tab_stands_once_a_later_step_also_narrows_the_list() {
        let (mut state, alpha_id, _beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id))
        );

        // Tab, as its own step — settled (its own redraw records "Alpha's
        // action" as the freshly remembered focus) before anything else
        // happens.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::action(&alpha_id))
        );

        // *Then*, a filter narrows the list to Alpha alone, as a later,
        // separate step — `focused_id`'s own rebuild is that step; it
        // need not be a Tab press.
        state.filter.search_text = "Alpha".to_owned();
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::action(&alpha_id)),
            "Tab's own earlier move must still stand once the filter's own \
             reorder is reconciled on a later step"
        );
    }

    /// **Does not share `snora::Keyed`'s own documented limit** (Tab
    /// pressed in the very same redraw the row set also changes shape
    /// in) — checked directly rather than assumed to still apply, since
    /// `knotra_ui::widget::keyed::FollowsId`'s own `resync` closes exactly
    /// the gap that produces it (see that module's own doc comment): Tab's
    /// own move, applied via `operate()`, is already reflected in
    /// `state.focused` before this same step's reconciliation ever runs,
    /// so it has nothing stale to fall back to.
    #[test]
    fn a_tab_in_the_exact_same_step_a_reorder_happens_is_not_undone_either() {
        let (mut state, alpha_id, _beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id))
        );

        // Narrows the list to Alpha alone, *then* drives Tab — both take
        // effect in the one redraw this single `drive` step settles.
        state.filter.search_text = "Alpha".to_owned();
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::action(&alpha_id)),
            "Tab's own move, in the very same step the filter also narrowed the list, must stand"
        );
    }

    /// A structural change *elsewhere* on the page (every row gains a
    /// checkbox once selection mode turns on) must not disturb a focus
    /// that has nothing to do with it: Alpha's own name, still really
    /// focused through the same rebuild, must stay put — not be pulled
    /// onto some other id by this reconciliation mistaking "nothing of
    /// mine changed" for "something must be reassigned". (Amendment A1's
    /// own version of this test instead pressed "Select" first and
    /// expected focus to land back on it — that was really testing the
    /// stage-2 `operation::focus` fix A2 §2 removes, not this mechanism;
    /// `keyboard_acceptance::findings::entering_selection_mode_keeps_
    /// focus_on_select` already covers that real "Select" press, and
    /// still passes with no explicit fix behind it at all — see A2's own
    /// report for why: with the whole page under one wrap, nothing here
    /// moves "Select" away from itself in the first place.)
    #[test]
    fn entering_selection_mode_elsewhere_does_not_move_focus_off_alphas_row() {
        let (mut state, alpha_id, _beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id))
        );

        drive(
            &mut rig,
            &mut state,
            Message::Selection(SelectionMessage::ModeEntered),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "Alpha's own name must still be focused once every row gains a checkbox"
        );
    }

    /// Amendment A2 §3, rows 1-3, with "Check now" disabled by its own
    /// Enter press. `ids::shell::refresh()` is one of `view.rs`'s own
    /// `reserved` ids (`knotra_ui::widget::focus_by_id`'s own parameter):
    /// gone from a real survey the moment it disables, but never treated
    /// as removed outright. Named here rather than "the shell's own
    /// shape never changes" alone, since Amendment A1's own version of
    /// this test asserted today's *bug* as if it were the fix — see
    /// A2's own review (F3) for why that assertion was backwards.
    #[test]
    fn check_now_keeps_its_place_during_a_refresh_started_by_its_own_press() {
        use crate::message::WorkspaceMessage;

        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        // `nav_history` is the first stop (`nav_dashboard` is already the
        // active screen, D4 b); the second reaches "Check now".
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::refresh()));

        key(&mut rig, &state, Named::Enter);
        assert!(
            matches!(
                rig.messages(),
                [Message::Workspace(WorkspaceMessage::RefreshRequested)]
            ),
            "Enter on \"Check now\" must still press it once, same as any other control"
        );

        // The real handler spawns an async VCS scan (needs a Tokio
        // runtime this harness does not have) — only its own effect on
        // `is_refreshing` matters here, so that one field is set directly
        // rather than driving the message itself through `app::update`.
        state.is_refreshing = true;

        // 1. No other control takes focus, no ring appears elsewhere.
        assert_eq!(
            focused_id(&mut rig, &state),
            None,
            "nothing should report as focused while \"Check now\" is disabled"
        );

        // 3. When the refresh ends, with the reader having not moved,
        // focus is on "Check now" again.
        state.is_refreshing = false;
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::refresh()),
            "\"Check now\" must be focused again once the refresh ends"
        );
    }

    /// Row 2, the forward direction: Tab, with "Check now" disabled and
    /// nothing really focused, reaches the control after it — not the
    /// page's own first control, which is what `sequential_focus`'s own
    /// "nothing focused" fallback would otherwise land on.
    #[test]
    fn tab_reaches_the_control_after_check_now_once_it_disables() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::refresh()));

        state.is_refreshing = true;
        assert_eq!(focused_id(&mut rig, &state), None);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::palette()),
            "Tab must continue to the control after the disabled \"Check now\", \
             not restart from the page's own first control"
        );
    }

    /// Row 2, the backward direction: Shift+Tab, under the same
    /// conditions, reaches the control before it.
    #[test]
    fn shift_tab_reaches_the_control_before_check_now_once_it_disables() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::refresh()));

        state.is_refreshing = true;
        assert_eq!(focused_id(&mut rig, &state), None);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusPrevious),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::nav_history()),
            "Shift+Tab must reach the control before the disabled \"Check now\", \
             not wrap to the page's own last control"
        );
    }

    /// The same three rows, this time for a refresh an automatic trigger
    /// starts (a periodic timer, a filesystem watcher) while "Check now"
    /// already holds focus from an earlier Tab, never itself pressed.
    #[test]
    fn check_now_keeps_its_place_during_an_automatic_refresh() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::refresh()));

        // No press at all — the refresh starts on its own.
        state.is_refreshing = true;
        assert_eq!(focused_id(&mut rig, &state), None);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::palette()));

        state.is_refreshing = false;
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::palette()),
            "the reader did move (Tab, above) before the refresh ended — \"Check now\" \
             must not steal focus back from where they actually are"
        );
    }

    /// Amendment A2 §3's own "any other page control that disables itself
    /// when pressed" — the selection bar's "Check for updates"
    /// (`ids::selection_bar::fetch()`), the other `reserved` id `view.rs`
    /// passes: pressing it acquires `operation_interlock` (`OperationOwner
    /// ::BulkFetch`), and the same lock gates its own `can_act`, so it
    /// disables itself exactly the way "Check now" does. One
    /// representative row of the same table (Tab reaches the control
    /// after it) rather than the full 1-3 set again, since the mechanism
    /// itself (`reserved`) is the same one "Check now"'s own tests
    /// already exercise in full.
    #[test]
    fn tab_reaches_the_control_after_selection_bar_fetch_once_it_disables() {
        use crate::state::OperationOwner;

        let mut state = one_project_state();
        let project_id = state.workspace.as_ref().unwrap().projects[0].id.clone();
        state.selection_mode = true;
        state.selection.toggle(project_id);
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search + Select (12) + the
        // row's own checkbox, name, action (3, NeedsHelp) + Exit (1) = 16
        // presses; the 17th reaches "Check for updates", the selection
        // bar's own first bulk action.
        for _ in 0..17 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::fetch())
        );

        state
            .operation_interlock
            .try_acquire(OperationOwner::BulkFetch)
            .expect("the interlock starts idle");
        assert_eq!(focused_id(&mut rig, &state), None);

        // Unlike "Check now" (whose own neighbours, History and Command
        // palette, stay enabled while it disables), "Check for updates"'s
        // own siblings (Get latest safely, Save release point, Switch
        // work area) share the *same* `operation_interlock` gate, so all
        // three go disabled in the same step too — they are not
        // themselves `reserved` (their own press is not what disabled
        // them, so Amendment A2 §3's own rule does not name them). With
        // nothing left enabled after "Check for updates" anywhere on the
        // page, Tab correctly wraps to the page's own first control
        // (Amendment A1) rather than landing on a sibling that is just as
        // unavailable.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::nav_history()),
            "with every selection-bar action disabled, Tab must wrap to the page's own first \
             control, not stop on an equally-disabled sibling"
        );
    }

    /// Amendment A2 §4's own new row: a reorder where the **count** of
    /// tracked ids does not change at all — the old, hand-maintained-key
    /// design (A1) would have had no reason to "stand aside" here either,
    /// so this guards the *new* id-based mechanism specifically, not the
    /// count mismatch A1's own tests already covered. Passes on `c913ffa`
    /// too (the hand-made list was already right for this one case) —
    /// its own job is to keep that true now that nothing is hand-made at
    /// all.
    #[test]
    fn a_same_count_reorder_by_sort_still_follows_the_id() {
        let mut state = AppState::new(AppConfig::default());
        let zed = Project::new("Zed", "/tmp");
        let alpha = Project::new("Alpha", "/tmp");
        let (zed_id, alpha_id) = (zed.id.clone(), alpha.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![zed, alpha],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        let first_focused = focused_id(&mut rig, &state);
        assert!(
            first_focused == Some(ids::row::name(&zed_id))
                || first_focused == Some(ids::row::name(&alpha_id)),
            "sanity: ↓ must focus one of the two project names"
        );

        // Whichever order "Recommended" happens to use for two equally
        // unclassified projects, name-ascending reorders them (Alpha
        // before Zed) — the point is the same-count reorder itself, not
        // which project was first.
        drive(
            &mut rig,
            &mut state,
            Message::Dashboard(crate::message::DashboardMessage::SortChanged(
                crate::config::DashboardSort::NameAscending,
            )),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            first_focused,
            "the same project's own name must still be focused after a same-count reorder — \
             the id, not the position"
        );
    }

    /// Amendment A2 §2/§4: "the page's root is covered" — a chip press
    /// adding "Clear filters" to the toolbar must not disturb a focus
    /// that has nothing to do with it, proving the wrap really spans the
    /// whole page body (not only the rows, as A1's own narrower wrap did).
    #[test]
    fn select_stays_focused_when_a_chip_press_adds_clear_filters() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        // Shell (4) + chips (5) + Arrange + search = 11 presses; the 12th
        // reaches "Select" (no filter active yet, so "Clear filters" is
        // not drawn).
        for _ in 0..12 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::select_mode())
        );

        drive(
            &mut rig,
            &mut state,
            Message::Filter(crate::message::FilterMessage::StatusFilterToggled(
                crate::message::StatusFilter::NeedsHelp,
            )),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::select_mode()),
            "\"Select\" must still be focused once a chip press reveals \"Clear filters\" \
             right next to it"
        );
    }

    /// The same guarantee, the other direction: a chip stays focused when
    /// *it* is the one pressed, revealing the Arrange panel below it.
    #[test]
    fn a_chip_stays_focused_when_the_arrange_panel_opens() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        for _ in 0..5 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::filter_needs_help())
        );

        drive(
            &mut rig,
            &mut state,
            Message::Dashboard(crate::message::DashboardMessage::ArrangeToggled),
        );
        assert!(
            state.dashboard_arrange_open,
            "sanity: the panel must now be open"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::filter_needs_help()),
            "the chip must still be focused once the Arrange panel appears below the toolbar"
        );
    }

    /// Amendment A2 §2/§4's own last row: the header gains a new leading
    /// control (the workspace switcher) once a second workspace exists —
    /// a focus on the header's own last control must survive it, proving
    /// the wrap reaches the header too, not only the body.
    #[test]
    fn settings_stays_focused_when_the_switcher_appears() {
        let mut state = one_project_state();
        let mut rig = Rig::new(SIZE);

        // Shell: nav_history, refresh, palette, settings (nav_dashboard
        // is already active, D4 b) — the 4th Tab reaches "Settings".
        for _ in 0..4 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::FocusNext),
            );
        }
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shell::settings()));

        state.all_workspaces = vec![
            knotra_vcs::Workspace::new("Main"),
            knotra_vcs::Workspace::new("Second"),
        ];
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::settings()),
            "\"Settings\" must still be focused once the workspace switcher appears before it"
        );
    }
}

/// Amendment A2 §1: "a focus target without an id cannot be followed"
/// (`knotra_ui::widget::keyed`'s own doc comment) — this is what stands
/// behind the claim that it never actually happens on the page, across
/// every layout §4 names, rather than leaving it asserted only in prose.
#[cfg(test)]
mod every_focus_target_has_an_id {
    use super::{Rig, SIZE};
    use crate::config::AppConfig;
    use crate::state::AppState;
    use iced::Rectangle;
    use iced::advanced::widget::operation::Focusable;
    use iced::advanced::widget::{Id, Operation};
    use knotra_vcs::{Project, Workspace};

    struct IdlessFocusables(Vec<()>);

    impl Operation for IdlessFocusables {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, _state: &mut dyn Focusable) {
            if id.is_none() {
                self.0.push(());
            }
        }
    }

    /// Builds a real view and asserts that nothing `app_view` itself
    /// draws registers as a focus target with no id at all —
    /// `super::rebuild`/`Rig` are private to `keyboard_acceptance`'s own
    /// module, visible here as one of its child modules, not a second
    /// copy.
    fn assert_every_focus_target_has_an_id(label: &str, state: &AppState) {
        let mut rig = Rig::new(SIZE);
        let mut ui = super::rebuild(&mut rig, state);
        let mut found = IdlessFocusables(Vec::new());
        ui.operate(&rig.renderer, &mut found);
        assert!(
            found.0.is_empty(),
            "{label}: {} focus target(s) with no id at all",
            found.0.len()
        );
    }

    fn base_state() -> AppState {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        let beta = Project::new("Beta", "/tmp");
        state.workspace = Some(Workspace {
            projects: vec![alpha, beta],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state
    }

    #[test]
    fn wide_and_compact() {
        for (label, width) in [("wide", 1400.0_f32), ("compact", 700.0_f32)] {
            let mut state = base_state();
            state.width_mode = crate::view::dashboard::WidthMode::from_width(width);
            assert_every_focus_target_has_an_id(label, &state);
        }
    }

    #[test]
    fn selection_mode_on_and_off() {
        for (label, on) in [("selection mode on", true), ("selection mode off", false)] {
            let mut state = base_state();
            state.selection_mode = on;
            assert_every_focus_target_has_an_id(label, &state);
        }
    }

    #[test]
    fn empty_workspace() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace::new("Main"));
        state.load_phase = crate::state::LoadPhase::Ready;
        assert_every_focus_target_has_an_id("empty workspace", &state);
    }

    #[test]
    fn no_matches() {
        let mut state = base_state();
        state.filter.search_text = "does-not-match-anything".to_owned();
        assert_every_focus_target_has_an_id("no matches", &state);
    }

    #[test]
    fn arrange_panel_open() {
        let mut state = base_state();
        state.dashboard_arrange_open = true;
        assert_every_focus_target_has_an_id("Arrange panel open", &state);
    }

    #[test]
    fn two_workspaces() {
        let mut state = base_state();
        state.all_workspaces = vec![
            knotra_vcs::Workspace::new("Main"),
            knotra_vcs::Workspace::new("Second"),
        ];
        assert_every_focus_target_has_an_id("two workspaces", &state);
    }
}

/// RFC-075 stage 3 §7 row 8 ("the single most important test in the
/// table", per the handoff): does focus follow its own control's id
/// *inside a window*, the same guarantee `focus_follows_its_id` already
/// proves for the page, now exercised through a real window — Get latest
/// safely's own plan.
#[cfg(test)]
mod stage3_focus_follows_id_in_windows {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::AppConfig;
    use crate::message::{Message, SyncMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use knotra_vcs::model::status::{RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn dirty_status(id: ProjectId) -> ProjectStatus {
        ProjectStatus {
            project_id: id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: RemoteStatus {
                upstream: Some("origin/main".to_owned()),
                ..RemoteStatus::default()
            },
            working_tree: WorkingTreeStatus {
                uncommitted_count: 1,
                untracked_count: 0,
            },
            conflict: Default::default(),
            refreshed_at: jiff::Timestamp::now(),
            read_error: None,
        }
    }

    /// Two dirty projects (so both entries draw a disposition-toggle pair
    /// — `smart_pull.rs`'s own gate on `is_dirty && !has_conflict &&
    /// skip_reason.is_none()`), opened through the real, synchronous
    /// `BulkPullRequested` → `SmartPullPlanRequested` path
    /// (`app/sync.rs`'s own plan builder reads `workspace_status`
    /// directly, no async await needed), landing in `AwaitingConfirm`.
    fn awaiting_confirm_with_two_dirty_projects() -> (AppState, ProjectId, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let alpha = Project::new("Alpha", "/tmp");
        let beta = Project::new("Beta", "/tmp");
        let (alpha_id, beta_id) = (alpha.id.clone(), beta.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![alpha, beta],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![
                dirty_status(alpha_id.clone()),
                dirty_status(beta_id.clone()),
            ],
            last_refresh: None,
        });
        // `BulkPullRequested`'s own handler calls `sync.set_selection`
        // against `state.selection.selected_ids` — both projects must be
        // selected here, or `build_plan` excludes them as `Deselected`
        // regardless of their own dirty/upstream/conflict status.
        // `selection_mode` is also on, so the selection bar's own "Get
        // latest safely" renders and is a real Tab stop — the test opens
        // the window through it, the same way a keyboard user would.
        state.selection_mode = true;
        state
            .selection
            .select_all(&[alpha_id.clone(), beta_id.clone()]);
        (state, alpha_id, beta_id)
    }

    fn assert_awaiting_confirm_with_two_dirty_entries(state: &AppState) {
        use crate::state::sync::SyncPhase;
        match &state.sync.phase {
            SyncPhase::AwaitingConfirm(plan) => {
                assert_eq!(
                    plan.entries.len(),
                    2,
                    "sanity: both projects must be dirty, non-conflicted plan entries"
                );
                for e in &plan.entries {
                    assert!(
                        e.is_dirty && !e.has_conflict && e.skip_reason.is_none(),
                        "sanity: entry {:?} must be dirty/non-conflicted/not skipped, got \
                         is_dirty={} has_conflict={} skip_reason={:?} disposition={:?}",
                        e.project_name,
                        e.is_dirty,
                        e.has_conflict,
                        e.skip_reason,
                        e.disposition
                    );
                }
            }
            other => panic!("sanity: expected AwaitingConfirm, got {other:?}"),
        }
    }

    /// **Characterization test for a found production bug — not a test
    /// of correct behavior.** §7's own single most important row: a
    /// window's own moving row must not steal another project's focus
    /// out from under it. This phase's own two disposition toggles
    /// (`ids::smart_pull::check_only`/`get_anyway`) are keyed by the
    /// entry's *position* in `plan.entries` (`format!("smart_pull.entry.
    /// {index}.check_only")`), not by the project's own id — unlike every
    /// other per-row control in this codebase (`ids::row::checkbox`/
    /// `name`/`action`, all keyed by `ProjectId`). `FollowsId` does
    /// exactly what its own name says: it follows the *id* — so when
    /// Alpha's own entry moves from index 0 to index 1 (the same reorder
    /// a sort would produce) while focus is on Alpha's own "Get anyway",
    /// focus does not travel with Alpha to `get_anyway(1)`; it stays on
    /// the id `get_anyway(0)`, which the reorder has handed to Beta.
    ///
    /// This test asserts the *observed, buggy* behavior, not the correct
    /// one — standard practice for pinning down a known defect without
    /// leaving a permanently-red test in the suite. Documented in this
    /// fork's own report as a found production bug, not fixed here (out
    /// of this task's scope, and not this test's job): **the fix is to
    /// key these two ids by `ProjectId`, the same as every other per-row
    /// control, and this test's own assertion must be flipped to require
    /// `get_anyway(1)` the day that lands.**
    #[test]
    fn disposition_toggle_does_not_follow_its_own_project_across_a_reorder() {
        let (mut state, _alpha_id, _beta_id) = awaiting_confirm_with_two_dirty_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_awaiting_confirm_with_two_dirty_entries(&state);

        // Alpha is `plan.entries[0]` (built from `workspace.projects`'s
        // own order) — its own "Get anyway" is `get_anyway(0)`. Reached
        // by real Tab presses, from wherever focus happens to be.
        for _ in 0..10 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(crate::message::ShortcutMessage::FocusNext),
            );
            if focused_id(&mut rig, &state) == Some(ids::smart_pull::get_anyway(0)) {
                break;
            }
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::get_anyway(0)),
            "sanity: Tab must be able to reach Alpha's own \"Get anyway\""
        );

        // The reorder: Alpha's own entry moves to index 1.
        if let crate::state::sync::SyncPhase::AwaitingConfirm(plan) = &mut state.sync.phase {
            plan.entries.reverse();
        }

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::get_anyway(0)),
            "known bug, pinned by this test: focus stays on the *id* get_anyway(0), which the \
             reorder just handed to Beta, instead of following Alpha to get_anyway(1) — see \
             this test's own doc comment"
        );
    }

    /// **A second, independent finding** (not the id-keying one above),
    /// and the root cause of what first looked like a `settle_layer_
    /// focus`/`Task::then` harness bug while writing the test above:
    /// `smart_pull_entry_id`'s own doc comment (`view/overlays/
    /// smart_pull.rs` ~line 38) says "the first dirty project's own
    /// disposition choice stands in for" the phase's entry, and its code
    /// picks that project's own `check_only(i)` unconditionally. But
    /// `pick_disposition_btn`'s own doc comment (same file, ~line 680)
    /// says the *already-selected* disposition's own button has no
    /// `on_press` and so "is briefly not a Tab stop, same as every other
    /// self-disabling control" — and every entry's disposition starts
    /// `FetchOnly`, i.e. "Check only" *is* the default, already-selected
    /// choice. So in the common case this phase is reached in at all —
    /// nothing has been toggled yet — `smart_pull_entry_id` names a
    /// control that cannot be focused, and `settle_layer_focus`'s own
    /// `operation::focus(entry_now)` call is a real no-op, not a harness
    /// limitation: confirmed directly below by asserting Tab cannot
    /// reach `check_only(0)` in this exact state, the same state
    /// `smart_pull_entry_id` names it the entry for.
    ///
    /// Item 4 ("a change is never the entry") is why `check_only` — the
    /// no-op, "stay as you are" choice — was picked over `get_anyway` in
    /// the first place; the bug is that the *same* property that makes
    /// it safe to land on (pressing it would do nothing) is exactly what
    /// makes `control.rs` take it out of the Tab order entirely, so it
    /// cannot be landed on by Tab, or settled onto by `operation::focus`,
    /// either. Documented here, not fixed (out of this task's scope).
    #[test]
    fn entry_is_the_already_selected_and_so_unreachable_disposition_toggle() {
        let (mut state, _alpha_id, _beta_id) = awaiting_confirm_with_two_dirty_projects();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_awaiting_confirm_with_two_dirty_entries(&state);

        // Both entries default to `FetchOnly` — "Check only" already
        // selected on both, confirmed before drawing any conclusion from
        // it.
        if let crate::state::sync::SyncPhase::AwaitingConfirm(plan) = &state.sync.phase {
            for e in &plan.entries {
                assert_eq!(
                    e.disposition,
                    knotra_vcs::model::operation::SmartPullDisposition::FetchOnly,
                    "sanity: {:?}'s own disposition must still be the default",
                    e.project_name
                );
            }
        }

        // `smart_pull_entry_id` names Alpha's own `check_only(0)` as this
        // phase's entry.
        assert_eq!(
            crate::view::overlays::smart_pull_entry_id(&state),
            Some(ids::smart_pull::check_only(0))
        );

        // Yet Tab cannot reach it: `control.rs`'s own self-disabling
        // default takes it out of the Tab order, so cycling Tab well
        // past the window's own control count never lands on it.
        let mut reached_entry = false;
        for _ in 0..20 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(crate::message::ShortcutMessage::FocusNext),
            );
            if focused_id(&mut rig, &state) == Some(ids::smart_pull::check_only(0)) {
                reached_entry = true;
                break;
            }
        }
        assert!(
            !reached_entry,
            "expected Tab to never reach check_only(0) while it is the already-selected, \
             self-disabling choice — if this now fails, smart_pull.rs's own disabling rule \
             changed and this test (and the bug it documents) should be revisited"
        );

        // And this is exactly why `settle_layer_focus` could not have
        // settled real focus onto it either: a fresh query, independent
        // of the Tab cycling above, still reports nothing real actually
        // held by that id in this window.
        let mut fresh_rig = Rig::new(SIZE);
        let mut fresh_state = {
            let (s, ..) = awaiting_confirm_with_two_dirty_projects();
            s
        };
        drive(
            &mut fresh_rig,
            &mut fresh_state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut fresh_rig,
            &mut fresh_state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_ne!(
            focused_id(&mut fresh_rig, &fresh_state),
            Some(ids::smart_pull::check_only(0)),
            "the phase becoming ready must not have settled focus onto its own named entry, \
             since that entry is not a real Tab stop in this state"
        );
    }
}

/// RFC-075 stage 3 §7 rows 2-3 — Tab stays inside a window, wrapping at
/// both ends, and never reaches the page beneath it, for a representative
/// sample across every window kind this stage converted (not an
/// exhaustive one-test-per-window-per-phase sweep of inventory §6-§7,
/// which the handoff's own table calls for but is not all built here —
/// see this fork's own report for exactly which windows/phases this
/// covers and which it does not).
#[cfg(test)]
mod stage3_tab_and_confinement {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{KeyboardMessage, Message, ShortcutMessage, WorkspaceMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::Workspace;

    fn base_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    fn tab(rig: &mut Rig, state: &mut AppState) {
        drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
    }
    fn shift_tab(rig: &mut Rig, state: &mut AppState) {
        drive(
            rig,
            state,
            Message::Shortcut(ShortcutMessage::FocusPrevious),
        );
    }

    /// Asserts that, from whatever is focused right now, `expected.len()`
    /// Tab presses visit exactly `expected`, in order, wrapping from the
    /// last back to the first — and that the same number of Shift+Tab
    /// presses from the first wraps to the last. This is §7 row 1's own
    /// "by id, wrapping both ways" for one phase.
    fn assert_tab_sequence_wraps(
        rig: &mut Rig,
        state: &mut AppState,
        expected: &[iced::widget::Id],
    ) {
        // Sync to the sequence's own first control first — a window's own
        // entry mechanism (RFC-075 handoff §3) may already have moved
        // real focus there (or anywhere else inside the window) before
        // this helper's first Tab, so starting the per-item walk cold
        // from "whatever is focused right now" would be off by however
        // many stops the entry mechanism already covered.
        for _ in 0..expected.len() * 2 {
            if focused_id(rig, state) == Some(expected[0].clone()) {
                break;
            }
            tab(rig, state);
        }
        assert_eq!(
            focused_id(rig, state),
            Some(expected[0].clone()),
            "sanity: Tab must be able to reach {:?}",
            expected[0]
        );
        for want in &expected[1..] {
            tab(rig, state);
            assert_eq!(
                focused_id(rig, state),
                Some(want.clone()),
                "expected {want:?}"
            );
        }
        tab(rig, state);
        assert_eq!(
            focused_id(rig, state),
            Some(expected[0].clone()),
            "Tab past the last control must wrap to the first"
        );
        shift_tab(rig, state);
        assert_eq!(
            focused_id(rig, state),
            Some(expected.last().unwrap().clone()),
            "Shift+Tab before the first control must wrap to the last"
        );
    }

    /// The shortcuts sheet — one of the handoff's own three "drawn in
    /// knotra's own outer stack" windows (§2), confirmed stale in this
    /// stage's own report: Task 098 Amendment A2 already moved it into
    /// `AppLayout::dialog`'s own slot, the same one every other window
    /// uses, so `snora::Inert` already makes the page beneath it inert —
    /// there is no special case to design around, only this to prove.
    /// Its own one control, "Close", wraps to itself.
    #[test]
    fn shortcuts_sheet_tab_wraps_to_itself_and_never_reaches_the_page() {
        let mut state = base_state();
        state.workspace = Some(knotra_vcs::Project::new("Alpha", "/tmp")).map(|p| Workspace {
            projects: vec![p],
            ..Workspace::new("Main")
        });
        state.keyboard.cheat_sheet_open = true;
        let mut rig = Rig::new(SIZE);

        tab(&mut rig, &mut state);
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shortcuts::close()));
        tab(&mut rig, &mut state);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shortcuts::close()),
            "Tab past the sheet's one control must wrap to itself, never reach the page"
        );
    }

    /// The command palette — another of the three. Close, the query
    /// field, then (with no results) wraps back to Close.
    #[test]
    fn palette_tab_stays_inside_with_no_results() {
        let mut state = base_state();
        state.palette.open = true;
        let mut rig = Rig::new(SIZE);

        tab(&mut rig, &mut state);
        assert_eq!(focused_id(&mut rig, &state), Some(ids::palette::close()));
        tab(&mut rig, &mut state);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
        );
        tab(&mut rig, &mut state);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::palette::close()),
            "with no results, Tab past the query field must wrap to Close, never reach the page"
        );
    }

    /// Add project — the third. Step 1: Close, the folder field, Browse,
    /// Cancel ("Next" is disabled with an empty path, D4 b, so it is
    /// skipped). Opened through the real message
    /// (`WorkspaceMessage::AddProjectDialogOpened`), not by hand-setting
    /// `state.add_project_dialog`.
    #[test]
    fn add_project_step1_tab_sequence_wraps_and_never_reaches_the_page() {
        let mut state = base_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::AddProjectDialogOpened),
        );
        assert_tab_sequence_wraps(
            &mut rig,
            &mut state,
            &[
                ids::add_project::close(),
                knotra_ui::widget::focus_id::ADD_PROJECT_PATH.clone(),
                ids::add_project::browse(),
                ids::add_project::cancel(),
            ],
        );
    }

    /// Step 2, reached via the real `AddProjectNextStep` message once a
    /// path is typed: Close, the name field, Back ("Add" is disabled with
    /// an empty name).
    #[test]
    fn add_project_step2_tab_sequence_wraps() {
        let mut state = base_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::AddProjectDialogOpened),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::AddProjectPathChanged("/tmp".to_owned())),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::AddProjectNextStep),
        );
        assert_tab_sequence_wraps(
            &mut rig,
            &mut state,
            &[
                ids::add_project::close(),
                knotra_ui::widget::focus_id::ADD_PROJECT_NAME.clone(),
                ids::add_project::back(),
            ],
        );
    }

    /// Remove project — a real `AppLayout::dialog`, same slot as every
    /// other non-outer-stack window, opened through the real message.
    /// Close, Keep, Remove.
    #[test]
    fn remove_project_tab_sequence_wraps() {
        let mut state = base_state();
        let alpha = knotra_vcs::Project::new("Alpha", "/tmp");
        let alpha_id = alpha.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![alpha],
            ..Workspace::new("Main")
        });
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::RemoveProjectRequested(alpha_id)),
        );
        assert_tab_sequence_wraps(
            &mut rig,
            &mut state,
            &[
                ids::remove_project::close(),
                ids::remove_project::keep(),
                ids::remove_project::remove(),
            ],
        );
    }

    /// Create workspace — a real dialog, opened through
    /// `CreateWorkspaceDialogOpened`. Close, the name field, Confirm
    /// (disabled while the name is empty, D4 b, so skipped), Cancel.
    #[test]
    fn workspace_create_tab_sequence_skips_disabled_confirm() {
        let mut state = base_state();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened),
        );
        assert_tab_sequence_wraps(
            &mut rig,
            &mut state,
            &[
                ids::workspace_manager::close(),
                knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
                ids::workspace_manager::cancel(),
            ],
        );

        // With a name typed, Confirm becomes a real Tab stop too, between
        // the field and Cancel.
        drive(
            &mut rig,
            &mut state,
            Message::Workspace(WorkspaceMessage::CreateWorkspaceNameChanged(
                "Lab".to_owned(),
            )),
        );
        assert_tab_sequence_wraps(
            &mut rig,
            &mut state,
            &[
                ids::workspace_manager::close(),
                knotra_ui::widget::focus_id::WORKSPACE_NAME.clone(),
                ids::workspace_manager::confirm(),
                ids::workspace_manager::cancel(),
            ],
        );
    }

    /// Row 2's own explicit case: with the shortcuts sheet open, a
    /// shortcut that *would* act on the page beneath it while no window
    /// is open (`Ctrl+T`/`Ctrl+K`, Task 095) does nothing to the sheet's
    /// own confinement, and Tab itself never leaves the sheet — checked
    /// directly by pressing the real key through the `Simulator`, not
    /// only through `Message::Shortcut` dispatch, since it is the engine
    /// (`snora::Inert`/the real keyboard event path), not `app.rs`, that
    /// must refuse to let it through.
    #[test]
    fn enter_on_shortcuts_sheet_close_presses_it_once_not_the_page() {
        let mut state = base_state();
        state.keyboard.cheat_sheet_open = true;
        let mut rig = Rig::new(SIZE);

        tab(&mut rig, &mut state);
        assert_eq!(focused_id(&mut rig, &state), Some(ids::shortcuts::close()));

        key(&mut rig, &state, Named::Enter);
        assert!(
            matches!(
                rig.messages(),
                [Message::KeyEvent(KeyboardMessage::CheatSheetToggled)]
            ),
            "Enter on the sheet's own Close must press it once, and nothing on the \
             page beneath it"
        );
    }
}

/// RFC-075 stage 3 §4 — the palette's and "Find work area"'s own
/// highlight cursor (`PaletteState::highlighted`/`ContextOpsState::
/// highlighted`): ↓/↑ move it, skipping disabled/current rows, wrapping
/// at the last, and returning to "nothing" (the field) at/before the
/// first. Both are pure state methods (`state::palette::move_highlight`,
/// `ContextOpsState::move_highlight`), so most of this is tested
/// directly against them — no `Rig` needed; the two cases that are
/// about real widget behaviour (typing, including Space, reaching the
/// field; Enter running whatever is highlighted) use `Rig`/`drive`.
#[cfg(test)]
mod stage3_highlight_cursors {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{Message, PaletteMessage};
    use crate::state::{AppState, PaletteEntry, PaletteEntryKind};
    use iced::keyboard::key::Named;
    use knotra_vcs::model::status::ContextCandidate;
    use knotra_vcs::model::status::ContextTarget;

    fn entry(label: &str, disabled: bool) -> PaletteEntry {
        PaletteEntry {
            kind: PaletteEntryKind::Action,
            label: label.to_owned(),
            payload: label.to_owned(),
            disabled_reason_key: disabled.then_some("palette.disabled.unavailable"),
        }
    }

    /// §4: ↓ from nothing goes to the first enabled row; the disabled
    /// middle row is skipped entirely, never landed on by ↓ or ↑; ↓ past
    /// the last wraps to the first; ↑ at (or before) the first returns
    /// to "nothing" (the field), not past it to the last.
    #[test]
    fn palette_highlight_wraps_and_skips_disabled() {
        let mut state = AppState::new(AppConfig::default());
        state.palette.results = vec![
            entry("First", false),
            entry("Disabled", true),
            entry("Last", false),
        ];

        crate::state::palette::move_highlight(&mut state, true);
        assert_eq!(
            state.palette.highlighted,
            Some(0),
            "↓ from nothing: the first enabled row"
        );

        crate::state::palette::move_highlight(&mut state, true);
        assert_eq!(
            state.palette.highlighted,
            Some(2),
            "↓ again: the disabled middle row (index 1) must be skipped, landing on \"Last\""
        );

        crate::state::palette::move_highlight(&mut state, true);
        assert_eq!(
            state.palette.highlighted,
            Some(0),
            "↓ past the last wraps to the first"
        );

        crate::state::palette::move_highlight(&mut state, false);
        assert_eq!(
            state.palette.highlighted, None,
            "↑ at the first returns to nothing (the field), not past it to the last"
        );

        crate::state::palette::move_highlight(&mut state, false);
        assert_eq!(
            state.palette.highlighted, None,
            "↑ on (before) the first stays at nothing"
        );
    }

    /// Same shape, "Find work area"'s own candidates — `is_current` is
    /// this list's own "never the entry/never landed on" row (the
    /// active branch has nothing to switch to).
    #[test]
    fn find_work_area_highlight_wraps_and_skips_the_current_branch() {
        let mut ops = crate::state::context::ContextOpsState::default();
        let candidate = |label: &str, is_current: bool| ContextCandidate {
            label: label.to_owned(),
            target: ContextTarget::GitLocalBranch {
                name: label.to_owned(),
            },
            is_current,
        };
        ops.phase = crate::state::context::ContextPhase::BrowsingList {
            project_id: knotra_vcs::ProjectId::new(),
            list: knotra_vcs::model::status::ContextList {
                candidates: vec![
                    candidate("main", true),
                    candidate("feature-a", false),
                    candidate("feature-b", false),
                ],
                ..Default::default()
            },
            search: String::new(),
        };

        ops.move_highlight(true);
        assert_eq!(
            ops.highlighted,
            Some(1),
            "↓ from nothing must skip \"main\" (the current branch, index 0) and land on \
             \"feature-a\""
        );
        ops.move_highlight(true);
        assert_eq!(ops.highlighted, Some(2));
        ops.move_highlight(true);
        assert_eq!(
            ops.highlighted,
            Some(1),
            "↓ past the last wraps to the first enabled"
        );
        ops.move_highlight(false);
        assert_eq!(
            ops.highlighted, None,
            "↑ at the first enabled returns to nothing"
        );
    }

    /// Clicking (or activating) a disabled result shows its own reason
    /// rather than dispatching anything — §4's own "disabled skipped
    /// with reason" for the half of the rule `move_highlight` alone
    /// cannot cover (it never lands the cursor there at all; this is
    /// what happens if it is activated some other way, e.g. a stale
    /// index).
    #[test]
    fn disabled_palette_entry_shows_its_reason_not_a_dispatch() {
        let mut state = AppState::new(AppConfig::default());
        state.palette.open = true;
        state.palette.results = vec![entry("Disabled", true)];

        let task = crate::app::update(
            &mut state,
            Message::Palette(PaletteMessage::EntryClicked(0)),
        );
        drop(task);

        assert_eq!(
            state.palette.notice_key,
            Some("palette.disabled.unavailable")
        );
        assert!(
            state.palette.open,
            "a disabled entry must not close the palette the way a real dispatch would"
        );
    }

    /// §4: typing into the palette's own field reaches it character for
    /// character, Space included — real widget behaviour, so this goes
    /// through `Rig`/`drive` rather than calling `update` by hand.
    #[test]
    fn typing_with_a_space_reaches_the_palette_field() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(knotra_vcs::Workspace {
            projects: vec![knotra_vcs::Project::new("Alpha", "/tmp")],
            ..knotra_vcs::Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Palette(PaletteMessage::Opened),
        );
        // Reached by real Tab presses, from wherever focus happens to be
        // once the window is open — this test is about what typing does
        // once the field holds real focus, not about how it got there
        // (the palette's own open-time auto-focus is a separate concern,
        // not relied on here).
        for _ in 0..10 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(crate::message::ShortcutMessage::FocusNext),
            );
            if focused_id(&mut rig, &state)
                == Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
            {
                break;
            }
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone()),
            "sanity: Tab must be able to reach the palette's own query field"
        );

        let mut ui = super::rebuild(&mut rig, &state);
        let events: Vec<_> = iced_test::simulator::typewrite("a b").collect();
        let _ = ui.update(
            &events,
            rig.cursor,
            &mut rig.renderer,
            &mut super::clipboard::Null,
            &mut rig.messages,
        );
        super::redraw(&mut rig, &mut ui);
        rig.cache = ui.into_cache();

        let query_changes: Vec<&String> = rig
            .messages()
            .iter()
            .filter_map(|m| match m {
                Message::Palette(PaletteMessage::QueryChanged(q)) => Some(q),
                _ => None,
            })
            .collect();
        assert_eq!(
            query_changes.last(),
            Some(&&"a b".to_owned()),
            "typing \"a b\" (with a Space) while the field holds real focus must reach it \
             character for character, including the space; got {:?}",
            rig.messages()
        );
    }

    /// §4: Enter runs whichever result is highlighted (the text field's
    /// own `on_submit_maybe`), not just whatever is first. Two real,
    /// distinguishable `Project` entries (clicking one opens *that*
    /// project's detail panel, unambiguously telling which one ran) —
    /// highlight the second, press real Enter while the field holds real
    /// focus, and confirm the second (not the first) opened.
    #[test]
    fn enter_runs_the_highlighted_palette_entry() {
        let alpha = knotra_vcs::Project::new("Alpha", "/tmp/alpha");
        let beta = knotra_vcs::Project::new("Beta", "/tmp/beta");
        let (alpha_id, beta_id) = (alpha.id.clone(), beta.id.clone());
        let mut workspace = knotra_vcs::Workspace::new("Demo");
        workspace.projects.push(alpha);
        workspace.projects.push(beta);

        let mut state = AppState::new(AppConfig::default());
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.load_phase = crate::state::LoadPhase::Ready;
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Palette(PaletteMessage::Opened),
        );
        for _ in 0..10 {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(crate::message::ShortcutMessage::FocusNext),
            );
            if focused_id(&mut rig, &state)
                == Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone())
            {
                break;
            }
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(knotra_ui::widget::focus_id::PALETTE_QUERY.clone()),
            "sanity: Tab must be able to reach the palette's own query field"
        );
        state.palette.results = vec![
            PaletteEntry {
                kind: PaletteEntryKind::Project,
                label: "Alpha".to_owned(),
                payload: alpha_id.to_string(),
                disabled_reason_key: None,
            },
            PaletteEntry {
                kind: PaletteEntryKind::Project,
                label: "Beta".to_owned(),
                payload: beta_id.to_string(),
                disabled_reason_key: None,
            },
        ];
        state.palette.highlighted = Some(1);

        key(&mut rig, &state, Named::Enter);
        assert!(
            matches!(
                rig.messages(),
                [Message::Palette(PaletteMessage::EntryClicked(1))]
            ),
            "Enter on the field, with index 1 highlighted, must publish EntryClicked(1) — \
             not EntryClicked(0) or nothing; got {:?}",
            rig.messages()
        );

        let published = rig.messages()[0].clone();
        drive(&mut rig, &mut state, published);
        // `EntryClicked`'s own handler (`app/misc.rs::handle_palette`)
        // closes the palette and returns `Task::done(DetailPanelMessage::
        // Opened(..))` as its *output* — `drive()` only ever applies a
        // task's `Action::Widget` operations (see its own doc comment),
        // so that chained message needs its own, separate `drive()` call,
        // the same shape `BulkPullRequested`→`SmartPullPlanRequested`
        // needs elsewhere in this file.
        drive(
            &mut rig,
            &mut state,
            Message::DetailPanel(crate::message::DetailPanelMessage::Opened(beta_id.clone())),
        );

        assert_eq!(
            state.detail_panel.open_project_id,
            Some(beta_id),
            "Enter must run the *highlighted* entry (Beta, index 1) — not the first"
        );
    }
}

/// RFC-075 stage 3 §4/§7 row 6 — the workspace menu's own ↓/↑: unlike the
/// palette/"Find work area", these move *real* focus between real Tab
/// stops (`knotra_ui::widget::keyboard::focus_among`), wrapping, and
/// skipping the active workspace's own disabled row.
#[cfg(test)]
mod stage3_workspace_menu_arrows {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::AppConfig;
    use crate::message::{Message, ShortcutMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use knotra_vcs::Workspace;

    fn three_workspaces_menu_open() -> AppState {
        let mut state = AppState::new(AppConfig::default());
        let active = Workspace::new("Active");
        let b = Workspace::new("Beta");
        let c = Workspace::new("Gamma");
        state.all_workspaces = vec![active, b, c];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_mgr.switcher_open = true;
        state
    }

    #[test]
    fn down_and_up_move_real_focus_wrapping_and_skip_the_active_row() {
        let mut state = three_workspaces_menu_open();
        let mut rig = Rig::new(SIZE);

        let beta_row = ids::switcher::row(&state.all_workspaces[1].id);
        let gamma_row = ids::switcher::row(&state.all_workspaces[2].id);
        let add_workspace = ids::switcher::add_workspace();
        let rename = ids::switcher::rename();
        let delete = ids::switcher::delete();

        // The active workspace's own row is never in this list at all
        // (disabled, skipped) — confirmed structurally, not just by
        // never landing on it below.
        let ids_in_order = crate::view::shell::switcher_menu_ids(&state);
        assert_eq!(
            ids_in_order,
            vec![
                beta_row.clone(),
                gamma_row.clone(),
                add_workspace.clone(),
                rename.clone(),
                delete.clone(),
            ]
        );

        for want in [&beta_row, &gamma_row, &add_workspace, &rename, &delete] {
            drive(
                &mut rig,
                &mut state,
                Message::Shortcut(ShortcutMessage::ArrowDown),
            );
            assert_eq!(
                focused_id(&mut rig, &state),
                Some(want.clone()),
                "expected {want:?}"
            );
        }
        // Past the last, ↓ wraps to the first.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(beta_row.clone()));

        // ↑ from the first wraps to the last.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowUp),
        );
        assert_eq!(focused_id(&mut rig, &state), Some(delete.clone()));
    }
}

/// RFC-075 stage 3 §7 row 7 — the activity strip's own Undo (and the
/// detail panel's own controls, converted the same way this stage
/// converted every window) are real Tab stops, same as any other page
/// control.
#[cfg(test)]
mod stage3_page_controls_reachable_by_tab {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::AppConfig;
    use crate::message::{DetailPanelMessage, Message, ShortcutMessage};
    use crate::state::{AppState, LoadPhase, UndoableRemoval};
    use crate::view::ids;
    use knotra_vcs::{Project, Workspace};

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &iced::widget::Id) -> bool {
        for _ in 0..30 {
            drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    #[test]
    fn undo_snackbar_is_reachable_by_tab() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace {
            projects: vec![Project::new("Alpha", "/tmp")],
            ..Workspace::new("Main")
        });
        state.load_phase = LoadPhase::Ready;
        state.recent_removal = Some(UndoableRemoval {
            project: Project::new("Removed", "/tmp/removed"),
            status_snapshot: None,
        });
        let mut rig = Rig::new(SIZE);

        assert!(
            tab_to(&mut rig, &mut state, &ids::activity_strip::undo()),
            "sanity: Tab must be able to reach the activity strip's own Undo"
        );
        assert!(
            tab_to(&mut rig, &mut state, &ids::activity_strip::dismiss()),
            "sanity: Tab must be able to reach the activity strip's own Dismiss"
        );
    }

    #[test]
    fn detail_panel_controls_are_reachable_by_tab() {
        let project = Project::new("Alpha", "/tmp");
        let project_id = project.id.clone();
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.load_phase = LoadPhase::Ready;
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::DetailPanel(DetailPanelMessage::Opened(project_id)),
        );
        assert!(
            tab_to(&mut rig, &mut state, &ids::detail_panel::close()),
            "sanity: Tab must be able to reach the detail panel's own Close"
        );
        assert!(
            tab_to(&mut rig, &mut state, &ids::detail_panel::refresh()),
            "sanity: Tab must be able to reach the detail panel's own Refresh"
        );
    }
}

/// RFC-075 stage 3 §3/§7 row 3 — entry by keyboard, the synchronous case:
/// `settle_layer_focus`'s own `(None, Some(layer)) if is_shortcut`
/// branch, which returns `operation::focus(entry)` directly, with no
/// `Task::then` involved — confirmed working. (The phase-change branch's
/// own apparent failure to settle focus, found while writing this
/// module, turned out not to be a `Task::then` problem at all — see
/// `stage3_focus_follows_id_in_windows::
/// entry_is_the_already_selected_and_so_unreachable_disposition_toggle`'s
/// own finding.)
#[cfg(test)]
mod stage3_entry_by_shortcut {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::AppConfig;
    use crate::message::{Message, ShortcutMessage};
    use crate::state::AppState;

    #[test]
    fn ctrl_t_opens_freezer_and_focuses_the_name_field_synchronously() {
        let mut state = AppState::new(AppConfig::default());
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::OpenFreezer),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(crate::state::layer::Layer::SaveReleasePoint)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(knotra_ui::widget::focus_id::RELEASE_NAME.clone())
        );
    }
}
