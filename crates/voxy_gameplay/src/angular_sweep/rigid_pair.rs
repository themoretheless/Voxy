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
    pub feature: Option<crate::convex::AxisFeature>,
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
                feature: hit.feature,
                time_s: segment.start_s + duration * hit.fraction,
                normal: Some(normal),
                model_error_m: error,
            });
        }
    }
    Ok(SpinPathHit {
        feature: None,
        time_s: path.duration(),
        normal: None,
        model_error_m: error,
    })
}

/// Sweep the admitted COM parabola and SpinPath together. A returned normal
/// still means possible contact under a floating model envelope, not an impact.
pub(crate) fn sweep_rigid_motion_static(
    path: &physics::rigid_motion::RigidMotion,
    shape: AffineBox,
    obstacles: &[AffineBox],
    steps: &mut usize,
    queries: &mut usize,
) -> Result<SpinPathHit, PhysicsError> {
    let initial = path.initial();
    let orientation = initial
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let template = RigidBoxMotion {
        origin: DVec3::from_array(initial.motion.position),
        displacement: DVec3::ZERO,
        orientation,
        angular: DVec3::ZERO,
        shape,
    };
    template.validate()?;
    for obstacle in obstacles {
        RigidBoxMotion {
            origin: DVec3::ZERO,
            shape: *obstacle,
            ..template
        }
        .validate()?;
    }
    let candidates: Vec<_> = obstacles.iter().collect();
    let radius = template.radius();
    let count = path
        .rotation()
        .map_or(1, |rotation| rotation.segments().len());
    let mut error = 0.;
    for index in 0..count {
        let (start, end, angular_speed, angular_error) =
            path.rotation()
                .map_or((0., path.duration(), 0., 0.), |rotation| {
                    let segment = rotation.segments()[index];
                    (
                        segment.start_s,
                        segment.end_s,
                        DVec3::from_array(segment.arc.angular_velocity()).length(),
                        segment.model_angular_error_rad,
                    )
                });
        let first = path
            .sample(start)
            .map_err(|_| PhysicsError::InvalidMotion)?;
        let last = path.sample(end).map_err(|_| PhysicsError::InvalidMotion)?;
        let duration = end - start;
        // The norm of affine velocity is convex: endpoint maxima bound the
        // whole interval, including a translation reversal inside an arc.
        let linear_speed = DVec3::from_array(first.motion.velocity)
            .length()
            .max(DVec3::from_array(last.motion.velocity).length());
        let speed = (linear_speed + angular_speed * radius) * duration;
        let coordinate_scale = DVec3::from_array(first.motion.position)
            .abs()
            .max_element()
            .max(DVec3::from_array(last.motion.position).abs().max_element())
            + linear_speed * duration
            + radius
            + 1.;
        error = (radius * angular_error + 64. * f64::EPSILON * coordinate_scale).next_up();
        if !speed.is_finite() || !error.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let hit = advance_with_enclosures(
            &candidates,
            |fraction| {
                let state = path
                    .sample(if fraction == 1. {
                        end
                    } else {
                        duration.mul_add(fraction, start).clamp(start, end)
                    })
                    .map_err(|_| PhysicsError::InvalidMotion)?;
                let rotation = state
                    .spin
                    .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
                Ok((
                    DVec3::from_array(state.motion.position) + rotation * shape.center,
                    shape.edges.map(|edge| rotation * edge),
                ))
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
                feature: hit.feature,
                time_s: duration.mul_add(hit.fraction, start),
                normal: Some(normal),
                model_error_m: error,
            });
        }
    }
    Ok(SpinPathHit {
        feature: None,
        time_s: path.duration(),
        normal: None,
        model_error_m: error,
    })
}

