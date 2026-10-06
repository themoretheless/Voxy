//! Weakly compressible SPH liquids, independent of voxels and rendering.
//! Poly6 density and spiky pressure follow Müller, Charypar & Gross (SCA 2003).
//! Internal viscosity uses a central projection of the viscosity kernel with
//! a three-dimensional shear normalization. Pair forces are equal and opposite.
use std::collections::BTreeMap;
use std::f64::consts::PI;

#[path = "liquid/viscoelastic.rs"]
mod viscoelastic;
pub use viscoelastic::{Conformation, MaxwellFluid};
#[path = "liquid/water_saturation.rs"]
mod water_saturation;
pub use water_saturation::{WaterSaturation, water_saturation, water_saturation_temperature};
#[path = "liquid/water_helmholtz.rs"]
mod water_helmholtz;
pub use water_helmholtz::{
    WaterCoexistence, WaterHomogeneousResponse, WaterHomogeneousState, water_coexistence,
    water_homogeneous_from_energy, water_homogeneous_response, water_homogeneous_state,
};
#[path = "liquid/water_equilibrium.rs"]
mod water_equilibrium;
pub use water_equilibrium::{
    WaterEquilibriumResponse, WaterEquilibriumState, WaterHeatContact,
    change_water_volume_adiabatically, exchange_water_contact_heat, transfer_water_heat,
    water_equilibrium_at_temperature, water_equilibrium_from_energy,
    water_equilibrium_from_entropy, water_equilibrium_response,
};

/// Material coefficients in a consistent unit system (normally SI).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub rest_density: f64,
    /// Speed of sound controls compressibility, and the stable time step.
    pub sound_speed: f64,
    /// Dynamic viscosity, mass / (length * time).
    pub viscosity: f64,
}
impl Material {
    /// Illustrative density/reference viscosity; requires `ShearThinning::CONDENSED_MILK_DEMO`.
    /// Not a measured brand calibration; sound speed is a numerical WCSPH parameter.
    pub const CONDENSED_MILK_DEMO: Self = Self {
        rest_density: 1300.0,
        sound_speed: 20.0,
        viscosity: 10.0,
    };

    pub const WATER: Self = Self {
        rest_density: 1000.0,
        sound_speed: 20.0,
        viscosity: 0.001,
    };
    pub const OIL: Self = Self {
        rest_density: 800.0,
        sound_speed: 20.0,
        viscosity: 0.1,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub mass: f64,
    pub material: usize,
}

/// Static containing box. Bounds describe wall surfaces, not particle centers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Container {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub restitution: f64,
    /// Fraction of tangential speed removed on wall impact.
    pub friction: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub smoothing_radius: f64,
    pub particle_radius: f64,
    pub gravity: [f64; 3],
    pub max_particles: usize,
    pub max_pairs: usize,
    pub max_neighbor_checks: usize,
    pub max_substeps: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            smoothing_radius: 0.2,
            particle_radius: 0.025,
            gravity: [0.0, -9.81, 0.0],
            max_particles: 16_384,
            max_pairs: 1_000_000,
            max_neighbor_checks: 4_000_000,
            max_substeps: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Liquid {
    particles: Vec<Particle>,
    maxwell_fluids: Vec<Option<MaxwellFluid>>,
    conformation: Vec<Conformation>,
    polymer_heat: f64,
    formulation: Formulation,
    materials: Vec<Material>,
    config: Config,
    surface_strengths: Vec<f64>,
    interface_penalties: BTreeMap<(usize, usize), f64>,
    wall_adhesion: Vec<Option<WallAdhesion>>,
    viscous_heating: bool,
    viscous_integrator: ViscousIntegrator,
    pressure_work: bool,
    boundary_coupling: Option<boundary_body::Coupling>,
    boundaries: Vec<BoundarySample>,
    boundary_grid: BTreeMap<[i64; 3], Vec<usize>>,
    boundary_velocities: Vec<[f64; 3]>,
    static_pressure_extrapolation: bool,
    reflecting_box: Option<ReflectingBox>,
    reflecting_no_slip: reflecting_box::WallViscosity,
    transport: Option<transport::Transport>,
    suspension_heat_buffer: Vec<f64>,
    suspension_heat_correction: Vec<f64>,
    property_responses: Vec<Option<PropertyResponse>>,
    shear_thinning: Vec<Option<ShearThinning>>,
    yield_stresses: Vec<f64>,
    thixotropy: Vec<Option<Thixotropy>>,
    structure: Vec<f64>,
    droplet_population: Option<Vec<bool>>,
    carrier_droplet_stage: bool,
    arrhenius_viscosity: Vec<Option<ArrheniusViscosity>>,
    gas_equations: Vec<Option<IdealGas>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepStats {
    pub substeps: usize,
    pub neighbor_pairs: usize,
    pub max_density_ratio: f64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidConfig,
    InvalidSurface,
    SurfaceBudget,
    InvalidTransport,
    InvalidPhaseChange,
    InvalidPropertyResponse,
    InvalidBuoyancy,
    BuoyancyBudget,
    MissingFluidCarrier,
    InvalidMaterial,
    InvalidSurfaceStrength,
    InvalidWallAdhesion,
    InvalidBoundary,
    BoundaryBudget,
    InvalidParticle,
    InvalidContainer,
    InvalidTimeStep,
    ParticleBudget,
    PairBudget,
    NeighborBudget,
    WorkBudgetExceeded,
    SubstepBudget,
    NumericalFailure,
    CollisionBackend,
    InvalidCollision,
    InitialOverlap,
    CollisionBudget,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "liquid simulation error: {self:?}")
    }
}
impl std::error::Error for Error {}

