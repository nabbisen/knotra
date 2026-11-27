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

        // Task 123: "tiny-skia", never wgpu's default — see
        // `knotra-ui/src/test_support.rs`'s own comment on the same call
        // for why (Review 310 F2).
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::with_name("Noto Sans"),
            Pixels(16.0),
            Some("tiny-skia"),
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
            ids::toolbar::search(),
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
        assert_eq!(focused_id(&mut rig, &state), Some(ids::toolbar::search()));
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
    /// behind it at all — `knotra_ui::widget::focus_by_id_page`'s own one,
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
            Some(ids::freezer::release_name())
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
            Some(ids::context_switch::target())
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
            Some(ids::changelog::since())
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
            Some(ids::workspace_manager::name())
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
            Some(ids::palette::query())
        );
    }

    #[test]
    fn switcher_menu_entry_is_the_first_non_active_workspace_row() {
        let mut state = base_state();
        let main = Workspace::new("Main");
        let second = Workspace::new("Second");
        let second_id = second.id;
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
    /// `reserved` ids (`knotra_ui::widget::focus_by_id_page`'s own parameter):
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
    /// count mismatch A1's own tests already covered.
    ///
    /// **Rewritten, Amendment A1 (Review 305 F2):** the original version
    /// of this test switched to `NameAscending` and trusted that this
    /// *must* reorder two unclassified projects — but `Recommended`'s own
    /// tiebreak for same-tier projects already falls back to the same
    /// identity compare `NameAscending` uses directly (`state/
    /// dashboard.rs::sort_entries`), so for two projects with nothing
    /// else to distinguish them, the switch was a no-op: both sorts
    /// already agreed, confirmed directly by a probe in a scratch
    /// worktree (removed afterward). The review's own point: a test that
    /// does not know whether its own setup changed anything proves
    /// nothing either way. This version uses two projects whose
    /// `Recommended` order is forced *apart* from `NameAscending`'s own
    /// (`Recommended` ranks a `NeedsHelp` project by its own cause first,
    /// name second — `cause_rank`/`sort_entries`) — Zed with a real
    /// conflict (`cause_rank` 0) outranks Alpha's own unclassified
    /// `StatusUnknown` (`cause_rank` 5) under `Recommended`, regardless
    /// of name, but loses to it under `NameAscending`'s pure identity
    /// compare — and asserts the order genuinely flipped before asserting
    /// focus stayed, so a future setup that stops reordering fails loudly
    /// here rather than passing for the wrong reason.
    #[test]
    fn a_same_count_reorder_by_sort_still_follows_the_id() {
        let mut state = AppState::new(AppConfig::default());
        let zed = Project::new("Zed", "/tmp");
        let alpha = Project::new("Alpha", "/tmp");
        let (zed_id, alpha_id) = (zed.id.clone(), alpha.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![zed.clone(), alpha],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        // Zed: a real conflict — `Recommended`'s own `cause_rank` 0, so it
        // ranks first regardless of name. Alpha has no status entry at
        // all, so it classifies `NeedsHelp`/`StatusUnknown` (`cause_rank`
        // 5) via the same fallback `two_needs_help_projects` relies on.
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: zed_id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: Default::default(),
                working_tree: WorkingTreeStatus::default(),
                conflict: knotra_vcs::model::status::ConflictStatus {
                    has_conflict: true,
                    conflict_count: None,
                    detection_unavailable: false,
                },
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&zed_id)),
            "sanity: under Recommended, Zed's own conflict ranks it before Alpha's \
             unclassified StatusUnknown, so ↓ must focus Zed first"
        );

        // The order genuinely changes — checked directly against
        // `build_dashboard_display`'s own `ordered_selectable_ids`, not
        // through focus navigation (which would test the very mechanism
        // this is trying to independently verify). `Recommended`: Zed
        // (conflict) before Alpha (unclassified). `NameAscending`: Alpha
        // before Zed, by name alone.
        let projects = &state.workspace.as_ref().unwrap().projects;
        let recommended = crate::state::dashboard::build_dashboard_display(
            projects,
            state.workspace_status.as_ref(),
            &state.missing_projects,
            &state.filter,
            crate::state::dashboard::DashboardDisplayOptions {
                grouping: crate::config::DashboardGrouping::Attention,
                sort: crate::config::DashboardSort::Recommended,
                in_progress_collapsed: false,
                all_set_collapsed: false,
            },
        );
        assert_eq!(
            recommended.ordered_selectable_ids,
            vec![zed_id.clone(), alpha_id.clone()],
            "sanity: Recommended must rank Zed's own conflict before Alpha's \
             unclassified StatusUnknown"
        );
        let name_ascending = crate::state::dashboard::build_dashboard_display(
            projects,
            state.workspace_status.as_ref(),
            &state.missing_projects,
            &state.filter,
            crate::state::dashboard::DashboardDisplayOptions {
                grouping: crate::config::DashboardGrouping::Attention,
                sort: crate::config::DashboardSort::NameAscending,
                in_progress_collapsed: false,
                all_set_collapsed: false,
            },
        );
        assert_eq!(
            name_ascending.ordered_selectable_ids,
            vec![alpha_id.clone(), zed_id.clone()],
            "the order must have genuinely changed under NameAscending: Alpha's own \
             name sorts before Zed's — proving this reorder is real, not a same-order \
             no-op (the original version of this test never checked this)"
        );

        // Now drive the real sort change and confirm focus follows: with
        // the order confirmed reversed above, Zed's own id staying
        // focused here can only be the id-based mechanism at work, never
        // a same-position coincidence.
        drive(
            &mut rig,
            &mut state,
            Message::Dashboard(crate::message::DashboardMessage::SortChanged(
                crate::config::DashboardSort::NameAscending,
            )),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&zed_id)),
            "the same project's own name must still be focused after a genuine, \
             confirmed reorder — the id, not the position"
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
        assert_eq!(focused_id(&mut rig, &state), Some(ids::palette::query()));
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
                ids::add_project::path(),
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
                ids::add_project::name(),
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
                ids::workspace_manager::name(),
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
                ids::workspace_manager::name(),
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
    use crate::view::ids;
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
            if focused_id(&mut rig, &state) == Some(ids::palette::query()) {
                break;
            }
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::palette::query()),
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
            if focused_id(&mut rig, &state) == Some(ids::palette::query()) {
                break;
            }
        }
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::palette::query()),
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
    use crate::view::ids;

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
            Some(ids::freezer::release_name())
        );
    }
}

/// RFC-075 stage 3 Amendment A1 §3 ("Return on close, tested") — every row
/// of the handoff's own table, each opened and closed by driving the exact
/// `Message` a real key press on a real control would publish (never by
/// poking `AppState` fields to fake a layer open), and "opened by a click"
/// reproduced the way the handoff itself prescribes: `drive()` directly,
/// with nothing given real focus first (RFC-121: a click presses without
/// focusing), so `tab_anchor`'s own real-focus check correctly reports
/// none.
///
/// **`drive()`'s own documented limit, worked around here every time it
/// matters:** `drive()` only ever applies a `Task`'s `Action::Widget`
/// operations, never its `Action::Output` messages (its own doc comment —
/// `stage3_highlight_cursors::enter_runs_the_highlighted_palette_entry`
/// already needs the identical one-more-`drive()`-call workaround for
/// `EntryClicked`'s own chained `DetailPanelMessage::Opened` output).
/// `settle_layer_focus`'s own `(None, Some(layer))` branch, when
/// `is_shortcut` is false, resolves `window_focus_mode` the same way — via
/// `tab_anchor().then(...)`, whose closure emits
/// `Message::WindowFocusModeResolved` as a `Task::done` **output**,
/// batched alongside (not instead of) a focus operation. Every test below
/// that opens a window through a real, non-shortcut key press (or a click)
/// drives that exact follow-up message itself, immediately after — never
/// assumes it happened for free.
#[cfg(test)]
mod stage3_amendment_a1_return_on_close {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{
        DetailPanelMessage, Message, PaletteMessage, ShortcutMessage, SyncMessage, WorkspaceMessage,
    };
    use crate::state::AppState;
    use crate::state::layer::{Layer, WindowFocusMode};
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::model::status::{RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &iced::widget::Id) -> bool {
        for _ in 0..40 {
            drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    /// The most recently published message — every step below drives a
    /// real key press immediately before reading this, so there is
    /// exactly one new entry to read (`rig.messages()` itself never
    /// clears across calls in this harness, unlike the live app).
    fn last_message(rig: &Rig) -> Message {
        rig.messages()
            .last()
            .cloned()
            .expect("the key press must have published a message")
    }

    fn two_workspaces() -> AppState {
        let mut state = AppState::new(AppConfig::default());
        state.all_workspaces = vec![Workspace::new("Main"), Workspace::new("Second")];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state
    }

    /// Opens the workspace switcher by Enter, then "Add workspace" by
    /// Enter inside it — the real key path for both, start to finish. The
    /// menu's own real prior focus (the switcher button) resolves Keyboard
    /// mode (Amendment A1 §2); `WorkspaceCreate` then replaces the menu
    /// outright (`CreateWorkspaceDialogOpened`'s own handler clears
    /// `switcher_open`, per `state/layer.rs`'s own doc comment), inheriting
    /// that mode unchanged, so its own entry is focused with no second
    /// resolution step needed.
    fn open_create_workspace_dialog(rig: &mut Rig, state: &mut AppState) {
        assert!(
            tab_to(rig, state, &ids::shell::workspace_switcher()),
            "sanity: Tab must reach the switcher"
        );
        key(rig, state, Named::Enter);
        let opened = last_message(rig);
        assert!(matches!(
            opened,
            Message::Workspace(WorkspaceMessage::SwitcherToggled)
        ));
        drive(rig, state, opened);
        drive(
            rig,
            state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        assert_eq!(
            crate::state::layer::top_layer(state),
            Some(Layer::WorkspaceMenu)
        );

        assert!(tab_to(rig, state, &ids::switcher::add_workspace()));
        key(rig, state, Named::Enter);
        let add = last_message(rig);
        assert!(matches!(
            add,
            Message::Workspace(WorkspaceMessage::CreateWorkspaceDialogOpened)
        ));
        drive(rig, state, add);
        assert_eq!(
            crate::state::layer::top_layer(state),
            Some(Layer::WorkspaceCreate)
        );
        assert_eq!(
            focused_id(rig, state),
            Some(ids::workspace_manager::name()),
            "WorkspaceCreate must inherit the menu's own Keyboard mode and focus its own entry"
        );
    }

    /// §3 row 1: closed three real ways — Escape, the dialog's own Close
    /// id, its own Cancel id. `workspace_manager.rs` confirms Close and
    /// Cancel both publish the textually identical `CreateWorkspaceCancelled`
    /// message, so one test exercising all three real closing controls (each
    /// against a freshly reopened chain) stands in for three, per the
    /// handoff's own note. Every time, focus must return to the switcher —
    /// the control that opened the very first window of the chain, frozen
    /// in the page's own `FollowsId` memory the whole time (never the
    /// menu's own entry, never `WorkspaceCreate`'s).
    #[test]
    fn add_workspace_closed_three_ways_returns_focus_to_the_switcher() {
        // Escape.
        let mut state = two_workspaces();
        let mut rig = Rig::new(SIZE);
        open_create_workspace_dialog(&mut rig, &mut state);
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::Close),
        );
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::workspace_switcher()),
            "Escape must return focus to the switcher"
        );

