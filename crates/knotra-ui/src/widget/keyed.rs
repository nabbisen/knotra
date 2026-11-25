//! RFC-075 stage 2 Amendment A1 §1, corrected by Amendment A2 §1: focus
//! follows the control it is on, by its own [`iced::widget::Id`] — not by
//! its place in the tree, which is how iced actually keeps a widget's
//! state (focus included). Review 299 F1 found the gap directly: a
//! background refresh that moves a project between the dashboard's own
//! tiers can hand keyboard focus to a **different** project's row, with
//! no key pressed, because the rebuilt tree's position that used to be
//! "Alpha's name" is now "Beta's name", and iced's own diffing keeps
//! whatever was focused at that position.
//!
//! **A2 F1: the ids are read from the tree, not copied by hand.**
//! `Operation::focusable` already hands every target its own `id` — the
//! same `id` stage 2's `view::ids` gives every page control — so this
//! widget's own survey (`Survey`, below) collects `(Id, is_focused)` for
//! every target it sees, in the order it sees them, and refocuses **by
//! id** (`FocusById`), never by counting positions. The hand-maintained
//! `dashboard_content_focus_keys` list and its "exactly one target per
//! key, or stand aside" guess are both gone with it — there is nothing
//! left for a view change to silently fall out of step with, because
//! nothing is derived from the view a second time.
//!
//! **A focus target with no id cannot be followed.** `Survey::focusable`
//! only records a target it is given an `id` for; one with none is
//! invisible to this widget — if it is genuinely focused, that reads as
//! "nothing is focused" here, and if this widget later decides something
//! else should be focused, the id-less target is unfocused along with
//! everything else and never gets it back. Every control RFC-075 stage 2
//! draws carries its own id (`view::ids`), so this should never actually
//! happen on the page; `keyboard_acceptance`'s own
//! `every_focus_target_on_the_page_has_an_id` test is what stands behind
//! that claim instead of just asserting it.
//!
//! **A2 F2: wraps the whole page, in one call, not only the dashboard's
//! own rows.** Review 299's two findings (opening Arrange, entering
//! selection mode) were both *toolbar* controls, outside stage 2's own
//! narrower wrap — each needed its own `operation::focus(id)` fix
//! because of it. **Two separate `focus_by_id` calls (header, body) were
//! tried first and found broken**: whichever region focus had just left
//! (crossing from the header into the body, say) cannot tell "my own
//! remembered control moved elsewhere, deliberately" from "my own
//! remembered control lost its spot structurally" — both present
//! identically as "nothing of mine is focused right now" from inside a
//! region that cannot see focus outside itself — so the region focus
//! left kept reclaiming it, and Tab could never leave the header at all
//! (every acceptance test got stuck on the shell's first control). One
//! wrap, around the whole page (`view.rs`'s own `page`, built as
//! `column![shell::view(state), body]` and given to `AppLayout::new`
//! with no separate `.header(...)` call — see that call site's own
//! comment for why bypassing that slot is safe here), removes the
//! "outside" entirely: both findings' own explicit `operation::focus`
//! fixes are gone from `app/misc.rs`, and their own tests
//! (`keyboard_acceptance::findings`) pass with no explicit fix behind
//! them at all, since nothing now moves focus away from the control
//! that was just pressed in the first place.
//!
//! **A2 F3: a control that disables itself when pressed has not gone.**
//! `snora::design::Control::operate` only reports a control's own id
//! while it can be pressed (`on_press.is_some()`) — a disabled control is
//! invisible to `Survey`, the same as one with no id at all, so without
//! more information this widget cannot tell "Check now, mid-refresh"
//! from "Alpha's row, removed outright": both simply vanish from the
//! survey. `reserved` is that information: ids the caller already knows
//! disable themselves without leaving — `view.rs` passes
//! `ids::shell::refresh()` and `ids::selection_bar::fetch()` (the two
//! found by reading every `view/` site gated on `is_refreshing`/
//! `operation_interlock.is_busy()` whose own press is what sets the
//! flag — see that call site's own comment for the rest, excluded for
//! not being *self*-disabling). [`merge_reserved`] reinstates a missing
//! reserved id into the tracked order at its last-known relative
//! position, so [`belongs_on`] still finds it (not a neighbour) once it
//! is disabled, and [`follow`](FollowsId::follow) leaves real focus at
//! none rather than assigning it to a reinstated-but-still-disabled id
//! (nothing to call `.focus()` on, and no ring would show regardless) —
//! when the id reappears enabled, the same "still in the tracked list"
//! path refocuses it for real, with no special case for "coming back".
//!
//! **Tab must still reach past a disabled-but-reserved control.**
//! `snora::keyboard::traverse`'s own `sequential_focus` counts only real
//! `focusable()` targets; with nothing really focused (Check now, once
//! disabled, cannot be) it restarts from the very first target in the
//! whole page, exactly the "no ring, then Tab jumps to the top" Amendment
//! A1 was written to rule out. [`tab_anchor`] answers the one question
//! `knotra_ui::widget::keyboard::traverse_wrapping` needs to do better:
//! "what does this wrapper itself still consider focus to belong to, and
//! in what order are its own tracked ids" — read directly off this
//! widget's own `State` via `Operation::custom`'s `&mut dyn Any` channel
//! (the same channel `snora_interaction::sequence::sequential_focus`
//! itself uses for its own "last clicked" mark), so a caller one level up
//! never needs its own private copy of what this widget already knows.

