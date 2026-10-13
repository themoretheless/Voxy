//! Compiled pose graph: a flat arena of nodes evaluated bottom-up in a fixed
//! topological order. Leaves own clocks; interior nodes are pure functions of
//! their inputs. A tick stages all clocks and parameters and publishes them
//! only on success, matching [`Animator`](crate::Animator) transactional
//! semantics. State machines activate only their current (and fading) state
//! subtrees; inactive subtrees keep their clocks.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};

use crate::state_machine::{
    ActiveFade, CompiledState, MachineState, MachineStatus, PhaseSync, StateId, StateMachine,
    StateMachineDesc,
};
use crate::{
    AnimationClip, AnimationError, BlendSpace1D, BlendSpace2D, BoneMask, Pose, Skeleton, rigs_match,
};

/// Upper bound on nodes per compiled graph.
pub const MAX_GRAPH_NODES: usize = 1024;

/// Index of a parameter in the graph's parameter set.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParamId(pub u16);

/// Index of a node in the graph arena.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub u16);

/// Runtime-settable parameter value. Kinds are fixed at compile time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamValue {
    Float(f32),
    Bool(bool),
    Int(i32),
}

impl ParamValue {
    fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Float(_), Self::Float(_))
                | (Self::Bool(_), Self::Bool(_))
                | (Self::Int(_), Self::Int(_))
        )
    }
}

/// Scalar node input: a constant or a float parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloatInput {
    Const(f32),
    Param(ParamId),
}

/// Node kinds. Interior nodes reference children by [`NodeId`].
#[derive(Clone, Debug)]
pub enum GraphNode {
    /// Clip playback with its own clock and root-motion integral.
    Clip {
        clip: Arc<AnimationClip>,
        speed: FloatInput,
    },
    /// Phase-synchronized 1D blend space with its own normalized phase clock.
    BlendSpace1D {
        space: Arc<BlendSpace1D>,
        parameter: FloatInput,
    },
    /// Phase-synchronized 2D blend space with its own normalized phase clock.
    BlendSpace2D {
        space: Arc<BlendSpace2D>,
        x: FloatInput,
        y: FloatInput,
    },
    /// Linear blend `a -> b` by weight in `[0, 1]`.
    Blend {
        a: NodeId,
        b: NodeId,
        weight: FloatInput,
    },
    /// Masked override of `base` by `overlay`, scaled by `weight`.
    Layer {
        base: NodeId,
        overlay: NodeId,
        mask: Arc<BoneMask>,
        weight: FloatInput,
    },
    /// `base + (additive - reference)` by mask, scaled by `weight`.
    Additive {
        base: NodeId,
        additive: NodeId,
        reference: Arc<Pose>,
        mask: Arc<BoneMask>,
        weight: FloatInput,
    },
    /// Named states over subtrees with parameter-driven crossfades.
    StateMachine(StateMachineDesc),
}

impl GraphNode {
    fn children(&self, out: &mut Vec<NodeId>) {
        out.clear();
        match self {
            Self::Clip { .. } | Self::BlendSpace1D { .. } | Self::BlendSpace2D { .. } => {}
            Self::Blend { a, b, .. } => out.extend([*a, *b]),
            Self::Layer { base, overlay, .. } => out.extend([*base, *overlay]),
            Self::Additive { base, additive, .. } => out.extend([*base, *additive]),
            Self::StateMachine(desc) => out.extend(desc.states.iter().map(|(_, n)| *n)),
        }
    }

    fn float_inputs(&self) -> [Option<FloatInput>; 2] {
        match self {
            Self::Clip { speed, .. } => [Some(*speed), None],
            Self::BlendSpace1D { parameter, .. } => [Some(*parameter), None],
            Self::BlendSpace2D { x, y, .. } => [Some(*x), Some(*y)],
            Self::Blend { weight, .. }
            | Self::Layer { weight, .. }
            | Self::Additive { weight, .. } => [Some(*weight), None],
            Self::StateMachine(_) => [None, None],
        }
    }
}

