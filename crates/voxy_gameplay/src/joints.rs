//! Authoring and fixed-tick constraint joints (`DistanceSpring`, `Hinge`, `Fixed`) between scene objects.
//!
//! The solver is kinematic: it repositions the owner relative to the target and knows nothing
//! about mass, velocity or impulses. Constraints are solved in world space, so owner and target
//! may have different parents. Hosts run it as the `joint.step` system of
//! [`crate::gameplay_schedule`], after kinematic movers and before characters.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use voxy_scene::{ObjectId, SceneGraph, Transform};

/// Physical joint constraint connecting its owning scene node to a `target` object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsJoint {
    /// Durable identity of the anchor/parent body this node is constrained to.
    pub target: ObjectId,
    /// Local anchor offset on the `target` node (metres).
    pub target_anchor: [f32; 3],
    /// Local anchor offset on this owning node (metres).
    pub local_anchor: [f32; 3],
    /// Constraint topology and parameters.
    pub kind: PhysicsJointKind,
}

/// Supported joint constraint formulations.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysicsJointKind {
    /// Rigid weld keeping the owner at a fixed relative position and orientation to `target`.
    Fixed { relative_rotation: [f32; 4] },
    /// Revolute hinge joint around `axis` (unit vector in target space) with angular limits `[min_rad, max_rad]`.
    Hinge {
        axis: [f32; 3],
        min_angle_rad: f32,
        max_angle_rad: f32,
    },
    /// Compliant distance spring-damper (XPBD / Hooke relaxation) maintaining `rest_length`.
    DistanceSpring {
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    },
}

impl PhysicsJoint {
    /// Validates that all joint parameters are finite and within physical bounds.
    #[must_use]
    pub fn valid(&self) -> bool {
        if self.target.0.is_empty()
            || self
                .target_anchor
                .iter()
                .chain(self.local_anchor.iter())
                .any(|v| !v.is_finite())
        {
            return false;
        }
        match self.kind {
            PhysicsJointKind::Fixed { relative_rotation } => {
                let q = Quat::from_array(relative_rotation);
                q.is_finite() && (q.length_squared() - 1.0).abs() <= 1e-3
            }
            PhysicsJointKind::Hinge {
                axis,
                min_angle_rad,
                max_angle_rad,
            } => {
                let a = Vec3::from_array(axis);
                a.is_finite()
                    && (a.length_squared() - 1.0).abs() <= 1e-3
                    && min_angle_rad.is_finite()
                    && max_angle_rad.is_finite()
                    && min_angle_rad <= max_angle_rad
            }
            PhysicsJointKind::DistanceSpring {
                rest_length,
                stiffness,
                damping,
            } => {
                rest_length.is_finite()
                    && rest_length >= 0.0
                    && stiffness.is_finite()
                    && stiffness > 0.0
                    && damping.is_finite()
                    && damping >= 0.0
            }
        }
    }
}

/// Solves a single XPBD / Hookean distance-spring positional correction step over `dt` seconds,
/// returning the updated `owner_pos` that relaxes the distance between `owner_pos` and `anchor_pos`
/// toward `rest_length`.
#[must_use]
pub fn solve_distance_spring_position(
    anchor_pos: Vec3,
    owner_pos: Vec3,
    rest_length: f32,
    stiffness: f32,
    damping: f32,
    dt: f32,
) -> Vec3 {
    if !anchor_pos.is_finite()
        || !owner_pos.is_finite()
        || !rest_length.is_finite()
        || !stiffness.is_finite()
        || !damping.is_finite()
        || !dt.is_finite()
        || dt <= 0.0
    {
        return owner_pos;
    }
    let delta = owner_pos - anchor_pos;
    let dist = delta.length();
    let dir = if dist > 1e-6 { delta / dist } else { Vec3::Y };
    let error = dist - rest_length.max(0.0);
    // Implicit first-order relaxation factor in [0, 1]:
    let alpha = ((stiffness.max(0.0) * dt) / (1.0 + (stiffness.max(0.0) + damping.max(0.0)) * dt))
        .clamp(0.0, 1.0);
    owner_pos - dir * (error * alpha)
}

