//! Prescribed root-box motion, constrained by the colliders present at the mover phase.
use crate::{BoxCollider, CharacterBody, CharacterPhysics, CollisionLayers, GameplayError};
use crate::{PhysicsError, character_world::affine_box, convex::AffineBox};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use voxy_scene::{NodeId, SceneGraph, Transform};

const MAX_MOVERS: usize = 128;
const MAX_OBSTACLES: usize = 256;
const MAX_SWEEPS: usize = 65_536;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KinematicBody {
    pub velocity_m_s: [f64; 3],
}

pub(crate) fn validate_kinematic_bodies(scene: &SceneGraph) -> Result<(), GameplayError> {
    for (index, (owner, body)) in scene.components::<KinematicBody>().enumerate() {
        if index >= MAX_MOVERS
            || !DVec3::from_array(body.velocity_m_s).is_finite()
            || scene.parent(owner)?.is_some()
            || scene.component::<CharacterBody>(owner)?.is_some()
            || scene.component::<crate::LiquidBody>(owner)?.is_some()
            || scene.component::<crate::AngularMotion>(owner)?.is_some()
        {
            return Err(
                "kinematic body requires finite velocity and an exclusively owned root".into(),
            );
        }
        let collider = scene
            .component::<BoxCollider>(owner)?
            .ok_or("kinematic body requires a box collider on the same object")?;
        crate::validate_extents(collider.half_extents)?;
        affine_box(scene, owner, collider.half_extents)?;
    }
    for owner in scene
        .components::<BoxCollider>()
        .map(|(owner, _)| owner)
        .chain(scene.components::<CharacterBody>().map(|(owner, _)| owner))
    {
        let mut ancestor = scene.parent(owner)?;
        while let Some(parent) = ancestor {
            if scene.component::<KinematicBody>(parent)?.is_some() {
                return Err("kinematic body currently supports one collider on its root".into());
            }
            ancestor = scene.parent(parent)?;
        }
    }
    Ok(())
}

type Obstacle = (NodeId, AffineBox, CollisionLayers);

#[derive(Debug, Default)]
pub struct PlatformMotion {
    pub(crate) scene_revision: Option<voxy_scene::SceneChangeToken>,
    pub(crate) riders: Vec<(NodeId, DVec3)>,
    pub(crate) boundaries: Vec<(NodeId, DVec3)>,
    pub(crate) step_s: f64,
    originals: Vec<(NodeId, Transform)>,
}

impl PlatformMotion {
    /// Restore platform poses when the shared dynamics phase rejects the step.
    /// # Errors
    /// Rejects a receipt after the scene has changed.
    pub fn rollback(&self, scene: &mut SceneGraph) -> Result<(), GameplayError> {
        if self.originals.is_empty() {
            return Ok(());
        }
        if self.scene_revision != Some(scene.change_token()) {
            return Err(PhysicsError::InvalidMotion.into());
        }
        scene.set_locals(&self.originals)?;
        Ok(())
    }

    /// Keeps this phase admissible after a later system of the same tick (`joint.step`) moved
    /// the `moved` nodes. The recorded platform displacements still describe the scene when it
    /// was unchanged between this phase and `before` and no moved node is a platform, a carried
    /// rider or one of their ancestors; the phase then adopts the current scene revision.
    /// Otherwise the stale revision stays and the character phase rejects the tick.
    /// # Errors
    /// Reports stale platform or rider handles.
    pub fn adopt_unrelated_moves(
        &mut self,
        scene: &SceneGraph,
        before: voxy_scene::SceneChangeToken,
        moved: &[NodeId],
    ) -> Result<(), GameplayError> {
        if self.scene_revision != Some(before) {
            return Ok(());
        }
        for &(node, _) in self.riders.iter().chain(&self.boundaries) {
            let mut current = Some(node);
            while let Some(owner) = current {
                if moved.contains(&owner) {
                    return Ok(());
                }
                current = scene.parent(owner)?;
            }
        }
        self.scene_revision = Some(scene.change_token());
        Ok(())
    }
}

/// Advance authored movers in node order before dynamic bodies and characters.
/// Each accepted pose becomes an obstacle for subsequent movers.
/// # Errors
/// Invalid descriptors, initial overlaps or budget failure preserve all mover poses.
pub fn step_kinematic_bodies(
    scene: &mut SceneGraph,
    characters: Option<&CharacterPhysics>,
    dt: f64,
) -> Result<PlatformMotion, GameplayError> {
    step_kinematic_bodies_with_dynamics(scene, characters, dt, false)
}

