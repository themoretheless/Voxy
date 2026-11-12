//! Static box collision world extraction, descriptor validation, and body relocation helpers.

use glam::{Quat, Vec3};
use physics::{AnchoredAabb, CharacterState, CollisionWorld, Origin, SweepResult};
use voxy_scene::{NodeId, SceneGraph, SceneId};

use crate::{BoxCollider, CharacterBody, CollisionLayers, PhysicsError, convex};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeBody {
    pub(crate) state: CharacterState,
    pub(crate) navigation: Option<crate::navigation::RouteCursor>,
    pub(crate) published: Vec3,
    pub(crate) descriptor: CharacterBody,
    pub(crate) edges: [glam::DVec3; 3],
    pub(crate) rest_edges: [glam::DVec3; 3],
    pub(crate) orientation: glam::DQuat,
    pub(crate) published_rotation: Quat,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StaticBox {
    pub(crate) half_extents: [f32; 3],
    pub(crate) shape: convex::AffineBox,
    pub(crate) owner: NodeId,
    pub(crate) layers: CollisionLayers,
    pub(crate) min: [f64; 3],
    pub(crate) max: [f64; 3],
}

#[derive(Clone, Debug)]
pub(crate) struct StaticWorld(
    pub(crate) Vec<StaticBox>,
    pub(crate) SceneId,
    pub(crate) Vec<convex::AffineBox>,
    pub(crate) bool,
);

impl StaticWorld {
    pub(crate) fn filtered(&self, layers: CollisionLayers) -> std::borrow::Cow<'_, Self> {
        if self
            .0
            .iter()
            .all(|obstacle| layers.interacts(obstacle.layers))
        {
            return std::borrow::Cow::Borrowed(self);
        }
        let boxes: Vec<_> = self
            .0
            .iter()
            .filter(|obstacle| layers.interacts(obstacle.layers))
            .copied()
            .collect();
        let shapes: Vec<_> = boxes.iter().map(|obstacle| obstacle.shape).collect();
        let non_axis_aligned = shapes.iter().any(|shape| !aligned_edges(shape.edges));
        std::borrow::Cow::Owned(Self(boxes, self.1, shapes, non_axis_aligned))
    }
}

impl CollisionWorld for StaticWorld {
    type Obstacle = NodeId;
    type Error = PhysicsError;
    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<SweepResult<NodeId>, PhysicsError> {
        if self.0.len() > max_candidates {
            return Err(PhysicsError::SweepBudget);
        }
        let anchor = [
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ];
        let mut result = SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        };
        for obstacle in &self.0 {
            let min: [f64; 3] = std::array::from_fn(|i| obstacle.min[i] - anchor[i]);
            let max: [f64; 3] = std::array::from_fn(|i| obstacle.max[i] - anchor[i]);
            let mut query = body;
            // The primitive reports inclusive time-zero contact. A controller
            // must allow separating/tangent motion at a touching face. Correct
            // only f64 rebase roundoff, not authored f32 penetrations.
            for axis in 0..3 {
                let epsilon = contact_epsilon(obstacle.min[axis], obstacle.max[axis]);
                let gap = query.min[axis] - max[axis];
                if gap.abs() <= epsilon {
                    query.min[axis] -= gap;
                    query.max[axis] -= gap;
                }
                let gap = query.max[axis] - min[axis];
                if gap.abs() <= epsilon {
                    query.min[axis] -= gap;
                    query.max[axis] -= gap;
                }
            }
            if (0..3).any(|axis| {
                (query.min[axis] >= max[axis] && displacement[axis] >= 0.0)
                    || (query.max[axis] <= min[axis] && displacement[axis] <= 0.0)
            }) {
                continue;
            }
            if let Some((fraction, normal)) = physics::sweep_box(query, displacement, min, max)
                && (fraction < result.fraction || result.obstacle.is_none())
            {
                result = SweepResult {
                    fraction,
                    normal,
                    obstacle: Some(obstacle.owner),
                };
            }
        }
        Ok(result)
    }
}

