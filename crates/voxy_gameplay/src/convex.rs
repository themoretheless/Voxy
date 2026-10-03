//! Continuous SAT for a translating oriented character against affine boxes.
//! Static boxes may rotate, scale and inherit shear; normals remain continuous.
use glam::DVec3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct AffineBox {
    pub center: DVec3,
    pub edges: [DVec3; 3],
}
impl AffineBox {
    pub(crate) fn axes_for(&self, body: [DVec3; 3]) -> impl Iterator<Item = DVec3> {
        // Normalize directions before crossing: tiny/large extents must not
        // remove separating axes or overflow their cross products.
        fn direction(vector: DVec3) -> DVec3 {
            let scale = vector.abs().max_element();
            if scale == 0. {
                DVec3::ZERO
            } else {
                (vector / scale).normalize()
            }
        }
        let body = body.map(direction);
        let obstacle = self.edges.map(direction);
        let mut axes = [DVec3::ZERO; 15];
        for (index, (a, b)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            axes[index] = body[a].cross(body[b]);
        }
        for (index, (a, b)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
            axes[index + 3] = obstacle[a].cross(obstacle[b]);
        }
        for (i, first) in body.into_iter().enumerate() {
            for (j, second) in obstacle.into_iter().enumerate() {
                axes[6 + i * 3 + j] = first.cross(second);
            }
        }
        axes.into_iter()
            .filter(|axis| *axis != DVec3::ZERO)
            .map(direction)
    }
    pub(crate) fn radius(&self, axis: DVec3) -> f64 {
        self.edges.iter().map(|edge| edge.dot(axis).abs()).sum()
    }
    #[cfg(test)]
    pub fn penetration(&self, center: DVec3, half: DVec3) -> Option<DVec3> {
        self.penetration_affine(center, aligned_edges(half))
    }
    pub fn penetration_affine(&self, center: DVec3, body: [DVec3; 3]) -> Option<DVec3> {
        let relative = center - self.center;
        let mut minimum = f64::INFINITY;
        let mut normal = DVec3::ZERO;
        for axis in self.axes_for(body) {
            let distance = relative.dot(axis);
            let overlap = self.radius(axis)
                + body.iter().map(|edge| edge.dot(axis).abs()).sum::<f64>()
                - distance.abs();
            // Ignore only double precision arithmetic at touching faces.
            let epsilon = 64. * f64::EPSILON * (1. + self.center.abs().max_element());
            if overlap <= epsilon {
                return None;
            }
            if overlap < minimum {
                minimum = overlap;
                normal = axis * if distance < 0. { -1. } else { 1. };
            }
        }
        Some(normal * (minimum + 1e-7))
    }
    #[cfg(test)]
    pub fn sweep(&self, center: DVec3, half: DVec3, displacement: DVec3) -> Option<(f64, DVec3)> {
        self.sweep_affine(center, aligned_edges(half), displacement)
    }
    pub fn sweep_affine(
        &self,
        center: DVec3,
        body: [DVec3; 3],
        displacement: DVec3,
    ) -> Option<(f64, DVec3)> {
        let relative = center - self.center;
        let mut enter = 0.;
        let mut exit: f64 = 1.;
        let mut normal = DVec3::ZERO;
        for axis in self.axes_for(body) {
            let radius =
                self.radius(axis) + body.iter().map(|edge| edge.dot(axis).abs()).sum::<f64>();
            let distance = relative.dot(axis);
            let speed = displacement.dot(axis);
            let epsilon = 64. * f64::EPSILON * (1. + self.center.abs().max_element());
            if distance.abs() >= radius - epsilon && distance * speed >= 0. {
                return None;
            }
            if speed.abs() < 1e-15 {
                if distance.abs() > radius {
                    return None;
                }
                continue;
            }
            let first = (-radius - distance) / speed;
            let second = (radius - distance) / speed;
            let low = first.min(second);
            let high = first.max(second);
            if low >= enter {
                enter = low;
                normal = axis * if speed > 0. { -1. } else { 1. };
            }
            exit = exit.min(high);
            if enter > exit {
                return None;
            }
        }
        if enter > 1. || exit < 0. || normal == DVec3::ZERO {
            None
        } else {
            Some((enter.max(0.), normal))
        }
    }
}