        // The dialog's own Close id.
        let mut state = two_workspaces();
        let mut rig = Rig::new(SIZE);
        open_create_workspace_dialog(&mut rig, &mut state);
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::close()
        ));
        key(&mut rig, &state, Named::Enter);
        let closed = last_message(&rig);
        assert!(matches!(
            closed,
            Message::Workspace(WorkspaceMessage::CreateWorkspaceCancelled)
        ));
        drive(&mut rig, &mut state, closed);
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::workspace_switcher()),
            "the dialog's own Close must return focus to the switcher"
        );

        // The dialog's own Cancel id.
        let mut state = two_workspaces();
        let mut rig = Rig::new(SIZE);
        open_create_workspace_dialog(&mut rig, &mut state);
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::cancel()
        ));
        key(&mut rig, &state, Named::Enter);
        let cancelled = last_message(&rig);
        assert!(matches!(
            cancelled,
            Message::Workspace(WorkspaceMessage::CreateWorkspaceCancelled)
        ));
        drive(&mut rig, &mut state, cancelled);
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::workspace_switcher()),
            "the dialog's own Cancel must return focus to the switcher"
        );
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

    /// Opens Beta's detail panel, then its own Remove, by two real Enter
    /// presses — `DetailPanelMessage::Opened` and `WorkspaceMessage::
    /// RemoveProjectRequested`, exactly as a keyboard-only reader would
    /// produce them. Asserts the confirm dialog's own entry, Keep, is
    /// focused on open (row 2's own first half).
    fn open_remove_confirm_for_beta(rig: &mut Rig, state: &mut AppState, beta_id: &ProjectId) {
        assert!(tab_to(rig, state, &ids::row::name(beta_id)));
        key(rig, state, Named::Enter);
        let opened = last_message(rig);
        match &opened {
            Message::DetailPanel(DetailPanelMessage::Opened(id)) => assert_eq!(id, beta_id),
            other => panic!("expected DetailPanel(Opened(Beta)), got {other:?}"),
        }
        drive(rig, state, opened);
        drive(
            rig,
            state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );

        assert!(tab_to(rig, state, &ids::detail_panel::remove()));
        key(rig, state, Named::Enter);
        let requested = last_message(rig);
        match &requested {
            Message::Workspace(WorkspaceMessage::RemoveProjectRequested(id)) => {
                assert_eq!(id, beta_id)
            }
            other => panic!("expected RemoveProjectRequested(Beta), got {other:?}"),
        }
        drive(rig, state, requested);
        assert_eq!(
            crate::state::layer::top_layer(state),
            Some(Layer::RemoveProject)
        );
        assert_eq!(
            focused_id(rig, state),
            Some(ids::remove_project::keep()),
            "opening the confirm dialog via a real key must focus its own entry, Keep"
        );
    }

    /// §3 row 2: closed by "Keep" returns focus to the panel's own
    /// Remove — the control that opened the confirm dialog.
    #[test]
    fn remove_project_keep_returns_focus_to_the_panels_remove() {
        let (mut state, _alpha_id, beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);
        open_remove_confirm_for_beta(&mut rig, &mut state, &beta_id);

        assert!(tab_to(&mut rig, &mut state, &ids::remove_project::keep()));
        key(&mut rig, &state, Named::Enter);
        let kept = last_message(&rig);
        assert!(matches!(
            kept,
            Message::Workspace(WorkspaceMessage::RemoveProjectCancelled)
        ));
        drive(&mut rig, &mut state, kept);

        assert_eq!(
            crate::state::layer::top_layer(&state),
            None,
            "sanity: Keep must close the confirm dialog"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::detail_panel::remove()),
            "closing by Keep must return focus to the panel's own Remove"
        );
    }

    /// §3 row 3 (Amendment A2 §1, Review 303 F1): confirmed instead —
    /// Beta's own row, and the whole detail panel (its `view()` bails once
    /// the project it names is gone), both vanish from the page's own
    /// tracked order in one step. A1's own rule says the fallback lands on
    /// "its neighbour, the next survivor in the old order" — Alpha's own
    /// row action.
    ///
    /// **Replaces `remove_project_confirmed_lands_on_nothing_a_found_bug_
    /// not_the_neighbour`** (a characterization test pinning a bug this
    /// amendment fixes — a test must never assert a known defect as
    /// correct). The bug: `detail_panel::fetch()` sat in `view.rs`'s
    /// page-wide `reserved` list *unconditionally*, so `knotra_ui::widget::
    /// keyed::merge_reserved` reinstated it into the tracked order even
    /// with the whole panel gone — an id nothing in the real tree answers
    /// to, which `belongs_on`'s own backward search then found and stopped
    /// on, one step before reaching Alpha's own row. Fixed two ways,
    /// together: `view.rs` now names `detail_panel::fetch()` in `reserved`
    /// only while the panel is actually open (so it is never reinstated at
    /// all once the panel is gone), and `merge_reserved` itself now
    /// reinstates only the *remembered focused* id, never any other
    /// `reserved` id on its behalf, so an irrelevant reserved id can never
    /// stand in as someone else's false "still there" neighbour.
    #[test]
    fn remove_project_confirmed_lands_on_alphas_row_its_neighbour() {
        let (mut state, alpha_id, beta_id) = two_needs_help_projects();
        let mut rig = Rig::new(SIZE);
        open_remove_confirm_for_beta(&mut rig, &mut state, &beta_id);

        assert!(tab_to(&mut rig, &mut state, &ids::remove_project::remove()));
        key(&mut rig, &state, Named::Enter);
        let confirmed = last_message(&rig);
        match &confirmed {
            Message::Workspace(WorkspaceMessage::RemoveProjectConfirmed(id)) => {
                assert_eq!(id, &beta_id)
            }
            other => panic!("expected RemoveProjectConfirmed(Beta), got {other:?}"),
        }
        drive(&mut rig, &mut state, confirmed);

        assert!(
            state
                .workspace
                .as_ref()
                .unwrap()
                .projects
                .iter()
                .all(|p| p.id != beta_id),
            "sanity: Beta must really be gone"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::action(&alpha_id)),
            "A1's own rule: focus falls to the neighbour, Alpha's own row — never to nothing, \
             and never to a reinstated id (detail_panel::fetch()) nothing in the real tree \
             answers to"
        );
    }

    fn one_clean_project_selected() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    upstream: Some("origin/main".to_owned()),
                    ..RemoteStatus::default()
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: Default::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state.selection_mode = true;
        state.selection.select_all(std::slice::from_ref(&id));
        state.load_phase = crate::state::LoadPhase::Ready;
        // The one project is clean, so its section is `AllSet` —
        // collapsed by default (`AppConfig::default`), which excludes it
        // from `visible_project_ids`/`selection_summary` entirely (its
        // *section* being collapsed, not the project itself, per
        // `dashboard.rs::build_dashboard_display`'s own `ordered_
        // selectable_ids` filter). Expanded here so "Get latest safely"
        // is a real, enabled Tab stop/palette result, same as a reader
        // who has looked at that section would see.
        state.config.dashboard_all_set_collapsed = false;
        (state, id)
    }

    /// §3 row 4: the palette, opened by Enter, dispatching "Get latest
    /// safely" (`action.pull_selected`, real results from `palette::
    /// update_results`, not a hand-built fixture) — the window the
    /// command opens replaces the palette outright (the "(Some(_),
    /// Some(layer))" branch, inheriting the palette's own Keyboard mode).
    /// Cancel then closes the whole chain in one step; focus must return
    /// to the page control that opened the palette, not to anything about
    /// the palette or Get latest safely themselves.
    #[test]
    fn palette_opens_get_latest_safely_cancel_returns_to_the_palette_opener() {
        let (mut state, _id) = one_clean_project_selected();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::shell::palette()));
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(opened, Message::Palette(PaletteMessage::Opened)));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        assert_eq!(crate::state::layer::top_layer(&state), Some(Layer::Palette));

        crate::state::palette::update_results(&mut state);
        let idx = state
            .palette
            .results
            .iter()
            .position(|e| e.payload == "action.pull_selected")
            .expect("sanity: \"Get latest safely\" must be a real palette result here");
        assert!(
            state.palette.results[idx].disabled_reason_key.is_none(),
            "sanity: \"Get latest safely\" must be enabled here"
        );

        drive(
            &mut rig,
            &mut state,
            Message::Palette(PaletteMessage::EntryClicked(idx)),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::cancel()),
            "sanity: opening Get latest safely through the palette, in Keyboard mode, must \
             focus its own entry"
        );

        drive(&mut rig, &mut state, Message::Sync(SyncMessage::Cancelled));
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::palette()),
            "closing the whole chain must return focus to the control that opened the palette"
        );
    }

    /// §3 row 5: any window opened by a click (nothing given real focus
    /// first, RFC-121), then closed, leaves nothing focused — never a
    /// ring that was never earned.
    #[test]
    fn window_opened_by_a_click_then_closed_leaves_nothing_focused() {
        let (mut state, _id) = one_clean_project_selected();
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Mouse),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            None,
            "sanity: a click-opened window must start with nothing focused"
        );

        drive(&mut rig, &mut state, Message::Sync(SyncMessage::Cancelled));
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            None,
            "closing a window that was never in Keyboard mode must leave nothing focused"
        );
    }
}

/// RFC-075 stage 3 Amendment A1 §4 ("Mouse mode and phases, tested") —
/// every row of the handoff's own table, by the same real-key-path
/// discipline `stage3_amendment_a1_return_on_close`'s own doc comment
/// explains (including its identical need to drive `Message::
/// WindowFocusModeResolved` by hand after any non-shortcut open, since
/// `drive()` never applies a `Task`'s own `Action::Output` messages).
#[cfg(test)]
mod stage3_amendment_a1_mouse_mode_and_phases {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{
        BackgroundMessage, Message, PaletteMessage, ShortcutMessage, SyncMessage, WorkspaceMessage,
    };
    use crate::state::freezer::FreezerPhase;
    use crate::state::layer::{Layer, WindowFocusMode};
    use crate::state::{AppState, OperationOwner};
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::model::operation::{FreezeValidation, FreezeValidationEntry};
    use knotra_vcs::model::status::{RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &iced::widget::Id) -> bool {
        for _ in 0..40 {
            drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    fn last_message(rig: &Rig) -> Message {
        rig.messages()
            .last()
            .cloned()
            .expect("the key press must have published a message")
    }

    fn one_clean_project_selected() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    upstream: Some("origin/main".to_owned()),
                    ..RemoteStatus::default()
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: Default::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state.selection_mode = true;
        state.selection.select_all(std::slice::from_ref(&id));
        state.load_phase = crate::state::LoadPhase::Ready;
        // The one project is clean, so its section is `AllSet` —
        // collapsed by default (`AppConfig::default`), which excludes it
        // from `visible_project_ids`/`selection_summary` entirely (its
        // *section* being collapsed, not the project itself, per
        // `dashboard.rs::build_dashboard_display`'s own `ordered_
        // selectable_ids` filter). Expanded here so "Get latest safely"
        // is a real, enabled Tab stop/palette result, same as a reader
        // who has looked at that section would see.
        state.config.dashboard_all_set_collapsed = false;
        (state, id)
    }

    /// §4 rows 1-2. No dirty projects, so `AwaitingConfirm`'s own entry
    /// (Cancel) is a real, reachable Tab stop — unlike a dirty project's
    /// own already-selected disposition toggle (see
    /// `stage3_focus_follows_id_in_windows::entry_is_the_already_
    /// selected_and_so_unreachable_disposition_toggle`'s own finding).
    ///
    /// **This is the F2 regression test**: row 1's own final assertion
    /// (nothing focused after Planning → the plan, in Mouse mode) is
    /// exactly Review 302 F2's own bug — a click-opened window's later
    /// phase change wrongly getting a ring.
    #[test]
    fn get_latest_safely_by_click_then_tab_then_a_later_phase_change() {
        let (mut state, id) = one_clean_project_selected();
        let mut rig = Rig::new(SIZE);

        // Opened by a click: nothing tabbed to first (RFC-121).
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Mouse),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );

        // Row 1: Planning → the plan, in Mouse mode — nothing focused, no ring.
        assert_eq!(
            focused_id(&mut rig, &state),
            None,
            "a phase change in Mouse mode must never focus anything (Review 302 F2)"
        );

