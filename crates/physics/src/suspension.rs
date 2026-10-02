//! Dilute spherical particles in a prescribed uniform viscous carrier (SI).
//! Creeping-flow Stokes drag; no turbulence, Brownian motion or phase change.
#[derive(Clone, Copy, Debug)]
pub struct Carrier {
    pub density_kg_m3: f64,
    pub viscosity_pa_s: f64,
    pub velocity_m_s: [f64; 3],
}
#[derive(Clone, Debug)]
pub struct Particle {
    radius_m: f64,
    density_kg_m3: f64,
    mass_kg: f64,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Step {
    /// Reaction impulse from drag plus buoyancy; caller's carrier must receive it.
    pub carrier_impulse_n_s: [f64; 3],
    pub maximum_reynolds: f64,
    pub kinetic_change_j: f64,
    pub viscous_heat_j: f64,
    /// Gravity plus buoyancy force work on the particle.
    pub effective_force_work_j: f64,
    /// Signed work delivered to the particle by carrier translation during drag.
    pub carrier_work_j: f64,
    pub energy_defect_j: f64,
}
impl Particle {
    /// # Errors
    /// Invalid or unrepresentable sphere geometry, density or state.
    pub fn new(
        radius_m: f64,
        density_kg_m3: f64,
        position_m: [f64; 3],
        velocity_m_s: [f64; 3],
    ) -> Result<Self, &'static str> {
        let mass = (4. / 3.) * std::f64::consts::PI * radius_m.powi(3) * density_kg_m3;
        if !radius_m.is_finite()
            || radius_m <= 0.
            || !density_kg_m3.is_finite()
            || density_kg_m3 <= 0.
            || !mass.is_finite()
            || mass <= 0.
            || !position_m
                .iter()
                .chain(velocity_m_s.iter())
                .all(|v| v.is_finite())
        {
            return Err("invalid suspended particle");
        }
        Ok(Self {
            radius_m,
            density_kg_m3,
            mass_kg: mass,
            position_m,
            velocity_m_s,
        })
    }
    #[must_use]
    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }
    /// Incompressible spherical solid volume, in cubic metres.
    #[must_use]
    pub fn volume_m3(&self) -> f64 {
        self.mass_kg / self.density_kg_m3
    }
    #[must_use]
    pub fn surface_area_m2(&self) -> f64 {
        4. * std::f64::consts::PI * self.radius_m * self.radius_m
    }
    #[must_use]
    pub fn position_m(&self) -> [f64; 3] {
        self.position_m
    }
    #[must_use]
    pub fn velocity_m_s(&self) -> [f64; 3] {
        self.velocity_m_s
    }
    /// Exact exponential velocity and integrated displacement for constant
    /// carrier/gravity. Validity is checked over the full interval: relative
    /// velocity follows a line segment, whose norm is bounded by its endpoints.
    /// Re must be <=0.1; outside this conservative creeping-flow regime reject.
    /// # Errors
    /// Invalid medium/time/gravity, regime violation or overflow; state is atomic.
    pub fn advance(
        &mut self,
        dt_s: f64,
        carrier: Carrier,
        gravity_m_s2: [f64; 3],
    ) -> Result<Step, &'static str> {
        if !dt_s.is_finite()
            || dt_s <= 0.
            || !carrier.density_kg_m3.is_finite()
            || carrier.density_kg_m3 <= 0.
            || !carrier.viscosity_pa_s.is_finite()
            || carrier.viscosity_pa_s <= 0.
            || !carrier
                .velocity_m_s
                .iter()
                .chain(gravity_m_s2.iter())
                .all(|v| v.is_finite())
        {
            return Err("invalid suspension medium or step");
        }
        let rate =
            6. * std::f64::consts::PI * carrier.viscosity_pa_s * self.radius_m / self.mass_kg;
        if !rate.is_finite() || rate <= 0. {
            return Err("unrepresentable particle relaxation");
        }
        let response = -(-rate * dt_s).exp_m1();
        let integral = response / rate;
        // dt-integral loses precision for extremely small relaxation timespans.
        let x = rate * dt_s;
        let accel_integral = if x < 1e-3 {
            dt_s * dt_s * (0.5 - x / 6. + x * x / 24. - x * x * x / 120.)
        } else {
            (dt_s - integral) / rate
        };
        let buoyancy = 1. - carrier.density_kg_m3 / self.density_kg_m3;
        let acceleration = gravity_m_s2.map(|g| g * buoyancy);
        let relative: [f64; 3] =
            std::array::from_fn(|i| self.velocity_m_s[i] - carrier.velocity_m_s[i]);
        let new_relative: [f64; 3] =
            std::array::from_fn(|i| relative[i] * (1. - response) + acceleration[i] * integral);
        let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
        let reynolds = 2. * self.radius_m * carrier.density_kg_m3 / carrier.viscosity_pa_s
            * norm(relative).max(norm(new_relative));
        if !reynolds.is_finite() || reynolds > 0.1 {
            return Err("particle exceeds Stokes flow regime");
        }
        let velocity = std::array::from_fn(|i| carrier.velocity_m_s[i] + new_relative[i]);
        let position = std::array::from_fn(|i| {
            self.position_m[i]
                + carrier.velocity_m_s[i] * dt_s
                + relative[i] * integral
                + acceleration[i] * accel_integral
        });
        let impulse = std::array::from_fn(|i| {
            self.mass_kg * (gravity_m_s2[i] * dt_s - (velocity[i] - self.velocity_m_s[i]))
        });
        let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f64>();
        let dv = std::array::from_fn(|i| velocity[i] - self.velocity_m_s[i]);
        let energy = 0.5
            * self.mass_kg
            * dot(
                dv,
                std::array::from_fn(|i| velocity[i] + self.velocity_m_s[i]),
            );
        // Integrate the squared relative velocity as a positive Gram form,
        // avoiding cancellation between terminal and transient terms.
        let ee = -(-2. * x).exp_m1() / 2.;
        let ef = response * response / (2. * rate);
        let ff = if x < 1e-3 {
            rate * dt_s.powi(3)
                * (1. / 3. - x / 4. + 7. * x * x / 60. - x * x * x / 24. + 31. * x.powi(4) / 2520.)
        } else {
            (dt_s - 2. * integral + ee / rate) / rate
        };
        if ee <= 0. || !ee.is_finite() || !ef.is_finite() || !ff.is_finite() {
            return Err("unrepresentable suspension heat integral");
        }
        let residual = ff - ef * ef / ee;
        if residual < -1e-12 * ff.abs() {
            return Err("invalid suspension dissipation integral");
        }
        let shifted = std::array::from_fn(|i| relative[i] + (ef / ee) * acceleration[i]);
        let heat = self.mass_kg
            * (ee * dot(shifted, shifted) + residual.max(0.) * dot(acceleration, acceleration));
        let displacement = std::array::from_fn(|i| {
            carrier.velocity_m_s[i] * dt_s
                + relative[i] * integral
                + acceleration[i] * accel_integral
        });
        let force_work = self.mass_kg * dot(acceleration, displacement);
        let drag_impulse = std::array::from_fn(|i| self.mass_kg * (dv[i] - acceleration[i] * dt_s));
        let carrier_work = dot(carrier.velocity_m_s, drag_impulse);
        let defect = energy + heat - force_work - carrier_work;
        let scale = energy.abs() + heat.abs() + force_work.abs() + carrier_work.abs();
        if ![heat, force_work, carrier_work, defect]
            .iter()
            .all(|v| v.is_finite())
            || heat < 0.
            || defect.abs() > 1e-10 * scale.max(1e-300)
        {
            return Err("suspension energy balance failure");
        }
        if !position
            .iter()
            .chain(velocity.iter())
            .chain(impulse.iter())
            .all(|v| v.is_finite())
            || !energy.is_finite()
        {
            return Err("suspension step overflow");
        }
        self.position_m = position;
        self.velocity_m_s = velocity;
        Ok(Step {
            carrier_impulse_n_s: impulse,
            maximum_reynolds: reynolds,
            kinetic_change_j: energy,
            viscous_heat_j: heat,
            effective_force_work_j: force_work,
            carrier_work_j: carrier_work,
            energy_defect_j: defect,
        })
    }
}

