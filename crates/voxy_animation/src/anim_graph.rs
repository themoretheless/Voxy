//! Compiled pose graph: a flat arena of nodes evaluated bottom-up in a fixed
//! topological order. Leaves own clocks; interior nodes are pure functions of
//! their inputs. A tick stages all clocks and publishes them only on success,
//! matching [`Animator`](crate::Animator) transactional semantics.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};

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
}

impl GraphNode {
    fn children(&self) -> [Option<NodeId>; 2] {
        match self {
            Self::Clip { .. } | Self::BlendSpace1D { .. } | Self::BlendSpace2D { .. } => {
                [None, None]
            }
            Self::Blend { a, b, .. } => [Some(*a), Some(*b)],
            Self::Layer { base, overlay, .. } => [Some(*base), Some(*overlay)],
            Self::Additive { base, additive, .. } => [Some(*base), Some(*additive)],
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

/// Validated, immutable graph shared by any number of players.
#[derive(Clone, Debug)]
pub struct AnimGraph {
    joints: Arc<[crate::Joint]>,
    nodes: Arc<[GraphNode]>,
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
    /// clips/spaces/masks/poses for another rig, or a cycle.
    pub fn compile(desc: GraphDesc, skeleton: &Skeleton) -> Result<Self, AnimationError> {
        let root = desc
            .root
            .ok_or(AnimationError::InvalidGraphNode(usize::MAX))?;
        let count = desc.nodes.len();
        if count == 0 || count > MAX_GRAPH_NODES {
            return Err(AnimationError::GraphNodeBudget);
        }
        let joint_count = skeleton.joints().len();
        for (index, node) in desc.nodes.iter().enumerate() {
            for child in node.children().into_iter().flatten() {
                if usize::from(child.0) >= count {
                    return Err(AnimationError::InvalidGraphNode(index));
                }
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
                GraphNode::Blend { .. } => true,
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
        let by_name = desc
            .params
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (name.clone(), ParamId(index as u16)))
            .collect();
        Ok(Self {
            joints: skeleton.joints.clone(),
            nodes: desc.nodes.into(),
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
}

// Iterative DFS from the root: only reachable nodes get clocks and evaluation.
fn topological_order(nodes: &[GraphNode], root: NodeId) -> Result<Vec<NodeId>, AnimationError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        OnStack,
        Done,
    }
    let mut marks = vec![Mark::Unvisited; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    let mut stack: Vec<(NodeId, u8)> = vec![(root, 0)];
    marks[usize::from(root.0)] = Mark::OnStack;
    while let Some(&mut (node, ref mut next)) = stack.last_mut() {
        let children = nodes[usize::from(node.0)].children();
        let mut descended = false;
        while usize::from(*next) < children.len() {
            let slot = *next;
            *next += 1;
            if let Some(child) = children[usize::from(slot)] {
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

/// Per-node clock. Clips store clip time; blend spaces store normalized phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct NodeClock(f64);

/// Mutable playback state over a shared [`AnimGraph`].
#[derive(Clone, Debug)]
pub struct GraphPlayer {
    graph: Arc<AnimGraph>,
    clocks: Vec<NodeClock>,
    params: Vec<ParamValue>,
}

impl GraphPlayer {
    /// Starts every clock at zero with the declared parameter defaults.
    ///
    /// # Errors
    /// Rejects a graph whose initial pose or palette is invalid.
    pub fn new(graph: Arc<AnimGraph>) -> Result<Self, AnimationError> {
        let player = Self {
            clocks: vec![NodeClock::default(); graph.nodes.len()],
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
        self.clocks.get(usize::from(node.0)).map(|c| c.0)
    }

    /// Advances every reachable leaf by `dt` seconds and evaluates the root.
    /// Any error leaves clocks and parameters unchanged.
    ///
    /// # Errors
    /// Rejects `dt` outside `[0, 1]` or nonfinite, and any sampling/blend error.
    pub fn advance(&mut self, dt: f64) -> Result<GraphFrame, AnimationError> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut clocks = self.clocks.clone();
        let frame = self.evaluate(&mut clocks, dt)?;
        self.clocks = clocks;
        Ok(frame)
    }

    fn float(&self, input: FloatInput) -> f32 {
        match input {
            FloatInput::Const(v) => v,
            FloatInput::Param(id) => match self.params[usize::from(id.0)] {
                ParamValue::Float(v) => v,
                // Compile guarantees float kind.
                ParamValue::Bool(_) | ParamValue::Int(_) => 0.0,
            },
        }
    }

    fn evaluate(&self, clocks: &mut [NodeClock], dt: f64) -> Result<GraphFrame, AnimationError> {
        let skeleton = Skeleton {
            joints: self.graph.joints.clone(),
        };
        let graph = &self.graph;
        let mut results: Vec<Option<(Pose, Vec3)>> = vec![None; graph.nodes.len()];
        for &id in graph.order.iter() {
            let index = usize::from(id.0);
            let take = |results: &mut [Option<(Pose, Vec3)>], child: NodeId| {
                results[usize::from(child.0)]
                    .take()
                    .ok_or(AnimationError::InvalidGraphNode(usize::from(child.0)))
            };
            let value = match &graph.nodes[index] {
                GraphNode::Clip { clip, speed } => {
                    let speed = f64::from(self.float(*speed));
                    if !speed.is_finite() {
                        return Err(AnimationError::InvalidPlaybackSpeed);
                    }
                    // Same clock reduction order as `Animator::advance_candidate_wall`.
                    let old_time = clip.phase(clocks[index].0);
                    let new_time = old_time + dt * speed;
                    let root_motion = clip
                        .root_curve
                        .integral(old_time, new_time, 1., 1.)
                        .as_vec3();
                    if !new_time.is_finite() || !root_motion.is_finite() {
                        return Err(AnimationError::NumericalOverflow);
                    }
                    let pose = clip.try_sample_clock(&skeleton, new_time)?;
                    clocks[index] = NodeClock(clip.phase(new_time));
                    (pose, root_motion)
                }
                GraphNode::BlendSpace1D { space, parameter } => {
                    let parameter = self.float(*parameter);
                    let duration = f64::from(space.cycle_duration(parameter)?);
                    let phase = (clocks[index].0 + dt / duration).rem_euclid(1.0);
                    let pose = space.evaluate(&skeleton, parameter, phase)?;
                    clocks[index] = NodeClock(phase);
                    (pose, Vec3::ZERO)
                }
                GraphNode::BlendSpace2D { space, x, y } => {
                    let p = Vec2::new(self.float(*x), self.float(*y));
                    let duration = f64::from(space.cycle_duration(p)?);
                    let phase = (clocks[index].0 + dt / duration).rem_euclid(1.0);
                    let pose = space.evaluate(&skeleton, p, phase)?;
                    clocks[index] = NodeClock(phase);
                    (pose, Vec3::ZERO)
                }
                GraphNode::Blend { a, b, weight } => {
                    let (pa, ma) = take(&mut results, *a)?;
                    let (pb, mb) = take(&mut results, *b)?;
                    let w = self.float(*weight);
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
                    let (pb, mb) = take(&mut results, *base)?;
                    let (po, _) = take(&mut results, *overlay)?;
                    let mask = mask.scaled(self.float(*weight))?;
                    (Pose::blend_masked(&pb, &po, &mask)?, mb)
                }
                GraphNode::Additive {
                    base,
                    additive,
                    reference,
                    mask,
                    weight,
                } => {
                    let (pb, mb) = take(&mut results, *base)?;
                    let (pa, _) = take(&mut results, *additive)?;
                    let mask = mask.scaled(self.float(*weight))?;
                    (Pose::apply_additive(&pb, &pa, reference, &mask)?, mb)
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
}

#[cfg(test)]
mod tests;
