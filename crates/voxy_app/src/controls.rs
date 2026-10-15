use glam::{Vec2, Vec3};
use winit::event::{ElementState, Touch, TouchPhase};
use winit::keyboard::KeyCode;

const LOOK_SENSITIVITY: f32 = 0.0025;
const TOUCH_LOOK_SENSITIVITY: f32 = 0.006;
const TOUCH_STICK_RADIUS: f32 = 80.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveIntent {
    pub local: Vec2,
    pub jump_pressed: bool,
    pub sprint: bool,
}

#[derive(Debug, Default)]
pub struct InputState {
    held: u16,
    jump_queued: bool,
    fire_queued: bool,
    look_delta: Vec2,
    touch_move: Vec2,
    move_touch: Option<TouchStick>,
    look_touch: Option<TouchLook>,
}

#[derive(Clone, Copy, Debug)]
struct TouchStick {
    id: u64,
    origin: Vec2,
}

#[derive(Clone, Copy, Debug)]
struct TouchLook {
    id: u64,
    origin: Vec2,
    previous: Vec2,
    moved: bool,
}

impl InputState {
    pub fn key(&mut self, key: KeyCode, state: ElementState, repeated: bool) {
        let pressed = state == ElementState::Pressed;
        if key == KeyCode::Space {
            if pressed && !repeated {
                self.jump_queued = true;
            }
            return;
        }
        let mask = match key {
            KeyCode::KeyW => 1,
            KeyCode::ArrowUp => 64,
            KeyCode::KeyS => 2,
            KeyCode::ArrowDown => 128,
            KeyCode::KeyA => 4,
            KeyCode::ArrowLeft => 256,
            KeyCode::KeyD => 8,
            KeyCode::ArrowRight => 512,
            KeyCode::ShiftLeft => 16,
            KeyCode::ShiftRight => 32,
            _ => return,
        };
        if pressed {
            self.held |= mask;
        } else {
            self.held &= !mask;
        }
    }

    pub fn mouse_motion(&mut self, delta: (f64, f64)) {
        #[allow(clippy::cast_possible_truncation)]
        let delta = Vec2::new(delta.0 as f32, delta.1 as f32);
        if delta.is_finite() {
            self.look_delta += delta * LOOK_SENSITIVITY;
        }
    }

    pub fn queue_fire(&mut self) {
        self.fire_queued = true;
    }

    pub fn take_fire(&mut self) -> bool {
        std::mem::take(&mut self.fire_queued)
    }

    pub fn touch(&mut self, touch: Touch, width: f32) {
        #[allow(clippy::cast_possible_truncation)]
        let position = Vec2::new(touch.location.x as f32, touch.location.y as f32);
        self.touch_event(touch.id, touch.phase, position, width);
    }

    fn touch_event(&mut self, id: u64, phase: TouchPhase, position: Vec2, width: f32) {
        match phase {
            TouchPhase::Started if position.x < width * 0.5 && self.move_touch.is_none() => {
                self.move_touch = Some(TouchStick {
                    id,
                    origin: position,
                });
            }
            TouchPhase::Started if self.look_touch.is_none() => {
                self.look_touch = Some(TouchLook {
                    id,
                    origin: position,
                    previous: position,
                    moved: false,
                });
            }
            TouchPhase::Moved => {
                if let Some(stick) = self.move_touch.filter(|stick| stick.id == id) {
                    self.touch_move = ((position - stick.origin) / TOUCH_STICK_RADIUS)
                        .clamp(Vec2::splat(-1.0), Vec2::ONE);
                } else if let Some(look) = &mut self.look_touch
                    && look.id == id
                {
                    self.look_delta += (position - look.previous) * TOUCH_LOOK_SENSITIVITY;
                    look.previous = position;
                    look.moved |= position.distance_squared(look.origin) > 12.0 * 12.0;
                }
            }
            TouchPhase::Ended => {
                if self.move_touch.is_some_and(|stick| stick.id == id) {
                    self.move_touch = None;
                    self.touch_move = Vec2::ZERO;
                }
                if self.look_touch.is_some_and(|look| look.id == id) {
                    if self.look_touch.is_some_and(|look| !look.moved) {
                        self.jump_queued = true;
                    }
                    self.look_touch = None;
                }
            }
            TouchPhase::Cancelled => {
                if self.move_touch.is_some_and(|stick| stick.id == id) {
                    self.move_touch = None;
                    self.touch_move = Vec2::ZERO;
                }
                if self.look_touch.is_some_and(|look| look.id == id) {
                    self.look_touch = None;
                }
            }
            TouchPhase::Started => {}
        }
    }