/// Collects every active `game.physics-joint.v1` binding, resolving durable target identities to
/// live nodes through `resolve` (the host owns the identity index).
/// # Errors
/// Rejects a target that does not resolve or a joint constrained to its own node.
pub fn collect_physics_joints(
    scene: &SceneGraph,
    mut resolve: impl FnMut(&ObjectId) -> Option<voxy_scene::NodeId>,
) -> Result<Vec<(voxy_scene::NodeId, voxy_scene::NodeId, PhysicsJoint)>, String> {
    let mut bindings = Vec::new();
    for (owner, joint) in scene.active_components::<PhysicsJoint>() {
        let target = resolve(&joint.target)
            .ok_or_else(|| format!("unresolved physics joint target {}", joint.target.0))?;
        if target == owner {
            return Err("physics joint targets its own node".into());
        }
        bindings.push((owner, target, joint.clone()));
    }
    Ok(bindings)
}

/// Preflight for every `game.physics-joint.v1`, active or not: parameters must pass
/// [`PhysicsJoint::valid`], the target must resolve through `resolve` (the host owns the identity
/// index) and must not be the owning node. Hosts call this when an authoring document changes and
/// before the first fixed tick, so `joint.step` never meets a broken descriptor.
/// # Errors
/// Names the first invalid, unresolved or self-targeting joint without changing the scene.
pub fn validate_physics_joints(
    scene: &SceneGraph,
    mut resolve: impl FnMut(&ObjectId) -> Option<voxy_scene::NodeId>,
) -> Result<(), String> {
    for (owner, joint) in scene.components::<PhysicsJoint>() {
        if !joint.valid() {
            return Err("invalid game.physics-joint.v1 descriptor".into());
        }
        let target = resolve(&joint.target).ok_or_else(|| {
            format!(
                "game.physics-joint.v1 target {} is not in the scene",
                joint.target.0
            )
        })?;
        if target == owner {
            return Err("game.physics-joint.v1 targets its own object".into());
        }
    }
    Ok(())
}

/// World-space rotation and translation of a node; scale is dropped (anchors are scaled first).
fn world_pose(
    scene: &SceneGraph,
    node: voxy_scene::NodeId,
) -> Result<(glam::Mat4, Quat, Vec3), String> {
    let world = scene.world_matrix(node).map_err(|e| e.to_string())?;
    let (scale, rotation, translation) = world.to_scale_rotation_translation();
    if !scale.is_finite() || !rotation.is_finite() || !translation.is_finite() {
        return Err("physics joint node has a degenerate world transform".into());
    }
    Ok((world, rotation.normalize(), translation))
}

