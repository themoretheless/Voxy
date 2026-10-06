//! Continuous SAT for a translating oriented character against affine boxes.
//! Static boxes may rotate, scale and inherit shear; normals remain continuous.
use glam::DVec3;

/// Exact coordinate row of the normalized real quaternion rotation, if known.
/// Linear component identities characterize signed coordinate-axis mappings;
/// comparisons are exact and never use a near-zero matrix tolerance.
pub(crate) fn rotation_coordinate_preimage(
    rotation: glam::DQuat,
    coordinate: usize,
) -> Option<(usize, f64)> {
    if coordinate >= 3 || !rotation.is_finite() {
        return None;
    }
    let q = rotation.to_array();
    if q.iter().all(|v| *v == 0.) {
        return None;
    }
    if (0..3).all(|i| i == coordinate || q[i] == 0.) {
        return Some((coordinate, 1.));
    }
    if q[coordinate] == 0. && q[3] == 0. {
        return Some((coordinate, -1.));
    }
    for source in 0..3 {
        if source == coordinate {
            continue;
        }
        let third = 3 - coordinate - source;
        let epsilon = if (coordinate, source) == (0, 1)
            || (coordinate, source) == (1, 2)
            || (coordinate, source) == (2, 0)
        {
            1.
        } else {
            -1.
        };
        for sign in [-1., 1.] {
            if q[coordinate] == sign * q[source] && q[third] == -epsilon * sign * q[3] {
                return Some((source, sign));
            }
        }
    }
    None
}
/// Unit-quaternion cross form, with exact structurally known coordinate rows.
/// Preserves support projections even after a coordinate-axis frame permutation.
pub(crate) fn rotate_vector(rotation: glam::DQuat, vector: DVec3) -> DVec3 {
    let q = rotation.xyz();
    let cross = 2. * q.cross(vector);
    let mut mapped = vector + rotation.w * cross + q.cross(cross);
    for coordinate in 0..3 {
        if let Some((source, sign)) = rotation_coordinate_preimage(rotation, coordinate) {
            mapped[coordinate] = sign * vector[source];
        }
    }
    mapped
}
/// B*q*B^-1 rotates q's imaginary vector through B and leaves its scalar fixed.
/// This avoids rounded quaternion products destroying exact coordinate rows.
pub(crate) fn reframe_rotation(basis: glam::DQuat, rotation: glam::DQuat) -> glam::DQuat {
    let vector = rotate_vector(basis, rotation.xyz());
    glam::DQuat::from_xyzw(vector.x, vector.y, vector.z, rotation.w).normalize()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AffineBox {
    pub center: DVec3,
    pub edges: [DVec3; 3],
}
/// One geometric point of the touching polytope, with a world-distance error bound.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AffineContact {
    pub fraction: f64,
    pub normal: DVec3,
    pub point: DVec3,
    pub tolerance: f64,
}

impl AffineBox {
    /// Translation sweep plus a point on both admitted contact surfaces.
    /// The obstacle is stationary; relative-frame callers must transport the point
    /// by the obstacle's actual motion at the returned fraction.
    pub(crate) fn sweep_affine_contact(
        &self,
        center: DVec3,
        edges: [DVec3; 3],
        displacement: DVec3,
    ) -> Result<Option<AffineContact>, super::PhysicsError> {
        let relative = center - self.center;
        if !self.center.is_finite()
            || !relative.is_finite()
            || !displacement.is_finite()
            || !glam::DMat3::from_cols(self.edges[0], self.edges[1], self.edges[2])
                .inverse()
                .is_finite()
            || !glam::DMat3::from_cols(edges[0], edges[1], edges[2])
                .inverse()
                .is_finite()
        {
            return Err(super::PhysicsError::ContactWitness);
        }
        if self.penetration_affine(center, edges).is_some() {
            return Err(super::PhysicsError::InitialOverlap);
        }
        let Some((fraction, normal)) = self.sweep_affine(center, edges, displacement) else {
            return Ok(None);
        };
        let moved = AffineBox {
            center: (center - self.center) + displacement * fraction,
            edges,
        };
        let (point, tolerance) = self.contact_point_relative(&moved, normal)?;
        Ok(Some(AffineContact {
            fraction,
            normal,
            point,
            tolerance,
        }))
    }

