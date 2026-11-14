//! Fixed-sample trigger transitions over the shared scene collision geometry.
use crate::convex::AffineBox;
use crate::support::MAX_COLLIDERS;
use crate::{CharacterBody, CharacterPhysics, CharacterTranslationTrace};
use crate::{PhysicsError, SceneQueries, SupportQueryBudget, affine_box, validate_extents};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use voxy_scene::{
    LoadedScene, NodeId, ObjectId, SceneCheckpointView, SceneGraph, SceneGraphError, SceneId,
};
pub use voxy_scene::{TriggerEvent, TriggerPhase};

const MAX_TRIGGERS: usize = 4096;
pub const MAX_PAIRS: usize = 65_536;

/// Nonblocking box with the node's published affine basis and query center.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxTrigger {
    pub half_extents: [f32; 3],
}

#[derive(Debug)]
pub struct TriggerTracker {
    scene: SceneId,
    pairs: BTreeSet<(NodeId, NodeId)>,
    max_pairs: usize,
}

impl TriggerTracker {
    /// # Errors
    /// Rejects a pair limit above the supported ceiling.
    pub fn new(scene: &SceneGraph, max_pairs: usize) -> Result<Self, PhysicsError> {
        if max_pairs > MAX_PAIRS {
            return Err(PhysicsError::Capacity);
        }
        Ok(Self {
            scene: scene.identity(),
            pairs: BTreeSet::new(),
            max_pairs,
        })
    }

    #[must_use]
    pub fn contains(&self, trigger: NodeId, collider: NodeId) -> bool {
        self.pairs.contains(&(trigger, collider))
    }

    /// Captures tracked pairs using persistent scene identities.
    /// # Errors
    /// Rejects foreign scenes and pairs whose identities cannot be saved.
    pub fn checkpoint(
        &self,
        scene: &SceneCheckpointView<'_>,
    ) -> Result<Vec<[ObjectId; 2]>, PhysicsError> {
        if scene.graph.identity() != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        let id = |node| {
            scene
                .reserved_identity(node)
                .cloned()
                .ok_or(SceneGraphError::InvalidNode)
        };
        let mut pairs = self
            .pairs
            .iter()
            .map(|&(a, b)| Ok([id(a)?, id(b)?]))
            .collect::<Result<Vec<_>, PhysicsError>>()?;
        pairs.sort_unstable();
        Ok(pairs)
    }

    /// Rebinds saved pairs, including retired identities awaiting an Exit.
    /// # Errors
    /// Rejects excess pairs, unknown identities, self pairs and duplicates.
    pub fn from_checkpoint(
        scene: &LoadedScene,
        saved: &[[ObjectId; 2]],
    ) -> Result<Self, PhysicsError> {
        let mut tracker = Self::new(&scene.graph, MAX_PAIRS)?;
        if saved.len() > tracker.max_pairs {
            return Err(PhysicsError::Capacity);
        }
        for [a, b] in saved {
            let a = scene.reserved_node(a).ok_or(SceneGraphError::InvalidNode)?;
            let b = scene.reserved_node(b).ok_or(SceneGraphError::InvalidNode)?;
            if a == b || !tracker.pairs.insert((a, b)) {
                return Err(SceneGraphError::InvalidNode.into());
            }
        }
        Ok(tracker)
    }

    /// Samples a game tick with the shared collision and query ceilings.
    /// # Errors
    /// Rejects invalid geometry or a tick exceeding either ceiling.
    pub fn sample_tick(
        &mut self,
        scene: &SceneGraph,
        physics: Option<&crate::CharacterPhysics>,
    ) -> Result<Vec<TriggerEvent>, PhysicsError> {
        self.sample_tick_with_traces(scene, physics, &[])
    }

    /// Samples the accepted controller tick and consumes its motion traces on success.
    /// # Errors
    /// Invalid geometry or exhausted budgets preserve both pairs and traces.
    pub fn sample_character_tick(
        &mut self,
        scene: &SceneGraph,
        physics: &mut CharacterPhysics,
    ) -> Result<Vec<TriggerEvent>, PhysicsError> {
        let events =
            self.sample_tick_with_traces(scene, Some(physics), &physics.translation_traces)?;
        physics.translation_traces.clear();
        Ok(events)
    }

    fn sample_tick_with_traces(
        &mut self,
        scene: &SceneGraph,
        physics: Option<&CharacterPhysics>,
        traces: &[CharacterTranslationTrace],
    ) -> Result<Vec<TriggerEvent>, PhysicsError> {
        if scene.identity() != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        if scene.active_components::<BoxTrigger>().next().is_none() && self.pairs.is_empty() {
            return Ok(Vec::new());
        }
        let queries = if let Some(physics) = physics {
            physics.queries(scene, MAX_COLLIDERS)?
        } else {
            SceneQueries::from_scene(scene, MAX_COLLIDERS)?
        };
        self.sample_with_traces(
            &queries,
            &mut SupportQueryBudget::new(crate::support::MAX_QUERIES)?,
            traces,
        )
    }

    /// Sample active authored triggers against the current query snapshot.
    /// Exits precede enters; each group is ordered by the two node identities.
    /// Persistent overlaps do not emit events. The trigger's own collider is
    /// excluded. This samples a pose, so callers choose the tick boundary.
    /// # Errors
    /// Rejects foreign snapshots, invalid active geometry and exhausted budgets.
    /// A failure preserves the previous pairs; work already spent stays charged.
    pub fn sample(
        &mut self,
        queries: &SceneQueries<'_>,
        budget: &mut SupportQueryBudget,
    ) -> Result<Vec<TriggerEvent>, PhysicsError> {
        self.sample_with_traces(queries, budget, &[])
    }