/// Reciprocal prepared trajectories. Shapes use their own principal COM frames.
/// Returns only a possible-contact prefix, with the normal in world coordinates.
pub(crate) fn sweep_rigid_motions(
    first: &physics::rigid_motion::RigidMotion,
    first_shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    second_shape: AffineBox,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<SpinPathHit, PhysicsError> {
    sweep_rigid_motions_impl(
        first,
        first_shape,
        second,
        second_shape,
        steps,
        queries,
        false,
        0.,
    )
}

fn sweep_rigid_motions_impl(
    first: &physics::rigid_motion::RigidMotion,
    first_shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    second_shape: AffineBox,
    steps: &mut usize,
    queries: &mut usize,
    nominal_contact: bool,
    from_s: f64,
) -> Result<SpinPathHit, PhysicsError> {
    if first.duration() != second.duration() {
        return Err(PhysicsError::InvalidMotion);
    }
    let radius = |path: &physics::rigid_motion::RigidMotion, shape| {
        let body = path.initial();
        let motion = RigidBoxMotion {
            origin: DVec3::from_array(body.motion.position),
            displacement: DVec3::ZERO,
            angular: DVec3::ZERO,
            orientation: body
                .spin
                .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation)),
            shape,
        };
        motion.validate()?;
        Ok::<_, PhysicsError>(motion.radius())
    };
    let ra = radius(first, first_shape)?;
    let rb = radius(second, second_shape)?;
    if nominal_contact && from_s == 0. {
        let a = first.initial();
        let b = second.initial();
        let qa = a
            .spin
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
        let qb = b
            .spin
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation))
            .conjugate();
        let center = qb
            * (DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position)
                + qa * first_shape.center);
        let edges = first_shape.edges.map(|e| qb * (qa * e));
        if second_shape.penetration_affine(center, edges).is_some() {
            return Err(PhysicsError::InitialOverlap);
        }
    }
    let mut knots = vec![0., first.duration()];
    for path in [first, second] {
        if let Some(rotation) = path.rotation() {
            knots.extend(rotation.segments().iter().map(|s| s.end_s));
        }
    }
    knots.sort_by(f64::total_cmp);
    knots.dedup();
    let angular = |path: &physics::rigid_motion::RigidMotion, start: f64| {
        path.rotation().map_or((0., 0.), |rotation| {
            let index = rotation.segments().partition_point(|s| s.end_s <= start);
            let segment = rotation.segments()[index];
            (
                DVec3::from_array(segment.arc.angular_velocity()).length(),
                segment.model_angular_error_rad,
            )
        })
    };
    let sample = |path: &physics::rigid_motion::RigidMotion, time| {
        path.sample(time).map_err(|_| PhysicsError::InvalidMotion)
    };
    let mut error = 0.;
    for interval in knots.windows(2) {
        let (start, end) = (interval[0].max(from_s), interval[1]);
        if end <= start {
            continue;
        }
        let dt = end - start;
        let a = sample(first, start)?;
        let b = sample(second, start)?;
        let ae = sample(first, end)?;
        let be = sample(second, end)?;
        let relative = DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position);
        let velocity = |a: physics::contact::ContactBody, b: physics::contact::ContactBody| {
            DVec3::from_array(a.motion.velocity) - DVec3::from_array(b.motion.velocity)
        };
        let linear = velocity(a, b).length().max(velocity(ae, be).length());
        let (wa, ea) = angular(first, start);
        let (wb, eb) = angular(second, start);
        let separation = relative.length() + linear * dt + ra;
        let speed = (linear + wa * ra + wb * separation) * dt;
        let coordinates = [a, b, ae, be]
            .iter()
            .map(|body| DVec3::from_array(body.motion.position).abs().max_element())
            .fold(0., f64::max);
        error = (ra * ea + rb * eb + 64. * f64::EPSILON * (coordinates + separation + rb + 1.))
            .next_up();
        if !speed.is_finite() || !error.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let absolute_motion = [a, b, ae, be]
            .iter()
            .map(|body| DVec3::from_array(body.motion.velocity).length())
            .fold(0., f64::max)
            * dt;
        // A fixed world-axis gap can cover an entire nominal arc even when
        // its initial gap is below the advancement contact tolerance. Retain
        // small physical velocities/spins; never clamp them to rest.
        // Match this query's nominal-trajectory contract: model residuals
        // stay in model_error_m, separate from nominal pose evaluation.
        let excursion = ((linear + wa * ra + wb * rb) * (1. + 512. * f64::EPSILON) * dt
            + 64. * f64::EPSILON * (coordinates + absolute_motion + relative.length() + ra + rb))
            .next_up();
        if nominal_contact && excursion.is_finite() {
            let qa = a
                .spin
                .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
            let qb = b
                .spin
                .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
            let dilation = (excursion
                + 2. * ra * (qa.length_squared() - 1.).abs()
                + 2. * rb * (qb.length_squared() - 1.).abs())
            .next_up();
            let moving = AffineBox {
                center: DVec3::from_array(a.motion.position) + qa * first_shape.center,
                edges: first_shape.edges.map(|edge| qa * edge),
            };
            let obstacle = AffineBox {
                center: DVec3::from_array(b.motion.position) + qb * second_shape.center,
                edges: second_shape.edges.map(|edge| qb * edge),
            };
            let mut covered = false;
            super::query(queries)?;
            for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
                if super::gap::lower(moving.center, moving.edges, &obstacle, axis, dilation)? > 0. {
                    covered = true;
                    break;
                }
            }
            if covered {
                continue;
            }
        }
        let hit = advance_with_enclosures(
            &[&second_shape],
            |fraction| {
                let time = if fraction == 1. {
                    end
                } else {
                    dt.mul_add(fraction, start).clamp(start, end)
                };
                let a = sample(first, time)?;
                let b = sample(second, time)?;
                let qa = a
                    .spin
                    .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
                let qb = b
                    .spin
                    .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation))
                    .conjugate();
                let relative =
                    DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position);
                Ok((
                    qb * (relative + qa * first_shape.center),
                    first_shape.edges.map(|e| qb * (qa * e)),
                ))
            },
            speed,
            ra,
            separation,
            0.,
            if nominal_contact { 0. } else { error },
            steps,
            queries,
            None,
            if nominal_contact {
                Some(32. * f64::EPSILON * (separation + rb + 1.))
            } else {
                None
            },
        )?;
        if let Some(normal) = hit.normal {
            let time = dt.mul_add(hit.fraction, start).clamp(start, end);
            let b = sample(second, time)?;
            let qb = b
                .spin
                .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
            return Ok(SpinPathHit {
                feature: hit.feature,
                time_s: time,
                normal: Some(qb * normal),
                model_error_m: error,
            });
        }
    }
    Ok(SpinPathHit {
        feature: None,
        time_s: first.duration(),
        normal: None,
        model_error_m: error,
    })
}

