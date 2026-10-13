//! Rigid debris dynamics: pieces released by fracture fall, tumble and land
//! on a plane with restitution-free impacts and Coulomb friction. Energy is
//! tracked explicitly so callers can audit each step.

use crate::math::{add, cross, dot, norm, scale, sub};
use crate::plasticity::mesh::DebrisPiece;

type Vec3 = [f64; 3];
type Mat3 = [[f64; 3]; 3];

/// Support plane `normal · x = offset` and contact material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisGround {
    pub normal: Vec3,
    pub offset_m: f64,
    pub friction: f64,
}

/// Rigid state of one piece in the world frame.
#[derive(Clone, Debug)]
pub struct DebrisBody {
    pub piece: DebrisPiece,
    /// Body-to-world rotation (columns are body axes).
    pub rotation: Mat3,
    pub position_m: Vec3,
    pub velocity_m_s: Vec3,
    pub angular_velocity_rad_s: Vec3,
    inverse_inertia_body: Mat3,
}

/// Cumulative energy ledger. By construction
/// `kinetic + potential + dissipated - correction - integration` equals the
/// initial mechanical energy exactly; `integration_j` exposes the scheme's
/// own first-order error in contact steps instead of hiding it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DebrisStepReport {
    pub kinetic_j: f64,
    pub potential_j: f64,
    /// Cumulative energy removed by inelastic contact impulses and friction.
    pub dissipated_j: f64,
    /// Cumulative potential energy added by positional push-out of penetration.
    pub correction_j: f64,
    /// Cumulative residual of the time integration (zero in free flight).
    pub integration_j: f64,
    pub contacts: usize,
    /// Set by the first step; later steps audit against it.
    pub initial_j: Option<f64>,
}

impl DebrisBody {
    /// Places a piece at its own centre with its released velocities.
    /// # Errors
    /// Singular or nonfinite inertia.
    pub fn new(piece: DebrisPiece) -> Result<Self, &'static str> {
        let inverse = invert(piece.inertia_kg_m2).ok_or("singular debris inertia")?;
        Ok(Self {
            position_m: piece.center_m,
            velocity_m_s: piece.velocity_m_s,
            angular_velocity_rad_s: piece.angular_velocity_rad_s,
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            inverse_inertia_body: inverse,
            piece,
        })
    }

    fn world_point(&self, local: Vec3) -> Vec3 {
        let r = sub(local, self.piece.center_m);
        add(self.position_m, mat_vec(self.rotation, r))
    }

    /// Hull vertices in the world frame.
    #[must_use]
    pub fn world_hull(&self) -> Vec<Vec3> {
        self.piece
            .hull
            .iter()
            .map(|&p| self.world_point(p))
            .collect()
    }

    fn inverse_inertia_world(&self) -> Mat3 {
        let r = self.rotation;
        let rt = transpose(r);
        mat_mul(mat_mul(r, self.inverse_inertia_body), rt)
    }

    #[must_use]
    pub fn kinetic_j(&self) -> f64 {
        let inertia_world = {
            let r = self.rotation;
            let inertia = invert(self.inverse_inertia_body).unwrap_or([[0.; 3]; 3]);
            mat_mul(mat_mul(r, inertia), transpose(r))
        };
        0.5 * self.piece.mass_kg * dot(self.velocity_m_s, self.velocity_m_s)
            + 0.5
                * dot(
                    self.angular_velocity_rad_s,
                    mat_vec(inertia_world, self.angular_velocity_rad_s),
                )
    }
}

