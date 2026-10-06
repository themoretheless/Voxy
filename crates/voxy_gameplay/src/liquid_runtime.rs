//! Scene-owned liquid world. Authored names bind once; particles and clocks live here.
use crate::LiquidSource;
use physics::liquid::{BodyGeometryHit, ContactWitness, RigidGeometryHit};
mod rigid_support;
use physics::liquid::{
    Config, Container, Liquid, Material, ParticleExchange, PulsedEmitter, StepStats,
};
use rigid_support::FeatureKey;
use std::collections::BTreeMap;
use voxy_scene::{NodeId, SceneGraph, SceneId};

#[derive(Clone, Debug, PartialEq)]
struct Source {
    descriptor: LiquidSource,
    emitter: PulsedEmitter,
    position: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
struct BodyOwner {
    node: NodeId,
    descriptor: crate::LiquidBody,
    state: physics::contact::ContactBody,
    principal_templates: Option<Vec<crate::convex::AffineBox>>,
    published: voxy_scene::Transform,
    colliders: Vec<ColliderOwner>,
    mass_descriptor: Option<crate::LiquidMassDistribution>,
    mass_properties: Option<physics::mass_properties::MassProperties>,
    rigid_frame: Option<crate::RigidBodyFrame>,
}

#[derive(Clone, Debug, PartialEq)]
struct ColliderOwner {
    node: NodeId,
    collider: crate::BoxCollider,
    path: Vec<(NodeId, voxy_scene::Transform)>,
}
fn body_colliders(scene: &SceneGraph, root: NodeId) -> Result<Vec<ColliderOwner>, String> {
    let mut result = Vec::new();
    for (node, collider) in scene.components::<crate::BoxCollider>() {
        let mut current = Some(node);
        let mut path = Vec::new();
        while let Some(id) = current {
            if id == root {
                for (child, _) in &path {
                    if scene
                        .component::<crate::LiquidBody>(*child)
                        .map_err(|e| format!("collider owner: {e:?}"))?
                        .is_some()
                        || scene
                            .component::<crate::CharacterBody>(*child)
                            .map_err(|e| format!("collider owner: {e:?}"))?
                            .is_some()
                        || scene
                            .component::<crate::AngularMotion>(*child)
                            .map_err(|e| format!("collider owner: {e:?}"))?
                            .is_some()
                    {
                        return Err("compound collider has another transform owner".into());
                    }
                }
                crate::validate_extents(collider.half_extents)
                    .map_err(|e| format!("compound extents: {e:?}"))?;
                crate::affine_box(scene, node, collider.half_extents)
                    .map_err(|e| format!("compound geometry: {e:?}"))?;
                result.push(ColliderOwner {
                    node,
                    collider: *collider,
                    path,
                });
                break;
            }
            path.push((
                id,
                scene
                    .local(id)
                    .map_err(|e| format!("collider pose: {e:?}"))?,
            ));
            current = scene
                .parent(id)
                .map_err(|e| format!("collider parent: {e:?}"))?;
        }
    }
    if result.is_empty() || result.len() > 128 {
        return Err("liquid body requires 1..128 owned BoxColliders".into());
    }
    Ok(result)
}

fn collider_template(
    body: &BodyOwner,
    collider: &ColliderOwner,
) -> Result<crate::convex::AffineBox, String> {
    fn matrix(pose: voxy_scene::Transform, translation: glam::DVec3) -> glam::DMat4 {
        glam::DMat4::from_scale_rotation_translation(
            pose.scale.as_dvec3(),
            pose.rotation.as_dquat(),
            translation,
        )
    }
    // Compose in root-relative f64 coordinates: published root translation must
    // never round a child's physical offset through a world-space f32 matrix.
    let mut transform = matrix(body.published, glam::DVec3::ZERO);
    for (_, pose) in collider.path.iter().rev() {
        transform *= matrix(*pose, pose.translation.as_dvec3());
    }
    let half = collider.collider.half_extents.map(f64::from);
    let shape = crate::convex::AffineBox {
        center: transform.transform_point3(glam::DVec3::ZERO),
        edges: [
            transform.transform_vector3(glam::DVec3::X * half[0]),
            transform.transform_vector3(glam::DVec3::Y * half[1]),
            transform.transform_vector3(glam::DVec3::Z * half[2]),
        ],
    };
    if !transform.inverse().is_finite()
        || !shape.center.is_finite()
        || shape
            .edges
            .iter()
            .any(|edge| !edge.is_finite() || edge.length_squared() < 1e-20)
    {
        return Err("nonfinite compound template".into());
    }
    Ok(shape)
}

/// One fluid world shared by all admitted scene sources.
/// Descriptor edits require constructing a new runtime; activation pauses source clocks.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidRuntime {
    scene: SceneId,
    sources: BTreeMap<NodeId, Source>,
    liquid: Liquid,
    body: Vec<BodyOwner>,
}

/// Snapshot-owned coupled reaction report; indices refer to this stable owner list.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSupportReactions {
    pub owners: Vec<NodeId>,
    pub reactions: physics::liquid::RigidWorldReactions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneLiquidStep {
    pub emissions: Vec<(NodeId, ParticleExchange)>,
    pub physics: StepStats,
    pub dynamics: Option<physics::liquid::DynamicEnvironmentReport>,
}

/// The same scene step plus admitted finite normal-reaction diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSupportedLiquidStep {
    pub step: SceneLiquidStep,
    pub support: Option<physics::liquid::SupportedWorldReport>,
}

fn position(scene: &SceneGraph, node: NodeId) -> Result<[f64; 3], String> {
    let p = scene
        .world_matrix(node)
        .map_err(|e| format!("liquid source transform: {e:?}"))?
        .transform_point3(glam::Vec3::ZERO);
    if !p.is_finite() {
        return Err("nonfinite liquid source position".into());
    }
    Ok([f64::from(p.x), f64::from(p.y), f64::from(p.z)])
}

struct SceneGeometry(crate::StaticWorld);
impl SceneGeometry {
    fn sweep_contact(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, String> {
        use physics::liquid::GeometryHit;
        if self.0.0.len() > max_candidates {
            return Err("liquid collider budget exceeded".into());
        }
        let center = glam::DVec3::from_array(center);
        let edges = [
            glam::DVec3::X * radius,
            glam::DVec3::Y * radius,
            glam::DVec3::Z * radius,
        ];
        let displacement = glam::DVec3::from_array(displacement);
        let mut result = BodyGeometryHit::from(GeometryHit::Clear);
        let mut earliest = f64::INFINITY;
        for obstacle in &self.0.0 {
            if obstacle.shape.penetration_affine(center, edges).is_some() {
                return Ok(GeometryHit::Overlap.into());
            }
            if let Some(contact) = obstacle
                .shape
                .sweep_affine_contact(center, edges, displacement)
                .map_err(|e| format!("liquid contact witness: {e:?}"))?
            {
                let fraction = contact.fraction;
                let normal = contact.normal;
                if fraction < earliest {
                    earliest = fraction;
                    result = BodyGeometryHit {
                        geometry: GeometryHit::Contact {
                            fraction,
                            normal: normal.to_array(),
                        },
                        witness: Some(ContactWitness {
                            point: contact.point.to_array(),
                            tolerance_m: contact.tolerance,
                        }),
                    };
                }
            }
        }
        Ok(result)
    }
}

impl physics::liquid::LiquidGeometry for SceneGeometry {
    type Error = String;
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::liquid::GeometryHit, String> {
        self.sweep_contact(center, radius, displacement, max_candidates)
            .map(|hit| hit.geometry)
    }
}

struct SceneBodyWorld {
    environment: SceneGeometry,
    // Root-relative world-oriented shapes for constrained translation; principal
    // COM-frame shapes when the corresponding trajectory has intrinsic Spin.
    templates: Vec<Vec<crate::convex::AffineBox>>,
}

fn sampling_frame(
    position: [f64; 3],
    duration: f64,
) -> Result<physics::rigid_motion::RigidMotion, physics::liquid::Error> {
    // Unit mass is only metadata for this stationary geometry sampling frame.
    // Environment walls never enter the solver's finite-body participant array.
    physics::contact::ContactBody {
        motion: physics::gravity::Body {
            mass: 1.,
            position,
            velocity: [0.; 3],
        },
        spin: None,
    }
    .prepare_motion(
        [0.; 3],
        [0.; 3],
        duration,
        physics::spin_path::Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 1,
            max_trials: 1,
        },
    )
    .map_err(|_| physics::liquid::Error::CollisionBackend)
}

fn trajectory_events(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    let mut steps = budget
        .checked_mul(64)
        .ok_or(physics::liquid::Error::CollisionBudget)?;
    let mut queries = steps;
    trajectory_events_counted(
        first,
        shapes,
        second,
        obstacles,
        budget,
        &mut steps,
        &mut queries,
    )
}
fn trajectory_events_counted(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    budget: usize,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    use physics::liquid::{Error, GeometryHit};
    if budget == 0
        || shapes
            .len()
            .checked_mul(obstacles.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    let mut result = RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear));
    let mut earliest = f64::INFINITY;
    for (first_index, shape) in shapes.iter().enumerate() {
        for (second_index, obstacle) in obstacles.iter().enumerate() {
            let contact = crate::angular_sweep::sweep_nominal_rigid_contact(
                first, *shape, second, *obstacle, steps, queries,
            );
            let hit = match contact {
                Ok(None) => RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear)),
                Ok(Some(contact)) => RigidGeometryHit {
                    feature: Some(
                        FeatureKey {
                            first: first_index,
                            second: second_index,
                            axis: contact.feature,
                        }
                        .encode()?,
                    ),
                    contact: BodyGeometryHit {
                        geometry: GeometryHit::Contact {
                            fraction: contact.time_s / first.duration(),
                            normal: contact.normal.to_array(),
                        },
                        witness: Some(ContactWitness {
                            point: contact.point.to_array(),
                            tolerance_m: contact.tolerance_m,
                        }),
                    },
                },
                Err(crate::PhysicsError::InitialOverlap) => {
                    return Ok(BodyGeometryHit::from(GeometryHit::Overlap).into());
                }
                Err(crate::PhysicsError::SweepBudget) => return Err(Error::CollisionBudget),
                Err(_) => return Err(Error::CollisionBackend),
            };
            if let GeometryHit::Contact { fraction, .. } = hit.contact.geometry {
                if fraction < earliest {
                    earliest = fraction;
                    result = hit;
                }
            }
        }
    }
    Ok(result)
}

fn affine_hit(
    shape: crate::convex::AffineBox,
    center: glam::DVec3,
    edges: [glam::DVec3; 3],
    displacement: glam::DVec3,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    if shape.penetration_affine(center, edges).is_some() {
        return Ok(GeometryHit::Overlap.into());
    }
    use crate::angular_sweep::{RigidBoxMotion, sweep_rigid_pair};
    let motion = |origin, edges, displacement| RigidBoxMotion {
        origin,
        displacement,
        orientation: glam::DQuat::IDENTITY,
        angular: glam::DVec3::ZERO,
        shape: crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges,
        },
    };
    let contact = sweep_rigid_pair(
        motion(center, edges, displacement),
        motion(shape.center, shape.edges, glam::DVec3::ZERO),
        &mut 1,
        &mut 1,
    )
    .map_err(|_| physics::liquid::Error::CollisionBackend)?;
    Ok(contact.map_or(GeometryHit::Clear.into(), |contact| {
        // Witness validation is part of admission even while angular motion is constrained.
        debug_assert!(contact.point.is_finite() && contact.tolerance.is_finite());
        BodyGeometryHit {
            geometry: GeometryHit::Contact {
                fraction: contact.fraction,
                normal: contact.normal.to_array(),
            },
            witness: Some(ContactWitness {
                point: contact.point.to_array(),
                tolerance_m: contact.tolerance,
            }),
        }
    }))
}
fn trajectory_environment_events(
    body: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    environment: &SceneGeometry,
    budget: usize,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    if shapes
        .len()
        .checked_mul(environment.0.0.len())
        .is_none_or(|n| n > budget)
    {
        return Err(physics::liquid::Error::CollisionBudget);
    }
    let mut steps = budget
        .checked_mul(64)
        .ok_or(physics::liquid::Error::CollisionBudget)?;
    let mut queries = steps;
    nearest_events(environment.0.0.iter().enumerate().map(|(wall, obstacle)| {
        let frame = sampling_frame(obstacle.shape.center.to_array(), body.duration())?;
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: obstacle.shape.edges,
        };
        let mut event = trajectory_events_counted(
            body,
            shapes,
            &frame,
            &[shape],
            budget,
            &mut steps,
            &mut queries,
        )?;
        if let Some(token) = event.feature {
            let mut key = FeatureKey::decode(token)?;
            key.second = wall;
            event.feature = Some(key.encode()?);
        }
        Ok(event)
    }))
}

/// Search every shape pair, admitting only explicit active support branches.
fn supported_shape_events(
    first: &physics::rigid_motion::RigidMotion,
    shapes: &[crate::convex::AffineBox],
    second: &physics::rigid_motion::RigidMotion,
    obstacles: &[crate::convex::AffineBox],
    supports: &[physics::liquid::RigidSupportPoint],
    budget: usize,
    fixed: bool,
) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
    if !first.has_constant_acceleration() || !second.has_constant_acceleration() {
        return Err(physics::liquid::Error::CollisionBackend);
    }
    use physics::liquid::Error;
    if budget == 0
        || supports.is_empty()
        || shapes
            .len()
            .checked_mul(obstacles.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    for support in supports {
        let key = FeatureKey::decode(support.feature.ok_or(Error::CollisionBackend)?)?;
        if key.first >= shapes.len() || key.second >= obstacles.len() {
            return Err(Error::InvalidCollision);
        }
    }
    let mut steps = budget.checked_mul(64).ok_or(Error::CollisionBudget)?;
    let mut queries = steps;
    let mut error: f64 = 0.;
    let mut events = Vec::new();
    for (ia, shape) in shapes.iter().enumerate() {
        for (ib, obstacle) in obstacles.iter().enumerate() {
            let active: Vec<_> = supports
                .iter()
                .copied()
                .filter(|s| {
                    let key = FeatureKey::decode(s.feature.unwrap()).unwrap();
                    key.first == ia && key.second == ib
                })
                .collect();
            if active.is_empty() {
                let mut hit = trajectory_events_counted(
                    first,
                    &[*shape],
                    second,
                    &[*obstacle],
                    budget,
                    &mut steps,
                    &mut queries,
                )?;
                if let Some(feature) = hit.feature {
                    let key = FeatureKey::decode(feature)?;
                    hit.feature = Some(
                        FeatureKey {
                            first: ia,
                            second: ib,
                            axis: key.axis,
                        }
                        .encode()?,
                    );
                }
                events.push(Ok(hit));
            } else {
                error = error.max(rigid_support::admit_motion(
                    first, *shape, second, *obstacle, fixed, &active,
                )?);
            }
        }
    }
    Ok(physics::liquid::SupportedGeometryHit {
        event: nearest_events(events.into_iter())?,
        support_error_m: error,
    })
}
fn nearest_events(
    events: impl Iterator<Item = Result<RigidGeometryHit, physics::liquid::Error>>,
) -> Result<RigidGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    let mut selected = RigidGeometryHit::from(BodyGeometryHit::from(GeometryHit::Clear));
    let mut earliest = f64::INFINITY;
    for event in events {
        let event = event?;
        match event.contact.geometry {
            GeometryHit::Overlap => return Ok(event),
            GeometryHit::Contact { fraction, .. } if fraction < earliest => {
                earliest = fraction;
                selected = event;
            }
            _ => {}
        }
    }
    Ok(selected)
}
fn nearest_hits(
    hits: impl Iterator<Item = Result<BodyGeometryHit, physics::liquid::Error>>,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    use physics::liquid::GeometryHit;
    let mut result = BodyGeometryHit::from(GeometryHit::Clear);
    let mut earliest = f64::INFINITY;
    for hit in hits {
        let hit = hit?;
        match hit.geometry {
            GeometryHit::Overlap => return Ok(hit),
            GeometryHit::Contact { fraction, .. } if fraction < earliest => {
                earliest = fraction;
                result = hit;
            }
            _ => {}
        }
    }
    Ok(result)
}
fn transport_contact(
    mut hit: BodyGeometryHit,
    displacement: glam::DVec3,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    if let (physics::liquid::GeometryHit::Contact { fraction, .. }, Some(witness)) =
        (hit.geometry, &mut hit.witness)
    {
        let offset = displacement * fraction;
        let point = glam::DVec3::from_array(witness.point) + offset;
        witness.point = point.to_array();
        witness.tolerance_m +=
            8. * f64::EPSILON * (point.abs().max_element() + offset.abs().max_element());
        if !point.is_finite() || !witness.tolerance_m.is_finite() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
    }
    Ok(hit)
}
fn transport_acceleration(
    mut hit: BodyGeometryHit,
    acceleration: [f64; 3],
    duration: f64,
) -> Result<BodyGeometryHit, physics::liquid::Error> {
    if let (physics::liquid::GeometryHit::Contact { fraction, .. }, Some(witness)) =
        (hit.geometry, &mut hit.witness)
    {
        let time = duration * fraction;
        let offset = glam::DVec3::from_array(acceleration) * (0.5 * time) * time;
        let point = glam::DVec3::from_array(witness.point) + offset;
        witness.point = point.to_array();
        witness.tolerance_m +=
            8. * f64::EPSILON * (point.abs().max_element() + offset.abs().max_element());
        if !point.is_finite() || !witness.tolerance_m.is_finite() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
    }
    Ok(hit)
}

