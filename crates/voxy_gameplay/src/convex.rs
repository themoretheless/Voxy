//! Continuous SAT for a translating axis-aligned character against affine boxes.
//! Static boxes may rotate, scale and inherit shear; normals remain continuous.
use glam::DVec3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct AffineBox {
    pub center: DVec3,
    pub edges: [DVec3; 3],
}
impl AffineBox {
    pub fn axes(&self) -> impl Iterator<Item = DVec3> {
        let mut axes = [DVec3::ZERO; 15];
        axes[..3].copy_from_slice(&[DVec3::X, DVec3::Y, DVec3::Z]);
        let mut count = 3;
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            axes[count] = self.edges[a].cross(self.edges[b]);
            count += 1;
        }
        for world in [DVec3::X, DVec3::Y, DVec3::Z] {
            for edge in self.edges {
                axes[count] = world.cross(edge);
                count += 1;
            }
        }
        axes.into_iter()
            .filter(|axis| axis.length_squared() > 1e-20)
            .map(DVec3::normalize)
    }
    fn radius(&self, axis: DVec3) -> f64 {
        self.edges.iter().map(|edge| edge.dot(axis).abs()).sum()
    }
    pub fn penetration(&self, center: DVec3, half: DVec3) -> Option<DVec3> {
        let relative = center - self.center;
        let mut minimum = f64::INFINITY;
        let mut normal = DVec3::ZERO;
        for axis in self.axes() {
            let distance = relative.dot(axis);
            let overlap = self.radius(axis) + half.dot(axis.abs()) - distance.abs();
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
    pub fn sweep(&self, center: DVec3, half: DVec3, displacement: DVec3) -> Option<(f64, DVec3)> {
        let relative = center - self.center;
        let mut enter = 0.;
        let mut exit: f64 = 1.;
        let mut normal = DVec3::ZERO;
        for axis in self.axes() {
            let radius = self.radius(axis) + half.dot(axis.abs());
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

pub(crate) fn recover(
    center: &mut DVec3,
    half: DVec3,
    boxes: &[AffineBox],
) -> Result<(), super::PhysicsError> {
    for _ in 0..16 {
        let correction = boxes
            .iter()
            .find_map(|obstacle| obstacle.penetration(*center, half));
        let Some(correction) = correction else {
            return Ok(());
        };
        *center += correction;
    }
    if boxes
        .iter()
        .any(|obstacle| obstacle.penetration(*center, half).is_some())
    {
        Err(super::PhysicsError::InitialOverlap)
    } else {
        Ok(())
    }
}

pub(crate) fn move_body(
    center: &mut DVec3,
    half: DVec3,
    velocity: &mut DVec3,
    dt: f64,
    boxes: &[AffineBox],
) -> bool {
    let mut remaining = *velocity * dt;
    let mut grounded = false;
    for _ in 0..4 {
        if remaining.length_squared() < 1e-20 {
            break;
        }
        let nearest = boxes
            .iter()
            .filter_map(|obstacle| obstacle.sweep(*center, half, remaining))
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
        grounded |= normal.y > 0.5;
    }
    if velocity.y <= 0. {
        let snap = DVec3::new(0., -0.005, 0.);
        if let Some((fraction, _)) = boxes
            .iter()
            .filter_map(|obstacle| obstacle.sweep(*center, half, snap))
            .filter(|(_, normal)| normal.y > 0.5)
            .min_by(|a, b| a.0.total_cmp(&b.0))
        {
            *center += snap * fraction;
            velocity.y = 0.;
            grounded = true;
        }
    }
    grounded
}

#[cfg(test)]
mod tests {
    use super::*;
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