/// Geometric contact on the admitted numerical trajectory. Model orbit error
/// remains explicit; this is not a certified contact on the exact physical orbit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RigidTrajectoryContact {
    pub feature: crate::convex::AxisFeature,
    pub time_s: f64,
    pub point: DVec3,
    pub normal: DVec3,
    pub tolerance_m: f64,
    pub model_error_m: f64,
    pub first: physics::contact::ContactBody,
    pub second: physics::contact::ContactBody,
}

pub(crate) fn sweep_nominal_rigid_contact(
    first: &physics::rigid_motion::RigidMotion,
    first_shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    second_shape: AffineBox,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<Option<RigidTrajectoryContact>, PhysicsError> {
    let mut from_s = 0.;
    loop {
        let hit = sweep_rigid_motions_impl(
            first,
            first_shape,
            second,
            second_shape,
            steps,
            queries,
            true,
            from_s,
        )?;
        let Some(normal) = hit.normal else {
            return Ok(None);
        };
        let a = first
            .sample(hit.time_s)
            .map_err(|_| PhysicsError::InvalidMotion)?;
        let b = second
            .sample(hit.time_s)
            .map_err(|_| PhysicsError::InvalidMotion)?;
        let qa = a
            .spin
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
        let qb = b
            .spin
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
        let relative = qb.conjugate()
            * (DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position)
                + qa * first_shape.center)
            - second_shape.center;
        let local = AffineBox {
            center: relative,
            edges: first_shape.edges.map(|e| qb.conjugate() * (qa * e)),
        };
        let (point, tolerance) =
            second_shape.contact_point_relative(&local, qb.conjugate() * normal)?;
        let point = DVec3::from_array(b.motion.position) + qb * point;
        let tolerance_m = tolerance + 16. * f64::EPSILON * (point.abs().max_element() + 1.);
        if !point.is_finite() || !tolerance_m.is_finite() {
            return Err(PhysicsError::ContactWitness);
        }
        let velocity = DVec3::from_array(
            a.point_velocity(point.to_array())
                .map_err(|_| PhysicsError::Solver)?,
        ) - DVec3::from_array(
            b.point_velocity(point.to_array())
                .map_err(|_| PhysicsError::Solver)?,
        );
        if velocity.dot(normal) >= 0. {
            if hit.time_s >= first.duration() {
                return Ok(None);
            }
            super::query(queries)?;
            from_s = separating_prefix(first, first_shape, second, normal, hit.time_s)?;
            if from_s >= first.duration() {
                return Ok(None);
            }
            continue;
        }
        return Ok(Some(RigidTrajectoryContact {
            feature: hit.feature.ok_or(PhysicsError::ContactWitness)?,
            time_s: hit.time_s,
            point,
            normal,
            tolerance_m,
            model_error_m: hit.model_error_m,
            first: a,
            second: b,
        }));
    }
}