/// Authoring-time description consumed by [`AnimGraph::compile`].
#[derive(Clone, Debug, Default)]
pub struct GraphDesc {
    pub nodes: Vec<GraphNode>,
    pub params: Vec<(Arc<str>, ParamValue)>,
    pub root: Option<NodeId>,
}

impl GraphDesc {
    /// Appends a node and returns its id.
    ///
    /// # Errors
    /// Rejects more than [`MAX_GRAPH_NODES`] nodes.
    pub fn add_node(&mut self, node: GraphNode) -> Result<NodeId, AnimationError> {
        if self.nodes.len() >= MAX_GRAPH_NODES {
            return Err(AnimationError::GraphNodeBudget);
        }
        let id = NodeId(self.nodes.len() as u16);
        self.nodes.push(node);
        Ok(id)
    }

    /// Declares a named parameter and returns its id.
    ///
    /// # Errors
    /// Rejects an empty or duplicate name.
    pub fn add_param(
        &mut self,
        name: &str,
        initial: ParamValue,
    ) -> Result<ParamId, AnimationError> {
        if name.is_empty() || self.params.iter().any(|(n, _)| n.as_ref() == name) {
            return Err(AnimationError::InvalidGraphParam);
        }
        if self.params.len() >= usize::from(u16::MAX) {
            return Err(AnimationError::InvalidGraphParam);
        }
        let id = ParamId(self.params.len() as u16);
        self.params.push((Arc::from(name), initial));
        Ok(id)
    }
}

/// Compiled node: authored kind plus resolved machine data.
#[derive(Clone, Debug)]
enum CompiledNode {
    Plain(GraphNode),
    Machine(StateMachine),
}

/// Validated, immutable graph shared by any number of players.
#[derive(Clone, Debug)]
pub struct AnimGraph {
    joints: Arc<[crate::Joint]>,
    nodes: Arc<[CompiledNode]>,
    /// Reachable nodes, children before parents.
    order: Arc<[NodeId]>,
    root: NodeId,
    params: Arc<[(Arc<str>, ParamValue)]>,
    by_name: Arc<HashMap<Arc<str>, ParamId>>,
}

impl AnimGraph {
    /// Validates node references, parameter kinds, rig compatibility and
    /// acyclicity, and fixes a bottom-up evaluation order.
    ///
    /// # Errors
    /// Rejects a missing root, dangling node/param ids, a non-float input,
    /// clips/spaces/masks/poses for another rig, invalid machines, or a cycle.
    pub fn compile(desc: GraphDesc, skeleton: &Skeleton) -> Result<Self, AnimationError> {
        let root = desc
            .root
            .ok_or(AnimationError::InvalidGraphNode(usize::MAX))?;
        let count = desc.nodes.len();
        if count == 0 || count > MAX_GRAPH_NODES {
            return Err(AnimationError::GraphNodeBudget);
        }
        let joint_count = skeleton.joints().len();
        let mut children = Vec::new();
        for (index, node) in desc.nodes.iter().enumerate() {
            node.children(&mut children);
            if children.iter().any(|c| usize::from(c.0) >= count) {
                return Err(AnimationError::InvalidGraphNode(index));
            }
            for input in node.float_inputs().into_iter().flatten() {
                match input {
                    FloatInput::Const(value) if !value.is_finite() => {
                        return Err(AnimationError::InvalidGraphParam);
                    }
                    FloatInput::Param(id) => match desc.params.get(usize::from(id.0)) {
                        Some((_, ParamValue::Float(_))) => {}
                        _ => return Err(AnimationError::InvalidGraphParam),
                    },
                    FloatInput::Const(_) => {}
                }
            }
            let compatible = match node {
                GraphNode::Clip { clip, .. } => clip.is_compatible_with(skeleton),
                GraphNode::BlendSpace1D { space, .. } => space
                    .samples()
                    .iter()
                    .all(|s| s.clip.is_compatible_with(skeleton)),
                GraphNode::BlendSpace2D { space, .. } => space
                    .samples()
                    .iter()
                    .all(|s| s.clip.is_compatible_with(skeleton)),
                GraphNode::Blend { .. } | GraphNode::StateMachine(_) => true,
                GraphNode::Layer { mask, .. } => mask.weights().len() == joint_count,
                GraphNode::Additive {
                    mask, reference, ..
                } => {
                    mask.weights().len() == joint_count
                        && reference.local().len() == joint_count
                        && rigs_match(&reference.rig, &skeleton.joints)
                }
            };
            if !compatible {
                return Err(AnimationError::SkeletonMismatch);
            }
        }
        let order = topological_order(&desc.nodes, root)?;
        let nodes = desc
            .nodes
            .iter()
            .map(|node| match node {
                GraphNode::StateMachine(machine) => {
                    compile_machine(machine, &desc.nodes, &desc.params).map(CompiledNode::Machine)
                }
                other => Ok(CompiledNode::Plain(other.clone())),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let by_name = desc
            .params
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), ParamId(index as u16)))
            .collect();
        Ok(Self {
            joints: skeleton.joints.clone(),
            nodes: nodes.into(),
            order: order.into(),
            root,
            params: desc.params.into(),
            by_name: Arc::new(by_name),
        })
    }

    #[must_use]
    pub fn param(&self, name: &str) -> Option<ParamId> {
        self.by_name.get(name).copied()
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Nodes reachable from the root, children before parents.
    #[must_use]
    pub fn evaluation_order(&self) -> &[NodeId] {
        &self.order
    }

    /// Compiled machine of a state-machine node.
    #[must_use]
    pub fn state_machine(&self, node: NodeId) -> Option<&StateMachine> {
        match self.nodes.get(usize::from(node.0))? {
            CompiledNode::Machine(machine) => Some(machine),
            CompiledNode::Plain(_) => None,
        }
    }
}

