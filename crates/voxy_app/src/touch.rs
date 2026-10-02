use glam::Vec2;
use winit::event::TouchPhase;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TouchAction {
    None,
    Rotate(f32),
    TogglePause,
}
#[derive(Clone, Copy, Debug)]
struct Contact {
    id: u64,
    start: Vec2,
    last: Vec2,
    dragged: bool,
}
#[derive(Debug, Default)]
pub(crate) struct TouchControls {
    active: Option<Contact>,
}
impl TouchControls {
    pub(crate) fn reset(&mut self) {
        self.active = None;
    }
    pub(crate) fn update(&mut self, id: u64, phase: TouchPhase, position: Vec2) -> TouchAction {
        if !position.is_finite() {
            self.reset();
            return TouchAction::None;
        }
        if phase == TouchPhase::Started {
            if self.active.is_none() {
                self.active = Some(Contact {
                    id,
                    start: position,
                    last: position,
                    dragged: false,
                });
            }
            return TouchAction::None;
        }
        let Some(mut contact) = self.active.filter(|contact| contact.id == id) else {
            return TouchAction::None;
        };
        contact.dragged |= contact.start.distance_squared(position) > 0.0001;
        let action = match phase {
            TouchPhase::Moved if contact.dragged => {
                TouchAction::Rotate((position.x - contact.last.x) * std::f32::consts::TAU)
            }
            TouchPhase::Ended if !contact.dragged => TouchAction::TogglePause,
            _ => TouchAction::None,
        };
        if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.active = None;
        } else {
            contact.last = position;
            self.active = Some(contact);
        }
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tap_toggles_pause_but_drag_does_not() {
        let mut controls = TouchControls::default();
        controls.update(1, TouchPhase::Started, Vec2::ZERO);
        assert_eq!(
            controls.update(1, TouchPhase::Ended, Vec2::ZERO),
            TouchAction::TogglePause
        );
        controls.update(1, TouchPhase::Started, Vec2::ZERO);
        let TouchAction::Rotate(angle) =
            controls.update(1, TouchPhase::Moved, Vec2::new(0.25, 0.0))
        else {
            panic!("drag must rotate");
        };
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(
            controls.update(1, TouchPhase::Ended, Vec2::new(0.25, 0.0)),
            TouchAction::None
        );
    }
    #[test]
    fn cancellation_and_secondary_fingers_do_not_trigger_actions() {
        let mut controls = TouchControls::default();
        controls.update(1, TouchPhase::Started, Vec2::ZERO);
        controls.update(2, TouchPhase::Started, Vec2::ZERO);
        assert_eq!(
            controls.update(2, TouchPhase::Ended, Vec2::ZERO),
            TouchAction::None
        );
        assert_eq!(
            controls.update(1, TouchPhase::Cancelled, Vec2::ZERO),
            TouchAction::None
        );
        assert_eq!(
            controls.update(1, TouchPhase::Ended, Vec2::ZERO),
            TouchAction::None
        );
        controls.update(1, TouchPhase::Started, Vec2::ZERO);
        controls.reset();
        assert_eq!(
            controls.update(1, TouchPhase::Ended, Vec2::ZERO),
            TouchAction::None
        );
    }
}