/// Monotone gap prefix along the second body's rotating supporting plane.
/// Uses a floating derivative/curvature bound on nominal constant-axis arcs.
/// Unresolved resting/edge cases fail explicitly instead of inventing clearance.
fn separating_prefix(
    first: &physics::rigid_motion::RigidMotion,
    shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    normal: DVec3,
    time: f64,
) -> Result<f64, PhysicsError> {
    let arc = |path: &physics::rigid_motion::RigidMotion| {
        path.rotation()
            .map_or((DVec3::ZERO, path.duration()), |rotation| {
                let i = rotation.segments().partition_point(|s| s.end_s <= time);
                let segment = rotation.segments()[i];
                (
                    DVec3::from_array(segment.arc.angular_velocity()),
                    segment.end_s,
                )
            })
    };
    let (wa, ea) = arc(first);
    let (wb, eb) = arc(second);
    let end = ea.min(eb);
    let horizon = end - time;
    let a = first
        .sample(time)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let b = second
        .sample(time)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let qa = a
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = b
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    if (qa.length_squared() - 1.).abs() > 64. * f64::EPSILON
        || (qb.length_squared() - 1.).abs() > 64. * f64::EPSILON
    {
        return Err(PhysicsError::SweepBudget);
    }
    let relative = DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position);
    let velocity = DVec3::from_array(a.motion.velocity) - DVec3::from_array(b.motion.velocity);
    let acceleration =
        DVec3::from_array(first.acceleration()) - DVec3::from_array(second.acceleration());
    if wa == DVec3::ZERO && wb == DVec3::ZERO {
        // A fixed supporting plane remains nonpenetrating under nonclosing
        // linear/quadratic normal motion, even while the patch slides sideways.
        if velocity.dot(normal) >= 0. && acceleration.dot(normal) >= 0. {
            return Ok(end);
        }
    }
    let center = qa * shape.center;
    let edges = shape.edges.map(|e| qa * e);
    let derivative = normal.dot(velocity - wb.cross(relative) + (wa - wb).cross(center))
        - edges
            .iter()
            .map(|e| normal.dot((wa - wb).cross(*e)).abs())
            .sum::<f64>();
    let radius = shape.center.length() + shape.edges.iter().map(|e| e.length()).sum::<f64>();
    let velocity_max = velocity
        .length()
        .max((velocity + acceleration * horizon).length());
    let separation = relative.length() + velocity_max * horizon;
    let curvature = acceleration.length()
        + 2. * wb.length() * velocity_max
        + wb.length_squared() * separation
        + (wa.length() + wb.length()).powi(2) * radius;
    let guard = 256.
        * f64::EPSILON
        * (1. + velocity_max + wa.length() * radius + wb.length() * (separation + radius));
    let lower = derivative - guard;
    let bound = curvature * (1. + 256. * f64::EPSILON) + guard;
    if !lower.is_finite() || !bound.is_finite() || lower <= 0. {
        return Err(PhysicsError::SweepBudget);
    }
    let prefix = horizon.min(lower / bound);
    let next = (time + prefix).min(end);
    if next <= time {
        return Err(PhysicsError::SweepBudget);
    }
    Ok(next)
}

