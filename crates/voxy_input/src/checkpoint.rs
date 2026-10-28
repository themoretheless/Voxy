//! Codec-neutral exact input state for an enclosing game-save transaction.
use super::*;

/// Integer float bits let an outer file codec retain the precise input state.
/// Physical control codes belong to the platform adapter; the outer save must
/// check adapter/project compatibility before exposing a restored game session.
#[derive(Clone, Debug, PartialEq)]
pub struct InputCheckpoint {
    pub version: u32,
    pub focused: bool,
    pub actions: Vec<ActionCheckpoint>,
    pub controls: Vec<(Control, u32)>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActionCheckpoint {
    pub name: String,
    pub bindings: Vec<(Control, u32)>,
    pub dead_zone_bits: u32,
    pub value_bits: u32,
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
}
impl InputMap {
    /// Capture held controls AND unconsumed logical edges, without finishing a frame.
    #[must_use]
    pub fn checkpoint(&self) -> InputCheckpoint {
        InputCheckpoint {
            version: 1,
            focused: self.focused,
            controls: self
                .controls
                .iter()
                .map(|(&control, &value)| (control, value.to_bits()))
                .collect(),
            actions: self
                .actions
                .iter()
                .map(|(name, action)| ActionCheckpoint {
                    name: name.clone(),
                    bindings: action
                        .bindings
                        .iter()
                        .map(|b| (b.control, b.scale.to_bits()))
                        .collect(),
                    dead_zone_bits: action.dead_zone.to_bits(),
                    value_bits: action.state.value.to_bits(),
                    held: action.state.held,
                    pressed: action.state.pressed,
                    released: action.state.released,
                })
                .collect(),
        }
    }
    /// Restore into a fully admitted candidate and retain this map's capacity policy.
    /// Recomputing verifies saved values/holds; it does not replay or erase saved edges.
    /// File byte limits and physical-device reconciliation belong to the caller.
    /// # Errors
    /// Rejects versions, capacities, duplicate/unbound controls/actions, invalid
    /// numbers and state inconsistent with its bindings/focus. Failure is atomic.
    pub fn restore_checkpoint(&mut self, saved: &InputCheckpoint) -> Result<(), InputError> {
        if saved.version != 1 {
            return Err(InputError::InvalidValue);
        }
        if saved.actions.len() > self.max_actions || saved.controls.len() > self.max_bindings {
            return Err(InputError::Capacity);
        }
        let total = saved
            .actions
            .iter()
            .try_fold(0usize, |sum, a| sum.checked_add(a.bindings.len()))
            .ok_or(InputError::Capacity)?;
        if total > self.max_bindings {
            return Err(InputError::Capacity);
        }
        let mut candidate = Self::new(self.max_actions, self.max_bindings);
        for action in &saved.actions {
            if candidate.actions.contains_key(&action.name) {
                return Err(InputError::InvalidBinding);
            }
            candidate.bind(
                &action.name,
                action
                    .bindings
                    .iter()
                    .map(|&(control, scale)| Binding {
                        control,
                        scale: f32::from_bits(scale),
                    })
                    .collect(),
            )?;
            candidate.set_dead_zone(&action.name, f32::from_bits(action.dead_zone_bits))?;
        }
        for &(control, bits) in &saved.controls {
            let value = f32::from_bits(bits);
            if !value.is_finite()
                || value.abs() > 1.
                || !saved.focused
                || !candidate
                    .actions
                    .values()
                    .any(|a| a.bindings.iter().any(|b| b.control == control))
                || candidate.controls.insert(control, value).is_some()
            {
                return Err(InputError::InvalidValue);
            }
        }
        candidate.focused = saved.focused;
        for action in &saved.actions {
            let value = f32::from_bits(action.value_bits);
            if !value.is_finite() || value.abs() > 1. || (!saved.focused && action.pressed) {
                return Err(InputError::InvalidValue);
            }
            candidate
                .actions
                .get_mut(&action.name)
                .ok_or(InputError::UnknownAction)?
                .state = ActionState {
                value,
                held: action.held,
                pressed: action.pressed,
                released: action.released,
            };
        }
        candidate.recompute();
        for action in &saved.actions {
            let state = candidate
                .state(&action.name)
                .ok_or(InputError::UnknownAction)?;
            if state.value.to_bits() != action.value_bits
                || state.held != action.held
                || state.pressed != action.pressed
                || state.released != action.released
            {
                return Err(InputError::InvalidValue);
            }
        }
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> InputMap {
        let mut input = InputMap::new(4, 8);
        input
            .bind(
                "move",
                vec![Binding {
                    control: Control { device: 1, code: 1 },
                    scale: 1.,
                }],
            )
            .unwrap();
        input
            .bind(
                "jump",
                vec![Binding {
                    control: Control { device: 2, code: 2 },
                    scale: 1.,
                }],
            )
            .unwrap();
        input.set_dead_zone("move", 0.1).unwrap();
        input
    }
    #[test]
    fn checkpoint_keeps_analog_quick_taps_logical_edges_and_focus() {
        let mut source = input();
        source.event(Control { device: 1, code: 1 }, 0.375).unwrap();
        source.event(Control { device: 2, code: 2 }, 1.).unwrap();
        source.event(Control { device: 2, code: 2 }, 0.).unwrap();
        source.activate("jump").unwrap();
        let saved = source.checkpoint();
        let mut target = InputMap::new(4, 8);
        target.restore_checkpoint(&saved).unwrap();
        assert_eq!(target.checkpoint(), saved);
        assert!(target.state("jump").unwrap().pressed && target.state("jump").unwrap().released);
        for focused in [true, false, true] {
            source.set_focused(focused);
            target.set_focused(focused);
            source.disconnect(1);
            target.disconnect(1);
            assert_eq!(source.checkpoint(), target.checkpoint());
            let mut reloaded = InputMap::new(4, 8);
            reloaded.restore_checkpoint(&source.checkpoint()).unwrap();
            assert_eq!(reloaded.checkpoint(), source.checkpoint());
            source.finish_frame();
            target.finish_frame();
            assert_eq!(source.checkpoint(), target.checkpoint());
        }
    }
    #[test]
    fn invalid_checkpoint_retains_values_bindings_and_pending_edges() {
        let mut map = input();
        map.activate("jump").unwrap();
        let saved = map.checkpoint();
        let mut version = saved.clone();
        version.version = 2;
        let mut duplicate = saved.clone();
        duplicate.actions.push(duplicate.actions[0].clone());
        let mut value = saved.clone();
        value.actions[0].value_bits = 0.5f32.to_bits();
        let mut nonfinite = saved.clone();
        nonfinite.actions[0].dead_zone_bits = f32::NAN.to_bits();
        let mut unbound = saved.clone();
        unbound.controls.push((
            Control {
                device: 99,
                code: 99,
            },
            0,
        ));
        for bad in [version, duplicate, value, nonfinite, unbound] {
            assert!(map.restore_checkpoint(&bad).is_err());
            assert_eq!(map.checkpoint(), saved);
        }
        let mut tiny = InputMap::new(1, 1);
        assert!(tiny.restore_checkpoint(&saved).is_err());
        assert!(tiny.checkpoint().actions.is_empty());
    }
}