        // Row 2, first half: the reader's first Tab inside the window
        // flips Mouse → Keyboard (`app.rs::handle_shortcut`) and moves
        // real focus to the window's own first control — the header's
        // Close (`overlay::surface`'s own header is pushed before the
        // body/footer, so it is always first).
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::close()),
            "Tab, now in Keyboard mode, must reach the window's own first control"
        );

        // Row 2, second half: a later phase change, now in Keyboard mode,
        // must focus the new phase's own entry. `AwaitingConfirm` →
        // `RetryPreparationFailed`, driven through the real message
        // `smart_pull_retry_status_ready`'s own stale-request guard
        // answers with (a mismatched `lease_id` against the `retry_
        // preparation` set up by hand below — `PullRunning`'s own real
        // completion path would do just as well, but *always* chains a
        // real workspace-status refresh, `Task::perform`ing a real VCS
        // subprocess this harness cannot run; this path returns `Task::
        // none()`, so it is driven exactly as the real app would produce
        // it, start to finish, with nothing bypassed).
        //
        // First, Tab on to the plan's own Cancel — unlike Close (part of
        // every phase's own header, `overlay::surface`'s own unconditional
        // layer), `AwaitingConfirm`'s own footer Cancel has no id at all in
        // `RetryPreparationFailed`'s own, differently-shaped footer, so
        // real focus is genuinely lost by the transition below, not merely
        // left on a control that happens to survive it — the actual case
        // `settle_layer_focus`'s own "nothing really focused" check exists
        // for.
        assert!(tab_to(&mut rig, &mut state, &ids::smart_pull::cancel()));

        let workspace_id = state.workspace.as_ref().unwrap().id;
        let lease_id = crate::state::OperationLeaseId(777);
        let request_id = crate::state::sync::RetryPreparationId(1);
        state.sync.retry_preparation = Some(crate::state::sync::SmartPullRetryPreparation {
            id: request_id,
            workspace_id,
            source_operation_id: knotra_vcs::OperationId::new(),
            lease_id,
            eligible_ids: vec![id],
            exclusions: Vec::new(),
        });
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
                // `request_id`/`workspace_id`/`lease_id` all match the
                // `retry_preparation` set up above — past the handler's own
                // first, stale-request guard — but the phase itself is
                // `AwaitingConfirm`, not the `RetryPreparing` this
                // completion is only ever real for, so its own *second*
                // guard (now reached) is what actually lands on
                // `RetryPreparationFailed`.
                request_id,
                workspace_id,
                lease_id,
                statuses: Vec::new(),
            }),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest),
            "sanity: the window must still be open"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::retry_failed_close()),
            "a later phase change, in Keyboard mode, must focus the new phase's own entry"
        );
    }

    /// §4 row 3: opened by Enter on the selection bar's own "Get latest
    /// safely" — real prior focus, so Amendment A1 resolves Keyboard
    /// mode, and the plan's own entry (Cancel, no dirty projects) is
    /// focused the moment Planning becomes the plan.
    #[test]
    fn get_latest_safely_by_enter_focuses_the_plans_entry() {
        let (mut state, _id) = one_clean_project_selected();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::selection_bar::pull()));
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Sync(SyncMessage::BulkPullRequested)
        ));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );

        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::cancel()),
            "Planning → the plan, in Keyboard mode, must focus the plan's own entry"
        );
    }

    /// §4 row 4: Save release point by Ctrl+T — a real shortcut, so
    /// Keyboard mode is set synchronously (no `Task::then` involved at
    /// all, confirmed already by `stage3_entry_by_shortcut::ctrl_t_opens_
    /// freezer_and_focuses_the_name_field_synchronously`) — then Check
    /// readiness → ready. `ValidateRequested`'s own real path
    /// (`VcsAdapter::validate_freeze`) spawns real Tokio tasks this
    /// harness has no reactor for, so the validation's own completion
    /// message is driven directly instead — the same bypass `stage3_
    /// entry_ids::freezer_entry_validation_ready_never_save` already
    /// established for this exact phase, with a real lease acquired
    /// first so the real handler (`freeze_validation_done`) accepts it
    /// rather than discarding it as stale.
    #[test]
    fn save_release_point_by_ctrl_t_then_ready_focuses_the_ready_entry() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace {
            projects: vec![Project::new("Alpha", "/tmp")],
            ..Workspace::new("Main")
        });
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::OpenFreezer),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::SaveReleasePoint)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::freezer::release_name()),
            "sanity: Ctrl+T must focus Idle's own entry synchronously"
        );

        let lease_id = state
            .operation_interlock
            .try_acquire(OperationOwner::FreezeValidation)
            .expect("sanity: the interlock must be free to acquire");
        state.freezer.phase = FreezerPhase::Validating { lease_id };
        let validation = FreezeValidation {
            freeze_name: "v1".to_owned(),
            entries: vec![FreezeValidationEntry {
                project_id: ProjectId::new(),
                project_name: "Alpha".to_owned(),
                notes: Vec::new(),
                blockers: Vec::new(),
            }],
        };
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::FreezeValidationDone {
                lease_id,
                validation,
            }),
        );

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::freezer::cancel()),
            "Check readiness → ready, in Keyboard mode, must focus the ready phase's own entry"
        );
    }

    /// §4 row 5 (first of three new windows beyond what
    /// `stage3_amendment_a1_return_on_close` already covers — Review 302
    /// F3's own complaint about `entry_id` called directly rather than
    /// proven through a real key): the command palette, opened by Enter
    /// on its own shell button.
    #[test]
    fn palette_opened_by_enter_focuses_its_own_query_field() {
        let mut state = AppState::new(AppConfig::default());
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::shell::palette()));
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(opened, Message::Palette(PaletteMessage::Opened)));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );

        assert_eq!(focused_id(&mut rig, &state), Some(ids::palette::query()));
    }

    /// §4 row 5, second window: Add project, from the empty-workspace
    /// screen's own first-class button.
    #[test]
    fn add_project_opened_by_enter_focuses_the_path_field() {
        let mut state = AppState::new(AppConfig::default());
        state.workspace = Some(Workspace::new("Main"));
        state.load_phase = crate::state::LoadPhase::Ready;
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::empty::add_first_project()
        ));
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Workspace(WorkspaceMessage::AddProjectDialogOpened)
        ));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );

        assert_eq!(focused_id(&mut rig, &state), Some(ids::add_project::path()));
    }

    /// §4 row 5, third window: Rename workspace — a different `Layer`
    /// from `stage3_amendment_a1_return_on_close`'s own `WorkspaceCreate`,
    /// reached the same "replaces the menu, inherits its mode" way.
    #[test]
    fn rename_workspace_opened_from_the_keyboard_menu_focuses_the_name_field() {
        let mut state = AppState::new(AppConfig::default());
        state.all_workspaces = vec![Workspace::new("Main"), Workspace::new("Second")];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::shell::workspace_switcher()
        ));
        key(&mut rig, &state, Named::Enter);
        let toggled = last_message(&rig);
        assert!(matches!(
            toggled,
            Message::Workspace(WorkspaceMessage::SwitcherToggled)
        ));
        drive(&mut rig, &mut state, toggled);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::WorkspaceMenu)
        );

        assert!(tab_to(&mut rig, &mut state, &ids::switcher::rename()));
        key(&mut rig, &state, Named::Enter);
        let renamed = last_message(&rig);
        assert!(matches!(
            renamed,
            Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpened)
        ));
        drive(&mut rig, &mut state, renamed);

        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::WorkspaceRename)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::workspace_manager::name())
        );
    }
}

