use crate::{UiError, WidgetId};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    None,
    Press(WidgetId),
    Release { id: WidgetId, clicked: bool },
    Cancel(WidgetId),
}
/// Platform-neutral keyboard focus. `set_order` receives only enabled focusable
/// widgets in traversal order; IDs must be distinct across widget lifetimes.
/// Native adapters combine Enter/Space into one activation action with edges.
#[derive(Debug)]
pub struct FocusRouter {
    order: Vec<WidgetId>,
    capacity: usize,
    focus: Option<WidgetId>,
    pressed: Option<WidgetId>,
    held: bool,
}
impl FocusRouter {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            order: Vec::new(),
            capacity,
            focus: None,
            pressed: None,
            held: false,
        }
    }
    #[must_use]
    pub fn focused(&self) -> Option<WidgetId> {
        self.focus
    }
    /// Reconciles enabled traversal entries atomically. Removed pressed targets
    /// cancel activation; a removed focused target clears focus.
    /// # Errors
    /// Rejects capacity and duplicate IDs without changing focus or activation.
    pub fn set_order(&mut self, order: &[WidgetId]) -> Result<KeyAction, UiError> {
        if order.len() > self.capacity {
            return Err(UiError::Capacity);
        }
        let mut unique = BTreeSet::new();
        if order.iter().any(|id| !unique.insert(*id)) {
            return Err(UiError::DuplicateId);
        }
        self.order.clear();
        self.order.extend_from_slice(order);
        if self.focus.is_some_and(|id| !unique.contains(&id)) {
            self.focus = None;
        }
        let cancelled = self.pressed.filter(|id| !unique.contains(id));
        if cancelled.is_some() {
            self.pressed = None;
        }
        Ok(cancelled.map_or(KeyAction::None, KeyAction::Cancel))
    }
    /// Explicit focus assignment, e.g. after a pointer press. Focus changes
    /// cancel held activation without allowing another press until key release.
    /// # Errors
    /// Rejects IDs absent from the enabled focus order.
    pub fn focus_to(&mut self, id: Option<WidgetId>) -> Result<KeyAction, UiError> {
        if id.is_some_and(|id| !self.order.contains(&id)) {
            return Err(UiError::UnknownWidget);
        }
        if self.focus == id {
            return Ok(KeyAction::None);
        }
        self.focus = id;
        Ok(self
            .pressed
            .take()
            .map_or(KeyAction::None, KeyAction::Cancel))
    }
    /// Wraps traversal; reverse starts from the final widget when unfocused.
    pub fn traverse(&mut self, reverse: bool) -> KeyAction {
        let index = self
            .focus
            .and_then(|id| self.order.iter().position(|item| *item == id));
        let next = if self.order.is_empty() {
            None
        } else {
            let index = match (index, reverse) {
                (None, false) => 0,
                (None | Some(0), true) => self.order.len() - 1,
                (Some(index), true) => index - 1,
                (Some(index), false) => (index + 1) % self.order.len(),
            };
            Some(self.order[index])
        };
        if self.focus == next {
            return KeyAction::None;
        }
        self.focus = next;
        self.pressed
            .take()
            .map_or(KeyAction::None, KeyAction::Cancel)
    }
    pub fn press(&mut self) -> KeyAction {
        if self.held {
            return KeyAction::None;
        }
        self.held = true;
        self.pressed = self.focus;
        self.pressed.map_or(KeyAction::None, KeyAction::Press)
    }
    pub fn release(&mut self) -> KeyAction {
        self.held = false;
        self.pressed
            .take()
            .map_or(KeyAction::None, |id| KeyAction::Release {
                id,
                clicked: self.focus == Some(id),
            })
    }
    /// Window focus loss clears focus and activation state.
    pub fn cancel(&mut self) -> KeyAction {
        self.focus = None;
        self.held = false;
        self.pressed
            .take()
            .map_or(KeyAction::None, KeyAction::Cancel)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_wrap_repeat_and_focus_change() {
        let mut router = FocusRouter::new(3);
        router.set_order(&[WidgetId(1), WidgetId(3)]).unwrap();
        router.traverse(true);
        assert_eq!(router.focused(), Some(WidgetId(3)));
        assert_eq!(router.press(), KeyAction::Press(WidgetId(3)));
        assert_eq!(router.press(), KeyAction::None);
        assert_eq!(router.traverse(false), KeyAction::Cancel(WidgetId(3)));
        assert_eq!(router.focused(), Some(WidgetId(1)));
        assert_eq!(router.press(), KeyAction::None);
        assert_eq!(router.release(), KeyAction::None);
        router.press();
        assert_eq!(
            router.release(),
            KeyAction::Release {
                id: WidgetId(1),
                clicked: true
            }
        );
    }
    #[test]
    fn removal_invalid_order_and_window_focus_loss() {
        let mut router = FocusRouter::new(2);
        router.set_order(&[WidgetId(1)]).unwrap();
        router.focus_to(Some(WidgetId(1))).unwrap();
        router.press();
        assert_eq!(
            router.set_order(&[WidgetId(1), WidgetId(1)]),
            Err(UiError::DuplicateId)
        );
        assert_eq!(
            router.focus_to(Some(WidgetId(2))),
            Err(UiError::UnknownWidget)
        );
        assert_eq!(router.focused(), Some(WidgetId(1)));
        assert_eq!(router.set_order(&[]), Ok(KeyAction::Cancel(WidgetId(1))));
        assert_eq!(router.release(), KeyAction::None);
        router.set_order(&[WidgetId(2)]).unwrap();
        router.traverse(false);
        router.press();
        assert_eq!(router.cancel(), KeyAction::Cancel(WidgetId(2)));
        assert_eq!(router.focused(), None);
        assert_eq!(router.release(), KeyAction::None);
    }
}