/// Prepare platform motion for the shared rigid-body and particle event loop.
/// # Errors
/// Invalid or unresolved geometry preserves mover poses.
pub fn step_kinematic_bodies_with_dynamics(
    scene: &mut SceneGraph,
    characters: Option<&CharacterPhysics>,
    dt: f64,
    couple_dynamic: bool,
) -> Result<PlatformMotion, GameplayError> {
    step_kinematic_with_limits(scene, characters, dt, couple_dynamic, &[])
}

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(crate) fn step_kinematic_with_limits(
    scene: &mut SceneGraph,
    characters: Option<&CharacterPhysics>,
    dt: f64,
    couple_dynamic: bool,
    limits: &[(NodeId, f64)],
) -> Result<PlatformMotion, GameplayError> {
    if scene.active_components::<KinematicBody>().next().is_none() {
        return Ok(PlatformMotion::default());
    }
    validate_kinematic_bodies(scene)?;
    if !dt.is_finite() || dt <= 0. || dt > 0.1 {
        return Err(PhysicsError::InvalidStep.into());
    }
    let box_count = scene.active_components::<BoxCollider>().count();
    let mut obstacles = Vec::new();
    let mut can_ride = Vec::new();
    let mut dynamic_colliders = Vec::new();
    for (owner, extents) in scene
        .active_components::<BoxCollider>()
        .map(|(owner, collider)| (owner, collider.half_extents))
        .chain(
            scene
                .active_components::<CharacterBody>()
                .map(|(owner, character)| (owner, character.half_extents)),
        )
    {
        if obstacles.len() >= MAX_OBSTACLES {
            return Err(PhysicsError::Capacity.into());
        }
        crate::validate_extents(extents)?;
        let mut shape = affine_box(scene, owner, extents)?;
        if let Some(characters) = characters
            && scene.component::<CharacterBody>(owner)?.is_some()
        {
            let accepted = characters.accepted_pose(scene, owner)?;
            if accepted.is_none_or(|pose| pose.grounded) {
                can_ride.push(obstacles.len());
            }
            if let Some(pose) = accepted {
                shape.center = pose.physical_center;
                shape.edges = std::array::from_fn(|axis| {
                    pose.physical_rotation
                        * ([DVec3::X, DVec3::Y, DVec3::Z][axis] * f64::from(extents[axis]))
                });
            }
        }
        if couple_dynamic && obstacles.len() < box_count {
            let mut root = owner;
            while let Some(parent) = scene.parent(root)? {
                root = parent;
            }
            if scene.component::<crate::LiquidBody>(root)?.is_some() {
                dynamic_colliders.push(obstacles.len());
            }
        }
        obstacles.push((owner, shape, CollisionLayers::on(scene, owner)));
    }
    let mut movers: Vec<_> = scene.active_components::<KinematicBody>().collect();
    movers.sort_unstable_by_key(|(owner, _)| *owner);
    let mut edits = Vec::with_capacity(movers.len());
    let mut motion = PlatformMotion {
        step_s: dt,
        ..Default::default()
    };
    let mut remaining = MAX_SWEEPS;
    let mut supports = Vec::new();
    for rider in can_ride {
        let (_, shape, layers) = obstacles[rider];
        let mut nearest: Option<(f64, NodeId)> = None;
        spend(&mut remaining, box_count)?;
        for (target, surface, surface_layers) in &obstacles[..box_count] {
            if !layers.interacts(*surface_layers) {
                continue;
            }
            if let Some((fraction, normal)) =
                surface.sweep_affine(shape.center, shape.edges, DVec3::new(0., -0.005, 0.))
                && normal.y > 0.5
                && nearest.is_none_or(|prior| (fraction, *target) < prior)
            {
                nearest = Some((fraction, *target));
            }
        }
        if let Some((_, platform)) = nearest
            && scene.component::<KinematicBody>(platform)?.is_some()
        {
            supports.push((platform, rider));
        }
    }
    for (owner, body) in movers {
        let pose = scene.local(owner)?;
        let fraction = limits
            .iter()
            .find_map(|&(node, limit)| (node == owner).then_some(limit))
            .unwrap_or(1.);
        let displacement = DVec3::from_array(body.velocity_m_s) * (dt * fraction);
        if !displacement.is_finite() {
            return Err(PhysicsError::InvalidMotion.into());
        }
        if displacement == DVec3::ZERO {
            continue;
        }
        let index = obstacles
            .iter()
            .position(|(id, _, _)| *id == owner)
            .ok_or(PhysicsError::InvalidBody)?;
        let mut group = vec![index];
        group.extend(
            supports
                .iter()
                .filter_map(|&(platform, rider)| (platform == owner).then_some(rider)),
        );
        let accepted = stop_at_contact(
            pose,
            displacement,
            &group,
            &obstacles,
            &dynamic_colliders,
            &mut remaining,
        )?;
        if accepted != pose {
            let delta = accepted.translation.as_dvec3() - pose.translation.as_dvec3();
            obstacles[index].1.center = accepted.translation.as_dvec3();
            for &rider in &group[1..] {
                obstacles[rider].1.center += delta;
                motion.riders.push((obstacles[rider].0, delta));
            }
            motion.boundaries.push((owner, delta));
            motion.originals.push((owner, pose));
            edits.push((owner, accepted));
        }
    }
    scene.set_locals(&edits)?;
    if !motion.riders.is_empty() || !motion.boundaries.is_empty() {
        motion.scene_revision = Some(scene.change_token());
    }
    Ok(motion)
}

