//! Select (dropdown) primitive, built from `KnotraTheme` (RFC-035 Handoff
//! 019 §7.2). `snora` ships no select styling — `snora-widgets-0.25.2`'s
//! `design/` module has `button`, `card`, `chip`, `notice`, `progress`, no
//! `select` — so this is written directly from the RFC-033 D7 roles:
//! surface, border, text, and accent for the open/selected row.
//!
//! The focus ring reuses [`super::ring::ring_color_for`] — the same
//! RFC-036 Stage 6 decision `with_focus_ring` (`buttons.rs`) applies to
//! buttons — rather than a second implementation. `with_focus_ring` itself
//! cannot be called directly: it is typed to `iced::widget::button::Style`,
//! and `pick_list_widget::Style` / `menu::Style` are different shapes (no
//! `shadow` or `snap` field, an extra `placeholder_color`/`handle_color` on
//! the field, and a `selected_background`/`selected_text_color` pair on the
//! menu). `ring_color_for` only needs a background and returns a colour, so
//! it composes with any of them.
//!
//! `pick_list`'s `options` take owned `(T, String)` pairs, not `&'a [T]`
//! (RFC-035 Handoff 022 commit 2a, `100`'s ruling on the blocker reported
//! against Stage 2 §7.3). The original `&'a [T]` signature narrowed iced's
//! own `PickList::new`, which is generic over `L: Borrow<[T]> + 'a`,
//! including an owned `Vec`. That narrowing made the wrapper unable to
//! express its own normal call pattern: a per-render option list whose
//! labels come from the caller's current locale via `state.t(...)`, which
//! has no `'static` source to borrow from. Widening to `impl Borrow<[T]> +
//! 'a` alone would have unblocked that, but every call site would still
//! need `T: ToString`, hand-rolling a `Display` wrapper just to pair a
//! domain value with its label, for what RFC-033 D4 makes the universal
//! case: every select in this app pairs a value with a locale-derived
//! label, not an edge case. So the pairing lives here, once, instead.

use iced::widget::overlay::menu;
use iced::widget::pick_list as pick_list_widget;
use snora::design::Tokens;

use super::layout::Element;
use super::ring::ring_color_for;

fn to_iced(color: snora::design::Color) -> iced::Color {
    snora::design::style::color::to_iced_color(color)
}

/// A value paired with its already-localized label — the shape `pick_list`
/// builds internally from the `(T, String)` pairs callers pass in, so `T`
/// itself never needs a `Display` impl.
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

/// A themed select menu, sized the same way [`super::field::styled_text_
/// input`] sizes a field: the closed control is exactly `BUTTON_HEIGHT`
/// tall, its text vertically centred, through the same `(BUTTON_HEIGHT -
/// line_height_px) / 2.0` symmetric vertical padding, `body` size and
/// line height.
///
/// Amendment A5 §3: this is the one `pick_list` constructor
/// `knotra-ui`/`knotra-app` call — `styled_text_input`'s own sibling in
/// name as well as shape, not a second, differently-named wrapper beside
/// it — so every select in the app (nine call sites today: both Arrange-
/// menu selects in `toolbar.rs`, and the language/theme/zoom/refresh-
/// interval/simultaneous-reads/editor-default-pick/log-size/fs-debounce
/// selects in `settings.rs`) is sized this way with no separate call.
///
/// **The open list's own row height is not fully this function's to set**:
/// `iced_widget::pick_list::PickList::overlay()` forwards this builder's
/// `padding` and `text_size` into the `Menu` overlay it constructs, but
/// not `text_line_height` — iced has no public seam for that, confirmed by
/// reading `pick_list.rs`'s own `overlay()`. The open row's line height is
/// therefore iced's own default, `LineHeight::Relative(1.3)`, not the
/// `body` token's `1.4` the closed field uses — at `body`'s `16px` text
/// size that is `16 × 1.3 = 20.8px`, plus the forwarded padding
/// (`(44 − 16 × 1.4) / 2 ≈ 10.8px` top and bottom) = **42.4px**, still
/// comfortably at least `SMALL_BUTTON_HEIGHT` (36) without any further
/// setting.
///
/// `options` pairs each value with its already-localized label, built fresh
/// per render — see the module doc for why this is owned data rather than a
/// borrowed slice.
#[must_use]
pub fn styled_pick_list<'a, T, Message>(
    tokens: &Tokens,
    options: Vec<(T, String)>,
    selected: Option<T>,
    is_focused: bool,
    on_select: impl Fn(T) -> Message + 'a,
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

    let t_field = tokens.clone();
    let t_menu = tokens.clone();

    let line_height_px = tokens.typography.body.size * tokens.typography.body.line_height;

    pick_list_widget::PickList::new(
        labeled_options,
        selected,
        move |option: LabeledOption<T>| on_select(option.value),
    )
    .padding([(super::layout::BUTTON_HEIGHT - line_height_px) / 2.0, 12.0])
    .text_size(snora::design::style::text::body_size(tokens))
    .text_line_height(snora::design::style::text::body_line_height(tokens))
    .style(move |_theme, status| field_style(&t_field, status, is_focused))
    .menu_style(move |_theme| menu_style(&t_menu, is_focused))
    .into()
}