    pub(crate) fn contact_point_relative(
        &self,
        other: &AffineBox,
        normal: DVec3,
    ) -> Result<(DVec3, f64), super::PhysicsError> {
        self.contact_patch_and_center_relative(other, normal)
            .map(|(center, _, tolerance)| (center, tolerance))
    }

    /// Admitted vertices of the shared contact patch, in this shape's frame.
    pub(crate) fn contact_patch_relative(
        &self,
        other: &AffineBox,
        normal: DVec3,
    ) -> Result<(Vec<DVec3>, f64), super::PhysicsError> {
        self.contact_patch_and_center_relative(other, normal)
            .map(|(_, points, tolerance)| (points, tolerance))
    }

    fn contact_patch_and_center_relative(
        &self,
        other: &AffineBox,
        normal: DVec3,
    ) -> Result<(DVec3, Vec<DVec3>, f64), super::PhysicsError> {
        use super::PhysicsError;
        // All clipping happens in the obstacle-relative frame, not huge world coordinates.
        let a = AffineBox {
            center: DVec3::ZERO,
            edges: self.edges,
        };
        let b = AffineBox {
            center: other.center,
            edges: other.edges,
        };
        let scale = a
            .edges
            .iter()
            .chain(&b.edges)
            .map(|e| e.abs().max_element())
            .fold(b.center.abs().max_element().max(1.), f64::max);
        let tolerance = 256. * f64::EPSILON * scale;
        fn slabs(shape: &AffineBox) -> Result<[(DVec3, f64); 3], PhysicsError> {
            let inverse =
                glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
            if !inverse.is_finite() {
                return Err(PhysicsError::ContactWitness);
            }
            let rows = inverse.transpose().to_cols_array_2d();
            let mut result = [(DVec3::ZERO, 0.); 3];
            for (slot, row) in result.iter_mut().zip(rows) {
                let row = DVec3::from_array(row);
                let length = row.x.hypot(row.y).hypot(row.z);
                if !length.is_finite() || length == 0. {
                    return Err(PhysicsError::ContactWitness);
                }
                *slot = (row / length, 1. / length);
            }
            Ok(result)
        }
        let planes_a = slabs(&a)?;
        let planes_b = slabs(&b)?;
        fn vertices(shape: &AffineBox) -> [DVec3; 8] {
            std::array::from_fn(|bits| {
                shape.center
                    + (0..3)
                        .map(|k| shape.edges[k] * if bits & (1 << k) == 0 { -1. } else { 1. })
                        .sum::<DVec3>()
            })
        }
        let va = vertices(&a);
        let vb = vertices(&b);
        let contains = |point: DVec3, shape: &AffineBox, planes: &[(DVec3, f64); 3]| {
            planes
                .iter()
                .all(|(axis, half)| axis.dot(point - shape.center).abs() <= half + tolerance)
        };
        let plane = a.radius(normal);
        let other_plane = normal.dot(b.center) - b.radius(normal);
        if !plane.is_finite() || (other_plane - plane).abs() > tolerance {
            return Err(PhysicsError::ContactWitness);
        }
        let mut points = Vec::with_capacity(160);
        let mut admit = |point: DVec3| {
            if point.is_finite()
                && (normal.dot(point) - plane).abs() <= tolerance
                && (normal.dot(point) - other_plane).abs() <= tolerance
                && contains(point, &a, &planes_a)
                && contains(point, &b, &planes_b)
            {
                if !points
                    .iter()
                    .any(|old: &DVec3| (*old - point).abs().max_element() <= tolerance)
                {
                    points.push(point);
                }
            }
        };
        for point in va.into_iter().chain(vb) {
            admit(point);
        }
        for (vertices, obstacle, planes) in [(va, &b, planes_b), (vb, &a, planes_a)] {
            for bits in 0..8 {
                for k in 0..3 {
                    if bits & (1 << k) != 0 {
                        continue;
                    }
                    let start = vertices[bits];
                    let delta = vertices[bits | (1 << k)] - start;
                    for (axis, half) in planes {
                        let speed = axis.dot(delta);
                        if speed == 0. {
                            continue;
                        }
                        for sign in [-1., 1.] {
                            let fraction =
                                (sign * half - axis.dot(start - obstacle.center)) / speed;
                            if fraction.is_finite() && (0. ..=1.).contains(&fraction) {
                                admit(start + delta * fraction);
                            }
                        }
                    }
                }
            }
        }
        if points.is_empty() {
            return Err(PhysicsError::ContactWitness);
        }
        // A convex combination stays inside both shapes; it is not an area-weighted
        // pressure center or a complete contact manifold.
        let count = points.len() as f64;
        let relative: DVec3 = points.iter().map(|p| *p / count).sum();
        if !contains(relative, &a, &planes_a)
            || !contains(relative, &b, &planes_b)
            || (normal.dot(relative) - plane).abs() > tolerance
            || (normal.dot(relative) - other_plane).abs() > tolerance
        {
            return Err(PhysicsError::ContactWitness);
        }
        let point = self.center + relative;
        if !point.is_finite() {
            return Err(PhysicsError::ContactWitness);
        }
        let points: Vec<_> = points.into_iter().map(|p| self.center + p).collect();
        if points.iter().any(|p| !p.is_finite()) {
            return Err(PhysicsError::ContactWitness);
        }
        Ok((
            point,
            points,
            tolerance + 8. * f64::EPSILON * self.center.abs().max_element(),
        ))
    }

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

#[cfg(test)]
mod rotation_tests {
    use super::*;
    #[test]
    fn unit_axis_rotation_preserves_fixed_coordinate_exactly() {
        let point = DVec3::new(0.125, -1000., 0.3);
        for coordinate in 0..3 {
            let mut axis = DVec3::ZERO;
            axis[coordinate] = 1.;
            for angle in [0.2, 0.3, 0.4, 1., 17.] {
                let rotation = glam::DQuat::from_axis_angle(axis, angle).normalize();
                let mapped = rotate_vector(rotation, point);
                assert_eq!(mapped[coordinate].to_bits(), point[coordinate].to_bits());
                assert!((mapped.length() - point.length()).abs() < 2e-12);
            }
        }
    }
}

#[cfg(test)]
mod coordinate_mapping_tests {
    use super::*;
    #[test]
    fn signed_coordinate_rows_are_exact_without_rounded_matrix_comparison() {
        let vector = DVec3::new(0.125, -1000., 0.3);
        for rotation in [
            glam::DQuat::from_xyzw(0., 0., 0.5, 0.5).normalize(),
            glam::DQuat::from_xyzw(0.2, 0.2, 0.3, 0.3).normalize(),
            glam::DQuat::from_xyzw(0.2, -0.2, 0.3, -0.3).normalize(),
            glam::DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5),
            glam::DQuat::from_xyzw(0.3, 0.4, 0., 0.).normalize(),
        ] {
            let mapped = rotate_vector(rotation, vector);
            for coordinate in 0..3 {
                if let Some((source, sign)) = rotation_coordinate_preimage(rotation, coordinate) {
                    assert_eq!(
                        mapped[coordinate].to_bits(),
                        (sign * vector[source]).to_bits()
                    );
                    println!(
                        "EXACT_COORDINATE_ROW {:?}",
                        (rotation.to_array(), coordinate, source, sign)
                    );
                }
            }
        }
        let changed = glam::DQuat::from_xyzw(0.2_f64.next_up(), 0.2, 0.3, 0.3).normalize();
        assert_eq!(rotation_coordinate_preimage(changed, 1), None);
    }
}