fn spend(remaining: &mut usize, count: usize) -> Result<(), PhysicsError> {
    *remaining = remaining
        .checked_sub(count)
        .ok_or(PhysicsError::SweepBudget)?;
    Ok(())
}

fn stop_at_contact(
    pose: Transform,
    displacement: DVec3,
    group: &[usize],
    obstacles: &[Obstacle],
    dynamic_colliders: &[usize],
    remaining: &mut usize,
) -> Result<Transform, PhysicsError> {
    let shape = obstacles[group[0]].1;
    spend(remaining, group.len() * obstacles.len())?;
    let pairs: Vec<_> = group
        .iter()
        .map(|&index| {
            let (_, shape, layers) = obstacles[index];
            let targets = obstacles
                .iter()
                .enumerate()
                .filter(|(slot, (_, _, other))| {
                    !group.contains(slot)
                        && (index != group[0] || !dynamic_colliders.contains(slot))
                        && layers.interacts(*other)
                })
                .map(|(_, (_, obstacle, _))| *obstacle)
                .collect::<Vec<_>>();
            (shape, targets)
        })
        .collect();
    let mut fraction = 1.0_f64;
    for (shape, candidates) in &pairs {
        for obstacle in candidates {
            if obstacle
                .penetration_affine(shape.center, shape.edges)
                .is_some()
            {
                return Err(PhysicsError::InitialOverlap);
            }
            if let Some((hit, _)) = obstacle.sweep_affine(shape.center, shape.edges, displacement) {
                fraction = fraction.min(hit);
            }
        }
    }
    let end_at = |fraction: f64| {
        let mut end = pose;
        end.translation = (shape.center + displacement * fraction).as_vec3();
        end
    };
    let mut is_clear = |end: Transform| {
        let motion = end.translation.as_dvec3() - shape.center;
        for (body, candidates) in &pairs {
            let center = body.center + motion;
            let extent = body.edges.iter().map(|edge| edge.abs()).sum::<DVec3>();
            if !center.is_finite() || (center.abs() + extent).max_element() > 1e6 {
                return Err(PhysicsError::CoordinateRange);
            }
            if motion == DVec3::ZERO {
                continue;
            }
            spend(remaining, candidates.len())?;
            for obstacle in candidates {
                if obstacle
                    .sweep_affine(body.center, body.edges, motion)
                    .is_some_and(|(hit, _)| hit < 1.)
                {
                    return Ok(false);
                }
            }
            match crate::angular_sweep::certify_published_pose(
                center, body.edges, candidates, remaining,
            ) {
                Ok(()) => {}
                Err(PhysicsError::InvalidMotion) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    };
    let end = end_at(fraction);
    if is_clear(end)? {
        return Ok(end);
    }
    // Rounding a contact endpoint can move the displayed box into its obstacle.
    // Keep only endpoints whose actual stored displacement passes the same sweep.
    let mut accepted = pose;
    let mut low = 0.;
    for _ in 0..24 {
        let mid = f64::midpoint(low, fraction);
        let end = end_at(mid);
        if is_clear(end)? {
            low = mid;
            accepted = end;
        } else {
            fraction = mid;
        }
    }
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn rejected_second_mover_preserves_first_candidate_pose() {
        let mut scene = SceneGraph::new(3);
        let mut spawn = |z| {
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::Z * z,
                        ..Default::default()
                    },
                )
                .unwrap();
            scene
                .insert_component(node, BoxCollider::default())
                .unwrap();
            node
        };
        let first = spawn(0.);
        let second = spawn(1.);
        spawn(1.);
        for owner in [first, second] {
            scene
                .insert_component(
                    owner,
                    KinematicBody {
                        velocity_m_s: [1., 0., 0.],
                    },
                )
                .unwrap();
        }
        let before = scene.local(first).unwrap();
        let token = scene.change_token();
        assert!(matches!(
            step_kinematic_bodies(&mut scene, None, 1. / 60.),
            Err(GameplayError::Physics(PhysicsError::InitialOverlap)),
        ));
        assert_eq!(scene.local(first).unwrap(), before);
        assert_eq!(scene.change_token(), token);
    }
}