#[cfg(test)]
fn aligned_edges(half: DVec3) -> [DVec3; 3] {
    [DVec3::X * half.x, DVec3::Y * half.y, DVec3::Z * half.z]
}
#[cfg(test)]
pub(crate) fn recover(
    center: &mut DVec3,
    half: DVec3,
    boxes: &[AffineBox],
) -> Result<(), super::PhysicsError> {
    recover_affine(center, aligned_edges(half), boxes)
}
pub(crate) fn recover_affine(
    center: &mut DVec3,
    body: [DVec3; 3],
    boxes: &[AffineBox],
) -> Result<(), super::PhysicsError> {
    for _ in 0..16 {
        let correction = boxes
            .iter()
            .find_map(|obstacle| obstacle.penetration_affine(*center, body));
        let Some(correction) = correction else {
            return Ok(());
        };
        *center += correction;
    }
    if boxes
        .iter()
        .any(|obstacle| obstacle.penetration_affine(*center, body).is_some())
    {
        Err(super::PhysicsError::InitialOverlap)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn move_body(
    center: &mut DVec3,
    half: DVec3,
    velocity: &mut DVec3,
    dt: f64,
    boxes: &[AffineBox],
) -> bool {
    move_body_affine_carrying_velocity(center, aligned_edges(half), velocity, dt, boxes, None)
}

/// A kinematic displacement also removes persistent velocity into its contacts.
pub(crate) fn move_body_affine_carrying_velocity(
    center: &mut DVec3,
    body: [DVec3; 3],
    velocity: &mut DVec3,
    dt: f64,
    boxes: &[AffineBox],
    mut carried: Option<&mut DVec3>,
) -> bool {
    let mut remaining = *velocity * dt;
    let mut grounded = false;
    for _ in 0..4 {
        if remaining.length_squared() < 1e-20 {
            break;
        }
        let nearest = boxes
            .iter()
            .filter_map(|obstacle| obstacle.sweep_affine(*center, body, remaining))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((fraction, normal)) = nearest else {
            *center += remaining;
            break;
        };
        *center += remaining * fraction;
        remaining *= 1. - fraction;
        let into = remaining.dot(normal);
        if into < 0. {
            remaining -= normal * into;
        }
        let into = velocity.dot(normal);
        if into < 0. {
            *velocity -= normal * into;
        }
        if let Some(carried) = carried.as_deref_mut() {
            let into = carried.dot(normal);
            if into < 0. {
                *carried -= normal * into;
            }
        }
        grounded |= normal.y > 0.5;
    }
    if velocity.y <= 0. && carried.as_ref().is_none_or(|carried| carried.y <= 0.) {
        let snap = DVec3::new(0., -0.005, 0.);
        if let Some((fraction, _)) = boxes
            .iter()
            .filter_map(|obstacle| obstacle.sweep_affine(*center, body, snap))
            .filter(|(_, normal)| normal.y > 0.5)
            .min_by(|a, b| a.0.total_cmp(&b.0))
        {
            *center += snap * fraction;
            velocity.y = 0.;
            if let Some(carried) = carried.as_deref_mut() {
                carried.y = carried.y.max(0.);
            }
            grounded = true;
        }
    }
    grounded
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oriented_face_contact_is_analytic_and_invariant_to_extent_scale() {
        let rotation = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
        for scale in [1e-8, 1., 1e8] {
            let obstacle = AffineBox {
                center: DVec3::Z * scale,
                edges: [
                    DVec3::X * 0.1 * scale,
                    DVec3::Y * 0.1 * scale,
                    DVec3::Z * 0.1 * scale,
                ],
            };
            let body = [
                rotation * DVec3::X * 0.4 * scale,
                DVec3::Y * 0.1 * scale,
                rotation * DVec3::Z * 0.05 * scale,
            ];
            let (fraction, normal) = obstacle
                .sweep_affine(DVec3::ZERO, body, DVec3::Z * 2. * scale)
                .unwrap();
            let expected = (1. - 0.2 - 0.05 * std::f64::consts::SQRT_2) / 2.;
            assert!(
                (fraction - expected).abs() < 1e-12,
                "scale={scale} fraction={fraction}"
            );
            assert!(normal.abs_diff_eq(-(rotation * DVec3::Z), 1e-12));
            assert!(
                obstacle
                    .penetration_affine(DVec3::Z * (2. * fraction - 1e-4) * scale, body)
                    .is_none()
            );
            assert!(
                obstacle
                    .penetration_affine(DVec3::Z * (2. * fraction + 1e-4) * scale, body)
                    .is_some()
            );
        }
    }
    #[test]
    fn rotated_thin_box_does_not_collide_with_empty_aabb_corner() {
        let r = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
        let obstacle = AffineBox {
            center: DVec3::ZERO,
            edges: [r * DVec3::X, DVec3::Y * 0.1, r * DVec3::Z * 0.05],
        };
        assert!(
            obstacle
                .penetration(DVec3::new(0.6, 0., 0.6), DVec3::splat(0.02))
                .is_none()
        );
        let (fraction, normal) = obstacle
            .sweep(
                DVec3::new(0., 0., 2.),
                DVec3::splat(0.02),
                DVec3::new(0., 0., -4.),
            )
            .unwrap();
        assert!((0.4..0.6).contains(&fraction));
        assert!(normal.x.abs() > 0.6 && normal.z.abs() > 0.6);
    }
    #[test]
    fn overlap_recovery_and_slope_contact_are_bounded() {
        let r = glam::DQuat::from_rotation_z(0.2);
        let obstacle = AffineBox {
            center: DVec3::ZERO,
            edges: [r * DVec3::X, r * DVec3::Y * 0.1, DVec3::Z],
        };
        let mut center = DVec3::new(0., 0.1, 0.);
        let half = DVec3::splat(0.05);
        recover(&mut center, half, std::slice::from_ref(&obstacle)).unwrap();
        assert!(obstacle.penetration(center, half).is_none());
        let mut velocity = DVec3::new(0., -1., 0.);
        assert!(move_body(
            &mut center,
            half,
            &mut velocity,
            0.1,
            &[obstacle]
        ));
        assert!(center.is_finite());
    }
}