fn compile_machine(
    desc: &StateMachineDesc,
    nodes: &[GraphNode],
    params: &[(Arc<str>, ParamValue)],
) -> Result<StateMachine, AnimationError> {
    if desc.states.is_empty() {
        return Err(AnimationError::InvalidStateMachine);
    }
    let entry = desc.entry.ok_or(AnimationError::InvalidStateMachine)?;
    if usize::from(entry.0) >= desc.states.len() {
        return Err(AnimationError::InvalidStateMachine);
    }
    let states = desc
        .states
        .iter()
        .map(|(name, node)| {
            // A state may not contain the machine itself; the graph-level
            // cycle check already rejects that, so this DFS terminates.
            let subtree = topological_order(nodes, *node)?;
            Ok(CompiledState {
                name: name.clone(),
                node: *node,
                subtree: subtree.into(),
            })
        })
        .collect::<Result<Vec<_>, AnimationError>>()?;
    for transition in &desc.transitions {
        let valid_state = |id: StateId| usize::from(id.0) < states.len();
        if !valid_state(transition.to) || transition.from.is_some_and(|f| !valid_state(f)) {
            return Err(AnimationError::InvalidStateMachine);
        }
        if !transition.duration.is_finite() || !(0.0..=60.0).contains(&transition.duration) {
            return Err(AnimationError::InvalidTransitionDuration);
        }
        if let PhaseSync::At(phase) = transition.sync
            && (!phase.is_finite() || !(0.0..=1.0).contains(&phase))
        {
            return Err(AnimationError::InvalidAnimationPhase);
        }
        transition.condition.validate(params)?;
    }
    Ok(StateMachine {
        states: states.into(),
        transitions: desc.transitions.clone().into(),
        entry,
    })
}

// Iterative DFS from `root`: only reachable nodes get clocks and evaluation.
fn topological_order(nodes: &[GraphNode], root: NodeId) -> Result<Vec<NodeId>, AnimationError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        OnStack,
        Done,
    }
    let mut marks = vec![Mark::Unvisited; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    let mut stack: Vec<(NodeId, usize)> = vec![(root, 0)];
    marks[usize::from(root.0)] = Mark::OnStack;
    let mut children = Vec::new();
    while let Some(&mut (node, ref mut next)) = stack.last_mut() {
        nodes[usize::from(node.0)].children(&mut children);
        let mut descended = false;
        while *next < children.len() {
            let child = children[*next];
            *next += 1;
            match marks[usize::from(child.0)] {
                Mark::OnStack => return Err(AnimationError::GraphCycle),
                Mark::Done => {}
                Mark::Unvisited => {
                    marks[usize::from(child.0)] = Mark::OnStack;
                    stack.push((child, 0));
                    descended = true;
                    break;
                }
            }
        }
        if !descended {
            marks[usize::from(node.0)] = Mark::Done;
            order.push(node);
            stack.pop();
        }
    }
    Ok(order)
}