type ForceState = (Vec<f64>, Vec<[f64; 3]>, f64);

impl Liquid {
    /// Creates an owned particle system, validating all coefficients and particle state.
    /// # Errors
    /// Rejects nonfinite/invalid configuration, coefficients, masses, material indices or budgets.
    pub fn new(
        particles: Vec<Particle>,
        materials: Vec<Material>,
        config: Config,
    ) -> Result<Self, Error> {
        if !positive(config.smoothing_radius)
            || !positive(config.particle_radius)
            || config.particle_radius >= config.smoothing_radius / 2.0
            || !finite(config.gravity)
            || config.max_particles == 0
            || config.max_pairs == 0
            || config.max_neighbor_checks == 0
            || config.max_substeps == 0
        {
            return Err(Error::InvalidConfig);
        }
        if materials.is_empty()
            || materials.iter().any(|m| {
                !positive(m.rest_density)
                    || !positive(m.sound_speed)
                    || !m.viscosity.is_finite()
                    || m.viscosity < 0.0
            })
        {
            return Err(Error::InvalidMaterial);
        }
        if particles.len() > config.max_particles {
            return Err(Error::ParticleBudget);
        }
        if particles.iter().any(|p| {
            !finite(p.position)
                || !finite(p.velocity)
                || !positive(p.mass)
                || p.material >= materials.len()
        }) {
            return Err(Error::InvalidParticle);
        }
        Ok(Self {
            maxwell_fluids: vec![None; materials.len()],
            conformation: vec![viscoelastic::IDENTITY; particles.len()],
            polymer_heat: 0.0,
            suspension_heat_buffer: vec![0.; particles.len()],
            suspension_heat_correction: vec![0.; particles.len()],
            structure: vec![1.0; particles.len()],
            droplet_population: None,
            carrier_droplet_stage: false,
            thixotropy: vec![None; materials.len()],
            particles,
            formulation: Formulation::MassDensity,
            property_responses: vec![None; materials.len()],
            shear_thinning: vec![None; materials.len()],
            yield_stresses: vec![0.0; materials.len()],
            arrhenius_viscosity: vec![None; materials.len()],
            gas_equations: vec![None; materials.len()],
            transport: None,
            viscous_heating: false,
            viscous_integrator: ViscousIntegrator::Sequential,
            pressure_work: false,
            boundary_coupling: None,
            boundaries: Vec::new(),
            boundary_grid: BTreeMap::new(),
            boundary_velocities: Vec::new(),
            static_pressure_extrapolation: false,
            reflecting_box: None,
            reflecting_no_slip: reflecting_box::WallViscosity::FreeSlip,
            surface_strengths: vec![0.0; materials.len()],
            interface_penalties: BTreeMap::new(),
            wall_adhesion: vec![None; materials.len()],
            materials,
            config,
        })
    }
    /// Sets the numerical cohesion/curvature strength of one material (zero disables it).
    /// This discretization coefficient is not calibrated SI surface tension.
    /// # Errors
    /// Rejects an unknown material, negative strength, NaN or infinity without changing state.
    pub fn set_surface_strength(&mut self, material: usize, strength: f64) -> Result<(), Error> {
        if material >= self.materials.len() || !strength.is_finite() || strength < 0.0 {
            return Err(Error::InvalidSurfaceStrength);
        }
        self.surface_strengths[material] = strength;
        Ok(())
    }

    #[must_use]
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
    #[must_use]
    pub fn mass(&self) -> f64 {
        self.particles.iter().map(|p| p.mass).sum()
    }

    /// Advances against arbitrary static geometry using the existing collision-world contract.
    /// Particles use an axis-aligned box of `particle_radius` on each side.
    /// The geometry backend remains read-only; every failure leaves liquid state unchanged.
    /// # Errors
    /// Also rejects invalid contact settings, backend errors, initial overlaps,
    /// malformed backend hits, and exhausted contact budgets.
    pub fn step_with_world(
        &mut self,
        dt: f64,
        container: Option<Container>,
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
    ) -> Result<StepStats, Error> {
        contact.validate()?;
        let radius = self.config.particle_radius;
        self.advance(dt, container, |particles, time| {
            for particle in particles {
                move_with_world(particle, time, radius, world, contact)?;
            }
            Ok(())
        })
    }

    /// Advances pressure, viscosity, gravity and container collisions using adaptive substeps.
    /// Errors leave the entire system unchanged. Pair and substep budgets bound work.
    /// # Errors
    /// Rejects invalid time/container, exhausted work budgets, or nonfinite intermediate state.
    pub fn step(&mut self, dt: f64, container: Option<Container>) -> Result<StepStats, Error> {
        self.advance(dt, container, |particles, time| {
            for particle in particles {
                for axis in 0..3 {
                    particle.position[axis] += particle.velocity[axis] * time;
                }
            }
            Ok(())
        })
    }

    fn advance(
        &mut self,
        dt: f64,
        container: Option<Container>,
        mut move_particles: impl FnMut(&mut [Particle], f64) -> Result<(), Error>,
    ) -> Result<StepStats, Error> {
        self.advance_with_mover(dt, container, |state, particles, time| {
            move_particles(particles, time)?;
            if state.boundary_coupling.is_some() {
                state.drift_boundary_body(time)?;
            }
            Ok(())
        })
    }