/// Applies one fixed-step constraint projection pass for all active `(owner_node, target_node, PhysicsJoint)`
/// bindings in `scene`. Constraints are solved in world space against the poses at the start of the
/// pass and written back as owner-local transforms, so owners and targets may sit anywhere in the
/// hierarchy. The owner's local scale is preserved.
/// # Errors
/// Rejects invalid node handles, invalid joint parameters, or non-finite resulting transforms.
pub fn step_physics_joints(
    scene: &mut SceneGraph,
    bindings: &[(voxy_scene::NodeId, voxy_scene::NodeId, PhysicsJoint)],
    dt: f32,
) -> Result<(), String> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err("invalid joint timestep".into());
    }
    let mut edits = Vec::with_capacity(bindings.len());
    for (owner, target, joint) in bindings {
        if !joint.valid() {
            return Err("invalid game.physics-joint.v1 parameters".into());
        }
        if !scene
            .active_in_hierarchy(*owner)
            .map_err(|e| e.to_string())?
            || !scene
                .active_in_hierarchy(*target)
                .map_err(|e| e.to_string())?
        {
            continue;
        }
        let (target_world, target_rotation, _) = world_pose(scene, *target)?;
        let (owner_world, mut rotation, mut translation) = world_pose(scene, *owner)?;
        let anchor_world = target_world.transform_point3(Vec3::from_array(joint.target_anchor));
        // Owner anchor offset in world axes, excluding rotation: rotate it by the solved rotation.
        let scaled_anchor = rotation.inverse()
            * owner_world.transform_vector3(Vec3::from_array(joint.local_anchor));

        match joint.kind {
            PhysicsJointKind::Fixed { relative_rotation } => {
                let rel_q = Quat::from_array(relative_rotation).normalize();
                rotation = (target_rotation * rel_q).normalize();
                translation = anchor_world - rotation * scaled_anchor;
            }
            PhysicsJointKind::Hinge {
                axis,
                min_angle_rad,
                max_angle_rad,
            } => {
                let axis = Vec3::from_array(axis).normalize();
                let rel = target_rotation.inverse() * rotation;
                let (rot_axis, mut angle) = rel.to_axis_angle();
                if angle > std::f32::consts::PI {
                    angle -= std::f32::consts::TAU;
                }
                if rot_axis.dot(axis) < 0.0 {
                    angle = -angle;
                }
                let clamped = angle.clamp(min_angle_rad, max_angle_rad);
                rotation = (target_rotation * Quat::from_axis_angle(axis, clamped)).normalize();
                translation = anchor_world - rotation * scaled_anchor;
            }
            PhysicsJointKind::DistanceSpring {
                rest_length,
                stiffness,
                damping,
            } => {
                let offset = rotation * scaled_anchor;
                let solved_attach = solve_distance_spring_position(
                    anchor_world,
                    translation + offset,
                    rest_length,
                    stiffness,
                    damping,
                    dt,
                );
                translation = solved_attach - offset;
            }
        }
        let parent_world = match scene.parent(*owner).map_err(|e| e.to_string())? {
            Some(parent) => scene.world_matrix(parent).map_err(|e| e.to_string())?,
            None => glam::Mat4::IDENTITY,
        };
        let world = glam::Mat4::from_rotation_translation(rotation, translation);
        let (_, local_rotation, local_translation) =
            (parent_world.inverse() * world).to_scale_rotation_translation();
        if !local_rotation.is_finite() || !local_translation.is_finite() {
            return Err("physics joint produced a non-finite transform".into());
        }
        let scale = scene.local(*owner).map_err(|e| e.to_string())?.scale;
        edits.push((
            *owner,
            Transform {
                translation: local_translation,
                rotation: local_rotation.normalize(),
                scale,
            },
        ));
    }
    scene.set_locals(&edits).map_err(|e| e.to_string())
}