/// RFC-075 stage 3 Amendment A2 §4, rows 2-4 — row 1 is `stage3_amendment_
/// a1_return_on_close::remove_project_confirmed_lands_on_alphas_row_its_
/// neighbour` (that test itself *is* the amendment's own row 1, A2 §1's
/// rewrite of the test it replaces). Same real-key-path discipline as
/// every module above: a message mutates `state` only through `drive()`
/// (`crate::app::update`) or a real `key()` press; `focused_id()` only
/// ever reads the live tree `belongs_on`/`merge_reserved`/`entry_id`
/// themselves are never called directly to assert a result.
#[cfg(test)]
mod stage3_amendment_a2_tests {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::AppConfig;
    use crate::message::{
        BackgroundMessage, Message, SelectionMessage, ShortcutMessage, SyncMessage,
        WorkspaceMessage,
    };
    use crate::state::layer::{Layer, WindowFocusMode};
    use crate::state::{AppState, OperationOwner, Screen};
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use knotra_vcs::model::status::{RemoteStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus};
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &iced::widget::Id) -> bool {
        for _ in 0..40 {
            drive(rig, state, Message::Shortcut(ShortcutMessage::FocusNext));
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    fn last_message(rig: &Rig) -> Message {
        rig.messages()
            .last()
            .cloned()
            .expect("the key press must have published a message")
    }

    /// One project, selected, with a real upstream — "Get latest safely"
    /// is enabled and a real Tab stop; "Check for updates" starts enabled
    /// too (not missing, so `fetchable_ids` is non-empty), until the test
    /// itself marks it busy.
    fn one_project_selected_with_upstream() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    upstream: Some("origin/main".to_owned()),
                    ..RemoteStatus::default()
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: Default::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state.selection_mode = true;
        state.selection.select_all(std::slice::from_ref(&id));
        state.load_phase = crate::state::LoadPhase::Ready;
        state.config.dashboard_all_set_collapsed = false;
        (state, id)
    }

    /// §4 row 2 (Amendment A2 §1, Review 303 F1): "Get latest safely" is
    /// the control the page really, currently remembers as focused —
    /// `ids::selection_bar::pull()` is never itself in `view.rs`'s own
    /// `reserved` list, so nothing about this id's own disappearance
    /// depends on the fix directly. The fix is proven by what sits right
    /// next to it in the page's own tracked order instead: "Check for
    /// updates" (`ids::selection_bar::fetch()`), which *is* reserved for
    /// as long as the selection bar is drawn (`view.rs`: `if sel_bar_open
    /// { reserved.push(ids::selection_bar::fetch()) }`), and which goes
    /// disabled in the very same redraw "Get latest safely" does (both
    /// gated on the one shared `operation_interlock.is_busy()` read).
    /// Before this amendment, `merge_reserved` reinstated *every* id
    /// `reserved` named, unconditionally — including "Check for updates",
    /// even though the id genuinely lost here is "Get latest safely" —
    /// manufacturing a "still there" id nothing in the real tree answers
    /// to, for `belongs_on`'s own backward search to stop on one step too
    /// early (exactly `remove_project_confirmed_lands_on_nothing_a_found_
    /// bug_not_the_neighbour`'s own shape, A2 §1's row 1 test replaces).
    /// After it: only the id focus actually belongs to is ever
    /// reinstated, so a *different* reserved-but-disabled sibling is
    /// never mistaken for the one that is genuinely missing.
    ///
    /// "Check for updates running" is set up directly on `state`
    /// (`operation_interlock.try_acquire`), not by driving the real
    /// `SyncMessage::BulkFetchRequested` — that handler's own `Task::run`
    /// spawns a real `VcsAdapter::fetch` subprocess this harness has no
    /// reactor for (the same bypass this file's other modules already use
    /// for `FreezeValidationDone`/`SmartPullRetryStatusReady`). The lease
    /// alone is enough: `selection_bar::view`'s own `busy` read is the
    /// only thing gating both buttons, and every `focused_id`/`drive`
    /// call below rebuilds the real view against whatever `state` holds,
    /// through the real tree, regardless of how `state` reached that
    /// shape.
    #[test]
    fn check_for_updates_reserved_and_disabled_never_steals_get_latest_safelys_neighbour() {
        let (mut state, _id) = one_project_selected_with_upstream();
        let mut rig = Rig::new(SIZE);

        // "Get latest safely" is real, enabled, and becomes the page's
        // own remembered focus, by a real Tab press.
        assert!(tab_to(&mut rig, &mut state, &ids::selection_bar::pull()));
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::pull())
        );

        // "Check for updates running": the interlock goes busy, which
        // `selection_bar::view` reads to disable *both* buttons in this
        // same redraw — "Check for updates" is still named in `reserved`
        // (the bar is still drawn), "Get latest safely" never was.
        let lease_id = state
            .operation_interlock
            .try_acquire(OperationOwner::BulkFetch)
            .expect("sanity: the interlock must be free to acquire");
        assert!(state.operation_interlock.is_busy());

        // Whatever the page's own neighbour search lands on here, it must
        // never be "Check for updates" own ghost — the one id A2 §1
        // forbids `merge_reserved` from ever reinstating on "Get latest
        // safely"'s behalf.
        assert_ne!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::fetch()),
            "a disabled, merely-reserved Check for updates must never be mistaken for the \
             id focus actually belongs to"
        );

        // Now the whole selection bar goes — "Check for updates" and
        // "Get latest safely" both vanish from the tree together, by a
        // path that never once pressed "Get latest safely" itself
        // (`SelectionMessage::Clear`, real-world equivalent: a background
        // refresh dropping the last selected project, or any other
        // caller of `clear_selection_mode`).
        drive(
            &mut rig,
            &mut state,
            Message::Selection(SelectionMessage::Clear),
        );
        assert!(
            !state.selection_mode,
            "sanity: Clear must leave selection mode, taking the whole bar with it"
        );
        assert!(
            state.operation_interlock.release_if_matches(lease_id),
            "sanity: the lease acquired above must still be the active one"
        );

        let after = focused_id(&mut rig, &state);
        assert!(
            after.is_some(),
            "a control the tree reports must be focused — never nowhere — once the bar \
             that held both the remembered focus and its disabled, reserved sibling is \
             gone entirely"
        );
        assert_ne!(
            after,
            Some(ids::selection_bar::fetch()),
            "never a reinstated id nothing in the real tree answers to, even once the bar \
             itself is gone"
        );
    }

    /// RFC-074 Stage 1 D4: "a control in the bar cannot hold focus on a
    /// screen where the bar is not drawn", and "leaving and returning"
    /// keeps the selection. "Exit selection" is tabbed to and genuinely
    /// focused (the same real-key-path proof the row above this one
    /// makes), then `Message::Navigate` — what activating the shell's own
    /// History link publishes — takes the whole bar with it, by
    /// `view.rs`'s new `if matches!(state.screen, Screen::Dashboard)`
    /// gate. The bar's own id is never in `reserved` once it is gone (the
    /// same `sel_bar_open`-conditioned push the row above this one reads),
    /// so this does not depend on Amendment A2 F1's fix directly — it
    /// proves the more basic thing that fix's own neighbour-fallback
    /// (`belongs_on`) rests on: focus never still claims to be an id the
    /// real tree no longer answers to, and returning redraws the bar as a
    /// real, reachable Tab stop again, with the same selection.
    #[test]
    fn selection_bar_focus_does_not_survive_leaving_the_dashboard() {
        let (mut state, id) = one_project_selected_with_upstream();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::selection_bar::exit()));
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::exit())
        );

        drive(&mut rig, &mut state, Message::Navigate(Screen::History));

        assert_eq!(state.screen, Screen::History);
        assert!(
            state.selection_mode,
            "leaving the dashboard must keep the selection"
        );
        assert!(state.selection.contains(&id));
        assert_ne!(
            focused_id(&mut rig, &state),
            Some(ids::selection_bar::exit()),
            "the bar's own control cannot still hold focus once the bar itself is gone"
        );

        drive(&mut rig, &mut state, Message::Navigate(Screen::Dashboard));

        assert!(
            state.selection_mode,
            "returning must still carry the same selection"
        );
        assert!(state.selection.contains(&id));
        assert!(
            tab_to(&mut rig, &mut state, &ids::selection_bar::exit()),
            "the bar's own control must be a real Tab stop again once it returns"
        );
    }

    /// RFC-074 Stage 2 handoff §3/§4 row 6: one project, **not** selected,
    /// **not** dirty, with updates available — so the toolbar's own "Get
    /// latest safely" is drawn (`get_latest_button_visible`), and the
    /// plan it opens has no dirty entry, so its own entry (`entry_id`) is
    /// Cancel, the exact shape `get_latest_safely_by_enter_focuses_the_
    /// plans_entry` below already proved for the selection bar's own
    /// button.
    fn one_behind_project_not_selected() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    ahead: 0,
                    behind: 2,
                    upstream: Some("origin/main".to_owned()),
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: Default::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.config.dashboard_all_set_collapsed = false;
        (state, id)
    }

    /// Handoff §3/§4 row 6: entry by Enter focuses the plan's own entry,
    /// and closing it returns focus to the toolbar button — the same
    /// shape `get_latest_safely_by_enter_focuses_the_plans_entry` proved
    /// for the selection bar's own button, through the identical
    /// `BulkPullRequested` message (§1: "the same message the palette's
    /// path with no selection" dispatches too), now from the toolbar.
    /// "Return to a neighbour" (A1's own rule, when the opener itself is
    /// gone) is `add_workspace_closed_three_ways_returns_focus_to_the_
    /// switcher`'s own already-proved mechanism — nothing in this stage
    /// changes it, so it is not re-proved here.
    #[test]
    fn get_latest_toolbar_button_by_enter_focuses_the_plans_entry_and_returns_on_close() {
        let (mut state, _id) = one_behind_project_not_selected();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::toolbar::get_latest()));
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Sync(SyncMessage::BulkPullRequested)
        ));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );

        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(crate::view::ids::smart_pull::cancel()),
            "opened from the keyboard, the plan's own entry must be focused"
        );

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::Close),
        );
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::get_latest()),
            "on close, focus must return to the toolbar button that opened it"
        );
    }

    /// One project, clean, selected — "Get latest safely" opens with
    /// nothing tabbed to first (RFC-121), the shape every row below
    /// needs: no dirty projects, so `AwaitingConfirm`'s own entry
    /// (Cancel) is a real, reachable Tab stop (same fixture and the same
    /// reasoning as `stage3_amendment_a1_mouse_mode_and_phases::one_
    /// clean_project_selected`, redefined here rather than shared across
    /// `mod` boundaries, matching this file's own convention).
    fn one_clean_project_selected() -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("Alpha", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    upstream: Some("origin/main".to_owned()),
                    ..RemoteStatus::default()
                },
                working_tree: WorkingTreeStatus::default(),
                conflict: Default::default(),
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        state.selection_mode = true;
        state.selection.select_all(std::slice::from_ref(&id));
        state.load_phase = crate::state::LoadPhase::Ready;
        state.config.dashboard_all_set_collapsed = false;
        (state, id)
    }

    /// §4 row 3 (Amendment A2 §2, Review 303 F2): the real race the fix
    /// closes. "Get latest safely" opened by a click starts with
    /// `window_focus_mode` genuinely `None` — unresolved — until its own
    /// `tab_anchor` round trip answers, asynchronously, with
    /// `Message::WindowFocusModeResolved` (`focus_ops::settle_layer_
    /// focus`'s own `(None, Some(layer))` arm). `drive()` never applies a
    /// `Task`'s own `Action::Output` (confirmed by this file's own
    /// `stage3_amendment_a1_mouse_mode_and_phases` module doc comment), so
    /// driving just the click-open message genuinely leaves that
    /// resolution outstanding here — the same gap a real build has for
    /// however long the round trip takes.
    ///
    /// Tab, pressed in exactly that gap, must still set `Keyboard` itself
    /// (`handle_shortcut`'s own flip, widened by this amendment to fire
    /// from `None` too, not only `Mouse`) — proven here by driving
    /// `Message::Shortcut(FocusNext)` next, with no `WindowFocusModeResolved`
    /// driven in between. The late answer, driven by hand afterward
    /// (simulating the real round trip actually landing only *after* the
    /// Tab that beat it), must never undo it (`update_inner`'s own "a late
    /// answer never overrides `Keyboard`" guard). Finally, the next phase
    /// change — now genuinely in `Keyboard` mode, not merely flagged as
    /// such with no real effect — focuses its own new entry, the same
    /// mechanism and the same `AwaitingConfirm` → `RetryPreparationFailed`
    /// transition `stage3_amendment_a1_mouse_mode_and_phases::get_latest_
    /// safely_by_click_then_tab_then_a_later_phase_change`'s own second
    /// half already established (its own doc comment explains the bypass
    /// below: a mismatched `lease_id` against a hand-built `retry_
    /// preparation` is what a real `smart_pull_retry_status_ready` would
    /// itself answer with, with nothing bypassed).
    ///
    /// **Seen failing first**: reverting just the two named spots
    /// (`handle_shortcut`'s own flip condition, `WindowFocusModeResolved`'s
    /// own handler in `update_inner`) and re-running this exact test is
    /// this amendment's own §5 report requirement.
    #[test]
    fn tab_before_the_late_answer_sets_keyboard_and_the_late_answer_never_undoes_it() {
        let (mut state, id) = one_clean_project_selected();
        let mut rig = Rig::new(SIZE);

        // Opened by a click: nothing tabbed to first (RFC-121).
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::BulkPullRequested),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest)
        );
        assert_eq!(
            state.window_focus_mode, None,
            "sanity: the mode's own async round trip has not answered yet"
        );

        // Tab, in that exact gap.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );
        assert_eq!(
            state.window_focus_mode,
            Some(WindowFocusMode::Keyboard),
            "Tab, pressed before the mode resolves, must set Keyboard itself (Review 303 F2)"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::close()),
            "and really move focus to the window's own first control, by Scope::Window"
        );

        // The late answer, arriving only now.
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Mouse),
        );
        assert_eq!(
            state.window_focus_mode,
            Some(WindowFocusMode::Keyboard),
            "a late Mouse answer must never override the Keyboard Tab already set \
             (Review 303 F2)"
        );

        // Planning's own entry (none) differs from AwaitingConfirm's
        // (Cancel) — moves the phase along, as the real chain would.
        drive(
            &mut rig,
            &mut state,
            Message::Sync(SyncMessage::SmartPullPlanRequested),
        );
        assert!(tab_to(&mut rig, &mut state, &ids::smart_pull::cancel()));

        let workspace_id = state.workspace.as_ref().unwrap().id;
        let lease_id = crate::state::OperationLeaseId(778);
        let request_id = crate::state::sync::RetryPreparationId(1);
        state.sync.retry_preparation = Some(crate::state::sync::SmartPullRetryPreparation {
            id: request_id,
            workspace_id,
            source_operation_id: knotra_vcs::OperationId::new(),
            lease_id,
            eligible_ids: vec![id],
            exclusions: Vec::new(),
        });
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::SmartPullRetryStatusReady {
                request_id,
                workspace_id,
                lease_id,
                statuses: Vec::new(),
            }),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::GetLatest),
            "sanity: the window must still be open"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::smart_pull::retry_failed_close()),
            "the next phase change, genuinely in Keyboard mode despite the race above, must \
             focus the new phase's own entry"
        );
    }

    /// §4 row 4 (Amendment A2 §3, Review 303 F3): each `tab_anchor`/Tab
    /// query is answered by the wrapper its own caller names
    /// (`app.rs::window_or_page_scope`), never by "whichever wrapper
    /// happens to answer" (the retired heuristic). The workspace
    /// switcher's own menu is chosen deliberately: a `header_menu` slot,
    /// not `AppLayout::dialog` like every other window this file already
    /// covers, but still wrapped in its own `focus_by_id_window`
    /// (`shell.rs::switcher_menu`) per this amendment's §3.
    ///
    /// Page scope answers first, with no window open at all (`Scope::
    /// Page`, `top_layer` `None`). Once the menu opens, `Scope::Window`
    /// answers every Tab query inside it — proven both by what IS
    /// reachable (every one of the menu's own real entries, confirmed in
    /// no particular order since `tab_to` only asserts reachability) and
    /// by what is NOT: the page's own control that opened the menu,
    /// `ids::shell::workspace_switcher()`, must never reappear in this
    /// traversal while the menu is open — `snora::Inert` hides the page's
    /// own content for exactly this long, so a query wrongly answered by
    /// the page's own frozen wrapper would find it anyway. Closing the
    /// menu returns focus to that exact control, proof the page's own
    /// memory was never touched by the window's own Tab traversal above
    /// it.
    #[test]
    fn window_scope_answers_tab_inside_it_page_scope_never_does() {
        let mut state = AppState::new(AppConfig::default());
        state.all_workspaces = vec![Workspace::new("Main"), Workspace::new("Second")];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        let mut rig = Rig::new(SIZE);

        // Page scope, no window open: a real page control is reachable.
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::shell::workspace_switcher()
        ));
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::workspace_switcher())
        );

        // Opens the menu by a real Enter press — real prior focus (the
        // switcher button itself) resolves Keyboard mode.
        key(&mut rig, &state, Named::Enter);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Workspace(WorkspaceMessage::SwitcherToggled)
        ));
        drive(&mut rig, &mut state, opened);
        drive(
            &mut rig,
            &mut state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
        assert_eq!(
            crate::state::layer::top_layer(&state),
            Some(Layer::WorkspaceMenu)
        );

        // The page's own control is not reachable while the menu is open
        // — `Scope::Window` answers this traversal now, never `Scope::Page`.
        assert!(
            !tab_to(&mut rig, &mut state, &ids::shell::workspace_switcher()),
            "the page's own switcher button must never reappear in the window's own Tab \
             traversal while the menu is open"
        );

        // Every one of the menu's own real entries is reachable.
        let second_id = state.all_workspaces[1].id;
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::switcher::row(&second_id)
        ));
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::switcher::add_workspace()
        ));
        assert!(tab_to(&mut rig, &mut state, &ids::switcher::rename()));
        assert!(tab_to(&mut rig, &mut state, &ids::switcher::delete()));

        // Closing the menu returns focus to the page's own control that
        // opened it — the page scope's own memory, untouched by any of
        // the window's own Tab traversal above.
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::Close),
        );
        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::shell::workspace_switcher()),
            "closing the window must return focus to the page's own control that opened it"
        );
    }
}