    pub fn take_move(&mut self) -> MoveIntent {
        let keyboard = Vec2::new(
            f32::from(i8::from(self.held & (8 | 512) != 0) - i8::from(self.held & (4 | 256) != 0)),
            f32::from(i8::from(self.held & (1 | 64) != 0) - i8::from(self.held & (2 | 128) != 0)),
        );
        let mut local = keyboard + Vec2::new(self.touch_move.x, -self.touch_move.y);
        if local.length_squared() > 1.0 {
            local = local.normalize();
        }
        MoveIntent {
            local,
            jump_pressed: std::mem::take(&mut self.jump_queued),
            sprint: self.held & (16 | 32) != 0,
        }
    }

    pub fn take_look(&mut self) -> Vec2 {
        std::mem::take(&mut self.look_delta)
    }

    /// Cancel every pending action when the window loses focus.
    pub fn cancel_all(&mut self) {
        *self = Self::default();
    }

    pub fn clear_keyboard(&mut self) {
        self.held = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraRig {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            yaw: -135_f32.to_radians(),
            pitch: -28_f32.to_radians(),
            distance: 13.0,
        }
    }
}

impl CameraRig {
    pub fn apply_look(&mut self, delta: Vec2) {
        self.yaw -= delta.x;
        self.pitch = (self.pitch - delta.y).clamp(-75_f32.to_radians(), -5_f32.to_radians());
    }