pub(crate) fn step_affine(
    runtime: &mut RuntimeBody,
    descriptor: CharacterBody,
    world: &StaticWorld,
    planar: [f64; 2],
    jump_pressed: bool,
    dt: f64,
    record: impl FnMut(glam::DVec3, Option<f64>),
) {
    let mut position = precise_center(runtime.state.body);
    let mut velocity = glam::DVec3::from_array(runtime.state.velocity);
    velocity.x = planar[0];
    velocity.z = planar[1];
    if jump_pressed && runtime.state.grounded {
        velocity.y = descriptor.jump_speed;
    }
    velocity.y = (velocity.y + descriptor.gravity * dt).max(-100.);
    let grounded = convex::move_body_affine_traced(
        &mut position,
        runtime.edges,
        &mut velocity,
        dt,
        &world.2,
        None,
        record,
    );
    relocate(runtime, position, descriptor);
    runtime.state.velocity = velocity.to_array();
    runtime.state.grounded = grounded;
}

pub(crate) fn precise_center(body: AnchoredAabb) -> glam::DVec3 {
    glam::DVec3::new(
        body.anchor.x as f64,
        body.anchor.y as f64,
        body.anchor.z as f64,
    ) + (glam::DVec3::from_array(body.min) + glam::DVec3::from_array(body.max)) * 0.5
}

pub(crate) fn relocate(
    runtime: &mut RuntimeBody,
    position: glam::DVec3,
    descriptor: CharacterBody,
) {
    let fresh_state = fresh(
        position.as_vec3(),
        descriptor,
        runtime.edges,
        runtime.published_rotation,
    );
    let anchor = fresh_state.state.body.anchor;
    let relative = position - glam::DVec3::new(anchor.x as f64, anchor.y as f64, anchor.z as f64);
    let half = runtime
        .edges
        .iter()
        .map(|edge| edge.abs())
        .sum::<glam::DVec3>();
    runtime.state.body = fresh_state.state.body;
    runtime.state.body.min = (relative - half).to_array();
    runtime.state.body.max = (relative + half).to_array();
}

pub(crate) fn static_world(scene: &SceneGraph) -> Result<StaticWorld, PhysicsError> {
    let mut boxes = Vec::new();
    let mut shapes = Vec::new();
    let mut any_non_axis_aligned = false;
    for (owner, collider) in scene.active_components::<BoxCollider>() {
        let shape = affine_box(scene, owner, collider.half_extents)?;
        let extent = shape
            .edges
            .iter()
            .map(|edge| edge.abs())
            .sum::<glam::DVec3>();
        let min = (shape.center - extent).to_array();
        let max = (shape.center + extent).to_array();
        if !aligned_edges(shape.edges) {
            any_non_axis_aligned = true;
        }
        shapes.push(shape);
        boxes.push(StaticBox {
            half_extents: collider.half_extents,
            shape,
            owner,
            layers: CollisionLayers::on(scene, owner),
            min,
            max,
        });
    }
    Ok(StaticWorld(
        boxes,
        scene.identity(),
        shapes,
        any_non_axis_aligned,
    ))
}

pub(crate) fn aligned_edges(edges: [glam::DVec3; 3]) -> bool {
    edges.iter().all(|edge| {
        edge.to_array()
            .into_iter()
            .filter(|value| *value != 0.)
            .count()
            == 1
    })
}

pub(crate) fn overlaps(body: &RuntimeBody, world: &StaticWorld) -> bool {
    let center = precise_center(body.state.body);
    world.0.iter().any(|obstacle| {
        obstacle
            .shape
            .penetration_affine(center, body.edges)
            .is_some()
    })
}

pub(crate) fn contact_epsilon(min: f64, max: f64) -> f64 {
    32.0 * f64::EPSILON * (1.0 + min.abs().max(max.abs()))
}

pub(crate) fn validate_static_collider(
    scene: &SceneGraph,
    owner: NodeId,
    collider: BoxCollider,
) -> Result<(), PhysicsError> {
    validate_extents(collider.half_extents)?;
    affine_box(scene, owner, collider.half_extents)?;
    if scene.component::<CharacterBody>(owner)?.is_some() {
        return Err(PhysicsError::InvalidBody);
    }
    let mut parent = scene.parent(owner)?;
    while let Some(id) = parent {
        if scene.component::<CharacterBody>(id)?.is_some() {
            return Err(PhysicsError::UnsupportedDynamicParent);
        }
        parent = scene.parent(id)?;
    }
    Ok(())
}

