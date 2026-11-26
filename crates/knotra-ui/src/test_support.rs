//! RFC-075 handoff stage 1 §1 — a keyboard test harness, driven through
//! `iced_runtime`'s `UserInterface`, the same level snora's own
//! `tests/modal_keyboard.rs` (registry, `snora-0.65.0/tests/`) and
//! `tests/sidebar_keyboard.rs` drive theirs through. `iced_test`'s
//! `Simulator` cannot run the widget `Operation`s `snora::keyboard::traverse`
//! returns — confirmed directly, by reading `Simulator`'s own public API
//! (`find`/`click`, both `Selector`-bound, the only methods that run an
//! `Operation` for a caller) and re-reading `sidebar_keyboard.rs`'s own doc
//! comment's claim to the same effect (RFC-075 stage 0, answer f) — so a
//! harness that needs to send Tab through the real router has to sit one
//! level lower, as this one does.
//!
//! **Lives here, in `knotra-ui`, behind the `test-support` feature** (not
//! `knotra-app`, and not `#[cfg(test)]` alone): `knotra-ui`'s own primitive
//! tests need it first (this stage), and `knotra-app`'s later-stage view
//! tests need the exact same driving code, not a second copy that could
//! drift from it. A `#[cfg(test)]`-only module compiles into neither
//! crate's dependents, so `knotra-app` could never reach it that way —
//! `test-support` un-gates it as an ordinary (if feature-gated) public
//! item instead, and `knotra-app`'s `[dev-dependencies]` turns the feature
//! on for its own test builds only (`Cargo.toml`: `knotra-ui = { workspace
//! = true, features = ["test-support"] }`), never for the shipped binary.
//!
//! `#[cfg(any(test, feature = "test-support"))]`: this crate's own tests
//! (via the plain `[dev-dependencies]` copy of `iced_test`) and an external
//! crate's tests (via the optional, feature-gated copy) both compile it,
//! and neither a normal build of this crate nor one of its dependents ever
//! does.

use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::{Theme, mouse};
use iced_test::core::clipboard;
use iced_test::core::event::Status;
use iced_test::core::renderer::Headless;
use iced_test::core::widget::operation::{self, Outcome};
use iced_test::core::widget::{Id, Operation};
use iced_test::core::{Element, Event, Font, Pixels, Rectangle, Size};
use iced_test::runtime::user_interface::{self, UserInterface};

/// Loads knotra's icon font into the renderer's font system — one of the
/// two gaps `crates/knotra-app/src/headless.rs`'s own doc comment names for
/// `Simulator`'s default `Settings`, closed the same way here: without it,
/// an icon glyph a view under test draws measures as whatever font's
/// missing-glyph box, not the icon's own advance width. `None` for the
/// three harness tests below (a plain snora page draws no icon).
fn load_icon_font(font_bytes: Option<&'static [u8]>) {
    use iced_test::renderer::graphics::text::font_system;
    if let Some(bytes) = font_bytes {
        font_system()
            .write()
            .expect("write to the font system")
            .load_font(std::borrow::Cow::Borrowed(bytes));
    }
}

/// Drives a view through `iced_runtime`'s `UserInterface` — key presses,
/// `snora::keyboard::traverse`'s own operations, and what either one
/// publishes or leaves focused. See the module doc for why this sits below
/// `iced_test::Simulator` rather than wrapping it.
pub struct Harness<Message> {
    ui: UserInterface<'static, Message, Theme, iced::Renderer>,
    renderer: iced::Renderer,
    messages: Vec<Message>,
    cursor: mouse::Cursor,
    size: Size,
}