    pub fn planar_basis(self) -> (Vec3, Vec3) {
        let forward = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos()).normalize_or_zero();
        let right = Vec3::new(forward.z, 0.0, -forward.x);
        (forward, right)
    }

    pub fn zoom(&mut self, lines: f32) {
        if lines.is_finite() {
            self.distance = (self.distance - lines * 2.0).clamp(4.0, 36.0);
        }
    }

    pub fn eye_and_target(self, actor: Vec3) -> (Vec3, Vec3) {
        let target = actor + Vec3::Y * 1.6;
        let direction = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        (target - direction * self.distance, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_keyboard_input_is_normalized_and_jump_is_edge_triggered() {
        let mut input = InputState::default();
        input.key(KeyCode::KeyW, ElementState::Pressed, false);
        input.key(KeyCode::KeyD, ElementState::Pressed, false);
        input.key(KeyCode::Space, ElementState::Pressed, false);
        let first = input.take_move();
        assert!((first.local.length() - 1.0).abs() < 1.0e-6);
        assert!(first.jump_pressed);
        assert!(!input.take_move().jump_pressed);
    }

    #[test]
    fn focus_loss_cancels_pending_actions_and_touch_ownership() {
        let mut input = InputState::default();
        input.key(KeyCode::KeyW, ElementState::Pressed, false);
        input.key(KeyCode::Space, ElementState::Pressed, false);
        input.mouse_motion((10.0, 20.0));
        input.queue_fire();
        input.touch_event(1, TouchPhase::Started, Vec2::new(100.0, 200.0), 1000.0);
        input.touch_event(1, TouchPhase::Moved, Vec2::new(150.0, 250.0), 1000.0);
        input.cancel_all();
        assert_eq!(input.take_move(), MoveIntent::default());
        assert_eq!(input.take_look(), Vec2::ZERO);
        assert!(!input.take_fire());
        assert!(input.move_touch.is_none() && input.look_touch.is_none());
    }

    #[test]
    fn releasing_one_direction_binding_keeps_its_other_binding_held() {
        let mut input = InputState::default();
        input.key(KeyCode::KeyW, ElementState::Pressed, false);
        input.key(KeyCode::ArrowUp, ElementState::Pressed, false);
        input.key(KeyCode::KeyW, ElementState::Released, false);
        assert_eq!(input.take_move().local, Vec2::Y);
        input.key(KeyCode::ArrowUp, ElementState::Released, false);
        assert_eq!(input.take_move().local, Vec2::ZERO);
    }
    #[test]
    fn releasing_one_shift_keeps_the_other_shift_sprinting() {
        let mut input = InputState::default();
        input.key(KeyCode::ShiftLeft, ElementState::Pressed, false);
        input.key(KeyCode::ShiftRight, ElementState::Pressed, false);
        input.key(KeyCode::ShiftLeft, ElementState::Released, false);
        assert!(input.take_move().sprint);
        input.key(KeyCode::ShiftRight, ElementState::Released, false);
        assert!(!input.take_move().sprint);
    }

    #[test]
    fn pitch_is_clamped_away_from_camera_poles() {
        let mut camera = CameraRig::default();
        camera.apply_look(Vec2::new(0.0, 1000.0));
        assert!((camera.pitch - -75_f32.to_radians()).abs() < f32::EPSILON);
        camera.apply_look(Vec2::new(0.0, -2000.0));
        assert!((camera.pitch - -5_f32.to_radians()).abs() < f32::EPSILON);
    }

    #[test]
    fn wheel_zoom_is_visible_and_bounded() {
        let mut camera = CameraRig::default();
        camera.zoom(2.0);
        assert!((camera.distance - 9.0).abs() < f32::EPSILON);
        camera.zoom(100.0);
        assert!((camera.distance - 4.0).abs() < f32::EPSILON);
        camera.zoom(-100.0);
        assert!((camera.distance - 36.0).abs() < f32::EPSILON);
    }

    #[test]
    fn right_touch_tap_queues_jump_but_drag_only_rotates() {
        let mut input = InputState::default();
        input.touch_event(1, TouchPhase::Started, Vec2::new(800.0, 200.0), 1000.0);
        input.touch_event(1, TouchPhase::Ended, Vec2::new(802.0, 201.0), 1000.0);
        assert!(input.take_move().jump_pressed);

        input.touch_event(2, TouchPhase::Started, Vec2::new(800.0, 200.0), 1000.0);
        input.touch_event(2, TouchPhase::Moved, Vec2::new(850.0, 220.0), 1000.0);
        input.touch_event(2, TouchPhase::Ended, Vec2::new(850.0, 220.0), 1000.0);
        assert!(!input.take_move().jump_pressed);
        assert_ne!(input.take_look(), Vec2::ZERO);
    }
}

/// Mouse targets are in radians, independent of render rate. Simulation positions
/// are interpolated by the caller; this rig smooths view angles without changing input.
#[derive(Clone, Copy, Debug)]
pub struct WalkCamera {
    pub yaw: f32,
    pub pitch: f32,
    target_yaw: f32,
    target_pitch: f32,
    pub distance: f32,
    target_distance: f32,
}
impl WalkCamera {
    pub fn new(direction: Vec3) -> Self {
        let yaw = direction.x.atan2(direction.z);
        let pitch = direction.y.clamp(-1., 1.).asin();
        Self {
            yaw,
            pitch,
            target_yaw: yaw,
            target_pitch: pitch,
            distance: 0.,
            target_distance: 0.,
        }
    }
    pub fn desired_distance(self) -> f32 {
        self.target_distance
    }
    pub fn set_distance(&mut self, distance: f32) {
        if distance.is_finite() {
            self.target_distance = distance.clamp(0., 8.);
        }
    }
    pub fn zoom(&mut self, lines: f32) {
        if lines.is_finite() {
            self.set_distance(self.target_distance - lines * 0.25);
        }
    }
    pub fn apply_look(&mut self, delta: Vec2) {
        if delta.is_finite() {
            self.target_yaw -= delta.x;
            self.target_pitch = (self.target_pitch - delta.y).clamp(-1.50, 1.50);
        }
    }
    pub fn step(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0. {
            let alpha = -(-40. * dt).exp_m1();
            self.yaw += (self.target_yaw - self.yaw) * alpha;
            self.pitch += (self.target_pitch - self.pitch) * alpha;
            self.distance += (self.target_distance - self.distance) * alpha;
        }
    }
    pub fn direction(self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }
    pub fn planar_basis(self) -> (Vec3, Vec3) {
        let forward = Vec3::new(self.yaw.sin(), 0., self.yaw.cos());
        (forward, Vec3::new(-forward.z, 0., forward.x))
    }
}
#[cfg(test)]
mod walk_camera_tests {
    use super::*;
    #[test]
    fn smoothing_is_independent_of_frame_subdivision() {
        let mut a = WalkCamera::new(Vec3::NEG_Z);
        a.apply_look(Vec2::new(1., 0.5));
        let mut b = a;
        a.step(0.1);
        for _ in 0..20 {
            b.step(0.005);
        }
        assert!(a.direction().distance(b.direction()) < 1e-6);
    }
    #[test]
    fn zoom_is_smooth_bounded_and_independent_of_frame_rate() {
        let mut a = WalkCamera::new(Vec3::NEG_Z);
        a.zoom(-100.);
        let mut b = a;
        assert_eq!(a.desired_distance(), 8.);
        assert_eq!(a.distance, 0.);
        a.step(0.1);
        for _ in 0..20 {
            b.step(0.005);
        }
        assert!((a.distance - b.distance).abs() < 1e-6);
        assert!(a.distance > 0. && a.distance < 8.);
    }
    #[test]
    fn invalid_look_and_time_do_not_poison_camera() {
        let mut camera = WalkCamera::new(Vec3::NEG_Z);
        camera.apply_look(Vec2::splat(f32::NAN));
        camera.step(f32::INFINITY);
        assert!(camera.direction().is_finite());
        assert!(camera.direction().distance(Vec3::NEG_Z) < 1e-6);
    }
}
