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

/// Evaluates a two-finger pinch-zoom distance ratio `current_span / initial_span`, clamped to
/// `[min_scale, max_scale]`. Returns `None` if Touch points are non-finite or `initial_span <= 1e-4`.
#[must_use]
pub fn pinch_zoom_ratio(
    initial_a: Vec2,
    initial_b: Vec2,
    current_a: Vec2,
    current_b: Vec2,
    min_scale: f32,
    max_scale: f32,
) -> Option<f32> {
    if !initial_a.is_finite()
        || !initial_b.is_finite()
        || !current_a.is_finite()
        || !current_b.is_finite()
        || !min_scale.is_finite()
        || !max_scale.is_finite()
        || min_scale <= 0.0
        || max_scale < min_scale
    {
        return None;
    }
    let initial_span = initial_a.distance(initial_b);
    if initial_span <= 1e-4 {
        return None;
    }
    let current_span = current_a.distance(current_b);
    Some((current_span / initial_span).clamp(min_scale, max_scale))
}

/// Simultaneous two-finger pinch scale ratio and signed rotation angle `(scale_ratio, delta_angle_rad)`
/// with dead-zones (`scale_deadzone`, `angle_deadzone_rad`) to prevent inadvertent twist during zoom.
#[must_use]
pub fn classify_two_finger_pinch_rotate(
    initial_a: Vec2,
    initial_b: Vec2,
    current_a: Vec2,
    current_b: Vec2,
    scale_deadzone: f32,
    angle_deadzone_rad: f32,
) -> Option<(f32, f32)> {
    let raw_scale = pinch_zoom_ratio(initial_a, initial_b, current_a, current_b, 0.1, 10.0)?;
    if !scale_deadzone.is_finite()
        || scale_deadzone < 0.0
        || !angle_deadzone_rad.is_finite()
        || angle_deadzone_rad < 0.0
    {
        return None;
    }
    let v0 = initial_b - initial_a;
    let v1 = current_b - current_a;
    if v1.length_squared() <= 1e-8 {
        return None;
    }
    let scale = if (raw_scale - 1.0).abs() <= scale_deadzone {
        1.0
    } else {
        raw_scale
    };
    let raw_angle = (v0.x * v1.y - v0.y * v1.x).atan2(v0.dot(v1));
    let angle = if raw_angle.abs() <= angle_deadzone_rad {
        0.0
    } else {
        raw_angle - raw_angle.signum() * angle_deadzone_rad
    };
    Some((scale, angle))
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
    #[test]
    fn pinch_zoom_ratio_computes_clamped_span_factor() {
        let ratio = pinch_zoom_ratio(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.2, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.4, 0.0),
            0.5,
            3.0,
        )
        .unwrap();
        assert!((ratio - 2.0).abs() < 1e-5);
        assert!(
            pinch_zoom_ratio(Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, Vec2::ONE, 0.5, 2.0).is_none()
        );
        let (scale, angle) = classify_two_finger_pinch_rotate(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 2.0),
            0.05,
            0.05,
        )
        .unwrap();
        assert!((scale - 2.0).abs() < 1e-5);
        assert!(angle > 1.4);
    }
}