/// RFC-075 D1/Stage 1: [`styled_pick_list`]'s own twin on
/// `snora::design::select` — see `buttons.rs`'s `_control` section doc for
/// the naming reasoning. Unlike `styled_pick_list`, the open/closed state
/// is the **caller's**, not iced's own internal overlay state (the handoff
/// table's "the open state passed in and `on_open_change`") — `opened` and
/// `on_open_change` thread it through. Options are the same already-
/// localized `(T, String)` pairs `styled_pick_list` takes, for the same
/// reason its own module doc gives (a per-render label with no `'static`
/// source to borrow from).
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

/// The closed (and opening) control's style: `surface` background,
/// `text_primary` text, `border` at rest, `accent` border when hovered or
/// open — then the focus ring on top when `is_focused`.
///
/// `pub(crate)` so `theme.rs`'s contrast test can drive the real function
/// across every `Status` rather than reconstructing its background from a
/// flat palette role (RFC-035 Handoff 022 §7.5, same reason `chip::style`
/// was made `pub(crate)` in Handoff 020).
pub(crate) fn field_style(
    tokens: &Tokens,
    status: pick_list_widget::Status,
    is_focused: bool,
) -> pick_list_widget::Style {
    let border_color = match status {
        pick_list_widget::Status::Active => to_iced(tokens.palette.border),
        pick_list_widget::Status::Hovered | pick_list_widget::Status::Opened { .. } => {
            to_iced(tokens.palette.accent)
        }
    };

    let base = pick_list_widget::Style {
        text_color: to_iced(tokens.palette.text_primary),
        placeholder_color: to_iced(tokens.palette.text_muted),
        handle_color: to_iced(tokens.palette.text_secondary),
        background: iced::Background::Color(to_iced(tokens.palette.surface)),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
    };

    if !is_focused {
        return base;
    }

    pick_list_widget::Style {
        border: iced::Border {
            color: ring_color_for(tokens, Some(base.background)),
            width: tokens.focus.ring_width,
            radius: base.border.radius,
        },
        ..base
    }
}

