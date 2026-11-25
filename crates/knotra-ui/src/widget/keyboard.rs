//! RFC-075 stage 2 §2, Amendment A1: snora's own keyboard model, extended
//! with the one thing it deliberately does not do — wrap — and the one
//! thing knotra's dashboard keys (D6) now need that `AppState` no longer
//! tracks: which real, live-focused control to move from.
//!
//! snora's `keyboard::traverse` moves real (`operation::Focusable`) focus
//! with Tab/Shift+Tab; past the last control it **leaves the group for one
//! press**, with nothing focused, and restarts from the first on the next
//! (`snora-0.65.0/src/keyboard.rs`'s own doc: "Tab at the end of a group
//! leaves it" — deliberate, for a window with controls after the group).
//! Amendment A1 decided the page itself has no "after": "nothing is ever
//! left focused on nothing... in a window and on the page alike." Closing
//! that gap needs a second traversal exactly when the first one leaves
//! nothing focused — and "what, if anything, is now focused" is something
//! only the real, live widget tree can answer, so this module asks it with
//! [`iced::advanced::widget::operate`] rather than reaching into
//! `snora_interaction`'s own (private) survey/act internals a second time.

use iced::Task;
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, operate};
use iced::keyboard::{Key, Modifiers};

/// An `Operation` that always resolves — unlike `iced_core::widget::
/// operation::focusable::find_focused`, whose `finish()` is `Outcome::None`
/// when nothing is focused, which `task::widget`'s channel can never turn
/// into a delivered `Task` output (a `None` outcome sends nothing, so a
/// caller waiting on it waits forever). This is that same walk, with
/// `finish()` always answering `Outcome::Some(..)`, so [`focused_id`]'s
/// `Task` always completes with a value to act on, "nothing is focused"
/// included.
struct FocusedId {
    found: Option<Id>,
}

impl Operation<Option<Id>> for FocusedId {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Id>>)) {
        operate(self);
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: iced::Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() && id.is_some() {
            self.found = id.cloned();
        }
    }

    fn finish(&self) -> Outcome<Option<Id>> {
        Outcome::Some(self.found.clone())
    }
}

/// The `Id` of whatever control currently holds real keyboard focus, or
/// `None` if nothing does — the one way to ask, from `update`, with no
/// synchronous access to the live widget tree.
pub fn focused_id() -> Task<Option<Id>> {
    operate(FocusedId { found: None })
}

/// Tab/Shift+Tab through `snora::keyboard::traverse`, wrapping at either end
/// (RFC-075 D4 a, Amendment A1) — the page's own replacement for knotra's
/// retired `advance_focus`. `None` when `key`/`modifiers` is not a Tab press
/// `traverse` would answer either (so a caller can hand it every key event
/// and fall through on `None`, the same contract `traverse` itself offers).
///
/// Runs `traverse` once; if that leaves nothing focused (the group's own
/// last-press behaviour), checks whether `scope`'s own `focus_by_id`
/// wrapper (Amendment A2 F3: the page's wrap, or the open window's —
/// `scope` names which, since the caller is the one that knows whether a
/// window is open) still remembers a control focus belongs to even
/// though it cannot report real focus right now — a control that
/// disables itself when pressed (Amendment A2 §3: "Check now" mid-
/// refresh, the selection bar's "Check for updates" mid-fetch): with
/// nothing really focused, `snora::keyboard::traverse` itself cannot tell
/// this apart from "nothing has ever been focused", and would restart
/// from the very first control in the whole group — the "no ring, then
/// Tab jumps to the top" Amendment A1 exists to rule out. Only when there
/// is no such remembered control (or no wrapper of that scope at all)
/// does this run `traverse` a **second** time in the same `Task` — which,
/// with nothing focused and no click-remembered starting point
/// (`traverse`'s own doc: a click's mark is cleared on every traversal,
/// focused or not), lands on the first target going forward or the last
/// going backward. That is the wrap.
#[must_use]
pub fn traverse_wrapping<Message: Send + 'static>(
    key: Key,
    modifiers: Modifiers,
    scope: super::keyed::Scope,
) -> Option<Task<Message>> {
    // Confirms the key is claimed (plain Tab/Shift+Tab) without running
    // anything yet — `traverse` is a pure function of its own arguments,
    // so building it again below, only in the branch that needs it, has
    // no effect beyond the confirmation this discards.
    let _ = snora::keyboard::traverse::<Message>(key.clone(), modifiers)?;

    let forward = !modifiers.shift();
    Some(focused_id().then(move |before: Option<Id>| {
        let key = key.clone();
        if before.is_some() {
            // The reader's own Tab, from a control that really holds
            // focus right now: run the real traversal; if that leaves
            // nothing focused (the group's own last-press behaviour),
            // run it a **second** time in the same `Task` — which, with
            // nothing focused and no click-remembered starting point
            // (`traverse`'s own doc: a click's mark is cleared on every
            // traversal, focused or not), lands on the first target
            // going forward or the last going backward. That is the
            // wrap.
            let real = snora::keyboard::traverse::<Message>(key.clone(), modifiers)
                .unwrap_or_else(Task::none);
            real.discard::<Option<Id>>()
                .chain(focused_id())
                .then(move |after: Option<Id>| {
                    if after.is_some() {
                        Task::none()
                    } else {
                        snora::keyboard::traverse(key.clone(), modifiers).unwrap_or_else(Task::none)
                    }
                })
        } else {
            // Nothing is really focused **before** this press at all —
            // `sequential_focus`'s own "nothing focused → start from the
            // first target" fallback would fire unconditionally here,
            // which is indistinguishable from the reader's own Tab
            // correctly landing somewhere (both report "something is
            // focused afterward"), so this must be checked *first*, not
            // inferred from the real traversal's own result. Amendment
            // A2 §3/F3: a `focus_by_id` wrapper may still remember a
            // control focus belongs to even though that control cannot
            // report real focus right now (it disabled itself on its own
            // press) — ask `scope`'s own wrapper before falling back to
            // `traverse`'s own natural "start from the first/last"
            // behaviour, which is right only when nothing has ever been
            // focused at all.
            super::keyed::tab_anchor(scope).then(move |anchor| {
                let reinstated = anchor.and_then(|a| {
                    let remembered = a.focused?;
                    neighbour(&a.ids, &remembered, forward)
                });
                if let Some(next) = reinstated {
                    iced::widget::operation::focus(next)
                } else {
                    snora::keyboard::traverse(key.clone(), modifiers).unwrap_or_else(Task::none)
                }
            })
        }
    }))
}

