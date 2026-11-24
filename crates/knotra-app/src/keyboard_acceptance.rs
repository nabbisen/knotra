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

    /// **Found while capturing this stage's GUI evidence (C1), and
    /// fixed**: opening the Arrange panel from the keyboard used to lose
    /// real focus entirely, rather than leaving it on "Arrange" — the
    /// very next Tab wrapped all the way back to the page's first control
    /// instead of continuing into Group/Sort, defeating the behaviour
    /// RFC-075 exists to provide.
    ///
    /// **Stays under Amendment A1**: this is a *toolbar* control, outside
    /// the region `knotra_ui::widget::focus_by_key` wraps (the dashboard's
    /// row content — see that module's own doc comment for why it stops
    /// there) — A1's own mechanism does not reach the toolbar, so it
    /// cannot make this fix unnecessary.
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

    /// The same finding, the same fix, for "Select" — also a toolbar
    /// control, also outside A1's own wrap.
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

    /// The reverse direction of the same guarantee: the stage-2 fixes
    /// already shipped (`ArrangeToggled`/`SelectionMessage::ModeEntered`
    /// re-asserting focus on a *toolbar* control once their own panel or
    /// selection bar appears) are not undone by this content's own
    /// reconciliation, even though entering selection mode also changes
    /// this content's own key set in the very same step (every row gains
    /// a checkbox). Without `FollowsId::resync`, this would regress: see
    /// its own doc comment for why.
    #[test]
    fn entering_selection_mode_does_not_undo_its_own_refocus_on_select() {
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
            Some(ids::toolbar::select_mode()),
            "the existing stage-2 fix must still win: focus on \"Select\", not pulled back \
             onto Alpha's row by this content's own reconciliation"
        );
    }

    /// Amendment A1 §1's own measurement: "Check now" (`ids::shell::
    /// refresh`) is a *shell* control, outside the region `focus_by_key`
    /// wraps (see that module's own doc comment for why it stops at the
    /// dashboard's row content) — a refresh disabling it mid-flight is not
    /// this mechanism's concern at all, and nothing here changes it. This
    /// records the observed behaviour rather than assuming it: pressing
    /// Enter disables "Check now" (`on_press_maybe(None)` while
    /// `is_refreshing`), and `snora::design::Control::operate`'s own rule
    /// ("registers as a focus target — if it can be pressed") means a
    /// disabled control stops reporting as focused at all, the same way
    /// `focused_id` finds nothing focused immediately afterward. The
    /// handoff's own worry — "a Tab pressed during the refresh must not
    /// start again from the top of the page" — does not materialize:
    /// `traverse_wrapping`'s own "nothing focused → start from the first
    /// target going forward" rule means the *next* Tab after "Check now"
    /// going disabled correctly continues to whatever comes after it in
    /// reading order (History, here), not back to the page's own start —
    /// because "the first target going forward" and "the control after
    /// Check now" are the same thing from a still-disabled Check now's own
    /// position; only Shift+Tab from here would reveal the difference,
    /// and it is not this table row's own question.
    #[test]
    fn check_now_disabling_mid_refresh_is_measured_not_fixed() {
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
        assert_eq!(
            focused_id(&mut rig, &state),
            None,
            "measured: a disabled control stops reporting as focused at all"
        );

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::nav_history()),
            "measured: Tab continues to the control after the now-disabled \"Check now\", \
             not back to the page's own start"
        );
    }
}
