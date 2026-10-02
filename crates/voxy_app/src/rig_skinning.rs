//! Rigid dual-quaternion skinning preserves rotational volume at blended joints.
use glam::{Mat4, Quat, Vec3};

/// Unsigned hinge angle and its gradient, evaluated through a signed dihedral.
/// Wrapping finite differences avoids the false zero gradient at a pi fold.
/// Degenerate faces produce no constraint rather than non-finite corrections.
fn signed_hinge_angle(p: [Vec3; 4]) -> Option<f32> {
    let edge = (p[1] - p[0]).try_normalize()?;
    let a = (p[1] - p[0]).cross(p[2] - p[0]).try_normalize()?;
    let b = (p[0] - p[1]).cross(p[3] - p[1]).try_normalize()?;
    let angle = edge.dot(a.cross(b)).atan2(a.dot(b));
    angle.is_finite().then_some(angle)
}
pub(crate) fn hinge_angle(p: [Vec3; 4]) -> Option<f32> {
    signed_hinge_angle(p).map(f32::abs)
}
pub(crate) fn hinge_angle_gradient(p: [Vec3; 4], epsilon: f32) -> Option<(f32, [Vec3; 4])> {
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return None;
    }
    let angle = signed_hinge_angle(p)?;
    let mut gradients = [Vec3::ZERO; 4];
    for j in 0..4 {
        for axis in 0..3 {
            let mut plus = p;
            let mut minus = p;
            plus[j][axis] += epsilon;
            minus[j][axis] -= epsilon;
            let difference = signed_hinge_angle(plus)? - signed_hinge_angle(minus)?;
            let wrapped = (difference + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            gradients[j][axis] = wrapped * angle.signum() / (2.0 * epsilon);
        }
        if !gradients[j].is_finite() {
            return None;
        }
    }
    Some((angle.abs(), gradients))
}

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
    fn fully_folded_hinge_has_a_finite_descent_direction() {
        let points = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Y];
        let (angle, gradient) = hinge_angle_gradient(points, 1e-4).unwrap();
        assert!((angle - std::f32::consts::PI).abs() < 1e-6);
        let norm: f32 = gradient.iter().map(|g| g.length_squared()).sum();
        assert!(norm.is_finite() && norm > 1.0);
        let moved = std::array::from_fn(|i| points[i] - gradient[i] * (0.1 / norm));
        let (after, _) = hinge_angle_gradient(moved, 1e-4).unwrap();
        assert!(after < angle - 0.05);
        let sum: Vec3 = gradient.into_iter().sum();
        assert!(sum.length() < 0.01);
        assert!(hinge_angle_gradient([Vec3::ZERO; 4], 1e-4).is_none());
        assert!(hinge_angle_gradient(points, f32::NAN).is_none());
    }
    #[test]
    fn hinge_gradient_mirrors_as_a_position_displacement() {
        let points = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(0.2, -0.5, 0.8)];
        let mirror = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
        let (angle, gradient) = hinge_angle_gradient(points, 1e-4).unwrap();
        let (other, reflected) = hinge_angle_gradient(points.map(mirror), 1e-4).unwrap();
        assert!((angle - other).abs() < 1e-6);
        for (a, b) in gradient.into_iter().zip(reflected) {
            assert!(mirror(a).abs_diff_eq(b, 0.003));
        }
    }
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