/// The id right after (`forward`) or right before `from` in `ids`, or
/// `None` at either end — deliberately **not** wrapping on its own: when
/// there is no neighbour this way, [`traverse_wrapping`]'s own fallback
/// (`snora::keyboard::traverse`, run with nothing focused) already lands
/// on the first/last target in the whole page, which is the same
/// destination a wrap from here would reach.
fn neighbour(ids: &[Id], from: &Id, forward: bool) -> Option<Id> {
    let at = ids.iter().position(|id| id == from)?;
    if forward {
        ids.get(at + 1).cloned()
    } else {
        at.checked_sub(1)
            .and_then(|before| ids.get(before))
            .cloned()
    }
}

/// Moves real focus to the id after (`forward`) or before the one `ids`
/// reports as currently focused, wrapping at either end; with focus on none
/// of `ids` (including nothing focused at all), lands on `ids[0]` regardless
/// of direction — RFC-075 D6's dashboard keys (↓ ↑ j k between project
/// names), now moving snora's real focus with `operation::focus(id)`
/// instead of a value `AppState` kept of its own. `ids` must be in display
/// order; `Task::none()` if empty.
pub fn focus_among<Message: Send + 'static>(ids: Vec<Id>, forward: bool) -> Task<Message> {
    if ids.is_empty() {
        return Task::none();
    }
    focused_id().then(move |current: Option<Id>| {
        let position = current.and_then(|id| ids.iter().position(|candidate| *candidate == id));
        let next = match position {
            Some(index) if forward => ids[(index + 1) % ids.len()].clone(),
            Some(index) => ids[(index + ids.len() - 1) % ids.len()].clone(),
            None => ids[0].clone(),
        };
        iced::widget::operation::focus(next)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Harness;
    use iced::widget::{column, text};
    use snora::design::Tokens;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Pressed(&'static str),
    }

    fn tokens() -> &'static Tokens {
        use std::sync::OnceLock;
        static T: OnceLock<Tokens> = OnceLock::new();
        T.get_or_init(Tokens::light)
    }

    fn three_controls() -> iced::Element<'static, Msg> {
        let make = |label: &'static str, id: &'static str| {
            iced::Element::from(
                snora::design::control(tokens(), text(label))
                    .height(crate::widget::layout::BUTTON_HEIGHT)
                    .id(iced::widget::Id::new(id))
                    .on_press(Msg::Pressed(label)),
            )
        };
        column![
            make("One", "one"),
            make("Two", "two"),
            make("Three", "three")
        ]
        .spacing(8)
        .into()
    }

    const SIZE: iced_test::core::Size = iced_test::core::Size {
        width: 400.0,
        height: 400.0,
    };

    /// `traverse_wrapping` is not itself drivable through [`Harness::tab`]
    /// (that method runs `snora::keyboard::traverse` directly, by design —
    /// see its own doc comment); this test runs its returned `Task`
    /// through the same `UserInterface` the harness wraps, exactly the way
    /// production `update()` hands it to the real runtime, and the only
    /// honest way to test a `Task` that chains a second operation on what
    /// the first one observed.
    fn run_wrapping(h: &mut Harness<Msg>, shift: bool) {
        use iced::futures::StreamExt;
        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let task = traverse_wrapping::<Msg>(
            Key::Named(iced::keyboard::key::Named::Tab),
            modifiers,
            super::super::keyed::Scope::Page,
        )
        .expect("Tab is claimed");
        let mut stream = iced_test::runtime::task::into_stream(task).expect("a task with work");
        loop {
            match iced::futures::executor::block_on(stream.next()) {
                Some(iced_test::runtime::Action::Widget(op)) => h.run_operation(op),
                Some(_) => continue,
                None => break,
            }
        }
    }

    #[test]
    fn wraps_past_the_last_control_onto_the_first() {
        let mut h = Harness::new(three_controls(), SIZE, None);
        run_wrapping(&mut h, false);
        run_wrapping(&mut h, false);
        run_wrapping(&mut h, false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("three")));

        // A fourth Tab would, without wrapping, leave nothing focused for
        // one press (snora's own documented behaviour) — `traverse_wrapping`
        // must not stop there.
        run_wrapping(&mut h, false);
        assert_eq!(
            h.focused_id(),
            Some(iced::widget::Id::new("one")),
            "Tab past the last control must wrap to the first, not leave \
             nothing focused"
        );
    }

    #[test]
    fn wraps_before_the_first_control_onto_the_last() {
        let mut h = Harness::new(three_controls(), SIZE, None);
        run_wrapping(&mut h, true);
        assert_eq!(
            h.focused_id(),
            Some(iced::widget::Id::new("three")),
            "Shift+Tab before the first control must wrap to the last"
        );
    }

    /// Runs any `Task<Msg>` built on `operate`/`task::effect` (never one
    /// that completes with a real `Msg`) through this harness, the same
    /// Action::Widget-at-a-time drive [`run_wrapping`] uses — generalized
    /// so [`focus_among`]'s own `Task` can be exercised the same way.
    fn run(h: &mut Harness<Msg>, task: Task<Msg>) {
        use iced::futures::StreamExt;
        let Some(mut stream) = iced_test::runtime::task::into_stream(task) else {
            return;
        };
        loop {
            match iced::futures::executor::block_on(stream.next()) {
                Some(iced_test::runtime::Action::Widget(op)) => h.run_operation(op),
                Some(_) => continue,
                None => break,
            }
        }
    }

    fn id(name: &'static str) -> iced::widget::Id {
        iced::widget::Id::new(name)
    }

    fn three_ids() -> Vec<iced::widget::Id> {
        vec![id("one"), id("two"), id("three")]
    }

    #[test]
    fn focus_among_with_nothing_focused_lands_on_the_first() {
        let mut h = Harness::new(three_controls(), SIZE, None);
        run(&mut h, focus_among(three_ids(), true));
        assert_eq!(h.focused_id(), Some(id("one")));

        // Direction does not matter when nothing was focused (RFC-075 D6,
        // unchanged from knotra's own retired `advance_card_focus` doc).
        let mut h = Harness::new(three_controls(), SIZE, None);
        run(&mut h, focus_among(three_ids(), false));
        assert_eq!(h.focused_id(), Some(id("one")));
    }

    #[test]
    fn focus_among_moves_forward_and_wraps() {
        let mut h = Harness::new(three_controls(), SIZE, None);
        run(&mut h, focus_among(three_ids(), true));
        run(&mut h, focus_among(three_ids(), true));
        assert_eq!(h.focused_id(), Some(id("two")));
        run(&mut h, focus_among(three_ids(), true));
        assert_eq!(h.focused_id(), Some(id("three")));
        run(&mut h, focus_among(three_ids(), true));
        assert_eq!(
            h.focused_id(),
            Some(id("one")),
            "forward past the last id must wrap to the first"
        );
    }

    #[test]
    fn focus_among_moves_backward_and_wraps() {
        let mut h = Harness::new(three_controls(), SIZE, None);
        run(&mut h, focus_among(three_ids(), true)); // -> "one"
        run(&mut h, focus_among(three_ids(), false));
        assert_eq!(
            h.focused_id(),
            Some(id("three")),
            "backward from the first id must wrap to the last"
        );
    }

    #[test]
    fn focus_among_ignores_focus_on_a_control_not_in_the_list() {
        // Tab focuses "one" directly (not through `focus_among`); "one" is
        // not in the two-id list below, so this must behave as "nothing
        // focused" and land on the list's own first id, not step relative
        // to a target it does not know.
        let mut h = Harness::new(three_controls(), SIZE, None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(id("one")));
        run(&mut h, focus_among(vec![id("two"), id("three")], true));
        assert_eq!(h.focused_id(), Some(id("two")));
    }
}
