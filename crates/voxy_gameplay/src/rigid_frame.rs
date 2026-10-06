//! COM/principal-frame conversion and admitted root-pose publication.
//! Persistent mechanical state and collision decisions remain with the caller.
use crate::PhysicsError;
use glam::{DMat3, DQuat, DVec3, Vec3};
use physics::{
    astrophysics_spin::Spin, contact::ContactBody, gravity::Body, mass_properties::MassProperties,
};
use voxy_scene::{NodeId, SceneGraph, Transform};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidBodyFrame {
    center_root: DVec3,
    center_scaled: DVec3,
    initial_root_affine: DMat3,
    principal_to_root: DQuat,
    initial_principal: DQuat,
    scale: Vec3,
    mass: f64,
    inertia: [f64; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedRigidPose {
    pub pose: Transform,
    /// Bound for the stored root affine transform over points within the supplied
    /// physical radius from COM. Includes root f32 matrix formation, not GPU math.
    pub point_error_m: f64,
    pub center_error_m: f64,
}
impl RigidBodyFrame {
    /// Properties use the admitted root-relative, initial world-oriented frame.
    pub fn new(pose: Transform, properties: MassProperties) -> Result<Self, PhysicsError> {
        pose.matrix()?;
        if pose.scale.to_array().iter().any(|v| *v == 0.)
            || !properties.mass.is_finite()
            || properties.mass <= 0.
            || properties.center.iter().any(|v| !v.is_finite())
            || properties.inertia.iter().flatten().any(|v| !v.is_finite())
            || properties
                .principal_axes
                .iter()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(PhysicsError::InvalidBody);
        }
        let columns = std::array::from_fn::<_, 3, _>(|k| {
            DVec3::new(
                properties.principal_axes[0][k],
                properties.principal_axes[1][k],
                properties.principal_axes[2][k],
            )
        });
        let matrix = DMat3::from_cols(columns[0], columns[1], columns[2]);
        let magnitude = properties
            .inertia
            .iter()
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        if magnitude == 0. || (matrix.determinant() - 1.).abs() > 2e-12 {
            return Err(PhysicsError::InvalidBody);
        }
        for i in 0..3 {
            for j in 0..3 {
                let gram = columns[i].dot(columns[j]);
                let reconstructed: f64 = (0..3)
                    .map(|k| {
                        properties.principal_axes[i][k]
                            * (properties.principal_moments[k] / magnitude)
                            * properties.principal_axes[j][k]
                    })
                    .sum();
                if (gram - if i == j { 1. } else { 0. }).abs() > 2e-12
                    || (reconstructed - properties.inertia[i][j] / magnitude).abs() > 2e-12
                {
                    return Err(PhysicsError::InvalidBody);
                }
            }
        }
        let principal = DQuat::from_mat3(&matrix).normalize();
        let spin = Spin {
            orientation: principal.to_array(),
            angular_momentum: [0.; 3],
            inertia: properties.principal_moments,
        };
        spin.angular_velocity()
            .map_err(|_| PhysicsError::InvalidBody)?;
        let root = pose.rotation.as_dquat().normalize();
        let initial_root_affine = DMat3::from_quat(pose.rotation.as_dquat());
        let inverse = initial_root_affine.inverse();
        if !inverse.is_finite() {
            return Err(PhysicsError::UnsupportedTransform);
        }

        Ok(Self {
            center_root: root.conjugate() * DVec3::from_array(properties.center),
            center_scaled: inverse * DVec3::from_array(properties.center),
            initial_root_affine,

            principal_to_root: (root.conjugate() * principal).normalize(),
            initial_principal: principal,
            scale: pose.scale,
            mass: properties.mass,
            inertia: properties.principal_moments,
        })
    }
    /// Seed from a root pivot's world position/velocity and declared world angular
    /// momentum. Convert velocity to COM using omega cross (COM - pivot).
    pub fn prepare_body(
        self,
        pivot: [f64; 3],
        pivot_velocity: [f64; 3],
        angular_momentum: [f64; 3],
    ) -> Result<ContactBody, PhysicsError> {
        let spin = Spin {
            orientation: self.initial_principal.to_array(),
            angular_momentum,
            inertia: self.inertia,
        };
        let omega = DVec3::from_array(
            spin.angular_velocity()
                .map_err(|_| PhysicsError::InvalidBody)?,
        );
        let root = self.initial_principal * self.principal_to_root.conjugate();
        let offset = root * self.center_root;
        let body = ContactBody {
            motion: Body {
                mass: self.mass,
                position: (DVec3::from_array(pivot) + offset).to_array(),
                velocity: (DVec3::from_array(pivot_velocity) + omega.cross(offset)).to_array(),
            },
            spin: Some(spin),
        };
        body.energy().map_err(|_| PhysicsError::InvalidBody)?;
        Ok(body)
    }
    /// Prepare a representable root pose while keeping the physical COM fixed.
    /// Radius covers physical points after authored scale, measured from COM.
    pub fn prepare_pose(
        self,
        body: ContactBody,
        radius_m: f64,
        max_point_error_m: f64,
    ) -> Result<PreparedRigidPose, PhysicsError> {
        if !radius_m.is_finite()
            || radius_m < 0.
            || !max_point_error_m.is_finite()
            || max_point_error_m < 0.
            || (body.motion.mass - self.mass).abs() > 1e-12 * self.mass
        {
            return Err(PhysicsError::InvalidBody);
        }
        body.energy().map_err(|_| PhysicsError::InvalidBody)?;
        let spin = body.spin.ok_or(PhysicsError::InvalidBody)?;
        if spin.inertia != self.inertia {
            return Err(PhysicsError::InvalidBody);
        }
        let root =
            (DQuat::from_array(spin.orientation) * self.principal_to_root.conjugate()).normalize();
        let center = DVec3::from_array(body.motion.position);
        let pivot = center - root * self.center_root;
        let pose = Transform {
            translation: pivot.as_vec3(),
            rotation: root.as_quat(),
            scale: self.scale,
        };
        let stored = pose.matrix()?;
        // Express the stored scene matrix in scaled root coordinates, the same
        // physical coordinate system used by center_root and radius_m.
        let actual = DMat3::from_cols(
            stored.x_axis.truncate().as_dvec3(),
            stored.y_axis.truncate().as_dvec3(),
            stored.z_axis.truncate().as_dvec3(),
        ) * DMat3::from_diagonal(self.scale.as_dvec3().recip());
        // Rotate the admitted initial affine shape rigidly. The authored f32
        // quaternion may have a norm defect; do not silently replace its shape
        // by the idealized normalized-root geometry in the residual audit.
        let delta =
            (DQuat::from_array(spin.orientation) * self.initial_principal.conjugate()).normalize();
        let desired = DMat3::from_quat(delta) * self.initial_root_affine;
        let center_error =
            (stored.w_axis.truncate().as_dvec3() + actual * self.center_root - center).length();
        let difference = actual - desired;
        let frobenius = difference
            .to_cols_array()
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        let guard = 32.
            * f64::EPSILON
            * (1. + center.abs().max_element() + self.center_root.length() + radius_m);
        let inverse_radius_scale = self
            .initial_root_affine
            .inverse()
            .to_cols_array()
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        let spin_norm_defect =
            2. * (DQuat::from_array(spin.orientation).length_squared() - 1.).abs() * radius_m;
        let error =
            center_error + frobenius * radius_m * inverse_radius_scale + spin_norm_defect + guard;
        if !error.is_finite() || error > max_point_error_m {
            return Err(PhysicsError::CoordinateRange);
        }
        Ok(PreparedRigidPose {
            pose,
            point_error_m: error,
            center_error_m: center_error,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RigidPoseEdit {
    pub node: NodeId,
    pub frame: RigidBodyFrame,
    pub body: ContactBody,
    pub radius_m: f64,
    pub max_point_error_m: f64,
}
/// Prepare all root poses, then use the existing atomic SceneGraph batch.
/// Callers retain ownership/binding validation and commit mechanical states only
/// after success. A later pose failure leaves every scene transform unchanged.
pub fn publish_rigid_poses(
    scene: &mut SceneGraph,
    edits: &[RigidPoseEdit],
) -> Result<Vec<PreparedRigidPose>, PhysicsError> {
    let mut nodes = std::collections::HashSet::new();
    let mut prepared = Vec::with_capacity(edits.len());
    let mut poses = Vec::with_capacity(edits.len());
    for edit in edits {
        if !nodes.insert(edit.node) || scene.local(edit.node)?.scale != edit.frame.scale {
            return Err(PhysicsError::InvalidBody);
        }
        if scene.parent(edit.node)?.is_some() {
            return Err(PhysicsError::UnsupportedDynamicParent);
        }
        let pose = edit
            .frame
            .prepare_pose(edit.body, edit.radius_m, edit.max_point_error_m)?;
        poses.push((edit.node, pose.pose));
        prepared.push(pose);
    }
    scene.set_locals(&poses)?;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(pose: Transform) -> RigidBodyFrame {
        let descriptor = crate::LiquidMassDistribution {
            parts: vec![crate::LiquidMassPart {
                mass_kg: 1.,
                center_m: [0.5, 0., 0.],
                half_edges_m: [[0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
            }],
        };
        RigidBodyFrame::new(pose, descriptor.prepare(1., pose).unwrap()).unwrap()
    }
    #[test]
    fn quarter_turn_moves_pivot_around_fixed_com_and_converts_pivot_velocity() {
        let frame = frame(Transform::default());
        let l = [0., 0., frame.inertia[2] * 2.];
        let mut body = frame.prepare_body([0.; 3], [0., -1., 0.], l).unwrap();
        assert_eq!(body.motion.position, [0.5, 0., 0.]);
        assert!(DVec3::from_array(body.motion.velocity).length() < 1e-14);
        assert!(
            DVec3::from_array(body.point_velocity([0.; 3]).unwrap())
                .abs_diff_eq(DVec3::new(0., -1., 0.), 1e-14)
        );
        body.spin
            .as_mut()
            .unwrap()
            .step([0.; 3], std::f64::consts::FRAC_PI_4)
            .unwrap();
        let prepared = frame.prepare_pose(body, 1., 1e-5).unwrap();
        assert!((prepared.pose.translation.as_dvec3() - DVec3::new(0.5, -0.5, 0.)).length() < 1e-6);
        assert!(prepared.center_error_m < 1e-6);
        assert_eq!(body.motion.position, [0.5, 0., 0.]);
    }
    #[test]
    fn shared_off_center_impulse_spin_and_scene_publication_keep_physical_com() {
        let frame = frame(Transform::default());
        let mut body = frame.prepare_body([0.; 3], [0.; 3], [0.; 3]).unwrap();
        body.apply_point_impulse([0., 0.5, 0.], [1., 0., 0.])
            .unwrap();
        assert_eq!(body.motion.velocity, [1., 0., 0.]);
        assert_eq!(body.spin.unwrap().angular_momentum, [0., 0., -0.5]);
        let path = body
            .prepare_motion(
                [0.; 3],
                [0.; 3],
                0.005,
                physics::spin_path::Config {
                    max_angular_error_rad: 1e-5,
                    min_step_s: 1e-9,
                    max_arcs: 1024,
                    max_trials: 4096,
                },
            )
            .unwrap();
        body = path.end();
        let mut scene = SceneGraph::new(1);
        let node = scene.spawn(None, Transform::default()).unwrap();
        let reports = publish_rigid_poses(
            &mut scene,
            &[RigidPoseEdit {
                node,
                frame,
                body,
                radius_m: 1.,
                max_point_error_m: 1e-5,
            }],
        )
        .unwrap();
        assert_eq!(scene.local(node).unwrap(), reports[0].pose);
        assert!(reports[0].center_error_m < 1e-6);
        assert_ne!(scene.local(node).unwrap().rotation, glam::Quat::IDENTITY);
        assert_eq!(body.motion.position, [0.505, 0., 0.]);
    }
    #[test]
    fn stored_scaled_affine_points_obey_publication_residual_gate() {
        let pose = Transform {
            translation: Vec3::new(4., -3., 2.),
            rotation: glam::Quat::from_rotation_z(0.4),
            scale: Vec3::new(2., 1., 0.5),
        };
        let frame = frame(pose);
        let mut body = frame
            .prepare_body(
                pose.translation.as_dvec3().to_array(),
                [0.; 3],
                [0.001, 0.002, 0.003],
            )
            .unwrap();
        body.spin.as_mut().unwrap().step([0.; 3], 0.005).unwrap();
        let report = frame.prepare_pose(body, 6., 1e-5).unwrap();
        assert_eq!(report.pose.scale, pose.scale);
        let actual = report.pose.matrix().unwrap().as_dmat4();
        let delta = (DQuat::from_array(body.spin.unwrap().orientation)
            * frame.initial_principal.conjugate())
        .normalize();
        let physical = DMat3::from_quat(delta) * frame.initial_root_affine;
        let com = DVec3::from_array(body.motion.position);
        for x in [-1., 0., 1.] {
            for y in [-1., 0., 1.] {
                for z in [-1., 0., 1.] {
                    let point = DVec3::new(x, y, z);
                    let scaled = pose.scale.as_dvec3() * point;
                    let expected = com + physical * (scaled - frame.center_scaled);
                    assert!(
                        (actual.transform_point3(point) - expected).length()
                            <= report.point_error_m
                    );
                }
            }
        }
    }
    #[test]
    fn late_publication_overflow_duplicate_and_parented_owner_preserve_all_poses() {
        let mut scene = SceneGraph::new(3);
        let first = scene.spawn(None, Transform::default()).unwrap();
        let second = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(first), Transform::default()).unwrap();
        let frame = frame(Transform::default());
        let mut a = frame.prepare_body([1., 0., 0.], [0.; 3], [0.; 3]).unwrap();
        let good = RigidPoseEdit {
            node: first,
            frame,
            body: a,
            radius_m: 1.,
            max_point_error_m: 1e-5,
        };
        a.motion.position = [1e100, 0., 0.];
        let bad = RigidPoseEdit {
            node: second,
            body: a,
            ..good
        };
        let poses = [scene.local(first).unwrap(), scene.local(second).unwrap()];
        assert!(publish_rigid_poses(&mut scene, &[good, bad]).is_err());
        assert_eq!(
            [scene.local(first).unwrap(), scene.local(second).unwrap()],
            poses
        );
        assert!(publish_rigid_poses(&mut scene, &[good, good]).is_err());
        assert_eq!(
            [scene.local(first).unwrap(), scene.local(second).unwrap()],
            poses
        );
        assert!(
            publish_rigid_poses(
                &mut scene,
                &[RigidPoseEdit {
                    node: child,
                    ..good
                }]
            )
            .is_err()
        );
        let mut too_far = good;
        too_far.body.motion.position = [100_000.003, 0., 0.];
        assert!(frame.prepare_pose(too_far.body, 1., 1e-6).is_err());
    }
}