/// Advances every body by `dt` under `gravity` against `ground`.
/// Contacts are resolved at the centroid of the penetrating hull vertices per
/// body with a positional push-out, an inelastic normal impulse and Coulomb
/// friction.
/// # Errors
/// Nonfinite or negative `dt`, invalid ground or numerical overflow.
pub fn step_debris(
    bodies: &mut [DebrisBody],
    dt: f64,
    gravity: Vec3,
    ground: &DebrisGround,
    ledger: &mut DebrisStepReport,
) -> Result<(), &'static str> {
    if !dt.is_finite() || dt < 0. || gravity.iter().any(|g| !g.is_finite()) {
        return Err("invalid debris step");
    }
    let n = norm(ground.normal);
    if !n.is_finite()
        || n == 0.
        || !ground.offset_m.is_finite()
        || !ground.friction.is_finite()
        || ground.friction < 0.
    {
        return Err("invalid debris ground");
    }
    let normal = scale(ground.normal, 1. / n);
    let offset = ground.offset_m / n;
    let mechanical = |bodies: &[DebrisBody]| {
        bodies
            .iter()
            .map(|b| b.kinetic_j() + b.piece.mass_kg * -dot(gravity, b.position_m))
            .sum::<f64>()
    };
    let start = mechanical(bodies);
    let initial = *ledger.initial_j.get_or_insert(start);
    let (dissipated_before, correction_before) = (ledger.dissipated_j, ledger.correction_j);
    let mut contacts = 0;
    // Bodies already touching the ground within one step's fall are in contact
    // before integration, so resting bodies never creep.
    let touch_tolerance = norm(gravity) * dt * dt;
    for body in bodies.iter_mut() {
        let old_velocity = body.velocity_m_s;
        let old_spin = body.angular_velocity_rad_s;
        body.velocity_m_s = add(body.velocity_m_s, scale(gravity, dt));

        // Contact patch at the current pose: the centroid of touching hull
        // vertices, so a face resting flat produces no spurious torque.
        let hull = body.world_hull();
        let mut sum = [0.; 3];
        let mut count = 0.;
        for p in hull {
            if offset - dot(p, normal) >= -touch_tolerance {
                sum = add(sum, p);
                count += 1.;
            }
        }
        let touching = count > 0.;
        if touching {
            contacts += 1;
            let point = scale(sum, 1. / count);
            let r = sub(point, body.position_m);
            let before = body.kinetic_j();
            let inverse_inertia = body.inverse_inertia_world();
            // Three or more touching vertices form a supporting patch: the
            // face is pinned, so tilting spin is removed and impulses act on
            // the centre of mass without rotational compliance.
            let patch = count >= 3.;
            if patch {
                let spin_n = dot(body.angular_velocity_rad_s, normal);
                body.angular_velocity_rad_s = scale(normal, spin_n);
            }
            let velocity = add(body.velocity_m_s, cross(body.angular_velocity_rad_s, r));
            let vn = dot(velocity, normal);
            let generalized = |direction: Vec3| {
                if patch {
                    return 1. / body.piece.mass_kg;
                }
                let rn = cross(r, direction);
                1. / body.piece.mass_kg + dot(rn, mat_vec(inverse_inertia, rn))
            };
            let mut impulse = [0.; 3];
            let mut normal_impulse = 0.;
            if vn < 0. {
                normal_impulse = -vn / generalized(normal);
                impulse = scale(normal, normal_impulse);
            }
            let tangential = sub(velocity, scale(normal, vn));
            let speed = norm(tangential);
            if speed > 0. {
                let direction = scale(tangential, 1. / speed);
                let stopping = speed / generalized(direction);
                impulse = sub(
                    impulse,
                    scale(direction, stopping.min(ground.friction * normal_impulse)),
                );
            }
            body.velocity_m_s = add(body.velocity_m_s, scale(impulse, 1. / body.piece.mass_kg));
            if !patch {
                body.angular_velocity_rad_s = add(
                    body.angular_velocity_rad_s,
                    mat_vec(inverse_inertia, cross(r, impulse)),
                );
            }
            ledger.dissipated_j += before - body.kinetic_j();
        }

        // Free flight: trapezoidal update is exact under constant gravity.
        // In contact: the post-impulse velocity carries the whole step, so a
        // stopped body stays put and the impact energy is fully booked.
        let (velocity, spin) = if touching {
            (body.velocity_m_s, body.angular_velocity_rad_s)
        } else {
            (
                scale(add(old_velocity, body.velocity_m_s), 0.5),
                scale(add(old_spin, body.angular_velocity_rad_s), 0.5),
            )
        };
        body.position_m = add(body.position_m, scale(velocity, dt));
        body.rotation = rotate(body.rotation, scale(spin, dt));

        // Residual penetration: positional push-out along the normal, booked
        // as potential energy added.
        let depth = body
            .world_hull()
            .into_iter()
            .map(|p| offset - dot(p, normal))
            .fold(0_f64, f64::max);
        if depth > 0. {
            body.position_m = add(body.position_m, scale(normal, depth));
            ledger.correction_j += body.piece.mass_kg * depth * -dot(gravity, normal);
        }
    }
    // Piece-piece contact: the deepest hull vertex of one piece inside the
    // other's hull is resolved with a shared inelastic normal impulse, Coulomb
    // friction and a mass-split positional push-out.
    for i in 0..bodies.len() {
        for j in i + 1..bodies.len() {
            let hull_i = bodies[i].world_hull();
            let hull_j = bodies[j].world_hull();
            // Normal points from piece `j` toward piece `i` (the push direction for `i`).
            let Some((point, depth, normal)) = sat_contact(
                &hull_i,
                &bodies[i].piece.hull_faces,
                &hull_j,
                &bodies[j].piece.hull_faces,
            ) else {
                continue;
            };
            let (penetrating, penetrated) = (i, j);
            contacts += 1;
            let (first, second) = bodies.split_at_mut(penetrated.max(penetrating));
            let (a, b) = if penetrating < penetrated {
                (&mut first[penetrating], &mut second[0])
            } else {
                (&mut second[0], &mut first[penetrated])
            };
            let before = a.kinetic_j() + b.kinetic_j();
            let ra = sub(point, a.position_m);
            let rb = sub(point, b.position_m);
            let inv_a = a.inverse_inertia_world();
            let inv_b = b.inverse_inertia_world();
            let generalized = |direction: Vec3| {
                let ca = cross(ra, direction);
                let cb = cross(rb, direction);
                1. / a.piece.mass_kg
                    + 1. / b.piece.mass_kg
                    + dot(ca, mat_vec(inv_a, ca))
                    + dot(cb, mat_vec(inv_b, cb))
            };
            let velocity = sub(
                add(a.velocity_m_s, cross(a.angular_velocity_rad_s, ra)),
                add(b.velocity_m_s, cross(b.angular_velocity_rad_s, rb)),
            );
            let vn = dot(velocity, normal);
            let mut impulse = [0.; 3];
            let mut normal_impulse = 0.;
            if vn < 0. {
                normal_impulse = -vn / generalized(normal);
                impulse = scale(normal, normal_impulse);
            }
            let tangential = sub(velocity, scale(normal, vn));
            let speed = norm(tangential);
            if speed > 0. && normal_impulse > 0. {
                let direction = scale(tangential, 1. / speed);
                let stopping = speed / generalized(direction);
                impulse = sub(
                    impulse,
                    scale(direction, stopping.min(ground.friction * normal_impulse)),
                );
            }
            a.velocity_m_s = add(a.velocity_m_s, scale(impulse, 1. / a.piece.mass_kg));
            a.angular_velocity_rad_s =
                add(a.angular_velocity_rad_s, mat_vec(inv_a, cross(ra, impulse)));
            b.velocity_m_s = sub(b.velocity_m_s, scale(impulse, 1. / b.piece.mass_kg));
            b.angular_velocity_rad_s =
                sub(b.angular_velocity_rad_s, mat_vec(inv_b, cross(rb, impulse)));
            ledger.dissipated_j += before - (a.kinetic_j() + b.kinetic_j());
            // Push-out split by inverse mass; booked as potential energy change.
            let total = 1. / a.piece.mass_kg + 1. / b.piece.mass_kg;
            let share_a = (1. / a.piece.mass_kg) / total;
            let move_a = scale(normal, depth * share_a);
            let move_b = scale(normal, -depth * (1. - share_a));
            a.position_m = add(a.position_m, move_a);
            b.position_m = add(b.position_m, move_b);
            ledger.correction_j +=
                a.piece.mass_kg * -dot(gravity, move_a) + b.piece.mass_kg * -dot(gravity, move_b);
        }
    }
    let mut kinetic = 0.;
    let mut potential = 0.;
    for body in bodies.iter() {
        kinetic += body.kinetic_j();
        potential += body.piece.mass_kg * -dot(gravity, body.position_m);
    }
    if !kinetic.is_finite() || !potential.is_finite() || !ledger.dissipated_j.is_finite() {
        return Err("debris step overflow");
    }
    // Whatever the step did not book explicitly is the integration residual.
    let booked =
        (ledger.dissipated_j - dissipated_before) - (ledger.correction_j - correction_before);
    ledger.integration_j += (kinetic + potential + booked) - start;
    let _ = initial;
    ledger.kinetic_j = kinetic;
    ledger.potential_j = potential;
    ledger.contacts = contacts;
    Ok(())
}

