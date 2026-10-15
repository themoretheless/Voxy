//! Rigid debris dynamics: pieces released by fracture fall, tumble and land
//! on a plane with restitution-free impacts and Coulomb friction. Energy is
//! tracked explicitly so callers can audit each step.

use crate::math::{add, cross, dot, norm, scale, sub};
use crate::plasticity::mesh::{DebrisPiece, DebrisRelease, convex_hull};
use crate::suspension::Particle;
use crate::wear::{FractureDustSettings, fracture_dust};

type Vec3 = [f64; 3];
type Mat3 = [[f64; 3]; 3];

/// Support plane `normal · x = offset` and contact material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisGround {
    pub normal: Vec3,
    pub offset_m: f64,
    pub friction: f64,
}

/// Static axis-aligned box obstacle (e.g. a solid voxel or merged voxel column)
/// against which debris convex hulls collide with inelastic impulses and friction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisBoxObstacle {
    pub min_m: Vec3,
    pub max_m: Vec3,
    pub friction: f64,
}

impl DebrisBoxObstacle {
    fn is_valid(&self) -> bool {
        self.min_m.iter().all(|v| v.is_finite())
            && self.max_m.iter().all(|v| v.is_finite())
            && (0..3).all(|a| self.min_m[a] < self.max_m[a])
            && self.friction.is_finite()
            && self.friction >= 0.
    }

    fn vertices(&self) -> [Vec3; 8] {
        let [x0, y0, z0] = self.min_m;
        let [x1, y1, z1] = self.max_m;
        [
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ]
    }
}

const BOX_FACES: [[usize; 3]; 12] = [
    [0, 2, 1],
    [0, 3, 2],
    [4, 5, 6],
    [4, 6, 7],
    [0, 1, 5],
    [0, 5, 4],
    [2, 3, 7],
    [2, 7, 6],
    [0, 4, 7],
    [0, 7, 3],
    [1, 2, 6],
    [1, 6, 5],
];

/// Settings for secondary impact comminution (crumbling of a debris body into
/// smaller sub-pieces and airborne dust upon a hard impact).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrumbleSettings {
    /// Minimum dissipated impact energy (J) required to trigger crumbling.
    pub min_impact_energy_j: f64,
    /// Minimum mass (kg) of each resulting child fragment.
    pub min_piece_mass_kg: f64,
    /// Material density (kg/m³) of emitted dust grains.
    pub density_kg_m3: f64,
    /// Fresh fracture surface energy (J/m²).
    pub surface_energy_j_m2: f64,
    /// Fraction of impact energy spent creating fine dust grains in `[0, 1]`.
    pub dust_energy_fraction: f64,
    /// Nominal radius (m) of emitted dust grains.
    pub grain_radius_m: f64,
    /// Maximum fraction of parent piece mass converted to dust in `[0, 0.5]`.
    pub max_dust_mass_fraction: f64,
}

/// Outcome of a secondary crumbling pass over a debris collection.
#[derive(Clone, Debug, Default)]
pub struct CrumbleReport {
    /// Number of parent pieces that split into smaller sub-pieces.
    pub crumbled_pieces: usize,
    /// Airborne dust particles shed during crumbling.
    pub particles: Vec<Particle>,
    /// Total mass (kg) transferred from rigid pieces into `particles`.
    pub dust_mass_kg: f64,
    /// Total fracture surface energy (J) consumed creating `particles`.
    pub surface_energy_j: f64,
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
    step_debris_with_obstacles(bodies, dt, gravity, ground, &[], ledger)
}