pub(crate) fn validate_extents(extents: [f32; 3]) -> Result<(), PhysicsError> {
    if extents
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0 || *value > 1e4)
    {
        return Err(PhysicsError::InvalidBody);
    }
    Ok(())
}

pub(crate) fn validate_body(body: CharacterBody) -> Result<(), PhysicsError> {
    validate_extents(body.half_extents)?;
    if !body.speed.is_finite()
        || !(0.0..=1000.0).contains(&body.speed)
        || !body.gravity.is_finite()
        || !(-1000.0..=0.0).contains(&body.gravity)
        || !body.jump_speed.is_finite()
        || !(0.0..=1000.0).contains(&body.jump_speed)
    {
        return Err(PhysicsError::InvalidBody);
    }
    Ok(())
}

pub(crate) fn affine_box(
    scene: &SceneGraph,
    owner: NodeId,
    extents: [f32; 3],
) -> Result<convex::AffineBox, PhysicsError> {
    let matrix = scene.world_matrix(owner)?;
    let edges = [
        matrix.x_axis.truncate().as_dvec3() * f64::from(extents[0]),
        matrix.y_axis.truncate().as_dvec3() * f64::from(extents[1]),
        matrix.z_axis.truncate().as_dvec3() * f64::from(extents[2]),
    ];
    let center = matrix.w_axis.truncate().as_dvec3();
    let extent = edges.iter().map(|edge| edge.abs()).sum::<glam::DVec3>();
    if !matrix.is_finite()
        || !matrix.inverse().is_finite()
        || edges.iter().any(|edge| edge.length_squared() < 1e-20)
    {
        return Err(PhysicsError::UnsupportedTransform);
    }
    if (center.abs() + extent).max_element() > 1e6 {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(convex::AffineBox { center, edges })
}

pub(crate) fn translation(scene: &SceneGraph, owner: NodeId) -> Result<Vec3, PhysicsError> {
    let mut cursor = Some(owner);
    while let Some(id) = cursor {
        let local = scene.local(id)?;
        if local.scale != Vec3::ONE
            || (id != owner
                && local.rotation != Quat::IDENTITY
                && local.rotation != -Quat::IDENTITY)
        {
            return Err(PhysicsError::UnsupportedTransform);
        }
        cursor = scene.parent(id)?;
    }
    let position = scene.world_matrix(owner)?.w_axis.truncate();
    if position.abs().max_element() > 1e6 {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(position)
}

pub(crate) fn fresh(
    center: Vec3,
    descriptor: CharacterBody,
    edges: [glam::DVec3; 3],
    rotation: Quat,
) -> RuntimeBody {
    let orientation = glam::DQuat::from_array(rotation.to_array().map(f64::from)).normalize();
    let rest_edges = edges.map(|edge| convex::rotate_vector(orientation.conjugate(), edge));
    let half = edges.iter().map(|edge| edge.abs()).sum::<glam::DVec3>();
    let min = (center.as_dvec3() - half).to_array();
    let max = (center.as_dvec3() + half).to_array();
    RuntimeBody {
        navigation: None,
        state: CharacterState {
            body: AnchoredAabb {
                anchor: Origin::default(),
                min,
                max,
            },
            velocity: [0.0; 3],
            grounded: false,
        },
        published: center,
        descriptor,
        edges,
        rest_edges,
        orientation,
        published_rotation: rotation,
    }
}

pub(crate) fn center_of(body: AnchoredAabb) -> Result<Vec3, PhysicsError> {
    let anchor = [
        body.anchor.x as f64,
        body.anchor.y as f64,
        body.anchor.z as f64,
    ];
    let position: [f64; 3] = std::array::from_fn(|i| anchor[i] + (body.min[i] + body.max[i]) * 0.5);
    if position
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(PhysicsError::CoordinateRange);
    }
    Ok(Vec3::from_array(position.map(|value| value as f32)))
}
