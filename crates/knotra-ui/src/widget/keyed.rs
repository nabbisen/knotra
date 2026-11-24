//! RFC-075 stage 2 Amendment A1 §1: focus follows the control it is on, by
//! its own [`iced::widget::Id`] — not by its place in the tree, which is
//! how iced actually keeps a widget's state (focus included). Review 299
//! F1 found the gap directly: a background refresh that moves a project
//! between the dashboard's own tiers can hand keyboard focus to a
//! **different** project's row, with no key pressed, because the rebuilt
//! tree's position that used to be "Alpha's name" is now "Beta's name",
//! and iced's own diffing keeps whatever was focused at that position.
//!
//! **The shape is snora's own**, read directly rather than guessed:
//! `snora::Keyed` (`snora-interaction-0.65.0/src/keyed.rs`) does exactly
//! this for its toast stack, keyed by a `u64`. It is `pub`, but its own
//! module doc says "it is not for applications", and the umbrella `snora`
//! crate does not re-export it — only `snora-interaction` does — so this
//! is knotra's own copy of the same algorithm, not a dependency on an
//! internal crate its own maintainers did not mean to expose this way.
//! The two private operations it drives its content with
//! (`crate::inert::{Refocus, Unfocus}`) are a few lines each,
//! reimplemented here rather than reached for.
//!
//! **Where it wraps, and why there**: around the dashboard's own row
//! content (`view::dashboard::view_body`'s output) — the one region whose
//! set of focus targets actually reorders (a refresh, a filter, a sort, a
//! group, a collapse, selection mode, a project added or removed). The
//! shell and the toolbar above it never reorder their own controls, so
//! they are outside it; the two existing stage-2 fixes
//! (`ArrangeToggled`/`SelectionMessage::ModeEntered` re-asserting focus
//! after their own panel/selection-bar appears) stay for exactly that
//! reason — they are *toolbar* controls, a different region this wrapper
//! does not reach, so neither fix becomes unnecessary here.
//!
//! **One mechanism, not one per message**: it hooks
//! `Event::Window(window::Event::RedrawRequested)`, which fires once per
//! frame regardless of which message caused the rebuild — a refresh, a
//! filter, a sort, a collapse, selection mode, or a project add/remove all
//! reach it the same way, with no call site needing its own fix.
//!
//! **Its stated limit does not carry over, read directly rather than
//! assumed**: `keyed.rs`'s own doc warns that pressing Tab in the very
//! frame the set of ids changes puts focus back on the id last *seen*
//! focused (from the previous redraw), not the one Tab chose — because
//! its own bookkeeping (`remember`) only updates on a raw `KeyPressed`
//! event. Knotra's own Tab/arrow moves never reach a widget as a raw key
//! at all (RFC-075 D1: they are routed through `app::update` and applied
//! as a `Task`'s own `operate()` calls, such as `operation::focus(id)`),
//! so that hook alone would never see them. [`FollowsId::resync`] closes
//! this more generally: it keeps `state.focused` true to **whatever is
//! actually focused** after *any* `operate()` call reaches this content
//! — Tab's own move among them — not only after a raw key press, so a
//! Task-driven move (Tab's own, or an existing fix's own
//! `operation::focus` to a sibling, such as "Select"'s) is already
//! reflected before the next redraw ever has to guess.
//! `keyboard_acceptance::focus_follows_its_id`'s own tests confirm both:
//! a same-step Tab move stands, and the existing Select/Arrange fixes are
//! not undone by this one.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Theme, Vector, window};

/// Wraps `content` so real keyboard focus follows the control it is on by
/// its own [`Id`], across a rebuild that reorders, adds, or removes the
/// focus targets inside it — see the module doc for the mechanism and
/// where it wraps. `ids` must be every focus target `content` draws, one
/// key per target, in the order they appear (Tab order): with any other
/// count, this stands aside and does nothing for that one redraw, the
/// same as `snora::Keyed` does, rather than guess at a mismatched tree.
pub fn focus_by_key<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    ids: &[Id],
) -> Element<'a, Message> {
    FollowsId {
        content: content.into(),
        keys: ids.iter().map(hash_id).collect(),
    }
    .into()
}

/// `Id` has no public integer of its own; this is deterministic for a
/// fixed `Id` (it derives `Hash`, and `DefaultHasher`'s initial state is
/// fixed, not randomized per run the way `HashMap`'s `RandomState` is), so
/// the same control hashes to the same key on every redraw. A collision
/// between two of a page's few dozen ids is astronomically unlikely for a
/// 64-bit digest, and would only mean this stands aside for that one case
/// (its own documented behaviour for a repeated key) — never a wrong
/// focus.
fn hash_id(id: &Id) -> u64 {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

#[derive(Default)]
struct State {
    /// The keys as they were at the last redraw.
    keys: Vec<u64>,
    /// The key whose item held focus at the last redraw.
    focused: Option<u64>,
}

struct FollowsId<'a, Message> {
    content: Element<'a, Message>,
    keys: Vec<u64>,
}

/// Which focus targets are focused, in Tab order — the same one-pass
/// survey `snora::Keyed`'s own `Flags` takes.
#[derive(Default)]
struct Flags(Vec<bool>);

