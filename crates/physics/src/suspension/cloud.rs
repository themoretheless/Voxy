use super::{FiniteCarrier, Particle};
#[derive(Clone, Copy, Debug)]
pub struct CloudExchange {
    pub viscous_heat_j: f64,
    /// Backward-Euler damping, kept separate from physical heat.
    pub numerical_loss_j: f64,
    /// Work of prescribed body forces at accepted endpoint velocities.
    pub body_force_work_j: f64,
    pub energy_defect_j: f64,
}
/// Simultaneous backward-Euler drag for a dilute cloud in one finite carrier.
/// Shared carrier velocity is solved once, so particle order does not determine
/// fluid feedback. Particle drift uses accepted endpoint velocity (first order).
/// # Errors
/// Invalid/bounded-work settings, Re>0.1 at either endpoint, volume fraction>0.01,
/// precision or balance failure. Entire cloud and carrier roll back together.
pub fn exchange_drag_cloud(
    particles: &mut [Particle],
    carrier: &mut FiniteCarrier,
    dt_s: f64,
) -> Result<CloudExchange, &'static str> {
    if particles.len() > 4096 {
        return Err("suspension cloud budget exceeded");
    }
    exchange_forced_cloud(
        particles,
        carrier,
        dt_s,
        &vec![[0.; 3]; particles.len()],
        [0.; 3],
    )
}
/// Shared-carrier implicit drag with constant prescribed body accelerations.
/// Forces perform work at endpoint velocities, consistent with backward Euler.
/// # Errors
/// Invalid accelerations, drag regime, budgets or balance; atomic for both phases.
pub fn exchange_forced_cloud(
    particles: &mut [Particle],
    carrier: &mut FiniteCarrier,
    dt_s: f64,
    particle_accelerations: &[[f64; 3]],
    carrier_acceleration: [f64; 3],
) -> Result<CloudExchange, &'static str> {
    if particle_accelerations.len() != particles.len()
        || !particle_accelerations
            .iter()
            .flatten()
            .chain(carrier_acceleration.iter())
            .all(|x| x.is_finite())
    {
        return Err("invalid cloud body acceleration");
    }
    if particles.len() > 4096 || !dt_s.is_finite() || dt_s <= 0. {
        return Err("invalid suspension cloud step");
    }
    let volume: f64 = particles.iter().map(|p| p.mass_kg / p.density_kg_m3).sum();
    if !volume.is_finite() || volume / carrier.volume_m3 > 0.01 {
        return Err("suspension cloud is not dilute");
    }
    let old_u = carrier.medium.velocity_m_s;
    let predicted_u: [f64; 3] = std::array::from_fn(|i| old_u[i] + dt_s * carrier_acceleration[i]);
    let mut coefficients = Vec::with_capacity(particles.len());
    let mut denominator = carrier.mass_kg;
    let mut rhs = [0.; 3];
    for (p, acceleration) in particles.iter().zip(particle_accelerations) {
        let beta_dt = 6. * std::f64::consts::PI * carrier.medium.viscosity_pa_s * p.radius_m * dt_s;
        if !beta_dt.is_finite() || beta_dt <= 0. {
            return Err("unrepresentable cloud drag coefficient");
        }
        if !(p.mass_kg + beta_dt).is_finite() {
            return Err("cloud drag mass overflow");
        }
        let gamma = beta_dt / (p.mass_kg + beta_dt);
        if gamma <= 0. {
            return Err("unrepresentable cloud relaxation fraction");
        }
        let weight = p.mass_kg * gamma;
        denominator += weight;
        for i in 0..3 {
            rhs[i] += weight * (p.velocity_m_s[i] + dt_s * acceleration[i] - predicted_u[i]);
        }
        coefficients.push((gamma, beta_dt));
    }
    if !denominator.is_finite() || denominator <= 0. {
        return Err("cloud carrier solve overflow");
    }
    let u: [f64; 3] = std::array::from_fn(|i| predicted_u[i] + rhs[i] / denominator);
    let du: [f64; 3] = std::array::from_fn(|i| u[i] - old_u[i]);
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f64>();
    // Velocity storage is in the laboratory frame. Near co-motion its rounding
    // error can exceed the relative kinetic change, even in a moving-frame audit.
    let mut energy_roundoff = (0..3)
        .map(|i| {
            8. * f64::EPSILON
                * carrier.mass_kg
                * (old_u[i].abs() + u[i].abs())
                * (du[i].abs() + dt_s * carrier_acceleration[i].abs())
        })
        .sum::<f64>();
    let mut next = particles.to_vec();
    let mut heat = 0.;
    let mut numerical = 0.5 * carrier.mass_kg * dot(du, du);
    let mut kinetic = 0.5 * carrier.mass_kg * dot(du, du);
    let mut laboratory_work = dt_s * carrier.mass_kg * dot(carrier_acceleration, u);
    let mut work = dt_s * carrier.mass_kg * dot(carrier_acceleration, du);
    let mut momentum: [f64; 3] =
        std::array::from_fn(|i| carrier.mass_kg * (du[i] - dt_s * carrier_acceleration[i]));
    let mut impulse_scale: [f64; 3] = std::array::from_fn(|i| {
        momentum[i].abs() + (dt_s * carrier.mass_kg * carrier_acceleration[i]).abs()
    });
    let mut momentum_roundoff: [f64; 3] = std::array::from_fn(|i| {
        2. * f64::EPSILON * carrier.mass_kg * (old_u[i].abs() + u[i].abs())
    });
    for (((p, candidate), (gamma, beta_dt)), acceleration) in particles
        .iter()
        .zip(&mut next)
        .zip(coefficients)
        .zip(particle_accelerations)
    {
        let predicted: [f64; 3] =
            std::array::from_fn(|i| p.velocity_m_s[i] + dt_s * acceleration[i]);
        let v = std::array::from_fn(|i| predicted[i] + gamma * (u[i] - predicted[i]));
        let dv: [f64; 3] = std::array::from_fn(|i| v[i] - p.velocity_m_s[i]);
        let old_w: [f64; 3] = std::array::from_fn(|i| p.velocity_m_s[i] - old_u[i]);
        let w: [f64; 3] = std::array::from_fn(|i| v[i] - u[i]);
        let speed = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
        let re = 2. * p.radius_m * carrier.medium.density_kg_m3 / carrier.medium.viscosity_pa_s
            * speed(old_w).max(speed(w));
        if !re.is_finite() || re > 0.1 {
            return Err("cloud particle exceeds Stokes flow regime");
        }
        energy_roundoff += (0..3)
            .map(|i| {
                8. * f64::EPSILON
                    * p.mass_kg
                    * (p.velocity_m_s[i].abs() + v[i].abs())
                    * (old_w[i].abs() + dv[i].abs() + dt_s * acceleration[i].abs())
            })
            .sum::<f64>();
        heat += beta_dt * dot(w, w);
        numerical += 0.5 * p.mass_kg * dot(dv, dv);
        kinetic += p.mass_kg * dot(dv, std::array::from_fn(|i| old_w[i] + 0.5 * dv[i]));
        laboratory_work += dt_s * p.mass_kg * dot(*acceleration, v);
        work += dt_s * p.mass_kg * dot(*acceleration, std::array::from_fn(|i| v[i] - old_u[i]));
        for i in 0..3 {
            let reaction = p.mass_kg * (dv[i] - dt_s * acceleration[i]);
            momentum[i] += reaction;
            impulse_scale[i] += reaction.abs() + (dt_s * p.mass_kg * acceleration[i]).abs();
            momentum_roundoff[i] +=
                2. * f64::EPSILON * p.mass_kg * (p.velocity_m_s[i].abs() + v[i].abs());
        }
        candidate.velocity_m_s = v;
        candidate.position_m = std::array::from_fn(|i| p.position_m[i] + dt_s * v[i]);
        if !candidate
            .position_m
            .iter()
            .chain(v.iter())
            .all(|v| v.is_finite())
        {
            return Err("cloud particle drift overflow");
        }
    }
    let defect = kinetic + heat + numerical - work;
    let new_heat = carrier.deposited_heat_j + heat;
    let new_numerical = carrier.numerical_loss_j + numerical;
    if !u.iter().all(|v| v.is_finite())
        || ![
            heat,
            numerical,
            work,
            laboratory_work,
            defect,
            new_heat,
            new_numerical,
        ]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("cloud drag energy or velocity overflow");
    }
    if defect.abs()
        > 1e-10 * (kinetic.abs() + heat + numerical + work.abs()).max(1e-300) + energy_roundoff
    {
        return Err("cloud drag energy balance failure");
    }
    if (0..3)
        .any(|i| momentum[i].abs() > 1e-10 * impulse_scale[i].max(1e-300) + momentum_roundoff[i])
    {
        return Err("cloud drag momentum balance failure");
    }
    if heat > 0. && new_heat <= carrier.deposited_heat_j {
        return Err("cloud drag heat inventory precision failure");
    }
    if numerical > 0. && new_numerical <= carrier.numerical_loss_j {
        return Err("cloud drag numerical inventory precision failure");
    }
    particles.clone_from_slice(&next);
    carrier.medium.velocity_m_s = u;
    carrier.deposited_heat_j = new_heat;
    carrier.numerical_loss_j = new_numerical;
    Ok(CloudExchange {
        viscous_heat_j: heat,
        numerical_loss_j: numerical,
        body_force_work_j: laboratory_work,
        energy_defect_j: defect,
    })
}