use std::any::Any;

use iced::Task;
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{self, Id, Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Theme, Vector, window};

/// What [`tab_anchor`] answers: every id this wrapper currently tracks, in
/// order, and which one (if any) it considers focus to belong to — real
/// or reserved-but-disabled.
#[derive(Clone, Default)]
pub struct Anchor {
    pub ids: Vec<Id>,
    pub focused: Option<Id>,
}

struct AnchorQuery {
    found: Option<Anchor>,
}

impl Operation<Option<Anchor>> for AnchorQuery {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Anchor>>)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if let Some(anchor) = state.downcast_ref::<Anchor>() {
            // More than one `focus_by_id` wrapper can be live in the tree at
            // once (the page's own, plus whichever window just opened this
            // same frame — every window wraps itself too). A window's own
            // wrapper starts with `State::default()` (`ids` empty) until its
            // first `RedrawRequested` ever reaches it, which cannot have
            // happened yet when this query runs as part of resolving the
            // very message that opened it. Unconditionally overwriting
            // `found` on every visit (this function's original body) let
            // that brand-new, meaningless empty answer clobber the page's
            // own real one the moment any window opened — which, read back
            // through `settle_layer_focus`'s own "(None, Some(layer))"
            // branch, made every window opened by a real key press (not a
            // bare shortcut) resolve to Mouse mode unconditionally,
            // regardless of whether the reader's own control genuinely held
            // focus. A wrapper's `ids` is empty only in that one
            // never-yet-surveyed case — every real window always tracks at
            // least its own close button — so preferring the last wrapper
            // with a genuinely non-empty `ids` keeps this query answering
            // for the one the reader was actually just using, in both
            // directions: the page's own frozen memory when a window has
            // just opened over it, and a window's own (by then populated)
            // memory once Tab has been pressed inside it at least once.
            if !anchor.ids.is_empty() || self.found.is_none() {
                self.found = Some(anchor.clone());
            }
        }
    }

    fn finish(&self) -> Outcome<Option<Anchor>> {
        Outcome::Some(self.found.clone())
    }
}

/// The nearest `focus_by_id` wrapper's own tracked ids and the id it
/// considers focus to belong to, or `None` if this page has no such
/// wrapper at all — always resolves (never `Outcome::None`, which
/// `task::widget`'s channel can never turn into a delivered value), the
/// same reason `knotra_ui::widget::keyboard::focused_id` does.
pub fn tab_anchor() -> Task<Option<Anchor>> {
    widget::operate(AnchorQuery { found: None })
}