/// Advances every body by `dt` under `gravity` against `ground`, static voxel/box
/// `obstacles`, and all other debris hulls.
///
/// # Errors
/// Nonfinite or negative `dt`, invalid ground/obstacle bounds, or numerical overflow.
pub fn step_debris_with_obstacles(
    bodies: &mut [DebrisBody],
    dt: f64,
    gravity: Vec3,
    ground: &DebrisGround,
    obstacles: &[DebrisBoxObstacle],
    ledger: &mut DebrisStepReport,
) -> Result<(), &'static str> {
    if !dt.is_finite() || dt < 0. || gravity.iter().any(|g| !g.is_finite()) {
        return Err("invalid debris step");
    }
    if obstacles.iter().any(|o| !o.is_valid()) {
        return Err("invalid debris obstacle");
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

        // Static voxel/box obstacles: resolve SAT overlap between the debris
        // hull and each axis-aligned box obstacle.
        for obs in obstacles {
            let hull = body.world_hull();
            let box_verts = obs.vertices();
            let Some((point, obs_depth, obs_normal)) =
                sat_contact(&hull, &body.piece.hull_faces, &box_verts, &BOX_FACES)
            else {
                continue;
            };
            contacts += 1;
            let before = body.kinetic_j();
            let r = sub(point, body.position_m);
            let inv_i = body.inverse_inertia_world();
            let generalized = |direction: Vec3| {
                let rn = cross(r, direction);
                1. / body.piece.mass_kg + dot(rn, mat_vec(inv_i, rn))
            };
            let vel = add(body.velocity_m_s, cross(body.angular_velocity_rad_s, r));
            let vn = dot(vel, obs_normal);
            let mut impulse = [0.; 3];
            let mut normal_impulse = 0.;
            if vn < 0. {
                normal_impulse = -vn / generalized(obs_normal);
                impulse = scale(obs_normal, normal_impulse);
            }
            let tangential = sub(vel, scale(obs_normal, vn));
            let speed = norm(tangential);
            if speed > 0. && normal_impulse > 0. {
                let direction = scale(tangential, 1. / speed);
                let stopping = speed / generalized(direction);
                impulse = sub(
                    impulse,
                    scale(direction, stopping.min(obs.friction * normal_impulse)),
                );
            }
            body.velocity_m_s = add(body.velocity_m_s, scale(impulse, 1. / body.piece.mass_kg));
            body.angular_velocity_rad_s = add(
                body.angular_velocity_rad_s,
                mat_vec(inv_i, cross(r, impulse)),
            );
            ledger.dissipated_j += before - body.kinetic_j();
            let push = scale(obs_normal, obs_depth);
            body.position_m = add(body.position_m, push);
            ledger.correction_j += body.piece.mass_kg * -dot(gravity, push);
        }
    }
    let pair_contacts_before = ledger.contacts;
    ledger.contacts = 0;
    resolve_pairs(bodies, gravity, ground.friction, ledger);
    contacts += ledger.contacts;
    ledger.contacts = pair_contacts_before;
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