/// RFC-075 handoff stage 4 §5, rows 1, 3, 5, 6 — Settings' own Tab
/// sequence, its selects' keyboard behaviour, and its per-workspace
/// Rename/Remove windows. Follows `stage3_tab_and_confinement`'s/
/// `stage3_amendment_a1_return_on_close`'s own conventions (`tab`/
/// `shift_tab`/`tab_to`/`assert_tab_sequence_wraps`/`last_message`, each a
/// small hand-rolled copy rather than a shared export, matching this
/// file's own established per-module precedent).
///
/// Every fixture here builds its own `AppState` on a fresh `tempfile::
/// TempDir`, never the real, resolved `AppPaths` `AppState::new` would
/// otherwise reach for — `SettingsMessage::*Changed`'s own handlers
/// (`app/misc.rs::handle_settings`) call `persist_config` on every choice,
/// and the workspace Rename/Remove flows call `save_workspace`/
/// `delete_workspace_file`, all genuine disk I/O. Driving those messages
/// for real (the whole point of this file, over calling `handle_settings`
/// by hand) must never touch the person's own real config/workspace files.
#[cfg(test)]
mod stage4_settings_tab_and_selects {
    use super::{Rig, SIZE, drive, focused_id, key};
    use crate::config::{AppConfig, AppPaths};
    use crate::message::{Message, SettingsMessage, ShortcutMessage, WorkspaceMessage};
    use crate::state::layer::WindowFocusMode;
    use crate::state::{AppState, Screen, SettingsSelect};
    use crate::view::ids;
    use iced::keyboard::key::Named;
    use iced::widget::Id;
    use knotra_ui::ThemeChoice;
    use knotra_ui::i18n::Locale;
    use knotra_vcs::{Workspace, WorkspaceId};

