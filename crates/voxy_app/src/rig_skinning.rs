//! Rigid dual-quaternion skinning preserves rotational volume at blended joints.
use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy)]
pub(crate) struct RigidSkinTransform {
    real: Quat,
    dual: Quat,
}
impl RigidSkinTransform {
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (_, real, translation) = matrix.to_scale_rotation_translation();
        let real = real.normalize();
        let dual = Quat::from_xyzw(translation.x, translation.y, translation.z, 0.) * real * 0.5;
        Self { real, dual }
    }
}
pub(crate) fn deform_point(
    p: Vec3,
    weights: &[(usize, f32)],
    palette: &[RigidSkinTransform],
) -> Vec3 {
    let reference = palette[weights
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("nonempty skin weights")
        .0]
        .real;
    let mut real = Quat::from_xyzw(0., 0., 0., 0.);
    let mut dual = real;
    for &(index, weight) in weights {
        if weight == 0. {
            continue;
        }
        let transform = palette[index];
        let signed_weight = if reference.dot(transform.real) < 0. {
            -weight
        } else {
            weight
        };
        real = real + transform.real * signed_weight;
        dual = dual + transform.dual * signed_weight;
    }
    let length = real.length();
    real = real / length;
    dual = dual / length;
    dual = dual - real * real.dot(dual);
    let translation = dual * real.conjugate() * 2.;
    real * p + Vec3::new(translation.x, translation.y, translation.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blended_rotation_preserves_joint_radius() {
        let palette = [
            RigidSkinTransform::from_matrix(Mat4::IDENTITY),
            RigidSkinTransform::from_matrix(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        ];
        let result = deform_point(Vec3::X, &[(0, 0.5), (1, 0.5), (0, 0.), (0, 0.)], &palette);
        assert!((result.length() - 1.).abs() < 1e-6);
        assert!((result - Vec3::new(0.5f32.sqrt(), 0.5f32.sqrt(), 0.)).length() < 1e-6);
    }
    #[test]
    fn rigid_translation_and_antipodal_quaternions_agree() {
        let matrix =
            Mat4::from_rotation_translation(Quat::from_rotation_y(1.2), Vec3::new(2., -3., 0.5));
        let a = RigidSkinTransform::from_matrix(matrix);
        let b = RigidSkinTransform {
            real: -a.real,
            dual: -a.dual,
        };
        let point = Vec3::new(0.2, 0.3, -0.4);
        assert!(
            (deform_point(point, &[(0, 0.25), (1, 0.75), (0, 0.), (0, 0.)], &[a, b])
                - matrix.transform_point3(point))
            .length()
                < 1e-6
        );
    }
}