/// Wraps `content` so real keyboard focus follows the control it is on by
/// its own [`Id`], across a rebuild that reorders, adds, or removes the
/// focus targets inside it — see the module doc for the mechanism, where
/// it wraps, and what `reserved` is for. Pass `&[]` when nothing in
/// `content` disables itself on its own press.
pub fn focus_by_id<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    reserved: &[Id],
) -> Element<'a, Message> {
    FollowsId {
        content: content.into(),
        reserved: reserved.to_vec(),
    }
    .into()
}

#[derive(Default)]
struct State {
    /// Every id tracked as of the last redraw — the real survey, with any
    /// `reserved` id reinstated at its last-known relative position when
    /// the real survey is currently missing it.
    ids: Vec<Id>,
    /// The id this widget currently considers focus belongs to — real or
    /// reserved-but-disabled.
    focused: Option<Id>,
}

struct FollowsId<'a, Message> {
    content: Element<'a, Message>,
    reserved: Vec<Id>,
}

/// Every focus target's own id, in order, and which one (if any) is
/// really, currently focused — the one pass `follow` needs, direct from
/// `Operation::focusable`'s own `id` rather than a count of `bool`s.
#[derive(Default)]
struct Survey {
    ids: Vec<Id>,
    focused: Option<Id>,
}

impl Operation for Survey {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        let Some(id) = id else {
            return;
        };
        self.ids.push(id.clone());
        if state.is_focused() {
            self.focused = Some(id.clone());
        }
    }
}

/// Unfocuses every target, then focuses the one whose id matches `target`
/// — the same one-pass "unfocus everyone else, focus the match"
/// `iced::widget::operation::focus` itself uses
/// (`iced_core-0.14.0/src/widget/operation/focusable.rs`).
struct FocusById<'a> {
    target: &'a Id,
}

impl Operation for FocusById<'_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        match id {
            Some(id) if id == self.target => state.focus(),
            _ => state.unfocus(),
        }
    }
}

/// Unfocuses every target, with nothing taking its place — "nothing
/// should be focused at all" has no single id to give `FocusById`.
struct UnfocusAll;

impl Operation for UnfocusAll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        state.unfocus();
    }
}

/// Reinstates each `reserved` id that `survey_ids` is currently missing
/// back into the tracked order, at the position its own surviving
/// neighbours (from `old_ids`, the order as of the *previous* redraw)
/// suggest — so a merely-disabled control is not mistaken for one that
/// has genuinely gone. An id `reserved` names that has never once been
/// seen (not in `old_ids` either — nothing to place it relative to) is
/// left out; it was never really there to begin with.
fn merge_reserved(old_ids: &[Id], survey_ids: &[Id], reserved: &[Id]) -> Vec<Id> {
    let mut merged = survey_ids.to_vec();
    for id in reserved {
        if merged.contains(id) {
            continue;
        }
        let Some(old_pos) = old_ids.iter().position(|old| old == id) else {
            continue;
        };
        let insert_at = old_ids[..old_pos]
            .iter()
            .filter(|old| merged.contains(old))
            .count();
        merged.insert(insert_at.min(merged.len()), id.clone());
    }
    merged
}

/// Where focus belongs when `focused`'s own id is not the one really
/// focused right now: still itself, if `now` still carries it (reordered,
/// or merely reinstated as disabled-but-not-gone by [`merge_reserved`]);
/// else its neighbour that is still there — the one after it in the old
/// order, else the one before — else nowhere. RFC-120 R-1's own rule,
/// read by id instead of counted by position.
fn belongs_on(old: &[Id], now: &[Id], focused: &Id) -> Option<Id> {
    if now.contains(focused) {
        return Some(focused.clone());
    }
    let was = old.iter().position(|id| id == focused)?;
    let survivor = |id: &&Id| now.contains(*id);
    old[was + 1..]
        .iter()
        .find(survivor)
        .or_else(|| old[..was].iter().rev().find(survivor))
        .cloned()
}