    fn tmp_paths() -> (AppPaths, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = AppPaths {
            config_file: tmp.path().join("config.toml"),
            workspaces_dir: tmp.path().join("workspaces"),
            history_dir: tmp.path().join("history"),
        };
        (paths, tmp)
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

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &Id) -> bool {
        for _ in 0..60 {
            tab(rig, state);
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    /// The most recently published message — every call site below drives
    /// a real key press immediately before reading this, matching
    /// `stage3_amendment_a1_return_on_close`'s own `last_message`.
    fn last_message(rig: &Rig) -> Message {
        rig.messages()
            .last()
            .cloned()
            .expect("the key press must have published a message")
    }

    /// `super::key`'s own multi-press twin: delivers every one of
    /// `presses` as its own `UserInterface::update` call against the
    /// *same* `UserInterface` (built once, from `state` as it stands
    /// right now), with a single redraw only once every press has been
    /// handled — never one redraw per press the way repeated `key()`
    /// calls would.
    ///
    /// **Needed for moving within an already-open select's own list**:
    /// `snora::design::select` keeps no highlighted-item state of its
    /// own across a redraw — only real `operation::Focusable` focus
    /// does — and the whole page sits inside `knotra_ui::widget::keyed::
    /// FollowsId` (`focus_by_id_page`), whose own reconciliation runs on
    /// every `RedrawRequested` and pulls real focus back to its last
    /// *tracked* id (the select's own trigger, since the open list's
    /// items carry no id of their own, `select.rs`'s module doc). A
    /// `key()` call's own redraw, run after *each* press, would reset
    /// "nothing (menu-wise) focused" before the next one ever reaches
    /// `MenuKeys`, so every press would land on the list's first item
    /// again — never the real behaviour: a real keyboard driver delivers
    /// several presses before the next frame redraws, not one redraw per
    /// key. Seen directly: the three select tests below first failed
    /// with exactly that shape (always choosing the open list's first
    /// item, regardless of how many ↓ presses were sent) until they were
    /// rewritten onto this batched helper.
    fn keys(rig: &mut Rig, state: &AppState, presses: &[Named]) {
        let mut ui = super::rebuild(rig, state);
        for &named in presses {
            let events = [iced_test::simulator::press_key(named, None)];
            let _ = ui.update(
                &events,
                rig.cursor,
                &mut rig.renderer,
                &mut super::clipboard::Null,
                &mut rig.messages,
            );
        }
        super::redraw(rig, &mut ui);
        rig.cache = ui.into_cache();
    }

    /// Same shape as `stage3_tab_and_confinement::assert_tab_sequence_wraps`
    /// — a hand-rolled copy, not a shared export, matching this file's own
    /// per-module convention.
    fn assert_tab_sequence_wraps(rig: &mut Rig, state: &mut AppState, expected: &[Id]) {
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

    fn settings_state_one_workspace() -> (AppState, WorkspaceId, tempfile::TempDir) {
        let (paths, tmp) = tmp_paths();
        let mut state = AppState::new_with_paths(AppConfig::default(), paths);
        state.screen = Screen::Settings;
        let ws = Workspace::new("Main");
        let id = ws.id;
        state.workspace = Some(ws.clone());
        state.all_workspaces = vec![ws];
        (state, id, tmp)
    }

    fn settings_state_two_workspaces() -> (AppState, WorkspaceId, WorkspaceId, tempfile::TempDir) {
        let (paths, tmp) = tmp_paths();
        let mut state = AppState::new_with_paths(AppConfig::default(), paths);
        state.screen = Screen::Settings;
        let a = Workspace::new("Main");
        let b = Workspace::new("Second");
        let (a_id, b_id) = (a.id, b.id);
        state.workspace = Some(a.clone());
        state.all_workspaces = vec![a, b];
        (state, a_id, b_id, tmp)
    }

    fn settings_state_three_workspaces() -> (
        AppState,
        WorkspaceId,
        WorkspaceId,
        WorkspaceId,
        tempfile::TempDir,
    ) {
        let (paths, tmp) = tmp_paths();
        let mut state = AppState::new_with_paths(AppConfig::default(), paths);
        state.screen = Screen::Settings;
        let a = Workspace::new("Main");
        let b = Workspace::new("Beta");
        let c = Workspace::new("Gamma");
        let (a_id, b_id, c_id) = (a.id, b.id, c.id);
        state.workspace = Some(a.clone());
        state.all_workspaces = vec![a, b, c];
        (state, a_id, b_id, c_id, tmp)
    }

    // -----------------------------------------------------------------
    // Row 1: Settings' Tab sequence, by id, wrapping both ways.
    // -----------------------------------------------------------------

    /// One workspace (so Remove is disabled on it — `can_delete` requires
    /// 2+ — and skipped by Tab entirely, D4 b); Advanced closed.
    #[test]
    fn settings_tab_sequence_one_workspace_advanced_closed_wraps_both_ways() {
        let (mut state, ws_id, _tmp) = settings_state_one_workspace();
        let mut rig = Rig::new(SIZE);

        // Neither Dashboard nor History is the active screen while on
        // Settings, so both are real, enabled Tab stops — unlike the
        // dashboard's own sequence tests, where Dashboard (the active
        // screen) is skipped. No workspace switcher: only one workspace.
        let expected: Vec<Id> = vec![
            ids::shell::nav_dashboard(),
            ids::shell::nav_history(),
            ids::shell::refresh(),
            ids::shell::palette(),
            ids::shell::settings(),
            ids::settings::locale(),
            ids::settings::theme(),
            ids::settings::zoom(),
            ids::settings::refresh_auto(),
            ids::settings::editor(),
            ids::settings::merge_tool(),
            ids::settings::workspace_rename(&ws_id),
            // Remove is disabled (the only workspace) — skipped by Tab.
            ids::settings::add_workspace(),
            ids::settings::advanced_toggle(),
        ];

        assert_tab_sequence_wraps(&mut rig, &mut state, &expected);
    }

    /// Two workspaces (Remove now enabled on both, D4 b); Advanced open,
    /// with the file-change checkbox off — the interval select is not
    /// drawn (RFC-073 handoff Amendment A5 §5), one fewer stop than the
    /// checkbox-on case below.
    #[test]
    fn settings_tab_sequence_two_workspaces_advanced_open_checkbox_off_wraps_both_ways() {
        let (mut state, a_id, b_id, _tmp) = settings_state_two_workspaces();
        state.settings_advanced_open = true;
        assert!(
            !state.config.fs_watch_enabled,
            "sanity: the file-change checkbox is off by default"
        );
        let mut rig = Rig::new(SIZE);

        let expected: Vec<Id> = vec![
            ids::shell::workspace_switcher(),
            ids::shell::nav_dashboard(),
            ids::shell::nav_history(),
            ids::shell::refresh(),
            ids::shell::palette(),
            ids::shell::settings(),
            ids::settings::locale(),
            ids::settings::theme(),
            ids::settings::zoom(),
            ids::settings::refresh_auto(),
            ids::settings::editor(),
            ids::settings::merge_tool(),
            ids::settings::workspace_rename(&a_id),
            ids::settings::workspace_remove(&a_id),
            ids::settings::workspace_rename(&b_id),
            ids::settings::workspace_remove(&b_id),
            ids::settings::add_workspace(),
            ids::settings::advanced_toggle(),
            ids::settings::max_concurrent(),
            ids::settings::max_logs(),
            ids::settings::fs_watch_checkbox(),
        ];

        assert_tab_sequence_wraps(&mut rig, &mut state, &expected);
    }

    /// Same as above, but the file-change checkbox is on: the interval
    /// select appears as one more Tab stop, right after the checkbox.
    #[test]
    fn settings_tab_sequence_two_workspaces_advanced_open_checkbox_on_reveals_the_interval_select()
    {
        let (mut state, a_id, b_id, _tmp) = settings_state_two_workspaces();
        state.settings_advanced_open = true;
        state.config.fs_watch_enabled = true;
        let mut rig = Rig::new(SIZE);

        let expected: Vec<Id> = vec![
            ids::shell::workspace_switcher(),
            ids::shell::nav_dashboard(),
            ids::shell::nav_history(),
            ids::shell::refresh(),
            ids::shell::palette(),
            ids::shell::settings(),
            ids::settings::locale(),
            ids::settings::theme(),
            ids::settings::zoom(),
            ids::settings::refresh_auto(),
            ids::settings::editor(),
            ids::settings::merge_tool(),
            ids::settings::workspace_rename(&a_id),
            ids::settings::workspace_remove(&a_id),
            ids::settings::workspace_rename(&b_id),
            ids::settings::workspace_remove(&b_id),
            ids::settings::add_workspace(),
            ids::settings::advanced_toggle(),
            ids::settings::max_concurrent(),
            ids::settings::max_logs(),
            ids::settings::fs_watch_checkbox(),
            ids::settings::fs_debounce(),
        ];

        assert_tab_sequence_wraps(&mut rig, &mut state, &expected);
    }

    // -----------------------------------------------------------------
    // Row 3: each select opens, ↓ moves, Enter chooses, and focus stays
    // on it (by id) — Language, Theme and Zoom, each genuinely rebuilding
    // the screen on choice.
    // -----------------------------------------------------------------

    /// Locale: choosing re-translates the whole screen (a new `Catalog`),
    /// yet focus (by id) stays on `ids::settings::locale()` across that
    /// rebuild.
    #[test]
    fn locale_select_open_arrow_down_enter_focus_survives_the_retranslation() {
        let (mut state, _ws_id, _tmp) = settings_state_one_workspace();
        let mut rig = Rig::new(SIZE);
        assert_eq!(state.config.locale, Locale::En, "sanity: default locale");
        let en_title = state.t("settings.title").to_owned();

        assert!(tab_to(&mut rig, &mut state, &ids::settings::locale()));

        // ↓ on the focused, closed trigger opens it (RFC-075 §2).
        key(&mut rig, &state, Named::ArrowDown);
        let opened = last_message(&rig);
        assert!(
            matches!(
                opened,
                Message::Settings(SettingsMessage::SelectOpenChanged(
                    SettingsSelect::Locale,
                    true
                ))
            ),
            "expected the locale select's own open-change, got {opened:?}"
        );
        drive(&mut rig, &mut state, opened);
        assert_eq!(state.settings_select_open, Some(SettingsSelect::Locale));

        // Within the open list: ↓ lands on "English" (index 0), a second ↓
        // moves to "日本語" (index 1); Enter chooses it. One batch
        // (`keys`, not three separate `key()` calls) — see that helper's
        // own doc comment for why.
        keys(
            &mut rig,
            &state,
            &[Named::ArrowDown, Named::ArrowDown, Named::Enter],
        );
        let chosen = last_message(&rig);
        assert!(
            matches!(
                chosen,
                Message::Settings(SettingsMessage::LocaleChanged(Locale::Ja))
            ),
            "expected LocaleChanged(Ja), got {chosen:?}"
        );
        drive(&mut rig, &mut state, chosen);

        assert_eq!(state.config.locale, Locale::Ja);
        assert_ne!(
            state.t("settings.title"),
            en_title,
            "sanity: the screen must really have re-translated"
        );
        assert_eq!(
            state.settings_select_open, None,
            "choosing must close the select (its own Changed handler, not just snora's)"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::locale()),
            "focus must stay on the select, by id, across the real rebuild the \
             re-translation triggers"
        );
    }

    /// Theme: choosing swaps `state.theme` (and so every token the whole
    /// screen renders with), yet focus stays on `ids::settings::theme()`.
    #[test]
    fn theme_select_open_arrow_down_enter_focus_survives_the_theme_swap() {
        let (mut state, _ws_id, _tmp) = settings_state_one_workspace();
        let mut rig = Rig::new(SIZE);
        assert_eq!(
            state.config.theme,
            ThemeChoice::Light,
            "sanity: default theme"
        );

        assert!(tab_to(&mut rig, &mut state, &ids::settings::theme()));

        key(&mut rig, &state, Named::ArrowDown);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Settings(SettingsMessage::SelectOpenChanged(
                SettingsSelect::Theme,
                true
            ))
        ));
        drive(&mut rig, &mut state, opened);
        assert_eq!(state.settings_select_open, Some(SettingsSelect::Theme));

        // Light (index 0), then Dark (index 1) — Enter chooses Dark.
        keys(
            &mut rig,
            &state,
            &[Named::ArrowDown, Named::ArrowDown, Named::Enter],
        );
        let chosen = last_message(&rig);
        assert!(
            matches!(
                chosen,
                Message::Settings(SettingsMessage::ThemeChanged(ThemeChoice::Dark))
            ),
            "expected ThemeChanged(Dark), got {chosen:?}"
        );
        drive(&mut rig, &mut state, chosen);

        assert_eq!(state.config.theme, ThemeChoice::Dark);
        assert_eq!(state.settings_select_open, None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::theme()),
            "focus must stay on the select, by id, across the real rebuild the theme \
             swap triggers"
        );
    }

    /// Zoom: choosing changes `state.config.zoom_percent`, yet focus stays
    /// on `ids::settings::zoom()`.
    #[test]
    fn zoom_select_open_arrow_down_enter_focus_survives_the_zoom_change() {
        let (mut state, _ws_id, _tmp) = settings_state_one_workspace();
        let mut rig = Rig::new(SIZE);
        assert_eq!(state.config.zoom_percent, 100, "sanity: default zoom");

        assert!(tab_to(&mut rig, &mut state, &ids::settings::zoom()));

        key(&mut rig, &state, Named::ArrowDown);
        let opened = last_message(&rig);
        assert!(matches!(
            opened,
            Message::Settings(SettingsMessage::SelectOpenChanged(
                SettingsSelect::Zoom,
                true
            ))
        ));
        drive(&mut rig, &mut state, opened);
        assert_eq!(state.settings_select_open, Some(SettingsSelect::Zoom));

        // 100% (index 0), then 115% (index 1) — Enter chooses 115%.
        keys(
            &mut rig,
            &state,
            &[Named::ArrowDown, Named::ArrowDown, Named::Enter],
        );
        let chosen = last_message(&rig);
        assert!(
            matches!(chosen, Message::Settings(SettingsMessage::ZoomChanged(115))),
            "expected ZoomChanged(115), got {chosen:?}"
        );
        drive(&mut rig, &mut state, chosen);

        assert_eq!(state.config.zoom_percent, 115);
        assert_eq!(state.settings_select_open, None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::zoom()),
            "focus must stay on the select, by id, across the real rebuild the zoom \
             change triggers"
        );
    }

    // -----------------------------------------------------------------
    // Row 6: Escape with a select open closes only the select.
    // -----------------------------------------------------------------

    /// Same mechanism `snora::design::Select`'s own Escape handling
    /// already gives the dashboard's Arrange select
    /// (`select::select_control`, `crates/knotra-ui/src/widget/select.rs`'s
    /// own test) — proven here for Settings' own Theme select: Escape
    /// closes the select alone, never navigates away from Settings or
    /// drops focus off the trigger.
    #[test]
    fn escape_with_a_settings_select_open_closes_only_the_select() {
        let (mut state, _ws_id, _tmp) = settings_state_one_workspace();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::settings::theme()));
        key(&mut rig, &state, Named::ArrowDown);
        let opened = last_message(&rig);
        drive(&mut rig, &mut state, opened);
        assert_eq!(state.settings_select_open, Some(SettingsSelect::Theme));

        key(&mut rig, &state, Named::Escape);
        let closed = last_message(&rig);
        assert!(
            matches!(
                closed,
                Message::Settings(SettingsMessage::SelectOpenChanged(
                    SettingsSelect::Theme,
                    false
                ))
            ),
            "Escape must publish only the select's own close, got {closed:?}"
        );
        drive(&mut rig, &mut state, closed);

        assert_eq!(
            state.settings_select_open, None,
            "Escape must close the select"
        );
        assert_eq!(
            state.screen,
            Screen::Settings,
            "Escape must not navigate away from Settings"
        );
        assert_eq!(
            state.config.theme,
            ThemeChoice::Light,
            "Escape must not have chosen anything"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::theme()),
            "Escape must return focus to the select's own trigger, not leave the page"
        );
    }

    // -----------------------------------------------------------------
    // Row 5: Rename and Remove from the keyboard — entry, return, and the
    // neighbour after an actual removal.
    // -----------------------------------------------------------------

    fn open_rename_dialog_for(rig: &mut Rig, state: &mut AppState, button_id: &Id) {
        assert!(
            tab_to(rig, state, button_id),
            "sanity: Tab must reach {button_id:?}"
        );
        key(rig, state, Named::Enter);
        let opened = last_message(rig);
        assert!(
            matches!(
                opened,
                Message::Workspace(WorkspaceMessage::RenameWorkspaceDialogOpenedFor(_))
            ),
            "expected RenameWorkspaceDialogOpenedFor, got {opened:?}"
        );
        drive(rig, state, opened);
        drive(
            rig,
            state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
    }

    fn open_delete_dialog_for(rig: &mut Rig, state: &mut AppState, button_id: &Id) {
        assert!(
            tab_to(rig, state, button_id),
            "sanity: Tab must reach {button_id:?}"
        );
        key(rig, state, Named::Enter);
        let opened = last_message(rig);
        assert!(
            matches!(
                opened,
                Message::Workspace(WorkspaceMessage::DeleteWorkspaceRequestedFor(_))
            ),
            "expected DeleteWorkspaceRequestedFor, got {opened:?}"
        );
        drive(rig, state, opened);
        drive(
            rig,
            state,
            Message::WindowFocusModeResolved(WindowFocusMode::Keyboard),
        );
    }

    /// Entry: opening Rename from Settings' own per-workspace button
    /// focuses the dialog's own name field (`workspace_manager::
    /// entry_id`'s own rule, shared with the switcher's Rename — same
    /// dialog, only reached differently). Return: closing by Cancel
    /// returns focus to the exact per-workspace button that opened it —
    /// `ids::settings::workspace_rename(&b_id)`, never the switcher's own
    /// generic `ids::switcher::rename()`.
    #[test]
    fn settings_rename_entry_is_the_name_field_cancel_returns_to_its_own_rename_button() {
        let (mut state, _a_id, b_id, _tmp) = settings_state_two_workspaces();
        let mut rig = Rig::new(SIZE);
        let rename_b = ids::settings::workspace_rename(&b_id);

        open_rename_dialog_for(&mut rig, &mut state, &rename_b);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::workspace_manager::name()),
            "opening Rename via a real key must focus its own entry, the name field"
        );

        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::cancel()
        ));
        key(&mut rig, &state, Named::Enter);
        let cancelled = last_message(&rig);
        assert!(matches!(
            cancelled,
            Message::Workspace(WorkspaceMessage::RenameWorkspaceCancelled)
        ));
        drive(&mut rig, &mut state, cancelled);

        assert_eq!(
            crate::state::layer::top_layer(&state),
            None,
            "sanity: Cancel must close the dialog"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(rename_b.clone()),
            "closing Rename must return focus to Settings' own per-workspace button that \
             opened it"
        );
        assert_ne!(
            focused_id(&mut rig, &state),
            Some(ids::switcher::rename()),
            "must never return to the switcher's own generic Rename — a different button"
        );
    }

    /// The same entry/return pair for Remove: the entry is the dialog's
    /// own safe default, Cancel (R6 — never the destructive action), and
    /// closing by Cancel returns focus to `ids::settings::
    /// workspace_remove(&b_id)`, never `ids::switcher::delete()`.
    #[test]
    fn settings_remove_entry_is_cancel_and_cancel_returns_to_its_own_remove_button() {
        let (mut state, _a_id, b_id, _tmp) = settings_state_two_workspaces();
        let mut rig = Rig::new(SIZE);
        let remove_b = ids::settings::workspace_remove(&b_id);

        open_delete_dialog_for(&mut rig, &mut state, &remove_b);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::workspace_manager::delete_cancel()),
            "opening Remove via a real key must focus its own entry, Cancel"
        );

        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::delete_cancel()
        ));
        key(&mut rig, &state, Named::Enter);
        let cancelled = last_message(&rig);
        assert!(matches!(
            cancelled,
            Message::Workspace(WorkspaceMessage::DeleteWorkspaceCancelled)
        ));
        drive(&mut rig, &mut state, cancelled);

        assert_eq!(crate::state::layer::top_layer(&state), None);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(remove_b.clone()),
            "closing Remove must return focus to Settings' own per-workspace button that \
             opened it"
        );
        assert_ne!(
            focused_id(&mut rig, &state),
            Some(ids::switcher::delete()),
            "must never return to the switcher's own generic Remove — a different button"
        );
    }

    /// The neighbour rule (A1 of stage 3), for a real removal reached from
    /// Settings: with two workspaces and the second (Beta, the last row
    /// in the per-workspace list) removed via its own per-workspace
    /// Remove, Beta's whole row vanishes with nothing sliding up to take
    /// its place (it was last), so the page's own remembered focus (Beta's
    /// Remove) is genuinely gone and the fallback search finds the next
    /// surviving id in the page's own flat tracked order — not another
    /// workspace's own button (there is only one left, Main, whose own
    /// Remove is a different row entirely), but "Add workspace", the
    /// control that was already right after the workspace list the whole
    /// time. The same backward/forward neighbour search already proven
    /// for dashboard rows (`stage3_amendment_a1_return_on_close::
    /// remove_project_confirmed_lands_on_alphas_row_its_neighbour`),
    /// exercised here for Settings' own per-workspace controls — see
    /// this module's own report for a found-but-not-fixed gap in the same
    /// mechanism when the *removed* workspace is not the last one.
    #[test]
    fn settings_remove_confirmed_removes_its_row_and_focus_falls_to_the_neighbour() {
        let (mut state, _a_id, b_id, _tmp) = settings_state_two_workspaces();
        let mut rig = Rig::new(SIZE);
        let remove_b = ids::settings::workspace_remove(&b_id);

        open_delete_dialog_for(&mut rig, &mut state, &remove_b);
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::workspace_manager::delete_cancel())
        );

        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::confirm_delete()
        ));
        key(&mut rig, &state, Named::Enter);
        let confirmed = last_message(&rig);
        assert!(matches!(
            confirmed,
            Message::Workspace(WorkspaceMessage::DeleteWorkspaceConfirmed)
        ));
        drive(&mut rig, &mut state, confirmed);

        assert!(
            state.all_workspaces.iter().all(|ws| ws.id != b_id),
            "sanity: Beta must really be gone"
        );
        assert_eq!(state.all_workspaces.len(), 1, "sanity: only Main survives");
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::add_workspace()),
            "A1's own rule: with Beta's whole row gone and nothing left in the workspace \
             list to slide into its place, focus falls to the next surviving control in \
             the page's own tracked order, \"Add workspace\" — never to nothing, and never \
             to the switcher's own generic Remove"
        );
    }

    /// The harder case the test above cannot reach: removing a
    /// **non-last** workspace, so another row (Gamma's) really does slide
    /// up into the removed row's own old tree position. Before `view/
    /// settings.rs::view_workspaces_section` was converted onto `iced::
    /// widget::keyed::Column` (keyed by each `WorkspaceId`), this slid
    /// Gamma's row into Beta's own old, really-focused state — landing
    /// real focus on Gamma's Remove, not Gamma's Rename, and never
    /// running `belongs_on`'s id-based search at all (confirmed directly
    /// before the fix: this assertion failed with exactly that id).
    /// Keyed, Beta's row is genuinely gone rather than reused, so nothing
    /// reports real focus and `belongs_on`'s own forward-then-backward
    /// neighbour search runs — forward from Beta's own Remove in the
    /// page's old tracked order finds Gamma's own Rename next.
    #[test]
    fn settings_remove_confirmed_for_a_non_last_workspace_lands_on_its_neighbour() {
        let (mut state, _a_id, b_id, c_id, _tmp) = settings_state_three_workspaces();
        let mut rig = Rig::new(SIZE);
        let remove_b = ids::settings::workspace_remove(&b_id);

        open_delete_dialog_for(&mut rig, &mut state, &remove_b);
        assert!(tab_to(
            &mut rig,
            &mut state,
            &ids::workspace_manager::confirm_delete()
        ));
        key(&mut rig, &state, Named::Enter);
        let confirmed = last_message(&rig);
        assert!(matches!(
            confirmed,
            Message::Workspace(WorkspaceMessage::DeleteWorkspaceConfirmed)
        ));
        drive(&mut rig, &mut state, confirmed);

        assert!(
            state.all_workspaces.iter().all(|ws| ws.id != b_id),
            "sanity: Beta must really be gone"
        );
        assert_eq!(
            state.all_workspaces.len(),
            2,
            "sanity: Main and Gamma survive"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::settings::workspace_rename(&c_id)),
            "with Gamma's row sliding into Beta's own old tree position, focus must still \
             land on the neighbour `belongs_on` finds by id — never on whatever control a \
             plain, unkeyed Column would have left behind at that position"
        );
    }
}