impl Operation for Flags {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0.push(state.is_focused());
    }
}

/// Takes focus from every target, so none is left focused once another
/// is given it below.
struct Unfocus;

impl Operation for Unfocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        state.unfocus();
    }
}

/// Gives focus to the target at one ordinal position among this content's
/// own focus targets.
struct Refocus {
    at: usize,
    seen: usize,
}

impl Operation for Refocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        if self.seen == self.at {
            state.focus();
        }
        self.seen += 1;
    }
}

/// Where focus belongs after the keys changed: the item it was on, else
/// its neighbour that is still there (next in the old order, else
/// previous), else nowhere — RFC-120 R-1's own rule, keyed here by this
/// page's own control ids instead of a toast's.
fn belongs_on(old: &[u64], now: &[u64], focused: u64) -> Option<usize> {
    if let Some(at) = now.iter().position(|k| *k == focused) {
        return Some(at);
    }
    let was = old.iter().position(|k| *k == focused)?;
    let survivor = |k: &&u64| now.contains(*k);
    let neighbour = old[was + 1..]
        .iter()
        .find(survivor)
        .or_else(|| old[..was].iter().rev().find(survivor))?;
    now.iter().position(|k| k == neighbour)
}

impl<Message> FollowsId<'_, Message> {
    /// Keeps `state.focused` true to what is **actually, currently**
    /// focused inside this content, after any `Operation` has just run
    /// through it (not only Tab/Shift+Tab's own — any `widget::operate`
    /// call reaches here, the same single traversal that, say,
    /// `operation::focus(some_sibling_id)` makes across the *whole* tree
    /// to move focus elsewhere and unfocus everything else on the way).
    ///
    /// Without this, a message that explicitly, deliberately moves real
    /// focus to a control **outside** this content (the Arrange/Select
    /// fixes RFC-075 stage 2 already ships) while *also* changing this
    /// content's own key set in the same step (entering selection mode
    /// does both at once: it reveals every row's checkbox, and re-asserts
    /// focus on "Select") would leave `state.focused` holding a stale key
    /// from the content's own last redraw — and the very next redraw's
    /// [`follow`](Self::follow), seeing the key set changed and nothing
    /// of its own currently focused, would "reclaim" focus back onto
    /// that stale key's new position, undoing the deliberate move. This
    /// runs on every `operate()` specifically so it observes the
    /// unfocus-everything-else side effect `operation::focus` itself
    /// causes, in the same pass that causes it — not a frame later.
    fn resync(&mut self, tree: &mut Tree, layout: layout::Layout<'_>, renderer: &iced::Renderer) {
        let mut flags = Flags::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut flags);
        if flags.0.len() == self.keys.len() {
            let state = tree.state.downcast_mut::<State>();
            state.focused = flags.0.iter().position(|f| *f).map(|at| self.keys[at]);
        }
    }

    fn follow(&mut self, tree: &mut Tree, layout: layout::Layout<'_>, renderer: &iced::Renderer) {
        let mut flags = Flags::default();
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut flags);
        let state = tree.state.downcast_mut::<State>();

        // One target per key, or this does not know what it is looking at.
        if flags.0.len() != self.keys.len() {
            state.keys = self.keys.clone();
            state.focused = None;
            return;
        }

        let now = flags.0.iter().position(|f| *f);
        let want = if state.keys == self.keys {
            // Nothing changed shape: focus moved only if the reader moved it.
            now
        } else if let Some(focused) = state.focused {
            belongs_on(&state.keys, &self.keys, focused)
        } else {
            now
        };

        if want != now {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut Unfocus,
            );
            if let Some(at) = want {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut Refocus { at, seen: 0 },
                );
            }
        }

        let state = tree.state.downcast_mut::<State>();
        state.keys = self.keys.clone();
        state.focused = want.map(|at| self.keys[at]);
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
        // See `resync`'s own doc comment: an operation already forwarded
        // above (an explicit `operation::focus(elsewhere)`, among others)
        // may have just changed what, if anything, is focused in here —
        // keep `state.focused` honest about it immediately, not only on
        // the next redraw.
        self.resync(tree, layout, renderer);
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
    use super::belongs_on;

    #[test]
    fn focus_stays_on_its_item_when_another_goes() {
        assert_eq!(belongs_on(&[3, 2, 1], &[2, 1], 2), Some(0));
        assert_eq!(belongs_on(&[3, 2, 1], &[3, 1], 1), Some(1));
    }

    #[test]
    fn a_removed_item_hands_focus_to_the_next_then_the_previous() {
        assert_eq!(belongs_on(&[3, 2, 1], &[3, 1], 2), Some(1));
        assert_eq!(belongs_on(&[3, 2, 1], &[2, 1], 3), Some(0));
        assert_eq!(belongs_on(&[3, 2, 1], &[3, 2], 1), Some(1));
    }

    #[test]
    fn nothing_left_is_nowhere() {
        assert_eq!(belongs_on(&[1], &[], 1), None);
    }

    #[test]
    fn the_nearest_survivor_is_used_when_the_next_one_went_too() {
        assert_eq!(belongs_on(&[3, 2, 1], &[3], 2), Some(0));
    }
}
