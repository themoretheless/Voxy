//! Directed projection gap for an arbitrary exact stored separating axis.
//! Generated axes need not be mathematically unit: normalize the distance bound.
use super::*;
#[derive(Clone, Copy)]
struct Interval(f64, f64);
impl Interval {
    fn rounded(lo: f64, hi: f64) -> Result<Self, PhysicsError> {
        let v = Self(lo.next_down(), hi.next_up());
        if !v.0.is_finite() || !v.1.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok(v)
    }
    fn add(self, b: Self) -> Result<Self, PhysicsError> {
        Self::rounded(self.0 + b.0, self.1 + b.1)
    }
    fn sub(self, b: Self) -> Result<Self, PhysicsError> {
        Self::rounded(self.0 - b.1, self.1 - b.0)
    }
    fn mul(self, b: Self) -> Result<Self, PhysicsError> {
        let values = [self.0 * b.0, self.0 * b.1, self.1 * b.0, self.1 * b.1];
        if values.into_iter().any(|v| !v.is_finite()) {
            return Err(PhysicsError::InvalidMotion);
        }
        Self::rounded(
            values.into_iter().fold(f64::INFINITY, f64::min),
            values.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn absolute(self) -> Self {
        Self(
            if self.0 <= 0. && self.1 >= 0. {
                0.
            } else {
                self.0.abs().min(self.1.abs())
            },
            self.0.abs().max(self.1.abs()),
        )
    }
}
/// Discrepancy between enclosed canonical world corners and the exact affine
/// shape defined by the actual stored center/edges. Convex interpolation of
/// corners extends the component and L1 caps to every material point of the box.
pub(super) fn world_pose_error(
    reference: PointBoxes,
    center: DVec3,
    edges: [DVec3; 3],
) -> Result<([f64; 3], f64), PhysicsError> {
    if !center.is_finite() || edges.iter().any(|v| !v.is_finite()) {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut axes = [0_f64; 3];
    for (corner, source) in reference.iter().enumerate() {
        for axis in 0..3 {
            let v = source[axis];
            if !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1] {
                return Err(PhysicsError::InvalidMotion);
            }
            let mut actual = Interval(center[axis], center[axis]);
            for (i, edge) in edges.iter().enumerate() {
                let value = edge[axis] * if corner & (1 << i) == 0 { -1. } else { 1. };
                actual = actual.add(Interval(value, value))?;
            }
            let difference = Interval(v[0], v[1]).sub(actual)?.absolute();
            axes[axis] = axes[axis].max(difference.1);
        }
    }
    let mut radius = Interval(0., 0.);
    for error in axes {
        radius = radius.add(Interval(error, error))?;
    }
    Ok((axes, radius.1))
}

#[cfg(test)]
mod world_error_tests {
    use super::*;
    #[test]
    fn world_corner_discrepancy_covers_exact_affine_shift_and_rejects_bad_boxes() {
        let center = DVec3::new(65536., 2., -3.);
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let delta = DVec3::new(0.125, -0.25, 0.5);
        let points = std::array::from_fn(|i| {
            let point = center
                + delta
                + edges[0] * if i & 1 == 0 { -1. } else { 1. }
                + edges[1] * if i & 2 == 0 { -1. } else { 1. }
                + edges[2] * if i & 4 == 0 { -1. } else { 1. };
            point.to_array().map(|x| [x, x])
        });
        let (axes, radius) = world_pose_error(points, center, edges).unwrap();
        for i in 0..3 {
            assert!(axes[i] >= delta[i].abs() && axes[i] < delta[i].abs() + 1e-8);
        }
        assert!(radius >= 0.875 && radius < 0.875 + 1e-8);
        let mut invalid = points;
        invalid[0][0] = [1., 0.];
        assert!(world_pose_error(invalid, center, edges).is_err());
        assert!(world_pose_error(points, DVec3::splat(f64::NAN), edges).is_err());
        println!("WORLD_POSE_ERROR {:?}", (axes, radius));
    }
}

fn dot(a: [Interval; 3], b: DVec3) -> Result<Interval, PhysicsError> {
    let mut result = Interval(0., 0.);
    for i in 0..3 {
        result = result.add(a[i].mul(Interval(b[i], b[i]))?)?;
    }
    Ok(result)
}
/// Lower world-distance separation for exact stored boxes inflated by clearance.
/// Any positive result is a separation certificate on this axis. Nonpositive
/// results are inconclusive, not a penetration or true-contact certificate.
/// Pose evaluation and the choice of axis remain separate inputs/obligations.
pub(super) fn lower(
    center: DVec3,
    edges: [DVec3; 3],
    obstacle: &AffineBox,
    axis: DVec3,
    clearance: f64,
) -> Result<f64, PhysicsError> {
    if [center, obstacle.center, axis]
        .into_iter()
        .chain(edges)
        .chain(obstacle.edges)
        .any(|v| !v.is_finite())
        || !clearance.is_finite()
        || clearance < 0.
    {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut relative = [Interval(0., 0.); 3];
    for i in 0..3 {
        relative[i] =
            Interval(center[i], center[i]).sub(Interval(obstacle.center[i], obstacle.center[i]))?;
    }
    let mut squared = Interval(0., 0.);
    for v in axis.to_array() {
        squared = squared.add(Interval(v, v).mul(Interval(v, v))?)?;
    }
    let norm = squared.1.sqrt().next_up();
    if !norm.is_finite() || axis == DVec3::ZERO {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut gap = dot(relative, axis)?.absolute();
    for edge in edges.into_iter().chain(obstacle.edges) {
        gap = gap.sub(dot(edge.to_array().map(|v| Interval(v, v)), axis)?.absolute())?;
    }
    gap = gap.sub(Interval(clearance, clearance).mul(Interval(norm, norm))?)?;
    // Only positive values are used to certify clearance/advancement. Returning
    // zero for an unproved positive gap cannot create a false separation.
    if gap.0 <= 0. {
        return Ok(0.);
    }
    let result = (gap.0 / norm).next_down();
    if !result.is_finite() {
        return Err(PhysicsError::InvalidMotion);
    }
    Ok(result.max(0.))
}
/// Directed advancement within a normalized unit interval. The supplied gap
/// must be a lower separation bound and speed an upper point-speed bound.
pub(super) fn advance_time(time: f64, gap: f64, speed: f64) -> Result<f64, PhysicsError> {
    if !time.is_finite()
        || !(0. ..=1.).contains(&time)
        || !gap.is_finite()
        || gap < 0.
        || !speed.is_finite()
        || speed <= 0.
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if gap == 0. {
        return Ok(time);
    }
    let numerator = (0.8 * gap).next_down().max(0.);
    let step = (numerator / speed).next_down().max(0.);
    // Overflow to infinity rounds downward to MAX, which still bounds the
    // unbounded exact sum from below. Clamping the result to 1 is then safe.
    Ok((time + step).next_down().max(time).min(1.))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advancement_never_exceeds_the_exact_gap_over_speed_step() {
        for (time, gap, speed) in [
            (0., 0.3, 0.7),
            (0.4, 0.2, 1.3),
            (0.9, 0.7, 0.1),
            (0.2, 1e-300, 1e300),
            (0.2, 1e300, 1e-300),
            (1., 0.5, 0.3),
        ] {
            let next = advance_time(time, gap, speed).unwrap();
            assert!(next >= time && next <= 1.);
            println!("ADVANCE_TIME_ENCLOSURE {:?}", (time, gap, speed, next));
        }
        assert_eq!(advance_time(0.4, 0., 1.).unwrap(), 0.4);
        assert!(advance_time(0., 1., 0.).is_err());
        assert!(advance_time(f64::NAN, 1., 1.).is_err());
    }
    #[test]
    fn projection_gap_accounts_for_axis_norm_cancellation_and_clearance() {
        let obstacle = AffineBox {
            center: DVec3::new(1e8, 0., 0.),
            edges: [DVec3::X * 0.1, DVec3::Y * 0.3, DVec3::Z * 0.2],
        };
        let edges = [DVec3::X * 0.2, DVec3::Y * 0.1, DVec3::Z * 0.1];
        for axis in [DVec3::X, DVec3::X * 2., DVec3::new(1., 0.2, -0.3)] {
            for clearance in [0., 0.01, 0.05] {
                let center = DVec3::new(1e8 + 0.8, 0.2, -0.1);
                let result = lower(center, edges, &obstacle, axis, clearance).unwrap();
                println!(
                    "GAP_ENCLOSURE {:?}",
                    (
                        center.to_array(),
                        edges.map(|v| v.to_array()),
                        obstacle.center.to_array(),
                        obstacle.edges.map(|v| v.to_array()),
                        axis.to_array(),
                        clearance,
                        result
                    )
                );
                assert!(result > 0.);
            }
        }
        assert_eq!(
            lower(obstacle.center, edges, &obstacle, DVec3::X, 0.01).unwrap(),
            0.
        );
        assert!(lower(obstacle.center, edges, &obstacle, DVec3::ZERO, 0.).is_err());
        assert!(lower(obstacle.center, edges, &obstacle, DVec3::X, -1.).is_err());
    }
}

/// Positive separation certificate directly from enclosing world corner boxes.
/// Avoids interpreting a rounded pose as exact geometry. Axis selection remains
/// arbitrary: any nonzero stored direction can certify a positive projection gap.
pub(super) fn lower_points(
    points: &[[[f64; 2]; 3]; 8],
    obstacle: &AffineBox,
    axis: DVec3,
    clearance: f64,
) -> Result<f64, PhysicsError> {
    if points
        .iter()
        .flatten()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        || !axis.is_finite()
        || axis == DVec3::ZERO
        || !obstacle.center.is_finite()
        || obstacle.edges.into_iter().any(|v| !v.is_finite())
        || !clearance.is_finite()
        || clearance < 0.
    {
        return Err(PhysicsError::InvalidMotion);
    }
    let center = dot(obstacle.center.to_array().map(|v| Interval(v, v)), axis)?;
    let mut support = Interval(0., 0.);
    for edge in obstacle.edges {
        support = support.add(dot(edge.to_array().map(|v| Interval(v, v)), axis)?.absolute())?;
    }
    let mut squared = Interval(0., 0.);
    for v in axis.to_array() {
        squared = squared.add(Interval(v, v).mul(Interval(v, v))?)?;
    }
    let norm = squared.1.sqrt().next_up();
    if !norm.is_finite() {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for point in points {
        let projection = dot(point.map(|v| Interval(v[0], v[1])), axis)?;
        lo = lo.min(projection.0);
        hi = hi.max(projection.1);
    }
    let dilation = Interval(clearance, clearance).mul(Interval(norm, norm))?;
    let positive = Interval(lo, lo).sub(center)?.sub(support)?.sub(dilation)?.0;
    let negative = center.sub(Interval(hi, hi))?.sub(support)?.sub(dilation)?.0;
    let gap = positive.max(negative);
    if gap <= 0. {
        return Ok(0.);
    }
    Ok((gap / norm).next_down().max(0.))
}

#[cfg(test)]
mod point_tests {
    use super::*;
    #[test]
    fn enclosing_corner_gaps_do_not_assume_an_exact_rounded_pose() {
        let obstacle = AffineBox {
            center: DVec3::new(2., 0.1, -0.2),
            edges: [DVec3::X * 0.1, DVec3::Y * 0.3, DVec3::Z * 0.2],
        };
        let points = std::array::from_fn::<_, 8, _>(|corner| {
            std::array::from_fn(|i| {
                let coordinate = if corner & (1 << i) == 0 { -0.2 } else { 0.2 };
                [coordinate - 0.01, coordinate + 0.01]
            })
        });
        for axis in [DVec3::X, DVec3::X * 2., DVec3::new(1., 0.2, -0.3)] {
            let gap = lower_points(&points, &obstacle, axis, 0.05).unwrap();
            assert!(gap > 0.);
            println!(
                "POINT_GAP_ENCLOSURE {:?}",
                (
                    points,
                    obstacle.center.to_array(),
                    obstacle.edges.map(|v| v.to_array()),
                    axis.to_array(),
                    0.05,
                    gap
                )
            );
        }
        let touching = [[[1.8, 2.2], [-0.2, 0.2], [-0.2, 0.2]]; 8];
        assert_eq!(
            lower_points(&touching, &obstacle, DVec3::X, 0.).unwrap(),
            0.
        );
    }
}

/// Certifies disjoint interiors of a proposed stored physical pose against every
/// obstacle. Interval separation is the fast path; an exact dyadic fallback can
/// prove projection touching. Continuous support trajectories remain separate.
pub(super) fn certify_pose(
    center: DVec3,
    edges: [DVec3; 3],
    boxes: &[AffineBox],
    queries: &mut usize,
) -> Result<(), PhysicsError> {
    if !center.is_finite() || edges.into_iter().any(|v| !v.is_finite()) {
        return Err(PhysicsError::InvalidMotion);
    }
    for obstacle in boxes {
        query(queries)?;
        let mut separated = false;
        for axis in obstacle.axes_for(edges) {
            if lower(center, edges, obstacle, axis, 0.).is_ok_and(|gap| gap > 0.)
                || exact_gap::sign(center, edges, obstacle, axis)? >= 0
            {
                separated = true;
                break;
            }
        }
        if !separated {
            return Err(PhysicsError::InvalidMotion);
        }
    }
    Ok(())
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    #[test]
    fn proposed_pose_requires_proven_separation_and_counts_queries() {
        let edges = [DVec3::X * 0.1, DVec3::Y * 0.2, DVec3::Z * 0.3];
        let wall = AffineBox {
            center: DVec3::X,
            edges,
        };
        let mut remaining = 2;
        certify_pose(DVec3::ZERO, edges, &[wall], &mut remaining).unwrap();
        assert_eq!(remaining, 1);
        // A rounded proposal can overlap even when the canonical trajectory
        // checkpoint was clear. It must be rejected as a separate candidate.
        assert!(matches!(
            certify_pose(DVec3::X * 0.9, edges, &[wall], &mut remaining),
            Err(PhysicsError::InvalidMotion)
        ));
        assert_eq!(remaining, 0);
        assert!(matches!(
            certify_pose(DVec3::ZERO, edges, &[wall], &mut 0),
            Err(PhysicsError::SweepBudget)
        ));
        assert!(certify_pose(DVec3::X * 0.8, edges, &[wall], &mut 1).is_err());
        let exact_edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let exact_wall = AffineBox {
            center: DVec3::X,
            edges: exact_edges,
        };
        certify_pose(DVec3::X * 0.75, exact_edges, &[exact_wall], &mut 1).unwrap();
        let huge_wall = AffineBox {
            center: -DVec3::splat(f64::MAX),
            edges,
        };
        certify_pose(DVec3::splat(f64::MAX), edges, &[huge_wall], &mut 1).unwrap();
        assert!(
            certify_pose(
                DVec3::X * 0.75_f64.next_up(),
                exact_edges,
                &[exact_wall],
                &mut 1
            )
            .is_err()
        );
        assert!(certify_pose(DVec3::splat(f64::NAN), edges, &[], &mut 0).is_err());
    }
}