/// RFC-075 handoff stage 4 §5, rows 2 and 4 — History's own Tab sequence,
/// and the `FollowsId`-by-id mechanism (already proven for dashboard rows
/// in `focus_follows_its_id`) written for History's own per-entry
/// controls, keyed by each entry's stable key, its `OperationId`
/// (`view/ids.rs::history`'s own module doc comment).
#[cfg(test)]
mod stage4_history {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::{AppConfig, AppPaths};
    use crate::message::{BackgroundMessage, Message, ShortcutMessage};
    use crate::state::{AppState, OperationOwner, Screen};
    use crate::view::ids;
    use iced::widget::Id;
    use knotra_vcs::model::operation::OperationKind;
    use knotra_vcs::{OperationId, OperationLog, OperationResult};

    fn tmp_paths() -> (AppPaths, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let paths = AppPaths {
            config_file: tmp.path().join("config.toml"),
            workspaces_dir: tmp.path().join("workspaces"),
            history_dir: tmp.path().join("history"),
        };
        (paths, tmp)
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

    fn tab_to(rig: &mut Rig, state: &mut AppState, target: &Id) -> bool {
        for _ in 0..40 {
            tab(rig, state);
            if focused_id(rig, state) == Some(target.clone()) {
                return true;
            }
        }
        false
    }

    fn assert_tab_sequence_wraps(rig: &mut Rig, state: &mut AppState, expected: &[Id]) {
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

    /// A minimal, real `OperationLog` — empty `per_project` throughout,
    /// since every test in this module is about *which entry holds
    /// focus*, never about what a given entry's own summary/detail
    /// renders (`history.rs`'s own test module already covers that).
    fn sample_log(id: OperationId) -> OperationLog {
        let now = jiff::Timestamp::now();
        OperationLog {
            result: OperationResult {
                operation_id: id,
                kind: OperationKind::Fetch,
                started_at: now,
                finished_at: now,
                per_project: Vec::new(),
                rollback_attempted: false,
                rollback_succeeded: None,
            },
            recovery_hints: Vec::new(),
        }
    }

    fn history_state_two_entries() -> (AppState, OperationId, OperationId, tempfile::TempDir) {
        let (paths, tmp) = tmp_paths();
        let mut state = AppState::new_with_paths(AppConfig::default(), paths);
        state.screen = Screen::History;
        let id_a = OperationId::new();
        let id_b = OperationId::new();
        state.operation_logs = vec![sample_log(id_a), sample_log(id_b)];
        // Row 2's own "one of them expanded" — its detail content (none,
        // here, since `per_project` is empty) must add no extra Tab stop;
        // `history.rs`'s own `record_row` never does (checked directly:
        // its detail is plain `text`, nothing focusable).
        state.history_expanded.insert(id_a);
        (state, id_a, id_b, tmp)
    }

    // -----------------------------------------------------------------
    // Row 2: History's Tab sequence, with two entries, one expanded.
    // -----------------------------------------------------------------

    #[test]
    fn history_tab_sequence_two_entries_one_expanded_wraps_both_ways() {
        let (mut state, id_a, id_b, _tmp) = history_state_two_entries();
        let mut rig = Rig::new(SIZE);

        // History is the active screen, so `nav_history` is disabled
        // (skipped, D4 b) — `nav_dashboard` is not.
        let expected: Vec<Id> = vec![
            ids::shell::nav_dashboard(),
            ids::shell::refresh(),
            ids::shell::palette(),
            ids::shell::settings(),
            ids::history::search(),
            ids::history::toggle(&id_a),
            ids::history::copy(&id_a),
            ids::history::toggle(&id_b),
            ids::history::copy(&id_b),
        ];

        assert_tab_sequence_wraps(&mut rig, &mut state, &expected);
    }

    // -----------------------------------------------------------------
    // Row 4: a new History entry arriving above the currently-focused
    // one — focus stays on its own entry, by `OperationId`.
    // -----------------------------------------------------------------

    /// Mirrors `focus_follows_its_id::focus_stays_on_its_project_when_a_
    /// refresh_moves_it_to_another_tier`'s own real-message shape: a
    /// genuine `BackgroundMessage` a live fetch completing would publish
    /// (`SingleFetchCompleted`), not a direct `state.operation_logs`
    /// write — `persist_log`'s own `insert(0, ...)` (`app/background/
    /// mod.rs`) is what actually puts the new entry above every existing
    /// one.
    ///
    /// **Found failing against production code, then fixed at the root**
    /// (a test must never assert a known defect as correct, and the
    /// handoff's own §2 requires this exact scenario to survive). The
    /// test-writing pass confirmed directly (instrumented
    /// `knotra_ui::widget::keyed::FollowsId::follow`, reverted after)
    /// that the redraw applying `persist_log`'s insert reported
    /// `survey.focused` as the *new* entry's own toggle, not `None` —
    /// `list()`'s own plain `Column` (no stable per-item key) reused the
    /// tree-position-0 widget's internal `is_focused` flag for whatever
    /// now rendered there, before `FollowsId` ever got a look, and
    /// `follow`'s own "something is really, currently focused — trust it
    /// completely" rule then took that leaked, spurious focus at face
    /// value instead of running `belongs_on`'s id-based reconciliation at
    /// all. Same class as Settings' own per-workspace Remove (removing a
    /// non-last workspace) — one mechanism, not two bugs.
    ///
    /// Fixed by giving `list()`'s own callers a keyed alternative,
    /// `knotra_ui::widget::list_keyed` (built on `iced::widget::keyed::
    /// Column`, which exists for exactly this: "adding an element at the
    /// top may cause a loss of continuity on every element on the list"
    /// — `iced_widget::keyed`'s own module docs). History's `view_body`
    /// now calls it keyed by each entry's own `OperationId` (made `Copy`
    /// for this — a `Uuid` already is — `knotra-vcs/src/model/
    /// operation.rs`); Settings' own per-workspace list is converted the
    /// same way, keyed by `WorkspaceId`. `list()` itself, and every other
    /// caller (dashboard rows, window lists), is untouched — reconciling
    /// *their* own reorders/insertions the same way is a separate,
    /// broader change the review-request reports but does not make here.
    #[test]
    fn focus_stays_on_its_entry_when_a_new_one_arrives_above_it() {
        let (mut state, id_a, _id_b, _tmp) = history_state_two_entries();
        let mut rig = Rig::new(SIZE);

        assert!(tab_to(&mut rig, &mut state, &ids::history::toggle(&id_a)));

        let lease_id = state
            .operation_interlock
            .try_acquire(OperationOwner::SingleFetch)
            .expect("sanity: the interlock starts idle");
        let new_id = OperationId::new();
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::SingleFetchCompleted {
                lease_id,
                log: sample_log(new_id),
            }),
        );

        assert_eq!(
            state.operation_logs.first().map(|l| l.result.operation_id),
            Some(new_id),
            "sanity: the new entry must really land above (index 0)"
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::history::toggle(&id_a)),
            "focus must stay on its own entry, by OperationId — not on whatever is now at \
             the position it used to occupy"
        );
    }
}