fn contact_patch_points(
    first: &[crate::convex::AffineBox],
    a: &physics::contact::ContactBody,
    second: &[crate::convex::AffineBox],
    b: &physics::contact::ContactBody,
    witness: ContactWitness,
    normal: [f64; 3],
    budget: usize,
) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
    contact_patch_points_with_error(first, a, second, b, witness, normal, budget, 0.)
}
fn contact_patch_points_with_error(
    first: &[crate::convex::AffineBox],
    a: &physics::contact::ContactBody,
    second: &[crate::convex::AffineBox],
    b: &physics::contact::ContactBody,
    witness: ContactWitness,
    normal: [f64; 3],
    budget: usize,
    error_m: f64,
) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
    use glam::{DQuat, DVec3};
    use physics::liquid::Error;
    if budget == 0
        || first
            .len()
            .checked_mul(second.len())
            .is_none_or(|n| n > budget)
    {
        return Err(Error::CollisionBudget);
    }
    let qa = a
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = b
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let ca = DVec3::from_array(a.motion.position);
    let cb = DVec3::from_array(b.motion.position);
    let n = DVec3::from_array(normal);
    let mut contacts = Vec::new();
    for first in first {
        for second in second {
            let center = cb + qb * second.center;
            let obstacle = crate::convex::AffineBox {
                center,
                edges: second.edges.map(|e| qb * e),
            };
            let relative = crate::convex::AffineBox {
                center: ca - cb + qa * first.center - qb * second.center,
                edges: first.edges.map(|e| qa * e),
            };
            let w = DVec3::from_array(witness.point) - center;
            let scale = relative.center.length()
                + relative
                    .edges
                    .iter()
                    .chain(&obstacle.edges)
                    .map(|e| e.length())
                    .sum::<f64>();
            let tolerance = witness.tolerance_m
                + 256. * f64::EPSILON * (1. + scale + center.abs().max_element());
            let contains = |point: DVec3, shape: crate::convex::AffineBox| {
                let rows = glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2])
                    .inverse();
                let local = rows * (point - shape.center);
                rows.is_finite()
                    && (0..3).all(|k| {
                        local[k].abs() <= 1. + tolerance * rows.transpose().col(k).length()
                    })
            };
            let local_obstacle = crate::convex::AffineBox {
                center: DVec3::ZERO,
                ..obstacle
            };
            if !contains(w, local_obstacle)
                || !contains(w, relative)
                || (n.dot(w) - obstacle.radius(n)).abs() > tolerance
                || (n.dot(w - relative.center) + relative.radius(n)).abs() > tolerance
            {
                continue;
            }
            let gap = relative.center.dot(n) - obstacle.radius(n) - relative.radius(n);
            if gap.abs() > tolerance {
                continue;
            }
            let projected = crate::convex::AffineBox {
                center: relative.center - n * gap,
                ..relative
            };
            let patch = if error_m == 0. {
                obstacle.contact_patch_relative(&projected, n)
            } else {
                obstacle.contact_patch_relative_with_error(&projected, n, error_m)
            };
            let Ok((points, _)) = patch else {
                continue;
            };
            for point in points {
                if !contacts
                    .iter()
                    .any(|old: &physics::contact::NormalContact| {
                        (DVec3::from_array(old.point) - point).length() <= tolerance
                    })
                {
                    if contacts.len() >= 128 {
                        return Err(Error::CollisionBudget);
                    }
                    contacts.push(physics::contact::NormalContact {
                        point: point.to_array(),
                        normal,
                    });
                }
            }
        }
    }
    if contacts.is_empty() {
        Err(Error::InvalidCollision)
    } else {
        Ok(contacts)
    }
}
impl physics::liquid::LiquidBodyWorld for SceneBodyWorld {
    fn rigid_pair_support_contacts(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_pair_support_contacts_with_error(i, first, j, second, budget, 0.)
    }
    fn rigid_environment_support_contacts(
        &self,
        i: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        self.rigid_environment_support_contacts_with_error(i, body, budget, 0.)
    }
    fn rigid_pair_support_contacts_with_error(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let a = self.templates.get(i).ok_or(Error::InvalidCollision)?;
        let b = self.templates.get(j).ok_or(Error::InvalidCollision)?;
        if i == j || budget == 0 || a.len().checked_mul(b.len()).is_none_or(|n| n > budget) {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in a.iter().enumerate() {
            for (ib, obstacle) in b.iter().enumerate() {
                for (witness, normal, axis) in rigid_support::snapshot_with_error(
                    *first,
                    *shape,
                    Some(*second),
                    *obstacle,
                    error_m,
                )? {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        first,
                        &[*obstacle],
                        second,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane = rigid_support::plane(
                        *first,
                        *shape,
                        Some(*second),
                        *obstacle,
                        axis,
                        normal,
                    )?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn rigid_environment_support_contacts_with_error(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        budget: usize,
        error_m: f64,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, physics::liquid::Error> {
        use physics::liquid::Error;
        let shapes = self.templates.get(index).ok_or(Error::InvalidCollision)?;
        let walls = &self.environment.0.0;
        if budget == 0
            || shapes
                .len()
                .checked_mul(walls.len())
                .is_none_or(|n| n > budget)
        {
            return Err(Error::CollisionBudget);
        }
        let mut supports = Vec::new();
        for (ia, shape) in shapes.iter().enumerate() {
            for (ib, wall) in walls.iter().enumerate() {
                for (witness, normal, axis) in
                    rigid_support::snapshot_with_error(*body, *shape, None, wall.shape, error_m)?
                {
                    let token = FeatureKey {
                        first: ia,
                        second: ib,
                        axis,
                    }
                    .encode()?;
                    let fixed = physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            mass: 1.,
                            position: [0.; 3],
                            velocity: [0.; 3],
                        },
                        spin: None,
                    };
                    let contacts = contact_patch_points_with_error(
                        &[*shape],
                        body,
                        &[wall.shape],
                        &fixed,
                        witness,
                        normal,
                        budget,
                        error_m,
                    )?;
                    let plane =
                        rigid_support::plane(*body, *shape, None, wall.shape, axis, normal)?;
                    let patch = rigid_support::attach(contacts, plane);
                    if patch.len() > 128usize.saturating_sub(supports.len()) {
                        return Err(Error::CollisionBudget);
                    }
                    supports.extend(patch.into_iter().map(|support| {
                        physics::liquid::RigidSupportPoint {
                            support,
                            feature: Some(token),
                            tolerance_m: witness.tolerance_m,
                            admission_error_m: error_m,
                            carrying_reaction: false,
                        }
                    }));
                }
            }
        }
        Ok(supports)
    }
    fn sweep_supported_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        if !first.has_constant_acceleration() || !second.has_constant_acceleration() {
            return Err(physics::liquid::Error::CollisionBackend);
        }
        supported_shape_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            supports,
            budget,
            false,
        )
    }
    fn sweep_supported_rigid_environment_event(
        &self,
        i: usize,
        body: &physics::rigid_motion::RigidMotion,
        supports: &[physics::liquid::RigidSupportPoint],
        budget: usize,
    ) -> Result<physics::liquid::SupportedGeometryHit, physics::liquid::Error> {
        if !body.has_constant_acceleration() {
            return Err(physics::liquid::Error::CollisionBackend);
        }
        let fixed = sampling_frame([0.; 3], body.duration())?;
        let walls: Vec<_> = self.environment.0.0.iter().map(|w| w.shape).collect();
        supported_shape_events(
            body,
            &self.templates[i],
            &fixed,
            &walls,
            supports,
            budget,
            true,
        )
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_pair_patch(i, first, j, second, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        contact_patch_points(&[*a], first, &[*b], second, witness, normal, budget)
    }
    fn rigid_environment_patch_for_feature(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        let Some(token) = feature else {
            return self.rigid_environment_patch(index, body, witness, normal, budget);
        };
        let key = FeatureKey::decode(token)?;
        let shape = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &[*shape],
            body,
            &[wall.shape],
            &fixed,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_pair_supports(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(i)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let b = self
            .templates
            .get(j)
            .and_then(|s| s.get(key.second))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_pair_patch_for_feature(
            i,
            first,
            j,
            second,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*first, *a, Some(*second), *b, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }
    fn rigid_environment_supports(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: u64,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalSupport>, physics::liquid::Error> {
        let key = FeatureKey::decode(feature)?;
        let a = self
            .templates
            .get(index)
            .and_then(|s| s.get(key.first))
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let wall = self
            .environment
            .0
            .0
            .get(key.second)
            .ok_or(physics::liquid::Error::InvalidCollision)?;
        let contacts = self.rigid_environment_patch_for_feature(
            index,
            body,
            witness,
            normal,
            Some(feature),
            budget,
        )?;
        let plane = rigid_support::plane(*body, *a, None, wall.shape, key.axis, normal)?;
        Ok(rigid_support::attach(contacts, plane))
    }

    fn rigid_pair_patch(
        &self,
        i: usize,
        first: &physics::contact::ContactBody,
        j: usize,
        second: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        contact_patch_points(
            &self.templates[i],
            first,
            &self.templates[j],
            second,
            witness,
            normal,
            budget,
        )
    }
    fn rigid_environment_patch(
        &self,
        index: usize,
        body: &physics::contact::ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, physics::liquid::Error> {
        if budget == 0
            || self.templates[index]
                .len()
                .checked_mul(self.environment.0.0.len())
                .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        let obstacles: Vec<_> = self.environment.0.0.iter().map(|b| b.shape).collect();
        let fixed = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: None,
        };
        contact_patch_points(
            &self.templates[index],
            body,
            &obstacles,
            &fixed,
            witness,
            normal,
            budget,
        )
    }

    fn sweep_particle_rigid_contact(
        &self,
        p: &physics::liquid::Particle,
        r: f64,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_particle_rigid_event(p, r, i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        a: &physics::rigid_motion::RigidMotion,
        j: usize,
        b: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_pair_event(i, a, j, b, budget)
            .map(|event| event.contact)
    }
    fn sweep_rigid_environment_contact(
        &self,
        i: usize,
        path: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.sweep_rigid_environment_event(i, path, budget)
            .map(|event| event.contact)
    }
    fn sweep_particle_motion_body_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() || particle.duration() != body.duration() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if body.has_constant_acceleration()
            && particle.has_constant_acceleration()
            && body.initial().spin.is_none()
            && particle.acceleration() == body.acceleration()
        {
            let state = body.initial().motion;
            let hit = self.sweep_particle_body_contact(
                p,
                radius,
                index,
                &physics::liquid::TranslatingBody {
                    mass: state.mass,
                    position: state.position,
                    velocity: state.velocity,
                },
                body.duration(),
                budget,
            )?;
            return transport_acceleration(hit, body.acceleration(), body.duration())
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_events(particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_particle_motion_environment_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        particle: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if particle.initial().spin.is_some() {
            return Err(physics::liquid::Error::InvalidCollision);
        }
        if particle.has_constant_acceleration() && particle.acceleration() == [0.; 3] {
            return self
                .sweep_particle_environment_contact(p, radius, particle.duration(), budget)
                .map(Into::into);
        }
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_environment_events(particle, &[shape], &self.environment, budget)
    }
    fn sweep_particle_rigid_event(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_particle_body_contact(
                    p,
                    radius,
                    index,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    body.duration(),
                    budget,
                )
                .map(Into::into);
        }
        let particle = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: p.mass,
                position: p.position,
                velocity: p.velocity,
            },
            spin: None,
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            body.duration(),
            physics::spin_path::Config {
                max_angular_error_rad: 1e-5,
                min_step_s: 1e-9,
                max_arcs: 1,
                max_trials: 1,
            },
        )
        .map_err(|_| physics::liquid::Error::CollisionBackend)?;
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * radius,
                glam::DVec3::Y * radius,
                glam::DVec3::Z * radius,
            ],
        };
        trajectory_events(&particle, &[shape], body, &self.templates[index], budget)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        first: &physics::rigid_motion::RigidMotion,
        j: usize,
        second: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if first.has_constant_acceleration()
            && second.has_constant_acceleration()
            && first.initial().spin.is_none()
            && second.initial().spin.is_none()
            && first.acceleration() == [0.; 3]
            && second.acceleration() == [0.; 3]
        {
            let a = first.initial().motion;
            let b = second.initial().motion;
            return self
                .sweep_body_pair_contact(
                    i,
                    &physics::liquid::TranslatingBody {
                        mass: a.mass,
                        position: a.position,
                        velocity: a.velocity,
                    },
                    j,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    first.duration(),
                    budget,
                )
                .map(Into::into);
        }
        trajectory_events(
            first,
            &self.templates[i],
            second,
            &self.templates[j],
            budget,
        )
    }
    fn sweep_rigid_environment_event(
        &self,
        index: usize,
        body: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, physics::liquid::Error> {
        if body.has_constant_acceleration()
            && body.initial().spin.is_none()
            && body.acceleration() == [0.; 3]
        {
            let b = body.initial().motion;
            return self
                .sweep_body_environment_contact(
                    index,
                    &physics::liquid::TranslatingBody {
                        mass: b.mass,
                        position: b.position,
                        velocity: b.velocity,
                    },
                    body.duration(),
                    budget,
                )
                .map(Into::into);
        }
        trajectory_environment_events(body, &self.templates[index], &self.environment, budget)
    }

    fn sweep_particle_body(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_body_contact(p, radius, index, body, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_pair(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_pair_contact(first_index, first, second_index, second, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_particle_environment(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_particle_environment_contact(p, radius, dt, budget)
            .map(|hit| hit.geometry)
    }
    fn sweep_body_environment(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        self.sweep_body_environment_contact(index, body, dt, budget)
            .map(|hit| hit.geometry)
    }

    fn sweep_particle_body_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index].len() > budget {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().map(|template| {
            let mut shape = *template;
            shape.center += glam::DVec3::from_array(body.position);
            affine_hit(
                shape,
                glam::DVec3::from_array(p.position),
                [
                    glam::DVec3::X * radius,
                    glam::DVec3::Y * radius,
                    glam::DVec3::Z * radius,
                ],
                (glam::DVec3::from_array(p.velocity) - glam::DVec3::from_array(body.velocity)) * dt,
            )
            .and_then(|hit| transport_contact(hit, glam::DVec3::from_array(body.velocity) * dt))
        }))
    }
    fn sweep_body_pair_contact(
        &self,
        first_index: usize,
        first: &physics::liquid::TranslatingBody,
        second_index: usize,
        second: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[first_index]
            .len()
            .checked_mul(self.templates[second_index].len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[first_index].iter().flat_map(|template| {
            self.templates[second_index].iter().map(move |obstacle| {
                let mut shape = *obstacle;
                shape.center += glam::DVec3::from_array(second.position);
                affine_hit(
                    shape,
                    template.center + glam::DVec3::from_array(first.position),
                    template.edges,
                    (glam::DVec3::from_array(first.velocity)
                        - glam::DVec3::from_array(second.velocity))
                        * dt,
                )
                .and_then(|hit| {
                    transport_contact(hit, glam::DVec3::from_array(second.velocity) * dt)
                })
            })
        }))
    }
    fn sweep_particle_environment_contact(
        &self,
        p: &physics::liquid::Particle,
        radius: f64,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        self.environment
            .sweep_contact(p.position, radius, p.velocity.map(|v| v * dt), budget)
            .map_err(|_| physics::liquid::Error::CollisionBackend)
    }
    fn sweep_body_environment_contact(
        &self,
        index: usize,
        body: &physics::liquid::TranslatingBody,
        dt: f64,
        budget: usize,
    ) -> Result<BodyGeometryHit, physics::liquid::Error> {
        if self.templates[index]
            .len()
            .checked_mul(self.environment.0.0.len())
            .is_none_or(|n| n > budget)
        {
            return Err(physics::liquid::Error::CollisionBudget);
        }
        nearest_hits(self.templates[index].iter().flat_map(|template| {
            self.environment.0.0.iter().map(move |obstacle| {
                affine_hit(
                    obstacle.shape,
                    template.center + glam::DVec3::from_array(body.position),
                    template.edges,
                    glam::DVec3::from_array(body.velocity) * dt,
                )
            })
        }))
    }
    fn has_environment(&self) -> bool {
        !self.environment.0.0.is_empty()
    }
}

fn scene_rotation_config() -> physics::spin_path::Config {
    physics::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}

impl SceneLiquidRuntime {
    /// Resolve durable material names once and admit every source, including inactive ones.
    /// # Errors
    /// Duplicate/empty material names, invalid physics/source settings or source budget.
    pub fn new(
        scene: &SceneGraph,
        materials: Vec<(String, Material)>,
        config: Config,
        max_sources: usize,
    ) -> Result<Self, String> {
        let mut slots = BTreeMap::new();
        for (slot, (name, _)) in materials.iter().enumerate() {
            if name.is_empty() || slots.insert(name.clone(), slot).is_some() {
                return Err("duplicate or empty liquid material identity".into());
            }
        }
        for (node, _) in scene.components::<crate::LiquidMassDistribution>() {
            if scene
                .component::<crate::LiquidBody>(node)
                .map_err(|e| format!("mass owner: {e:?}"))?
                .is_none()
            {
                return Err("mass distribution requires a liquid body owner".into());
            }
        }
        let owners: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if owners.len() > 128 {
            return Err("scene liquid translating-body budget exceeded (128)".into());
        }
        let body = owners
            .iter()
            .map(|(node, descriptor)| {
                if !descriptor.mass_kg.is_finite()
                    || descriptor.mass_kg <= 0.
                    || descriptor
                        .initial_velocity_m_s
                        .iter()
                        .any(|v| !v.is_finite())
                {
                    return Err("invalid liquid body mass/velocity".to_string());
                }
                if scene
                    .parent(*node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
                    || scene
                        .component::<crate::CharacterBody>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                    || scene
                        .component::<crate::AngularMotion>(*node)
                        .map_err(|e| format!("liquid body ownership: {e:?}"))?
                        .is_some()
                {
                    return Err("liquid body requires an exclusively owned root transform".into());
                }
                let colliders = body_colliders(scene, *node)?;
                let world =
                    crate::static_world(scene).map_err(|e| format!("liquid environment: {e:?}"))?;
                for own in world
                    .0
                    .iter()
                    .filter(|b| colliders.iter().any(|c| c.node == b.owner))
                {
                    if world.0.iter().any(|b| {
                        !colliders.iter().any(|c| c.node == b.owner)
                            && b.shape
                                .penetration_affine(own.shape.center, own.shape.edges)
                                .is_some()
                    }) {
                        return Err("liquid body initially overlaps scene geometry".into());
                    }
                }
                let published = scene
                    .local(*node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?;
                let mass_descriptor = scene
                    .component::<crate::LiquidMassDistribution>(*node)
                    .map_err(|e| format!("mass descriptor: {e:?}"))?
                    .cloned();
                let mass_properties = mass_descriptor
                    .as_ref()
                    .map(|d| d.prepare(descriptor.mass_kg, published))
                    .transpose()?;
                let rigid_frame = mass_properties
                    .map(|p| crate::RigidBodyFrame::new(published, p))
                    .transpose()
                    .map_err(|e| format!("mass principal frame: {e:?}"))?;
                let pivot = position(scene, *node)?;
                let state = if let Some(frame) = rigid_frame {
                    frame
                        .prepare_body(pivot, descriptor.initial_velocity_m_s, [0.; 3])
                        .map_err(|e| format!("rigid body seed: {e:?}"))?
                } else {
                    physics::contact::ContactBody {
                        motion: physics::gravity::Body {
                            position: pivot,
                            velocity: descriptor.initial_velocity_m_s,
                            mass: descriptor.mass_kg,
                        },
                        spin: None,
                    }
                };
                let mut owner = BodyOwner {
                    node: *node,
                    descriptor: **descriptor,
                    state,
                    principal_templates: None,
                    published,
                    colliders,
                    mass_descriptor,
                    mass_properties,
                    rigid_frame,
                };
                if let (Some(properties), Some(spin)) = (owner.mass_properties, owner.state.spin) {
                    let inverse = glam::DQuat::from_array(spin.orientation).conjugate();
                    owner.principal_templates = Some(
                        owner
                            .colliders
                            .iter()
                            .map(|collider| {
                                let shape = collider_template(&owner, collider)?;
                                Ok(crate::convex::AffineBox {
                                    center: inverse
                                        * (shape.center
                                            - glam::DVec3::from_array(properties.center)),
                                    edges: shape.edges.map(|edge| inverse * edge),
                                })
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    );
                }
                Ok(owner)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let liquid = Liquid::new(
            Vec::new(),
            materials.into_iter().map(|(_, m)| m).collect(),
            config,
        )
        .map_err(|e| format!("liquid world: {e:?}"))?;
        let mut sources = BTreeMap::new();
        for (node, descriptor) in scene.components::<LiquidSource>() {
            if sources.len() >= max_sources {
                return Err("scene liquid source budget exceeded".into());
            }
            let slot = *slots.get(&descriptor.material_asset).ok_or_else(|| {
                format!("unresolved liquid material: {}", descriptor.material_asset)
            })?;
            let position = position(scene, node)?;
            let emitter = descriptor
                .prepare(position, slot)
                .map_err(|e| format!("liquid source: {e:?}"))?;
            sources.insert(
                node,
                Source {
                    descriptor: descriptor.clone(),
                    emitter,
                    position,
                },
            );
        }
        Ok(Self {
            scene: scene.identity(),
            sources,
            liquid,
            body,
        })
    }

    #[must_use]
    pub fn liquid(&self) -> &Liquid {
        &self.liquid
    }
    #[must_use]
    pub fn source_elapsed(&self, node: NodeId) -> Option<f64> {
        self.sources.get(&node).map(|s| s.emitter.elapsed())
    }

    /// Validate the runtime belongs to this scene and still owns its admitted sources.
    /// # Errors
    /// Foreign scenes, source addition/removal or descriptor edits require rebind.
    pub fn validate_bindings(&self, scene: &SceneGraph) -> Result<(), String> {
        if scene.identity() != self.scene {
            return Err("foreign scene liquid runtime".into());
        }
        let descriptors: BTreeMap<_, _> = scene.components::<LiquidSource>().collect();
        if descriptors.len() != self.sources.len()
            || self.sources.iter().any(|(node, source)| {
                descriptors
                    .get(node)
                    .is_none_or(|d| **d != source.descriptor)
            })
        {
            return Err("liquid source ownership or descriptor changed; rebind runtime".into());
        }
        let bodies: Vec<_> = scene.components::<crate::LiquidBody>().collect();
        if bodies.len() != self.body.len() {
            return Err("liquid body ownership changed; rebind runtime".into());
        }
        if scene.components::<crate::LiquidMassDistribution>().count()
            != self
                .body
                .iter()
                .filter(|b| b.mass_descriptor.is_some())
                .count()
        {
            return Err("mass distribution ownership changed; rebind runtime".into());
        }
        for body in &self.body {
            if scene
                .component::<crate::LiquidMassDistribution>(body.node)
                .map_err(|e| format!("mass binding: {e:?}"))?
                != body.mass_descriptor.as_ref()
            {
                return Err("mass distribution changed; rebind runtime".into());
            }
            if scene
                .component::<crate::CharacterBody>(body.node)
                .map_err(|e| format!("liquid body ownership: {e:?}"))?
                .is_some()
                || scene
                    .component::<crate::AngularMotion>(body.node)
                    .map_err(|e| format!("liquid body ownership: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body has another transform owner".into());
            }
            if body_colliders(scene, body.node)? != body.colliders {
                return Err("compound collider bindings changed; rebind runtime".into());
            }
            if !bodies
                .iter()
                .any(|(node, descriptor)| *node == body.node && **descriptor == body.descriptor)
                || scene
                    .local(body.node)
                    .map_err(|e| format!("liquid body pose: {e:?}"))?
                    != body.published
                || scene
                    .parent(body.node)
                    .map_err(|e| format!("liquid body parent: {e:?}"))?
                    .is_some()
            {
                return Err("liquid body descriptor/pose changed; rebind runtime".into());
            }
        }
        Ok(())
    }

    /// Prepare emission, fluid/body motion and scene publication as one transaction.
    /// # Errors
    /// Any preparation/publication failure preserves this runtime and the scene pose.
    pub fn tick_and_publish(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, String> {
        self.tick_and_publish_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Prepare and atomically publish with explicit restitution/contact and angular budgets.
    /// The normal angular solver currently requires zero tangential friction.
    /// # Errors
    /// Invalid settings, unresolved contacts or publication errors preserve both owners and scene.
    pub fn tick_and_publish_with_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, String> {
        let mut candidate = self.clone();
        let report = candidate.tick_with_dynamics(scene, dt, container, dynamics, rotation)?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    /// Publish an already prepared body pose after the host's other preparation succeeds.
    /// # Errors
    /// Foreign/edited scene bindings or unrepresentable pose reject before mutation.
    pub fn publish_body_pose(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        self.validate_bindings(scene)?;
        let poses = self
            .body
            .iter()
            .map(|body| {
                let mut pose = body.published;
                if let Some(frame) = body.rigid_frame {
                    let radius = body
                        .principal_templates
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|s| {
                            s.center.length() + s.edges.iter().map(|e| e.length()).sum::<f64>()
                        })
                        .fold(0., f64::max);
                    pose = frame
                        .prepare_pose(body.state, radius, 1e-5)
                        .map_err(|e| format!("rigid pose publication: {e:?}"))?
                        .pose;
                } else {
                    pose.translation =
                        glam::DVec3::from_array(body.state.motion.position).as_vec3();
                }
                pose.matrix()
                    .map_err(|e| format!("liquid body publication: {e:?}"))?;
                Ok((body.node, pose))
            })
            .collect::<Result<Vec<_>, String>>()?;
        scene
            .set_locals(&poses)
            .map_err(|e| format!("liquid body publication: {e:?}"))?;
        for (body, (_, pose)) in self.body.iter_mut().zip(poses) {
            body.published = pose;
        }
        Ok(())
    }
    /// First admitted body for compatibility; use body_states for the entire world.
    #[must_use]
    pub fn body_state(&self) -> Option<(NodeId, physics::liquid::TranslatingBody)> {
        self.body_states().next()
    }

    /// All admitted COM states (legacy bodies use the root pivot), including paused
    /// inactive bodies, in stable owner order. Rotation is exposed by body_rigid_states.
    pub fn body_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::liquid::TranslatingBody)> + '_ {
        self.body.iter().map(|b| {
            (
                b.node,
                physics::liquid::TranslatingBody {
                    position: b.state.motion.position,
                    velocity: b.state.motion.velocity,
                    mass: b.state.motion.mass,
                },
            )
        })
    }

    /// Admitted mass tensor and COM offset in the root-relative world-oriented frame.
    /// Legacy bodies without an explicit distribution have no inferred inertia.
    pub fn body_mass_properties(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<physics::mass_properties::MassProperties>)> + '_ {
        self.body.iter().map(|b| (b.node, b.mass_properties))
    }

    /// Persistent mechanical COM states, including intrinsic rotation when mass is authored.
    pub fn body_rigid_states(
        &self,
    ) -> impl Iterator<Item = (NodeId, physics::contact::ContactBody)> + '_ {
        self.body.iter().map(|b| (b.node, b.state))
    }

    /// Admitted COM/principal adapters.
    pub fn body_rigid_frames(
        &self,
    ) -> impl Iterator<Item = (NodeId, Option<crate::RigidBodyFrame>)> + '_ {
        self.body.iter().map(|b| (b.node, b.rigid_frame))
    }

    /// Build the active rigid geometry world once for both dynamics and support queries.
    fn mechanical_world(
        &self,
        scene: &SceneGraph,
    ) -> Result<
        (
            SceneBodyWorld,
            Vec<usize>,
            Vec<physics::contact::ContactBody>,
        ),
        String,
    > {
        let mut geometry = SceneGeometry(
            crate::static_world(scene).map_err(|e| format!("liquid collider: {e:?}"))?,
        );
        let mut active = Vec::new();
        let mut templates = Vec::new();
        let mut states = Vec::new();
        for (owner, body) in self.body.iter().enumerate() {
            if !scene
                .active_in_hierarchy(body.node)
                .map_err(|e| format!("liquid body activity: {e:?}"))?
            {
                continue;
            }
            let mut shapes = Vec::new();
            for (slot, collider) in body.colliders.iter().enumerate() {
                if let Some(index) = geometry.0.0.iter().position(|b| b.owner == collider.node) {
                    geometry.0.0.remove(index);
                    shapes.push(if let Some(principal) = &body.principal_templates {
                        principal[slot]
                    } else {
                        collider_template(body, collider)?
                    });
                }
            }
            active.push(owner);
            templates.push(shapes);
            states.push(body.state);
        }
        Ok((
            SceneBodyWorld {
                environment: geometry,
                templates,
            },
            active,
            states,
        ))
    }

    /// Gather current supporting patches and resolve coupled reactions under
    /// configured gravity plus additional COM loads. This is read-only and does
    /// not certify or advance a constrained trajectory. Returned indices follow owners.
    /// # Errors
    /// Invalid/edited scene bindings, no active bodies, duplicate/foreign/inactive
    /// load owners, geometry overlap, limits or reaction nonconvergence.
    pub fn support_reactions(
        &self,
        scene: &SceneGraph,
        additional: &[(NodeId, physics::contact::ContactWrench)],
        limits: physics::liquid::DynamicWorldConfig,
        config: physics::contact::ReactionConfig,
    ) -> Result<SceneSupportReactions, String> {
        self.validate_bindings(scene)?;
        let (world, active, states) = self.mechanical_world(scene)?;
        let owners: Vec<_> = active.iter().map(|i| self.body[*i].node).collect();
        let mut wrenches = vec![physics::contact::ContactWrench::default(); owners.len()];
        let mut used = Vec::new();
        for (node, wrench) in additional {
            let index = owners
                .iter()
                .position(|owner| owner == node)
                .ok_or_else(|| "support load has a foreign or inactive owner".to_string())?;
            if used.contains(&index) {
                return Err("duplicate support load owner".into());
            }
            used.push(index);
            wrenches[index] = *wrench;
        }
        let reactions = self
            .liquid
            .rigid_world_reactions(&states, &world, &wrenches, limits, config)
            .map_err(|e| format!("scene support reactions: {e:?}"))?;
        Ok(SceneSupportReactions { owners, reactions })
    }

    /// Emit then simulate on an explicitly supplied fixed interval. No caller state changes.
    /// World positions follow scene transforms; nozzle velocity is displacement / interval.
    /// Inactive sources track position without advancing their pulse clock.
    /// # Errors
    /// Foreign scene, descriptor/ownership edits, invalid time, emission or physics failure.
    /// The entire world and every source clock remain unchanged on any error.
    pub fn tick(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
    ) -> Result<SceneLiquidStep, String> {
        self.tick_with_dynamics(
            scene,
            dt,
            container,
            Default::default(),
            scene_rotation_config(),
        )
    }

    /// Simulate using caller-specified mechanical response and angular admission budgets.
    /// # Errors
    /// Same transactional binding, emission and geometry guarantees as tick.
    pub fn tick_with_dynamics(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
    ) -> Result<SceneLiquidStep, String> {
        self.tick_impl(scene, dt, container, dynamics, rotation, None)
            .map(|report| report.step)
    }

    /// Advance and publish through the existing scene owner/event loop with
    /// geometry-admitted finite normal reactions. Unsupported changing branches
    /// reject with complete runtime/scene rollback, including emitted sources.
    /// # Errors
    /// Invalid budgets/bindings, unresolved support evolution or pose publication.
    pub fn tick_and_publish_with_supported_dynamics(
        &mut self,
        scene: &mut SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: physics::liquid::SupportedWorldConfig,
    ) -> Result<SceneSupportedLiquidStep, String> {
        supports
            .validate()
            .map_err(|e| format!("supported liquid config: {e:?}"))?;
        let mut candidate = self.clone();
        let report =
            candidate.tick_impl(scene, dt, container, dynamics, rotation, Some(supports))?;
        candidate.publish_body_pose(scene)?;
        *self = candidate;
        Ok(report)
    }

    fn tick_impl(
        &mut self,
        scene: &SceneGraph,
        dt: f64,
        container: Option<Container>,
        dynamics: physics::liquid::DynamicWorldConfig,
        rotation: physics::spin_path::Config,
        supports: Option<physics::liquid::SupportedWorldConfig>,
    ) -> Result<SceneSupportedLiquidStep, String> {
        self.validate_bindings(scene)?;
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid liquid fixed interval".into());
        }
        let mut candidate = self.clone();
        let mut emissions = Vec::new();
        for (node, source) in &mut candidate.sources {
            let current = position(scene, *node)?;
            source.emitter.template.particle.position = current;
            source.emitter.source_velocity =
                std::array::from_fn(|axis| (current[axis] - source.position[axis]) / dt);
            source.position = current;
            if scene
                .active_in_hierarchy(*node)
                .map_err(|e| format!("liquid source activity: {e:?}"))?
            {
                let exchange = source
                    .emitter
                    .advance(&mut candidate.liquid, dt)
                    .map_err(|e| format!("liquid source {node:?}: {e:?}"))?;
                emissions.push((*node, exchange));
            }
        }
        if !candidate.body.is_empty() && container.is_some() {
            return Err(
                "authored liquid bodies use scene colliders, not an extra container".into(),
            );
        }
        let (world, active, mut states) = candidate.mechanical_world(scene)?;
        let mut support_report = None;
        let (physics, dynamics) = if states.is_empty() {
            (
                candidate
                    .liquid
                    .step_with_geometry(dt, container, &world.environment, dynamics.contact)
                    .map_err(|e| format!("scene liquid step: {e:?}"))?,
                None,
            )
        } else {
            let report = if let Some(config) = supports {
                let loads = vec![physics::contact::ContactWrench::default(); states.len()];
                let report = candidate
                    .liquid
                    .step_with_supported_rigid_body_forces(
                        dt,
                        &mut states,
                        &world,
                        dynamics,
                        128,
                        rotation,
                        &loads,
                        config,
                    )
                    .map_err(|e| format!("scene supported liquid body step: {e:?}"))?;
                support_report = Some(report);
                report.rigid.world
            } else {
                candidate
                    .liquid
                    .step_with_rigid_body_world(dt, &mut states, &world, dynamics, 128, rotation)
                    .map_err(|e| format!("scene liquid body step: {e:?}"))?
            };
            for (owner, state) in active.into_iter().zip(states) {
                candidate.body[owner].state = state;
            }
            (report.dynamics.fluid, Some(report))
        };
        *self = candidate;
        Ok(SceneSupportedLiquidStep {
            step: SceneLiquidStep {
                emissions,
                physics,
                dynamics,
            },
            support: support_report,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LiquidPulse;
    use voxy_scene::Transform;
    #[test]
    fn supported_motion_scene_owner_publishes_and_restores_after_late_failure() {
        let mut scene = support_floor();
        let body = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0., 0.125, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::BoxCollider {
                    half_extents: [0.125; 3],
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidBody {
                    mass_kg: 2.,
                    initial_velocity_m_s: [1., 0., 0.],
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidMassDistribution {
                    parts: vec![crate::LiquidMassPart {
                        mass_kg: 2.,
                        center_m: [0.; 3],
                        half_edges_m: [[0.125, 0., 0.], [0., 0.125, 0.], [0., 0., 0.125]],
                    }],
                },
            )
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let before = runtime.clone();
        let pose = scene.local(body).unwrap();
        assert!(
            runtime
                .tick_and_publish_with_supported_dynamics(
                    &mut scene,
                    0.1,
                    None,
                    Default::default(),
                    rigid_config(),
                    physics::liquid::SupportedWorldConfig {
                        max_intervals: 2,
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert_eq!(runtime, before);
        assert_eq!(scene.local(body).unwrap(), pose);
        for tick in 1..=2 {
            let report = runtime
                .tick_and_publish_with_supported_dynamics(
                    &mut scene,
                    0.1,
                    None,
                    Default::default(),
                    rigid_config(),
                    Default::default(),
                )
                .unwrap();
            assert!(
                (scene.local(body).unwrap().translation.x as f64 - 0.1 * tick as f64).abs() < 1e-7
            );
            assert!((scene.local(body).unwrap().translation.y as f64 - 0.125).abs() < 1e-8);
            assert!((report.support.unwrap().environment_reaction_impulse[1] + 2.).abs() < 1e-10);
        }
    }
    #[test]
    fn supported_motion_retains_small_spin_and_rejects_unresolved_real_rotation() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.125; 3])]],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 1e-12)];
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.01,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                Default::default(),
            )
            .unwrap();
        assert_ne!(bodies[0].spin.unwrap().angular_momentum[2], 0.);
        assert_ne!(bodies[0].spin.unwrap().orientation[2], 0.);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
        let mut rotating = [rigid_body([0., 0.125, 0.], [0.; 3], 1.)];
        let before = rotating;
        let fluid_before = liquid.clone();
        assert!(
            liquid
                .step_with_supported_rigid_body_forces(
                    0.01,
                    &mut rotating,
                    &world,
                    Default::default(),
                    1,
                    rigid_config(),
                    &[ContactWrench::default()],
                    Default::default()
                )
                .is_err()
        );
        assert_eq!(rotating, before);
        assert_eq!(liquid, fluid_before);
    }
    #[test]
    fn supported_motion_keeps_real_tangential_acceleration_and_separates_work() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.125; 3])]],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies = [rigid_body([0., 0.125, 0.], [1., 0., 0.], 0.)];
        bodies[0].motion.mass = 2.;
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench {
                    force: [4., 0., 0.],
                    torque: [0.; 3],
                }],
                Default::default(),
            )
            .unwrap();
        assert!((bodies[0].motion.position[0] - 0.11).abs() < 1e-12);
        assert!((bodies[0].motion.velocity[0] - 1.2).abs() < 1e-12);
        assert!((bodies[0].motion.position[1] - 0.125).abs() < 1e-12);
        assert!(bodies[0].motion.velocity[1].abs() < 1e-11);
        assert!((report.rigid.external_work - 0.44).abs() < 1e-11);
        assert!(report.reaction_work.abs() < 1e-11);
        assert!(report.rigid.integration_energy_residual.abs() < 1e-11);
        assert!((report.environment_reaction_impulse[1] + 2.).abs() < 1e-11);
        // Floor force is 20 N and x(t)=t+t². Its opposite angular impulse
        // about the world origin integrates the moving application point.
        let expected_moment = 20. * (0.1_f64.powi(2) / 2. + 0.1_f64.powi(3) / 3.);
        assert!((report.reaction_angular_impulse[2] - expected_moment).abs() < 1e-11);
        assert!((report.environment_reaction_angular_impulse[2] + expected_moment).abs() < 1e-11);
        assert!(
            report
                .reaction_angular_balance_residual
                .iter()
                .all(|x| x.abs() < 1e-11)
        );

        assert_eq!(
            report.environment_reaction_impulse,
            report.rigid.world.environment_impulse
        );
        assert_eq!(report.rigid.world.dynamics.contacts, 0);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
        assert!(report.supported_intervals > 0);
        assert!(report.max_support_error_m < 1e-11);
    }
    #[test]
    fn cubic_collision_detects_inside_step_motion_and_preserves_world_feature() {
        use physics::{
            astrophysics_spin::TorquePolynomial,
            liquid::{GeometryHit, LiquidBodyWorld},
        };
        let mut scene = SceneGraph::new(2);
        let node = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.125, 1., 1.],
                },
            )
            .unwrap();
        let shape = support_box([0.125; 3]);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![shape], vec![shape]],
        };
        let zero = TorquePolynomial::constant([0.; 3]);
        for (a, j, spin) in [(36., -72., true), (0., 36., false)] {
            let mut initial = rigid_body([-3., 0., 0.], [0.; 3], 0.);
            if !spin {
                initial.spin = None;
            }
            let path = initial
                .prepare_affine_motion([a, 0., 0.], [j, 0., 0.], zero, 1., rigid_config())
                .unwrap();
            let end = path.end();
            assert!((end.motion.position[0] - 3.).abs() < 1e-12);
            let original = path.clone();
            // Both endpoint poses are separated, yet the cubic path crosses the wall.
            let event = world.sweep_rigid_environment_event(0, &path, 256).unwrap();
            let GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
                panic!("missed cubic crossing: {event:?}");
            };
            let mut lo = 0.;
            let mut hi = 1.;
            for _ in 0..64 {
                let mid = (lo + hi) * 0.5;
                if -3. + a * mid * mid / 2. + j * mid * mid * mid / 6. < -0.25 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            assert!((fraction - (lo + hi) * 0.5).abs() < 1e-8);
            assert_eq!(normal, [-1., 0., 0.]);
            assert!((event.contact.witness.unwrap().point[0] + 0.125).abs() < 1e-8);
            assert_eq!(
                FeatureKey::decode(event.feature.unwrap()).unwrap().second,
                0
            );
            assert!(world.sweep_rigid_environment_event(0, &path, 0).is_err());
            assert_eq!(path, original);
            let fixed = rigid_body([0.; 3], [0.; 3], 0.)
                .prepare_motion([0.; 3], [0.; 3], 1., rigid_config())
                .unwrap();
            let pair = world
                .sweep_rigid_pair_event(0, &path, 1, &fixed, 256)
                .unwrap();
            let GeometryHit::Contact {
                fraction: pair_fraction,
                normal: pair_normal,
            } = pair.contact.geometry
            else {
                panic!("missed cubic pair");
            };
            assert!((pair_fraction - fraction).abs() < 1e-8);
            assert_eq!(pair_normal, normal);
            let compound = SceneBodyWorld {
                environment: SceneGeometry(crate::static_world(&scene).unwrap()),
                templates: vec![
                    vec![shape],
                    vec![crate::convex::AffineBox {
                        center: glam::DVec3::X * 0.5,
                        ..shape
                    }],
                ],
            };
            let shifted = compound
                .sweep_rigid_pair_event(0, &path, 1, &fixed, 256)
                .unwrap();
            let GeometryHit::Contact {
                fraction: shifted_fraction,
                ..
            } = shifted.contact.geometry
            else {
                panic!("missed compound cubic pair");
            };
            let mut lo = 0.;
            let mut hi = 1.;
            for _ in 0..64 {
                let mid = (lo + hi) * 0.5;
                if -3. + a * mid * mid / 2. + j * mid * mid * mid / 6. < 0.25 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            assert!((shifted_fraction - (lo + hi) * 0.5).abs() < 1e-8);
            assert!((shifted.contact.witness.unwrap().point[0] - 0.375).abs() < 1e-8);
        }
    }

    #[test]
    fn supported_motion_finite_sliding_transmits_moving_arm_torque_without_internal_couple() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![
                vec![support_box([0.125; 3])],
                vec![support_box([1., 0.125, 1.])],
            ],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies = [
            rigid_body([0., 0.375, 0.], [1., 0., 0.], 0.),
            rigid_body([0., 0.125, 0.], [0.; 3], 0.),
        ];
        bodies[0].motion.mass = 2.;
        bodies[1].motion.mass = 3.;
        let before_query = bodies;
        let before_liquid = liquid.clone();
        let rate_report = liquid
            .rigid_world_reaction_rates(
                &bodies,
                &world,
                &[ContactWrench::default(); 2],
                &[ContactWrench::default(); 2],
                Default::default(),
                physics::contact::ReactionRateConfig {
                    reaction: physics::liquid::SupportedWorldConfig::default().reaction,
                    jerk_tolerance: 1e-10,
                },
            )
            .unwrap();
        let rates = rate_report.rate.as_ref().unwrap();
        for wrench in &rates.wrenches_rate {
            assert!(
                wrench
                    .force
                    .iter()
                    .chain(&wrench.torque)
                    .all(|x| x.abs() < 1e-9)
            );
        }
        let floor_moment_rate: f64 = rate_report
            .reactions
            .supports
            .iter()
            .zip(&rates.forces_rate)
            .filter(|(s, _)| s.second.is_none())
            .map(|(s, f)| s.support.contact.point[0] * f[1] - s.support.contact.point[1] * f[0])
            .sum();
        assert!((floor_moment_rate - 20.).abs() < 1e-9);
        assert!(
            rate_report
                .environment_force_rate
                .iter()
                .all(|x| x.abs() < 1e-9)
        );
        assert_eq!(bodies, before_query);
        assert_eq!(liquid, before_liquid);
        let dt = 1e-4;
        let admission = physics::liquid::SupportedWorldConfig::default();
        let report = liquid
            .step_with_supported_rigid_body_forces(
                dt,
                &mut bodies,
                &world,
                Default::default(),
                2,
                rigid_config(),
                &[ContactWrench::default(); 2],
                admission,
            )
            .unwrap();
        assert!((bodies[0].motion.position[0] - dt).abs() < 1e-12);
        // Moving top application point imposes -20t torque on the lower body.
        assert!((bodies[1].spin.unwrap().angular_momentum[2] + 10. * dt * dt).abs() < 1e-12);
        assert!(
            report
                .reaction_angular_balance_residual
                .iter()
                .all(|x| x.abs() < 1e-12)
        );
        assert!((report.environment_reaction_impulse[1] + 50. * dt).abs() < 1e-12);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
        assert!(report.max_support_error_m <= admission.max_geometry_error_m);
    }
    #[test]
    fn supported_motion_stack_holds_and_releases_under_changed_load_without_rest_clamping() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.03125; 3])]; 3],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
            .into_iter()
            .map(|(m, y)| {
                let mut b = rigid_body([0., y, 0.], [0.; 3], 0.);
                b.motion.mass = m;
                b
            })
            .collect();
        let before = bodies.clone();
        let mut impulse = 0.;
        for _ in 0..10 {
            let report = liquid
                .step_with_supported_rigid_body_forces(
                    0.1,
                    &mut bodies,
                    &world,
                    Default::default(),
                    3,
                    rigid_config(),
                    &[ContactWrench::default(); 3],
                    Default::default(),
                )
                .unwrap();
            impulse += report.environment_reaction_impulse[1];
            assert_eq!(report.rigid.world.dynamics.contacts, 0);
            assert!(report.reaction_work.abs() < 1e-10);
        }
        assert!((impulse + 100.).abs() < 1e-9);
        for (a, b) in bodies.iter().zip(before) {
            assert!((a.motion.position[1] - b.motion.position[1]).abs() < 1e-11);
            assert!(glam::DVec3::from_array(a.motion.velocity).length() < 1e-10);
        }
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                Default::default(),
                3,
                rigid_config(),
                &[
                    ContactWrench {
                        force: [0., 40., 0.],
                        torque: [0.; 3],
                    },
                    ContactWrench::default(),
                    ContactWrench::default(),
                ],
                Default::default(),
            )
            .unwrap();
        assert!((bodies[0].motion.position[1] - 0.20625).abs() < 1e-10);
        assert!((bodies[0].motion.velocity[1] - 1.).abs() < 1e-10);
        assert!((report.environment_reaction_impulse[1] + 8.).abs() < 1e-9);
    }
    #[test]
    fn supported_motion_searches_other_walls_and_rolls_back_late_interval_budget() {
        use physics::contact::ContactWrench;
        let mut scene = support_floor();
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.5, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.0625, 2., 1.],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.125; 3])]],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies = [rigid_body([0., 0.125, 0.], [5., 0., 0.], 0.)];
        bodies[0].motion.mass = 2.;
        let initial = bodies;
        let fluid_before = liquid.clone();
        let config = physics::liquid::DynamicWorldConfig {
            contact: physics::liquid::ContactConfig {
                restitution: 1.,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            liquid.step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                config,
                1,
                rigid_config(),
                &[ContactWrench::default()],
                physics::liquid::SupportedWorldConfig {
                    max_intervals: 2,
                    ..Default::default()
                }
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
        assert_eq!(bodies, initial);
        assert_eq!(liquid, fluid_before);
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                config,
                1,
                rigid_config(),
                &[ContactWrench::default()],
                Default::default(),
            )
            .unwrap();
        assert_eq!(report.rigid.world.dynamics.contacts, 1);
        assert!((bodies[0].motion.velocity[0] + 5.).abs() < 1e-9);
        assert!((bodies[0].motion.position[0] - 0.125).abs() < 1e-8);
        assert!((report.rigid.world.environment_impulse[0] - 20.).abs() < 1e-9);
        assert!((report.rigid.world.environment_impulse[1] + 2.).abs() < 1e-9);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    }
    fn support_query_config() -> physics::contact::ReactionConfig {
        physics::contact::ReactionConfig {
            max_sweeps: 8192,
            acceleration_tolerance: 1e-9,
            normal_velocity_tolerance: 1e-10,
        }
    }
    fn support_box(half: [f64; 3]) -> crate::convex::AffineBox {
        crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: std::array::from_fn(|k| {
                [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z][k] * half[k]
            }),
        }
    }
    fn support_floor() -> SceneGraph {
        let mut scene = SceneGraph::new(2);
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0., -0.125, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [4., 0.125, 4.],
                },
            )
            .unwrap();
        scene
    }
    #[test]
    fn scene_support_query_uses_authored_owners_gravity_and_all_corner_branches_read_only() {
        use physics::contact::ContactWrench;
        let mut scene = SceneGraph::new(8);
        let wall = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [1.; 3],
                },
            )
            .unwrap();
        let body = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(2., 2., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::BoxCollider {
                    half_extents: [1.; 3],
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidBody {
                    mass_kg: 2.,
                    initial_velocity_m_s: [0.; 3],
                },
            )
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidMassDistribution {
                    parts: vec![crate::LiquidMassPart {
                        mass_kg: 2.,
                        center_m: [0.; 3],
                        half_edges_m: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    }],
                },
            )
            .unwrap();
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [-10., -15., 0.],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let before = runtime.clone();
        let pose = scene.local(body).unwrap();
        let torque = ContactWrench {
            force: [0.; 3],
            torque: [0., 0., 10.],
        };
        let report = runtime
            .support_reactions(
                &scene,
                &[(body, torque)],
                Default::default(),
                support_query_config(),
            )
            .unwrap();
        assert_eq!(report.owners, vec![body]);
        assert_eq!(report.reactions.supports.len(), 4);
        assert!(
            report
                .reactions
                .supports
                .iter()
                .any(|s| s.support.contact.normal == [1., 0., 0.])
        );
        assert!(
            report
                .reactions
                .supports
                .iter()
                .any(|s| s.support.contact.normal == [0., 1., 0.])
        );
        let reaction = report.reactions.reaction.unwrap();
        assert!(reaction.acceleration_residual <= 1e-9);
        let wrench = reaction.wrenches[0];
        assert!((wrench.force[0] - 20.).abs() < 1e-7);
        assert!((wrench.force[1] - 30.).abs() < 1e-7);
        assert!((wrench.torque[2] + 10.).abs() < 1e-7);
        assert_eq!(runtime, before);
        assert_eq!(scene.local(body).unwrap(), pose);
        assert!(
            runtime
                .support_reactions(
                    &scene,
                    &[(body, torque), (body, torque)],
                    Default::default(),
                    support_query_config()
                )
                .is_err()
        );
        assert!(
            runtime
                .support_reactions(
                    &scene,
                    &[(wall, torque)],
                    Default::default(),
                    support_query_config()
                )
                .is_err()
        );
        scene.set_active(body, false).unwrap();
        assert!(
            runtime
                .support_reactions(
                    &scene,
                    &[(body, torque)],
                    Default::default(),
                    support_query_config()
                )
                .is_err()
        );
        assert_eq!(runtime, before);
        scene.set_active(body, true).unwrap();
        scene
            .component_mut::<crate::BoxCollider>(body)
            .unwrap()
            .unwrap()
            .half_extents[0] = 0.5;
        assert!(
            runtime
                .support_reactions(&scene, &[], Default::default(), support_query_config())
                .is_err()
        );
        assert_eq!(runtime, before);
    }
    #[test]
    fn scene_support_network_discovers_all_stack_patches_and_load_paths() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.03125; 3])]; 3],
        };
        let bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
            .into_iter()
            .map(|(mass, y)| {
                let mut body = rigid_body([0., y, 0.], [0.; 3], 0.);
                body.motion.mass = mass;
                body
            })
            .collect();
        for order in [[0, 1, 2], [2, 0, 1]] {
            let input: Vec<_> = order.map(|i| bodies[i]).into_iter().collect();
            let before = input.clone();
            let external: Vec<_> = input
                .iter()
                .map(|b| ContactWrench {
                    force: [0., -10. * b.motion.mass, 0.],
                    torque: [0.; 3],
                })
                .collect();
            let report = physics::liquid::resolve_rigid_world_reactions(
                &input,
                &world,
                &external,
                Default::default(),
                support_query_config(),
            )
            .unwrap();
            assert_eq!(report.queries, 6);
            assert_eq!(report.supports.len(), 12);
            assert_eq!(input, before);
            let reaction = report.reaction.unwrap();
            assert!(reaction.acceleration_residual <= 1e-9);
            assert_eq!(reaction.instantaneous_power, 0.);
            assert!((report.environment_force[1] + 100.).abs() < 1e-7);
            for (wrench, load) in reaction.wrenches.iter().zip(&external) {
                for k in 0..3 {
                    assert!((wrench.force[k] + load.force[k]).abs() < 1e-7);
                    assert!((wrench.torque[k] + load.torque[k]).abs() < 1e-7);
                }
            }
            let strength = |mass: f64| -> f64 {
                report
                    .supports
                    .iter()
                    .zip(&reaction.forces)
                    .filter(|(s, _)| {
                        input[s.first].motion.mass == mass
                            && s.second.is_some()
                            && input[s.second.unwrap()].motion.mass > mass
                    })
                    .map(|(_, f)| f[1])
                    .sum()
            };
            if order == [0, 1, 2] {
                assert!((strength(2.) - 20.).abs() < 1e-7);
                assert!((strength(3.) - 50.).abs() < 1e-7);
            }
        }
    }
    #[test]
    fn scene_support_network_is_covariant_under_a_proper_world_frame_permutation() {
        use physics::contact::ContactWrench;
        let q = glam::DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5);
        let mut scene = support_floor();
        let floor = scene.components::<crate::BoxCollider>().next().unwrap().0;
        let mut pose = scene.local(floor).unwrap();
        pose.translation = (q * pose.translation.as_dvec3()).as_vec3();
        pose.rotation = q.as_quat();
        scene.set_local(floor, pose).unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.03125; 3])]; 3],
        };
        let bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
            .into_iter()
            .map(|(mass, y)| {
                let mut b = rigid_body((q * glam::DVec3::Y * y).to_array(), [0.; 3], 0.);
                b.motion.mass = mass;
                b.spin.as_mut().unwrap().orientation = q.to_array();
                b
            })
            .collect();
        let external: Vec<_> = bodies
            .iter()
            .map(|b| ContactWrench {
                force: (q * glam::DVec3::Y * (-10. * b.motion.mass)).to_array(),
                torque: [0.; 3],
            })
            .collect();
        let report = physics::liquid::resolve_rigid_world_reactions(
            &bodies,
            &world,
            &external,
            Default::default(),
            support_query_config(),
        )
        .unwrap();
        assert_eq!(report.supports.len(), 12);
        assert!(
            (glam::DVec3::from_array(report.environment_force) + q * glam::DVec3::Y * 100.)
                .length()
                < 1e-7
        );
        for (w, load) in report.reaction.unwrap().wrenches.iter().zip(external) {
            assert!(
                (glam::DVec3::from_array(w.force) + glam::DVec3::from_array(load.force)).length()
                    < 1e-7
            );
            assert!(glam::DVec3::from_array(w.torque).length() < 1e-7);
        }
    }
    #[test]
    fn scene_support_network_transmits_beam_force_and_torque_to_finite_supports() {
        use physics::contact::ContactWrench;
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![
                vec![support_box([1.25, 0.125, 0.25])],
                vec![support_box([0.125, 0.125, 0.25])],
                vec![support_box([0.125, 0.125, 0.25])],
            ],
        };
        let mut bodies = [
            rigid_body([0., 0.375, 0.], [0.; 3], 0.),
            rigid_body([-1., 0.125, 0.], [0.; 3], 0.),
            rigid_body([1., 0.125, 0.], [0.; 3], 0.),
        ];
        for (b, m) in bodies.iter_mut().zip([2., 3., 5.]) {
            b.motion.mass = m;
        }
        let before = bodies;
        let external = [
            ContactWrench {
                force: [0., -20., 0.],
                torque: [0., 0., 6.],
            },
            ContactWrench {
                force: [0., -30., 0.],
                torque: [0.; 3],
            },
            ContactWrench {
                force: [0., -50., 0.],
                torque: [0.; 3],
            },
        ];
        let report = physics::liquid::resolve_rigid_world_reactions(
            &bodies,
            &world,
            &external,
            Default::default(),
            support_query_config(),
        )
        .unwrap();
        assert_eq!(report.supports.len(), 16);
        let reaction = report.reaction.unwrap();
        assert_eq!(bodies, before);
        assert!((report.environment_force[1] + 100.).abs() < 1e-7);
        for (wrench, load) in reaction.wrenches.iter().zip(&external) {
            for k in 0..3 {
                assert!((wrench.force[k] + load.force[k]).abs() < 1e-7);
                assert!((wrench.torque[k] + load.torque[k]).abs() < 1e-7);
            }
        }
        let mut ground_moment = [0.; 3];
        for (entry, force) in report.supports.iter().zip(&reaction.forces) {
            if entry.second.is_none() {
                let moment = glam::DVec3::from_array(entry.support.contact.point)
                    .cross(-glam::DVec3::from_array(*force));
                for k in 0..3 {
                    ground_moment[k] += moment[k];
                }
            }
        }
        let applied_moment: glam::DVec3 = bodies
            .iter()
            .zip(&external)
            .map(|(body, load)| {
                glam::DVec3::from_array(body.motion.position)
                    .cross(glam::DVec3::from_array(load.force))
                    + glam::DVec3::from_array(load.torque)
            })
            .sum();
        assert_eq!(applied_moment.z, -14.);
        assert!((glam::DVec3::from_array(ground_moment) - applied_moment).length() < 1e-7);
    }
    #[test]
    fn scene_support_network_rejects_overlap_and_late_budgets_without_state_writes() {
        use physics::contact::ContactWrench;
        use physics::liquid::{DynamicWorldConfig, Error};
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.125; 3])]; 2],
        };
        let bodies = [
            rigid_body([0., 0.375, 0.], [0.; 3], 0.),
            rigid_body([0., 0.125, 0.], [0.; 3], 0.),
        ];
        let before = bodies;
        let external = [ContactWrench {
            force: [0., -10., 0.],
            torque: [0.; 3],
        }; 2];
        for limits in [
            DynamicWorldConfig {
                max_queries: 2,
                ..Default::default()
            },
            DynamicWorldConfig {
                max_contacts: 7,
                ..Default::default()
            },
        ] {
            assert_eq!(
                physics::liquid::resolve_rigid_world_reactions(
                    &bodies,
                    &world,
                    &external,
                    limits,
                    support_query_config()
                ),
                Err(Error::CollisionBudget)
            );
            assert_eq!(bodies, before);
        }
        let mut overlapping = bodies;
        overlapping[1].motion.position[1] -= 0.01;
        let bad_before = overlapping;
        assert_eq!(
            physics::liquid::resolve_rigid_world_reactions(
                &overlapping,
                &world,
                &external,
                Default::default(),
                support_query_config()
            ),
            Err(Error::InitialOverlap)
        );
        assert_eq!(overlapping, bad_before);
        let mut separated = bodies;
        separated[0].motion.position[1] += 0.01;
        separated[1].motion.position[1] += 0.01;
        let report = physics::liquid::resolve_rigid_world_reactions(
            &separated,
            &world,
            &external,
            Default::default(),
            support_query_config(),
        )
        .unwrap();
        assert_eq!(report.supports.len(), 4);
        assert_eq!(report.environment_force, [0.; 3]);
        let mut outgoing = bodies;
        outgoing[0].motion.velocity[1] = 1.;
        let report = physics::liquid::resolve_rigid_world_reactions(
            &outgoing,
            &world,
            &external,
            Default::default(),
            support_query_config(),
        )
        .unwrap();
        assert!((report.environment_force[1] + 10.).abs() < 1e-7);
    }
    fn rigid_config() -> physics::spin_path::Config {
        physics::spin_path::Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        }
    }
    fn rigid_body(position: [f64; 3], velocity: [f64; 3], z: f64) -> physics::contact::ContactBody {
        physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position,
                velocity,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., z],
                inertia: [1.; 3],
            }),
        }
    }
    fn rigid_liquid(particles: Vec<physics::liquid::Particle>) -> Liquid {
        Liquid::new(
            particles,
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                max_particles: 8,
                ..Config::default()
            },
        )
        .unwrap()
    }
    fn elastic_rigid() -> physics::liquid::DynamicWorldConfig {
        physics::liquid::DynamicWorldConfig {
            contact: physics::liquid::ContactConfig {
                restitution: 1.,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    fn rigid_shapes() -> [crate::convex::AffineBox; 2] {
        [
            crate::convex::AffineBox {
                center: glam::DVec3::ZERO,
                edges: [
                    glam::DVec3::X * 0.04,
                    glam::DVec3::Y * 0.02,
                    glam::DVec3::Z * 0.02,
                ],
            },
            crate::convex::AffineBox {
                center: glam::DVec3::X * 0.04,
                edges: [
                    glam::DVec3::X * 0.04,
                    glam::DVec3::Y * 2.,
                    glam::DVec3::Z * 0.02,
                ],
            },
        ]
    }
    #[test]
    fn scene_geometry_and_shared_core_advance_real_off_center_rigid_collision() {
        let scene = SceneGraph::new(1);
        let shapes = rigid_shapes();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: shapes.map(|s| vec![s]).to_vec(),
        };
        let mut liquid = rigid_liquid(Vec::new());
        let mut bodies = [
            rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.),
            rigid_body([0.; 3], [0.; 3], 0.),
        ];
        let before = bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>();
        let report = liquid
            .step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &world,
                elastic_rigid(),
                2,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!((bodies[0].motion.velocity[0] - 1.).abs() < 1e-11);
        assert!((bodies[1].motion.velocity[0] - 2.).abs() < 1e-11);
        assert!((bodies[1].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
        assert!((bodies[1].spin.unwrap().orientation[2] - (-0.08_f64).sin()).abs() < 1e-11);
        assert!(
            (bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>()
                + report.dynamics.dissipated_energy
                - before)
                .abs()
                < 1e-11
        );
    }
    #[test]
    fn scene_geometry_particle_recoil_rotates_the_body_and_restores_on_budget_failure() {
        let scene = SceneGraph::new(1);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[1]]],
        };
        let mut liquid = rigid_liquid(vec![physics::liquid::Particle {
            position: [-0.07, 1., 0.],
            velocity: [3., 0., 0.],
            mass: 1.,
            material: 0,
        }]);
        let mut bodies = [rigid_body([0.; 3], [0.; 3], 0.)];
        let originals = (liquid.clone(), bodies);
        let mut limited = elastic_rigid();
        limited.contact.max_candidates = 0;
        assert!(
            liquid
                .step_with_rigid_body_world(0.03, &mut bodies, &world, limited, 1, rigid_config())
                .is_err()
        );
        assert_eq!((liquid.clone(), bodies), originals);
        let report = liquid
            .step_with_rigid_body_world(
                0.03,
                &mut bodies,
                &world,
                elastic_rigid(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!(
            (liquid.particles()[0].velocity[0] + bodies[0].motion.velocity[0] - 3.).abs() < 1e-11
        );
        assert!((bodies[0].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
        assert_ne!(bodies[0].spin.unwrap().orientation, [0., 0., 0., 1.]);
    }
    #[test]
    fn authored_static_wall_reflects_a_spinning_body_with_boundary_impulse_balance() {
        let mut scene = SceneGraph::new(1);
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.04, 0., 0.),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.04, 2., 0.02],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[0]]],
        };
        let mut liquid = rigid_liquid(Vec::new());
        let mut bodies = [rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.3)];
        let energy = bodies[0].energy().unwrap();
        let report = liquid
            .step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &world,
                elastic_rigid(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!(bodies[0].motion.velocity[0] < 0.);
        assert!((bodies[0].motion.velocity[0] + report.environment_impulse[0] - 3.).abs() < 1e-11);
        assert!(
            (bodies[0].energy().unwrap() + report.dynamics.dissipated_energy - energy).abs()
                < 1e-11
        );
        assert!((bodies[0].spin.unwrap().angular_momentum[2] - 0.3).abs() > 1e-3);
    }
    #[test]
    fn moving_body_contact_witness_is_transported_to_contact_time() {
        use physics::liquid::LiquidBodyWorld;
        let scene = SceneGraph::new(1);
        let shape = crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * 0.5,
                glam::DVec3::Y * 0.5,
                glam::DVec3::Z * 0.5,
            ],
        };
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![shape], vec![shape]],
        };
        let a = physics::liquid::TranslatingBody {
            mass: 1.,
            position: [-3., 0., 0.],
            velocity: [4., 0., 0.],
        };
        let b = physics::liquid::TranslatingBody {
            mass: 1.,
            position: [0.; 3],
            velocity: [1., 0., 0.],
        };
        let hit = world.sweep_body_pair_contact(0, &a, 1, &b, 1., 10).unwrap();
        let physics::liquid::GeometryHit::Contact { fraction, .. } = hit.geometry else {
            panic!("contact expected");
        };
        assert!((fraction - 2. / 3.).abs() < 1e-12);
        let witness = hit.witness.unwrap();
        assert!((witness.point[0] - (-0.5 + fraction)).abs() < witness.tolerance_m + 1e-12);
        let particle = physics::liquid::Particle {
            position: [-3., 0., 0.],
            velocity: [4., 0., 0.],
            mass: 1.,
            material: 0,
        };
        let hit = world
            .sweep_particle_body_contact(&particle, 0.5, 1, &b, 1., 10)
            .unwrap();
        assert_eq!(
            hit.geometry,
            world
                .sweep_particle_body(&particle, 0.5, 1, &b, 1., 10)
                .unwrap()
        );
        assert!((hit.witness.unwrap().point[0] - witness.point[0]).abs() < 1e-12);
    }

    fn fixture(max_particles: usize) -> (SceneGraph, NodeId, NodeId, SceneLiquidRuntime) {
        let mut scene = SceneGraph::new(2);
        let mut nodes = Vec::new();
        for x in [0., 10.] {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(x, 0., 0.),
                        ..Transform::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    LiquidSource {
                        pulses: vec![LiquidPulse {
                            start_s: 0.,
                            duration_s: 1.,
                            volume_m3: 0.001,
                            speed_m_s: 2.,
                        }],
                        density_kg_m3: 1000.,
                        particle_volume_m3: 0.001,
                        nozzle_radius_m: 0.,
                        direction: [1., 0., 0.],
                        material_asset: "water".into(),
                    },
                )
                .unwrap();
            nodes.push(node);
        }
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                max_particles,
                ..Config::default()
            },
            2,
        )
        .unwrap();
        (scene, nodes[0], nodes[1], runtime)
    }

    #[test]
    fn shared_world_emits_at_authored_origins_and_pauses_inactive_sources() {
        let (mut scene, first, second, mut runtime) = fixture(10);
        let report = runtime.tick(&scene, 0.001, None).unwrap();
        assert_eq!(report.emissions.len(), 2);
        let particles = runtime.liquid().particles();
        assert_eq!(particles.len(), 2);
        assert!((runtime.liquid().mass() - 0.002).abs() < 1e-15);
        for (p, x) in particles.iter().zip([0., 10.]) {
            assert!((p.position[0] - x - 0.002).abs() < 1e-12);
            assert_eq!(p.velocity, [2., 0., 0.]);
        }
        scene.set_active(second, false).unwrap();
        runtime.tick(&scene, 0.001, None).unwrap();
        assert_eq!(runtime.source_elapsed(first), Some(0.002));
        assert_eq!(runtime.source_elapsed(second), Some(0.001));
        assert_eq!(runtime.liquid().particles().len(), 3);
        assert!((runtime.liquid().particles()[1].position[0] - 10.004).abs() < 1e-12);
    }

    #[test]
    fn moving_nozzle_transfers_world_velocity_and_admitted_preflight_succeeds() {
        let (mut scene, first, second, mut runtime) = fixture(10);
        crate::validate_game_descriptors_with_liquid_runtime(&scene, 2, &runtime).unwrap();
        assert!(crate::validate_game_descriptors(&scene, 2).is_err());
        scene.set_active(second, false).unwrap();
        let mut transform = scene.local(first).unwrap();
        transform.translation.x = 0.01;
        scene.set_local(first, transform).unwrap();
        runtime.tick(&scene, 0.01, None).unwrap();
        let p = runtime.liquid().particles()[0];
        let nozzle_velocity = f64::from(transform.translation.x) / 0.01;
        assert!((p.velocity[0] - 2. - nozzle_velocity).abs() < 1e-12);
        assert!(
            (p.position[0] - f64::from(transform.translation.x) - p.velocity[0] * 0.01).abs()
                < 1e-12
        );
    }

    #[test]
    fn rotated_wall_uses_exact_geometry_and_oblique_normal() {
        let (mut scene, first, second, _) = fixture(10);
        scene.set_active(second, false).unwrap();
        scene.remove_component::<LiquidSource>(second).unwrap();
        scene
            .set_local(
                second,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        scene.set_active(second, true).unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Config::default()
            },
            1,
        )
        .unwrap();
        runtime.tick(&scene, 0.05, None).unwrap();
        let p = runtime.liquid().particles()[0];
        assert!((p.velocity[0] - 1.).abs() < 1e-6);
        assert!((p.velocity[1] + 1.).abs() < 1e-6);
        assert!(p.position[1] < 0.);
        assert_eq!(runtime.source_elapsed(first), Some(0.05));
        assert!((runtime.liquid().mass() - 0.05).abs() < 1e-12);
        let before = runtime.clone();
        scene
            .component_mut::<crate::BoxCollider>(second)
            .unwrap()
            .unwrap()
            .half_extents = [1., 1., 1.];
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
    }

    #[test]
    fn finite_translating_body_uses_scene_affine_template_and_recoils() {
        use physics::liquid::{DynamicWorldConfig, Particle, TranslatingBody};
        let mut scene = SceneGraph::new(1);
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        let original = scene.local(node).unwrap();
        let geometry = SceneGeometry(crate::static_world(&scene).unwrap());
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [0.; 3],
                velocity: [2., 0., 0.],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
        )
        .unwrap();
        let mut body = TranslatingBody {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 3.,
        };
        let report = liquid
            .step_with_dynamic_geometry(0.05, &mut body, &geometry, DynamicWorldConfig::default())
            .unwrap();
        assert_eq!(report.contacts, 1);
        let p = liquid.particles()[0];
        assert!((p.velocity[0] - 1.25).abs() < 1e-6);
        assert!((p.velocity[1] + 0.75).abs() < 1e-6);
        assert!((body.velocity[0] - 0.25).abs() < 1e-6);
        assert!((body.velocity[1] - 0.25).abs() < 1e-6);
        for a in 0..3 {
            assert!((p.velocity[a] + 3. * body.velocity[a] - [2., 0., 0.][a]).abs() < 1e-12);
        }
        assert!((report.dissipated_energy - 0.75).abs() < 1e-6);
        assert!(body.position[0] > 0. && body.position[1] > 0.);
        assert_eq!(scene.local(node).unwrap(), original);
    }

    #[test]
    fn authored_body_recoil_publishes_pose_and_failure_preserves_all_owners() {
        let (mut scene, first, second, _) = fixture(10);
        scene.remove_component::<LiquidSource>(second).unwrap();
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .pulses[0]
            .duration_s = 0.05;
        scene
            .set_local(
                second,
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::BoxCollider {
                    half_extents: [0.01, 2., 2.],
                },
            )
            .unwrap();
        scene
            .insert_component(
                second,
                crate::LiquidBody {
                    mass_kg: 3.,
                    initial_velocity_m_s: [0.; 3],
                },
            )
            .unwrap();
        let initial = scene.local(second).unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            1,
        )
        .unwrap();
        let report = runtime.tick_and_publish(&mut scene, 0.05, None).unwrap();
        assert_eq!(report.dynamics.unwrap().dynamics.contacts, 1);
        let state = runtime.body_state().unwrap().1;
        assert!((state.velocity[0] - 0.25).abs() < 1e-6);
        assert!((state.velocity[1] - 0.25).abs() < 1e-6);
        let pose = scene.local(second).unwrap();
        assert!(pose.translation.x > initial.translation.x && pose.translation.y > 0.);
        assert_eq!(pose.rotation, initial.rotation);
        assert_eq!(pose.scale, initial.scale);
        runtime.validate_bindings(&scene).unwrap();
        let before = runtime.clone();
        let pose_before = scene.local(second).unwrap();
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .particle_volume_m3 = 1e-10;
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert_eq!(scene.local(second).unwrap(), pose_before);
    }

    #[test]
    fn body_publication_overflow_and_initial_overlap_are_admitted_atomically() {
        let mut scene = SceneGraph::new(2);
        let body = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(body, crate::BoxCollider::default())
            .unwrap();
        scene
            .insert_component(
                body,
                crate::LiquidBody {
                    mass_kg: 3.,
                    initial_velocity_m_s: [1e100, 0., 0.],
                },
            )
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            1,
        )
        .unwrap();
        let before = runtime.clone();
        let pose = scene.local(body).unwrap();
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert_eq!(scene.local(body).unwrap(), pose);
        let wall = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(wall, crate::BoxCollider::default())
            .unwrap();
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                1
            )
            .is_err()
        );
    }

    #[test]
    fn late_emission_and_physics_failures_roll_back_every_owner() {
        let (scene, _, _, mut runtime) = fixture(1);
        let before = runtime.clone();
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        let (scene, _, _, mut runtime) = fixture(10);
        let before = runtime.clone();
        assert!(
            runtime
                .tick(
                    &scene,
                    0.001,
                    Some(Container {
                        min: [1.; 3],
                        max: [0.; 3],
                        restitution: 0.,
                        friction: 0.
                    })
                )
                .is_err()
        );
        assert_eq!(runtime, before);
    }

    #[test]
    fn foreign_scene_edits_unresolved_material_and_budget_are_rejected() {
        let (mut scene, first, _, mut runtime) = fixture(10);
        let before = runtime.clone();
        assert!(runtime.tick(&SceneGraph::new(2), 0.001, None).is_err());
        scene
            .component_mut::<LiquidSource>(first)
            .unwrap()
            .unwrap()
            .density_kg_m3 = 500.;
        assert!(runtime.tick(&scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("oil".into(), Material::OIL)],
                Config::default(),
                2
            )
            .is_err()
        );
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("water".into(), Material::WATER)],
                Config::default(),
                1
            )
            .is_err()
        );
    }
    fn body_pair_fixture() -> (SceneGraph, Vec<NodeId>, SceneLiquidRuntime) {
        let mut scene = SceneGraph::new(8);
        let mut nodes = Vec::new();
        for (x, velocity) in [(-0.2, 3.), (0., 0.), (2., 7.)] {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(x, 0., 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    crate::BoxCollider {
                        half_extents: [0.05; 3],
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    node,
                    crate::LiquidBody {
                        mass_kg: 1.,
                        initial_velocity_m_s: [velocity, 0., 0.],
                    },
                )
                .unwrap();
            nodes.push(node);
        }
        scene.set_active(nodes[2], false).unwrap();
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        (scene, nodes, runtime)
    }

    #[test]
    fn multiple_scene_bodies_exchange_impulse_publish_and_pause_inactive_owner() {
        let (mut scene, nodes, mut runtime) = body_pair_fixture();
        let paused = scene.local(nodes[2]).unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let states: Vec<_> = runtime.body_states().collect();
        assert_eq!(states.len(), 3);
        for (_, state) in &states[..2] {
            assert!((state.velocity[0] - 1.5).abs() < 1e-12);
        }
        assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
        assert_eq!(states[2].1.position, [2., 0., 0.]);
        assert_eq!(states[2].1.velocity, [7., 0., 0.]);
        assert_eq!(scene.local(nodes[2]).unwrap(), paused);
        for (node, state) in &states[..2] {
            assert_eq!(
                scene.local(*node).unwrap().translation,
                glam::DVec3::from_array(state.position).as_vec3()
            );
        }
        runtime.validate_bindings(&scene).unwrap();
        runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
    }

    #[test]
    fn later_body_publication_failure_preserves_all_scene_poses() {
        let (mut scene, nodes, mut runtime) = body_pair_fixture();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        runtime.body[0].state.motion.position[0] += 1.;
        runtime.body[1].state.motion.position[0] = 1e100;
        let before = runtime.clone();
        assert!(runtime.publish_body_pose(&mut scene).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
    }
    #[test]
    fn child_only_compound_hits_another_body_and_shape_edits_reject_atomically() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let states: Vec<_> = runtime.body_states().collect();
        assert!((states[0].1.velocity[0] - 1.5).abs() < 1e-12);
        assert!((states[1].1.velocity[0] - 1.5).abs() < 1e-12);
        assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
        runtime.validate_bindings(&scene).unwrap();
        let before = runtime.clone();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        scene
            .set_local(
                child,
                Transform {
                    translation: glam::Vec3::new(0.2, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(runtime.tick_and_publish(&mut scene, 0.01, None).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
    }

    #[test]
    fn compound_gap_is_not_filled_by_a_bounding_box() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        for y in [-1., 1.] {
            let child = scene
                .spawn(
                    Some(nodes[0]),
                    Transform {
                        translation: glam::Vec3::new(0., y, 0.),
                        rotation: glam::Quat::from_rotation_z(0.4),
                        scale: glam::Vec3::new(1., 2., 1.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    child,
                    crate::BoxCollider {
                        half_extents: [0.05; 3],
                    },
                )
                .unwrap();
        }
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 0);
        let states: Vec<_> = runtime.body_states().collect();
        assert_eq!(states[0].1.velocity, [3., 0., 0.]);
        assert_eq!(states[1].1.velocity, [0.; 3]);
        assert!((states[0].1.position[0] - 0.1).abs() < 1e-7);
    }
    #[test]
    fn compound_child_strikes_static_wall_and_balances_external_impulse() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene.set_active(nodes[1], false).unwrap();
        scene
            .remove_component::<crate::BoxCollider>(nodes[0])
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.2, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.1, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert_eq!(runtime.body_state().unwrap().1.velocity, [0.; 3]);
        assert!((report.environment_impulse[0] - 3.).abs() < 1e-12);
        assert!((report.dynamics.dissipated_energy - 4.5).abs() < 1e-12);
        runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
    }

    #[test]
    fn root_relative_compound_template_preserves_small_offsets_far_from_origin() {
        let (mut scene, nodes, _) = body_pair_fixture();
        scene
            .set_local(
                nodes[0],
                Transform {
                    translation: glam::Vec3::new(1e5, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0.03, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.01; 3],
                },
            )
            .unwrap();
        let runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0,
        )
        .unwrap();
        let body = &runtime.body[0];
        let collider = body.colliders.iter().find(|c| c.node == child).unwrap();
        let template = collider_template(body, collider).unwrap();
        assert_eq!(template.center.x, f64::from(0.03_f32));
    }
    #[test]
    fn compound_candidates_are_bounded_and_invalid_extents_reject() {
        use physics::liquid::LiquidBodyWorld;
        let (mut scene, nodes, runtime) = body_pair_fixture();
        let template = collider_template(&runtime.body[0], &runtime.body[0].colliders[0]).unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![template; 2], vec![template; 2]],
        };
        assert_eq!(
            world.sweep_body_pair(
                0,
                &runtime.body_states().next().unwrap().1,
                1,
                &runtime.body_states().nth(1).unwrap().1,
                0.1,
                3
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
        assert_eq!(
            world.sweep_body_environment(0, &runtime.body_states().next().unwrap().1, 0.1, 1),
            Err(physics::liquid::Error::CollisionBudget)
        );
        scene
            .component_mut::<crate::BoxCollider>(nodes[0])
            .unwrap()
            .unwrap()
            .half_extents = [-0.05; 3];
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                0
            )
            .is_err()
        );
    }
    #[test]
    fn liquid_impulse_recoils_child_only_compound_owner() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                root,
                crate::LiquidBody {
                    mass_kg: 1.,
                    initial_velocity_m_s: [0.; 3],
                },
            )
            .unwrap();
        let child = scene
            .spawn(
                Some(root),
                Transform {
                    translation: glam::Vec3::new(0.1, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.02; 3],
                },
            )
            .unwrap();
        let config = Config {
            gravity: [0.; 3],
            particle_radius: 0.01,
            ..Default::default()
        };
        let mut runtime =
            SceneLiquidRuntime::new(&scene, vec![("water".into(), Material::WATER)], config, 0)
                .unwrap();
        runtime.liquid = Liquid::new(
            vec![physics::liquid::Particle {
                position: [0.; 3],
                velocity: [3., 0., 0.],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            config,
        )
        .unwrap();
        let report = runtime
            .tick_and_publish(&mut scene, 0.05, None)
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let body = runtime.body_state().unwrap().1;
        let particle = &runtime.liquid.particles()[0];
        assert!((particle.velocity[0] - 1.5).abs() < 1e-12);
        assert!((body.velocity[0] - 1.5).abs() < 1e-12);
        assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
        assert_eq!(report.environment_impulse, [0.; 3]);
        assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
    }
    #[test]
    fn common_gravity_preserves_particle_body_relative_motion_across_sph_substeps() {
        let scene = SceneGraph::new(1);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[0]]],
        };
        let particle = physics::liquid::Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 2.,
            material: 0,
        };
        let mut liquid = Liquid::new(
            vec![particle],
            vec![Material::WATER],
            Config {
                gravity: [0., -2., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut initial = rigid_body([10., 0., 0.], [0.; 3], 0.);
        initial.spin = None;
        initial.motion.mass = 3.;
        let mut bodies = [initial];
        let report = liquid
            .step_with_rigid_body_forces(
                0.01,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[physics::contact::ContactWrench::default()],
            )
            .unwrap();
        assert!(report.world.dynamics.fluid.substeps > 1);
        assert_eq!(report.world.dynamics.contacts, 0);
        let p = liquid.particles()[0];
        for actual in [p.position[1], bodies[0].motion.position[1]] {
            assert!((actual + 0.0001).abs() < 1e-14);
        }
        for actual in [p.velocity[1], bodies[0].motion.velocity[1]] {
            assert!((actual + 0.02).abs() < 1e-14);
        }
        assert!((p.position[1] - bodies[0].motion.position[1]).abs() < 1e-14);
        assert!((report.external_work - 0.0006).abs() < 1e-14);
    }

    #[test]
    fn common_acceleration_event_transports_the_real_world_witness_quadratically() {
        use physics::liquid::{GeometryHit, LiquidBodyWorld};
        let scene = SceneGraph::new(1);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[1]]],
        };
        let p = physics::liquid::Particle {
            position: [-0.1, 0., 0.],
            velocity: [3., 0., 0.],
            mass: 1.,
            material: 0,
        };
        let particle = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: p.mass,
                position: p.position,
                velocity: p.velocity,
            },
            spin: None,
        }
        .prepare_motion([0., -2., 0.], [0.; 3], 0.04, rigid_config())
        .unwrap();
        let mut initial = rigid_body([0.; 3], [0.; 3], 0.);
        initial.spin = None;
        initial.motion.mass = 3.;
        let body = initial
            .prepare_motion([0., -6., 0.], [0.; 3], 0.04, rigid_config())
            .unwrap();
        let event = world
            .sweep_particle_motion_body_event(&p, 0.01, &particle, 0, &body, 1)
            .unwrap();
        let GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
            panic!("contact required")
        };
        let time = fraction * 0.04;
        assert!((time - 0.03).abs() < 1e-14);
        assert_eq!(normal, [-1., 0., 0.]);
        assert!((event.contact.witness.unwrap().point[1] + time * time).abs() < 1e-14);
    }

    #[test]
    fn unresolved_resting_gravity_support_preserves_complete_public_state() {
        let mut scene = SceneGraph::new(2);
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.03125, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.03125, 1., 0.03125],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![crate::convex::AffineBox {
                center: glam::DVec3::ZERO,
                edges: [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z].map(|e| e * 0.03125),
            }]],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [20., 0., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let initial_fluid = liquid.clone();
        let mut bodies = [rigid_body([-0.03125, 0., 0.], [0.; 3], 0.)];
        let before = bodies;
        assert_eq!(
            liquid.step_with_rigid_body_forces(
                0.01,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[physics::contact::ContactWrench::default()]
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
        assert_eq!(before, bodies);
        assert_eq!(liquid.particles(), initial_fluid.particles());
    }

    #[test]
    fn accelerated_nonspinning_body_uses_real_parabolic_wall_query_and_remainder() {
        let mut scene = SceneGraph::new(2);
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.04, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.04, 2., 0.02],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[0]]],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [20., 0., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut body = rigid_body([-0.1, 0., 0.], [0.; 3], 0.);
        body.spin = None;
        let initial = body.energy().unwrap();
        // Keep the real publication array so endpoint comparisons prove the
        // actual geometry/core path rather than a separate prepared reference.
        let mut bodies = [body];
        let report = liquid
            .step_with_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                elastic_rigid(),
                1,
                rigid_config(),
                &[physics::contact::ContactWrench::default()],
            )
            .unwrap();
        let impact = (2. * 0.06 / 20_f64).sqrt();
        let remainder = 0.1 - impact;
        let expected_x = -0.04 - 20. * impact * remainder + 10. * remainder * remainder;
        let expected_v = -20. * impact + 20. * remainder;
        assert_eq!(report.world.dynamics.contacts, 1);
        assert!((bodies[0].motion.position[0] - expected_x).abs() < 1e-10);
        assert!((bodies[0].motion.velocity[0] - expected_v).abs() < 1e-10);
        assert!((report.world.environment_impulse[0] - 40. * impact).abs() < 1e-10);
        assert_eq!(report.world.dynamics.dissipated_energy, 0.);
        assert!(
            (bodies[0].energy().unwrap()
                - initial
                - report.external_work
                - report.integration_energy_residual)
                .abs()
                < 1e-10
        );
    }

    #[test]
    fn real_simultaneous_rigid_faces_resolve_as_one_network() {
        let scene = SceneGraph::new(1);
        let shape = rigid_shapes()[0];
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![shape]; 3],
        };
        let original = [
            rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.),
            rigid_body([0.; 3], [0.; 3], 0.),
            rigid_body([0.1, 0., 0.], [-3., 0., 0.], 0.),
        ];
        for order in [[0, 1, 2], [2, 0, 1]] {
            let mut states = order.map(|i| original[i]);
            let mut liquid = rigid_liquid(Vec::new());
            {
                use physics::liquid::{GeometryHit, LiquidBodyWorld};
                let paths: Vec<_> = states
                    .iter()
                    .map(|b| {
                        b.prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
                            .unwrap()
                    })
                    .collect();
                let mut fractions = Vec::new();
                for i in 0..3 {
                    for j in i + 1..3 {
                        let event = world
                            .sweep_rigid_pair_event(i, &paths[i], j, &paths[j], 4096)
                            .unwrap();
                        assert!(event.feature.is_some() && event.contact.witness.is_some());
                        let GeometryHit::Contact { fraction, .. } = event.contact.geometry else {
                            panic!("initial approaching pair required")
                        };
                        fractions.push(fraction);
                    }
                }
                fractions.sort_by(f64::total_cmp);
                assert_eq!(fractions[0], fractions[1]);
                assert!(fractions[2] > fractions[0]);
            }
            let report = liquid
                .step_with_rigid_body_world(
                    0.02,
                    &mut states,
                    &world,
                    Default::default(),
                    3,
                    rigid_config(),
                )
                .unwrap();
            assert_eq!(report.dynamics.contacts, 2);
            assert!((report.dynamics.dissipated_energy - 9.).abs() < 1e-11);
            assert_eq!(report.environment_impulse, [0.; 3]);
            for (body, i) in states.iter().zip(order) {
                assert!(body.motion.velocity.iter().all(|v| v.abs() < 1e-12));
                assert!(
                    body.spin
                        .unwrap()
                        .angular_momentum
                        .iter()
                        .all(|v| v.abs() < 1e-12)
                );
                assert!((body.motion.position[0] - [-0.08, 0., 0.08][i]).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn compound_event_key_preserves_exact_shape_pair_and_support_source() {
        use physics::liquid::LiquidBodyWorld;
        let scene = SceneGraph::new(1);
        let shapes = rigid_shapes();
        let first_far = crate::convex::AffineBox {
            center: glam::DVec3::new(5., 10., 0.),
            ..shapes[0]
        };
        let second_far = crate::convex::AffineBox {
            center: glam::DVec3::new(10., -10., 0.),
            ..shapes[1]
        };
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![first_far, shapes[0]], vec![second_far, shapes[1]]],
        };
        let a = rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.)
            .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
            .unwrap();
        let b = rigid_body([0.; 3], [0.; 3], 0.)
            .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
            .unwrap();
        let event = world.sweep_rigid_pair_event(0, &a, 1, &b, 16).unwrap();
        assert_eq!(
            event.contact,
            world.sweep_rigid_pair_contact(0, &a, 1, &b, 16).unwrap()
        );
        let key = FeatureKey::decode(event.feature.unwrap()).unwrap();
        assert_eq!(
            (key.first, key.second, key.axis),
            (1, 1, crate::convex::AxisFeature::BodyFace(0))
        );
        let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry
        else {
            panic!("contact required")
        };
        let mut sa = a.sample(fraction * a.duration()).unwrap();
        let mut sb = b.sample(fraction * b.duration()).unwrap();
        sa.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.25];
        sb.spin.as_mut().unwrap().angular_momentum = [0., 0., -0.6];
        let points = world
            .rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                event.feature.unwrap(),
                1,
            )
            .unwrap();
        assert_eq!(points.len(), 4);
        assert!(
            points
                .iter()
                .all(|s| s.plane == physics::contact::SupportPlane::First)
        );
        let bad = FeatureKey { first: 99, ..key }.encode().unwrap();
        assert_eq!(
            world.rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                bad,
                1
            ),
            Err(physics::liquid::Error::InvalidCollision)
        );
        assert_eq!(
            world.rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                255,
                1
            ),
            Err(physics::liquid::Error::InvalidCollision)
        );
        assert_eq!(
            world.rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                event.feature.unwrap(),
                0
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
    }

    #[test]
    fn edge_event_source_rebuilds_normal_rate_from_post_impact_spin() {
        use glam::{DQuat, DVec3};
        use physics::liquid::LiquidBodyWorld;
        let h = 0.02;
        let (s, c) = 0.35_f64.sin_cos();
        let (t, d) = 0.45_f64.sin_cos();
        let first_shape = crate::convex::AffineBox {
            center: DVec3::ZERO,
            edges: [
                DVec3::Y,
                DVec3::new(c, 0., -s) * h,
                DVec3::new(s, 0., c) * h,
            ],
        };
        let second_shape = crate::convex::AffineBox {
            center: DVec3::ZERO,
            edges: [
                DVec3::Z,
                DVec3::new(d, t, 0.) * h,
                DVec3::new(-t, d, 0.) * h,
            ],
        };
        let separation = first_shape.radius(DVec3::X) + second_shape.radius(DVec3::X);
        let scene = SceneGraph::new(1);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![first_shape], vec![second_shape]],
        };
        let a = rigid_body([-separation - 0.03, 0., 0.], [3., 0., 0.], 0.4)
            .prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
            .unwrap();
        let mut initial_b = rigid_body([0.; 3], [0.; 3], 0.);
        initial_b.spin.as_mut().unwrap().angular_momentum = [-0.6, 0., 0.];
        let b = initial_b
            .prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
            .unwrap();
        let event = world.sweep_rigid_pair_event(0, &a, 1, &b, 64).unwrap();
        let key = FeatureKey::decode(event.feature.unwrap()).unwrap();
        assert_eq!(key.axis, crate::convex::AxisFeature::Edges(0, 0));
        let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry
        else {
            panic!("contact required")
        };
        let mut sa = a.sample(fraction * a.duration()).unwrap();
        let mut sb = b.sample(fraction * b.duration()).unwrap();
        let previous = world
            .rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                event.feature.unwrap(),
                1,
            )
            .unwrap();
        assert_eq!(previous.len(), 1);
        sa.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.8];
        sb.spin.as_mut().unwrap().angular_momentum = [-0.2, 0., 0.];
        let updated = world
            .rigid_pair_supports(
                0,
                &sa,
                1,
                &sb,
                event.contact.witness.unwrap(),
                normal,
                event.feature.unwrap(),
                1,
            )
            .unwrap();
        let physics::contact::SupportPlane::Rate { normal_rate } = updated[0].plane else {
            panic!("edge normal rate required")
        };
        assert_ne!(updated[0].plane, previous[0].plane);
        let ea = DQuat::from_array(sa.spin.unwrap().orientation) * first_shape.edges[0];
        let eb = DQuat::from_array(sb.spin.unwrap().orientation) * second_shape.edges[0];
        let wa = DVec3::from_array(sa.spin.unwrap().angular_velocity().unwrap());
        let wb = DVec3::from_array(sb.spin.unwrap().angular_velocity().unwrap());
        let sign = if ea.cross(eb).dot(DVec3::from_array(normal)) < 0. {
            -1.
        } else {
            1.
        };
        let rotate = |edge: DVec3, omega: DVec3, time: f64| {
            let speed = omega.length();
            let axis = omega / speed;
            let (s, c) = (speed * time).sin_cos();
            edge * c + axis.cross(edge) * s + axis * axis.dot(edge) * (1. - c)
        };
        let n = |time| rotate(ea, wa, time).cross(rotate(eb, wb, time)).normalize() * sign;
        let delta = 1e-5;
        let numeric = (n(delta) - n(-delta)) / (2. * delta);
        assert!((numeric - DVec3::from_array(normal_rate)).length() < 1e-8);
        assert!(
            DVec3::from_array(normal)
                .dot(DVec3::from_array(normal_rate))
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn environment_event_keeps_wall_identity_and_world_plane() {
        use glam::{DQuat, DVec3};
        use physics::liquid::LiquidBodyWorld;
        let mut scene = SceneGraph::new(3);
        for x in [10., 0.04] {
            let wall = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::new(x, 0., 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(
                    wall,
                    crate::BoxCollider {
                        half_extents: [0.04, 2., 0.02],
                    },
                )
                .unwrap();
        }
        let q = DQuat::from_rotation_z(0.4);
        let mut shape = rigid_shapes()[0];
        shape.edges = shape.edges.map(|e| q * e);
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![shape]],
        };
        let expected_wall = world
            .environment
            .0
            .0
            .iter()
            .position(|w| w.shape.center.x < 1.)
            .unwrap();
        let path = rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.)
            .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
            .unwrap();
        let event = world.sweep_rigid_environment_event(0, &path, 64).unwrap();
        let token = event.feature.unwrap();
        let key = FeatureKey::decode(token).unwrap();
        assert_eq!(key.second, expected_wall);
        assert_eq!(key.axis, crate::convex::AxisFeature::ObstacleFace(0));
        let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry
        else {
            panic!("contact required")
        };
        let mut body = path.sample(fraction * path.duration()).unwrap();
        body.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.8];
        let supports = world
            .rigid_environment_supports(0, &body, event.contact.witness.unwrap(), normal, token, 1)
            .unwrap();
        assert!(!supports.is_empty());
        assert!(
            supports
                .iter()
                .all(|s| s.plane == physics::contact::SupportPlane::World)
        );
        let bad = FeatureKey { second: 99, ..key }.encode().unwrap();
        assert_eq!(
            world.rigid_environment_supports(
                0,
                &body,
                event.contact.witness.unwrap(),
                normal,
                bad,
                1
            ),
            Err(physics::liquid::Error::InvalidCollision)
        );
        assert_eq!(
            world.rigid_environment_supports(
                0,
                &body,
                event.contact.witness.unwrap(),
                normal,
                token,
                0
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
        // A fixed wall's normal must not inherit the moving body's angular rate.
        let gap = physics::contact::normal_gap_acceleration(
            &body,
            None,
            supports[0],
            physics::contact::ContactWrench {
                force: [0.; 3],
                torque: [0.; 3],
            },
            None,
        )
        .unwrap();
        assert!(gap.is_finite());
        assert!(DVec3::from_array(normal).is_finite());
        // Run actual scene feature admission through the physical impact and
        // reaction solvers. Rebuild supports after impact changes angular momentum.
        let points: Vec<_> = supports.iter().map(|s| s.contact).collect();
        physics::contact::resolve_normal_manifold(
            &mut body,
            None,
            &points,
            physics::contact::ManifoldConfig {
                max_sweeps: 100,
                velocity_tolerance: 1e-11,
            },
        )
        .unwrap();
        let post = world
            .rigid_environment_supports(0, &body, event.contact.witness.unwrap(), normal, token, 1)
            .unwrap();
        let external = physics::contact::ContactWrench {
            force: [3., 0., 0.],
            torque: [0.; 3],
        };
        let snapshot = body;
        let reaction = physics::contact::resolve_normal_reactions(
            &body,
            None,
            &post,
            external,
            None,
            physics::contact::ReactionConfig {
                max_sweeps: 100,
                acceleration_tolerance: 1e-10,
                normal_velocity_tolerance: 1e-10,
            },
        )
        .unwrap();
        assert_eq!(snapshot, body);
        assert!(reaction.first_wrench.force[0] < 0.);
        assert!(reaction.second_wrench.is_none());
        assert!(reaction.normal_accelerations.iter().all(|g| *g >= -1e-10));
        assert!(reaction.acceleration_residual <= 1e-10);
        assert!(reaction.instantaneous_power.abs() < 1e-9);
        // Independently evaluate the material-point acceleration for the spherical
        // inertia fixture. The wall normal has no Coriolis contribution.
        let omega = DVec3::from_array(body.spin.unwrap().angular_velocity().unwrap());
        let alpha = DVec3::from_array(reaction.first_wrench.torque);
        let acceleration = (DVec3::from_array(external.force)
            + DVec3::from_array(reaction.first_wrench.force))
            / body.motion.mass;
        let com = DVec3::from_array(body.motion.position);
        let n = DVec3::from_array(normal);
        for (support, actual) in post.iter().zip(&reaction.normal_accelerations) {
            let r = DVec3::from_array(support.contact.point) - com;
            let expected = n.dot(acceleration + alpha.cross(r) + omega.cross(omega.cross(r)));
            assert!((expected - actual).abs() < 1e-11);
        }
    }

    #[test]
    fn authored_wall_patch_stops_a_rigid_body_and_closes_boundary_ledger() {
        let mut scene = SceneGraph::new(2);
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(0.04, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.04, 2., 0.02],
                },
            )
            .unwrap();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![rigid_shapes()[0]]],
        };
        let mut liquid = rigid_liquid(Vec::new());
        let mut bodies = [rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.)];
        let energy = bodies[0].energy().unwrap();
        let report = liquid
            .step_with_rigid_body_world(
                0.03,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        assert!(bodies[0].motion.velocity.iter().all(|v| v.abs() < 1e-11));
        assert!(
            bodies[0]
                .spin
                .unwrap()
                .angular_momentum
                .iter()
                .all(|v| v.abs() < 1e-11)
        );
        assert!((report.environment_impulse[0] - 3.).abs() < 1e-11);
        assert!(
            (bodies[0].energy().unwrap() + report.dynamics.dissipated_energy - energy).abs()
                < 1e-11
        );
        let before = (liquid.clone(), bodies);
        let next = liquid
            .step_with_rigid_body_world(
                0.001,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(next.dynamics.contacts, 0);
        assert_eq!(bodies[0].motion.velocity, before.1[0].motion.velocity);
        assert_eq!(
            bodies[0].spin.unwrap().angular_momentum,
            before.1[0].spin.unwrap().angular_momentum
        );
        assert!(
            (glam::DVec3::from_array(bodies[0].motion.position)
                - glam::DVec3::from_array(before.1[0].motion.position))
            .length()
                < 1e-14
        );
        assert!(
            bodies[0].spin.unwrap().orientation[..3]
                .iter()
                .all(|v| v.abs() < 1e-15)
        );
    }

    #[test]
    fn scene_patch_solves_contact_velocities_but_free_rotation_requires_sustained_constraint() {
        use physics::liquid::LiquidBodyWorld;
        let scene = SceneGraph::new(1);
        let shapes = rigid_shapes();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: shapes.map(|s| vec![s]).to_vec(),
        };
        let mut a = rigid_body([-0.04, 1., 0.], [3., 0., 0.], 0.);
        let mut b = rigid_body([0.; 3], [0.; 3], 0.);
        let contacts = world
            .rigid_pair_patch(
                0,
                &a,
                1,
                &b,
                ContactWitness {
                    point: [0., 1., 0.],
                    tolerance_m: 1e-13,
                },
                [-1., 0., 0.],
                4,
            )
            .unwrap();
        assert_eq!(contacts.len(), 4);
        let report = physics::contact::resolve_normal_manifold(
            &mut a,
            Some(&mut b),
            &contacts,
            physics::contact::ManifoldConfig {
                max_sweeps: 100,
                velocity_tolerance: 1e-11,
            },
        )
        .unwrap();
        assert!(report.velocity_residual <= 1e-11);
        for c in contacts {
            assert!(
                -(a.point_velocity(c.point).unwrap()[0] - b.point_velocity(c.point).unwrap()[0])
                    >= -1e-11
            );
        }
        let dt = 0.001;
        let a = a
            .prepare_motion([0.; 3], [0.; 3], dt, rigid_config())
            .unwrap()
            .end();
        let b = b
            .prepare_motion([0.; 3], [0.; 3], dt, rigid_config())
            .unwrap()
            .end();
        let qa = glam::DQuat::from_array(a.spin.unwrap().orientation);
        let qb = glam::DQuat::from_array(b.spin.unwrap().orientation);
        let obstacle = crate::convex::AffineBox {
            center: glam::DVec3::from_array(b.motion.position) + qb * shapes[1].center,
            edges: shapes[1].edges.map(|e| qb * e),
        };
        assert!(
            obstacle
                .penetration_affine(
                    glam::DVec3::from_array(a.motion.position) + qa * shapes[0].center,
                    shapes[0].edges.map(|e| qa * e)
                )
                .is_some(),
            "a zero normal velocity at impact does not imply a clear free-rotation remainder"
        );
        assert_eq!(
            world.rigid_pair_patch(
                0,
                &a,
                1,
                &b,
                ContactWitness {
                    point: [0., 1., 0.],
                    tolerance_m: 1e-13
                },
                [-1., 0., 0.],
                0
            ),
            Err(physics::liquid::Error::CollisionBudget)
        );
    }

    #[test]
    fn authored_rigid_owners_rotate_publish_persist_pause_and_rollback() {
        let mut scene = SceneGraph::new(8);
        let mut nodes = Vec::new();
        for (position, velocity, center, half) in [
            ([-0.1, 1., 0.], [3., 0., 0.], 0., [0.04, 0.02, 0.02]),
            ([0.; 3], [0.; 3], 0.04, [0.04, 2., 0.02]),
        ] {
            let root = scene
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::from_array(position),
                        ..Default::default()
                    },
                )
                .unwrap();
            let child = scene
                .spawn(
                    Some(root),
                    Transform {
                        translation: glam::Vec3::new(center, 0., 0.),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(child, crate::BoxCollider { half_extents: half })
                .unwrap();
            scene
                .insert_component(
                    root,
                    crate::LiquidBody {
                        mass_kg: 1.,
                        initial_velocity_m_s: velocity,
                    },
                )
                .unwrap();
            // Mass distribution is explicit and independent of collision proxies.
            let h = 1.5_f64.sqrt();
            scene
                .insert_component(
                    root,
                    crate::LiquidMassDistribution {
                        parts: vec![crate::LiquidMassPart {
                            mass_kg: 1.,
                            center_m: [0.; 3],
                            half_edges_m: [[h, 0., 0.], [0., h, 0.], [0., 0., h]],
                        }],
                    },
                )
                .unwrap();
            nodes.push(root);
        }
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let energy: f64 = runtime
            .body_rigid_states()
            .map(|(_, s)| s.energy().unwrap())
            .sum();
        let before = runtime.clone();
        let poses_before: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        assert!(
            runtime
                .tick_and_publish(&mut scene, 0.03, None)
                .unwrap_err()
                .contains("CollisionBudget")
        );
        assert_eq!(runtime, before);
        assert_eq!(
            nodes
                .iter()
                .map(|n| scene.local(*n).unwrap())
                .collect::<Vec<_>>(),
            poses_before
        );
        let report = runtime
            .tick_and_publish_with_dynamics(&mut scene, 0.03, None, elastic_rigid(), rigid_config())
            .unwrap()
            .dynamics
            .unwrap();
        assert_eq!(report.dynamics.contacts, 1);
        let states: Vec<_> = runtime.body_rigid_states().collect();
        assert!((states[0].1.motion.velocity[0] - 1.).abs() < 1e-7);
        assert!((states[1].1.motion.velocity[0] - 2.).abs() < 1e-7);
        assert!((states[1].1.spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-7);
        let after: f64 = states.iter().map(|(_, s)| s.energy().unwrap()).sum();
        assert!((after + report.dynamics.dissipated_energy - energy).abs() < 1e-11);
        // Published child geometry follows the persistent principal template,
        // within the same admitted root-pose point error used by publication.
        for owner in &runtime.body {
            let spin = owner.state.spin.unwrap();
            let q = glam::DQuat::from_array(spin.orientation);
            let com = glam::DVec3::from_array(owner.state.motion.position);
            for (collider, shape) in owner
                .colliders
                .iter()
                .zip(owner.principal_templates.as_ref().unwrap())
            {
                let actual =
                    crate::affine_box(&scene, collider.node, collider.collider.half_extents)
                        .unwrap();
                assert!((actual.center - (com + q * shape.center)).length() < 1e-5);
                for k in 0..3 {
                    assert!((actual.edges[k] - q * shape.edges[k]).length() < 1e-5);
                }
            }
        }
        let rotation = scene.local(nodes[1]).unwrap().rotation;
        assert!(rotation.z.abs() > 0.001);
        runtime.validate_bindings(&scene).unwrap();
        runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
        assert_ne!(scene.local(nodes[1]).unwrap().rotation, rotation);
        for node in &nodes {
            scene.set_active(*node, false).unwrap();
        }
        let states: Vec<_> = runtime.body_rigid_states().collect();
        runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
        assert_eq!(runtime.body_rigid_states().collect::<Vec<_>>(), states);
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        runtime.body[0].state.motion.position[0] += 1.;
        runtime.body[1].state.motion.position[0] = 1e100;
        let before = runtime.clone();
        assert!(runtime.publish_body_pose(&mut scene).is_err());
        assert_eq!(runtime, before);
        assert_eq!(
            nodes
                .iter()
                .map(|n| scene.local(*n).unwrap())
                .collect::<Vec<_>>(),
            poses
        );
    }

    #[test]
    fn explicit_mass_distribution_admits_scaled_tensor_and_edits_rollback() {
        let (mut scene, nodes, _) = body_pair_fixture();
        let distribution = crate::LiquidMassDistribution {
            parts: vec![crate::LiquidMassPart {
                mass_kg: 1.,
                center_m: [0.25, 0., 0.],
                half_edges_m: [[0.1, 0., 0.], [0., 0.2, 0.], [0., 0., 0.3]],
            }],
        };
        scene
            .insert_component(nodes[0], distribution.clone())
            .unwrap();
        let mut pose = scene.local(nodes[0]).unwrap();
        pose.scale = glam::Vec3::new(2., 1., 1.);
        scene.set_local(nodes[0], pose).unwrap();
        // Duplicate collision proxies do not duplicate explicitly authored mass.
        let duplicate = scene
            .spawn(Some(nodes[0]), voxy_scene::Transform::default())
            .unwrap();
        scene
            .insert_component(duplicate, crate::BoxCollider::default())
            .unwrap();
        let mut runtime = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
            0,
        )
        .unwrap();
        let properties = runtime.body_mass_properties().next().unwrap().1.unwrap();
        assert_eq!(properties.center, [0.5, 0., 0.]);
        assert!((properties.inertia[0][0] - (0.04 + 0.09) / 3.).abs() < 1e-12);
        assert!((properties.inertia[1][1] - (0.04 + 0.09) / 3.).abs() < 1e-12);
        assert!(runtime.body_mass_properties().nth(1).unwrap().1.is_none());
        runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
        let before = runtime.clone();
        let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
        scene
            .component_mut::<crate::LiquidMassDistribution>(nodes[0])
            .unwrap()
            .unwrap()
            .parts[0]
            .mass_kg = 2.;
        assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
        assert_eq!(runtime, before);
        for (node, pose) in nodes.iter().zip(poses) {
            assert_eq!(scene.local(*node).unwrap(), pose);
        }
        assert!(
            SceneLiquidRuntime::new(
                &scene,
                vec![("unused".into(), Material::WATER)],
                Config::default(),
                0
            )
            .is_err()
        );
        let orphan = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
        scene.insert_component(orphan, distribution).unwrap();
        let error = SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0,
        )
        .unwrap_err();
        assert!(error.contains("requires a liquid body owner"));
    }
}
