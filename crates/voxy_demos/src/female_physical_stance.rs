//! Fixed stance preview using the existing accepted character transaction.
use crate::female_rig::FemaleRig;
use glam::{DMat4, DVec3, Quat, Vec3};
use voxy_animation::{AnimatorFrame, Pose};
use voxy_render::SceneVertex;

pub const STEP: f64 = 1. / 120.;
#[derive(Debug)]
pub struct PhysicalStance {
    scene: voxy_scene::SceneGraph,
    physics: voxy_gameplay::CharacterPhysics,
    input: voxy_input::InputMap,
    feet: voxy_editor::FootRuntime,
    owner: voxy_scene::NodeId,
    center: DVec3,
    pub pose: Pose,
    pub palette: Vec<glam::Mat4>,
    pub offset: DVec3,
    pub floor_height: f64,
}
impl PhysicalStance {
    pub fn new(rig: &FemaleRig, vertices: &[SceneVertex]) -> Result<Self, String> {
        let bindings = rig.foot_support_geometry(vertices, 0.02)?;
        let globals = rig
            .skeleton
            .bind_pose64()
            .global_matrices(&rig.skeleton)
            .map_err(|e| e.to_string())?;
        let mut minimum = f64::INFINITY;
        for (bone, geometry) in &bindings {
            for point in &geometry.points {
                minimum = minimum.min(
                    globals[usize::from(*bone)]
                        .transform_point3(DVec3::from_array(*point))
                        .y,
                );
            }
        }
        let mut scene = voxy_scene::SceneGraph::new(2);
        let floor_y = (minimum + 0.01 - 0.1) as f32;
        let floor = scene
            .spawn(
                None,
                voxy_scene::Transform {
                    translation: Vec3::Y * floor_y,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        scene
            .insert_component(
                floor,
                voxy_gameplay::BoxCollider {
                    half_extents: [4., 0.1, 4.],
                },
            )
            .map_err(|e| e.to_string())?;
        let floor_height = f64::from(floor_y) + f64::from(0.1f32);
        let center = Vec3::Y * (floor_height + 0.5) as f32;
        let owner = scene
            .spawn(
                None,
                voxy_scene::Transform {
                    translation: center,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        scene
            .insert_component(
                owner,
                voxy_gameplay::CharacterBody {
                    half_extents: [0.2, 0.5, 0.2],
                    gravity: -9.81,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
        let physics = voxy_gameplay::CharacterPhysics::new(&scene, 1, 1).with_depenetration(true);
        let feet = rig.fixed_stance_foot_runtime(
            vertices,
            voxy_gameplay::FootContactSettings::default(),
            0.02,
            [true, true],
        )?;
        Ok(Self {
            scene,
            physics,
            input: voxy_input::InputMap::new(8, 8),
            feet,
            owner,
            center: center.as_dvec3(),
            pose: rig.skeleton.bind_pose(),
            palette: rig
                .skeleton
                .bind_pose()
                .skin_matrices(&rig.skeleton)
                .map_err(|e| e.to_string())?,
            offset: DVec3::ZERO,
            floor_height,
        })
    }
    pub fn step(&mut self, rig: &FemaleRig, time: f64) -> Result<(), String> {
        if !time.is_finite() || time < 0. {
            return Err("invalid physical stance time".into());
        }
        let phase = time.rem_euclid(1.) as f32;
        let mut pose = rig.skeleton.bind_pose();
        let compression = 0.05 * (std::f32::consts::PI * phase).sin().powi(2);
        let pelvis = pose.local()[0].translation - Vec3::Y * compression;
        pose.set_joint_translation(0, pelvis)
            .map_err(|e| e.to_string())?;
        for (index, joint) in rig.skeleton.joints().iter().enumerate() {
            let sign = match joint.name.as_ref() {
                "left_foot" => 1.,
                "right_foot" => -1.,
                _ => continue,
            };
            pose.set_joint_rotation(
                index,
                Quat::from_rotation_x(sign * 0.25 * (std::f32::consts::TAU * phase).sin()),
            )
            .map_err(|e| e.to_string())?;
        }
        let source = AnimatorFrame {
            properties: Default::default(),
            skin_matrices: pose
                .skin_matrices(&rig.skeleton)
                .map_err(|e| e.to_string())?,
            pose,
            root_motion: Vec3::ZERO,
            root_motion_joint: 0,
            transition_weight: 1.,
        };
        let (_, (feet, frame, offset)) = self
            .physics
            .fixed_step_with_preparation(
                &mut self.scene,
                &mut self.input,
                STEP,
                &[],
                &[],
                |preview, budget| {
                    let actor = preview
                        .characters
                        .iter()
                        .find(|p| p.owner == self.owner)
                        .ok_or("missing physical stance actor")?;
                    let offset = actor.physical_center - self.center;
                    let mut feet = self.feet.clone();
                    let frame = feet.correct_fixed_stance(
                        source,
                        DMat4::from_translation(offset),
                        actor.grounded,
                        preview,
                        budget,
                    )?;
                    if !actor.grounded
                        || (0..2).any(|index| feet.support_lock(index).ok().flatten().is_none())
                    {
                        return Err("physical stance lost accepted support".to_string());
                    }
                    Ok((feet, frame, offset))
                },
            )
            .map_err(|e| format!("physical stance tick rejected: {e:?}"))?;
        self.feet = feet;
        self.palette = frame.skin_matrices;
        self.pose = frame.pose;
        self.offset = offset;
        Ok(())
    }
}

/// Computes the signed Zero Moment Point (ZMP) stability margin in meters relative to an
/// axis-aligned biped support polygon `[min_x, max_x] x [min_z, max_z]`.
/// Positive values indicate the ZMP lies strictly inside the support footprint; negative values
/// measure distance outside the boundary.
#[must_use]
pub fn zmp_stability_margin(
    com_pos: [f64; 3],
    com_accel: [f64; 3],
    gravity_y: f64,
    support_min_xz: [f64; 2],
    support_max_xz: [f64; 2],
) -> Option<f64> {
    if com_pos.iter().chain(&com_accel).any(|v| !v.is_finite())
        || !gravity_y.is_finite()
        || gravity_y <= 0.0
        || support_min_xz[0] > support_max_xz[0]
        || support_min_xz[1] > support_max_xz[1]
    {
        return None;
    }
    let denom = gravity_y + com_accel[1];
    if denom.abs() < 1e-6 {
        return None;
    }
    let zmp_x = com_pos[0] - (com_pos[1] * com_accel[0]) / denom;
    let zmp_z = com_pos[2] - (com_pos[1] * com_accel[2]) / denom;
    let margin_x = (zmp_x - support_min_xz[0]).min(support_max_xz[0] - zmp_x);
    let margin_z = (zmp_z - support_min_xz[1]).min(support_max_xz[1] - zmp_z);
    Some(margin_x.min(margin_z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zmp_stability_margin_is_positive_inside_support_and_negative_under_high_acceleration() {
        let rest = zmp_stability_margin(
            [0.0, 0.9, 0.0],
            [0.0, 0.0, 0.0],
            9.81,
            [-0.12, -0.10],
            [0.12, 0.15],
        )
        .unwrap();
        assert!((rest - 0.10).abs() < 1e-9);
        let tipping = zmp_stability_margin(
            [0.0, 0.9, 0.0],
            [3.5, 0.0, 0.0],
            9.81,
            [-0.12, -0.10],
            [0.12, 0.15],
        )
        .unwrap();
        assert!(tipping < 0.0);
    }
}
