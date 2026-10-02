//! Platform-neutral named actions. Platform adapters supply physical control values.
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
    actions: BTreeMap<String, Action>,
    controls: BTreeMap<Control, f32>,
    max_actions: usize,
    max_bindings: usize,
    focused: bool,
}
impl InputMap {
    #[must_use]
    pub fn new(max_actions: usize, max_bindings: usize) -> Self {
        Self {
            actions: BTreeMap::new(),
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
        let previous = self.actions.get(name);
        let total: usize = self.actions.values().map(|a| a.bindings.len()).sum();
        let old_len = previous.map_or(0, |a| a.bindings.len());
        if (previous.is_none() && self.actions.len() >= self.max_actions)
            || total - old_len + bindings.len() > self.max_bindings
        {
            return Err(InputError::Capacity);
        }
        let state = previous.map_or(ActionState::default(), |a| a.state);
        let dead_zone = previous.map_or(0.0, |a| a.dead_zone);
        self.actions.insert(
            name.into(),
            Action {
                bindings,
                state,
                dead_zone,
            },
        );
        // Drop controls no longer referenced by any binding, keeping memory bounded.
        self.controls.retain(|control, _| {
            self.actions
                .values()
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
                .actions
                .values()
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
        let action = self
            .actions
            .get_mut(name)
            .ok_or(InputError::UnknownAction)?;
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
        let action = self
            .actions
            .get_mut(name)
            .ok_or(InputError::UnknownAction)?;
        action.dead_zone = threshold;
        self.recompute();
        Ok(())
    }

    fn recompute(&mut self) {
        for action in self.actions.values_mut() {
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
            for action in self.actions.values_mut() {
                action.state.pressed = false;
            }
        }
    }
    /// Disconnecting one device releases only its controls.
    pub fn disconnect(&mut self, device: u32) {
        self.controls.retain(|control, _| control.device != device);
        self.recompute();
    }
    #[must_use]
    pub fn state(&self, name: &str) -> Option<ActionState> {
        self.actions.get(name).map(|a| a.state)
    }
    pub fn finish_frame(&mut self) {
        for action in self.actions.values_mut() {
            action.state.pressed = false;
            action.state.released = false;
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
}