/// The open menu's style: `surface_raised` background (matching
/// `overlay::raised_card`'s elevated treatment), `accent`/`accent_text` for
/// the selected row — then the focus ring on top when `is_focused`, since
/// the field and the menu are the same logical control for keyboard-focus
/// purposes.
pub(crate) fn menu_style(tokens: &Tokens, is_focused: bool) -> menu::Style {
    let base = menu::Style {
        background: iced::Background::Color(to_iced(tokens.palette.surface_raised)),
        border: iced::Border {
            color: to_iced(tokens.palette.border),
            width: 1.0,
            radius: tokens.radius.sm.into(),
        },
        text_color: to_iced(tokens.palette.text_primary),
        selected_text_color: to_iced(tokens.palette.accent_text),
        selected_background: iced::Background::Color(to_iced(tokens.palette.accent)),
        shadow: iced::Shadow::default(),
    };

    if !is_focused {
        return base;
    }

    menu::Style {
        border: iced::Border {
            color: ring_color_for(tokens, Some(base.background)),
            width: tokens.focus.ring_width,
            radius: base.border.radius,
        },
        ..base
    }
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

    const SIZE: iced_test::core::Size = iced_test::core::Size {
        width: 400.0,
        height: 400.0,
    };

    /// RFC-075 handoff stage 1 §3, row 4. Three closed-trigger openers
    /// (Enter, Space, ↓ — all three, since `Select::on_open_change`'s own
    /// doc comment names exactly these three plus the trigger's own press);
    /// choosing an item by Tab then Enter, with no close published by
    /// `on_select` itself (**"the caller closes it"** — confirmed directly
    /// against `snora_widgets::form::Select`'s own doc comment, in the
    /// registry: "What opening and closing make: `true` from the trigger,
    /// Enter, Space and ↓ ... `false` from Escape and from the trigger when
    /// open" — choosing is in neither list); and Escape closing with focus
    /// back on the trigger.
    ///
    /// **"↓ moves" inside the open menu is not checked here.** Tried
    /// directly: with an item already Tab-focused inside the open
    /// dropdown, a raw `ArrowDown` moves nothing this harness can see —
    /// `focused_id()` (built on `operation::Focusable`) reports the same
    /// id before and after. Whatever moves the open menu's own highlighted
    /// row is internal to `snora_widgets::form::Select`'s menu overlay, not
    /// a second `Focusable` target `operate()` can read — the same kind of
    /// gap stage 0's own report named for the focus ring ("read from
    /// pixels", not `Operation`), and not one this stage's harness builds
    /// pixel-sampling for. Reported here rather than asserted on a guess.
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
            let mut h = Harness::new(view(&tokens, None, false), SIZE, None);
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
        let mut h = Harness::new(view(&tokens, None, false), SIZE, None);
        h.tab(false);
        h.enter();
        let mut h = h.rebuilt(view(&tokens, None, true));
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
        let mut h = Harness::new(view(&tokens, Some("a"), true), SIZE, None);
        h.tab(false);
        h.escape();
        assert_eq!(h.messages(), [Msg::OpenChanged(false)]);
        let mut h = h.rebuilt(view(&tokens, Some("a"), false));
        assert_eq!(
            h.focused_id(),
            Some(id),
            "Escape must return focus to the trigger, not leave nothing focused"
        );
    }

    /// RFC-075 handoff stage 1 §3, row 2. **Found, not achieved**: the
    /// table's own "44 px tall like today" does not hold for
    /// `select_control`, measured directly (both rendered at 220×60,
    /// `KnotraTheme::light()`, read back and the non-background bounding
    /// box found by scanning pixels, not inferred):
    ///
    /// - `styled_pick_list`'s own box: `y[0..43]`, **44px** — its
    ///   `.padding([(BUTTON_HEIGHT - line_height) / 2.0, 12.0])` is built
    ///   to land exactly there (this module's own doc comment on
    ///   `styled_pick_list`).
    /// - `select_control`'s own box: `y[9..34]`, **26px**, inset within
    ///   the 44px the control's own min-target floor reserves —
    ///   `(44 - 26) / 2 = 9`, the measured top inset exactly. **The floor
    ///   does reserve 44px** (confirmed: `tokens.targets.min` is wired to
    ///   `BUTTON_HEIGHT` for every `KnotraTheme` preset, `theme.rs`), so
    ///   Tab's target and the row spacing around it are the full 44px —
    ///   what is smaller is the **drawn** border and fill, 18px shorter
    ///   than `styled_pick_list`'s.
    ///
    /// Root cause, read directly rather than guessed: `Select`'s own
    /// trigger padding comes from `Skin::designed`'s own `padding:
    /// Padding::from([tokens.spacing.xs, tokens.spacing.sm])`
    /// (`snora_widgets::form::Skin::designed`, in the registry) — a small,
    /// fixed padding with no path for a caller to make larger. `Select`
    /// has no `.height(...)` (or any sizing) builder at all (confirmed:
    /// its full builder surface is `.opened`/`.on_open_change`/
    /// `.placeholder`/`.id`, read directly above), so nothing this crate
    /// calls can close the 18px gap without forking `snora-widgets`,
    /// which is out of this stage's scope. Reported, not patched around.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and reads back pixels — the pinned face (Noto Sans) is installed \
                  only on the Linux CI runner (Task 108 A1 §4, `crates/knotra-app/src/\
                  label_overflow_guard.rs`'s own precedent); the height this measures \
                  would be some other font's on macOS/Windows"
    )]
    fn select_control_box_is_shorter_than_styled_pick_lists_at_rest() {
        use crate::test_support::Harness;

        let tokens = crate::theme::KnotraTheme::light().tokens;
        let size = iced_test::core::Size {
            width: 220.0,
            height: 60.0,
        };
        let old: iced::Element<'static, Msg> = styled_pick_list(
            &tokens,
            vec![("a", "Alpha".to_owned()), ("b", "Beta".to_owned())],
            Some("a"),
            false,
            Msg::Selected,
        );
        let new: iced::Element<'static, Msg> = select_control(
            &tokens,
            vec![("a", "Alpha".to_owned()), ("b", "Beta".to_owned())],
            Some("a"),
            false,
            Msg::Selected,
            Msg::OpenChanged,
            iced::widget::Id::new("select"),
        );
        let theme = crate::theme::KnotraTheme::light();
        let icon_font = Some(crate::widget::icon::FONT_BYTES);
        let mut h_old = Harness::new(old, size, icon_font);
        let (old_rgba, ow, oh) = h_old.screenshot(&theme.base);
        let mut h_new = Harness::new(new, size, icon_font);
        let (new_rgba, nw, nh) = h_new.screenshot(&theme.base);

        fn non_background_height(rgba: &[u8], w: u32, h: u32) -> u32 {
            let mut min_y = u32::MAX;
            let mut max_y = 0u32;
            for y in 0..h {
                for x in 0..w.min(110) {
                    let i = (((y * w) + x) * 4) as usize;
                    if rgba[i..i + 3] != [255, 255, 255] {
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                }
            }
            max_y - min_y + 1
        }

        let old_height = non_background_height(&old_rgba, ow, oh);
        let new_height = non_background_height(&new_rgba, nw, nh);
        assert_eq!(
            old_height, 44,
            "styled_pick_list's own box must be BUTTON_HEIGHT tall — its padding formula \
             is derived from fixed token values, not glyph metrics, so this holds across \
             every font/platform (local: 26px; CI's Ubuntu runner: 28px — both measured \
             directly, confirmed by this test failing CI at 26 before this range was \
             widened to cover both)"
        );
        assert!(
            (20..=32).contains(&new_height),
            "select_control's drawn box height ({new_height}px) moved well outside the \
             20-32px this stage measured (26px locally, 28px on CI's Ubuntu runner — \
             select_control's height is glyph-metric-derived, unlike styled_pick_list's \
             fixed-formula 44, so it is expected to move a little by font/platform, just \
             not this much) — re-measure and update this test, its doc comment and the \
             handoff report together"
        );
    }

    /// Same shape as `buttons.rs`'s
    /// `filled_accent_style_gets_a_different_ring_color_than_a_transparent_one`
    /// — RFC-035 Handoff 019 §8's required per-primitive check that focused
    /// and unfocused styles differ, which is what would have caught
    /// RFC-036's invisible-ring defect earlier.
    #[test]
    fn field_focused_and_unfocused_styles_differ() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let unfocused = field_style(&tokens, pick_list_widget::Status::Active, false);
            let focused = field_style(&tokens, pick_list_widget::Status::Active, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }

    #[test]
    fn menu_focused_and_unfocused_styles_differ() {
        for tokens in [
            crate::theme::KnotraTheme::dark().tokens,
            crate::theme::KnotraTheme::light().tokens,
        ] {
            let unfocused = menu_style(&tokens, false);
            let focused = menu_style(&tokens, true);
            assert_ne!(unfocused.border, focused.border);
        }
    }
}
