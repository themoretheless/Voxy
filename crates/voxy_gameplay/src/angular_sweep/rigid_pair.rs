//! Two constant world-axis angular arcs on the existing advancement kernel.
use super::{AffineBox, DQuat, DVec3, PhysicsError, advance_with_enclosures, rotate_vector};
use crate::convex::AffineContact;

/// A finite affine shape attached to a rigid frame. Angular is the total world
/// angular displacement over this interval, not a torque-driven Spin trajectory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RigidBoxMotion {
    pub origin: DVec3,
    pub displacement: DVec3,
    pub orientation: DQuat,
    pub angular: DVec3,
    pub shape: AffineBox,
}
impl RigidBoxMotion {
    fn validate(self) -> Result<(), PhysicsError> {
        if !self.origin.is_finite()
            || !self.displacement.is_finite()
            || !self.angular.is_finite()
            || !self.orientation.is_finite()
            || (self.orientation.length() - 1.).abs() > 1e-9
            || !self.shape.center.is_finite()
            || !glam::DMat3::from_cols(
                self.shape.edges[0],
                self.shape.edges[1],
                self.shape.edges[2],
            )
            .inverse()
            .is_finite()
        {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok(())
    }
    fn rotation(self, time: f64) -> DQuat {
        let angle = self.angular.x.hypot(self.angular.y).hypot(self.angular.z);
        if angle == 0. {
            return self.orientation.normalize();
        }
        (DQuat::from_axis_angle(self.angular / angle, angle * time) * self.orientation).normalize()
    }
    pub(super) fn sample(self, time: f64) -> AffineBox {
        let q = self.rotation(time);
        AffineBox {
            center: self.origin + self.displacement * time + rotate_vector(q, self.shape.center),
            edges: self.shape.edges.map(|edge| rotate_vector(q, edge)),
        }
    }
    fn radius(self) -> f64 {
        (0..8)
            .map(|bits| {
                let vertex = self.shape.center
                    + (0..3)
                        .map(|k| self.shape.edges[k] * if bits & (1 << k) == 0 { -1. } else { 1. })
                        .sum::<DVec3>();
                vertex.x.hypot(vertex.y).hypot(vertex.z)
            })
            .fold(0., f64::max)
    }
}

/// Point-witnessed pair collision under translating, constant world-axis arcs.
/// Reuses conservative advancement; budgets count poses and obstacle queries.
/// A failed witness/budget is an error, never an unproved clear interval.
pub(crate) fn sweep_rigid_pair(
    first: RigidBoxMotion,
    second: RigidBoxMotion,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<Option<AffineContact>, PhysicsError> {
    first.validate()?;
    second.validate()?;
    if *steps == 0 || *queries == 0 {
        return Err(PhysicsError::SweepBudget);
    }
    if first.angular == DVec3::ZERO && second.angular == DVec3::ZERO {
        *steps -= 1;
        super::query(queries)?;
        let a = first.sample(0.);
        let b = second.sample(0.);
        let contact =
            b.sweep_affine_contact(a.center, a.edges, first.displacement - second.displacement)?;
        return Ok(contact.map(|mut hit| {
            hit.point += second.displacement * hit.fraction;
            hit.tolerance +=
                8. * f64::EPSILON * (second.displacement * hit.fraction).abs().max_element();
            hit
        }));
    }
    let relative = first.origin - second.origin;
    let displacement = first.displacement - second.displacement;
    let length = |v: DVec3| v.x.hypot(v.y).hypot(v.z);
    let radius = first.radius();
    let separation_bound = length(relative) + length(displacement) + radius;
    let speed = (length(displacement)
        + length(first.angular) * radius
        + length(second.angular) * separation_bound)
        * (1. + 512. * f64::EPSILON);
    if !speed.is_finite() || !separation_bound.is_finite() {
        return Err(PhysicsError::InvalidMotion);
    }
    let sample = |time: f64| {
        let qa = first.rotation(time);
        let qb = second.rotation(time).conjugate();
        let center = rotate_vector(
            qb,
            relative + displacement * time + rotate_vector(qa, first.shape.center),
        );
        let edges = first
            .shape
            .edges
            .map(|edge| rotate_vector(qb, rotate_vector(qa, edge)));
        if !center.is_finite() || edges.iter().any(|e| !e.is_finite()) {
            return Err(PhysicsError::InvalidMotion);
        }
        Ok((center, edges))
    };
    let (initial, initial_edges) = sample(0.)?;
    if second
        .shape
        .penetration_affine(initial, initial_edges)
        .is_some()
    {
        return Err(PhysicsError::InitialOverlap);
    }
    let scale = separation_bound.max(second.radius()).max(1.);
    // Tighter than the witness clip tolerance, leaving room for directed gap rounding.
    let tolerance = 32. * f64::EPSILON * scale;
    let hit = advance_with_enclosures(
        &[&second.shape],
        sample,
        speed,
        radius,
        radius,
        0.,
        0.,
        steps,
        queries,
        None,
        Some(tolerance),
    )?;
    let Some(normal) = hit.normal else {
        return Ok(None);
    };
    let (center, edges) = sample(hit.fraction)?;
    let shape = AffineBox {
        center: center - second.shape.center,
        edges,
    };
    let (point, local_tolerance) = second.shape.contact_point_relative(&shape, normal)?;
    let rotation = second.rotation(hit.fraction);
    let origin = second.origin + second.displacement * hit.fraction;
    let point = origin + rotate_vector(rotation, point);
    let normal = rotate_vector(rotation, normal).normalize();
    let tolerance = local_tolerance
        + 16. * f64::EPSILON * (origin.abs().max_element() + length(point - origin));
    if !point.is_finite() || !normal.is_finite() || !tolerance.is_finite() {
        return Err(PhysicsError::ContactWitness);
    }
    Ok(Some(AffineContact {
        fraction: hit.fraction,
        normal,
        point,
        tolerance,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn motion(origin: DVec3, half: DVec3, angular: DVec3) -> RigidBoxMotion {
        RigidBoxMotion {
            origin,
            displacement: DVec3::ZERO,
            orientation: DQuat::IDENTITY,
            angular,
            shape: AffineBox {
                center: DVec3::ZERO,
                edges: [DVec3::X * half.x, DVec3::Y * half.y, DVec3::Z * half.z],
            },
        }
    }
    fn inside(shape: AffineBox, hit: AffineContact) {
        let inverse =
            glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
        let local = inverse * (hit.point - shape.center);
        for k in 0..3 {
            assert!(
                local[k].abs() <= 1. + hit.tolerance * inverse.transpose().col(k).length() * 4.,
                "{local:?} {:?}",
                hit.point
            );
        }
    }
    fn run(a: RigidBoxMotion, b: RigidBoxMotion) -> AffineContact {
        let hit = sweep_rigid_pair(a, b, &mut 10_000, &mut 10_000)
            .unwrap()
            .unwrap();
        inside(a.sample(hit.fraction), hit);
        inside(b.sample(hit.fraction), hit);
        hit
    }
    #[test]
    fn transient_rotating_contact_is_found_with_both_endpoints_clear() {
        let a = motion(
            DVec3::ZERO,
            DVec3::new(2., 0.02, 0.02),
            DVec3::Z * std::f64::consts::PI,
        );
        let b = motion(DVec3::new(0., 1.8, 0.), DVec3::splat(0.05), DVec3::ZERO);
        for time in [0., 1.] {
            let first = a.sample(time);
            let second = b.sample(time);
            assert!(
                second
                    .penetration_affine(first.center, first.edges)
                    .is_none()
            );
        }
        let hit = run(a, b);
        assert!(hit.fraction > 0.4 && hit.fraction < 0.5);
        let before = a.sample(hit.fraction - 1e-6);
        let wall = b.sample(hit.fraction);
        assert!(
            wall.penetration_affine(before.center, before.edges)
                .is_none()
        );
        let after = a.sample(hit.fraction + 1e-6);
        assert!(wall.penetration_affine(after.center, after.edges).is_some());
    }
    #[test]
    fn rotating_pair_is_covariant_and_transports_world_witness() {
        let mut a = motion(
            DVec3::ZERO,
            DVec3::new(2., 0.02, 0.02),
            DVec3::Z * std::f64::consts::PI,
        );
        let mut b = motion(
            DVec3::new(0., 1.8, 0.),
            DVec3::splat(0.05),
            -DVec3::Z * std::f64::consts::PI,
        );
        a.displacement = DVec3::new(0.1, 0.2, 0.);
        b.displacement = a.displacement;
        let base = run(a, b);
        let basis = DQuat::from_euler(glam::EulerRot::XYZ, 0.3, -0.6, 0.2);
        let offset = DVec3::new(1000., -400., 20.);
        for m in [&mut a, &mut b] {
            m.origin = offset + basis * m.origin;
            m.displacement = basis * m.displacement;
            m.angular = basis * m.angular;
            m.orientation = basis;
        }
        let reframed = run(a, b);
        assert!((reframed.fraction - base.fraction).abs() < 1e-10);
        assert!((reframed.point - (offset + basis * base.point)).length() < 1e-9);
        assert!((reframed.normal - basis * base.normal).length() < 1e-10);
    }
    #[test]
    fn translated_zero_arc_matches_exact_sweep_and_budget_errors_are_explicit() {
        let mut a = motion(DVec3::new(-3., 0.5, 0.), DVec3::splat(0.5), DVec3::ZERO);
        let mut b = motion(DVec3::ZERO, DVec3::ONE, DVec3::ZERO);
        a.displacement = DVec3::X * 4.;
        b.displacement = DVec3::X;
        let hit = run(a, b);
        assert!((hit.fraction - 0.5).abs() < 1e-12);
        assert!((hit.point - DVec3::new(-0.5, 0.5, 0.)).length() < 1e-12);
        assert_eq!(
            sweep_rigid_pair(a, b, &mut 0, &mut 10).unwrap_err(),
            PhysicsError::SweepBudget
        );
        a.angular = DVec3::Z * std::f64::consts::PI;
        assert_eq!(
            sweep_rigid_pair(a, b, &mut 1, &mut 1).unwrap_err(),
            PhysicsError::SweepBudget
        );
        a.angular = DVec3::splat(f64::NAN);
        assert_eq!(
            sweep_rigid_pair(a, b, &mut 100, &mut 100).unwrap_err(),
            PhysicsError::InvalidMotion
        );
    }
}

/// A safe nominal prefix under the supplied Spin model-error envelope.
/// Some(normal) denotes possible contact, not a point-witnessed physical impact.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpinPathHit {
    pub time_s: f64,
    pub normal: Option<DVec3>,
    pub model_error_m: f64,
}
/// Shape is attached to the principal Spin frame around the center of mass.
/// Translation is constant velocity here; no implicit COM/pivot substitution.
pub(crate) fn sweep_spin_path_static(
    path: &physics::spin_path::SpinPath,
    origin: DVec3,
    velocity: DVec3,
    shape: AffineBox,
    obstacles: &[AffineBox],
    steps: &mut usize,
    queries: &mut usize,
) -> Result<SpinPathHit, PhysicsError> {
    let candidates: Vec<_> = obstacles.iter().collect();
    let mut error = 0.;
    for segment in path.segments() {
        let duration = segment.arc.duration();
        let motion = RigidBoxMotion {
            origin: origin + velocity * segment.start_s,
            displacement: velocity * duration,
            orientation: DQuat::from_array(segment.arc.start().orientation),
            angular: DVec3::from_array(segment.arc.angular_velocity()) * duration,
            shape,
        };
        motion.validate()?;
        for obstacle in obstacles {
            if !obstacle.center.is_finite()
                || !glam::DMat3::from_cols(obstacle.edges[0], obstacle.edges[1], obstacle.edges[2])
                    .inverse()
                    .is_finite()
            {
                return Err(PhysicsError::InvalidMotion);
            }
        }
        let radius = motion.radius();
        error = (radius * segment.model_angular_error_rad).next_up();
        let speed = motion.displacement.length() + motion.angular.length() * radius;
        if !error.is_finite() || !speed.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let hit = advance_with_enclosures(
            &candidates,
            |time| {
                let pose = motion.sample(time);
                Ok((pose.center, pose.edges))
            },
            speed,
            radius,
            radius,
            0.,
            error,
            steps,
            queries,
            None,
            None,
        )?;
        if let Some(normal) = hit.normal {
            return Ok(SpinPathHit {
                time_s: segment.start_s + duration * hit.fraction,
                normal: Some(normal),
                model_error_m: error,
            });
        }
    }
    Ok(SpinPathHit {
        time_s: path.duration(),
        normal: None,
        model_error_m: error,
    })
}

#[cfg(test)]
mod spin_bridge_tests {
    use super::*;
    use physics::{astrophysics_spin::Spin, spin_path::Config};
    #[test]
    fn admitted_precessing_path_is_swept_with_model_envelope_not_fake_impact() {
        let spin = Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0.1, 0.2, 1.5],
            inertia: [1., 1.1, 1.2],
        };
        let path = spin
            .prepare_path(
                [0.; 3],
                2.6,
                Config {
                    max_angular_error_rad: 0.002,
                    min_step_s: 1e-9,
                    max_arcs: 30_000,
                    max_trials: 100_000,
                },
            )
            .unwrap();
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 2., DVec3::Y * 0.02, DVec3::Z * 0.02],
        };
        let wall = AffineBox {
            center: DVec3::new(0., 1.8, 0.),
            edges: [DVec3::X * 0.05, DVec3::Y * 0.05, DVec3::Z * 0.5],
        };
        let hit = sweep_spin_path_static(
            &path,
            DVec3::ZERO,
            DVec3::ZERO,
            shape,
            &[wall],
            &mut 100_000,
            &mut 100_000,
        )
        .unwrap();
        assert!(hit.normal.is_some());
        assert!(hit.time_s > 0. && hit.time_s < path.duration());
        assert!(hit.model_error_m > 0. && hit.model_error_m <= 0.005);
        let far = AffineBox {
            center: DVec3::new(20., 20., 20.),
            ..wall
        };
        let clear = sweep_spin_path_static(
            &path,
            DVec3::ZERO,
            DVec3::ZERO,
            shape,
            &[far],
            &mut 100_000,
            &mut 100_000,
        )
        .unwrap();
        assert!(clear.normal.is_none());
        assert_eq!(clear.time_s, path.duration());
        assert_eq!(
            sweep_spin_path_static(
                &path,
                DVec3::ZERO,
                DVec3::ZERO,
                shape,
                &[wall],
                &mut 0,
                &mut 0
            )
            .unwrap_err(),
            PhysicsError::SweepBudget
        );
        println!(
            "SPIN_MODEL_SWEEP arcs={} time_s={} error_m={}",
            path.segments().len(),
            hit.time_s,
            hit.model_error_m
        );
    }
}