/// Body of the `joint.step` system shared by hosts: resolves and solves every active joint, then
/// keeps the platform phase of this tick admissible when the joints moved nothing it carries
/// (see [`crate::PlatformMotion::adopt_unrelated_moves`]).
/// # Errors
/// Same as [`collect_physics_joints`] and [`step_physics_joints`].
pub fn step_scene_physics_joints(
    scene: &mut SceneGraph,
    resolve: impl FnMut(&ObjectId) -> Option<voxy_scene::NodeId>,
    dt: f32,
    platform: &mut crate::PlatformMotion,
) -> Result<(), String> {
    let bindings = collect_physics_joints(scene, resolve)?;
    if bindings.is_empty() {
        return Ok(());
    }
    let before = scene.change_token();
    step_physics_joints(scene, &bindings, dt)?;
    let owners: Vec<_> = bindings.iter().map(|(owner, _, _)| *owner).collect();
    platform
        .adopt_unrelated_moves(scene, before, &owners)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_spring_joint_relaxes_toward_rest_length() {
        let mut scene = SceneGraph::new(4);
        let anchor = scene.spawn(None, Transform::default()).unwrap();
        let bob = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.0, -3.0, 0.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        let spring = PhysicsJoint {
            target: ObjectId("anchor".into()),
            target_anchor: [0.0, 0.0, 0.0],
            local_anchor: [0.0, 0.0, 0.0],
            kind: PhysicsJointKind::DistanceSpring {
                rest_length: 1.0,
                stiffness: 50.0,
                damping: 2.0,
            },
        };
        assert!(spring.valid());
        step_physics_joints(&mut scene, &[(bob, anchor, spring)], 1.0 / 60.0).unwrap();
        let updated = scene.local(bob).unwrap().translation;
        assert!(updated.y > -3.0 && updated.y < -1.0);
    }

    #[test]
    fn fixed_and_hinge_joints_follow_targets_across_different_parents() {
        let mut scene = SceneGraph::new(8);
        let at = |x: f32, y: f32, z: f32| Transform {
            translation: Vec3::new(x, y, z),
            ..Transform::default()
        };
        // Target lives under a moved, rotated parent; owner under a differently placed one.
        let target_parent = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(10.0, 0.0, 0.0),
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                    ..Transform::default()
                },
            )
            .unwrap();
        let target = scene.spawn(Some(target_parent), at(0.0, 2.0, 0.0)).unwrap();
        let owner_parent = scene.spawn(None, at(-4.0, 1.0, 3.0)).unwrap();
        let owner = scene.spawn(Some(owner_parent), at(0.0, 0.0, 0.0)).unwrap();

        let fixed = PhysicsJoint {
            target: ObjectId("target".into()),
            target_anchor: [0.0, 0.0, 1.0],
            local_anchor: [0.0, 0.5, 0.0],
            kind: PhysicsJointKind::Fixed {
                relative_rotation: [0.0, 0.0, 0.0, 1.0],
            },
        };
        step_physics_joints(&mut scene, &[(owner, target, fixed.clone())], 1.0 / 60.0).unwrap();
        let target_world = scene.world_matrix(target).unwrap();
        let owner_world = scene.world_matrix(owner).unwrap();
        let anchor = target_world.transform_point3(Vec3::new(0.0, 0.0, 1.0));
        let attach = owner_world.transform_point3(Vec3::new(0.0, 0.5, 0.0));
        assert!(anchor.distance(attach) < 1e-4, "{anchor} vs {attach}");
        let (_, owner_rotation, _) = owner_world.to_scale_rotation_translation();
        let (_, target_rotation, _) = target_world.to_scale_rotation_translation();
        // acos near 1 loses half the mantissa, so compare the quaternion dot instead of the angle.
        assert!(owner_rotation.dot(target_rotation).abs() > 1.0 - 1e-6);

        // Hinge: owner twisted 2 rad about the target's Y axis is clamped to the 0.5 rad limit.
        let twisted = target_rotation * Quat::from_rotation_y(2.0);
        let parent_rotation = scene
            .world_matrix(owner_parent)
            .unwrap()
            .to_scale_rotation_translation()
            .1;
        let mut local = scene.local(owner).unwrap();
        local.rotation = parent_rotation.inverse() * twisted;
        scene.set_local(owner, local).unwrap();
        let hinge = PhysicsJoint {
            kind: PhysicsJointKind::Hinge {
                axis: [0.0, 1.0, 0.0],
                min_angle_rad: -0.5,
                max_angle_rad: 0.5,
            },
            ..fixed
        };
        step_physics_joints(&mut scene, &[(owner, target, hinge)], 1.0 / 60.0).unwrap();
        let owner_world = scene.world_matrix(owner).unwrap();
        let (_, owner_rotation, _) = owner_world.to_scale_rotation_translation();
        let relative = target_rotation.inverse() * owner_rotation;
        let (axis, angle) = relative.to_axis_angle();
        assert!(
            (angle - 0.5).abs() < 1e-4 && axis.dot(Vec3::Y) > 0.999,
            "{axis} {angle}"
        );
        let attach = owner_world.transform_point3(Vec3::new(0.0, 0.5, 0.0));
        assert!(anchor.distance(attach) < 1e-4);
    }

    #[test]
    fn collect_resolves_targets_and_rejects_missing_or_self_references() {
        let mut scene = SceneGraph::new(4);
        let anchor = scene.spawn(None, Transform::default()).unwrap();
        let bob = scene.spawn(None, Transform::default()).unwrap();
        let joint = PhysicsJoint {
            target: ObjectId("anchor".into()),
            target_anchor: [0.0; 3],
            local_anchor: [0.0; 3],
            kind: PhysicsJointKind::DistanceSpring {
                rest_length: 1.0,
                stiffness: 10.0,
                damping: 0.0,
            },
        };
        scene.insert_component(bob, joint.clone()).unwrap();
        let bindings =
            collect_physics_joints(&scene, |id| (id.0 == "anchor").then_some(anchor)).unwrap();
        assert_eq!(bindings, vec![(bob, anchor, joint)]);
        assert!(collect_physics_joints(&scene, |_| None).is_err());
        assert!(collect_physics_joints(&scene, |_| Some(bob)).is_err());
    }

    #[test]
    fn joint_step_keeps_a_moving_platform_phase_admissible_unless_it_moves_the_platform() {
        let mut scene = SceneGraph::new(8);
        let at = |x: f32| Transform {
            translation: Vec3::new(x, 0.0, 0.0),
            ..Transform::default()
        };
        let platform = scene.spawn(None, at(0.0)).unwrap();
        let follower = scene.spawn(None, at(0.0)).unwrap();
        let post = scene.spawn(None, at(5.0)).unwrap();
        scene
            .insert_component(platform, crate::BoxCollider::default())
            .unwrap();
        scene
            .insert_component(
                platform,
                crate::KinematicBody {
                    velocity_m_s: [0.6, 0.0, 0.0],
                },
            )
            .unwrap();
        let weld = |target: &str| PhysicsJoint {
            target: ObjectId(target.into()),
            target_anchor: [0.0, 1.0, 0.0],
            local_anchor: [0.0; 3],
            kind: PhysicsJointKind::Fixed {
                relative_rotation: [0.0, 0.0, 0.0, 1.0],
            },
        };
        scene.insert_component(follower, weld("platform")).unwrap();
        let resolve = |id: &ObjectId| match id.0.as_str() {
            "platform" => Some(platform),
            "post" => Some(post),
            _ => None,
        };
        let mut motion = crate::step_kinematic_bodies(&mut scene, None, 0.1).unwrap();
        step_scene_physics_joints(&mut scene, resolve, 0.1, &mut motion).unwrap();
        let moved = scene.local(platform).unwrap().translation.x;
        assert!((moved - 0.06).abs() < 1e-6, "{moved}");
        assert_eq!(
            scene.local(follower).unwrap().translation,
            Vec3::new(moved, 1.0, 0.0)
        );
        // The follower is not part of the platform phase, so its receipt is still current.
        motion.rollback(&mut scene).unwrap();
        assert_eq!(scene.local(platform).unwrap().translation.x, 0.0);

        // A joint that drags the platform itself invalidates the receipt, as before.
        scene.insert_component(platform, weld("post")).unwrap();
        let mut motion = crate::step_kinematic_bodies(&mut scene, None, 0.1).unwrap();
        step_scene_physics_joints(&mut scene, resolve, 0.1, &mut motion).unwrap();
        assert!(motion.rollback(&mut scene).is_err());
    }

    #[test]
    fn preflight_rejects_invalid_unresolved_and_self_targeting_joints_even_when_inactive() {
        let mut scene = SceneGraph::new(4);
        let anchor = scene.spawn(None, Transform::default()).unwrap();
        let bob = scene.spawn(None, Transform::default()).unwrap();
        let mut joint = PhysicsJoint {
            target: ObjectId("anchor".into()),
            target_anchor: [0.0; 3],
            local_anchor: [0.0; 3],
            kind: PhysicsJointKind::Hinge {
                axis: [0.0, 1.0, 0.0],
                min_angle_rad: -1.0,
                max_angle_rad: 1.0,
            },
        };
        scene.insert_component(bob, joint.clone()).unwrap();
        scene.set_active(bob, false).unwrap();
        let resolve = |id: &ObjectId| (id.0 == "anchor").then_some(anchor);
        validate_physics_joints(&scene, resolve).unwrap();
        crate::validate_game_descriptors(&scene, 4).unwrap();
        assert!(
            validate_physics_joints(&scene, |_| None)
                .unwrap_err()
                .contains("target anchor is not in the scene")
        );
        assert!(
            validate_physics_joints(&scene, |_| Some(bob))
                .unwrap_err()
                .contains("targets its own object")
        );
        joint.kind = PhysicsJointKind::Hinge {
            axis: [0.0, 2.0, 0.0],
            min_angle_rad: -1.0,
            max_angle_rad: 1.0,
        };
        scene.insert_component(bob, joint).unwrap();
        assert!(validate_physics_joints(&scene, resolve).is_err());
        assert!(
            crate::validate_game_descriptors(&scene, 4)
                .unwrap_err()
                .to_string()
                .contains("invalid game.physics-joint.v1 descriptor")
        );
    }
}