/// Secondary comminution (crumbling): splits eligible `DebrisBody` pieces into
/// smaller sub-pieces along their longest local axis and converts a fraction of
/// `impact_energy_j` into airborne dust grains (`Particle`), conserving total
/// mass and linear momentum.
///
/// # Errors
/// Rejects invalid [`CrumbleSettings`] or nonfinite `impact_energy_j`.
pub fn crumble_debris_on_impact(
    bodies: &mut Vec<DebrisBody>,
    impact_energy_j: f64,
    settings: CrumbleSettings,
) -> Result<CrumbleReport, &'static str> {
    if !impact_energy_j.is_finite()
        || impact_energy_j < 0.
        || !settings.min_impact_energy_j.is_finite()
        || settings.min_impact_energy_j <= 0.
        || !settings.min_piece_mass_kg.is_finite()
        || settings.min_piece_mass_kg <= 0.
        || !settings.density_kg_m3.is_finite()
        || settings.density_kg_m3 <= 0.
        || !settings.surface_energy_j_m2.is_finite()
        || settings.surface_energy_j_m2 <= 0.
        || !settings.dust_energy_fraction.is_finite()
        || !(0. ..=1.).contains(&settings.dust_energy_fraction)
        || !settings.grain_radius_m.is_finite()
        || settings.grain_radius_m <= 0.
        || !settings.max_dust_mass_fraction.is_finite()
        || !(0. ..=0.5).contains(&settings.max_dust_mass_fraction)
    {
        return Err("invalid crumble settings");
    }
    if impact_energy_j < settings.min_impact_energy_j || bodies.is_empty() {
        return Ok(CrumbleReport::default());
    }

    let mut report = CrumbleReport::default();
    let mut next_bodies = Vec::with_capacity(bodies.len() * 2);
    let per_body_energy = impact_energy_j / bodies.len() as f64;

    for body in bodies.drain(..) {
        if body.piece.mass_kg < 2.0 * settings.min_piece_mass_kg || body.piece.hull.len() < 4 {
            next_bodies.push(body);
            continue;
        }
        let world_hull = body.world_hull();
        let mut world_piece = body.piece.clone();
        world_piece.hull = world_hull.clone();
        world_piece.center_m = body.position_m;
        world_piece.velocity_m_s = body.velocity_m_s;
        let single_release = DebrisRelease {
            pieces: vec![world_piece],
            retained_nodes: Vec::new(),
            kinetic_j: body.kinetic_j(),
            deformation_kinetic_j: 0.,
            surface_created_m2: 0.,
        };
        let dust = fracture_dust(
            &single_release,
            FractureDustSettings {
                density_kg_m3: settings.density_kg_m3,
                surface_energy_j_m2: settings.surface_energy_j_m2,
                energy_fraction: 0.,
                extra_energy_j: settings.dust_energy_fraction * per_body_energy,
                grain_radius_m: settings.grain_radius_m,
                max_mass_fraction: settings.max_dust_mass_fraction,
            },
        )?;

        let remaining_mass = body.piece.mass_kg - dust.mass_kg;
        if remaining_mass < 2.0 * settings.min_piece_mass_kg {
            next_bodies.push(body);
            continue;
        }

        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &p in &world_hull {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        let extents = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let split_axis = if extents[0] >= extents[1] && extents[0] >= extents[2] {
            0
        } else if extents[1] >= extents[2] {
            1
        } else {
            2
        };
        let mid = 0.5 * (lo[split_axis] + hi[split_axis]);
        let child_mass = 0.5 * remaining_mass;
        let child_inertia = body
            .piece
            .inertia_kg_m2
            .map(|row| row.map(|v| 0.25 * v * (remaining_mass / body.piece.mass_kg)));

        let build_half = |upper: bool| -> Result<DebrisBody, &'static str> {
            let mut h_lo = lo;
            let mut h_hi = hi;
            if upper {
                h_lo[split_axis] = mid;
            } else {
                h_hi[split_axis] = mid;
            }
            let center = [
                0.5 * (h_lo[0] + h_hi[0]),
                0.5 * (h_lo[1] + h_hi[1]),
                0.5 * (h_lo[2] + h_hi[2]),
            ];
            let mut pts = Vec::with_capacity(8);
            for &x in &[h_lo[0], h_hi[0]] {
                for &y in &[h_lo[1], h_hi[1]] {
                    for &z in &[h_lo[2], h_hi[2]] {
                        pts.push([x, y, z]);
                    }
                }
            }
            let (hull, hull_faces) = convex_hull(&pts)?;
            let mut vel = body.velocity_m_s;
            let sign = if upper { 1.0 } else { -1.0 };
            vel[split_axis] += sign * 0.05;
            let piece = DebrisPiece {
                nodes: Vec::new(),
                hull,
                hull_faces,
                mass_kg: child_mass,
                inertia_kg_m2: child_inertia,
                center_m: center,
                velocity_m_s: vel,
                angular_velocity_rad_s: body.angular_velocity_rad_s,
                kinetic_j: 0.5 * child_mass * dot(vel, vel),
                deformation_kinetic_j: 0.,
            };
            DebrisBody::new(piece)
        };

        let left = build_half(false)?;
        let right = build_half(true)?;
        next_bodies.push(left);
        next_bodies.push(right);
        report.crumbled_pieces += 1;
        report.dust_mass_kg += dust.mass_kg;
        report.surface_energy_j += dust.surface_energy_j;
        report.particles.extend(dust.particles);
    }

    *bodies = next_bodies;
    Ok(report)
}