/// Separating-axis contact between two convex hulls over their face normals
/// (edge–edge axes are not tested). Returns the support patch centroid of
/// hull `a` along the minimal-overlap axis, the overlap depth, and the unit
/// normal pointing from `b` toward `a`. Coincident faces of aligned boxes resolve
/// along the axis of least overlap instead of a zero-distance side plane.
fn sat_contact(
    a: &[Vec3],
    faces_a: &[[usize; 3]],
    b: &[Vec3],
    faces_b: &[[usize; 3]],
) -> Option<(Vec3, f64, Vec3)> {
    let mut best: Option<(f64, Vec3)> = None;
    let project = |points: &[Vec3], n: Vec3| {
        points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                let d = dot(*p, n);
                (lo.min(d), hi.max(d))
            })
    };
    for (hull, faces) in [(a, faces_a), (b, faces_b)] {
        for &[p, q, r] in faces {
            let n = cross(sub(hull[q], hull[p]), sub(hull[r], hull[p]));
            let length = norm(n);
            if length == 0. {
                continue;
            }
            let n = scale(n, 1. / length);
            let (lo_a, hi_a) = project(a, n);
            let (lo_b, hi_b) = project(b, n);
            if hi_a <= lo_b || hi_b <= lo_a {
                return None;
            }
            // Overlap when `a` sits below `b` along `n`, and the reverse.
            let below = hi_a - lo_b;
            let above = hi_b - lo_a;
            let (overlap, direction) = if below < above {
                (below, scale(n, -1.))
            } else {
                (above, n)
            };
            if best.is_none_or(|(d, _)| overlap < d) {
                best = Some((overlap, direction));
            }
        }
    }
    let (depth, normal) = best?;
    // Contact patch: the centroid of `a`'s support vertices against the push
    // direction (within the overlap), so a face landing flat exerts no torque.
    let lowest = a
        .iter()
        .map(|p| dot(*p, normal))
        .fold(f64::INFINITY, f64::min);
    let tolerance = depth.max(1e-9 * (1. + lowest.abs()));
    let mut sum = [0.; 3];
    let mut count = 0.;
    for p in a {
        if dot(*p, normal) <= lowest + tolerance {
            sum = add(sum, *p);
            count += 1.;
        }
    }
    if count == 0. {
        return None;
    }
    Some((scale(sum, 1. / count), depth, normal))
}