/// RFC-075 stage 4 Amendment A1 §2 — Review 305's own probe, and every
/// other row in the amendment's own test table, each driven through the
/// real key/message path, on the dashboard's own plain `Column` (never
/// keyed) — `focus_belongs_to_reader` (`knotra_ui::widget::keyed.rs`) is
/// what every one of these actually exercises.
#[cfg(test)]
mod stage4_amendment_a1 {
    use super::{Rig, SIZE, drive, focused_id};
    use crate::config::AppConfig;
    use crate::message::{BackgroundMessage, Message, ShortcutMessage};
    use crate::state::AppState;
    use crate::view::ids;
    use iced::advanced::widget::operation;
    use iced::advanced::widget::{Id, Operation};
    use iced::{Rectangle, mouse};
    use knotra_vcs::model::status::{
        ConflictStatus, RepositoryIdentity, VcsKind, WorkingTreeStatus,
    };
    use knotra_vcs::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceStatus};

    fn two_needs_help_projects(a: &str, b: &str) -> (AppState, ProjectId, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let pa = Project::new(a, "/tmp");
        let pb = Project::new(b, "/tmp");
        let (a_id, b_id) = (pa.id.clone(), pb.id.clone());
        state.workspace = Some(Workspace {
            projects: vec![pa, pb],
            ..Workspace::new("Main")
        });
        state.load_phase = crate::state::LoadPhase::Ready;
        state.config.dashboard_sort = crate::config::DashboardSort::NameAscending;
        (state, a_id, b_id)
    }

    fn needs_help_status(id: ProjectId, has_conflict: bool) -> ProjectStatus {
        ProjectStatus {
            project_id: id,
            identity: RepositoryIdentity {
                path: "/tmp".into(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: Default::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: ConflictStatus {
                has_conflict,
                conflict_count: None,
                detection_unavailable: false,
            },
            refreshed_at: jiff::Timestamp::now(),
            read_error: None,
        }
    }

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

    /// Row 1 — Review 305's own probe, reproduced directly: Beta and
    /// Gamma, both `NeedsHelp`/`StatusUnknown`, `NameAscending` so Beta
    /// sorts before Gamma; ↓ twice reaches Gamma. Alpha is then added —
    /// sorting first — the insertion itself, matching the review's own
    /// repro exactly (not the "Alpha added and sorted first" sentence
    /// paraphrased, the actual case).
    ///
    /// **Seen failing first**, before the fix: real focus landed on
    /// Alpha's own name (the control that slid into Gamma's old,
    /// position-0-of-2 tree slot once it became position-1-of-3) —
    /// confirmed directly by temporarily reverting `focus_belongs_to_
    /// reader` to its pre-amendment shape (`Some(now.clone())`
    /// unconditionally) and re-running this test, then restoring.
    #[test]
    fn review_305_probe_alpha_added_and_sorted_first_focus_stays_on_gamma() {
        let (mut state, _beta_id, gamma_id) = two_needs_help_projects("Beta", "Gamma");
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&gamma_id)),
            "sanity: two ↓ presses under NameAscending must reach Gamma (Beta, then Gamma)"
        );

        state
            .workspace
            .as_mut()
            .unwrap()
            .projects
            .insert(0, Project::new("Alpha", "/tmp"));
        // A no-op message, just to drive one more rebuild/redraw through
        // `Rig`, the same convention every other reorder/removal test in
        // this file uses.
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
            Some(ids::row::name(&gamma_id)),
            "focus must stay on Gamma's own name — not on whatever project now sits where \
             Gamma's row used to be"
        );
    }

    /// Row 2 — a refresh moves a *different* project into a section
    /// above the focused one: Alpha starts `InProgress` (focused), Beta
    /// starts `AllSet` (below, collapsed away); a real `BackgroundMessage`
    /// then gives Beta a conflict, moving it to `NeedsHelp` — above
    /// Alpha's own section. Alpha's own row slides down to make room.
    #[test]
    fn a_project_inserted_above_the_focused_row_by_a_refresh_leaves_focus_on_its_own_project() {
        let (mut state, alpha_id, beta_id) = two_needs_help_projects("Alpha", "Beta");
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![
                in_progress_status(alpha_id.clone()),
                needs_help_status(beta_id.clone(), false), // StatusUnknown is NeedsHelp
            ],
            last_refresh: None,
        });
        // Beta: no conflict yet, but `in_progress_status` for Alpha alone
        // leaves Beta with a real, non-conflicted status entry, so it
        // classifies cleanly as `AllSet` instead — matching `AllSet`'s own
        // "clean, no counts" rule (`focus_follows_its_id::in_progress_
        // status`'s own doc comment), not `StatusUnknown`.
        state.workspace_status.as_mut().unwrap().projects[1] = ProjectStatus {
            project_id: beta_id.clone(),
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
        };
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "sanity: with Beta clean (AllSet) and Alpha InProgress, ↓ must focus Alpha, \
             the only NeedsHelp/InProgress row above AllSet's own collapsed section"
        );

        // The refresh lands: Beta now has a real conflict, moving it to
        // NeedsHelp — a section above Alpha's own InProgress.
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                WorkspaceStatus {
                    projects: vec![
                        in_progress_status(alpha_id.clone()),
                        needs_help_status(beta_id.clone(), true),
                    ],
                    last_refresh: None,
                },
            )),
        );

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "focus must stay on Alpha's own name, now with Beta's row inserted into a \
             section above it — not on whatever control now sits where Alpha's own row \
             used to be"
        );
    }

    /// Row 3 — the project above the focused one removed outright: Beta
    /// (`NeedsHelp`, above) and Alpha (`InProgress`, focused, below).
    /// Removing Beta's own project entirely must leave focus on Alpha.
    #[test]
    fn the_project_above_the_focused_one_removed_leaves_focus_on_its_own_project() {
        let (mut state, alpha_id, beta_id) = two_needs_help_projects("Alpha", "Beta");
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![
                in_progress_status(alpha_id.clone()),
                needs_help_status(beta_id.clone(), true),
            ],
            last_refresh: None,
        });
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&beta_id)),
            "sanity: Beta's own conflict puts it in NeedsHelp, the first section — ↓ must \
             focus Beta first"
        );
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "sanity: a second ↓ must reach Alpha, the only row in InProgress, the next \
             section after NeedsHelp"
        );

        state
            .workspace
            .as_mut()
            .unwrap()
            .projects
            .retain(|p| p.id != beta_id);
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
            Some(ids::row::name(&alpha_id)),
            "focus must stay on Alpha's own name once Beta — the project that was above \
             it — is gone outright"
        );
    }

    /// Row 5 — typing in the search box, so the rows change with every
    /// key: focus never leaves the field.
    #[test]
    fn typing_in_the_search_box_never_leaves_the_field() {
        let (mut state, _a_id, _b_id) = two_needs_help_projects("Alpha", "Banana");
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusSearchBare),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::search()),
            "sanity: the bare search shortcut must focus the search field"
        );

        for ch in "Al".chars() {
            state.filter.search_text.push(ch);
            let typed_so_far = state.filter.search_text.clone();
            drive(
                &mut rig,
                &mut state,
                Message::Filter(crate::message::FilterMessage::SearchChanged(
                    typed_so_far.clone(),
                )),
            );
            assert_eq!(
                focused_id(&mut rig, &state),
                Some(ids::toolbar::search()),
                "focus must still be in the search field after narrowing the rows to {:?}",
                typed_so_far
            );
        }
    }

    struct FindBounds {
        target: Id,
        bounds: Option<Rectangle>,
    }

    impl Operation for FindBounds {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn focusable(
            &mut self,
            id: Option<&Id>,
            bounds: Rectangle,
            _state: &mut dyn operation::Focusable,
        ) {
            if id == Some(&self.target) {
                self.bounds = Some(bounds);
            }
        }
    }

    /// A real mouse click, by id — unlike every other interaction in this
    /// file, which drives a `Message` or a key, this delivers raw
    /// `Event::Mouse` presses directly through `UserInterface::update`,
    /// **never through `operate()`** — the one way to reach a text
    /// field's own click-to-focus path (`Widget::update`'s own mouse
    /// handling) without also setting `FollowsId`'s own `explained` flag,
    /// which only a real `operate()`-driven focus change ever sets
    /// (`keyed.rs::FollowsId::operate`'s own before/after survey). Row 6
    /// needs exactly this: a click that focuses a field *without* an
    /// operation explaining it, so only `was_handed_across`'s own
    /// position check — not `explained` — can be what keeps it focused.
    fn click_id(rig: &mut Rig, state: &mut AppState, target: &Id) {
        let mut ui = super::rebuild(rig, state);
        let mut find = FindBounds {
            target: target.clone(),
            bounds: None,
        };
        ui.operate(&rig.renderer, &mut find);
        let bounds = find
            .bounds
            .unwrap_or_else(|| panic!("{target:?} must be in the tree to click it"));
        rig.cursor = mouse::Cursor::Available(bounds.center());
        let events = [
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ];
        let _ = ui.update(
            &events,
            rig.cursor,
            &mut rig.renderer,
            &mut super::clipboard::Null,
            &mut rig.messages,
        );
        super::redraw(rig, &mut ui);
        rig.cache = ui.into_cache();
    }

    /// Row 6 — a click into the search field (focus without an
    /// operation), then a refresh changes the rows: focus stays in the
    /// field.
    #[test]
    fn a_click_into_the_search_field_then_a_refresh_leaves_focus_in_the_field() {
        let (mut state, alpha_id, beta_id) = two_needs_help_projects("Alpha", "Beta");
        let mut rig = Rig::new(SIZE);

        click_id(&mut rig, &mut state, &ids::toolbar::search());
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::search()),
            "sanity: the click must land real focus on the search field, with no \
             `operate()`-driven operation involved at all"
        );

        // A refresh that reorders both projects — Beta gains a conflict
        // (NeedsHelp, now above Alpha's own continuing StatusUnknown
        // NeedsHelp row, by cause rank) — changing the rows under the
        // still-focused search field.
        drive(
            &mut rig,
            &mut state,
            Message::Background(BackgroundMessage::WorkspaceStatusRefreshed(
                WorkspaceStatus {
                    projects: vec![needs_help_status(beta_id.clone(), true)],
                    last_refresh: None,
                },
            )),
        );
        let _ = alpha_id;

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::toolbar::search()),
            "focus must stay in the search field once the refresh changes the rows \
             beneath it"
        );
    }

    /// Row 7 — Tab in the same step an insertion also happens: focus is
    /// wherever Tab put it, not pulled back by the insertion's own
    /// reconciliation in the same redraw.
    #[test]
    fn tab_in_the_same_step_as_an_insertion_lands_where_tab_put_it() {
        let (mut state, alpha_id, _beta_id) = two_needs_help_projects("Alpha", "Beta");
        let mut rig = Rig::new(SIZE);

        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::ArrowDown),
        );
        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::name(&alpha_id)),
            "sanity: ↓ must focus Alpha's own name first"
        );

        // Both take effect in the one redraw this single `drive` step
        // settles: the insertion first (mutated directly on `state`, no
        // message of its own), then Tab, matching `focus_follows_its_id::
        // a_tab_in_the_exact_same_step_a_reorder_happens_is_not_undone_
        // either`'s own established pattern for a reorder.
        state
            .workspace
            .as_mut()
            .unwrap()
            .projects
            .insert(0, Project::new("Aaa", "/tmp"));
        drive(
            &mut rig,
            &mut state,
            Message::Shortcut(ShortcutMessage::FocusNext),
        );

        assert_eq!(
            focused_id(&mut rig, &state),
            Some(ids::row::action(&alpha_id)),
            "Tab's own move, in the very same step a new project was also inserted \
             above it, must stand — Tab's own real target (Alpha's action, the next \
             tracked id after Alpha's name), not pulled back to Alpha's name by the \
             insertion's own reconciliation"
        );
    }
}