/// Multi-fragment stress-cone / Rosin-Rammler comminution (`крошение`):
/// Subdivides eligible `DebrisBody` pieces into 2, 4, or 8 polyhedral fragments
/// according to impact energy ratio (`impact_energy_j / min_impact_energy_j`),
/// assigning Weibull / Rosin-Rammler fragment mass weights while strictly conserving
/// total mass (fragments + airborne `Particle` dust) and linear momentum.
///
/// # Errors
/// Rejects invalid [`CrumbleSettings`] or nonfinite `impact_energy_j`.
pub fn crumble_debris_multi_fragment(
    bodies: &mut Vec<DebrisBody>,
    impact_energy_j: f64,
    settings: CrumbleSettings,
) -> Result<CrumbleReport, &'static str> {
    if !impact_energy_j.is_finite()
        || impact_energy_j < 0.
        || !settings.min_impact_energy_j.is_finite()
        || settings.min_impact_energy_j <= 0.
        || !settings.min_piece_mass_kg.is_finite()
        || settings.min_piece_mass_kg <= 0.
        || !settings.density_kg_m3.is_finite()
        || settings.density_kg_m3 <= 0.
        || !settings.surface_energy_j_m2.is_finite()
        || settings.surface_energy_j_m2 <= 0.
        || !settings.dust_energy_fraction.is_finite()
        || !(0. ..=1.).contains(&settings.dust_energy_fraction)
        || !settings.grain_radius_m.is_finite()
        || settings.grain_radius_m <= 0.
        || !settings.max_dust_mass_fraction.is_finite()
        || !(0. ..=0.5).contains(&settings.max_dust_mass_fraction)
    {
        return Err("invalid crumble settings");
    }
    if impact_energy_j < settings.min_impact_energy_j || bodies.is_empty() {
        return Ok(CrumbleReport::default());
    }

    let mut report = CrumbleReport::default();
    let mut next_bodies = Vec::with_capacity(bodies.len() * 4);
    let per_body_energy = impact_energy_j / bodies.len() as f64;
    let energy_ratio = per_body_energy / settings.min_impact_energy_j;

    for body in bodies.drain(..) {
        if body.piece.mass_kg < 2.0 * settings.min_piece_mass_kg || body.piece.hull.len() < 4 {
            next_bodies.push(body);
            continue;
        }
        let world_hull = body.world_hull();
        let mut world_piece = body.piece.clone();
        world_piece.hull = world_hull.clone();
        world_piece.center_m = body.position_m;
        world_piece.velocity_m_s = body.velocity_m_s;
        let single_release = DebrisRelease {
            pieces: vec![world_piece],
            retained_nodes: Vec::new(),
            kinetic_j: body.kinetic_j(),
            deformation_kinetic_j: 0.,
            surface_created_m2: 0.,
        };
        let dust = fracture_dust(
            &single_release,
            FractureDustSettings {
                density_kg_m3: settings.density_kg_m3,
                surface_energy_j_m2: settings.surface_energy_j_m2,
                energy_fraction: 0.,
                extra_energy_j: settings.dust_energy_fraction * per_body_energy,
                grain_radius_m: settings.grain_radius_m,
                max_mass_fraction: settings.max_dust_mass_fraction,
            },
        )?;

        let remaining_mass = body.piece.mass_kg - dust.mass_kg;
        if remaining_mass < 2.0 * settings.min_piece_mass_kg {
            next_bodies.push(body);
            continue;
        }

        // Choose subdivision grid (1x1x2 = 2, 2x1x2 = 4, or 2x2x2 = 8) limited by min_piece_mass_kg
        let max_children_by_mass = (remaining_mass / settings.min_piece_mass_kg).floor() as usize;
        let target_children = if energy_ratio >= 4.0 && max_children_by_mass >= 8 {
            8
        } else if energy_ratio >= 2.0 && max_children_by_mass >= 4 {
            4
        } else {
            2
        };

        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &p in &world_hull {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        let extents = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let mut axes = [0usize, 1, 2];
        axes.sort_by(|&a, &b| extents[b].total_cmp(&extents[a]));

        let (nx, ny, nz) = match target_children {
            8 => (2usize, 2usize, 2usize),
            4 => {
                let mut counts = [1usize; 3];
                counts[axes[0]] = 2;
                counts[axes[1]] = 2;
                (counts[0], counts[1], counts[2])
            }
            _ => {
                let mut counts = [1usize; 3];
                counts[axes[0]] = 2;
                (counts[0], counts[1], counts[2])
            }
        };

        // Rosin-Rammler / Weibull mass weights w_k = (-ln(1 - u_k))^(1/m_weibull)
        let weibull_m = 2.2_f64;
        let mut raw_weights = Vec::with_capacity(target_children);
        for k in 0..target_children {
            let u = ((k as f64) + 0.5) / (target_children as f64 + 0.5);
            let w = (-(1.0 - u).ln()).powf(1.0 / weibull_m).max(0.35);
            raw_weights.push(w);
        }
        let weight_sum: f64 = raw_weights.iter().sum();
        let floor_total = (target_children as f64) * settings.min_piece_mass_kg;
        let distributable = (remaining_mass - floor_total).max(0.0);
        let child_masses: Vec<f64> = raw_weights
            .iter()
            .map(|&w| settings.min_piece_mass_kg + distributable * (w / weight_sum))
            .collect();

        let parent_center = [
            0.5 * (lo[0] + hi[0]),
            0.5 * (lo[1] + hi[1]),
            0.5 * (lo[2] + hi[2]),
        ];
        let mut child_centers = Vec::with_capacity(target_children);
        let mut child_boxes = Vec::with_capacity(target_children);
        for ix in 0..nx {
            for iy in 0..ny {
                for iz in 0..nz {
                    let fx0 = ix as f64 / nx as f64;
                    let fx1 = (ix + 1) as f64 / nx as f64;
                    let fy0 = iy as f64 / ny as f64;
                    let fy1 = (iy + 1) as f64 / ny as f64;
                    let fz0 = iz as f64 / nz as f64;
                    let fz1 = (iz + 1) as f64 / nz as f64;
                    let c_lo = [
                        lo[0] + fx0 * extents[0],
                        lo[1] + fy0 * extents[1],
                        lo[2] + fz0 * extents[2],
                    ];
                    let c_hi = [
                        lo[0] + fx1 * extents[0],
                        lo[1] + fy1 * extents[1],
                        lo[2] + fz1 * extents[2],
                    ];
                    let center = [
                        0.5 * (c_lo[0] + c_hi[0]),
                        0.5 * (c_lo[1] + c_hi[1]),
                        0.5 * (c_lo[2] + c_hi[2]),
                    ];
                    child_centers.push(center);
                    child_boxes.push((c_lo, c_hi));
                }
            }
        }

        // Compute outward stress-cone radial velocity offsets and subtract mass-weighted mean
        // so sum(m_k * dv_k) == 0 to machine precision.
        let mut dv_list = Vec::with_capacity(target_children);
        let mut weighted_dv = [0.0; 3];
        for (center, &m_k) in child_centers.iter().zip(&child_masses) {
            let dir = sub(*center, parent_center);
            let d_norm = norm(dir);
            let dv = if d_norm > 1e-12 {
                scale(dir, 0.06 / d_norm)
            } else {
                [0.0; 3]
            };
            weighted_dv = add(weighted_dv, scale(dv, m_k));
            dv_list.push(dv);
        }
        let dv_correction = scale(weighted_dv, 1.0 / remaining_mass);

        for ((c_lo, c_hi), (center, (&child_mass, dv_raw))) in child_boxes.into_iter().zip(
            child_centers
                .into_iter()
                .zip(child_masses.iter().zip(dv_list)),
        ) {
            let mut pts = Vec::with_capacity(8);
            for &x in &[c_lo[0], c_hi[0]] {
                for &y in &[c_lo[1], c_hi[1]] {
                    for &z in &[c_lo[2], c_hi[2]] {
                        pts.push([x, y, z]);
                    }
                }
            }
            let (hull, hull_faces) = convex_hull(&pts)?;
            let dv = sub(dv_raw, dv_correction);
            let vel = add(body.velocity_m_s, dv);
            let mass_fraction = child_mass / body.piece.mass_kg;
            let child_inertia = body
                .piece
                .inertia_kg_m2
                .map(|row| row.map(|v| 0.5 * v * mass_fraction));
            let piece = DebrisPiece {
                nodes: Vec::new(),
                hull,
                hull_faces,
                mass_kg: child_mass,
                inertia_kg_m2: child_inertia,
                center_m: center,
                velocity_m_s: vel,
                angular_velocity_rad_s: body.angular_velocity_rad_s,
                kinetic_j: 0.5 * child_mass * dot(vel, vel),
                deformation_kinetic_j: 0.,
            };
            next_bodies.push(DebrisBody::new(piece)?);
        }

        report.crumbled_pieces += 1;
        report.dust_mass_kg += dust.mass_kg;
        report.surface_energy_j += dust.surface_energy_j;
        report.particles.extend(dust.particles);
    }

    *bodies = next_bodies;
    Ok(report)
}