#[cfg(test)]
mod spin_bridge_tests {
    use super::*;
    use physics::{astrophysics_spin::Spin, spin_path::Config};
    #[test]
    fn nominal_fixed_axis_gap_covers_small_motion_but_not_a_closing_interval() {
        let config = Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.04, DVec3::Y * 0.02, DVec3::Z * 0.02],
        };
        let body = |x, velocity, spin| physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [x, 0., 0.],
                velocity: [velocity, 0., 0.],
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., spin, 0.],
                inertia: [1.; 3],
            }),
        };
        let wall = body(0., 0., 0.)
            .prepare_motion([0.; 3], [0.; 3], 0.02, config)
            .unwrap();
        let start = body(-0.080000000000006, 0., 1e-13);
        let small = start
            .prepare_motion([0.; 3], [0.; 3], 0.02, config)
            .unwrap();
        assert!(
            sweep_nominal_rigid_contact(&small, shape, &wall, shape, &mut 10000, &mut 10000)
                .unwrap()
                .is_none()
        );
        assert_eq!(small.initial(), start);
        assert_ne!(
            small.end().spin.unwrap().orientation,
            start.spin.unwrap().orientation
        );
        let mut approaching = start;
        approaching.motion.velocity[0] = 1e-10;
        let closing = approaching
            .prepare_motion([0.; 3], [0.; 3], 0.02, config)
            .unwrap();
        let hit =
            sweep_nominal_rigid_contact(&closing, shape, &wall, shape, &mut 10000, &mut 10000)
                .unwrap()
                .unwrap();
        assert_eq!(hit.normal, -DVec3::X);
        assert!(hit.time_s < 1e-4);
        // The represented nominal arc is the contract here, not the true
        // torque-driven orbit or an interval transcendental evaluation proof.
    }

    #[test]
    fn separating_touch_is_not_an_impact_and_does_not_hide_accelerated_return() {
        let config = Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let body = |x, v| physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [x, 0., 0.],
                velocity: [v, 0., 0.],
            },
            spin: None,
        };
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.5, DVec3::Y * 0.5, DVec3::Z * 0.5],
        };
        let wall = body(0., 0.)
            .prepare_motion([0.; 3], [0.; 3], 1.1, config)
            .unwrap();
        let outward = body(-1., -1.)
            .prepare_motion([0.; 3], [0.; 3], 1.1, config)
            .unwrap();
        assert!(
            sweep_nominal_rigid_contact(&outward, shape, &wall, shape, &mut 10000, &mut 10000)
                .unwrap()
                .is_none()
        );
        let returning = body(-1., -1.)
            .prepare_motion([2., 0., 0.], [0.; 3], 1.1, config)
            .unwrap();
        let hit =
            sweep_nominal_rigid_contact(&returning, shape, &wall, shape, &mut 10000, &mut 10000)
                .unwrap()
                .unwrap();
        assert!((hit.time_s - 1.).abs() < 1e-12);
        let resting = body(-1., 0.)
            .prepare_motion([0.; 3], [0.; 3], 1.1, config)
            .unwrap();
        assert!(
            sweep_nominal_rigid_contact(&resting, shape, &wall, shape, &mut 10000, &mut 10000)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn unresolved_rotating_face_touch_is_rejected_instead_of_declared_clear() {
        let config = Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let body = |x, spin| physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [x, 0., 0.],
                velocity: [0.; 3],
            },
            spin,
        };
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.5, DVec3::Y * 0.5, DVec3::Z * 0.5],
        };
        let a = body(
            -1.,
            Some(Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., 2.],
                inertia: [1.; 3],
            }),
        )
        .prepare_motion([0.; 3], [0.; 3], 0.1, config)
        .unwrap();
        let b = body(0., None)
            .prepare_motion([0.; 3], [0.; 3], 0.1, config)
            .unwrap();
        assert_eq!(
            sweep_nominal_rigid_contact(&a, shape, &b, shape, &mut 10000, &mut 10000).unwrap_err(),
            PhysicsError::SweepBudget
        );
    }
    #[test]
    fn trajectory_point_witness_drives_off_center_reciprocal_impulse() {
        let config = Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let body = |mass, position, velocity| physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass,
                position,
                velocity,
            },
            spin: Some(Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0.; 3],
                inertia: [1.; 3],
            }),
        };
        let a = body(1., [-2., 0.5, 0.], [2., 0., 0.])
            .prepare_motion([0.5, 0., 0.], [0.; 3], 1., config)
            .unwrap();
        let b = body(2., [0.; 3], [0.; 3])
            .prepare_motion([0.; 3], [0.; 3], 1., config)
            .unwrap();
        let shape = |half: f64| AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * half, DVec3::Y * half, DVec3::Z * half],
        };
        let sa = shape(0.2);
        let sb = shape(0.5);
        let hit = sweep_nominal_rigid_contact(&a, sa, &b, sb, &mut 10000, &mut 10000)
            .unwrap()
            .unwrap();
        let exact = -4. + 2. * 5.3_f64.sqrt();
        assert!((hit.time_s - exact).abs() < 1e-12);
        assert!((hit.point - DVec3::new(-0.5, 0.4, 0.)).length() < 1e-12);
        for (state, shape) in [(hit.first, sa), (hit.second, sb)] {
            let q = DQuat::from_array(state.spin.unwrap().orientation);
            let local = q.conjugate() * (hit.point - DVec3::from_array(state.motion.position));
            for k in 0..3 {
                assert!(local[k].abs() <= shape.edges[k].length() + hit.tolerance_m);
            }
        }
        assert!(hit.model_error_m > 0.);
        let first = hit.first;
        let second = hit.second;
        let energy = first.energy().unwrap() + second.energy().unwrap();
        let angular = |s: physics::contact::ContactBody| {
            DVec3::from_array(s.motion.position)
                .cross(DVec3::from_array(s.motion.velocity) * s.motion.mass)
                + DVec3::from_array(s.spin.unwrap().angular_momentum)
        };
        let momentum =
            |s: physics::contact::ContactBody| DVec3::from_array(s.motion.velocity) * s.motion.mass;
        let before_l = angular(first) + angular(second);
        let before_p = momentum(first) + momentum(second);
        let event = physics::rigid_motion::prepare_impact(
            &a,
            &b,
            hit.time_s,
            hit.point.to_array(),
            hit.normal.to_array(),
            0.6,
            config,
        )
        .unwrap();
        let first = event.first;
        let second = event.second;
        let report = event.impulse;
        assert_eq!(event.first_remainder.as_ref().unwrap().initial(), first);
        assert_eq!(event.second_remainder.as_ref().unwrap().initial(), second);
        assert!(event.endpoints().iter().all(|body| body.energy().is_ok()));
        assert!(
            (first.energy().unwrap() + second.energy().unwrap() + report.dissipated_energy
                - energy)
                .abs()
                < 1e-12
        );
        assert!((angular(first) + angular(second) - before_l).length() < 1e-12);
        assert!((momentum(first) + momentum(second) - before_p).length() < 1e-12);
        assert!(second.spin.unwrap().angular_momentum[2] < 0.);
        assert_eq!(a.sample(hit.time_s).unwrap(), hit.first);
        assert_eq!(b.sample(hit.time_s).unwrap(), hit.second);
        assert_eq!(
            sweep_nominal_rigid_contact(&a, sa, &b, sb, &mut 0, &mut 0).unwrap_err(),
            PhysicsError::SweepBudget
        );
        let overlapping = body(1., [0.; 3], [0.; 3])
            .prepare_motion([0.; 3], [0.; 3], 1., config)
            .unwrap();
        assert_eq!(
            sweep_nominal_rigid_contact(&overlapping, sa, &b, sb, &mut 10000, &mut 10000)
                .unwrap_err(),
            PhysicsError::InitialOverlap
        );
    }
    #[test]
    fn reciprocal_accelerating_bodies_find_analytic_contact_and_reverse_normal() {
        let config = Config {
            max_angular_error_rad: 1e-4,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let prepare = |sign: f64| {
            physics::contact::ContactBody {
                motion: physics::gravity::Body {
                    mass: 1.,
                    position: [-2. * sign, 0., 0.],
                    velocity: [2. * sign, 0., 0.],
                },
                spin: None,
            }
            .prepare_motion([2. * sign, 0., 0.], [0.; 3], 1., config)
            .unwrap()
        };
        let a = prepare(1.);
        let b = prepare(-1.);
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1],
        };
        let hit = sweep_rigid_motions(&a, shape, &b, shape, &mut 10000, &mut 10000).unwrap();
        let reverse = sweep_rigid_motions(&b, shape, &a, shape, &mut 10000, &mut 10000).unwrap();
        let exact = 2.9_f64.sqrt() - 1.;
        assert!((hit.time_s - exact).abs() < 1e-6);
        assert!((hit.time_s - reverse.time_s).abs() < 1e-12);
        assert!((hit.normal.unwrap() + reverse.normal.unwrap()).length() < 1e-12);
        assert_eq!(
            sweep_rigid_motions(&a, shape, &b, shape, &mut 0, &mut 0).unwrap_err(),
            PhysicsError::SweepBudget
        );
    }
    #[test]
    fn two_torque_paths_merge_distinct_knots_and_keep_world_normal_reciprocal() {
        let config = Config {
            max_angular_error_rad: 1e-5,
            min_step_s: 1e-9,
            max_arcs: 20000,
            max_trials: 60000,
        };
        let prepare = |position, momentum, force, torque| {
            physics::contact::ContactBody {
                motion: physics::gravity::Body {
                    mass: 1.,
                    position,
                    velocity: [0.; 3],
                },
                spin: Some(Spin {
                    orientation: [0., 0., 0., 1.],
                    angular_momentum: momentum,
                    inertia: [1.; 3],
                }),
            }
            .prepare_motion(force, torque, 1., config)
            .unwrap()
        };
        let a = prepare(
            [0.; 3],
            [0., 0., std::f64::consts::PI],
            [0.1, 0., 0.],
            [0., 0., 0.02],
        );
        let b = prepare(
            [0., 1.8, 0.],
            [0., 0., -0.7],
            [-0.1, 0., 0.],
            [0., 0., -0.005],
        );
        assert_ne!(
            a.rotation().unwrap().segments().len(),
            b.rotation().unwrap().segments().len()
        );
        let rod = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 2., DVec3::Y * 0.02, DVec3::Z * 0.02],
        };
        let box_shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.05, DVec3::Y * 0.05, DVec3::Z * 0.05],
        };
        let hit = sweep_rigid_motions(&a, rod, &b, box_shape, &mut 100000, &mut 100000).unwrap();
        let reverse =
            sweep_rigid_motions(&b, box_shape, &a, rod, &mut 100000, &mut 100000).unwrap();
        assert!(hit.time_s > 0.3 && hit.time_s < 0.5);
        assert!((hit.time_s - reverse.time_s).abs() < 1e-5);
        assert!((hit.normal.unwrap() + reverse.normal.unwrap()).length() < 1e-4);
        assert!(hit.model_error_m > 0. && reverse.model_error_m > 0.);
        let contact = sweep_nominal_rigid_contact(&a, rod, &b, box_shape, &mut 100000, &mut 100000)
            .unwrap()
            .unwrap();
        assert!(contact.time_s >= hit.time_s);
        for (body, shape) in [(contact.first, rod), (contact.second, box_shape)] {
            let q = DQuat::from_array(body.spin.unwrap().orientation);
            let local = q.conjugate() * (contact.point - DVec3::from_array(body.motion.position))
                - shape.center;
            let inverse =
                glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
            let coordinates = inverse * local;
            for k in 0..3 {
                assert!(
                    coordinates[k].abs()
                        <= 1. + contact.tolerance_m * inverse.transpose().col(k).length() * 4.
                );
            }
        }
    }
    #[test]
    fn accelerating_com_reversal_finds_contact_with_identical_clear_endpoints() {
        let body = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [-2., 0., 0.],
                velocity: [4., 0., 0.],
            },
            spin: None,
        };
        let config = Config {
            max_angular_error_rad: 1e-4,
            min_step_s: 1e-9,
            max_arcs: 10000,
            max_trials: 30000,
        };
        let path = body
            .prepare_motion([-4., 0., 0.], [0.; 3], 2., config)
            .unwrap();
        assert_eq!(path.initial().motion.position, path.end().motion.position);
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1],
        };
        let wall = shape;
        let hit = sweep_rigid_motion_static(&path, shape, &[wall], &mut 10000, &mut 10000).unwrap();
        let exact = 1. - 0.1_f64.sqrt();
        assert!(hit.normal.is_some());
        assert!(
            (hit.time_s - exact).abs() < 1e-6,
            "{} vs {}",
            hit.time_s,
            exact
        );
        assert!(hit.time_s <= exact + 1e-12);
        let far = AffineBox {
            center: DVec3::new(0., 2., 0.),
            ..wall
        };
        assert!(
            sweep_rigid_motion_static(&path, shape, &[far], &mut 10000, &mut 10000)
                .unwrap()
                .normal
                .is_none()
        );
        assert_eq!(
            sweep_rigid_motion_static(&path, shape, &[wall], &mut 0, &mut 0).unwrap_err(),
            PhysicsError::SweepBudget
        );
    }
    #[test]
    fn accelerated_rotating_shape_matches_independent_spherical_pose_scan() {
        let body = physics::contact::ContactBody {
            motion: physics::gravity::Body {
                mass: 1.,
                position: [-1., 0., 0.],
                velocity: [0.; 3],
            },
            spin: Some(Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., 4.],
                inertia: [1.; 3],
            }),
        };
        let path = body
            .prepare_motion(
                [2., 0., 0.],
                [0.; 3],
                0.5,
                Config {
                    max_angular_error_rad: 1e-4,
                    min_step_s: 1e-9,
                    max_arcs: 10000,
                    max_trials: 30000,
                },
            )
            .unwrap();
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X, DVec3::Y * 0.02, DVec3::Z * 0.02],
        };
        let wall = AffineBox {
            center: DVec3::new(-0.85, 0.8, 0.),
            edges: [DVec3::X * 0.03, DVec3::Y * 0.03, DVec3::Z * 0.03],
        };
        let analytic = |t: f64| {
            let q = DQuat::from_rotation_z(4. * t);
            (
                DVec3::new(-1. + t * t, 0., 0.),
                shape.edges.map(|edge| q * edge),
            )
        };
        for t in [0., 0.5] {
            let (center, edges) = analytic(t);
            assert!(wall.penetration_affine(center, edges).is_none());
        }
        let reference = (1..=20000)
            .map(|i| i as f64 * 0.5 / 20000.)
            .find(|&t| {
                let (center, edges) = analytic(t);
                wall.penetration_affine(center, edges).is_some()
            })
            .expect("analytic transient contact");
        let hit = sweep_rigid_motion_static(&path, shape, &[wall], &mut 10000, &mut 10000).unwrap();
        assert!(hit.normal.is_some());
        assert!(hit.time_s <= reference);
        assert!(
            (hit.time_s - reference).abs() < 1e-4,
            "{} vs {}",
            hit.time_s,
            reference
        );
    }
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