fn mat_vec(m: Mat3, v: Vec3) -> Vec3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

fn transpose(m: Mat3) -> Mat3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

fn mat_mul(a: Mat3, b: Mat3) -> Mat3 {
    let bt = transpose(b);
    [
        [dot(a[0], bt[0]), dot(a[0], bt[1]), dot(a[0], bt[2])],
        [dot(a[1], bt[0]), dot(a[1], bt[1]), dot(a[1], bt[2])],
        [dot(a[2], bt[0]), dot(a[2], bt[1]), dot(a[2], bt[2])],
    ]
}

fn invert(m: Mat3) -> Option<Mat3> {
    let c = [cross(m[1], m[2]), cross(m[2], m[0]), cross(m[0], m[1])];
    let det = dot(m[0], c[0]);
    if !det.is_finite() || det.abs() <= f64::MIN_POSITIVE {
        return None;
    }
    // Inverse = adj / det, adj rows are the cofactor columns.
    let inv = transpose([
        scale(c[0], 1. / det),
        scale(c[1], 1. / det),
        scale(c[2], 1. / det),
    ]);
    inv.iter().flatten().all(|v| v.is_finite()).then_some(inv)
}

/// Rotates the frame by the rotation vector `theta` (left-multiplied) and
/// re-orthonormalizes with Gram–Schmidt.
fn rotate(r: Mat3, theta: Vec3) -> Mat3 {
    let angle = norm(theta);
    if angle == 0. {
        return r;
    }
    let axis = scale(theta, 1. / angle);
    let (s, c) = angle.sin_cos();
    let k = [
        [0., -axis[2], axis[1]],
        [axis[2], 0., -axis[0]],
        [-axis[1], axis[0], 0.],
    ];
    let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let kk = mat_mul(k, k);
    let mut rot = identity;
    for i in 0..3 {
        for j in 0..3 {
            rot[i][j] += s * k[i][j] + (1. - c) * kk[i][j];
        }
    }
    let mut out = mat_mul(rot, r);
    // Orthonormalize columns.
    let mut cols = transpose(out);
    cols[0] = scale(cols[0], 1. / norm(cols[0]));
    cols[1] = sub(cols[1], scale(cols[0], dot(cols[0], cols[1])));
    cols[1] = scale(cols[1], 1. / norm(cols[1]));
    cols[2] = cross(cols[0], cols[1]);
    out = transpose(cols);
    out
}

#[cfg(test)]
mod tests;