/// Homogeneous finite carrier control volume for local two-way drag exchange.
/// Its thermal field stores deposited energy; no temperature/EOS is inferred.
#[derive(Clone, Debug)]
pub struct FiniteCarrier {
    medium: Carrier,
    volume_m3: f64,
    mass_kg: f64,
    deposited_heat_j: f64,
    numerical_loss_j: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct DragExchange {
    pub particle_impulse_n_s: [f64; 3],
    pub deposited_heat_j: f64,
    pub energy_defect_j: f64,
}
impl FiniteCarrier {
    /// # Errors
    /// Invalid carrier parameters or unrepresentable finite mass.
    pub fn new(medium: Carrier, volume_m3: f64) -> Result<Self, &'static str> {
        let mass = medium.density_kg_m3 * volume_m3;
        if !volume_m3.is_finite()
            || volume_m3 <= 0.
            || !mass.is_finite()
            || mass <= 0.
            || !medium.density_kg_m3.is_finite()
            || medium.density_kg_m3 <= 0.
            || !medium.viscosity_pa_s.is_finite()
            || medium.viscosity_pa_s <= 0.
            || !medium.velocity_m_s.iter().all(|v| v.is_finite())
        {
            return Err("invalid finite suspension carrier");
        }
        Ok(Self {
            medium,
            volume_m3,
            mass_kg: mass,
            deposited_heat_j: 0.,
            numerical_loss_j: 0.,
        })
    }
    #[must_use]
    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }
    #[must_use]
    pub fn velocity_m_s(&self) -> [f64; 3] {
        self.medium.velocity_m_s
    }
    #[must_use]
    pub fn numerical_loss_j(&self) -> f64 {
        self.numerical_loss_j
    }
    #[must_use]
    pub fn deposited_heat_j(&self) -> f64 {
        self.deposited_heat_j
    }
}
impl Particle {
    /// Exact local two-way drag exchange without external forces. Both masses
    /// retain total momentum; dissipated relative kinetic energy heats carrier.
    /// One homogeneous volume; spatial advection/deposition and gravity are
    /// separate operations. No added mass or hydrodynamic history force.
    /// # Errors
    /// Invalid time, Re>0.1, particle volume fraction>0.01 or failed balances.
    /// Particle and carrier commit together; neither mutates on failure.
    pub fn exchange_drag(
        &mut self,
        dt_s: f64,
        carrier: &mut FiniteCarrier,
    ) -> Result<DragExchange, &'static str> {
        if !dt_s.is_finite() || dt_s <= 0. {
            return Err("invalid drag exchange time");
        }
        let volume = self.mass_kg / self.density_kg_m3;
        if volume / carrier.volume_m3 > 0.01 {
            return Err("suspension carrier is not dilute");
        }
        let relative: [f64; 3] =
            std::array::from_fn(|i| self.velocity_m_s[i] - carrier.medium.velocity_m_s[i]);
        let speed = relative[0].hypot(relative[1]).hypot(relative[2]);
        let re = 2. * self.radius_m * carrier.medium.density_kg_m3 * speed
            / carrier.medium.viscosity_pa_s;
        if !re.is_finite() || re > 0.1 {
            return Err("particle exceeds Stokes flow regime");
        }
        let total = self.mass_kg + carrier.mass_kg;
        let p_fraction = self.mass_kg / total;
        let c_fraction = carrier.mass_kg / total;
        let reduced = self.mass_kg * c_fraction;
        let rate =
            6. * std::f64::consts::PI * carrier.medium.viscosity_pa_s * self.radius_m / reduced;
        if !total.is_finite() || reduced <= 0. || !rate.is_finite() || rate <= 0. {
            return Err("unrepresentable coupled drag rate");
        }
        let x = rate * dt_s;
        let response = -(-x).exp_m1();
        let lost_integral = if x < 1e-3 {
            dt_s * x * (0.5 - x / 6. + x * x / 24. - x * x * x / 120.)
        } else {
            dt_s - response / rate
        };
        let dv: [f64; 3] = relative.map(|w| -c_fraction * response * w);
        let du: [f64; 3] = relative.map(|w| p_fraction * response * w);
        let v = std::array::from_fn(|i| self.velocity_m_s[i] + dv[i]);
        let u = std::array::from_fn(|i| carrier.medium.velocity_m_s[i] + du[i]);
        let position = std::array::from_fn(|i| {
            self.position_m[i] + self.velocity_m_s[i] * dt_s
                - c_fraction * relative[i] * lost_integral
        });
        let heat = 0.5 * reduced * speed * speed * (-(-2. * x).exp_m1());
        let new_heat = carrier.deposited_heat_j + heat;
        let actual_dv: [f64; 3] = std::array::from_fn(|i| v[i] - self.velocity_m_s[i]);
        let actual_du: [f64; 3] = std::array::from_fn(|i| u[i] - carrier.medium.velocity_m_s[i]);
        for i in 0..3 {
            let p = self.mass_kg * actual_dv[i];
            let c = carrier.mass_kg * actual_du[i];
            if (p + c).abs() > 1e-10 * (p.abs() + c.abs()).max(1e-300)
                || (dv[i] != 0. && actual_dv[i] == 0.)
                || (du[i] != 0. && actual_du[i] == 0.)
            {
                return Err("unrepresentable coupled drag momentum exchange");
            }
        }
        let kinetic_change = (0..3)
            .map(|i| {
                self.mass_kg * actual_dv[i] * (self.velocity_m_s[i] + 0.5 * actual_dv[i])
                    + carrier.mass_kg
                        * actual_du[i]
                        * (carrier.medium.velocity_m_s[i] + 0.5 * actual_du[i])
            })
            .sum::<f64>();
        let defect = kinetic_change + heat;
        if !position
            .iter()
            .chain(v.iter())
            .chain(u.iter())
            .all(|v| v.is_finite())
            || !heat.is_finite()
            || !new_heat.is_finite()
            || (heat > 0. && new_heat <= carrier.deposited_heat_j)
            || !defect.is_finite()
            || defect.abs() > 1e-10 * (heat.abs() + kinetic_change.abs()).max(1e-300)
        {
            return Err("coupled drag balance failure");
        }
        let impulse = actual_dv.map(|d| self.mass_kg * d);
        self.position_m = position;
        self.velocity_m_s = v;
        carrier.medium.velocity_m_s = u;
        carrier.deposited_heat_j = new_heat;
        Ok(DragExchange {
            particle_impulse_n_s: impulse,
            deposited_heat_j: heat,
            energy_defect_j: defect,
        })
    }
}

mod cloud;
pub use cloud::{
    CloudExchange, exchange_drag_cloud, exchange_forced_cloud, exchange_gravity_cloud,
};
