//! Conservative advancement along a complete fixed-center angular arc.
use super::{PhysicsError, convex::AffineBox};
use glam::{DQuat, DVec3};

#[derive(Debug)]
pub(crate) struct Hit {
    pub fraction: f64,
    pub normal: Option<DVec3>,
}

// Exact extrema of sum_i |a_i cos(t) + b_i sin(t) + c_i| on a bounded arc.
// Split at sign changes, then evaluate endpoints and stationary points.
fn support_max(edges: [DVec3; 3], axis: DVec3, normal: DVec3, angle: f64) -> f64 {
    let end = angle.min(std::f64::consts::TAU);
    let terms = edges.map(|edge| {
        let axial = axis * edge.dot(axis);
        [
            normal.dot(edge - axial),
            normal.dot(axis.cross(edge)),
            normal.dot(axial),
        ]
    });
    let evaluate = |t: f64| {
        terms
            .iter()
            .map(|[a, b, c]| (a * t.cos() + b * t.sin() + c).abs())
            .sum::<f64>()
    };
    let mut cuts = vec![0., end];
    for [a, b, c] in terms {
        let radius = a.hypot(b);
        if radius == 0. || c.abs() > radius {
            continue;
        }
        let offset = b.atan2(a);
        let root = (-c / radius).clamp(-1., 1.).acos();
        for base in [offset - root, offset + root] {
            for period in -1..=2 {
                let t = base + f64::from(period) * std::f64::consts::TAU;
                if t > 0. && t < end {
                    cuts.push(t);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut maximum = cuts.iter().copied().map(evaluate).fold(0_f64, f64::max);
    for interval in cuts.windows(2) {
        let middle = (interval[0] + interval[1]) * 0.5;
        let (mut a, mut b) = (0., 0.);
        for [first, second, constant] in terms {
            let sign = if first * middle.cos() + second * middle.sin() + constant >= 0. {
                1.
            } else {
                -1.
            };
            a += sign * first;
            b += sign * second;
        }
        for period in -2..=4 {
            let t = b.atan2(a) + f64::from(period) * std::f64::consts::PI;
            if t > interval[0] && t < interval[1] {
                maximum = maximum.max(evaluate(t));
            }
        }
    }
    maximum + 64. * f64::EPSILON * edges.iter().map(|edge| edge.length()).sum::<f64>()
}

pub(crate) fn sweep(
    center: DVec3,
    edges: [DVec3; 3],
    angular: DVec3,
    boxes: &[AffineBox],
    iterations: usize,
) -> Result<Hit, PhysicsError> {
    let angle = angular.length();
    if angle == 0. || boxes.is_empty() {
        return Ok(Hit {
            fraction: 1.,
            normal: None,
        });
    }
    let axis = angular / angle;
    let mut radius = 0_f64;
    let mut rotation_radius = 0_f64;
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [-1., 1.] {
                let corner = edges[0] * x + edges[1] * y + edges[2] * z;
                radius = radius.max(corner.length());
                rotation_radius = rotation_radius.max(corner.cross(axis).length());
            }
        }
    }
    let mut candidates = Vec::new();
    for obstacle in boxes {
        let relative = center - obstacle.center;
        let epsilon = 128.
            * f64::EPSILON
            * (1. + center.abs().max_element() + obstacle.center.abs().max_element() + radius);
        let separated = obstacle.axes_for(edges).any(|normal| {
            let space = relative.dot(normal).abs() - obstacle.radius(normal);
            space >= radius || space - support_max(edges, axis, normal, angle) >= -epsilon
        });
        if !separated {
            candidates.push(obstacle);
        }
    }
    if candidates.is_empty() {
        return Ok(Hit {
            fraction: 1.,
            normal: None,
        });
    }
    let speed_bound = angle * rotation_radius;
    let mut time = 0.;
    for _ in 0..iterations {
        let rotation = DQuat::from_axis_angle(axis, angle * time);
        let current = edges.map(|edge| rotation * edge);
        let mut distance = f64::INFINITY;
        let mut contact = DVec3::ZERO;
        let mut tolerance = 0.;
        for obstacle in &candidates {
            let relative = center - obstacle.center;
            let mut separation = f64::NEG_INFINITY;
            let mut normal = DVec3::ZERO;
            for axis in obstacle.axes_for(current) {
                let gap = relative.dot(axis).abs()
                    - obstacle.radius(axis)
                    - current.iter().map(|edge| edge.dot(axis).abs()).sum::<f64>();
                if gap > separation {
                    separation = gap;
                    normal = axis * if relative.dot(axis) < 0. { -1. } else { 1. };
                }
            }
            if separation < distance {
                distance = separation;
                contact = normal;
                tolerance = 128.
                    * f64::EPSILON
                    * (1.
                        + center.abs().max_element()
                        + obstacle.center.abs().max_element()
                        + radius)
                    + 1e-8 * rotation_radius;
            }
        }
        if distance <= tolerance {
            return Ok(Hit {
                fraction: time,
                normal: Some(contact),
            });
        }
        // A projection gap is a lower bound on Euclidean separation. Every body
        // point travels at most angle*radius over the normalized unit interval.
        let next = (time + 0.8 * distance / speed_bound).min(1.);
        if next >= 1. {
            return Ok(Hit {
                fraction: 1.,
                normal: None,
            });
        }
        if next <= time {
            return Err(PhysicsError::SweepBudget);
        }
        time = next;
    }
    Err(PhysicsError::SweepBudget)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body() -> [DVec3; 3] {
        [DVec3::X * 0.4, DVec3::Y * 0.1, DVec3::Z * 0.02]
    }
    fn wall() -> AffineBox {
        AffineBox {
            center: DVec3::Z * 0.25,
            edges: [DVec3::X * 2., DVec3::Y * 2., DVec3::Z * 0.02],
        }
    }
    #[test]
    fn middle_of_half_and_full_turn_hits_even_when_endpoints_are_clear() {
        let edges = body();
        let obstacle = wall();
        let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
        for turn in [
            std::f64::consts::PI,
            std::f64::consts::TAU,
            4. * std::f64::consts::TAU,
        ] {
            let final_edges = edges.map(|edge| DQuat::from_rotation_y(turn) * edge);
            assert!(obstacle.penetration_affine(DVec3::ZERO, edges).is_none());
            assert!(
                obstacle
                    .penetration_affine(DVec3::ZERO, final_edges)
                    .is_none()
            );
            let hit = sweep(DVec3::ZERO, edges, DVec3::Y * turn, &[obstacle], 256).unwrap();
            assert!((hit.fraction * turn - angle).abs() < 1e-7, "{hit:?}");
            assert!(hit.fraction * turn <= angle);
            assert!(
                obstacle
                    .penetration_affine(
                        DVec3::ZERO,
                        edges.map(|edge| DQuat::from_rotation_y(turn * hit.fraction) * edge)
                    )
                    .is_none()
            );
        }
    }
    #[test]
    fn grounded_yaw_and_turning_away_are_certified_over_the_whole_arc() {
        let floor = AffineBox {
            center: DVec3::Y * -0.1,
            edges: [DVec3::X * 2., DVec3::Y * 0.1, DVec3::Z * 2.],
        };
        assert_eq!(
            sweep(
                DVec3::Y * 0.1,
                body(),
                DVec3::Y * 4. * std::f64::consts::TAU,
                &[floor],
                1
            )
            .unwrap()
            .fraction,
            1.
        );
        let obstacle = wall();
        let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
        let edges = body().map(|edge| DQuat::from_rotation_y(angle) * edge);
        assert_eq!(
            sweep(DVec3::ZERO, edges, -DVec3::Y * angle, &[obstacle], 1)
                .unwrap()
                .fraction,
            1.
        );
    }
    #[test]
    fn projection_extrema_bound_independent_quaternion_samples() {
        for index in 0..20 {
            let axis = DVec3::new(0.3 + f64::from(index) * 0.01, 0.7, -0.2).normalize();
            let normal = DVec3::new(-0.4, 0.1 + f64::from(index) * 0.03, 0.8).normalize();
            let angle = 0.1 + f64::from(index) * 0.7;
            let edges = [DVec3::new(0.4, 0.1, -0.03), DVec3::Y * 0.1, DVec3::Z * 0.02];
            let upper = support_max(edges, axis, normal, angle);
            let mut observed = 0_f64;
            for step in 0..=2000 {
                let rotation = DQuat::from_axis_angle(axis, angle * f64::from(step) / 2000.);
                let support = edges
                    .iter()
                    .map(|edge| (rotation * *edge).dot(normal).abs())
                    .sum::<f64>();
                assert!(support <= upper + 1e-14);
                observed = observed.max(support);
            }
            assert!(upper - observed < 1e-5);
        }
    }
    #[test]
    fn budget_failure_is_explicit_and_tall_yaw_uses_perpendicular_radius() {
        assert!(matches!(
            sweep(
                DVec3::ZERO,
                body(),
                DVec3::Y * std::f64::consts::PI,
                &[wall()],
                1
            ),
            Err(PhysicsError::SweepBudget)
        ));
        let mut edges = body();
        edges[1] = DVec3::Y * 1000.;
        let hit = sweep(
            DVec3::ZERO,
            edges,
            DVec3::Y * std::f64::consts::PI,
            &[wall()],
            256,
        )
        .unwrap();
        assert!((0.1..0.3).contains(&hit.fraction));
    }
}