impl<Message: 'static> Harness<Message> {
    /// Builds a headless renderer at `logical_size` and the given `root`
    /// element on it, with "Noto Sans" as the default face — the other of
    /// `headless.rs`'s two gaps, closed the same way: this machine's (and
    /// CI's Linux runner's, Task 108 A1 §4) actual system sans-serif, not
    /// `iced_test`'s own "Fira Sans" fallback. `font_bytes`: see
    /// [`load_icon_font`].
    pub fn new(
        root: Element<'static, Message, Theme, iced::Renderer>,
        logical_size: impl Into<Size>,
        font_bytes: Option<&'static [u8]>,
    ) -> Self {
        load_icon_font(font_bytes);
        let size = logical_size.into();
        // Task 123: "tiny-skia", never wgpu's default — wgpu's own
        // `Headless::new` starts a real Vulkan instance and touches the
        // host's display even when the caller never asked for a window,
        // which many test threads doing this at once segfaults (Review
        // 310 F2). `Some("tiny-skia")` makes wgpu's own impl return `None`
        // immediately (its own `backend.is_some_and(|b| b != "wgpu")`
        // check) and the fallback renderer moves straight to tiny-skia,
        // touching neither.
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::with_name("Noto Sans"),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("the default headless renderer starts");
        let ui = UserInterface::build(root, size, user_interface::Cache::default(), &mut renderer);
        Self {
            ui,
            renderer,
            messages: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            size,
        }
    }

    /// Every message published since the harness was built, in order.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// What an application's `view()` does after `update()`: a fresh tree
    /// of widgets around the same underlying widget state — carried across
    /// by `self.ui.into_cache()`, the same way `snora`'s own
    /// `tests/modal_keyboard.rs::Rig::rebuilt_with` does it. Without this,
    /// a test that presses a key, rebuilds the view to reflect the message
    /// it published, and checks what is focused **afterward** would always
    /// see "nothing" — a brand new `UserInterface` has no memory of the
    /// one before it. Consumes `self` and returns the rebuilt harness, so
    /// a call site reads `h = h.rebuilt(new_view(...))`, never two live
    /// copies of the same interface at once.
    pub fn rebuilt(self, root: Element<'static, Message, Theme, iced::Renderer>) -> Self {
        let mut renderer = self.renderer;
        let ui = UserInterface::build(root, self.size, self.ui.into_cache(), &mut renderer);
        Self {
            ui,
            renderer,
            messages: self.messages,
            cursor: self.cursor,
            size: self.size,
        }
    }

    fn deliver(&mut self, events: &[Event]) -> Vec<Status> {
        self.ui
            .update(
                events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut self.messages,
            )
            .1
    }

    /// A single named key, no modifiers — Enter, Space, Escape, an arrow.
    pub fn key(&mut self, key: Named) -> Vec<Status> {
        self.deliver(&[iced_test::simulator::press_key(key, None)])
    }

    pub fn enter(&mut self) {
        let _ = self.key(Named::Enter);
    }

    pub fn space(&mut self) {
        let _ = self.key(Named::Space);
    }

    pub fn escape(&mut self) {
        let _ = self.key(Named::Escape);
    }

    pub fn arrow_down(&mut self) {
        let _ = self.key(Named::ArrowDown);
    }

    pub fn arrow_up(&mut self) {
        let _ = self.key(Named::ArrowUp);
    }

    /// Delivers one `RedrawRequested` — the event every real frame sends
    /// before drawing (`iced_winit`'s own shell loop), and the only event
    /// `knotra_ui::widget::keyed::FollowsId` acts on to reconcile focus.
    /// A plain [`rebuilt`](Self::rebuilt)/[`focused_id`](Self::focused_id)
    /// call never delivers this on its own — it only rebuilds the tree or
    /// queries what is *already* really focused, neither of which runs a
    /// `FollowsId` wrapper's own `follow()` step.
    pub fn redraw(&mut self) {
        let _ = self.deliver(&[Event::Window(
            iced_test::core::window::Event::RedrawRequested(std::time::Instant::now()),
        )]);
    }

    /// Types `text`, one character at a time, as a real keyboard would —
    /// `iced_test::simulator::typewrite`, the same sequence `iced_test`
    /// itself builds for `Simulator::typewrite`.
    pub fn type_text(&mut self, text: &str) {
        let events: Vec<Event> = iced_test::simulator::typewrite(text).collect();
        let _ = self.deliver(&events);
    }

    /// Tab, or Shift+Tab, through **`snora::keyboard::traverse`** — never
    /// iced's own `focus_next`/`focus_previous` directly, which knotra's
    /// real `update` never calls either (RFC-075 D1): the application
    /// routes every Tab through `traverse`, so a harness that drove
    /// `focus_next` instead would pass a case the real app does not reach.
    /// Builds the task `traverse` returns, takes the one widget operation
    /// out of it, and runs that operation on this interface — the same
    /// take-apart `snora`'s own `tests/modal_keyboard.rs::Rig::traverse_tab`
    /// does, confirmed by reading it directly in the registry.
    pub fn tab(&mut self, shift: bool) {
        use iced::futures::StreamExt;

        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let task =
            snora::keyboard::traverse::<Message>(iced::keyboard::Key::Named(Named::Tab), modifiers)
                .expect("Tab is claimed by snora::keyboard::traverse");
        let mut stream =
            iced_test::runtime::task::into_stream(task).expect("a task with work in it");
        let Some(iced_test::runtime::Action::Widget(op)) =
            iced::futures::executor::block_on(stream.next())
        else {
            panic!("snora::keyboard::traverse did not produce a widget operation");
        };
        self.run_operation(op);
    }

    /// Runs one widget operation, as taken out of an `Action::Widget`, to
    /// completion against this harness's own `UserInterface` — the driving
    /// loop [`tab`](Self::tab) uses for `snora::keyboard::traverse`'s own
    /// operation, exposed so a `Task`-returning helper built on more than
    /// one operation (RFC-075 stage 2's `knotra_ui::widget::keyboard`) can
    /// be driven through the same `UserInterface` one `Action::Widget` at a
    /// time, by whatever takes its `Task` apart — the same take-apart
    /// `snora`'s own `tests/modal_keyboard.rs::Rig::traverse_tab` does for
    /// a single operation.
    pub fn run_operation(&mut self, mut op: Box<dyn Operation<()>>) {
        loop {
            self.ui
                .operate(&self.renderer, &mut operation::black_box(op.as_mut()));
            match op.finish() {
                Outcome::None | Outcome::Some(()) => return,
                Outcome::Chain(next) => op = next,
            }
        }
    }

    /// Lets a test name the comparison it means: `tab(false)`/`tab(true)`
    /// read ambiguous at a call site on their own.
    pub fn shift_tab(&mut self) {
        self.tab(true);
    }

    /// The focused control's own `.id()`, or `None` if nothing is focused —
    /// the same `Operation::focusable` callback snora's own test `Rig::
    /// targets` uses, kept to "the first (and, in a well-formed tree, only)
    /// focused target" rather than collecting every target, since every
    /// caller here only ever wants to know what is focused.
    pub fn focused_id(&mut self) -> Option<Id> {
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
        let mut found = FindFocused(None);
        self.ui.operate(&self.renderer, &mut found);
        found.0
    }

    pub fn logical_size(&self) -> Size {
        self.size
    }

    /// Draws the current tree at scale 1 and reads the pixels back, RGBA,
    /// row-major — `knotra_ui::theme::KnotraTheme`'s own light/dark/
    /// high-contrast presets each derive an `iced::Theme` (`snora_style::
    /// theme::theme`), so a caller passes that. The same two-call shape
    /// (`draw` then `Headless::screenshot`) `snora`'s own test `Rig::frame`
    /// uses. Returns `(rgba, width, height)` — physical pixels, equal to
    /// the logical size at scale 1.
    pub fn screenshot(&mut self, theme: &Theme) -> (Vec<u8>, u32, u32) {
        use iced::theme::Base;
        let base = theme.base();
        self.ui.draw(
            &mut self.renderer,
            theme,
            &iced_test::core::renderer::Style {
                text_color: base.text_color,
            },
            self.cursor,
        );
        let physical = Size::new(self.size.width as u32, self.size.height as u32);
        let rgba = Headless::screenshot(&mut self.renderer, physical, 1.0, base.background_color);
        (rgba, physical.width, physical.height)
    }
}

