//! Platform-neutral named actions. Platform adapters supply physical control values.
mod checkpoint;
pub use checkpoint::{ActionCheckpoint, InputCheckpoint};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Control {
    pub device: u32,
    pub code: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binding {
    pub control: Control,
    pub scale: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionState {
    pub value: f32,
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    Capacity,
    InvalidBinding,
    UnknownAction,
    InvalidValue,
}
impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "input error: {self:?}")
    }
}
impl std::error::Error for InputError {}
/// Dense integer handle for $O(1)$ per-tick action polling without string hashing or tree traversal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionId(u32);

#[derive(Debug)]
struct Action {
    bindings: Vec<Binding>,
    state: ActionState,
    dead_zone: f32,
}
/// Action values are summed and clamped to [-1,1]; held means nonzero magnitude.
/// Press/release transitions accumulate until `finish_frame`, preserving quick taps.
/// Control identifiers are supplied by the adapter, not durable platform key names.
#[derive(Debug)]
pub struct InputMap {
    action_ids: BTreeMap<String, ActionId>,
    action_slots: Vec<Action>,
    controls: BTreeMap<Control, f32>,
    max_actions: usize,
    max_bindings: usize,
    focused: bool,
}
impl InputMap {
    #[must_use]
    pub fn new(max_actions: usize, max_bindings: usize) -> Self {
        Self {
            action_ids: BTreeMap::new(),
            action_slots: Vec::new(),
            controls: BTreeMap::new(),
            max_actions,
            max_bindings,
            focused: true,
        }
    }
    /// Adds/replaces bindings atomically. Rebinding immediately observes held controls.
    /// # Errors
    /// Rejects empty names, nonfinite/scales outside [-1,1], duplicate controls and limits.
    pub fn bind(&mut self, name: &str, bindings: Vec<Binding>) -> Result<(), InputError> {
        if name.is_empty()
            || bindings
                .iter()
                .any(|b| !b.scale.is_finite() || b.scale.abs() > 1.0)
        {
            return Err(InputError::InvalidBinding);
        }
        for (index, binding) in bindings.iter().enumerate() {
            if bindings[..index]
                .iter()
                .any(|b| b.control == binding.control)
            {
                return Err(InputError::InvalidBinding);
            }
        }
        let existing_id = self.action_ids.get(name).copied();
        let previous = existing_id.map(|id| &self.action_slots[id.0 as usize]);
        let total: usize = self.action_slots.iter().map(|a| a.bindings.len()).sum();
        let old_len = previous.map_or(0, |a| a.bindings.len());
        if (previous.is_none() && self.action_slots.len() >= self.max_actions)
            || (previous.is_none() && u32::try_from(self.action_slots.len()).is_err())
            || total - old_len + bindings.len() > self.max_bindings
        {
            return Err(InputError::Capacity);
        }
        let state = previous.map_or(ActionState::default(), |a| a.state);
        let dead_zone = previous.map_or(0.0, |a| a.dead_zone);
        let next_action = Action {
            bindings,
            state,
            dead_zone,
        };
        if let Some(id) = existing_id {
            self.action_slots[id.0 as usize] = next_action;
        } else {
            let id = ActionId(self.action_slots.len() as u32);
            self.action_slots.push(next_action);
            self.action_ids.insert(name.into(), id);
        }
        // Drop controls no longer referenced by any binding, keeping memory bounded.
        self.controls.retain(|control, _| {
            self.action_slots
                .iter()
                .any(|a| a.bindings.iter().any(|b| b.control == *control))
        });
        self.recompute();
        Ok(())
    }
    /// # Errors
    /// Rejects nonfinite or out-of-range values. Unbound controls are ignored.
    pub fn event(&mut self, control: Control, value: f32) -> Result<(), InputError> {
        if !value.is_finite() || value.abs() > 1.0 {
            return Err(InputError::InvalidValue);
        }
        if !self.focused
            || !self
                .action_slots
                .iter()
                .any(|a| a.bindings.iter().any(|b| b.control == control))
        {
            return Ok(());
        }
        self.controls.insert(control, value);
        self.recompute();
        Ok(())
    }
    /// Adds a completed logical activation to the current input frame. Physical
    /// control values/holds are preserved; quick activations survive until a
    /// successful consumer calls `finish_frame`.
    /// # Errors
    /// Rejects an unknown named action.
    pub fn activate(&mut self, name: &str) -> Result<(), InputError> {
        let id = self
            .action_ids
            .get(name)
            .copied()
            .ok_or(InputError::UnknownAction)?;
        let action = &mut self.action_slots[id.0 as usize];
        if self.focused {
            action.state.pressed = true;
            action.state.released |= !action.state.held;
        }
        Ok(())
    }
    /// Sets a symmetric scalar dead zone after binding aggregation. Values beyond
    /// the threshold are rescaled to the full range; existing held state updates.
    /// # Errors
    /// Rejects unknown actions and thresholds outside [0,1), including NaN.
    pub fn set_dead_zone(&mut self, name: &str, threshold: f32) -> Result<(), InputError> {
        if !threshold.is_finite() || !(0.0..1.0).contains(&threshold) {
            return Err(InputError::InvalidValue);
        }
        let id = self
            .action_ids
            .get(name)
            .copied()
            .ok_or(InputError::UnknownAction)?;
        self.action_slots[id.0 as usize].dead_zone = threshold;
        self.recompute();
        Ok(())
    }