    fn sample_with_traces(
        &mut self,
        queries: &SceneQueries<'_>,
        budget: &mut SupportQueryBudget,
        traces: &[CharacterTranslationTrace],
    ) -> Result<Vec<TriggerEvent>, PhysicsError> {
        let scene = queries.source_scene();
        if scene.identity() != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        if scene.active_components::<BoxTrigger>().count() > MAX_TRIGGERS {
            return Err(PhysicsError::Capacity);
        }
        let has_swept_motion = traces.iter().any(|trace| !trace.segments.is_empty());
        let mut next = BTreeSet::new();
        let mut crossings = Vec::new();
        let mut traced_pairs = BTreeMap::new();
        for (trigger, descriptor) in scene.active_components::<BoxTrigger>() {
            budget.charge_work(1)?;
            let layers = queries.layer_filter(crate::CollisionLayers::on(scene, trigger));
            let accepts = |candidate| candidate != trigger && layers(candidate);
            let remaining = (self.max_pairs - next.len()).min(MAX_COLLIDERS);
            for collider in
                queries.overlap_box(trigger, descriptor.half_extents, budget, remaining, accepts)?
            {
                next.insert((trigger, collider));
            }
            if !has_swept_motion {
                continue;
            }
            let zone = queries.query_box(trigger, descriptor.half_extents)?;
            let mut append = |pair,
                              fixed: &AffineBox,
                              moving,
                              segments: &[glam::DVec3],
                              budget: &mut SupportQueryBudget| {
                let touching = append_crossings(
                    fixed,
                    moving,
                    pair,
                    segments,
                    self.pairs.contains(&pair),
                    budget,
                    &mut crossings,
                )?;
                traced_pairs.insert(pair, touching);
                Ok::<_, PhysicsError>(())
            };
            if let Some(owner) = character_ancestor(scene, trigger, budget)? {
                if let Ok(index) = traces.binary_search_by_key(&owner, |trace| trace.owner) {
                    let trace = &traces[index];
                    let moving = if trace.complete {
                        AffineBox {
                            center: zone.center + (trace.origin - trace.end),
                            ..zone
                        }
                    } else {
                        trigger_at_trace_origin(
                            scene,
                            trigger,
                            descriptor.half_extents,
                            trace,
                            budget,
                        )?
                    };
                    if !trace.segments.is_empty() {
                        for (collider, fixed) in queries.collider_boxes() {
                            budget.charge_work(1)?;
                            if accepts(collider)
                                && scene.component::<crate::BoxCollider>(collider)?.is_some()
                            {
                                append(
                                    (trigger, collider),
                                    &fixed,
                                    moving,
                                    &trace.segments,
                                    budget,
                                )?;
                            }
                        }
                    }
                    let timed = has_timed_path(trace);
                    for target in traces {
                        budget.charge_work(1)?;
                        if target.owner == owner || !accepts(target.owner) {
                            continue;
                        }
                        let target_box = AffineBox {
                            center: target.origin,
                            edges: target.edges,
                        };
                        let pair = (trigger, target.owner);
                        match (trace.segments.is_empty(), target.segments.is_empty()) {
                            (true, false) if trace.complete || has_timed_path(target) => {
                                append(pair, &moving, target_box, &target.segments, budget)?;
                            }
                            (false, true) if target.complete => {
                                append(pair, &target_box, moving, &trace.segments, budget)?;
                            }
                            (false, false) if timed && has_timed_path(target) => {
                                let relative = relative_segments(trace, target, budget)?;
                                append(pair, &target_box, moving, &relative, budget)?;
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                for trace in traces {
                    budget.charge_work(1)?;
                    // The segments before a rotation still describe exact
                    // translations against this stationary trigger. Reconcile
                    // the final pose separately after that known prefix.
                    if !trace.segments.is_empty() && accepts(trace.owner) {
                        let moving = AffineBox {
                            center: trace.origin,
                            edges: trace.edges,
                        };
                        append(
                            (trigger, trace.owner),
                            &zone,
                            moving,
                            &trace.segments,
                            budget,
                        )?;
                    }
                }
            }
        }
        if traces.iter().any(|trace| trace.rotation_arc.is_some()) {
            for (trigger, descriptor) in scene.active_components::<BoxTrigger>() {
                let layers = queries.layer_filter(crate::CollisionLayers::on(scene, trigger));
                let mut record =
                    |collider, target: BoxArc, moving: BoxArc, budget: &mut SupportQueryBudget| {
                        let pair = (trigger, collider);
                        if trigger != collider && layers(collider) {
                            let previous = traced_pairs
                                .get(&pair)
                                .copied()
                                .unwrap_or_else(|| self.pairs.contains(&pair));
                            let touching = append_arc_transits(
                                target,
                                moving,
                                pair,
                                previous,
                                budget,
                                &mut crossings,
                            )?;
                            traced_pairs.insert(pair, touching);
                        }
                        Ok::<_, PhysicsError>(())
                    };
                let owner = character_ancestor(scene, trigger, budget)?;
                let owner_trace = owner.and_then(|owner| {
                    traces
                        .binary_search_by_key(&owner, |trace| trace.owner)
                        .ok()
                        .map(|index| &traces[index])
                });
                if let Some((trace, arc)) =
                    owner_trace.and_then(|trace| trace.rotation_arc.map(|arc| (trace, arc)))
                {
                    let mut start = trigger_at_trace_origin(
                        scene,
                        trigger,
                        descriptor.half_extents,
                        trace,
                        budget,
                    )?;
                    start.center = arc.pivot + (start.center - trace.origin);
                    let moving = BoxArc::rotating(start, arc);
                    for (collider, target) in queries.collider_boxes() {
                        budget.charge_work(1)?;
                        if collider == trace.owner {
                            continue;
                        }
                        let other = traces
                            .binary_search_by_key(&collider, |trace| trace.owner)
                            .ok()
                            .map(|index| &traces[index]);
                        if let Some((other, arc)) =
                            other.and_then(|other| other.rotation_arc.map(|arc| (other, arc)))
                        {
                            let start = AffineBox {
                                center: arc.pivot,
                                edges: other.edges,
                            };
                            record(collider, BoxArc::rotating(start, arc), moving, budget)?;
                        } else if scene.component::<crate::BoxCollider>(collider)?.is_some()
                            || other.is_some_and(stationary_trace)
                        {
                            record(collider, BoxArc::stationary(target), moving, budget)?;
                        }
                    }
                } else if owner.is_none() || owner_trace.is_some_and(stationary_trace) {
                    let zone = queries.query_box(trigger, descriptor.half_extents)?;
                    for trace in traces {
                        budget.charge_work(1)?;
                        if let Some(arc) = trace.rotation_arc {
                            let start = AffineBox {
                                center: arc.pivot,
                                edges: trace.edges,
                            };
                            record(
                                trace.owner,
                                BoxArc::stationary(zone),
                                BoxArc::rotating(start, arc),
                                budget,
                            )?;
                        }
                    }
                }
            }
        }
        let event = |(trigger, collider), phase| TriggerEvent {
            trigger,
            collider,
            phase,
        };
        for (&pair, &touching) in &traced_pairs {
            let current = next.contains(&pair);
            if touching != current {
                crossings.push(event(
                    pair,
                    if current {
                        TriggerPhase::Enter
                    } else {
                        TriggerPhase::Exit
                    },
                ));
            }
        }
        let events = self
            .pairs
            .difference(&next)
            .filter(|pair| !traced_pairs.contains_key(pair))
            .map(|pair| event(*pair, TriggerPhase::Exit))
            .chain(
                next.difference(&self.pairs)
                    .filter(|pair| !traced_pairs.contains_key(pair))
                    .map(|pair| event(*pair, TriggerPhase::Enter)),
            )
            .chain(crossings)
            .collect();
        self.pairs = next;
        Ok(events)
    }
}

fn character_ancestor(
    scene: &SceneGraph,
    owner: NodeId,
    budget: &mut SupportQueryBudget,
) -> Result<Option<NodeId>, PhysicsError> {
    let mut current = Some(owner);
    while let Some(node) = current {
        budget.charge_work(1)?;
        if scene.component::<CharacterBody>(node)?.is_some() {
            return Ok(Some(node));
        }
        current = scene.parent(node)?;
    }
    Ok(None)
}

fn trigger_at_trace_origin(
    scene: &SceneGraph,
    trigger: NodeId,
    half_extents: [f32; 3],
    trace: &CharacterTranslationTrace,
    budget: &mut SupportQueryBudget,
) -> Result<AffineBox, PhysicsError> {
    use glam::{DMat3, DMat4};

    let body = scene
        .component::<CharacterBody>(trace.owner)?
        .ok_or(SceneGraphError::InvalidNode)?;
    let [x, y, z] =
        std::array::from_fn(|axis| trace.edges[axis] / f64::from(body.half_extents[axis]));
    let basis = DMat3::from_cols(x, y, z);
    let mut relative = DMat4::IDENTITY;
    let mut node = trigger;
    while node != trace.owner {
        budget.charge_work(1)?;
        relative = scene.local(node)?.matrix()?.as_dmat4() * relative;
        node = scene.parent(node)?.ok_or(SceneGraphError::InvalidNode)?;
    }
    let axes = [relative.x_axis, relative.y_axis, relative.z_axis];
    let shape = AffineBox {
        center: trace.origin + basis * relative.w_axis.truncate(),
        edges: std::array::from_fn(|axis| {
            basis * axes[axis].truncate() * f64::from(half_extents[axis])
        }),
    };
    if !shape.center.is_finite() || shape.edges.iter().any(|edge| !edge.is_finite()) {
        return Err(PhysicsError::UnsupportedTransform);
    }
    Ok(shape)
}

fn stationary_trace(trace: &CharacterTranslationTrace) -> bool {
    trace.complete && trace.segments.is_empty() && trace.origin == trace.end
}

fn has_timed_path(trace: &CharacterTranslationTrace) -> bool {
    // A later angular phase can invalidate completeness without changing
    // the common timing of the input-driven translation prefix.
    trace.end_fractions.len() == trace.segments.len()
        && trace.end_fractions.first().is_some_and(|first| *first > 0.)
        && trace.end_fractions.last().is_some_and(|last| *last <= 1.)
        && trace.end_fractions.windows(2).all(|pair| pair[0] < pair[1])
}

fn relative_segments(
    a: &CharacterTranslationTrace,
    b: &CharacterTranslationTrace,
    budget: &mut SupportQueryBudget,
) -> Result<Vec<glam::DVec3>, PhysicsError> {
    let mut a = a
        .end_fractions
        .iter()
        .copied()
        .zip(a.segments.iter().copied())
        .peekable();
    let mut b = b
        .end_fractions
        .iter()
        .copied()
        .zip(b.segments.iter().copied())
        .peekable();
    let (mut start, mut start_a, mut start_b) = (0., 0., 0.);
    let mut relative = Vec::new();
    while a.peek().is_some() || b.peek().is_some() {
        budget.charge_work(1)?;
        let (end_a, movement_a) = a.peek().copied().unwrap_or((1., glam::DVec3::ZERO));
        let (end_b, movement_b) = b.peek().copied().unwrap_or((1., glam::DVec3::ZERO));
        let end = end_a.min(end_b);
        if end <= start {
            return Err(PhysicsError::InvalidMotion);
        }
        let portion = |segment, from, to| {
            if segment == glam::DVec3::ZERO {
                glam::DVec3::ZERO
            } else {
                segment * ((end - start) / (to - from))
            }
        };
        relative.push(portion(movement_a, start_a, end_a) - portion(movement_b, start_b, end_b));
        if end == end_a {
            a.next();
            start_a = end;
        }
        if end == end_b {
            b.next();
            start_b = end;
        }
        start = end;
    }
    Ok(relative)
}

fn append_crossings(
    fixed: &AffineBox,
    mut moving: AffineBox,
    (trigger, collider): (NodeId, NodeId),
    segments: &[glam::DVec3],
    previous: bool,
    budget: &mut SupportQueryBudget,
    events: &mut Vec<TriggerEvent>,
) -> Result<bool, PhysicsError> {
    let mut touching = previous;
    let mut transition = |next| {
        if touching != next {
            events.push(TriggerEvent {
                trigger,
                collider,
                phase: if next {
                    TriggerPhase::Enter
                } else {
                    TriggerPhase::Exit
                },
            });
            touching = next;
        }
    };
    for &segment in segments {
        budget.charge_work(1)?;
        if let Some(contact) = fixed.translation_interval(&moving, segment)? {
            let (enter, exit) = (contact.enter, contact.exit);
            if enter > 0. {
                transition(false);
            }
            transition(true);
            if exit < 1. {
                transition(false);
            }
        } else {
            transition(false);
        }
        moving.center += segment;
    }
    Ok(touching)
}

#[derive(Clone, Copy)]
struct BoxArc {
    local: AffineBox,
    pivot: glam::DVec3,
    requested: glam::DVec3,
    stop: f64,
}

impl BoxArc {
    fn stationary(shape: AffineBox) -> Self {
        Self {
            pivot: shape.center,
            local: AffineBox {
                center: glam::DVec3::ZERO,
                ..shape
            },
            requested: glam::DVec3::ZERO,
            stop: 0.,
        }
    }

    fn rotating(shape: AffineBox, arc: crate::character_physics::AcceptedRotationArc) -> Self {
        Self {
            local: AffineBox {
                center: shape.center - arc.pivot,
                ..shape
            },
            pivot: arc.pivot,
            requested: arc.requested,
            stop: arc.stop,
        }
    }

    fn pose(self, time: f64) -> AffineBox {
        let rotation = glam::DQuat::from_scaled_axis(self.requested * time.min(self.stop));
        AffineBox {
            center: self.pivot + crate::convex::rotate_vector(rotation, self.local.center),
            edges: self
                .local
                .edges
                .map(|edge| crate::convex::rotate_vector(rotation, edge)),
        }
    }

    fn radius_bound(self) -> f64 {
        self.local.center.length()
            + self
                .local
                .edges
                .iter()
                .map(|edge| edge.length())
                .sum::<f64>()
    }
}

fn append_arc_transits(
    mut target: BoxArc,
    mut moving: BoxArc,
    (trigger, collider): (NodeId, NodeId),
    previous: bool,
    budget: &mut SupportQueryBudget,
    events: &mut Vec<TriggerEvent>,
) -> Result<bool, PhysicsError> {
    target.pivot -= moving.pivot;
    moving.pivot = glam::DVec3::ZERO;
    let moving_radius = moving.radius_bound();
    let target_radius = target.radius_bound();
    let pivot_distance = target.pivot.length();
    // Each pivot-centered sphere encloses every point for the entire arc.
    // Include roundoff slack so touching bounds still reach the SAT checks.
    let radius_sum = moving_radius + target_radius;
    let bound_slack = 128. * f64::EPSILON * (1. + pivot_distance + radius_sum);
    let mut touching = previous;
    let mut transition = |next| {
        if next != touching {
            events.push(TriggerEvent {
                trigger,
                collider,
                phase: if next {
                    TriggerPhase::Enter
                } else {
                    TriggerPhase::Exit
                },
            });
            touching = next;
        }
    };
    if pivot_distance > radius_sum + bound_slack {
        transition(false);
        return Ok(false);
    }

    let separation = |time| {
        let target = target.pose(time);
        let shape = moving.pose(time);
        let offset = shape.center - target.center;
        target
            .axes_for(shape.edges)
            .map(|axis| offset.dot(axis).abs() - target.radius(axis) - shape.radius(axis))
            .fold(f64::NEG_INFINITY, f64::max)
    };
    // Both requests share the angular phase. A blocked arc holds its
    // accepted pose from its stop fraction to the end of the phase.
    let moving_speed = moving.requested.length() * moving_radius;
    let target_speed = target.requested.length() * target_radius;
    let slack =
        128. * f64::EPSILON * (1. + pivot_distance + moving_speed + target_speed + radius_sum);
    budget.charge_work(2)?;
    let (start_gap, end_gap) = (separation(0.), separation(1.));
    if start_gap.abs() > slack {
        transition(start_gap < 0.);
    }
    let mut spans = vec![(0., 1.)];
    while let Some((left, right)) = spans.pop() {
        budget.charge_work(1)?;
        let middle = (left + right) * 0.5;
        let gap = separation(middle);
        let variation = |stop: f64| {
            let sample = middle.min(stop);
            (sample - left.min(stop)).max(right.min(stop) - sample)
        };
        let travel = moving_speed * variation(moving.stop) + target_speed * variation(target.stop);
        // A positive separating gap, or a negative gap equal to the
        // distance to the boundary of the Minkowski difference, certifies
        // the midpoint's state while both shapes move less than that gap.
        let resolved = gap.abs() > travel + slack;
        if resolved || travel <= slack || middle <= left || middle >= right {
            if gap.abs() > slack {
                transition(gap < 0.);
            }
            continue;
        }
        // Visit in angular time order and collapse adjacent equal states.
        spans.push((middle, right));
        spans.push((left, middle));
    }
    if end_gap.abs() > slack {
        transition(end_gap < 0.);
    }
    Ok(touching)
}

pub(crate) fn validate_triggers(scene: &SceneGraph, capacity: usize) -> Result<(), PhysicsError> {
    if scene.components::<BoxTrigger>().count() > capacity.min(MAX_TRIGGERS) {
        return Err(PhysicsError::Capacity);
    }
    for (owner, descriptor) in scene.components::<BoxTrigger>() {
        validate_extents(descriptor.half_extents)?;
        affine_box(scene, owner, descriptor.half_extents)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BoxCollider;
    use voxy_scene::Transform;

    #[test]
    fn layers_filter_contacts_queries_and_swept_trigger_events_after_live_edit() {
        use crate::{CollisionLayers, RayQuery};
        use glam::{DVec3, Vec3};

        let mut scene = SceneGraph::new(3);
        let character = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                character,
                CharacterBody {
                    half_extents: [0.1; 3],
                    speed: 120.,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                BoxCollider {
                    half_extents: [0.1; 3],
                },
            )
            .unwrap();
        let trigger = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 0.5,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        let isolated = CollisionLayers {
            membership: 2,
            mask: 2,
        };
        for node in [wall, trigger] {
            scene.insert_component(node, isolated).unwrap();
        }
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        let mut tracker = TriggerTracker::new(&scene, 2).unwrap();
        let mut input = crate::player_input().unwrap();
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        assert_eq!(scene.local(character).unwrap().translation.x, 2.);
        assert!(
            tracker
                .sample_character_tick(&scene, &mut physics)
                .unwrap()
                .is_empty()
        );

        scene.set_local(character, Transform::default()).unwrap();
        scene.insert_component(character, isolated).unwrap();
        let queries = physics.queries(&scene, 2).unwrap();
        let hit = queries
            .cast_ray(
                RayQuery {
                    origin: DVec3::ZERO,
                    direction: DVec3::X,
                    max_distance: 2.,
                    solid: true,
                },
                &mut SupportQueryBudget::new(8).unwrap(),
                |node| node != character && queries.layer_filter(isolated)(node),
            )
            .unwrap()
            .unwrap();
        assert_eq!(hit.owner, wall);
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        assert!(scene.local(character).unwrap().translation.x < 1.);
        assert_eq!(
            tracker.sample_character_tick(&scene, &mut physics).unwrap(),
            [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                trigger,
                collider: character,
                phase
            })
        );
    }

    #[test]
    fn relative_segments_merge_distinct_contact_times_and_rest_after_stop() {
        use glam::DVec3;
        let mut scene = SceneGraph::new(2);
        let mut make = |segments, end_fractions| CharacterTranslationTrace {
            owner: scene.spawn(None, Transform::default()).unwrap(),
            origin: DVec3::ZERO,
            edges: [DVec3::X, DVec3::Y, DVec3::Z],
            complete: true,
            end: DVec3::ZERO,
            rotation_arc: None,
            segments,
            end_fractions,
        };
        let a = make(vec![DVec3::X * 2., DVec3::Z * 2.], vec![0.5, 1.]);
        let b = make(vec![DVec3::X], vec![0.25]);
        assert_eq!(
            relative_segments(&a, &b, &mut SupportQueryBudget::new(3).unwrap()).unwrap(),
            [DVec3::ZERO, DVec3::X, DVec3::Z * 2.]
        );
    }

    #[test]
    fn two_characters_cross_during_slides_with_different_contact_times() {
        use glam::Vec3;
        for rotates in [false, true] {
            let mut scene = SceneGraph::new(3);
            let trigger = scene.spawn(None, Transform::default()).unwrap();
            let collider = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::new(0.79, 0., -0.8),
                        ..Default::default()
                    },
                )
                .unwrap();
            for (owner, speed) in [(trigger, 120.), (collider, 30.)] {
                scene
                    .insert_component(
                        owner,
                        CharacterBody {
                            half_extents: [0.1; 3],
                            speed,
                            gravity: 0.,
                            jump_speed: 0.,
                        },
                    )
                    .unwrap();
            }
            let wall = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::X,
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    wall,
                    BoxCollider {
                        half_extents: [0.1, 2., 3.],
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.07; 3],
                    },
                )
                .unwrap();
            let mut physics = CharacterPhysics::new(&scene, 2, 1);
            let mut tracker = TriggerTracker::new(&scene, 2).unwrap();
            let mut input = crate::player_input().unwrap();
            input.event(crate::RIGHT, 1.).unwrap();
            input.event(crate::FORWARD, 1.).unwrap();
            if rotates {
                let motions = [crate::CharacterMotion {
                    owner: collider,
                    displacement: Vec3::ZERO,
                    angular_displacement: Vec3::X * 0.25,
                }];
                let applied = physics
                    .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &motions)
                    .unwrap();
                assert_eq!(applied[0].angular_fraction, 1.);
            } else {
                physics
                    .fixed_step(&mut scene, &mut input, 1. / 60.)
                    .unwrap();
            }
            let [first, second] = physics.translation_traces.as_slice() else {
                panic!("two paths expected")
            };
            assert_eq!((first.segments.len(), second.segments.len()), (2, 2));
            assert_ne!(first.end_fractions[0], second.end_fractions[0]);
            assert_eq!(second.complete, !rotates);
            let events = tracker.sample_character_tick(&scene, &mut physics).unwrap();
            assert_eq!(
                events
                    .into_iter()
                    .filter(|event| event.collider == collider)
                    .map(|event| event.phase)
                    .collect::<Vec<_>>(),
                [TriggerPhase::Enter, TriggerPhase::Exit]
            );
            assert!(!tracker.contains(trigger, collider));
        }
    }

    #[test]
    fn crossing_detected_when_either_character_is_stationary() {
        use glam::Vec3;
        for trigger_moves in [false, true] {
            let mut scene = SceneGraph::new(2);
            let moving = scene.spawn(None, Transform::default()).unwrap();
            let stationary = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::X,
                        ..Default::default()
                    },
                )
                .unwrap();
            let (trigger, collider) = if trigger_moves {
                (moving, stationary)
            } else {
                (stationary, moving)
            };
            for (owner, speed) in [(moving, 120.), (stationary, 0.)] {
                scene
                    .insert_component(
                        owner,
                        CharacterBody {
                            half_extents: [0.1; 3],
                            speed,
                            gravity: 0.,
                            jump_speed: 0.,
                        },
                    )
                    .unwrap();
            }
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.1; 3],
                    },
                )
                .unwrap();
            let mut physics = CharacterPhysics::new(&scene, 2, 0);
            let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
            let mut input = crate::player_input().unwrap();
            input.event(crate::RIGHT, 1.).unwrap();
            physics
                .fixed_step(&mut scene, &mut input, 1. / 60.)
                .unwrap();
            assert_eq!(scene.local(moving).unwrap().translation.x, 2.);
            assert_eq!(scene.local(stationary).unwrap().translation.x, 1.);
            assert_eq!(
                tracker.sample_character_tick(&scene, &mut physics).unwrap(),
                [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                    trigger,
                    collider,
                    phase
                })
            );
            assert!(!tracker.contains(trigger, collider));
        }
    }

    #[test]
    fn moving_trigger_overtakes_a_character_during_the_same_tick() {
        use glam::Vec3;

        let mut scene = SceneGraph::new(2);
        let trigger = scene.spawn(None, Transform::default()).unwrap();
        let collider = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X,
                    ..Default::default()
                },
            )
            .unwrap();
        for (owner, speed) in [(trigger, 240.), (collider, 60.)] {
            scene
                .insert_component(
                    owner,
                    CharacterBody {
                        half_extents: [0.1; 3],
                        speed,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
        }
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.1; 3],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 2, 0);
        let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
        let mut input = crate::player_input().unwrap();
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        assert_eq!(scene.local(trigger).unwrap().translation.x, 4.);
        assert_eq!(scene.local(collider).unwrap().translation.x, 2.);
        assert_eq!(
            tracker.sample_character_tick(&scene, &mut physics).unwrap(),
            [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                trigger,
                collider,
                phase
            })
        );
        assert!(!tracker.contains(trigger, collider));
    }

    #[test]
    fn distant_angular_pairs_do_not_spend_sat_budget() {
        use glam::DVec3;
        let mut scene = SceneGraph::new(2);
        let trigger = scene.spawn(None, Transform::default()).unwrap();
        let collider = scene.spawn(None, Transform::default()).unwrap();
        let origin = DVec3::splat(100_000.);
        let edges = [DVec3::X, DVec3::Y, DVec3::Z];
        let rotating = BoxArc::rotating(
            AffineBox {
                center: origin + DVec3::X * 2.,
                edges,
            },
            crate::character_physics::AcceptedRotationArc {
                pivot: origin,
                requested: DVec3::Z * std::f64::consts::TAU,
                stop: 1.,
            },
        );
        let mut budget = SupportQueryBudget::new(0).unwrap();
        let mut events = Vec::new();
        for distance in 10..1034 {
            let target = BoxArc::stationary(AffineBox {
                center: origin + DVec3::X * f64::from(distance),
                edges,
            });
            assert!(
                !append_arc_transits(
                    target,
                    rotating,
                    (trigger, collider),
                    false,
                    &mut budget,
                    &mut events
                )
                .unwrap()
            );
        }
        assert!(events.is_empty());
        let touching_bounds = BoxArc::stationary(AffineBox {
            center: origin + DVec3::X * 8.,
            edges,
        });
        assert_eq!(
            append_arc_transits(
                touching_bounds,
                rotating,
                (trigger, collider),
                false,
                &mut budget,
                &mut events
            ),
            Err(PhysicsError::SweepBudget),
        );
    }

    #[test]
    fn angular_exit_and_reentry_are_reported_with_overlapping_endpoints() {
        use glam::Vec3;
        let mut scene = SceneGraph::new(2);
        let collider = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                collider,
                CharacterBody {
                    half_extents: [1., 0.05, 0.05],
                    speed: 0.,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let trigger = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 0.8,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 0);
        let mut input = crate::player_input().unwrap();
        let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
        assert_eq!(
            tracker.sample_tick(&scene, Some(&physics)).unwrap(),
            [TriggerEvent {
                trigger,
                collider,
                phase: TriggerPhase::Enter
            }]
        );
        physics
            .fixed_step_with_rigid_motion(
                &mut scene,
                &mut input,
                1. / 60.,
                &[crate::CharacterMotion {
                    owner: collider,
                    displacement: Vec3::ZERO,
                    angular_displacement: Vec3::Z * std::f32::consts::PI,
                }],
            )
            .unwrap();
        assert_eq!(
            tracker.sample_character_tick(&scene, &mut physics).unwrap(),
            [TriggerPhase::Exit, TriggerPhase::Enter].map(|phase| TriggerEvent {
                trigger,
                collider,
                phase
            }),
        );
        assert!(tracker.contains(trigger, collider));
    }

    #[test]
    fn full_rotation_reports_both_passes_in_order() {
        use glam::Vec3;
        let mut scene = SceneGraph::new(2);
        let collider = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                collider,
                CharacterBody {
                    half_extents: [1., 0.05, 0.05],
                    speed: 0.,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let trigger = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.6, 0.6, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 0);
        let mut input = crate::player_input().unwrap();
        let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
        physics
            .fixed_step_with_rigid_motion(
                &mut scene,
                &mut input,
                1. / 60.,
                &[crate::CharacterMotion {
                    owner: collider,
                    displacement: Vec3::ZERO,
                    angular_displacement: Vec3::Z * std::f32::consts::TAU,
                }],
            )
            .unwrap();
        let phases = [
            TriggerPhase::Enter,
            TriggerPhase::Exit,
            TriggerPhase::Enter,
            TriggerPhase::Exit,
        ];
        assert_eq!(
            tracker.sample_character_tick(&scene, &mut physics).unwrap(),
            phases.map(|phase| TriggerEvent {
                trigger,
                collider,
                phase
            })
        );
        assert!(!tracker.contains(trigger, collider));
    }

    #[test]
    fn two_rotating_characters_report_a_shared_phase_transit() {
        use glam::Vec3;
        let mut scene = SceneGraph::new(3);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        let collider = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(diagonal + 0.14, diagonal - 0.14, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        for (id, half_extents) in [(owner, [0.03; 3]), (collider, [0.25, 0.03, 0.03])] {
            scene
                .insert_component(
                    id,
                    CharacterBody {
                        half_extents,
                        speed: 0.,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
        }
        let trigger = scene
            .spawn(
                Some(owner),
                Transform {
                    translation: Vec3::X,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.04; 3],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 2, 0);
        let mut input = crate::player_input().unwrap();
        let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
        assert!(
            tracker
                .sample_tick(&scene, Some(&physics))
                .unwrap()
                .is_empty()
        );
        let motions = [(owner, 1.), (collider, -1.)].map(|(owner, sign)| crate::CharacterMotion {
            owner,
            displacement: Vec3::ZERO,
            angular_displacement: Vec3::Z * sign * std::f32::consts::FRAC_PI_2,
        });
        let applied = physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &motions)
            .unwrap();
        assert!(applied.iter().all(|motion| motion.angular_fraction == 1.));
        assert_eq!(
            tracker.sample_character_tick(&scene, &mut physics).unwrap(),
            [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                trigger,
                collider,
                phase
            }),
        );
        assert!(!tracker.contains(trigger, collider));
    }

    #[test]
    fn angular_pose_holds_when_the_controller_stops_early() {
        use glam::DVec3;
        let arc = BoxArc::rotating(
            AffineBox {
                center: DVec3::X,
                edges: [DVec3::X, DVec3::Y, DVec3::Z],
            },
            crate::character_physics::AcceptedRotationArc {
                pivot: DVec3::ZERO,
                requested: DVec3::Z * std::f64::consts::TAU,
                stop: 0.25,
            },
        );
        assert!(arc.pose(0.25).center.abs_diff_eq(DVec3::Y, 1e-12));
        assert_eq!(arc.pose(0.25).center, arc.pose(0.75).center);
        assert_ne!(arc.pose(0.125).center, arc.pose(0.25).center);
    }

    #[test]
    fn offset_trigger_orbits_its_owner_and_crosses_stationary_colliders() {
        use glam::Vec3;
        for attached in [false, true] {
            let mut scene = SceneGraph::new(3);
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    CharacterBody {
                        half_extents: [0.05; 3],
                        speed: 0.,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
            let trigger = scene
                .spawn(
                    Some(owner),
                    Transform {
                        translation: Vec3::X,
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.05; 3],
                    },
                )
                .unwrap();
            let diagonal = std::f32::consts::FRAC_1_SQRT_2;
            let collider = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::new(diagonal, diagonal, 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            if attached {
                scene
                    .insert_component(
                        collider,
                        CharacterBody {
                            half_extents: [0.05; 3],
                            speed: 0.,
                            gravity: 0.,
                            jump_speed: 0.,
                        },
                    )
                    .unwrap();
            } else {
                scene
                    .insert_component(
                        collider,
                        BoxCollider {
                            half_extents: [0.05; 3],
                        },
                    )
                    .unwrap();
            }
            let mut physics = CharacterPhysics::new(&scene, 2, 1);
            let mut input = crate::player_input().unwrap();
            let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
            assert!(
                tracker
                    .sample_tick(&scene, Some(&physics))
                    .unwrap()
                    .is_empty()
            );
            let applied = physics
                .fixed_step_with_rigid_motion(
                    &mut scene,
                    &mut input,
                    1. / 60.,
                    &[crate::CharacterMotion {
                        owner,
                        displacement: Vec3::ZERO,
                        angular_displacement: Vec3::Z * std::f32::consts::FRAC_PI_2,
                    }],
                )
                .unwrap();
            assert_eq!(applied[0].angular_fraction, 1.);
            assert_eq!(
                tracker.sample_character_tick(&scene, &mut physics).unwrap(),
                [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                    trigger,
                    collider,
                    phase
                }),
            );
            assert!(!tracker.contains(trigger, collider));
        }
    }

    #[test]
    fn stationary_trigger_reports_transit_inside_an_accepted_rotation() {
        use glam::Vec3;
        for attached in [false, true] {
            let mut scene = SceneGraph::new(3);
            let collider = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    collider,
                    CharacterBody {
                        half_extents: [1., 0.05, 0.05],
                        speed: 0.,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
            let parent = attached.then(|| {
                let owner = scene
                    .spawn(
                        None,
                        Transform {
                            translation: Vec3::Z,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                scene
                    .insert_component(
                        owner,
                        CharacterBody {
                            half_extents: [0.05; 3],
                            speed: 0.,
                            gravity: 0.,
                            jump_speed: 0.,
                        },
                    )
                    .unwrap();
                owner
            });
            let trigger = scene
                .spawn(
                    parent,
                    Transform {
                        translation: Vec3::new(0.6, 0.6, if attached { -1. } else { 0. }),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.05; 3],
                    },
                )
                .unwrap();
            let mut physics = CharacterPhysics::new(&scene, 2, 0);
            let mut input = crate::player_input().unwrap();
            let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
            assert!(
                tracker
                    .sample_tick(&scene, Some(&physics))
                    .unwrap()
                    .is_empty()
            );
            let applied = physics
                .fixed_step_with_rigid_motion(
                    &mut scene,
                    &mut input,
                    1. / 60.,
                    &[crate::CharacterMotion {
                        owner: collider,
                        displacement: Vec3::ZERO,
                        angular_displacement: Vec3::Z * std::f32::consts::FRAC_PI_2,
                    }],
                )
                .unwrap();
            assert_eq!(applied[0].angular_fraction, 1.);
            assert_eq!(
                tracker.sample_character_tick(&scene, &mut physics).unwrap(),
                [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                    trigger,
                    collider,
                    phase
                }),
            );
            assert!(!tracker.contains(trigger, collider));
            assert!(
                tracker
                    .sample_character_tick(&scene, &mut physics)
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn stationary_trigger_detects_translation_before_a_rotation_in_the_same_tick() {
        use glam::Vec3;
        for attached in [false, true] {
            let mut scene = SceneGraph::new(3);
            let collider = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    collider,
                    CharacterBody {
                        half_extents: [0.1, 0.2, 0.3],
                        speed: 0.,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
            let parent = attached.then(|| {
                let owner = scene
                    .spawn(
                        None,
                        Transform {
                            translation: Vec3::Z,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                let body = *scene.component::<CharacterBody>(collider).unwrap().unwrap();
                scene.insert_component(owner, body).unwrap();
                owner
            });
            let trigger = scene
                .spawn(
                    parent,
                    Transform {
                        translation: if attached { Vec3::X - Vec3::Z } else { Vec3::X },
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.1; 3],
                    },
                )
                .unwrap();

            let mut physics = CharacterPhysics::new(&scene, 2, 0);
            let mut input = crate::player_input().unwrap();
            physics
                .fixed_step_with_rigid_motion(
                    &mut scene,
                    &mut input,
                    1. / 60.,
                    &[crate::CharacterMotion {
                        owner: collider,
                        displacement: Vec3::X * 2.,
                        angular_displacement: Vec3::Y * 0.5,
                    }],
                )
                .unwrap();
            let trace = physics
                .translation_traces
                .iter()
                .find(|trace| trace.owner == collider)
                .unwrap();
            assert!(!trace.complete);
            assert!(!trace.segments.is_empty());
            let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
            assert_eq!(
                tracker.sample_character_tick(&scene, &mut physics).unwrap(),
                [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                    trigger,
                    collider,
                    phase
                }),
            );
            assert!(!tracker.contains(trigger, collider));
        }
    }

    #[test]
    fn attached_trigger_crosses_an_offset_static_collider() {
        use glam::Vec3;

        for rotates in [false, true] {
            let mut scene = SceneGraph::new(4);
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    CharacterBody {
                        half_extents: [0.1; 3],
                        speed: 120.,
                        gravity: 0.,
                        jump_speed: 0.,
                    },
                )
                .unwrap();
            let parent = scene
                .spawn(
                    Some(owner),
                    Transform {
                        scale: Vec3::splat(0.5),
                        ..Default::default()
                    },
                )
                .unwrap();
            let trigger = scene
                .spawn(
                    Some(parent),
                    Transform {
                        translation: Vec3::Z * 2.,
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    trigger,
                    BoxTrigger {
                        half_extents: [0.1; 3],
                    },
                )
                .unwrap();
            let collider = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::X + Vec3::Z,
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    collider,
                    BoxCollider {
                        half_extents: [0.1; 3],
                    },
                )
                .unwrap();

            let mut physics = CharacterPhysics::new(&scene, 1, 1);
            let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
            let mut input = crate::player_input().unwrap();
            input.event(crate::RIGHT, 1.).unwrap();
            if rotates {
                let applied = physics
                    .fixed_step_with_rigid_motion(
                        &mut scene,
                        &mut input,
                        1. / 60.,
                        &[crate::CharacterMotion {
                            owner,
                            displacement: Vec3::ZERO,
                            angular_displacement: Vec3::Y * std::f32::consts::FRAC_PI_2,
                        }],
                    )
                    .unwrap();
                assert_eq!(applied[0].angular_fraction, 1.);
            } else {
                physics
                    .fixed_step(&mut scene, &mut input, 1. / 60.)
                    .unwrap();
            }
            assert_eq!(scene.local(owner).unwrap().translation.x, 2.);
            assert_eq!(
                tracker.sample_character_tick(&scene, &mut physics).unwrap(),
                [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
                    trigger,
                    collider,
                    phase
                })
            );
            assert!(!tracker.contains(trigger, collider));
            assert!(
                tracker
                    .sample_character_tick(&scene, &mut physics)
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn traced_pair_delivers_exit_and_reentry_even_when_both_endpoints_overlap() {
        use glam::DVec3;

        let mut scene = SceneGraph::new(2);
        let trigger = scene.spawn(None, Transform::default()).unwrap();
        let collider = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [0.5; 3],
                },
            )
            .unwrap();
        scene
            .insert_component(
                collider,
                BoxCollider {
                    half_extents: [0.25; 3],
                },
            )
            .unwrap();
        let queries = SceneQueries::from_scene(&scene, 2).unwrap();
        let mut tracker = TriggerTracker::new(&scene, 1).unwrap();
        let mut budget = SupportQueryBudget::new(32).unwrap();
        let event = |phase| TriggerEvent {
            trigger,
            collider,
            phase,
        };
        assert_eq!(
            tracker.sample(&queries, &mut budget).unwrap(),
            [event(TriggerPhase::Enter)]
        );
        let trace = CharacterTranslationTrace {
            owner: collider,
            origin: DVec3::ZERO,
            edges: [DVec3::X, DVec3::Y, DVec3::Z].map(|axis| axis * 0.25),
            segments: vec![DVec3::X, DVec3::NEG_X],
            complete: true,
            end_fractions: Vec::new(),
            end: DVec3::ZERO,
            rotation_arc: None,
        };
        assert_eq!(
            tracker
                .sample_with_traces(&queries, &mut budget, &[trace])
                .unwrap(),
            [event(TriggerPhase::Exit), event(TriggerPhase::Enter)]
        );
        assert!(tracker.contains(trigger, collider));
        assert!(tracker.sample(&queries, &mut budget).unwrap().is_empty());
    }

    #[test]
    fn failed_sample_preserves_pairs_until_a_complete_sample() {
        let mut scene = SceneGraph::new(3);
        let trigger = scene.spawn(None, Transform::default()).unwrap();
        let collider = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                trigger,
                BoxTrigger {
                    half_extents: [1.; 3],
                },
            )
            .unwrap();
        for owner in [trigger, collider] {
            scene
                .insert_component(
                    owner,
                    BoxCollider {
                        half_extents: [0.5; 3],
                    },
                )
                .unwrap();
        }
        let mut tracker = TriggerTracker::new(&scene, MAX_PAIRS).unwrap();
        let sample = |tracker: &mut TriggerTracker, scene: &SceneGraph, budget| {
            tracker.sample(
                &SceneQueries::from_scene(scene, 3).unwrap(),
                &mut SupportQueryBudget::new(budget).unwrap(),
            )
        };
        let event = |phase| TriggerEvent {
            trigger,
            collider,
            phase,
        };
        assert_eq!(
            sample(&mut tracker, &scene, 3).unwrap(),
            [event(TriggerPhase::Enter)]
        );
        let foreign = SceneGraph::new(0);
        assert!(sample(&mut tracker, &foreign, 3).is_err());
        scene.remove_subtree(collider).unwrap();
        assert!(sample(&mut tracker, &scene, 1).is_err());
        assert!(tracker.contains(trigger, collider));
        assert_eq!(
            sample(&mut tracker, &scene, 3).unwrap(),
            [event(TriggerPhase::Exit)]
        );
        assert!(!tracker.contains(trigger, collider));
        assert!(sample(&mut tracker, &scene, 3).unwrap().is_empty());
    }
}