/// Output of one graph tick.
#[derive(Clone, Debug)]
pub struct GraphFrame {
    pub pose: Pose,
    pub skin_matrices: Vec<Mat4>,
    /// Root-joint translation displacement over the tick, blended with the
    /// same weights as the pose.
    pub root_motion: Vec3,
}

/// Per-node runtime state. Clips store clip time; blend spaces store
/// normalized phase; machines store their current state and fade.
#[derive(Clone, Debug)]
enum NodeState {
    Clock(f64),
    Machine(MachineState),
}

/// Mutable playback state over a shared [`AnimGraph`].
#[derive(Clone, Debug)]
pub struct GraphPlayer {
    graph: Arc<AnimGraph>,
    states: Vec<NodeState>,
    params: Vec<ParamValue>,
}

impl GraphPlayer {
    /// Starts every clock at zero and every machine at its entry state, with
    /// the declared parameter defaults.
    ///
    /// # Errors
    /// Rejects a graph whose initial pose or palette is invalid.
    pub fn new(graph: Arc<AnimGraph>) -> Result<Self, AnimationError> {
        let player = Self {
            states: graph.nodes.iter().map(initial_state).collect(),
            params: graph.params.iter().map(|(_, value)| *value).collect(),
            graph,
        };
        let mut probe = player.clone();
        probe.advance(0.0)?;
        Ok(player)
    }

    #[must_use]
    pub fn graph(&self) -> &Arc<AnimGraph> {
        &self.graph
    }

    /// # Errors
    /// Rejects an unknown id or a value of another kind.
    pub fn set_param(&mut self, id: ParamId, value: ParamValue) -> Result<(), AnimationError> {
        let slot = self
            .params
            .get_mut(usize::from(id.0))
            .ok_or(AnimationError::InvalidGraphParam)?;
        if !slot.same_kind(value) {
            return Err(AnimationError::InvalidGraphParam);
        }
        if let ParamValue::Float(v) = value
            && !v.is_finite()
        {
            return Err(AnimationError::InvalidGraphParam);
        }
        *slot = value;
        Ok(())
    }

    #[must_use]
    pub fn param(&self, id: ParamId) -> Option<ParamValue> {
        self.params.get(usize::from(id.0)).copied()
    }

    /// Clip time or normalized blend-space phase of a leaf node.
    #[must_use]
    pub fn clock(&self, node: NodeId) -> Option<f64> {
        match self.states.get(usize::from(node.0))? {
            NodeState::Clock(time) => Some(*time),
            NodeState::Machine(_) => None,
        }
    }

    /// Current state and fade of a state-machine node.
    #[must_use]
    pub fn machine_status(&self, node: NodeId) -> Option<MachineStatus> {
        match self.states.get(usize::from(node.0))? {
            NodeState::Machine(state) => Some(MachineStatus {
                current: state.current,
                fading_from: state.fade.as_ref().map(|f| f.source),
                weight: state
                    .fade
                    .as_ref()
                    .map_or(1.0, |f| f.curve.weight(f.elapsed / f.duration) as f32),
            }),
            NodeState::Clock(_) => None,
        }
    }