    fn advance_with_mover(
        &mut self,
        dt: f64,
        container: Option<Container>,
        mut move_particles: impl FnMut(&mut Self, &mut [Particle], f64) -> Result<(), Error>,
    ) -> Result<StepStats, Error> {
        if self.thixotropy.iter().all(Option::is_none)
            && self.maxwell_fluids.iter().all(Option::is_none)
            && self.transport.as_ref().is_none_or(|t| t.species.is_none())
        {
            return self.advance_with_mover_inner(dt, container, move_particles);
        }
        let mut candidate = self.clone();
        let stats = candidate.advance_with_mover_inner(dt, container, &mut move_particles)?;
        *self = candidate;
        Ok(stats)
    }
    fn advance_with_mover_inner(
        &mut self,
        dt: f64,
        container: Option<Container>,
        mut move_particles: impl FnMut(&mut Self, &mut [Particle], f64) -> Result<(), Error>,
    ) -> Result<StepStats, Error> {
        if !positive(dt) || dt > 0.1 {
            return Err(Error::InvalidTimeStep);
        }
        if let Some(c) = container {
            validate_container(c, self.config.particle_radius)?;
        }
        let mut next = self.particles.clone();
        let mut transport = self.transport.clone();
        let mut remaining = dt;
        let mut stats = StepStats {
            substeps: 0,
            neighbor_pairs: 0,
            max_density_ratio: 0.0,
        };
        while remaining > 0.0 && (!next.is_empty() || self.boundary_coupling.is_some()) {
            if stats.substeps >= self.config.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let mut pairs = pairs(
                &next,
                self.config.smoothing_radius,
                self.config.max_pairs,
                self.config.max_neighbor_checks,
            )?;
            self.retain_carrier_pairs(&mut pairs);
            let properties = self.evaluate_materials(&next, transport.as_ref())?;
            let (density, mut acceleration, viscous_rate) =
                self.forces(&next, &pairs, &properties)?;
            if let Some(walls) = container {
                self.apply_wall_adhesion(&next, walls, &mut acceleration)?;
            }

            let moving_boundary = if self.boundary_coupling.is_some() {
                Some(self.boundary_body_forces(&next, &properties, &density, pairs.len())?)
            } else {
                None
            };
            let boundary_speed = if self.viscous_heating {
                self.boundary_surface_speed(&next, &properties, pairs.len())?
            } else {
                0.0
            };
            let mut step = remaining;
            if viscous_rate > 0.0 && !self.exact_stationary_viscosity() {
                step = step.min(0.25 / viscous_rate);
            }
            for (i, p) in next.iter().enumerate() {
                step = step.min(self.particle_time_limit(
                    p,
                    properties[i],
                    acceleration[i],
                    container,
                    boundary_speed,
                ));
                stats.max_density_ratio = stats
                    .max_density_ratio
                    .max(density[i] / properties[i].rest_density);
            }
            if let Some(boundary) = &moving_boundary {
                step = step.min(self.boundary_body_time_limit(boundary, &next)?);
            }
            if !positive(step) {
                return Err(Error::NumericalFailure);
            }
            // Structure uses the same start-of-substep state as the explicit
            // mechanical coefficients. Update on every accepted adaptive substep.
            let updated_structure = if self.thixotropy.iter().any(Option::is_some) {
                Some(self.advanced_structure(&next, transport.as_ref(), step)?)
            } else {
                None
            };
            if let Some(fields) = &mut transport
                && let Some(pressure_acceleration) =
                    self.advance_thermal(&mut next, &pairs, &properties, &density, fields, step)?
            {
                for (total, pressure) in acceleration.iter_mut().zip(pressure_acceleration) {
                    for axis in 0..3 {
                        total[axis] -= pressure[axis];
                    }
                }
            }
            if self.carrier_droplet_stage {
                for (i, a) in acceleration.iter_mut().enumerate() {
                    if self
                        .droplet_population
                        .as_ref()
                        .is_some_and(|flags| flags[i])
                    {
                        *a = self.config.gravity;
                    }
                }
            }
            for (p, a) in next.iter_mut().zip(acceleration) {
                for (axis, component) in a.into_iter().enumerate() {
                    p.velocity[axis] += component * step;
                }
            }
            self.advance_polymer_memory(&next, &pairs, &properties, transport.as_mut(), step)?;
            if let Some(boundary) = moving_boundary {
                self.advance_boundary_body(&mut next, transport.as_mut(), &boundary, step)?;
                self.evaluate_materials(&next, transport.as_ref())?;
            }
            move_particles(self, &mut next, step)?;
            validate_moved_particles(&mut next, container, self.config.particle_radius)?;
            self.validate_reflecting_positions(&next)?;
            if let Some(structure) = updated_structure {
                self.structure = structure;
                self.evaluate_materials(&next, transport.as_ref())?;
            }
            remaining = (remaining - step).max(0.0);
            stats.substeps += 1;
            stats.neighbor_pairs = stats.neighbor_pairs.max(pairs.len());
        }
        self.particles = next;
        self.transport = transport;
        Ok(stats)
    }

