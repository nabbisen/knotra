//! Keyboard focus model (RFC-036 D1).
//!
//! iced 0.14 only implements `Focusable` for `text_input`/`text_editor`; a
//! `button`'s style closure has no way to know it is focused. knotra
//! therefore owns focus itself for everything except text inputs, and keeps
//! iced's own text-input focus in lockstep with it via [`reconcile`] rather
//! than letting the two disagree — see D1's "dual-focus hazard".

use std::borrow::Cow;

/// A single position in a view's keyboard focus order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FocusTarget {
    /// A control iced cannot focus on its own — button, chip, checkbox,
    /// section header, row action. The key is minted by the view that
    /// declares the order and must stay stable across frames for the same
    /// control, so [`resolve`] can find it again after the view rebuilds.
    Control(Cow<'static, str>),
    /// A text input, which iced already focuses on its own. Carries the same
    /// `Id` `knotra_ui::widget::focus_input` uses, so [`reconcile`] can keep
    /// knotra-focus and iced-focus in lockstep (R12).
    TextInput(iced::widget::Id),
}

impl FocusTarget {
    pub const fn control(key: &'static str) -> Self {
        FocusTarget::Control(Cow::Borrowed(key))
    }

    /// Dynamic per-row keys (e.g. a project ID baked into the key) that
    /// `control`'s `&'static str` cannot express.
    pub fn control_dynamic(key: String) -> Self {
        FocusTarget::Control(Cow::Owned(key))
    }

    pub fn text_input(id: iced::widget::Id) -> Self {
        FocusTarget::TextInput(id)
    }
}

/// A view's declared focus order: each target paired with the `Message` its
/// activation (Enter/Space) dispatches. `None` means the target is a valid
/// Tab stop with nothing to activate right now — e.g. a disabled control,
/// which RFC-033 D6 requires stay reachable so the user can see *why* it is
/// unavailable, even though activating it does nothing (R3).
pub type FocusOrder<Message> = Vec<(FocusTarget, Option<Message>)>;

/// Which direction Tab/Shift-Tab moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Previous,
}

/// Resolves the effective current target from a possibly-stale `current`.
///
/// `None` means knotra-focus has not been engaged at all (a freshly opened
/// screen or overlay) and stays `None` — nothing should appear focused
/// merely because the user has not pressed Tab yet. `Some(target)` where
/// `target` no longer exists in `order` (e.g. a filtered-out row or a closed
/// overlay) falls back to the first target instead, per R9: focus that
/// *was* held is never lost silently, unlike focus that was never held.
pub fn resolve<'a, Message>(
    order: &'a FocusOrder<Message>,
    current: Option<&FocusTarget>,
) -> Option<&'a FocusTarget> {
    let current = current?;
    match order.iter().find(|(target, _)| target == current) {
        Some((target, _)) => Some(target),
        None => order.first().map(|(target, _)| target),
    }
}

/// Advances the resolved current target one step in `direction`, wrapping
/// around at either end (R1).
pub fn advance<'a, Message>(
    order: &'a FocusOrder<Message>,
    current: Option<&FocusTarget>,
    direction: Direction,
) -> Option<&'a FocusTarget> {
    if order.is_empty() {
        return None;
    }

    let index =
        resolve(order, current).and_then(|target| order.iter().position(|(t, _)| t == target));

    let next_index = match (index, direction) {
        (None, Direction::Next) => 0,
        (None, Direction::Previous) => order.len() - 1,
        (Some(i), Direction::Next) => (i + 1) % order.len(),
        (Some(i), Direction::Previous) => (i + order.len() - 1) % order.len(),
    };

    order.get(next_index).map(|(target, _)| target)
}

/// The `Message` the resolved current target's activation (Enter/Space)
/// dispatches, if any (R3). Returns `None` both when there is nothing to
/// activate (a disabled control) and when the order is empty.
pub fn activation_message<Message: Clone>(
    order: &FocusOrder<Message>,
    current: Option<&FocusTarget>,
) -> Option<Message> {
    let resolved = resolve(order, current)?;
    order
        .iter()
        .find(|(target, _)| target == resolved)
        .and_then(|(_, message)| message.clone())
}

/// Whether the resolved current target is a text input — used to gate Enter,
/// Space, and bare `/` so they reach the field instead of activating a
/// control or jumping focus (R3a, R4).
pub fn is_text_input_focused<Message>(
    order: &FocusOrder<Message>,
    current: Option<&FocusTarget>,
) -> bool {
    matches!(resolve(order, current), Some(FocusTarget::TextInput(_)))
}

/// What must happen to iced's own text-input focus when knotra-focus moves
/// from `previous` to `next` (R12, D1's reconciliation rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciliation {
    /// Neither the old nor the new target is a text input; iced's focus is
    /// untouched.
    None,
    /// The new target is a text input: iced must focus it, so typed
    /// characters follow knotra-focus.
    FocusTextInput(iced::widget::Id),
    /// The old target was a text input and the new one is not: iced's focus
    /// must be explicitly cleared, or the text input would keep receiving
    /// keystrokes after the visible ring has moved off it — the hazard D1
    /// describes, in reverse.
    ClearTextInputFocus,
}