impl<Message> FollowsId<'_, Message> {
    fn follow(&mut self, tree: &mut Tree, layout: layout::Layout<'_>, renderer: &iced::Renderer) {
        let mut survey = Survey::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut survey);

        // RFC-075 stage 3: `snora::Inert` (what a dialog/sheet/menu wraps
        // the page in while it is open) hides every `focusable()` call
        // from its own content's `operate` entirely — confirmed by
        // reading it directly (`snora-interaction-0.65.0/src/inert.rs`'s
        // own `Hide`: "`focusable` is the one thing not forwarded"), not
        // merely a disabled-control count mismatch. Content wrapped this
        // way surveys as empty, every redraw, for as long as the window
        // stays open — if reconciliation ran anyway, it would read
        // `survey.focused == None` as "lost, go find a neighbour" and
        // overwrite `state.focused` with a guess a `FocusById` attempt
        // can never actually land (also hidden by the same `Inert`),
        // corrupting the one thing a window's own close needs: which id
        // to return focus to. Freezing `state` here — reporting it
        // exactly as it was, never touching it — is what lets that
        // return happen with no explicit code of its own: once the
        // window closes and this content is reachable again, the very
        // next redraw finds `state.focused` exactly as it was before the
        // window opened, and reconciles normally from there.
        if survey.ids.is_empty() {
            return;
        }

        let state = tree.state.downcast_ref::<State>();
        let new_ids = merge_reserved(&state.ids, &survey.ids, &self.reserved);

        let want = if let Some(now) = &survey.focused {
            // Something is really, currently focused — trust it
            // completely: the reader's own move (Tab, an arrow), another
            // fix's own `operation::focus` to a sibling, or simply
            // nothing having changed. Never second-guessed.
            Some(now.clone())
        } else if let Some(remembered) = &state.focused {
            belongs_on(&state.ids, &new_ids, remembered)
        } else {
            None
        };

        if want != survey.focused {
            if let Some(id) = &want {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut FocusById { target: id },
                );
            } else {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut UnfocusAll,
                );
            }
        }

        let state = tree.state.downcast_mut::<State>();
        state.ids = new_ids;
        state.focused = want;
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for FollowsId<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Widget::size(self.content.as_widget())
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
        // Offers `tab_anchor`'s own query a look at this widget's current
        // `State` through `Operation::custom`'s `&mut dyn Any` channel —
        // a no-op for every operation that does not ask for it (its own
        // default body), the same shape `sequential_focus`'s own "last
        // clicked" mark already uses this channel for.
        let state = tree.state.downcast_ref::<State>();
        let mut anchor = Anchor {
            ids: state.ids.clone(),
            focused: state.focused.clone(),
        };
        operation.custom(None, layout.bounds(), &mut anchor);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            self.follow(tree, layout, renderer);
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<FollowsId<'a, Message>> for Element<'a, Message> {
    fn from(value: FollowsId<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{belongs_on, merge_reserved};
    use iced::widget::Id;

    fn id(name: &'static str) -> Id {
        Id::new(name)
    }

    fn ids(names: &[&'static str]) -> Vec<Id> {
        names.iter().map(|n| id(n)).collect()
    }

    #[test]
    fn focus_stays_on_its_item_when_another_goes() {
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["b", "a"]), &id("b")),
            Some(id("b"))
        );
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["c", "a"]), &id("a")),
            Some(id("a"))
        );
    }

    #[test]
    fn a_removed_item_hands_focus_to_the_next_then_the_previous() {
        // Middle: the one after it.
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["c", "a"]), &id("b")),
            Some(id("a"))
        );
        // First: the one after it.
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["b", "a"]), &id("c")),
            Some(id("b"))
        );
        // Last: the one before it.
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["c", "b"]), &id("a")),
            Some(id("b"))
        );
    }

    #[test]
    fn nothing_left_is_nowhere() {
        assert_eq!(belongs_on(&ids(&["a"]), &[], &id("a")), None);
    }

    #[test]
    fn the_nearest_survivor_is_used_when_the_next_one_went_too() {
        // "b" and the one after it ("a") both went; "c" is what is left.
        assert_eq!(
            belongs_on(&ids(&["c", "b", "a"]), &ids(&["c"]), &id("b")),
            Some(id("c"))
        );
    }

    #[test]
    fn a_reserved_id_missing_from_the_survey_is_reinstated_at_its_old_place() {
        let old = ids(&["history", "refresh", "palette", "settings"]);
        let survey = ids(&["history", "palette", "settings"]); // "refresh" disabled
        let merged = merge_reserved(&old, &survey, &ids(&["refresh"]));
        assert_eq!(merged, ids(&["history", "refresh", "palette", "settings"]));
    }

    #[test]
    fn a_reserved_id_never_seen_before_is_left_out() {
        let merged = merge_reserved(&[], &ids(&["a"]), &ids(&["refresh"]));
        assert_eq!(merged, ids(&["a"]));
    }

    #[test]
    fn belongs_on_finds_a_reinstated_reserved_id_rather_than_its_neighbour() {
        let old = ids(&["history", "refresh", "palette", "settings"]);
        let merged = merge_reserved(
            &old,
            &ids(&["history", "palette", "settings"]),
            &ids(&["refresh"]),
        );
        assert_eq!(
            belongs_on(&old, &merged, &id("refresh")),
            Some(id("refresh"))
        );
    }

    /// RFC-075 stage 3: `follow`'s own "empty survey → freeze, never
    /// reconcile" guard — confirmed through the real widget, not just
    /// `merge_reserved`/`belongs_on` in isolation, since the thing being
    /// guarded against is `follow` *itself* corrupting `state.focused`
    /// while every `focusable()` call is hidden (the real shape of a
    /// window open over this content, per `snora::Inert`).
    #[test]
    fn an_empty_survey_leaves_remembered_focus_untouched() {
        use crate::test_support::Harness;
        use iced::Element;
        use iced::widget::{Space, column, text};
        use iced_test::core::Size;
        use snora::design::Tokens;

        use super::focus_by_id;

        #[derive(Debug, Clone, PartialEq)]
        enum Msg {
            Pressed(&'static str),
        }

        fn tokens() -> &'static Tokens {
            use std::sync::OnceLock;
            static T: OnceLock<Tokens> = OnceLock::new();
            T.get_or_init(Tokens::light)
        }

        fn three_controls() -> Element<'static, Msg> {
            let make = |label: &'static str, id: &'static str| {
                Element::from(
                    snora::design::control(tokens(), text(label))
                        .height(crate::widget::BUTTON_HEIGHT)
                        .id(Id::new(id))
                        .on_press(Msg::Pressed(label)),
                )
            };
            focus_by_id(
                column![
                    make("One", "one"),
                    make("Two", "two"),
                    make("Three", "three")
                ]
                .spacing(8),
                &[],
            )
        }

        let size = Size::new(400.0, 400.0);
        let mut h = Harness::new(three_controls(), size, None);
        h.tab(false);
        h.tab(false);
        h.redraw();
        assert_eq!(h.focused_id(), Some(Id::new("two")));

        // The window opens: this content becomes `Inert`'s own hidden
        // content — simulated here directly as "draws nothing at all",
        // the same empty survey `Inert` itself produces.
        h = h.rebuilt(focus_by_id(Space::new(), &[]));
        h.redraw();
        assert_eq!(h.focused_id(), None, "nothing to find while hidden");

        // The window closes: the same content comes back.
        h = h.rebuilt(three_controls());
        h.redraw();
        assert_eq!(
            h.focused_id(),
            Some(Id::new("two")),
            "focus must still be on \"Two\" — not reset to \"One\" (the start) or lost \
             entirely — once this content is reachable again"
        );
    }
}