    fn particle_time_limit(
        &self,
        p: &Particle,
        m: Material,
        acceleration: [f64; 3],
        container: Option<Container>,
        boundary_speed: f64,
    ) -> f64 {
        let h = self.config.smoothing_radius;
        let mut step = f64::INFINITY;
        if let Some(law) = self.maxwell_fluids[p.material] {
            step = step.min(0.1 * law.relaxation_time);
            step = step.min(0.1 * h / (law.modulus / m.rest_density).sqrt());
        }
        if container.is_some()
            && let Some(adhesion) = self.wall_adhesion[p.material]
            && adhesion.acceleration > 0.0
        {
            step = step.min(0.1 * (adhesion.range / adhesion.acceleration).sqrt());
        }
        let surface = self.surface_strengths[p.material];
        if surface > 0.0 {
            step = step.min(0.1 * (h / (surface * m.rest_density)).sqrt());
        }
        step = step.min(0.25 * h / (m.sound_speed + norm(p.velocity).max(boundary_speed)));
        let a = norm(acceleration);
        if a > 0.0 {
            step = step.min(0.25 * (h / a).sqrt());
        }
        if m.viscosity > 0.0 && !self.exact_stationary_viscosity() {
            step = step.min(0.125 * h * h * m.rest_density / m.viscosity);
        }
        step
    }

    // The heated, fixed-boundary-free stage exponentiates each viscous pair.
    // Explicit diffusion stability bounds do not apply to that contraction.
    // Moving/finite-body and sampled-boundary paths retain conservative limits.
    fn exact_stationary_viscosity(&self) -> bool {
        self.viscous_heating && self.boundaries.is_empty() && self.boundary_coupling.is_none()
    }

    fn particle_densities(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
    ) -> Vec<f64> {
        let h = self.config.smoothing_radius;
        let self_kernel = density_kernel(self.formulation, h, 0.0);
        let mut density: Vec<_> = particles.iter().map(|p| p.mass * self_kernel).collect();
        for &(i, j) in pairs {
            let r = norm(sub(particles[i].position, particles[j].position));
            let w = density_kernel(self.formulation, h, r);
            let ratio = match self.formulation {
                Formulation::MassDensity => 1.0,
                Formulation::RestVolume | Formulation::RestVolumeWendland => {
                    properties[i].rest_density / properties[j].rest_density
                }
            };
            density[i] += particles[j].mass * w * ratio;
            density[j] += particles[i].mass * w / ratio;
        }
        if self.carrier_droplet_stage {
            for (i, rho) in density.iter_mut().enumerate() {
                if self
                    .droplet_population
                    .as_ref()
                    .is_some_and(|flags| flags[i])
                {
                    *rho = properties[i].rest_density;
                }
            }
        }
        density
    }

    fn forces(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
    ) -> Result<ForceState, Error> {
        self.validate_gas_geometry()?;
        let h = self.config.smoothing_radius;
        let mut density = self.particle_densities(particles, pairs, properties);
        let boundary_pairs = self.boundary_pairs(particles, pairs.len())?;
        self.add_boundary_density(properties, &boundary_pairs, &mut density);
        let images = self.image_pairs(particles, pairs.len())?;
        self.image_density(particles, properties, &images, &mut density);
        let pressure: Vec<_> = particles
            .iter()
            .zip(properties)
            .zip(&density)
            .map(|((particle, material), rho)| {
                self.material_pressure(particle.material, *material, *rho)
            })
            .collect::<Result<_, _>>()?;
        let normals = color_normals(particles, pairs, &density, h);
        let mut acceleration = vec![self.config.gravity; particles.len()];
        let mut viscous_rates = vec![0.0_f64; particles.len()];
        for &(i, j) in pairs {
            let a = particles[i];
            let b = particles[j];
            let delta = sub(a.position, b.position);
            let r = norm(delta);
            let direction = if r > 1e-12 {
                delta.map(|x| x / r)
            } else {
                [1.0, 0.0, 0.0]
            };
            let pressure_force = self.thermodynamic_pressure_magnitude(
                [a.mass, b.mass],
                [density[i], density[j]],
                [pressure[i], pressure[j]],
                [properties[i].rest_density, properties[j].rest_density],
                [h, r],
                [a.material, b.material],
            );
            let mu = 0.5 * (properties[i].viscosity + properties[j].viscosity);
            let viscosity =
                viscous_conductance([a.mass, b.mass], mu, [density[i], density[j]], r, h);
            viscous_rates[i] += viscosity / a.mass;
            viscous_rates[j] += viscosity / b.mass;
            let surface = if a.material == b.material {
                self.surface_strengths[a.material]
            } else {
                0.0
            };
            let correction = (properties[i].rest_density + properties[j].rest_density)
                / (density[i] + density[j]);
            let cohesion = if r > 1e-12 {
                surface * a.mass * b.mass * correction * cohesion_kernel(r, h)
            } else {
                0.0
            };
            // Harmonic pair mass makes curvature exchange symmetric for unequal masses.
            let curvature = surface * correction * (2.0 * a.mass * b.mass / (a.mass + b.mass));
            let curvature_normal: f64 = sub(normals[i], normals[j])
                .iter()
                .zip(direction)
                .map(|(v, n)| v * n)
                .sum();
            let relative_normal: f64 = sub(b.velocity, a.velocity)
                .iter()
                .zip(direction)
                .map(|(v, n)| v * n)
                .sum();
            let interface_force = self.interface_force(a, b, r);
            for axis in 0..3 {
                let force = (pressure_force - cohesion + interface_force) * direction[axis]
                    - curvature * curvature_normal * direction[axis]
                    + if self.viscous_heating {
                        0.0
                    } else {
                        viscosity * relative_normal * direction[axis]
                    };
                acceleration[i][axis] += force / a.mass;
                acceleration[j][axis] -= force / b.mass;
            }
        }
        self.add_image_forces(
            particles,
            properties,
            &density,
            &images,
            &mut acceleration,
            &mut viscous_rates,
        )?;
        if self.boundary_coupling.is_none() {
            self.add_boundary_forces(
                particles,
                properties,
                &density,
                &boundary_pairs,
                &mut acceleration,
                &mut viscous_rates,
            )?;
        }
        self.add_polymer_forces(particles, pairs, properties, &mut acceleration)?;
        if density.iter().any(|&rho| !positive(rho)) || acceleration.iter().any(|&a| !finite(a)) {
            return Err(Error::NumericalFailure);
        }
        Ok((
            density,
            acceleration,
            viscous_rates.into_iter().fold(0.0, f64::max),
        ))
    }
}