/// Computes the reconciliation required by a knotra-focus transition from
/// `previous` to `next`. Callers must apply the result (see
/// `knotra_ui::widget::{focus_input, clear_input_focus}`) in the same `Task`
/// that changes knotra-focus — see Guardrail 2 of RFC-036's Developer
/// Handoff: no path may change one without the other.
pub fn reconcile(previous: Option<&FocusTarget>, next: Option<&FocusTarget>) -> Reconciliation {
    match next {
        Some(FocusTarget::TextInput(id)) => Reconciliation::FocusTextInput(id.clone()),
        _ => match previous {
            Some(FocusTarget::TextInput(_)) => Reconciliation::ClearTextInputFocus,
            _ => Reconciliation::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestMessage {
        A,
        B,
        C,
    }

    fn control_order() -> FocusOrder<TestMessage> {
        vec![
            (FocusTarget::control("a"), Some(TestMessage::A)),
            (FocusTarget::control("b"), Some(TestMessage::B)),
            (FocusTarget::control("c"), Some(TestMessage::C)),
        ]
    }

    #[test]
    fn advance_next_from_none_lands_on_first() {
        let order = control_order();
        assert_eq!(
            advance(&order, None, Direction::Next),
            Some(&FocusTarget::control("a"))
        );
    }

    #[test]
    fn advance_next_wraps_from_last_to_first() {
        let order = control_order();
        let last = FocusTarget::control("c");
        assert_eq!(
            advance(&order, Some(&last), Direction::Next),
            Some(&FocusTarget::control("a"))
        );
    }

    #[test]
    fn advance_previous_wraps_from_first_to_last() {
        let order = control_order();
        let first = FocusTarget::control("a");
        assert_eq!(
            advance(&order, Some(&first), Direction::Previous),
            Some(&FocusTarget::control("c"))
        );
    }

    #[test]
    fn advance_on_empty_order_is_none() {
        let order: FocusOrder<TestMessage> = Vec::new();
        assert_eq!(advance(&order, None, Direction::Next), None);
    }

    #[test]
    fn resolve_falls_back_to_first_when_current_target_is_gone() {
        // R9: the view rebuilt without "b" (e.g. a filtered-out row).
        let order = vec![
            (FocusTarget::control("a"), Some(TestMessage::A)),
            (FocusTarget::control("c"), Some(TestMessage::C)),
        ];
        let stale = FocusTarget::control("b");
        assert_eq!(
            resolve(&order, Some(&stale)),
            Some(&FocusTarget::control("a"))
        );
    }

    #[test]
    fn activation_message_returns_the_current_targets_message() {
        let order = control_order();
        let b = FocusTarget::control("b");
        assert_eq!(activation_message(&order, Some(&b)), Some(TestMessage::B));
    }

    #[test]
    fn activation_message_is_none_for_a_disabled_control() {
        // A disabled control is still a Tab stop (order contains it) but
        // activating it dispatches nothing.
        let order = vec![
            (FocusTarget::control("a"), Some(TestMessage::A)),
            (FocusTarget::control("disabled"), None),
        ];
        let disabled = FocusTarget::control("disabled");
        assert_eq!(activation_message(&order, Some(&disabled)), None);
        // ...and it is still reachable by Tab.
        assert_eq!(
            advance(&order, Some(&FocusTarget::control("a")), Direction::Next),
            Some(&disabled)
        );
    }

    #[test]
    fn is_text_input_focused_distinguishes_control_from_text_input() {
        let text_id = iced::widget::Id::new("field");
        let order = vec![
            (FocusTarget::control("button"), Some(TestMessage::A)),
            (FocusTarget::text_input(text_id.clone()), None),
        ];
        let button = FocusTarget::control("button");
        let field = FocusTarget::text_input(text_id);
        assert!(!is_text_input_focused(&order, Some(&button)));
        assert!(is_text_input_focused(&order, Some(&field)));
    }

    #[test]
    fn reconcile_focuses_iced_when_moving_onto_a_text_input() {
        let id = iced::widget::Id::new("field");
        let button = FocusTarget::control("button");
        let field = FocusTarget::text_input(id.clone());
        assert_eq!(
            reconcile(Some(&button), Some(&field)),
            Reconciliation::FocusTextInput(id)
        );
    }

    #[test]
    fn reconcile_clears_iced_focus_when_moving_off_a_text_input() {
        let id = iced::widget::Id::new("field");
        let button = FocusTarget::control("button");
        let field = FocusTarget::text_input(id);
        assert_eq!(
            reconcile(Some(&field), Some(&button)),
            Reconciliation::ClearTextInputFocus
        );
    }

    #[test]
    fn reconcile_does_nothing_between_two_controls() {
        let a = FocusTarget::control("a");
        let b = FocusTarget::control("b");
        assert_eq!(reconcile(Some(&a), Some(&b)), Reconciliation::None);
    }

    #[test]
    fn reconcile_does_nothing_between_two_text_inputs_other_than_focusing_the_new_one() {
        // Moving text-input -> text-input must still (re)issue focus for the
        // new one — it's the same branch as control -> text-input — this
        // test exists to make that explicit rather than assumed.
        let id_a = iced::widget::Id::new("a");
        let id_b = iced::widget::Id::new("b");
        let a = FocusTarget::text_input(id_a);
        let b = FocusTarget::text_input(id_b.clone());
        assert_eq!(
            reconcile(Some(&a), Some(&b)),
            Reconciliation::FocusTextInput(id_b)
        );
    }
}