/// RFC-075 handoff stage 1 §3, row 2: compares two same-size RGBA buffers
/// pixel by pixel and reports every differing one — used to check that a
/// new, `Control`-based primitive draws **identically** to its old
/// `button`-based twin at rest. `Ok(())` only on a byte-for-byte match (the
/// task's own "expect identical pixels at rest", not a tolerance): this
/// crate's own `label_overflow_guard.rs`-equivalent precedent
/// (`crates/knotra-app`) already treats "approximately the same" as the
/// wrong bar for text rendered by the same font at a logical size with no
/// shaping variance between two calls — the same reasoning applies here,
/// to every pixel, not only a label's measured width.
pub fn pixel_diff(
    a: &[u8],
    a_size: (u32, u32),
    b: &[u8],
    b_size: (u32, u32),
) -> Result<(), String> {
    if a_size != b_size {
        return Err(format!(
            "size differs: {a_size:?} vs {b_size:?} — not a pixel comparison at all"
        ));
    }
    if a.len() != b.len() {
        return Err(format!(
            "buffer length differs: {} vs {} bytes for the same {a_size:?}",
            a.len(),
            b.len()
        ));
    }
    let (width, _height) = a_size;
    let mut differing = Vec::new();
    for (i, (pa, pb)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        if pa != pb {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            differing.push((x, y, pa.to_vec(), pb.to_vec()));
        }
    }
    if differing.is_empty() {
        return Ok(());
    }
    let shown: Vec<String> = differing
        .iter()
        .take(10)
        .map(|(x, y, pa, pb)| format!("({x},{y}): {pa:?} vs {pb:?}"))
        .collect();
    Err(format!(
        "{} of {} pixels differ (showing up to 10): {}",
        differing.len(),
        (a.len() / 4),
        shown.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::Harness;
    use iced::Element;
    use iced::widget::{column, text};
    use iced_test::core::Size;
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

    /// Three `snora::design::control`s, the third disabled (`on_press_maybe
    /// (None)`) — a plain page with nothing else to confuse Tab's order,
    /// each given its own id so [`Harness::focused_id`] can name which one
    /// it landed on.
    fn three_controls(disable_third: bool) -> Element<'static, Msg> {
        let make = |label: &'static str, id: &'static str, enabled: bool| {
            let control = snora::design::control(tokens(), text(label))
                .id(iced::widget::Id::new(id))
                .on_press_maybe(enabled.then_some(Msg::Pressed(label)));
            iced::Element::from(control)
        };
        column![
            make("One", "one", true),
            make("Two", "two", true),
            make("Three", "three", !disable_third),
        ]
        .spacing(8)
        .into()
    }

    const SIZE: Size = Size {
        width: 400.0,
        height: 400.0,
    };

    #[test]
    fn tab_moves_through_three_controls_in_order() {
        let mut h = Harness::new(three_controls(false), SIZE, None);
        assert_eq!(h.focused_id(), None, "nothing focused before any Tab");

        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("one")));
        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("two")));
        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("three")));
    }

    #[test]
    fn shift_tab_goes_back() {
        let mut h = Harness::new(three_controls(false), SIZE, None);
        h.tab(false);
        h.tab(false);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("three")));

        h.shift_tab();
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("two")));
        h.shift_tab();
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("one")));
    }

    #[test]
    fn a_disabled_control_is_skipped() {
        let mut h = Harness::new(three_controls(true), SIZE, None);
        h.tab(false);
        assert_eq!(h.focused_id(), Some(iced::widget::Id::new("one")));
        h.tab(false);
        assert_eq!(
            h.focused_id(),
            Some(iced::widget::Id::new("two")),
            "disabled \"Three\" must never receive focus"
        );
        // Past the last enabled control, Tab leaves the group (RFC-075 D4
        // a, stage 0's own finding) — nothing focused, not "Three".
        h.tab(false);
        assert_eq!(h.focused_id(), None);
    }
}