    fn recompute(&mut self) {
        for action in &mut self.action_slots {
            let value = action
                .bindings
                .iter()
                .map(|b| self.controls.get(&b.control).copied().unwrap_or(0.0) * b.scale)
                .sum::<f32>()
                .clamp(-1.0, 1.0);
            let value = if value.abs() <= action.dead_zone {
                0.0
            } else {
                value.signum() * (value.abs() - action.dead_zone) / (1.0 - action.dead_zone)
            };
            let held = value.abs() > 1e-6;
            action.state.pressed |= held && !action.state.held;
            action.state.released |= !held && action.state.held;
            action.state.value = value;
            action.state.held = held;
        }
    }
    /// Clears held physical controls and pending presses on focus loss.
    /// Releases remain observable; focus gain requires fresh events.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.controls.clear();
            self.recompute();
            // Focus loss cancels unconsumed presses; releases remain observable.
            for action in &mut self.action_slots {
                action.state.pressed = false;
            }
        }
    }
    /// Disconnecting one device releases only its controls.
    pub fn disconnect(&mut self, device: u32) {
        self.controls.retain(|control, _| control.device != device);
        self.recompute();
    }
    /// Resolves a bound action name to a dense [`ActionId`] for $O(1)$ polling.
    #[must_use]
    pub fn action_id(&self, name: &str) -> Option<ActionId> {
        self.action_ids.get(name).copied()
    }
    /// Reads an action's current state in $O(1)$ by [`ActionId`].
    #[must_use]
    #[inline]
    pub fn state_by_id(&self, id: ActionId) -> Option<ActionState> {
        self.action_slots.get(id.0 as usize).map(|a| a.state)
    }
    #[must_use]
    pub fn state(&self, name: &str) -> Option<ActionState> {
        self.action_ids
            .get(name)
            .map(|id| self.action_slots[id.0 as usize].state)
    }
    pub fn finish_frame(&mut self) {
        for action in &mut self.action_slots {
            action.state.pressed = false;
            action.state.released = false;
        }
    }
}

/// Applies a radial deadzone and power response curve to a 2D analog stick vector.
/// Preserves direction while mapping magnitudes in `(dead_zone, 1.0]` smoothly to `(0.0, 1.0]`.
///
/// # Errors
/// Rejects non-finite inputs, `dead_zone` outside `[0.0, 1.0)`, or `exponent <= 0.0`.
pub fn apply_radial_deadzone(
    stick: [f32; 2],
    dead_zone: f32,
    exponent: f32,
) -> Result<[f32; 2], InputError> {
    if !stick[0].is_finite()
        || !stick[1].is_finite()
        || !dead_zone.is_finite()
        || !(0.0..1.0).contains(&dead_zone)
        || !exponent.is_finite()
        || exponent <= 0.0
    {
        return Err(InputError::InvalidValue);
    }
    let mag = stick[0].hypot(stick[1]);
    if mag <= dead_zone {
        return Ok([0.0, 0.0]);
    }
    let clamped = mag.min(1.0);
    let normalized = (clamped - dead_zone) / (1.0 - dead_zone);
    let curved = normalized.powf(exponent);
    let scale = curved / mag;
    Ok([stick[0] * scale, stick[1] * scale])
}