    /// Advances every active leaf by `dt` seconds and evaluates the root.
    /// Any error leaves clocks, machines and parameters unchanged.
    ///
    /// # Errors
    /// Rejects `dt` outside `[0, 1]` or nonfinite, and any sampling/blend error.
    pub fn advance(&mut self, dt: f64) -> Result<GraphFrame, AnimationError> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut staged = Staged {
            states: self.states.clone(),
            params: self.params.clone(),
        };
        let frame = self.evaluate(&mut staged, dt)?;
        self.states = staged.states;
        self.params = staged.params;
        Ok(frame)
    }

    fn evaluate(&self, staged: &mut Staged, dt: f64) -> Result<GraphFrame, AnimationError> {
        let skeleton = Skeleton {
            joints: self.graph.joints.clone(),
        };
        let graph = &self.graph;
        let ticks = self.activate(staged, dt)?;
        let mut results: Vec<Option<(Pose, Vec3)>> = vec![None; graph.nodes.len()];
        for &id in graph.order.iter() {
            let index = usize::from(id.0);
            let Some(dt) = ticks[index] else {
                continue;
            };
            let value = match &graph.nodes[index] {
                CompiledNode::Plain(node) => {
                    self.evaluate_plain(node, index, &skeleton, staged, &mut results, dt)?
                }
                CompiledNode::Machine(machine) => {
                    evaluate_machine(machine, index, staged, &mut results, dt)?
                }
            };
            results[index] = Some(value);
        }
        let (pose, root_motion) = results[usize::from(graph.root.0)]
            .take()
            .ok_or(AnimationError::InvalidGraphNode(usize::from(graph.root.0)))?;
        let skin_matrices = pose.skin_matrices(&skeleton)?;
        Ok(GraphFrame {
            pose,
            skin_matrices,
            root_motion,
        })
    }

    /// Top-down pass: decides machine transitions, then assigns each active
    /// node its tick. Fading source subtrees receive only the remaining fade
    /// time, as in [`Animator`](crate::Animator).
    fn activate(&self, staged: &mut Staged, dt: f64) -> Result<Vec<Option<f64>>, AnimationError> {
        let graph = &self.graph;
        let mut ticks: Vec<Option<f64>> = vec![None; graph.nodes.len()];
        ticks[usize::from(graph.root.0)] = Some(dt);
        let mut children = Vec::new();
        let assign = |ticks: &mut [Option<f64>], node: NodeId, dt: f64| {
            let slot = &mut ticks[usize::from(node.0)];
            *slot = Some(slot.map_or(dt, |old| old.max(dt)));
        };
        for &id in graph.order.iter().rev() {
            let index = usize::from(id.0);
            let Some(dt) = ticks[index] else {
                continue;
            };
            match &graph.nodes[index] {
                CompiledNode::Plain(node) => {
                    node.children(&mut children);
                    for &child in &children {
                        assign(&mut ticks, child, dt);
                    }
                }
                CompiledNode::Machine(machine) => {
                    self.decide_transition(machine, index, staged)?;
                    let NodeState::Machine(state) = &staged.states[index] else {
                        return Err(AnimationError::InvalidGraphNode(index));
                    };
                    assign(
                        &mut ticks,
                        machine.states[usize::from(state.current.0)].node,
                        dt,
                    );
                    if let Some(fade) = &state.fade
                        && fade.frozen.is_none()
                    {
                        let fade_dt = dt.min((fade.duration - fade.elapsed).max(0.0));
                        assign(
                            &mut ticks,
                            machine.states[usize::from(fade.source.0)].node,
                            fade_dt,
                        );
                    }
                }
            }
        }
        Ok(ticks)
    }

    fn leaf_phase(&self, node: NodeId, staged: &Staged) -> Option<f64> {
        let index = usize::from(node.0);
        match (&self.graph.nodes[index], &staged.states[index]) {
            (CompiledNode::Plain(GraphNode::Clip { clip, .. }), NodeState::Clock(time)) => {
                Some(clip.phase(*time) / f64::from(clip.duration))
            }
            (
                CompiledNode::Plain(
                    GraphNode::BlendSpace1D { .. } | GraphNode::BlendSpace2D { .. },
                ),
                NodeState::Clock(phase),
            ) => Some(*phase),
            _ => None,
        }
    }

    fn set_leaf_phase(&self, node: NodeId, phase: f64, staged: &mut Staged) {
        let index = usize::from(node.0);
        let value = match &self.graph.nodes[index] {
            CompiledNode::Plain(GraphNode::Clip { clip, .. }) => phase * f64::from(clip.duration),
            CompiledNode::Plain(
                GraphNode::BlendSpace1D { .. } | GraphNode::BlendSpace2D { .. },
            ) => phase,
            _ => return,
        };
        staged.states[index] = NodeState::Clock(value);
    }

    fn decide_transition(
        &self,
        machine: &StateMachine,
        index: usize,
        staged: &mut Staged,
    ) -> Result<(), AnimationError> {
        let NodeState::Machine(state) = &staged.states[index] else {
            return Err(AnimationError::InvalidGraphNode(index));
        };
        let current = state.current;
        let fading = state.fade.is_some();
        let displayed = state.displayed.clone();
        let phase = self.leaf_phase(machine.states[usize::from(current.0)].node, staged);
        let chosen = machine
            .transitions
            .iter()
            .filter(|t| t.from.is_none_or(|from| from == current) && t.to != current)
            .filter(|t| !fading || t.interruptible)
            .filter(|t| t.condition.holds(&staged.params, phase))
            .max_by_key(|t| t.priority);
        let Some(transition) = chosen else {
            return Ok(());
        };
        transition.condition.consume_triggers(&mut staged.params);
        let target = &machine.states[usize::from(transition.to.0)];
        // Entering a state restarts its subtree unless phase is carried over.
        for &node in target.subtree.iter() {
            let i = usize::from(node.0);
            staged.states[i] = initial_state(&self.graph.nodes[i]);
        }
        match transition.sync {
            PhaseSync::Restart => {}
            PhaseSync::Preserve => {
                if let Some(phase) = phase {
                    self.set_leaf_phase(target.node, phase, staged);
                }
            }
            PhaseSync::At(phase) => self.set_leaf_phase(target.node, phase, staged),
        }
        let fade = if transition.duration == 0.0 {
            None
        } else {
            Some(ActiveFade {
                source: current,
                // Interrupting a fade continues from the displayed blend.
                frozen: if fading { displayed.clone() } else { None },
                elapsed: 0.0,
                duration: f64::from(transition.duration),
                curve: transition.curve,
            })
        };
        staged.states[index] = NodeState::Machine(MachineState {
            current: transition.to,
            fade,
            displayed,
        });
        Ok(())
    }

    fn evaluate_plain(
        &self,
        node: &GraphNode,
        index: usize,
        skeleton: &Skeleton,
        staged: &mut Staged,
        results: &mut [Option<(Pose, Vec3)>],
        dt: f64,
    ) -> Result<(Pose, Vec3), AnimationError> {
        let float = |input: FloatInput| match input {
            FloatInput::Const(v) => v,
            FloatInput::Param(id) => match staged.params[usize::from(id.0)] {
                ParamValue::Float(v) => v,
                // Compile guarantees float kind.
                ParamValue::Bool(_) | ParamValue::Int(_) => 0.0,
            },
        };
        let clock = |staged: &Staged| match staged.states[index] {
            NodeState::Clock(time) => time,
            NodeState::Machine(_) => 0.0,
        };
        Ok(match node {
            GraphNode::Clip { clip, speed } => {
                let speed = f64::from(float(*speed));
                if !speed.is_finite() {
                    return Err(AnimationError::InvalidPlaybackSpeed);
                }
                // Same clock reduction order as `Animator::advance_candidate_wall`.
                let old_time = clip.phase(clock(staged));
                let new_time = old_time + dt * speed;
                let root_motion = clip
                    .root_curve
                    .integral(old_time, new_time, 1., 1.)
                    .as_vec3();
                if !new_time.is_finite() || !root_motion.is_finite() {
                    return Err(AnimationError::NumericalOverflow);
                }
                let pose = clip.try_sample_clock(skeleton, new_time)?;
                staged.states[index] = NodeState::Clock(clip.phase(new_time));
                (pose, root_motion)
            }
            GraphNode::BlendSpace1D { space, parameter } => {
                let parameter = float(*parameter);
                let duration = f64::from(space.cycle_duration(parameter)?);
                let phase = (clock(staged) + dt / duration).rem_euclid(1.0);
                let pose = space.evaluate(skeleton, parameter, phase)?;
                staged.states[index] = NodeState::Clock(phase);
                (pose, Vec3::ZERO)
            }
            GraphNode::BlendSpace2D { space, x, y } => {
                let p = Vec2::new(float(*x), float(*y));
                let duration = f64::from(space.cycle_duration(p)?);
                let phase = (clock(staged) + dt / duration).rem_euclid(1.0);
                let pose = space.evaluate(skeleton, p, phase)?;
                staged.states[index] = NodeState::Clock(phase);
                (pose, Vec3::ZERO)
            }
            GraphNode::Blend { a, b, weight } => {
                let (pa, ma) = take(results, *a)?;
                let (pb, mb) = take(results, *b)?;
                let w = float(*weight);
                let pose = Pose::blend(&pa, &pb, w)?;
                let w = w.clamp(0.0, 1.0);
                (pose, ma + (mb - ma) * w)
            }
            GraphNode::Layer {
                base,
                overlay,
                mask,
                weight,
            } => {
                let (pb, mb) = take(results, *base)?;
                let (po, _) = take(results, *overlay)?;
                let mask = mask.scaled(float(*weight))?;
                (Pose::blend_masked(&pb, &po, &mask)?, mb)
            }
            GraphNode::Additive {
                base,
                additive,
                reference,
                mask,
                weight,
            } => {
                let (pb, mb) = take(results, *base)?;
                let (pa, _) = take(results, *additive)?;
                let mask = mask.scaled(float(*weight))?;
                (Pose::apply_additive(&pb, &pa, reference, &mask)?, mb)
            }
            GraphNode::StateMachine(_) => return Err(AnimationError::InvalidGraphNode(index)),
        })
    }
}