#[allow(clippy::cast_possible_truncation)]
fn cell(position: [f64; 3], h: f64) -> Result<[i64; 3], Error> {
    let mut result = [0; 3];
    for axis in 0..3 {
        let x = (position[axis] / h).floor();
        // Exact integer range in f64, leaving ample room for neighbor offsets.
        if !x.is_finite() || x.abs() > 4_503_599_627_370_496.0 {
            return Err(Error::NumericalFailure);
        }
        result[axis] = x as i64;
    }
    Ok(result)
}
fn pairs(
    particles: &[Particle],
    h: f64,
    budget: usize,
    check_budget: usize,
) -> Result<Vec<(usize, usize)>, Error> {
    let mut grid: BTreeMap<[i64; 3], Vec<usize>> = BTreeMap::new();
    for (i, p) in particles.iter().enumerate() {
        grid.entry(cell(p.position, h)?).or_default().push(i);
    }
    let mut result = Vec::new();
    let mut checks = 0;
    for (i, p) in particles.iter().enumerate() {
        let c = cell(p.position, h)?;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(neighbors) = grid.get(&[c[0] + x, c[1] + y, c[2] + z]) {
                        for &j in neighbors {
                            if j <= i {
                                continue;
                            }
                            if checks >= check_budget {
                                return Err(Error::NeighborBudget);
                            }
                            checks += 1;
                            if norm(sub(p.position, particles[j].position)) < h {
                                if result.len() >= budget {
                                    return Err(Error::PairBudget);
                                }
                                result.push((i, j));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}
// Akinci 2013 cohesion shape: short-range repulsion and longer-range attraction.
fn cohesion_kernel(radius: f64, support: f64) -> f64 {
    let q = radius / support;
    if q >= 1.0 {
        return 0.0;
    }
    let shape = (1.0 - q).powi(3) * q.powi(3);
    let value = if q > 0.5 {
        shape
    } else {
        2.0 * shape - 1.0 / 64.0
    };
    32.0 / (PI * support.powi(3)) * value
}

fn collide(p: &mut Particle, c: Container, radius: f64) {
    for axis in 0..3 {
        let low = c.min[axis] + radius;
        let high = c.max[axis] - radius;
        let impact = (p.position[axis] < low && p.velocity[axis] < 0.0)
            || (p.position[axis] > high && p.velocity[axis] > 0.0);
        p.position[axis] = p.position[axis].clamp(low, high);
        if impact {
            p.velocity[axis] *= -c.restitution;
            for tangent in 0..3 {
                if tangent != axis {
                    p.velocity[tangent] *= 1.0 - c.friction;
                }
            }
        }
    }
}
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}
fn finite(v: [f64; 3]) -> bool {
    v.into_iter().all(f64::is_finite)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Static collision response and limits, applied per particle per fluid substep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactConfig {
    pub restitution: f64,
    pub friction: f64,
    pub max_contacts: usize,
    pub max_candidates: usize,
}
impl Default for ContactConfig {
    fn default() -> Self {
        Self {
            restitution: 0.0,
            friction: 0.0,
            max_contacts: 8,
            max_candidates: 4096,
        }
    }
}
impl ContactConfig {
    fn validate(self) -> Result<(), Error> {
        if !(0.0..=1.0).contains(&self.restitution)
            || !(0.0..=1.0).contains(&self.friction)
            || self.max_contacts == 0
            || self.max_candidates == 0
        {
            return Err(Error::InvalidCollision);
        }
        Ok(())
    }
}
fn move_with_world(
    particle: &mut Particle,
    dt: f64,
    radius: f64,
    world: &impl crate::CollisionWorld,
    contact: ContactConfig,
) -> Result<(), Error> {
    let mut remaining = dt;
    for collision in 0..=contact.max_contacts {
        let displacement = particle.velocity.map(|v| v * remaining);
        let hit = world
            .sweep_aabb(
                crate::AnchoredAabb {
                    anchor: crate::Origin::default(),
                    min: particle.position.map(|x| x - radius),
                    max: particle.position.map(|x| x + radius),
                },
                displacement,
                contact.max_candidates,
            )
            .map_err(|_| Error::CollisionBackend)?;
        if !hit.fraction.is_finite() || !(0.0..=1.0).contains(&hit.fraction) {
            return Err(Error::InvalidCollision);
        }
        let normal_count = hit.normal.iter().filter(|&&n| n != 0).count();
        if normal_count == 0 {
            if hit.obstacle.is_some() {
                return Err(Error::InitialOverlap);
            }
            if hit.fraction < 1.0 {
                return Err(Error::InvalidCollision);
            }
            for (axis, delta) in displacement.into_iter().enumerate() {
                particle.position[axis] += delta;
            }
            return Ok(());
        }
        if normal_count != 1
            || hit.normal.iter().any(|&n| !(-1..=1).contains(&n))
            || hit.obstacle.is_none()
        {
            return Err(Error::InvalidCollision);
        }
        if collision == contact.max_contacts {
            return Err(Error::CollisionBudget);
        }
        let axis = hit
            .normal
            .iter()
            .position(|&n| n != 0)
            .ok_or(Error::InvalidCollision)?;
        let normal = f64::from(hit.normal[axis]);
        if particle.velocity[axis] * normal >= 0.0 {
            return Err(Error::InvalidCollision);
        }
        for (axis, delta) in displacement.into_iter().enumerate() {
            particle.position[axis] += delta * hit.fraction;
        }
        particle.velocity[axis] *= -contact.restitution;
        for tangent in 0..3 {
            if tangent != axis {
                particle.velocity[tangent] *= 1.0 - contact.friction;
            }
        }
        // Move to the exterior side by a few ulps to prevent repeated zero-time hits.
        particle.position[axis] +=
            normal * 64.0 * f64::EPSILON * particle.position[axis].abs().max(radius).max(1.0);
        remaining *= 1.0 - hit.fraction;
        if remaining <= 0.0 {
            return Ok(());
        }
    }
    Err(Error::CollisionBudget)
}

#[path = "liquid/buoyancy.rs"]
mod buoyancy;
pub use buoyancy::{BuoyancyConfig, BuoyancyReport, FloatingBody, FluidLayer};

#[path = "liquid/suspension.rs"]
mod suspension;
#[path = "liquid/transport.rs"]
mod transport;
pub use suspension::SuspensionCellInventory;
pub use transport::{LiquidField, TransportMaterial};

#[path = "liquid/properties.rs"]
mod properties;
pub use properties::PropertyResponse;

#[path = "liquid/surface.rs"]
mod surface;
pub use surface::{LiquidSurface, SurfaceConfig};
#[path = "liquid/surface_volume.rs"]
mod surface_volume;
pub use surface_volume::{SurfaceVolumeControl, VolumeMatchedSurface};

#[path = "liquid/adhesion.rs"]
mod adhesion;
pub use adhesion::WallAdhesion;

#[path = "liquid/phase.rs"]
mod phase;
pub use phase::PhaseChange;

#[path = "liquid/dissipation.rs"]
mod dissipation;
pub use dissipation::{ViscousIntegrator, ViscousRelaxationStats};

#[path = "liquid/exchange.rs"]
mod exchange;
pub use exchange::{ExchangeTotals, ParticleExchange, ParticleInput};

#[path = "liquid/forcing.rs"]
mod forcing;
pub use forcing::ForcingReport;

#[path = "liquid/body_contacts.rs"]
mod body_contacts;
pub use body_contacts::{BodyContactConfig, BodyContactReport};

fn validate_container(c: Container, radius: f64) -> Result<(), Error> {
    if !finite(c.min)
        || !finite(c.max)
        || !(0.0..=1.0).contains(&c.restitution)
        || !(0.0..=1.0).contains(&c.friction)
        || (0..3).any(|axis| c.max[axis] - c.min[axis] <= 2.0 * radius)
    {
        return Err(Error::InvalidContainer);
    }
    Ok(())
}

#[path = "liquid/pressure_work.rs"]
mod pressure_work;

fn pressure_magnitude(
    mass: [f64; 2],
    density: [f64; 2],
    pressure: [f64; 2],
    reference: [f64; 2],
    support: f64,
    radius: f64,
    formulation: Formulation,
) -> f64 {
    let (ratio, gradient) = match formulation {
        Formulation::MassDensity => (
            1.0,
            45.0 / (PI * support.powi(6)) * (support - radius).max(0.0).powi(2),
        ),
        // Differentiate the same kernel used by the volume density sum.
        Formulation::RestVolume | Formulation::RestVolumeWendland => (
            reference[0] / reference[1],
            density_kernel_gradient(formulation, support, radius),
        ),
    };
    mass[0]
        * mass[1]
        * (pressure[0] / density[0].powi(2) * ratio + pressure[1] / density[1].powi(2) / ratio)
        * gradient
}

fn density_kernel(formulation: Formulation, support: f64, radius: f64) -> f64 {
    if formulation == Formulation::RestVolumeWendland {
        let q = radius / support;
        21.0 / (2.0 * PI * support.powi(3)) * (1.0 - q).max(0.0).powi(4) * (1.0 + 4.0 * q)
    } else {
        315.0 / (64.0 * PI * support.powi(9))
            * (support * support - radius * radius).max(0.0).powi(3)
    }
}

fn density_kernel_gradient(formulation: Formulation, support: f64, radius: f64) -> f64 {
    if formulation == Formulation::RestVolumeWendland {
        let q = radius / support;
        210.0 / (PI * support.powi(4)) * q * (1.0 - q).max(0.0).powi(3)
    } else {
        6.0 * 315.0 / (64.0 * PI * support.powi(9))
            * radius
            * (support * support - radius * radius).max(0.0).powi(2)
    }
}

#[path = "liquid/boundary.rs"]
mod boundary;
pub use boundary::{BoundaryDiagnostics, BoundarySample};

#[path = "liquid/translating_world.rs"]
mod translating_world;

pub use translating_world::TranslatingWorldReport;

#[path = "liquid/dynamic_world.rs"]
mod dynamic_world;
pub use dynamic_world::{
    DynamicImpactReport, DynamicWorldConfig, DynamicWorldReport, ThermalTranslatingBody,
    TranslatingBody,
};

#[path = "liquid/boundary_body.rs"]
mod boundary_body;
pub use boundary_body::{BoundaryBodyReport, BoundaryWorldReport};

fn validate_moved_particles(
    particles: &mut [Particle],
    container: Option<Container>,
    radius: f64,
) -> Result<(), Error> {
    for p in particles {
        if let Some(c) = container {
            collide(p, c, radius);
        }
        if !finite(p.position) || !finite(p.velocity) {
            return Err(Error::NumericalFailure);
        }
    }
    Ok(())
}

#[path = "liquid/rotation.rs"]
mod rotation;
pub use rotation::{RotatingBody, RotatingBoundaryReport, TensorBody};

/// Instantaneous lab-space state, before integration and collisions.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidDiagnostics {
    pub densities: Vec<f64>,
    /// Forces divided by particle mass. Exact thermal pair kicks are not included.
    pub accelerations: Vec<[f64; 3]>,
    pub viscous_rate: f64,
}
impl Liquid {
    /// # Errors
    /// Invalid effective properties, exhausted neighbor budgets or numerical failure.
    pub fn diagnostics(&self) -> Result<FluidDiagnostics, Error> {
        let neighbors = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let properties = self.effective_materials()?;
        let (densities, accelerations, viscous_rate) =
            self.forces(&self.particles, &neighbors, &properties)?;
        Ok(FluidDiagnostics {
            densities,
            accelerations,
            viscous_rate,
        })
    }
}

fn viscous_conductance(
    masses: [f64; 2],
    viscosity: f64,
    densities: [f64; 2],
    separation: f64,
    support: f64,
) -> f64 {
    if separation <= 1e-12 {
        return 0.0;
    }
    5.0 * masses[0] * masses[1] * viscosity * 45.0 / (PI * support.powi(6))
        * (support - separation).max(0.0)
        / (densities[0] * densities[1])
}

fn color_normals(
    particles: &[Particle],
    pairs: &[(usize, usize)],
    density: &[f64],
    h: f64,
) -> Vec<[f64; 3]> {
    let poly6 = 315.0 / (64.0 * PI * h.powi(9));
    // Color-field normals from same-phase neighbors. Poly6 is differentiable at zero.
    let mut normals = vec![[0.0; 3]; particles.len()];
    for &(i, j) in pairs {
        if particles[i].material != particles[j].material {
            continue;
        }
        let delta = sub(particles[i].position, particles[j].position);
        let radius_squared: f64 = delta.iter().map(|v| v * v).sum();
        let gradient_scale = -6.0 * poly6 * (h * h - radius_squared).max(0.0).powi(2) * h;
        for (axis, component) in delta.into_iter().enumerate() {
            normals[i][axis] += particles[j].mass / density[j] * gradient_scale * component;
            normals[j][axis] -= particles[i].mass / density[i] * gradient_scale * component;
        }
    }
    normals
}

#[path = "liquid/interface.rs"]
mod interface;

/// Coupled density and pressure discretization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formulation {
    /// Standard mass-weighted SPH, retained as the default.
    MassDensity,
    /// Neighbor reference volumes support the particle's own material density.
    /// Pressure differentiates the volume-density poly6 sum. Experimental multiphase mode.
    RestVolume,
    /// Reference-volume density with normalized 3D Wendland C2 kernel and its gradient.
    /// Experimental alternative intended to avoid kernel pairing; not a calibrated solver.
    RestVolumeWendland,
}
impl Liquid {
    /// Selects density/pressure operators without changing mass, velocities or fields.
    /// Switching in a running simulation can change pressure abruptly.
    /// Disable a reflecting box before selecting `MassDensity`; stepping otherwise fails.
    pub fn set_formulation(&mut self, formulation: Formulation) {
        self.formulation = formulation;
    }
}

#[cfg(test)]
mod density_kernel_tests {
    use super::{Formulation, PI, density_kernel};
    #[test]
    fn normalized_volume_kernels_and_wendland_nonnegative_spectral_samples() {
        for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
            for support in [0.17_f64, 1.0, 3.0] {
                let intervals = 4000;
                let integral: f64 = (0..intervals)
                    .map(|i| {
                        let radius = support * (f64::from(i) + 0.5) / f64::from(intervals);
                        4.0 * PI
                            * radius.powi(2)
                            * density_kernel(formulation, support, radius)
                            * support
                            / f64::from(intervals)
                    })
                    .sum();
                assert!((integral - 1.0).abs() < 1e-8);
            }
        }
        // Radial 3D Fourier integral samples; these check the implemented shape,
        // not a proof of spectral nonnegativity for all wavenumbers.
        for frequency in [1.0_f64, 5.0, 10.0, 20.0, 30.0, 50.0] {
            let intervals = 4000;
            let transform: f64 = (0..intervals)
                .map(|i| {
                    let radius = (f64::from(i) + 0.5) / f64::from(intervals);
                    let phase = frequency * radius;
                    4.0 * PI
                        * radius.powi(2)
                        * density_kernel(Formulation::RestVolumeWendland, 1.0, radius)
                        * phase.sin()
                        / phase
                        / f64::from(intervals)
                })
                .sum();
            assert!(transform > 0.0, "frequency {frequency}: {transform}");
        }
    }
}

#[path = "liquid/wall_pressure.rs"]
mod wall_pressure;
pub use wall_pressure::WallPressure;

#[path = "liquid/reflecting_box.rs"]
mod reflecting_box;
pub use reflecting_box::{ReflectingBox, ReflectingDiagnostics, ReflectingViscousDiagnostics};

#[path = "liquid/reservoir.rs"]
mod reservoir;

#[path = "liquid/rheology.rs"]
mod rheology;
pub use rheology::{HerschelBulkley, ShearThinning, WhippedCreamProfile};

#[path = "liquid/arrhenius.rs"]
mod arrhenius;
pub use arrhenius::ArrheniusViscosity;

#[path = "liquid/emission.rs"]
mod emission;
pub use emission::{
    EmissionPulse, EmissionReaction, PulsedEmitter, SourceAccuracy, ViscosityProfile,
};

#[path = "liquid/splitting.rs"]
mod splitting;
pub use splitting::ImpactExchange;

#[path = "liquid/body_heat.rs"]
mod body_heat;
pub use body_heat::ThermalBodyStep;

#[path = "liquid/thermal_interface.rs"]
mod thermal_interface;
pub use thermal_interface::ThermalPlanePatch;

#[path = "liquid/thixotropy.rs"]
mod thixotropy;
pub use thixotropy::Thixotropy;

#[path = "liquid/gas.rs"]
mod gas;
pub use gas::IdealGas;

#[path = "liquid/species.rs"]
mod species;

#[path = "liquid/reaction.rs"]
mod reaction;
pub use reaction::{Reaction, ReactionAccuracy, ReactionKinetics};

#[path = "liquid/saturation.rs"]
mod saturation;
pub use saturation::SaturationCurve;

#[path = "liquid/evaporation.rs"]
pub(crate) mod evaporation;
pub use evaporation::{SolutionVaporInterface, VaporCell, VaporExchangeAccuracy, VaporInterface};

#[path = "liquid/mixture_properties.rs"]
mod mixture_properties;
pub use mixture_properties::{SpeciesProperties, ViscosityBlend};

#[path = "liquid/mixture_profile.rs"]
mod mixture_profile;
pub use mixture_profile::FluidMixtureProfile;

#[path = "liquid/mixture_heat.rs"]
mod mixture_heat;

#[path = "liquid/film_capture.rs"]
mod film_capture;
pub use film_capture::FilmCapture;

#[path = "liquid/film_rebound.rs"]
mod film_rebound;
pub use film_rebound::{FilmRebound, FilmReboundReport};
#[path = "liquid/film_multi_rebound.rs"]
mod film_multi_rebound;
pub use film_multi_rebound::FilmMultiReboundReport;
#[path = "liquid/film_impact_events.rs"]
mod film_impact_events;
pub use film_impact_events::{
    DropletFlight, DropletFlightReport, DropletLifecycle, FilmImpactControl, FilmImpactEventReport,
};

#[path = "liquid/droplet_coalescence.rs"]
mod droplet_coalescence;
pub use droplet_coalescence::{DropletCoalescenceControl, DropletCoalescenceReport};
#[path = "liquid/droplet_merge.rs"]
mod droplet_merge;
pub use droplet_merge::DropletMergeReport;
#[path = "liquid/droplet_split.rs"]
mod droplet_split;
pub use droplet_split::{DropletSplit, DropletSplitReport};

#[path = "liquid/droplet_drag.rs"]
mod droplet_drag;
pub use droplet_drag::{DropletDragReport, DropletGas};

#[path = "liquid/droplet_finite_gas.rs"]
mod droplet_finite_gas;
pub use droplet_finite_gas::FiniteDropletDragReport;
#[path = "liquid/droplet_gas_grid.rs"]
mod droplet_gas_grid;
pub use droplet_gas_grid::{FiniteDropletGasGrid, GasGridTotals, SpatialDropletDragReport};
#[path = "liquid/droplet_gas_transport.rs"]
mod droplet_gas_transport;
pub use droplet_gas_transport::{GasGridBoundary, GasGridFlowControl, GasGridFlowReport};
#[path = "liquid/droplet_gas_heat.rs"]
mod droplet_gas_heat;
pub use droplet_gas_heat::{GasGridHeatControl, GasGridHeatReport};
#[path = "liquid/droplet_gas_viscosity.rs"]
mod droplet_gas_viscosity;
pub use droplet_gas_viscosity::{
    GasGridViscosityBoundary, GasGridViscosityControl, GasGridViscosityReport,
};
#[path = "liquid/droplet_gas_wall_viscosity.rs"]
mod droplet_gas_wall_viscosity;
pub use droplet_gas_wall_viscosity::{
    GasGridViscousReport, GasGridWallViscosityControl, GasGridWallViscosityReport,
};

#[path = "liquid/film_mixture_capture.rs"]
mod film_mixture_capture;
pub use film_mixture_capture::{DepositingImpact, DepositingImpactReport, FilmMixtureCapture};

#[path = "liquid/impact_spray.rs"]
mod impact_spray;
pub use impact_spray::{ImpactSpray, ImpactSprayReport};
#[path = "liquid/splash_onset.rs"]
mod splash_onset;
pub use splash_onset::{DryWallSplashOnset, ImpactNumbers};

#[path = "liquid/droplet_population.rs"]
mod droplet_population;

#[path = "liquid/geometry.rs"]
mod geometry;
pub use geometry::{GeometryHit, LiquidGeometry};

pub use dynamic_world::{DynamicEnvironmentReport, DynamicLiquidEnvironment};

#[path = "liquid/multi_body.rs"]
mod multi_body;
pub use multi_body::{
    BodyGeometryHit, ContactWitness, LiquidBodyWorld, RigidGeometryHit, RigidWorldReport,
};