/// Settings for geometric Archard abrasive wear (`стираемость`) of rigid debris hulls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisAbrasionSettings {
    /// Indentation hardness $H$ (Pa).
    pub hardness_pa: f64,
    /// Dimensionless Archard wear coefficient $k$.
    pub archard_coefficient: f64,
    /// Solid material density $\rho$ (kg/m³).
    pub density_kg_m3: f64,
    /// Radius (m) of emitted abrasive dust particles.
    pub grain_radius_m: f64,
    /// Maximum fraction of body mass that can be abraded in a single step in `(0, 0.5]`.
    pub max_mass_fraction_per_step: f64,
}

/// Outcome of geometric Archard hull abrasion over a debris collection.
#[derive(Clone, Debug, Default)]
pub struct DebrisAbrasionReport {
    pub abraded_bodies: usize,
    pub removed_volume_m3: f64,
    pub removed_mass_kg: f64,
    pub max_recession_depth_m: f64,
    pub particles: Vec<Particle>,
}

/// Applies Archard abrasive wear ($dV = k F_n \Delta s / H$) to `bodies` sliding against
/// `ground`, receding the contacting convex hull vertices inward along the contact normal,
/// reducing `piece.mass_kg` and `piece.inertia_kg_m2`, and emitting airborne [`Particle`]
/// dust grains that conserve total mass (`remaining_body_mass + dust_mass == initial_mass`)
/// and linear momentum.
///
/// # Errors
/// Rejects invalid settings, negative load/sliding distance, or degenerate hull geometry.
pub fn abrade_debris_hull(
    bodies: &mut [DebrisBody],
    ground: &DebrisGround,
    normal_load_n: f64,
    sliding_distance_m: f64,
    settings: DebrisAbrasionSettings,
) -> Result<DebrisAbrasionReport, &'static str> {
    if !normal_load_n.is_finite()
        || normal_load_n < 0.
        || !sliding_distance_m.is_finite()
        || sliding_distance_m < 0.
        || !settings.hardness_pa.is_finite()
        || settings.hardness_pa <= 0.
        || !settings.archard_coefficient.is_finite()
        || settings.archard_coefficient < 0.
        || !settings.density_kg_m3.is_finite()
        || settings.density_kg_m3 <= 0.
        || !settings.grain_radius_m.is_finite()
        || settings.grain_radius_m <= 0.
        || !settings.max_mass_fraction_per_step.is_finite()
        || !(0.0..=0.5).contains(&settings.max_mass_fraction_per_step)
    {
        return Err("invalid debris abrasion settings or load");
    }
    let n_len = norm(ground.normal);
    if !n_len.is_finite() || n_len == 0. || !ground.offset_m.is_finite() {
        return Err("invalid debris ground for abrasion");
    }
    if normal_load_n == 0.
        || sliding_distance_m == 0.
        || settings.archard_coefficient == 0.
        || bodies.is_empty()
    {
        return Ok(DebrisAbrasionReport::default());
    }

    let world_normal = scale(ground.normal, 1.0 / n_len);
    let grain_mass = (4.0 / 3.0)
        * std::f64::consts::PI
        * settings.grain_radius_m.powi(3)
        * settings.density_kg_m3;
    if !grain_mass.is_finite() || grain_mass <= 0. {
        return Err("unrepresentable abrasion dust grain mass");
    }

    let mut report = DebrisAbrasionReport::default();
    for body in bodies.iter_mut() {
        let local_normal = mat_vec(transpose(body.rotation), world_normal);
        let mut min_proj = f64::INFINITY;
        let mut max_proj = f64::NEG_INFINITY;
        for &v in &body.piece.hull {
            let proj = dot(v, local_normal);
            min_proj = min_proj.min(proj);
            max_proj = max_proj.max(proj);
        }
        let span = max_proj - min_proj;
        if !span.is_finite() || span <= 1e-9 {
            continue;
        }

        let raw_volume = settings.archard_coefficient
            * (normal_load_n / settings.hardness_pa)
            * sliding_distance_m;
        let max_mass = body.piece.mass_kg * settings.max_mass_fraction_per_step;
        let raw_mass = (raw_volume * settings.density_kg_m3).min(max_mass);
        let grain_count = (raw_mass / grain_mass).floor() as usize;
        if grain_count == 0 {
            continue;
        }
        let removed_mass = (grain_count as f64) * grain_mass;
        let removed_volume = removed_mass / settings.density_kg_m3;
        let mass_fraction = removed_mass / body.piece.mass_kg;
        let recession_depth = (span * mass_fraction).min(0.4 * span);

        // Recede vertices near the bottom contact patch inward along local_normal
        let contact_band = min_proj + 0.25 * span;
        let mut new_pts = body.piece.hull.clone();
        let mut contact_centroid_local = [0.0; 3];
        let mut touched_count = 0.0;
        for pt in &mut new_pts {
            let proj = dot(*pt, local_normal);
            if proj <= contact_band {
                contact_centroid_local = add(contact_centroid_local, *pt);
                touched_count += 1.0;
                *pt = add(*pt, scale(local_normal, recession_depth));
            }
        }
        if touched_count == 0.0 {
            continue;
        }
        contact_centroid_local = scale(contact_centroid_local, 1.0 / touched_count);
        let contact_world = body.world_point(contact_centroid_local);

        let (new_hull, new_faces) = convex_hull(&new_pts)?;
        let remaining_ratio = (1.0 - mass_fraction).max(0.5);
        let new_inertia = body
            .piece
            .inertia_kg_m2
            .map(|row| row.map(|v| v * remaining_ratio));
        let Some(new_inv_inertia) = invert(new_inertia) else {
            continue;
        };

        body.piece.hull = new_hull;
        body.piece.hull_faces = new_faces;
        body.piece.mass_kg -= removed_mass;
        body.piece.inertia_kg_m2 = new_inertia;
        body.inverse_inertia_body = new_inv_inertia;

        // Emit up to 64 representative dust particles (or exact grain_count when small)
        // when grain_count <= 64, each particle has grain_radius_m; when larger, scale radius so
        // total particle mass equals removed_mass exactly.
        let emit_count = grain_count.min(64);
        let eff_mass_per_particle = removed_mass / (emit_count as f64);
        let eff_radius = ((3.0 * eff_mass_per_particle)
            / (4.0 * std::f64::consts::PI * settings.density_kg_m3))
            .cbrt();
        for idx in 0..emit_count {
            let offset = (idx as f64 - 0.5 * (emit_count as f64)) * 1e-4;
            let pos = [
                contact_world[0] + offset,
                contact_world[1],
                contact_world[2],
            ];
            let p = Particle::new(eff_radius, settings.density_kg_m3, pos, body.velocity_m_s)?;
            report.particles.push(p);
        }

        report.abraded_bodies += 1;
        report.removed_volume_m3 += removed_volume;
        report.removed_mass_kg += removed_mass;
        report.max_recession_depth_m = report.max_recession_depth_m.max(recession_depth);
    }

    Ok(report)
}