/// Fixed-capacity action lookahead buffer (e.g. 150 ms pre-land jump buffering).
#[derive(Clone, Debug, PartialEq)]
pub struct ActionBuffer {
    entries: BTreeMap<String, f32>,
    max_entries: usize,
}

impl ActionBuffer {
    #[must_use]
    pub const fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
        }
    }

    /// Records or refreshes a buffered action with a positive time-to-live in seconds.
    ///
    /// # Errors
    /// Rejects empty action names, non-positive/non-finite TTL, or capacity overflow.
    pub fn record(&mut self, action: &str, ttl_seconds: f32) -> Result<(), InputError> {
        if action.is_empty() {
            return Err(InputError::InvalidBinding);
        }
        if !ttl_seconds.is_finite() || ttl_seconds <= 0.0 {
            return Err(InputError::InvalidValue);
        }
        if !self.entries.contains_key(action) && self.entries.len() >= self.max_entries {
            return Err(InputError::Capacity);
        }
        self.entries.insert(action.to_owned(), ttl_seconds);
        Ok(())
    }

    /// Advances timers by `dt_seconds` and expires elapsed buffered actions.
    pub fn step(&mut self, dt_seconds: f32) {
        if !dt_seconds.is_finite() || dt_seconds <= 0.0 {
            return;
        }
        self.entries.retain(|_, remaining| {
            *remaining -= dt_seconds;
            *remaining > 0.0
        });
    }

    /// Consumes the buffered action if it is still active.
    pub fn consume(&mut self, action: &str) -> bool {
        self.entries.remove(action).is_some()
    }
}

/// Evaluates a single-step 1€ (One-Euro) adaptive low-pass filter (Casiez et al. 2012)
/// that smooths slow jitter via `min_cutoff_hz` while opening bandwidth via `beta` during fast motion.
#[must_use]
pub fn one_euro_smooth(
    prev_value: f32,
    raw_value: f32,
    dt_s: f32,
    min_cutoff_hz: f32,
    beta: f32,
) -> f32 {
    if !raw_value.is_finite() {
        return if prev_value.is_finite() {
            prev_value
        } else {
            0.0
        };
    }
    if !prev_value.is_finite()
        || !dt_s.is_finite()
        || dt_s <= 0.0
        || !min_cutoff_hz.is_finite()
        || min_cutoff_hz <= 0.0
    {
        return raw_value;
    }
    let speed = ((raw_value - prev_value) / dt_s).abs();
    let b = if beta.is_finite() && beta >= 0.0 {
        beta
    } else {
        0.0
    };
    let cutoff = min_cutoff_hz + b * speed;
    let tau = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
    let alpha = (1.0 / (1.0 + tau / dt_s)).clamp(0.0, 1.0);
    prev_value + alpha * (raw_value - prev_value)
}

/// Stateful double-tap detector with configurable inter-tap window and hold-duration guard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleTapDetector {
    pub max_inter_tap_s: f32,
    pub max_hold_s: f32,
    elapsed_s: f32,
    press_started_s: Option<f32>,
    first_release_s: Option<f32>,
}

impl DoubleTapDetector {
    #[must_use]
    pub fn new(max_inter_tap_s: f32, max_hold_s: f32) -> Option<Self> {
        if !max_inter_tap_s.is_finite()
            || !max_hold_s.is_finite()
            || max_inter_tap_s <= 0.0
            || max_hold_s <= 0.0
        {
            return None;
        }
        Some(Self {
            max_inter_tap_s,
            max_hold_s,
            elapsed_s: 0.0,
            press_started_s: None,
            first_release_s: None,
        })
    }

