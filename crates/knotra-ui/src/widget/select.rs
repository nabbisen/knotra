//! Select (dropdown) primitive, on `snora::design::select` (RFC-075
//! stage 3). `options` pairs each value with its own already-localized
//! label — `select_control`'s own doc comment says why this is owned data
//! rather than a borrowed slice.

use snora::design::Tokens;

use super::layout::Element;

/// A value paired with its already-localized label — the shape
/// `snora::design::select` builds internally from the `(T, String)` pairs
/// callers pass in, so `T` itself never needs a `Display` impl.
#[derive(Clone, PartialEq)]
struct LabeledOption<T> {
    value: T,
    label: String,
}

impl<T> std::fmt::Display for LabeledOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// A themed select menu, built on `snora::design::select`. The open/closed
/// state is the **caller's**, not iced's own internal overlay state —
/// `opened` and `on_open_change` thread it through. `options` pairs each
/// value with its already-localized label, built fresh per render (a
/// per-render label, from `state.t(...)`, has no `'static` source to
/// borrow from, so this is owned data rather than a borrowed slice).
///
/// No separate colour function of its own: `snora::design::select` reads
/// its colours straight from `tokens` internally (`snora_widgets::form::
/// Skin::designed`, confirmed by reading it in the registry — a
/// `pub(crate)` type this crate cannot call into, unlike `Control::style`'s
/// open closure), so there is nothing here for a `pub(crate) field_style`-
/// shaped function to wrap. The handoff report's own contrast-coverage
/// section says how this was checked instead.
#[must_use]
pub fn select_control<'a, T, Message>(
    tokens: &Tokens,
    options: Vec<(T, String)>,
    selected: Option<T>,
    opened: bool,
    on_select: impl Fn(T) -> Message + 'a,
    on_open_change: impl Fn(bool) -> Message + 'a,
    id: iced::widget::Id,
) -> Element<'a, Message>
where
    T: PartialEq + Clone + 'a,
    Message: Clone + 'a,
{
    let labeled_options: Vec<LabeledOption<T>> = options
        .into_iter()
        .map(|(value, label)| LabeledOption { value, label })
        .collect();
    let selected = selected.and_then(|value| {
        labeled_options
            .iter()
            .find(|option| option.value == value)
            .cloned()
    });

    snora::design::select(
        tokens,
        labeled_options,
        selected.as_ref(),
        move |option: LabeledOption<T>| on_select(option.value),
        super::layout::APP_DIRECTION,
    )
    .opened(opened)
    .on_open_change(on_open_change)
    .id(id)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Msg {
        Selected(&'static str),
        OpenChanged(bool),
    }

    fn view(
        tokens: &Tokens,
        selected: Option<&'static str>,
        opened: bool,
    ) -> iced::Element<'static, Msg> {
        let tokens = tokens.clone();
        select_control(
            &tokens,
            vec![("a", "Alpha".to_owned()), ("b", "Beta".to_owned())],
            selected,
            opened,
            Msg::Selected,
            Msg::OpenChanged,
            iced::widget::Id::new("select"),
        )
    }

    /// RFC-075 stage 2 §0 (Review 298 F1): `view` alone, as stage 1's own
    /// test built it, answers no ↓/↑ at all — snora answers them through
    /// `MenuKeys`, which only `snora::render` installs
    /// (`snora-0.65.0/src/render.rs:327`, confirmed directly: `MenuKeys::
    /// new(layers)` wraps the whole stack unconditionally). knotra's own
    /// real view goes through `snora::render` too (`view.rs`'s `app_view`),
    /// so a test that skips it is testing a shape the app never draws.
    /// Confirmed failing before this wrapping was added: stage 1's own
    /// `ArrowDown` press against the bare `view()` left `focused_id()`
    /// unchanged, which is the "not checked" limitation Review 298 found.
    fn rendered(
        tokens: &Tokens,
        selected: Option<&'static str>,
        opened: bool,
    ) -> iced::Element<'static, Msg> {
        let layout = snora::AppLayout::new(view(tokens, selected, opened))
            .direction(super::super::layout::APP_DIRECTION);
        snora::render(layout)
    }

    const SIZE: iced_test::core::Size = iced_test::core::Size {
        width: 400.0,
        height: 400.0,
    };

    /// RFC-075 handoff stage 1 §3, row 4, corrected by stage 2 §0 (Review
    /// 298 F1). Three closed-trigger openers (Enter, Space, ↓ — all three,
    /// since `Select::on_open_change`'s own doc comment names exactly these
    /// three plus the trigger's own press); choosing an item by Tab then
    /// Enter, with no close published by `on_select` itself (**"the caller
    /// closes it"** — confirmed directly against `snora_widgets::form::
    /// Select`'s own doc comment, in the registry: "What opening and
    /// closing make: `true` from the trigger, Enter, Space and ↓ ...
    /// `false` from Escape and from the trigger when open" — choosing is in
    /// neither list); and Escape closing with focus back on the trigger.
    ///
    /// Every view here is built through [`rendered`], not the bare
    /// [`view`] stage 1 used — see that helper's own doc comment for why.
    #[test]
    fn opens_on_enter_space_or_down_enter_chooses_escape_closes_and_returns_focus() {
        use crate::test_support::Harness;

        let tokens = crate::theme::KnotraTheme::light().tokens;
        let id = iced::widget::Id::new("select");

        type Opener = (&'static str, fn(&mut Harness<Msg>));
        let openers: [Opener; 3] = [
            ("Enter", Harness::enter),
            ("Space", Harness::space),
            ("ArrowDown", Harness::arrow_down),
        ];
        for (name, press) in openers {
            let mut h = Harness::new(rendered(&tokens, None, false), SIZE, None);
            h.tab(false);
            assert_eq!(
                h.focused_id(),
                Some(id.clone()),
                "{name}: Tab reaches the trigger"
            );
            press(&mut h);
            assert_eq!(
                h.messages(),
                [Msg::OpenChanged(true)],
                "{name} on the closed trigger must open it"
            );
        }

        // Tab into the open dropdown, Enter chooses — no close published.
        let mut h = Harness::new(rendered(&tokens, None, false), SIZE, None);
        h.tab(false);
        h.enter();
        let mut h = h.rebuilt(rendered(&tokens, None, true));
        h.tab(false); // "Alpha"
        h.tab(false); // "Beta"
        h.enter();
        assert_eq!(
            h.messages(),
            [Msg::OpenChanged(true), Msg::Selected("b")],
            "Enter on the second item chooses it, and on_select alone \
             publishes no close"
        );

        // Escape: closes, and returns focus to the trigger.
        let mut h = Harness::new(rendered(&tokens, Some("a"), true), SIZE, None);
        h.tab(false);
        h.escape();
        assert_eq!(h.messages(), [Msg::OpenChanged(false)]);
        let mut h = h.rebuilt(rendered(&tokens, Some("a"), false));
        assert_eq!(
            h.focused_id(),
            Some(id),
            "Escape must return focus to the trigger, not leave nothing focused"
        );
    }

    /// RFC-075 stage 2 §0 (Review 298 F1): snora answers ↓ ↑ in an open
    /// select's list through `MenuKeys` — confirmed by reading
    /// `snora-interaction-0.65.0/src/menu_keys.rs` directly: it moves real
    /// `operation::Focusable` state (rules 1-2), captures the key before
    /// the tree sees it (rule 4), and never publishes a message itself
    /// (rule 5, "moving never presses") — so the only way to observe where
    /// it landed from outside is to press Enter and read which item was
    /// chosen. The list's own items carry no `Id` of their own (built from
    /// `LabeledOption`, never `.id(...)`), so `focused_id()` cannot name
    /// them the way it names the trigger; choosing is the honest signal.
    ///
    /// Three items ("a"/"b"/"c"), not the two the rest of this file's tests
    /// use: with only two, wrapping from the first to the last and simply
    /// moving to the next land on the same item, which would not tell
    /// "wraps" apart from "moves". **Confirmed failing before stage 2 §0's
    /// fix**: built against the bare [`view`]-equivalent (no
    /// `snora::render`), the first case below published nothing at all —
    /// nothing answers ↓ without `MenuKeys`, so Enter afterward chose
    /// whatever `on_select` was last given, not "a".
    #[test]
    fn arrow_down_focuses_and_moves_in_the_open_list_arrow_up_wraps_to_the_last() {
        use crate::test_support::Harness;

        fn three_item_select(opened: bool) -> iced::Element<'static, Msg> {
            let tokens = crate::theme::KnotraTheme::light().tokens;
            let content = select_control(
                &tokens,
                vec![
                    ("a", "Alpha".to_owned()),
                    ("b", "Beta".to_owned()),
                    ("c", "Gamma".to_owned()),
                ],
                None,
                opened,
                Msg::Selected,
                Msg::OpenChanged,
                iced::widget::Id::new("select"),
            );
            snora::render(
                snora::AppLayout::new(content).direction(super::super::layout::APP_DIRECTION),
            )
        }

        // ↓ with nothing focused in the open list starts at the first item.
        let mut h = Harness::new(three_item_select(true), SIZE, None);
        h.arrow_down();
        h.enter();
        assert_eq!(
            h.messages(),
            [Msg::Selected("a")],
            "the first ↓ in an open list must land on the first item"
        );

        // A second ↓ moves to the next item.
        let mut h = Harness::new(three_item_select(true), SIZE, None);
        h.arrow_down();
        h.arrow_down();
        h.enter();
        assert_eq!(
            h.messages(),
            [Msg::Selected("b")],
            "a second ↓ must move to the item after the first"
        );

        // ↑ on the first item wraps to the last, skipping the middle one —
        // the three-item list is what makes this distinguishable from a
        // plain move.
        let mut h = Harness::new(three_item_select(true), SIZE, None);
        h.arrow_down(); // -> "a", the first item
        h.arrow_up(); // wraps -> "c", the last item
        h.enter();
        assert_eq!(
            h.messages(),
            [Msg::Selected("c")],
            "↑ on the first item must wrap to the last, not move to nothing"
        );
    }
}