/// Voxel/world contact settings for [`step_debris_in_world`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisWorldContact {
    pub friction: f64,
    /// Sweep candidate budget passed to the collision backend.
    pub max_candidates: usize,
    /// Contacts resolved per body per step before the remainder is dropped.
    pub max_contacts: usize,
}

impl Default for DebrisWorldContact {
    fn default() -> Self {
        Self {
            friction: 0.6,
            max_candidates: 16_384,
            max_contacts: 4,
        }
    }
}

/// Advances every body against a voxel [`CollisionWorld`](crate::CollisionWorld):
/// the hull's axis-aligned bounds are swept by the step displacement, each hit
/// stops the normal motion with an inelastic impulse (Coulomb friction on the
/// support patch), and the remaining displacement slides along the surface.
/// Piece–piece contacts are resolved afterwards as in [`step_debris`].
/// Units follow the collision world (voxel units for `physics_voxel`).
/// # Errors
/// Invalid step, a body initially overlapping the world, or backend failures.
pub fn step_debris_in_world(
    bodies: &mut [DebrisBody],
    dt: f64,
    gravity: Vec3,
    contact: DebrisWorldContact,
    world: &impl crate::CollisionWorld,
    ledger: &mut DebrisStepReport,
) -> Result<(), &'static str> {
    if !dt.is_finite()
        || dt < 0.
        || gravity.iter().any(|g| !g.is_finite())
        || !contact.friction.is_finite()
        || contact.friction < 0.
        || contact.max_candidates == 0
        || contact.max_contacts == 0
    {
        return Err("invalid debris world step");
    }
    let mechanical = |bodies: &[DebrisBody]| {
        bodies
            .iter()
            .map(|b| b.kinetic_j() + b.piece.mass_kg * -dot(gravity, b.position_m))
            .sum::<f64>()
    };
    let start = mechanical(bodies);
    ledger.initial_j.get_or_insert(start);
    let (dissipated_before, correction_before) = (ledger.dissipated_j, ledger.correction_j);
    let mut contacts = 0;
    for body in bodies.iter_mut() {
        let old_velocity = body.velocity_m_s;
        let old_spin = body.angular_velocity_rad_s;
        body.velocity_m_s = add(body.velocity_m_s, scale(gravity, dt));
        // Free flight uses the trapezoidal (exact) displacement.
        let mut displacement = scale(add(old_velocity, body.velocity_m_s), 0.5 * dt);
        let mut spin = scale(add(old_spin, body.angular_velocity_rad_s), 0.5);
        for _ in 0..contact.max_contacts {
            let hull = body.world_hull();
            let (min, max) = hull.iter().fold(
                ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]),
                |(lo, hi), p| {
                    (
                        [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
                        [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
                    )
                },
            );
            let hit = world
                .sweep_aabb(
                    crate::AnchoredAabb {
                        anchor: crate::Origin::default(),
                        min,
                        max,
                    },
                    displacement,
                    contact.max_candidates,
                )
                .map_err(|_| "debris collision backend failure")?;
            if !hit.fraction.is_finite() || !(0. ..=1.).contains(&hit.fraction) {
                return Err("invalid debris sweep result");
            }
            let axis_count = hit.normal.iter().filter(|n| **n != 0).count();
            if axis_count == 0 {
                if hit.obstacle.is_some() {
                    return Err("debris initially overlaps the world");
                }
                body.position_m = add(body.position_m, displacement);
                break;
            }
            // Advance to the contact, then resolve the impulse on the support patch.
            body.position_m = add(body.position_m, scale(displacement, hit.fraction));
            contacts += 1;
            let normal = [
                f64::from(hit.normal[0]),
                f64::from(hit.normal[1]),
                f64::from(hit.normal[2]),
            ];
            let hull = body.world_hull();
            let lowest = hull
                .iter()
                .map(|p| dot(*p, normal))
                .fold(f64::INFINITY, f64::min);
            let mut sum = [0.; 3];
            let mut count: f64 = 0.;
            for p in &hull {
                if dot(*p, normal) <= lowest + 1e-9 * (1. + lowest.abs()) {
                    sum = add(sum, *p);
                    count += 1.;
                }
            }
            let point = scale(sum, 1. / count.max(1.));
            let patch = count >= 3.;
            let before = body.kinetic_j();
            if patch {
                let spin_n = dot(body.angular_velocity_rad_s, normal);
                body.angular_velocity_rad_s = scale(normal, spin_n);
            }
            let r = sub(point, body.position_m);
            let inverse_inertia = body.inverse_inertia_world();
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
                    scale(direction, stopping.min(contact.friction * normal_impulse)),
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
            // The remainder of the step slides along the surface at the new velocity.
            let remaining = 1. - hit.fraction;
            displacement = scale(body.velocity_m_s, remaining * dt);
            let dn = dot(displacement, normal);
            if dn < 0. {
                displacement = sub(displacement, scale(normal, dn));
            }
            spin = body.angular_velocity_rad_s;
            if norm(displacement) == 0. {
                break;
            }
        }
        body.rotation = rotate(body.rotation, scale(spin, dt));
    }
    // Piece–piece contacts reuse the ground-free path with the world friction.
    let mut pair_ledger = DebrisStepReport::default();
    resolve_pairs(bodies, gravity, contact.friction, &mut pair_ledger);
    ledger.dissipated_j += pair_ledger.dissipated_j;
    ledger.correction_j += pair_ledger.correction_j;
    contacts += pair_ledger.contacts;
    let mut kinetic = 0.;
    let mut potential = 0.;
    for body in bodies.iter() {
        kinetic += body.kinetic_j();
        potential += body.piece.mass_kg * -dot(gravity, body.position_m);
    }
    if !kinetic.is_finite() || !potential.is_finite() || !ledger.dissipated_j.is_finite() {
        return Err("debris world step overflow");
    }
    let booked =
        (ledger.dissipated_j - dissipated_before) - (ledger.correction_j - correction_before);
    ledger.integration_j += (kinetic + potential + booked) - start;
    ledger.kinetic_j = kinetic;
    ledger.potential_j = potential;
    ledger.contacts = contacts;
    Ok(())
}

/// Piece–piece contacts (see [`step_debris`]); increments `ledger.contacts`
/// and books dissipation and push-out potential into the ledger.
fn resolve_pairs(
    bodies: &mut [DebrisBody],
    gravity: Vec3,
    friction: f64,
    ledger: &mut DebrisStepReport,
) {
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
            ledger.contacts += 1;
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
                    scale(direction, stopping.min(friction * normal_impulse)),
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
