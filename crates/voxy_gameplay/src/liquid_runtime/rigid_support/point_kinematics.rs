//! Velocity, acceleration and jerk of a clipped support-patch vertex, from the active slab constraints.
use super::*;

/// Differentiate the active slab constraints defining a clipped patch vertex.
/// Coincident constraints must agree; topology kinks are not assigned an
/// arbitrary material owner. Shapes are principal-body templates, except fixed
/// environment geometry, whose coordinates are already world-oriented.
pub(in crate::liquid_runtime) fn point_velocity(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
) -> Result<[f64; 3], Error> {
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        true,
        |_, body, _, velocity| Ok(body.map_or(0., |_| velocity)),
    )
}

/// Candidate second derivative from the same independent active-plane basis.
/// The factor two accounts for both the rotating plane and sliding along it.
/// Redundant plane rates may carry pressure-solver residual. Their actual
/// distance is bounded by whole-interval admission, not forced to agree exactly.
pub(in crate::liquid_runtime) fn point_acceleration(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    loads: [physics::contact::ContactWrench; 2],
) -> Result<[f64; 3], Error> {
    if loads.iter().any(|load| {
        load.force
            .iter()
            .chain(&load.torque)
            .any(|x| !x.is_finite())
    }) {
        return Err(Error::InvalidCollision);
    }
    let velocity = DVec3::from_array(point_velocity(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
    )?);
    let p = DVec3::from_array(point);
    // Each shape contributes all its active planes through one load owner.
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        false,
        |owner, body, normal, _| {
            let load = loads[owner];
            let Some(body) = body else {
                return Ok(0.);
            };
            if body.spin.is_none() && load.torque != [0.; 3] {
                return Err(Error::InvalidCollision);
            }
            let material = DVec3::from_array(
                body.point_velocity(point)
                    .map_err(|_| Error::InvalidCollision)?,
            );
            let arm = p - DVec3::from_array(body.motion.position);
            let (omega, alpha) = if let Some(spin) = body.spin {
                (
                    DVec3::from_array(
                        spin.angular_velocity()
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_acceleration(load.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                )
            } else {
                (DVec3::ZERO, DVec3::ZERO)
            };
            let acceleration = DVec3::from_array(load.force) / body.motion.mass
                + alpha.cross(arm)
                + omega.cross(omega.cross(arm));
            let value =
                normal.dot(acceleration) + 2. * omega.cross(normal).dot(material - velocity);
            if !value.is_finite() {
                return Err(Error::InvalidCollision);
            }
            Ok(value)
        },
    )
}

/// Candidate third derivative of the same active-plane basis. Required by cubic
/// common-point trajectories; not a certificate of finite branch persistence.
#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(in crate::liquid_runtime) fn point_jerk(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    loads: [physics::contact::ContactWrench; 2],
    rates: [physics::contact::ContactWrench; 2],
) -> Result<[f64; 3], Error> {
    if rates.iter().any(|load| {
        load.force
            .iter()
            .chain(&load.torque)
            .any(|v| !v.is_finite())
    }) {
        return Err(Error::InvalidCollision);
    }
    let velocity = DVec3::from_array(point_velocity(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
    )?);
    let acceleration = DVec3::from_array(point_acceleration(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        loads,
    )?);
    let p = DVec3::from_array(point);
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        false,
        |owner, body, normal, _| {
            let Some(body) = body else {
                return Ok(0.);
            };
            let load = loads[owner];
            let rate = rates[owner];
            if body.spin.is_none() && rate.torque != [0.; 3] {
                return Err(Error::InvalidCollision);
            }
            let (omega, alpha, beta) = if let Some(spin) = body.spin {
                (
                    DVec3::from_array(
                        spin.angular_velocity()
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_acceleration(load.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_jerk(load.torque, rate.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                )
            } else {
                (DVec3::ZERO, DVec3::ZERO, DVec3::ZERO)
            };
            let nd = omega.cross(normal);
            let ndd = alpha.cross(normal) + omega.cross(nd);
            let nddd = beta.cross(normal)
                + 2. * alpha.cross(nd)
                + omega.cross(alpha.cross(normal))
                + omega.cross(omega.cross(nd));
            let value = normal.dot(DVec3::from_array(rate.force) / body.motion.mass)
                - 3. * nd.dot(acceleration - DVec3::from_array(load.force) / body.motion.mass)
                - 3. * ndd.dot(velocity - DVec3::from_array(body.motion.velocity))
                - nddd.dot(p - DVec3::from_array(body.motion.position));
            if !value.is_finite() {
                return Err(Error::InvalidCollision);
            }
            Ok(value)
        },
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn solve_point_constraints(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    require_coincident_rates: bool,
    mut rate: impl FnMut(usize, Option<ContactBody>, DVec3, f64) -> Result<f64, Error>,
) -> Result<[f64; 3], Error> {
    first.energy().map_err(|_| Error::InvalidCollision)?;
    if let Some(second) = second {
        second.energy().map_err(|_| Error::InvalidCollision)?;
    }
    let point = DVec3::from_array(point);
    if !point.is_finite() || !tolerance_m.is_finite() || tolerance_m < 0. {
        return Err(Error::InvalidCollision);
    }
    let mut constraints = Vec::with_capacity(6);
    for (owner, (body, shape)) in [(Some(first), first_shape), (second, second_shape)]
        .into_iter()
        .enumerate()
    {
        let q = body
            .and_then(|b| b.spin)
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
        let center = body.map_or(shape.center, |b| {
            DVec3::from_array(b.motion.position) + rotate_vector(q, shape.center)
        });
        let edges = if body.is_some() {
            shape.edges.map(|edge| rotate_vector(q, edge))
        } else {
            shape.edges
        };
        let inverse = glam::DMat3::from_cols(edges[0], edges[1], edges[2]).inverse();
        if !inverse.is_finite() {
            return Err(Error::InvalidCollision);
        }
        let coordinates = inverse * (point - center);
        let rows = inverse.transpose().to_cols_array();
        let velocity = body
            .map_or(Ok([0.; 3]), |b| b.point_velocity(point.to_array()))
            .map_err(|_| Error::InvalidCollision)?;
        let velocity = DVec3::from_array(velocity);
        for k in 0..3 {
            let row = DVec3::new(rows[k * 3], rows[k * 3 + 1], rows[k * 3 + 2]);
            let length = row.length();
            let gap = (coordinates[k].abs() - 1.) / length;
            let guard = tolerance_m
                + 1024. * f64::EPSILON * (1. + edges[k].length() + (point - center).length());
            if !gap.is_finite() || gap > guard {
                return Err(Error::InvalidCollision);
            }
            if gap.abs() <= guard {
                let normal = row / length;
                constraints.push((normal, rate(owner, body, normal, normal.dot(velocity))?));
            }
        }
    }
    for i in 0..constraints.len() {
        for j in i + 1..constraints.len() {
            for k in j + 1..constraints.len() {
                let matrix =
                    glam::DMat3::from_cols(constraints[i].0, constraints[j].0, constraints[k].0)
                        .transpose();
                let determinant = matrix.determinant().abs();
                if determinant < 1e-10 {
                    continue;
                }
                let velocity = matrix.inverse()
                    * DVec3::new(constraints[i].1, constraints[j].1, constraints[k].1);
                let guard = 4096. * f64::EPSILON * (1. + velocity.length()) / determinant;
                if velocity.is_finite()
                    && (!require_coincident_rates
                        || constraints
                            .iter()
                            .all(|(normal, rate)| (normal.dot(velocity) - rate).abs() <= guard))
                {
                    return Ok(velocity.to_array());
                }
            }
        }
    }
    Err(Error::CollisionBackend)
}

#[cfg(test)]
mod point_velocity_tests {
    use super::*;

    fn body(center: DVec3, velocity: DVec3) -> ContactBody {
        ContactBody {
            motion: physics::gravity::Body {
                position: center.to_array(),
                velocity: velocity.to_array(),
                mass: 1.,
            },
            spin: None,
        }
    }
    fn box_shape(x: f64, y: f64, z: f64) -> AffineBox {
        AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * x, DVec3::Y * y, DVec3::Z * z],
        }
    }

    #[test]
    fn rotating_corner_velocity_matches_independent_finite_difference() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::rotation::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let floor = AffineBox {
            center: DVec3::Y * -0.5,
            ..box_shape(4., 0.5, 4.)
        };
        let actual = DVec3::from_array(
            point_velocity(
                first,
                box_shape(1., 1., 1.),
                None,
                floor,
                [1., 0., 1.],
                1e-12,
            )
            .unwrap(),
        );
        assert!((actual - DVec3::new(0.3, 0., -0.3)).length() < 1e-13);
        for h in [1e-3, 1e-4] {
            let corner =
                |t: f64| DVec3::Y + DQuat::from_rotation_y(0.3 * t) * DVec3::new(1., -1., 1.);
            assert!(((corner(h) - corner(-h)) / (2. * h) - actual).length() < 1e-8);
        }
    }

    #[test]
    fn clipped_vertex_follows_both_bodies_instead_of_one_material_owner() {
        let first = body(DVec3::Y, DVec3::new(0.4, 0., 7.));
        let second = body(-DVec3::Y, DVec3::new(8., 0., -0.2));
        let point = [1., 0., 0.5];
        let actual = DVec3::from_array(
            point_velocity(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
            )
            .unwrap(),
        );
        let vertex = |t: f64| {
            DVec3::new(
                (1. + 0.4 * t).min(2. + 8. * t),
                0.,
                (1. + 7. * t).min(0.5 - 0.2 * t),
            )
        };
        let h = 1e-4;
        assert!(((vertex(h) - vertex(-h)) / (2. * h) - actual).length() < 1e-11);
        assert!((actual - DVec3::from_array(first.point_velocity(point).unwrap())).length() > 1.);
        assert!((actual - DVec3::from_array(second.point_velocity(point).unwrap())).length() > 1.);
        let swapped = point_velocity(
            second,
            box_shape(2., 1., 0.5),
            Some(first),
            box_shape(1., 1., 1.),
            point,
            1e-12,
        )
        .unwrap();
        assert!((actual - DVec3::from_array(swapped)).length() < 1e-13);
        // Oblique slabs exercise inverse-matrix rows independently of the
        // axis-aligned fixture. A shared affine map preserves intersection.
        let map = glam::DMat3::from_quat(DQuat::from_rotation_z(0.7))
            * glam::DMat3::from_cols(
                DVec3::new(1.2, 0.1, 0.),
                DVec3::new(0.2, 0.8, 0.1),
                DVec3::new(-0.1, 0.3, 1.1),
            );
        let shift = DVec3::new(4., -3., 2.);
        let transformed_body = |b: ContactBody| {
            body(
                map * DVec3::from_array(b.motion.position) + shift,
                map * DVec3::from_array(b.motion.velocity),
            )
        };
        let transformed_shape = |shape: AffineBox| AffineBox {
            center: map * shape.center,
            edges: shape.edges.map(|edge| map * edge),
        };
        let transformed = DVec3::from_array(
            point_velocity(
                transformed_body(first),
                transformed_shape(box_shape(1., 1., 1.)),
                Some(transformed_body(second)),
                transformed_shape(box_shape(2., 1., 0.5)),
                (map * DVec3::from_array(point) + shift).to_array(),
                1e-12,
            )
            .unwrap(),
        );
        let reference = (map * vertex(h) + shift - (map * vertex(-h) + shift)) / (2. * h);
        assert!((reference - transformed).length() < 1e-10);
    }

    #[test]
    fn clipped_acceleration_includes_sliding_on_a_rotating_plane() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::rotation::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let second = body(-DVec3::Y, DVec3::ZERO);
        let point = [1., 0., 0.5];
        let loads = [physics::contact::ContactWrench::default(); 2];
        let acceleration = DVec3::from_array(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                loads,
            )
            .unwrap(),
        );
        assert!((acceleration - DVec3::new(0.09, 0., 0.)).length() < 1e-13);
        let vertex = |t: f64| DVec3::new((1. + 0.5 * (0.3 * t).sin()) / (0.3 * t).cos(), 0., 0.5);
        for h in [1e-3, 3e-4] {
            let independent = (vertex(h) - 2. * vertex(0.) + vertex(-h)) / (h * h);
            assert!((independent - acceleration).length() < 1e-7);
        }
        let swapped = point_acceleration(
            second,
            box_shape(2., 1., 0.5),
            Some(first),
            box_shape(1., 1., 1.),
            point,
            1e-12,
            loads,
        )
        .unwrap();
        assert!((acceleration - DVec3::from_array(swapped)).length() < 1e-13);
        first.spin.as_mut().unwrap().angular_momentum = [0.; 3];
        let loaded = [
            physics::contact::ContactWrench {
                force: [2., 0., 0.],
                torque: [0., 0.4, 0.],
            },
            physics::contact::ContactWrench {
                force: [0., 0., -0.6],
                torque: [0.; 3],
            },
        ];
        let actual = DVec3::from_array(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                loaded,
            )
            .unwrap(),
        );
        let vertex = |t: f64| {
            let angle = 0.2 * t * t;
            let z = 0.5 - 0.3 * t * t;
            DVec3::new(t * t + (1. + z * angle.sin()) / angle.cos(), 0., z)
        };
        let h = 3e-4;
        assert!(((vertex(h) - 2. * vertex(0.) + vertex(-h)) / (h * h) - actual).length() < 1e-6);
        assert!((actual - DVec3::new(2.2, 0., -0.6)).length() < 1e-13);
        let mut invalid = loads;
        invalid[0].force[0] = f64::NAN;
        assert_eq!(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                invalid
            ),
            Err(Error::InvalidCollision)
        );
    }

    #[test]
    fn clipped_jerk_matches_independent_plane_intersection() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::rotation::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let second = body(-DVec3::Y, DVec3::ZERO);
        let loads = [physics::contact::ContactWrench::default(); 2];
        let rates = loads;
        let actual = DVec3::from_array(
            point_jerk(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                [1., 0., 0.5],
                1e-12,
                loads,
                rates,
            )
            .unwrap(),
        );
        let vertex = |t: f64| DVec3::new((1. + 0.5 * (0.3 * t).sin()) / (0.3 * t).cos(), 0., 0.5);
        let h = 0.01;
        let independent = (vertex(2. * h) - 2. * vertex(h) + 2. * vertex(-h) - vertex(-2. * h))
            / (2. * h * h * h);
        assert!((actual - DVec3::new(0.027, 0., 0.)).length() < 1e-13);
        assert!((independent - actual).length() < 1e-6);
        first.spin.as_mut().unwrap().angular_momentum = [0.; 3];
        let rates = [
            physics::contact::ContactWrench {
                force: [0.6, 0., 0.],
                torque: [0., 0.4, 0.],
            },
            physics::contact::ContactWrench {
                force: [0., 0., -0.2],
                torque: [0.; 3],
            },
        ];
        let actual = DVec3::from_array(
            point_jerk(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                [1., 0., 0.5],
                1e-12,
                loads,
                rates,
            )
            .unwrap(),
        );
        assert!((actual - DVec3::new(0.8, 0., -0.2)).length() < 1e-13);
    }

    #[test]
    fn topology_kinks_and_unowned_points_are_rejected() {
        let shape = box_shape(1., 1., 1.);
        let stationary = body(DVec3::ZERO, DVec3::ZERO);
        let moving = body(DVec3::ZERO, DVec3::X);
        assert_eq!(
            point_velocity(stationary, shape, Some(moving), shape, [1., 1., 1.], 1e-12),
            Err(Error::CollisionBackend)
        );
        assert_eq!(
            point_velocity(stationary, shape, None, shape, [0.; 3], 1e-12),
            Err(Error::CollisionBackend)
        );
        assert_eq!(
            point_velocity(stationary, shape, None, shape, [2., 0., 0.], 1e-12),
            Err(Error::InvalidCollision)
        );
        let mut invalid = stationary;
        invalid.motion.mass = 0.;
        assert_eq!(
            point_velocity(invalid, shape, None, shape, [1.; 3], 1e-12),
            Err(Error::InvalidCollision)
        );
    }
}
