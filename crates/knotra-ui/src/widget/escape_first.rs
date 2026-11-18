//! Task 111 — a wrapper that sees Escape before its content does.
//!
//! iced 0.14's focused `text_input` **captures** a plain Escape press
//! itself — it unfocuses and calls `shell.capture_event()`
//! (`iced_widget` 0.14.2 `text_input.rs:1235`) — before anything above it
//! in the tree (snora's own `on_close_modals`/`OnEscape`, or knotra's
//! `keyboard::listen()` subscription, which never runs on a captured
//! event) gets a chance to see the key at all. So in any window whose
//! cursor is in a field (the command palette, Add project, Rename/New
//! workspace), the first Escape press only removed the cursor, and the
//! window needed a second press to actually close (Review 281 §2).
//!
//! `EscapeFirst` fixes this by placing itself as the outermost widget
//! around a dialog/sheet's content — the same position the owner's
//! `view.rs` already wraps each layer's content at. Being outermost, its
//! own `update` runs *before* it delegates into the content tree at all,
//! so it can act on a plain Escape press itself and never call the
//! content's own `update` for that event — the content, and whatever it
//! would otherwise have captured the key for, never sees it.
//!
//! **An input-method composition keeps its Escape.** While the user is
//! mid-composition (a non-empty IME pre-edit — e.g. typing Japanese),
//! Escape belongs to the input method, which is what cancels it; this
//! wrapper must not steal that press. It tracks `Event::InputMethod`'s own
//! `Preedit`/`Commit`/`Closed` variants as they pass through on their way
//! to the content (every event not captured by this widget is still
//! forwarded, which is how the content's own IME integration keeps
//! working at all) and only acts on Escape while the tracked pre-edit is
//! empty.

use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, input_method, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Theme, Vector, keyboard};

/// Wraps `content` so a plain, non-repeat Escape press publishes
/// `on_escape` and is captured before `content` ever receives it — unless
/// an input-method composition is in progress, in which case Escape is let
/// through unchanged (this module's own doc comment explains why).
#[must_use]
pub fn escape_first<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_escape: Message,
) -> Element<'a, Message> {
    Element::new(EscapeFirst {
        content: content.into(),
        on_escape,
    })
}

struct EscapeFirst<'a, Message> {
    content: Element<'a, Message>,
    on_escape: Message,
}

/// Whether the content currently holds a non-empty IME pre-edit — the one
/// thing this widget needs to remember between events.
#[derive(Default)]
struct State {
    preedit_nonempty: bool,
}

impl<Message: Clone> iced::advanced::Widget<Message, Theme, iced::Renderer>
    for EscapeFirst<'_, Message>
{
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
        self.content.as_widget().size()
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
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let state: &mut State = tree.state.downcast_mut();

        if let Event::InputMethod(ime_event) = event {
            match ime_event {
                input_method::Event::Preedit(text, _) => {
                    state.preedit_nonempty = !text.is_empty();
                }
                input_method::Event::Commit(_) | input_method::Event::Closed => {
                    state.preedit_nonempty = false;
                }
                input_method::Event::Opened => {}
            }
        }

        let is_plain_escape_press = matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                repeat: false,
                ..
            })
        );

        if is_plain_escape_press && !state.preedit_nonempty {
            shell.publish(self.on_escape.clone());
            shell.capture_event();
            return;
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
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