#[cfg(test)]
mod contact_witness_tests {
    use super::*;
    #[test]
    fn clipped_face_patch_drives_shared_normal_manifold_without_collapsing_to_centroid() {
        let wall = box_shape(DVec3::ZERO, DVec3::splat(0.5));
        let shape = box_shape(DVec3::new(-1., 0., 0.), DVec3::splat(0.5));
        let normal = -DVec3::X;
        let (points, tolerance) = wall.contact_patch_relative(&shape, normal).unwrap();
        assert_eq!(points.len(), 4);
        for &point in &points {
            on_shapes(
                wall,
                shape,
                AffineContact {
                    fraction: 0.,
                    point,
                    normal,
                    tolerance,
                },
            );
        }
        let contacts: Vec<_> = points
            .iter()
            .map(|p| physics::contact::NormalContact {
                point: p.to_array(),
                normal: normal.to_array(),
            })
            .collect();
        let mut body = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: shape.center.to_array(),
                velocity: [2., 0., 0.],
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., 0.5],
                inertia: [1.; 3],
            }),
        };
        let report = physics::contact::resolve_normal_manifold(
            &mut body,
            None,
            &contacts,
            physics::contact::ManifoldConfig {
                max_sweeps: 1000,
                velocity_tolerance: 1e-10,
            },
        )
        .unwrap();
        assert!(body.motion.velocity[0].abs() < 1e-10);
        assert!(
            body.spin
                .unwrap()
                .angular_momentum
                .iter()
                .all(|v| v.abs() < 2e-10)
        );
        assert!(report.kinetic_energy_change < 0.);
    }
    fn box_shape(center: DVec3, half: DVec3) -> AffineBox {
        AffineBox {
            center,
            edges: [DVec3::X * half.x, DVec3::Y * half.y, DVec3::Z * half.z],
        }
    }
    fn on_shapes(a: AffineBox, b: AffineBox, hit: AffineContact) {
        for shape in [a, b] {
            let inverse =
                glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
            let local = inverse * (hit.point - shape.center);
            for k in 0..3 {
                let row_length = inverse.transpose().col(k).length();
                assert!(
                    local[k].abs() <= 1. + hit.tolerance * row_length * 2.,
                    "point={:?} local={local:?} bound={}",
                    hit.point,
                    hit.tolerance
                );
            }
        }
        assert!((hit.normal.length() - 1.).abs() < 1e-12);
        assert!(
            (hit.normal.dot(hit.point - b.center) + b.radius(hit.normal)).abs()
                <= hit.tolerance * 2.
        );
        assert!(
            (hit.normal.dot(hit.point - a.center) - a.radius(hit.normal)).abs()
                <= hit.tolerance * 2.
        );
    }
    #[test]
    fn clipped_partial_face_witness_is_not_midpoint_of_support_centers() {
        let a = box_shape(DVec3::ZERO, DVec3::ONE);
        let b = box_shape(DVec3::new(-3., 1.3, 0.), DVec3::splat(0.5));
        let displacement = DVec3::X * 3.;
        let hit = a
            .sweep_affine_contact(b.center, b.edges, displacement)
            .unwrap()
            .unwrap();
        assert!((hit.fraction - 0.5).abs() < 1e-12);
        assert!((hit.point - DVec3::new(-1., 0.9, 0.)).length() < 1e-12);
        on_shapes(
            a,
            AffineBox {
                center: b.center + displacement * hit.fraction,
                ..b
            },
            hit,
        );
        // The support-center midpoint at y=0.65 would be outside b's contact face.
        assert!(hit.point.y > 0.8);
    }
    #[test]
    fn rotated_sheared_and_edge_contacts_have_points_in_both_shapes() {
        for i in 0..32 {
            let angle = f64::from(i) * 0.137;
            let qa = glam::DQuat::from_euler(glam::EulerRot::XYZ, angle, 0.3, 0.2);
            let qb = glam::DQuat::from_euler(glam::EulerRot::XYZ, -0.4, angle + 0.2, -angle);
            let a = AffineBox {
                center: DVec3::ZERO,
                edges: [
                    qa * DVec3::X * 0.7,
                    qa * DVec3::new(0.3, 0.8, 0.),
                    qa * DVec3::Z * 0.9,
                ],
            };
            let b = AffineBox {
                center: DVec3::new(-5., 0.1, 0.2),
                edges: [
                    qb * DVec3::X * 0.4,
                    qb * DVec3::Y * 0.5,
                    qb * DVec3::new(0.1, 0., 0.3),
                ],
            };
            let displacement = DVec3::X * 10.;
            let hit = a
                .sweep_affine_contact(b.center, b.edges, displacement)
                .unwrap()
                .unwrap();
            let original = a.sweep_affine(b.center, b.edges, displacement).unwrap();
            assert_eq!(hit.fraction, original.0);
            assert_eq!(hit.normal, original.1);
            on_shapes(
                a,
                AffineBox {
                    center: b.center + displacement * hit.fraction,
                    ..b
                },
                hit,
            );
        }
    }
    #[test]
    fn geometric_witness_drives_shared_point_impulse_with_off_center_torque() {
        use physics::{
            astrophysics_spin::Spin,
            contact::{ContactBody, resolve_normal_impact},
            gravity::Body,
        };
        let a = box_shape(DVec3::ZERO, DVec3::ONE);
        let b = box_shape(DVec3::new(-3., 0.5, 0.), DVec3::splat(0.5));
        let hit = a
            .sweep_affine_contact(b.center, b.edges, DVec3::X * 3.)
            .unwrap()
            .unwrap();
        let spin = Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0.; 3],
            inertia: [1.; 3],
        };
        let mut first = ContactBody {
            motion: Body {
                mass: 1.,
                position: (b.center + DVec3::X * 3. * hit.fraction).to_array(),
                velocity: [3., 0., 0.],
            },
            spin: Some(spin),
        };
        let mut second = ContactBody {
            motion: Body {
                mass: 1.,
                position: [0.; 3],
                velocity: [0.; 3],
            },
            spin: Some(spin),
        };
        let report = resolve_normal_impact(
            &mut first,
            Some(&mut second),
            hit.point.to_array(),
            hit.normal.to_array(),
            0.,
        )
        .unwrap();
        assert!((report.inverse_effective_mass - 2.25).abs() < 1e-12);
        assert!((second.motion.velocity[0] - 4. / 3.).abs() < 1e-12);
        assert!((second.spin.unwrap().angular_momentum[2] + 2. / 3.).abs() < 1e-12);
        assert!(
            (first.energy().unwrap() + second.energy().unwrap() + report.dissipated_energy - 4.5)
                .abs()
                < 1e-12
        );
    }
    #[test]
    fn witness_is_stable_under_uniform_scale_and_large_translation() {
        for scale in [0.001, 1., 1000.] {
            for origin in [DVec3::ZERO, DVec3::splat(100_000.)] {
                let a = box_shape(origin, DVec3::splat(scale));
                let b = box_shape(
                    origin + DVec3::new(-3., 1.3, 0.) * scale,
                    DVec3::splat(0.5 * scale),
                );
                let displacement = DVec3::X * 3. * scale;
                let hit = a
                    .sweep_affine_contact(b.center, b.edges, displacement)
                    .unwrap()
                    .unwrap();
                on_shapes(
                    a,
                    AffineBox {
                        center: b.center + displacement * hit.fraction,
                        ..b
                    },
                    hit,
                );
            }
        }
    }
    #[test]
    fn clear_separating_overlap_and_degenerate_inputs_are_explicit() {
        let a = box_shape(DVec3::ZERO, DVec3::ONE);
        let b = box_shape(DVec3::new(-3., 5., 0.), DVec3::splat(0.5));
        assert!(
            a.sweep_affine_contact(b.center, b.edges, DVec3::X * 3.)
                .unwrap()
                .is_none()
        );
        assert!(
            a.sweep_affine_contact(DVec3::new(-1.5, 0., 0.), b.edges, -DVec3::X)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            a.sweep_affine_contact(DVec3::ZERO, b.edges, DVec3::X)
                .unwrap_err(),
            crate::PhysicsError::InitialOverlap
        );
        assert!(
            a.sweep_affine_contact(b.center, b.edges, DVec3::splat(f64::NAN))
                .is_err()
        );
        assert!(
            a.sweep_affine_contact(DVec3::splat(f64::INFINITY), b.edges, DVec3::X)
                .is_err()
        );
        let invalid = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::ZERO; 3],
        };
        assert!(
            invalid
                .sweep_affine_contact(DVec3::new(-3., 0., 0.), b.edges, DVec3::X * 3.)
                .is_err()
        );
    }
}
