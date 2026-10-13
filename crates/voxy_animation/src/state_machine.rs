//! State machine node: named states over graph subtrees, parameter-driven
//! transitions with crossfades, interruption and phase synchronization.
//! Runtime state lives in the owning [`GraphPlayer`](crate::GraphPlayer).

use std::sync::Arc;

use crate::anim_graph::{NodeId, ParamId, ParamValue};
use crate::{AnimationError, Pose};

/// Index of a state in its machine.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(pub u16);

/// Transition predicate over parameters and the current state's phase.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Always,
    FloatGt(ParamId, f32),
    FloatLt(ParamId, f32),
    Bool(ParamId, bool),
    IntEq(ParamId, i32),
    /// A bool parameter consumed (reset to `false`) when the transition fires.
    Trigger(ParamId),
    /// Current state's leaf phase in `[0, 1]` reached this value.
    PhaseAtLeast(f64),
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

impl Condition {
    pub(crate) fn validate(&self, params: &[(Arc<str>, ParamValue)]) -> Result<(), AnimationError> {
        let kind = |id: ParamId| params.get(usize::from(id.0)).map(|(_, v)| *v);
        match self {
            Self::Always => Ok(()),
            Self::FloatGt(id, v) | Self::FloatLt(id, v) => match kind(*id) {
                Some(ParamValue::Float(_)) if v.is_finite() => Ok(()),
                _ => Err(AnimationError::InvalidGraphParam),
            },
            Self::Bool(id, _) | Self::Trigger(id) => match kind(*id) {
                Some(ParamValue::Bool(_)) => Ok(()),
                _ => Err(AnimationError::InvalidGraphParam),
            },
            Self::IntEq(id, _) => match kind(*id) {
                Some(ParamValue::Int(_)) => Ok(()),
                _ => Err(AnimationError::InvalidGraphParam),
            },
            Self::PhaseAtLeast(phase) => {
                if phase.is_finite() && (0.0..=1.0).contains(phase) {
                    Ok(())
                } else {
                    Err(AnimationError::InvalidAnimationPhase)
                }
            }
            Self::All(items) | Self::Any(items) => {
                items.iter().try_for_each(|c| c.validate(params))
            }
        }
    }

    /// `phase` is the current state's leaf phase, if it has one.
    pub(crate) fn holds(&self, params: &[ParamValue], phase: Option<f64>) -> bool {
        match self {
            Self::Always => true,
            Self::FloatGt(id, v) => {
                matches!(params[usize::from(id.0)], ParamValue::Float(x) if x > *v)
            }
            Self::FloatLt(id, v) => {
                matches!(params[usize::from(id.0)], ParamValue::Float(x) if x < *v)
            }
            Self::Bool(id, v) => {
                matches!(params[usize::from(id.0)], ParamValue::Bool(x) if x == *v)
            }
            Self::Trigger(id) => matches!(params[usize::from(id.0)], ParamValue::Bool(true)),
            Self::IntEq(id, v) => {
                matches!(params[usize::from(id.0)], ParamValue::Int(x) if x == *v)
            }
            Self::PhaseAtLeast(v) => phase.is_some_and(|p| p >= *v),
            Self::All(items) => items.iter().all(|c| c.holds(params, phase)),
            Self::Any(items) => items.iter().any(|c| c.holds(params, phase)),
        }
    }

    pub(crate) fn consume_triggers(&self, params: &mut [ParamValue]) {
        match self {
            Self::Trigger(id) => params[usize::from(id.0)] = ParamValue::Bool(false),
            Self::All(items) | Self::Any(items) => {
                for c in items {
                    c.consume_triggers(params);
                }
            }
            _ => {}
        }
    }
}

/// Crossfade weight curve over normalized elapsed time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FadeCurve {
    #[default]
    Linear,
    SmoothStep,
}

impl FadeCurve {
    #[must_use]
    pub fn weight(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// How the target state's leaf clock is initialized on entry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PhaseSync {
    /// Restart the target subtree from zero.
    #[default]
    Restart,
    /// Copy the source leaf's normalized phase (feet stay aligned).
    Preserve,
    /// Start at a fixed normalized phase.
    At(f64),
}

/// Edge between states. `from = None` means "from any state".
#[derive(Clone, Debug)]
pub struct StateTransition {
    pub from: Option<StateId>,
    pub to: StateId,
    pub condition: Condition,
    /// Seconds; zero switches immediately.
    pub duration: f32,
    pub curve: FadeCurve,
    pub sync: PhaseSync,
    /// Whether this edge may fire while another crossfade is still active.
    pub interruptible: bool,
    /// Higher wins when several edges hold in the same tick.
    pub priority: u8,
}

/// Authoring-time description of one machine.
#[derive(Clone, Debug, Default)]
pub struct StateMachineDesc {
    pub states: Vec<(Arc<str>, NodeId)>,
    pub transitions: Vec<StateTransition>,
    pub entry: Option<StateId>,
}

impl StateMachineDesc {
    /// # Errors
    /// Rejects an empty or duplicate state name.
    pub fn add_state(&mut self, name: &str, node: NodeId) -> Result<StateId, AnimationError> {
        if name.is_empty() || self.states.iter().any(|(n, _)| n.as_ref() == name) {
            return Err(AnimationError::InvalidStateMachine);
        }
        if self.states.len() >= usize::from(u16::MAX) {
            return Err(AnimationError::InvalidStateMachine);
        }
        let id = StateId(self.states.len() as u16);
        self.states.push((Arc::from(name), node));
        Ok(id)
    }
}

/// Compiled, immutable machine. Subtrees are resolved by the graph compiler.
#[derive(Clone, Debug)]
pub struct StateMachine {
    pub(crate) states: Arc<[CompiledState]>,
    pub(crate) transitions: Arc<[StateTransition]>,
    pub(crate) entry: StateId,
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledState {
    pub name: Arc<str>,
    pub node: NodeId,
    /// Every node reachable from `node`, children before parents.
    pub subtree: Arc<[NodeId]>,
}

impl StateMachine {
    #[must_use]
    pub fn state(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|s| s.name.as_ref() == name)
            .map(|i| StateId(i as u16))
    }

    #[must_use]
    pub fn state_node(&self, id: StateId) -> Option<NodeId> {
        self.states.get(usize::from(id.0)).map(|s| s.node)
    }

    #[must_use]
    pub fn entry(&self) -> StateId {
        self.entry
    }
}

/// Active crossfade between two states.
#[derive(Clone, Debug)]
pub(crate) struct ActiveFade {
    pub source: StateId,
    /// Displayed blend captured when an earlier fade was interrupted.
    pub frozen: Option<Arc<Pose>>,
    pub elapsed: f64,
    pub duration: f64,
    pub curve: FadeCurve,
}

/// Runtime state of one machine node inside a player.
#[derive(Clone, Debug)]
pub(crate) struct MachineState {
    pub current: StateId,
    pub fade: Option<ActiveFade>,
    /// Pose displayed last tick; the interruption snapshot source.
    pub displayed: Option<Arc<Pose>>,
}

/// Read-only view of a machine node's runtime state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MachineStatus {
    pub current: StateId,
    pub fading_from: Option<StateId>,
    /// Target weight in `[0, 1]`; one when no fade is active.
    pub weight: f32,
}

#[cfg(test)]
mod tests;