/// Gravity plus hydrostatic buoyancy on grains, with equal opposite buoyancy
/// reaction on the finite carrier. This assumes a locally hydrostatic pressure
/// gradient; it must not also be applied through a separate pressure-force model.
/// Carrier translation is left to its owning spatial solver.
/// # Errors
/// Invalid gravity or any forced-cloud validation failure; atomic update.
pub fn exchange_gravity_cloud(
    particles: &mut [Particle],
    carrier: &mut FiniteCarrier,
    dt_s: f64,
    gravity: [f64; 3],
) -> Result<CloudExchange, &'static str> {
    if particles.len() > 4096 || !gravity.iter().all(|x| x.is_finite()) {
        return Err("invalid gravity cloud settings");
    }
    let displaced_mass: f64 = particles
        .iter()
        .map(|p| carrier.medium.density_kg_m3 * p.mass_kg / p.density_kg_m3)
        .sum();
    let carrier_acceleration = gravity.map(|g| g * (1. + displaced_mass / carrier.mass_kg));
    let accelerations: Vec<_> = particles
        .iter()
        .map(|p| gravity.map(|g| g * (1. - carrier.medium.density_kg_m3 / p.density_kg_m3)))
        .collect();
    exchange_forced_cloud(
        particles,
        carrier,
        dt_s,
        &accelerations,
        carrier_acceleration,
    )
}