    /// Advances the internal clock and processes a press (`true`) or release (`false`) edge,
    /// returning `true` when a valid second tap completes within `max_inter_tap_s`.
    pub fn observe_edge(&mut self, dt_s: f32, pressed: bool) -> bool {
        if dt_s.is_finite() && dt_s > 0.0 {
            self.elapsed_s += dt_s;
        }
        if pressed {
            self.press_started_s = Some(self.elapsed_s);
            false
        } else {
            let Some(start) = self.press_started_s.take() else {
                return false;
            };
            if self.elapsed_s - start > self.max_hold_s {
                self.first_release_s = None;
                return false;
            }
            if let Some(prev_release) = self.first_release_s {
                if self.elapsed_s - prev_release <= self.max_inter_tap_s {
                    self.first_release_s = None;
                    return true;
                }
            }
            self.first_release_s = Some(self.elapsed_s);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LEFT: Control = Control { device: 0, code: 1 };
    const RIGHT: Control = Control { device: 0, code: 2 };
    fn binding(control: Control, scale: f32) -> Binding {
        Binding { control, scale }
    }
    #[test]
    fn logical_activation_preserves_physical_hold_and_fixed_frame_edges() {
        let mut map = InputMap::new(1, 1);
        map.bind("jump", vec![binding(LEFT, 1.0)]).unwrap();
        map.activate("jump").unwrap();
        let pulse = map.state("jump").unwrap();
        assert!(pulse.pressed && pulse.released && !pulse.held);
        assert_eq!(pulse.value.to_bits(), 0.0_f32.to_bits());
        map.event(LEFT, 0.0).unwrap();
        assert!(map.state("jump").unwrap().pressed);
        map.finish_frame();
        assert!(!map.state("jump").unwrap().pressed);
        map.event(LEFT, 1.0).unwrap();
        map.finish_frame();
        map.activate("jump").unwrap();
        let held = map.state("jump").unwrap();
        assert!(held.held && held.pressed && !held.released);
        assert_eq!(held.value.to_bits(), 1.0_f32.to_bits());
        map.set_focused(false);
        map.activate("jump").unwrap();
        assert!(!map.state("jump").unwrap().pressed);
        assert_eq!(map.activate("unknown"), Err(InputError::UnknownAction));
    }
    #[test]
    fn taps_repeat_and_multiple_bindings_preserve_edges() {
        let mut map = InputMap::new(2, 4);
        map.bind("jump", vec![binding(LEFT, 1.0), binding(RIGHT, 1.0)])
            .unwrap();
        map.event(LEFT, 1.0).unwrap();
        map.finish_frame();
        map.event(LEFT, 1.0).unwrap();
        assert!(!map.state("jump").unwrap().pressed);
        map.event(RIGHT, 1.0).unwrap();
        map.event(LEFT, 0.0).unwrap();
        assert!(map.state("jump").unwrap().held);
        map.event(RIGHT, 0.0).unwrap();
        assert!(map.state("jump").unwrap().released);
        map.finish_frame();
        map.event(LEFT, 1.0).unwrap();
        map.event(LEFT, 0.0).unwrap();
        let state = map.state("jump").unwrap();
        assert!(state.pressed && state.released && !state.held);
    }
    #[test]
    fn axis_rebinding_focus_and_disconnect() {
        let mut map = InputMap::new(1, 2);
        map.bind("move", vec![binding(LEFT, -1.0), binding(RIGHT, 1.0)])
            .unwrap();
        map.event(LEFT, 1.0).unwrap();
        assert_eq!(
            map.state("move").unwrap().value.to_bits(),
            (-1.0_f32).to_bits()
        );
        map.event(RIGHT, 1.0).unwrap();
        assert!(map.state("move").unwrap().value.abs() < f32::EPSILON);
        map.bind("move", vec![binding(RIGHT, 1.0)]).unwrap();
        assert_eq!(
            map.state("move").unwrap().value.to_bits(),
            1.0_f32.to_bits()
        );
        map.set_focused(false);
        assert!(map.state("move").unwrap().released);
        map.event(RIGHT, 1.0).unwrap();
        assert!(!map.state("move").unwrap().held);
        map.set_focused(true);
        map.event(RIGHT, 0.5).unwrap();
        map.disconnect(0);
        assert!(map.state("move").unwrap().value.abs() < f32::EPSILON);
    }
    #[test]
    fn invalid_bindings_and_capacity_preserve_previous_configuration() {
        let mut map = InputMap::new(1, 1);
        map.bind("jump", vec![binding(LEFT, 1.0)]).unwrap();
        assert_eq!(map.bind("other", vec![]), Err(InputError::Capacity));
        assert_eq!(
            map.bind("jump", vec![binding(RIGHT, f32::NAN)]),
            Err(InputError::InvalidBinding)
        );
        assert_eq!(map.event(LEFT, 2.0), Err(InputError::InvalidValue));
        map.event(LEFT, 1.0).unwrap();
        assert!(map.state("jump").unwrap().held);
    }
    #[test]
    fn disconnection_releases_only_its_device_and_focus_cancels_pending_taps() {
        let second = Control { device: 1, code: 1 };
        let mut map = InputMap::new(2, 2);
        map.bind("first", vec![binding(LEFT, 1.0)]).unwrap();
        map.bind("second", vec![binding(second, 1.0)]).unwrap();
        map.event(LEFT, 1.0).unwrap();
        map.event(second, 1.0).unwrap();
        map.finish_frame();
        map.disconnect(0);
        assert!(map.state("first").unwrap().released);
        assert!(!map.state("first").unwrap().held);
        assert!(map.state("second").unwrap().held);
        assert!(!map.state("second").unwrap().released);
        map.event(LEFT, 1.0).unwrap();
        map.event(LEFT, 0.0).unwrap();
        assert!(map.state("first").unwrap().pressed);
        map.set_focused(false);
        assert!(!map.state("first").unwrap().pressed);
        assert!(map.state("second").unwrap().released);
        map.set_focused(true);
        assert!(!map.state("second").unwrap().held);
    }
    #[test]
    fn analog_dead_zone_rejects_drift_rescales_and_preserves_rebinding() {
        let mut map = InputMap::new(1, 1);
        map.bind("move", vec![binding(LEFT, 1.0)]).unwrap();
        map.set_dead_zone("move", 0.2).unwrap();
        map.event(LEFT, 0.1).unwrap();
        assert!(!map.state("move").unwrap().held);
        map.event(LEFT, 0.6).unwrap();
        assert!((map.state("move").unwrap().value - 0.5).abs() < 1e-6);
        map.event(LEFT, -1.0).unwrap();
        assert!((map.state("move").unwrap().value + 1.0).abs() < 1e-6);
        map.bind("move", vec![binding(LEFT, 1.0)]).unwrap();
        map.event(LEFT, 0.1).unwrap();
        assert!(map.state("move").unwrap().released);
        assert_eq!(
            map.set_dead_zone("move", 1.0),
            Err(InputError::InvalidValue)
        );
        assert_eq!(
            map.set_dead_zone("missing", 0.1),
            Err(InputError::UnknownAction)
        );
    }
    #[test]
    fn radial_deadzone_and_action_buffer_window() {
        assert_eq!(
            apply_radial_deadzone([0.1, 0.1], 0.2, 1.0).unwrap(),
            [0.0, 0.0]
        );
        let out = apply_radial_deadzone([0.6, 0.0], 0.2, 2.0).unwrap();
        assert!((out[0] - 0.25).abs() < 1e-6 && out[1].abs() < 1e-6);
        assert_eq!(
            apply_radial_deadzone([0.0, 0.0], 1.0, 1.0),
            Err(InputError::InvalidValue)
        );

        let mut buffer = ActionBuffer::new(1);
        buffer.record("jump", 0.15).unwrap();
        assert_eq!(buffer.record("dash", 0.15), Err(InputError::Capacity));
        buffer.step(0.10);
        assert!(buffer.consume("jump"));
        assert!(!buffer.consume("jump"));
        buffer.record("jump", 0.05).unwrap();
        buffer.step(0.06);
        assert!(!buffer.consume("jump"));

        let slow = one_euro_smooth(0.0, 0.01, 1.0 / 60.0, 1.0, 0.5);
        let fast = one_euro_smooth(0.0, 10.0, 1.0 / 60.0, 1.0, 0.5);
        assert!(slow / 0.01 < fast / 10.0);

        let mut dtap = DoubleTapDetector::new(0.25, 0.15).unwrap();
        assert!(!dtap.observe_edge(0.0, true));
        assert!(!dtap.observe_edge(0.05, false));
        assert!(!dtap.observe_edge(0.10, true));
        assert!(dtap.observe_edge(0.05, false));
    }
}