/// Staged copy of mutable player state for one tick.
struct Staged {
    states: Vec<NodeState>,
    params: Vec<ParamValue>,
}

fn initial_state(node: &CompiledNode) -> NodeState {
    match node {
        CompiledNode::Plain(_) => NodeState::Clock(0.0),
        CompiledNode::Machine(machine) => NodeState::Machine(MachineState {
            current: machine.entry,
            fade: None,
            displayed: None,
        }),
    }
}

fn take(
    results: &mut [Option<(Pose, Vec3)>],
    child: NodeId,
) -> Result<(Pose, Vec3), AnimationError> {
    results[usize::from(child.0)]
        .take()
        .ok_or(AnimationError::InvalidGraphNode(usize::from(child.0)))
}

fn evaluate_machine(
    machine: &StateMachine,
    index: usize,
    staged: &mut Staged,
    results: &mut [Option<(Pose, Vec3)>],
    dt: f64,
) -> Result<(Pose, Vec3), AnimationError> {
    let NodeState::Machine(state) = &mut staged.states[index] else {
        return Err(AnimationError::InvalidGraphNode(index));
    };
    let (target, target_motion) = take(results, machine.states[usize::from(state.current.0)].node)?;
    let Some(fade) = &mut state.fade else {
        let shared = Arc::new(target);
        state.displayed = Some(shared.clone());
        return Ok(((*shared).clone(), target_motion));
    };
    let start = fade.curve.weight(fade.elapsed / fade.duration);
    fade.elapsed = (fade.elapsed + dt).min(fade.duration);
    let end = fade.curve.weight(fade.elapsed / fade.duration);
    let complete = fade.elapsed >= fade.duration;
    let (source, source_motion) = match &fade.frozen {
        Some(frozen) => ((**frozen).clone(), Vec3::ZERO),
        None => take(results, machine.states[usize::from(fade.source.0)].node)?,
    };
    // Trapezoidal weighting of each subtree's displacement over the tick.
    let mean = ((start + end) * 0.5) as f32;
    let root_motion = source_motion + (target_motion - source_motion) * mean;
    let pose = if complete {
        target
    } else {
        Pose::blend(&source, &target, end as f32)?
    };
    if complete {
        state.fade = None;
    }
    let shared = Arc::new(pose);
    state.displayed = Some(shared.clone());
    Ok(((*shared).clone(), root_motion))
}

#[cfg(test)]
mod tests;
