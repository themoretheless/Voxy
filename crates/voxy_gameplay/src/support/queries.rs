//! Borrowed read-only scene queries over the existing affine collision boxes.
use super::*;
use crate::{CharacterPhysics, StaticBox, affine_box, precise_center};

#[derive(Clone, Copy, Debug)]
pub struct RayQuery {
    pub origin: DVec3,
    /// Unit world-space direction; max_distance is measured in world units.
    pub direction: DVec3,
    pub max_distance: f64,
    /// Inside a solid returns distance zero; otherwise return its exit surface.
    pub solid: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub owner: NodeId,
    pub position: DVec3,
    /// None only for a solid interior hit, which has no unique surface normal.
    pub normal: Option<DVec3>,
    pub distance: f64,
    pub started_inside: bool,
}
/// Borrows both source owners, so neither scene nor accepted physical state can
/// change while this snapshot is used. No second solver or collision store.
#[derive(Debug)]
pub struct SceneQueries<'a> {
    _scene: &'a SceneGraph,
    _physics: Option<&'a CharacterPhysics>,
    world: SupportWorld,
}
impl CharacterPhysics {
    /// Include active characters at accepted f64 poses and active static boxes.
    /// Unstepped or externally edited characters use their authored affine pose.
    /// # Errors
    /// Rejects foreign scenes, malformed geometry and collider capacity limits.
    pub fn queries<'a>(
        &'a self,
        scene: &'a SceneGraph,
        capacity: usize,
    ) -> Result<SceneQueries<'a>, PhysicsError> {
        SceneQueries::build(scene, Some(self), capacity)
    }
}
impl<'a> SceneQueries<'a> {
    /// Query the published scene geometry without requiring a simulation owner.
    /// For accepted solver precision use CharacterPhysics::queries instead.
    /// The source cannot be mutated while its query snapshot is still used:
    /// ```compile_fail
    /// use voxy_gameplay::{RayQuery, SceneQueries, SupportQueryBudget};
    /// use voxy_scene::SceneGraph;
    /// use glam::DVec3;
    /// let mut scene = SceneGraph::new(1);
    /// let queries = SceneQueries::from_scene(&scene, 1).unwrap();
    /// scene.spawn(None, Default::default()).unwrap();
    /// queries.cast_ray(RayQuery { origin: DVec3::ZERO, direction: DVec3::X,
    ///     max_distance: 1., solid: true }, &mut SupportQueryBudget::new(1).unwrap(), |_| true);
    /// ```
    /// # Errors
    /// Rejects malformed geometry or a collider count above the caller budget.
    pub fn from_scene(scene: &'a SceneGraph, capacity: usize) -> Result<Self, PhysicsError> {
        Self::build(scene, None, capacity)
    }
    fn build(
        scene: &'a SceneGraph,
        physics: Option<&'a CharacterPhysics>,
        capacity: usize,
    ) -> Result<Self, PhysicsError> {
        if capacity > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        if let Some(physics) = physics {
            physics.validate(scene)?;
        } else {
            CharacterPhysics::new(scene, capacity, capacity).validate(scene)?;
        }
        let count = scene
            .active_components::<BoxCollider>()
            .count()
            .checked_add(scene.active_components::<crate::CharacterBody>().count())
            .ok_or(PhysicsError::Capacity)?;
        if count > capacity {
            return Err(PhysicsError::Capacity);
        }
        let mut world = static_world(scene)?;
        for (owner, body) in scene.active_components::<crate::CharacterBody>() {
            let shape = if let Some(physics) = physics
                && physics.accepted_pose(scene, owner)?.is_some()
            {
                let runtime = &physics.states[&owner];
                crate::convex::AffineBox {
                    center: precise_center(runtime.state.body),
                    edges: runtime.edges,
                }
            } else {
                affine_box(scene, owner, body.half_extents)?
            };
            let extent = shape.edges.iter().map(|edge| edge.abs()).sum::<DVec3>();
            world.0.push(StaticBox {
                owner,
                half_extents: body.half_extents,
                shape,
                axis_aligned: crate::aligned_edges(shape.edges),
                min: (shape.center - extent).to_array(),
                max: (shape.center + extent).to_array(),
            });
        }
        Ok(Self {
            _scene: scene,
            _physics: physics,
            world: SupportWorld::from_static_world(&world)?,
        })
    }
    /// Nearest hit with deterministic NodeId ties. Predicate excludes owners
    /// before intersection; every scanned collider consumes the shared budget.
    /// # Errors
    /// Rejects invalid rays, exhausted budgets and unstable geometric evaluation.
    /// No partial hit is returned on any failure.
    pub fn cast_ray(
        &self,
        ray: RayQuery,
        budget: &mut SupportQueryBudget,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Option<RayHit>, PhysicsError> {
        validate_ray(ray)?;
        let mut best: Option<RayHit> = None;
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            if let Some(hit) = intersect(surface, ray)?
                && best.is_none_or(|best| (hit.distance, hit.owner) < (best.distance, best.owner))
            {
                best = Some(hit);
            }
        }
        Ok(best)
    }
    /// All hits sorted by distance then stable owner. Capacity exhaustion rejects
    /// the whole query instead of truncating the result or publishing callbacks.
    /// # Errors
    /// Rejects invalid rays, hit/query budgets and unstable intersections.
    pub fn intersect_ray(
        &self,
        ray: RayQuery,
        budget: &mut SupportQueryBudget,
        max_hits: usize,
        accept: impl Fn(NodeId) -> bool,
    ) -> Result<Vec<RayHit>, PhysicsError> {
        validate_ray(ray)?;
        if max_hits > MAX_COLLIDERS {
            return Err(PhysicsError::Capacity);
        }
        let mut hits = Vec::with_capacity(max_hits.min(self.world.surfaces.len()));
        for surface in &self.world.surfaces {
            budget.spend()?;
            if !accept(surface.owner) {
                continue;
            }
            if let Some(hit) = intersect(surface, ray)? {
                if hits.len() == max_hits {
                    return Err(PhysicsError::Capacity);
                }
                hits.push(hit);
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then(a.owner.cmp(&b.owner))
        });
        Ok(hits)
    }
}
fn validate_ray(ray: RayQuery) -> Result<(), PhysicsError> {
    if !ray.origin.is_finite()
        || ray.origin.abs().max_element() > 1e6
        || !ray.direction.is_finite()
        || (ray.direction.length_squared() - 1.).abs() > 1e-12
        || !ray.max_distance.is_finite()
        || !(0. ..=1e6).contains(&ray.max_distance)
    {
        return Err(PhysicsError::InvalidMotion);
    }
    Ok(())
}
fn intersect(surface: &Surface, ray: RayQuery) -> Result<Option<RayHit>, PhysicsError> {
    let p = surface.inverse * (ray.origin - surface.center);
    let v = surface.inverse * ray.direction;
    if !p.is_finite() || !v.is_finite() {
        return Err(PhysicsError::Solver);
    }
    let inside = p.abs().max_element() < 1.;
    if inside && ray.solid {
        return Ok(Some(RayHit {
            owner: surface.owner,
            position: ray.origin,
            normal: None,
            distance: 0.,
            started_inside: true,
        }));
    }
    let (mut enter, mut exit) = (f64::NEG_INFINITY, f64::INFINITY);
    let (mut near_face, mut far_face) = ((0, 0.), (0, 0.));
    for axis in 0..3 {
        if v[axis] == 0. {
            if p[axis].abs() > 1. {
                return Ok(None);
            }
            continue;
        }
        let (mut near, mut far) = ((-1. - p[axis]) / v[axis], (1. - p[axis]) / v[axis]);
        let mut sign = -1.;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            sign = 1.;
        }
        if near > enter {
            enter = near;
            near_face = (axis, sign);
        }
        if far < exit {
            exit = far;
            far_face = (axis, -sign);
        }
        if enter > exit {
            return Ok(None);
        }
    }
    let (distance, (face, sign)) = if enter >= 0. {
        (enter, near_face)
    } else {
        (exit, far_face)
    };
    let distance = if distance == 0. { 0. } else { distance };
    if !distance.is_finite() || distance < 0. || distance > ray.max_distance || sign == 0. {
        return Ok(None);
    }
    let mut local = p + v * distance;
    local[face] = sign;
    let guard =
        128. * f64::EPSILON * (1. + p.abs().max_element() + v.abs().max_element() * distance);
    if !local.is_finite() || local.abs().max_element() > 1. + guard {
        return Err(PhysicsError::Solver);
    }
    local = local.clamp(DVec3::NEG_ONE, DVec3::ONE);
    let position = surface.center + surface.edges * local;
    let on_ray = ray.origin + ray.direction * distance;
    let tolerance =
        256. * f64::EPSILON * (1. + ray.origin.abs().max_element() + position.abs().max_element());
    if !position.is_finite() || !position.abs_diff_eq(on_ray, tolerance) {
        return Err(PhysicsError::Solver);
    }
    Ok(Some(RayHit {
        owner: surface.owner,
        position,
        normal: Some(surface.normals[face] * sign),
        distance,
        started_inside: inside,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};
    use voxy_scene::Transform;
    fn ray(origin: DVec3, direction: DVec3) -> RayQuery {
        RayQuery {
            origin,
            direction,
            max_distance: 10.,
            solid: true,
        }
    }
    fn budget() -> SupportQueryBudget {
        SupportQueryBudget::new(64).unwrap()
    }
    fn box_at(scene: &mut SceneGraph, x: f32) -> NodeId {
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * x,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                BoxCollider {
                    half_extents: [0.5; 3],
                },
            )
            .unwrap();
        owner
    }
    #[test]
    fn nearest_all_hits_filters_and_stable_ties_include_characters() {
        let mut scene = SceneGraph::new(4);
        let first = box_at(&mut scene, 3.);
        let duplicate = box_at(&mut scene, 3.);
        let far = box_at(&mut scene, 5.);
        let body = scene
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
                body,
                crate::CharacterBody {
                    half_extents: [0.25; 3],
                    ..Default::default()
                },
            )
            .unwrap();
        let queries = SceneQueries::from_scene(&scene, 4).unwrap();
        let r = ray(DVec3::ZERO, DVec3::X);
        let hit = queries
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(hit.owner, body);
        assert_eq!(hit.distance, 0.75);
        assert_eq!(hit.normal, Some(-DVec3::X));
        assert_eq!(
            queries
                .cast_ray(r, &mut budget(), |id| id != body)
                .unwrap()
                .unwrap()
                .owner,
            first
        );
        let hits = queries
            .intersect_ray(r, &mut budget(), 4, |id| id != body)
            .unwrap();
        assert_eq!(
            hits.iter().map(|hit| hit.owner).collect::<Vec<_>>(),
            vec![first, duplicate, far]
        );
        assert!(
            queries
                .cast_ray(r, &mut budget(), |_| false)
                .unwrap()
                .is_none()
        );
        scene.set_active(body, false).unwrap();
        assert_eq!(
            SceneQueries::from_scene(&scene, 3)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
                .owner,
            first
        );
    }
    #[test]
    fn interior_solid_hollow_boundary_and_finite_segment_semantics() {
        let mut scene = SceneGraph::new(1);
        box_at(&mut scene, 0.);
        let q = SceneQueries::from_scene(&scene, 1).unwrap();
        let mut r = ray(DVec3::ZERO, DVec3::X);
        r.max_distance = 0.;
        let hit = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert!(hit.started_inside && hit.normal.is_none());
        assert_eq!(hit.distance, 0.);
        r.solid = false;
        assert!(q.cast_ray(r, &mut budget(), |_| true).unwrap().is_none());
        r.max_distance = 1.;
        let hit = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert_eq!(hit.distance, 0.5);
        assert_eq!(hit.normal, Some(DVec3::X));
        assert!(hit.started_inside);
        r.origin.x = 0.5;
        r.max_distance = 0.;
        let boundary = q.cast_ray(r, &mut budget(), |_| true).unwrap().unwrap();
        assert_eq!(boundary.distance.to_bits(), 0_f64.to_bits());
        assert_eq!(boundary.normal, Some(DVec3::X));
        assert!(!boundary.started_inside);
        r = ray(DVec3::new(-1.5, 0., 0.), DVec3::X);
        r.max_distance = 1.;
        assert_eq!(
            q.cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
                .distance,
            1.
        );
        r.max_distance = f64::from_bits(1_f64.to_bits() - 1);
        assert!(q.cast_ray(r, &mut budget(), |_| true).unwrap().is_none());
        assert!(
            q.cast_ray(
                ray(DVec3::new(0.500001, -1., 0.), DVec3::Y),
                &mut budget(),
                |_| true
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn affine_shear_and_reflection_return_world_point_and_outward_normal() {
        let mut scene = SceneGraph::new(2);
        let parent = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(3., -1., 2.),
                    rotation: Quat::from_rotation_z(0.3),
                    scale: Vec3::new(-2., 0.5, 1.5),
                },
            )
            .unwrap();
        let owner = scene
            .spawn(
                Some(parent),
                Transform {
                    rotation: Quat::from_rotation_y(0.4),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                BoxCollider {
                    half_extents: [0.2, 0.3, 0.4],
                },
            )
            .unwrap();
        let matrix = scene.world_matrix(owner).unwrap();
        let edges = [
            matrix.x_axis.truncate().as_dvec3() * f64::from(0.2_f32),
            matrix.y_axis.truncate().as_dvec3() * f64::from(0.3_f32),
            matrix.z_axis.truncate().as_dvec3() * f64::from(0.4_f32),
        ];
        let point =
            matrix.w_axis.truncate().as_dvec3() + edges[0] + edges[1] * 0.2 - edges[2] * 0.1;
        let normal =
            edges[1].cross(edges[2]).normalize() * f64::from(matrix.determinant().signum());
        let q = SceneQueries::from_scene(&scene, 1).unwrap();
        let hit = q
            .cast_ray(ray(point + normal * 2.5, -normal), &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - 2.5).abs() < 1e-12);
        assert!(hit.position.abs_diff_eq(point, 1e-12));
        assert!(hit.normal.unwrap().abs_diff_eq(normal, 1e-12));
    }
    #[test]
    fn invalid_queries_and_late_budget_failures_never_return_partial_hits() {
        let mut scene = SceneGraph::new(2);
        box_at(&mut scene, 2.);
        box_at(&mut scene, 4.);
        let token = scene.change_token();
        let q = SceneQueries::from_scene(&scene, 2).unwrap();
        let r = ray(DVec3::ZERO, DVec3::X);
        assert_eq!(
            q.cast_ray(r, &mut SupportQueryBudget::new(1).unwrap(), |_| true),
            Err(PhysicsError::SweepBudget)
        );
        assert_eq!(
            q.intersect_ray(r, &mut budget(), 1, |_| true),
            Err(PhysicsError::Capacity)
        );
        for bad in [
            RayQuery {
                direction: DVec3::ZERO,
                ..r
            },
            RayQuery {
                direction: DVec3::X * 2.,
                ..r
            },
            RayQuery {
                max_distance: f64::NAN,
                ..r
            },
            RayQuery {
                origin: DVec3::splat(f64::INFINITY),
                ..r
            },
        ] {
            let mut b = budget();
            assert_eq!(
                q.cast_ray(bad, &mut b, |_| true),
                Err(PhysicsError::InvalidMotion)
            );
            assert_eq!(b.remaining(), 64);
        }
        assert!(matches!(
            SceneQueries::from_scene(&scene, 1),
            Err(PhysicsError::Capacity)
        ));
        assert_eq!(scene.change_token(), token);
    }
    #[test]
    fn accepted_solver_precision_survives_controller_checkpoint_and_external_teleport() {
        let mut scene = SceneGraph::new(1);
        let owner = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::X * 100000.,
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                crate::CharacterBody {
                    half_extents: [0.1; 3],
                    speed: 0.6,
                    gravity: 0.,
                    jump_speed: 0.,
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 0);
        let mut input = crate::player_input().unwrap();
        input.event(crate::RIGHT, 1.).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        let pose = physics.accepted_pose(&scene, owner).unwrap().unwrap();
        assert_ne!(
            pose.physical_center.x,
            f64::from(pose.world_matrix.w_axis.x)
        );
        let r = ray(DVec3::new(99998., 0., 0.), DVec3::X);
        let expected = pose.physical_center.x - 99998. - f64::from(0.1_f32);
        let hit = physics
            .queries(&scene, 1)
            .unwrap()
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - expected).abs() < 1e-10);
        let published = SceneQueries::from_scene(&scene, 1)
            .unwrap()
            .cast_ray(r, &mut budget(), |_| true)
            .unwrap()
            .unwrap();
        assert!((hit.distance - published.distance).abs() > 1e-4);
        let saved = physics
            .checkpoint(&scene, |_| Some(voxy_scene::ObjectId("body".into())))
            .unwrap();
        let saved = crate::CharacterPhysicsCheckpoint::from_json(
            &saved.to_json(100000).unwrap(),
            100000,
            1,
        )
        .unwrap();
        let mut restored = CharacterPhysics::new(&scene, 1, 0);
        restored
            .restore_checkpoint(&scene, &saved, |_| Some(owner))
            .unwrap();
        assert_eq!(
            restored
                .queries(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap(),
            hit
        );
        let mut local = scene.local(owner).unwrap();
        local.translation.x += 1.;
        scene.set_local(owner, local).unwrap();
        assert_eq!(
            physics
                .queries(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap(),
            SceneQueries::from_scene(&scene, 1)
                .unwrap()
                .cast_ray(r, &mut budget(), |_| true)
                .unwrap()
                .unwrap()
        );
    }
}
