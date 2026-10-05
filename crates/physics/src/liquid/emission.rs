//! Configurable viscosity profiles and deterministic, volume-accounted pulse emission.
//! Presets are illustrative numerical controls.
use super::{Error, Liquid, Material, ParticleInput, ShearThinning, finite, norm, positive};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViscosityProfile {
    pub material: Material,
    pub shear_thinning: ShearThinning,
}
impl ViscosityProfile {
    pub const FLUID_DEMO: Self = Self::preset(0.01, 0.9);
    pub const VISCOUS_DEMO: Self = Self::preset(0.1, 0.7);
    pub const THICK_DEMO: Self = Self::preset(1.0, 0.5);
    const fn preset(viscosity: f64, flow_index: f64) -> Self {
        Self {
            material: Material {
                rest_density: 1000.0,
                sound_speed: 20.0,
                viscosity,
            },
            shear_thinning: ShearThinning {
                reference_rate: 1.0,
                flow_index,
                minimum_rate: 0.01,
                minimum_viscosity: viscosity * 0.01,
                maximum_viscosity: viscosity * 100.0,
            },
        }
    }
}

/// Constant-volume-rate pulse. Time is seconds and volume is cubic metres in SI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmissionPulse {
    pub start: f64,
    pub duration: f64,
    pub volume: f64,
    pub speed: f64,
}

/// Absolute conservation admission budgets in SI units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceAccuracy {
    pub mass_kg: f64,
    pub momentum_kg_m_s: f64,
    pub energy_j: f64,
}
impl Default for SourceAccuracy {
    fn default() -> Self {
        Self {
            mass_kg: 1e-12,
            momentum_kg_m_s: 1e-10,
            energy_j: 1e-10,
        }
    }
}
/// Momentum flux removed from an external source reservoir. Applying these
/// impulses to a body also requires removing the emitted mass from that body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmissionReaction {
    pub particles: super::ParticleExchange,
    pub source_impulse: [f64; 3],
    /// Angular impulse about the explicitly supplied world-space origin.
    pub source_angular_impulse: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PulsedEmitter {
    pulses: Vec<EmissionPulse>,
    elapsed: f64,
    emitted_samples: u32,
    /// Source position, material and transported fields. Mass/velocity are replaced.
    pub template: ParticleInput,
    pub direction: [f64; 3],
    /// World velocity of the nozzle in m/s, added to relative pulse velocity.
    /// Position/orientation updates remain explicit caller-owned transforms.
    pub source_velocity: [f64; 3],
    /// Explicit source density; must match the desired material calibration.
    pub density: f64,
    /// Upper bound on each emitted particle's volume.
    pub particle_volume: f64,
    /// Circular aperture radius, perpendicular to direction. Zero keeps a point source.
    /// Area sampling does not guarantee minimum particle separation.
    pub nozzle_radius: f64,
}
impl PulsedEmitter {
    /// # Errors
    /// Rejects negative times/volumes, nonpositive durations and invalid speeds.
    pub fn new(pulses: Vec<EmissionPulse>, template: ParticleInput) -> Result<Self, Error> {
        if pulses.iter().any(|p| {
            !p.start.is_finite()
                || p.start < 0.0
                || !positive(p.duration)
                || !(p.start + p.duration).is_finite()
                || !p.volume.is_finite()
                || p.volume < 0.0
                || !p.speed.is_finite()
                || p.speed < 0.0
        }) {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            pulses,
            elapsed: 0.0,
            emitted_samples: 0,
            template,
            direction: [0.0, 1.0, 0.0],
            source_velocity: [0.0; 3],
            density: 1000.0,
            particle_volume: 1e-8,
            nozzle_radius: 0.0,
        })
    }
    #[must_use]
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    /// Emits the overlap of every pulse with this interval; failure rolls back time and fluid.
    /// Partial final particles preserve volume rather than rounding it away.
    /// # Errors
    /// Invalid settings/time, overflow, source validation or the liquid particle budget.
    pub fn advance(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
    ) -> Result<super::ParticleExchange, Error> {
        self.advance_composed(liquid, dt, None, None)
            .map(|(ledger, _)| ledger)
    }
    /// Emits source particles with a complete species mass-fraction vector.
    /// # Errors
    /// Usual emission errors or missing/invalid composition configuration.
    /// Both elapsed source time and the complete fluid roll back together.
    pub fn advance_with_species(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        composition: &[f64],
    ) -> Result<super::ParticleExchange, Error> {
        self.advance_composed(liquid, dt, Some(composition), None)
            .map(|(ledger, _)| ledger)
    }
    /// Emits particles and returns opposite linear/angular momentum flux for
    /// an external source reservoir. Does not mutate a rigid body or reservoir.
    /// All receipt validation precedes publication of fluid and emission time.
    pub fn advance_with_reaction(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        origin: [f64; 3],
        composition: Option<&[f64]>,
    ) -> Result<EmissionReaction, Error> {
        let (particles, angular) = self.advance_composed(liquid, dt, composition, Some(origin))?;
        Ok(EmissionReaction {
            source_impulse: particles.added.momentum.map(|v| -v),
            source_angular_impulse: angular.map(|v| -v),
            particles,
        })
    }
    /// Point nozzle at the centre of a translating source. Source mass includes
    /// retained dry mass. Energy reserve pays for fluid thermal/kinetic energy
    /// and recoil. No position drift, rotating nozzle or rigid-shape update.
    /// Fluid, emitter, source mechanics and reserve publish atomically.
    pub fn advance_from_translating_source(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        body: &mut super::TranslatingBody,
        dry_mass: f64,
        energy_reserve_j: &mut f64,
        composition: Option<&[f64]>,
    ) -> Result<EmissionReaction, Error> {
        self.advance_from_translating_source_with_accuracy(
            liquid,
            dt,
            body,
            dry_mass,
            energy_reserve_j,
            composition,
            SourceAccuracy::default(),
        )
    }
    /// Same transaction with explicit absolute conservation tolerances.
    pub fn advance_from_translating_source_with_accuracy(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        body: &mut super::TranslatingBody,
        dry_mass: f64,
        energy_reserve_j: &mut f64,
        composition: Option<&[f64]>,
        accuracy: SourceAccuracy,
    ) -> Result<EmissionReaction, Error> {
        if !positive(accuracy.mass_kg)
            || !positive(accuracy.momentum_kg_m_s)
            || !positive(accuracy.energy_j)
        {
            return Err(Error::InvalidConfig);
        }
        if !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
            || !positive(dry_mass)
            || body.mass < dry_mass
            || self.nozzle_radius != 0.
            || !energy_reserve_j.is_finite()
            || *energy_reserve_j < 0.
        {
            return Err(Error::InvalidConfig);
        }
        let kinetic = |mass: f64, velocity: [f64; 3]| {
            velocity.iter().map(|v| (0.5 * mass * v) * v).sum::<f64>()
        };
        let initial_kinetic = kinetic(body.mass, body.velocity);
        if !initial_kinetic.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let mut source = self.clone();
        source.template.particle.position = body.position;
        source.source_velocity = body.velocity;
        let (_, _, prepared) =
            source.prepare_emission(dt, liquid.config.max_particles - liquid.particles.len())?;
        let mut emitted_mass = 0.;
        let mut exhaust_momentum = [0.; 3];
        for input in prepared {
            let particle = input.particle;
            if !positive(particle.mass) {
                return Err(Error::NumericalFailure);
            }
            emitted_mass += particle.mass;
            for axis in 0..3 {
                exhaust_momentum[axis] +=
                    particle.mass * (particle.velocity[axis] - body.velocity[axis]);
            }
        }
        let mass = body.mass - emitted_mass;
        if !emitted_mass.is_finite()
            || !finite(exhaust_momentum)
            || !positive(mass)
            || mass < dry_mass
            || (emitted_mass > 0. && mass == body.mass)
        {
            return Err(Error::NumericalFailure);
        }
        if emitted_mass == 0. {
            // Empty exchange validates transport configuration without cloning
            // or rebuilding fluid arrays. Only source clock/config publishes.
            let receipt = source.advance_with_reaction(liquid, dt, body.position, composition)?;
            *self = source;
            return Ok(receipt);
        }
        let average_mass = 0.5 * body.mass + 0.5 * mass;
        source.source_velocity = std::array::from_fn(|axis| {
            body.velocity[axis] - 0.5 * exhaust_momentum[axis] / average_mass
        });
        let mut fluid = liquid.clone();
        let receipt = source.advance_with_reaction(&mut fluid, dt, body.position, composition)?;
        if receipt.particles.added.mass != emitted_mass {
            return Err(Error::NumericalFailure);
        }
        // Evaluate emitted relative kinetic energy and recoil in the source
        // frame. Common translational kinetic energy cancels analytically.
        let mut relative_momentum = [0.; 3];
        let mut relative_energy = 0.;
        for particle in &fluid.particles()[liquid.particles().len()..] {
            let relative =
                std::array::from_fn(|axis| particle.velocity[axis] - body.velocity[axis]);
            for axis in 0..3 {
                relative_momentum[axis] += particle.mass * relative[axis];
            }
            relative_energy += kinetic(particle.mass, relative);
        }
        let recoil = relative_momentum.map(|p| -p / mass);
        let velocity = std::array::from_fn(|axis| body.velocity[axis] + recoil[axis]);
        if (0..3).any(|axis| recoil[axis] != 0. && velocity[axis] == body.velocity[axis]) {
            return Err(Error::NumericalFailure);
        }
        let final_kinetic = kinetic(mass, velocity);
        let required = relative_energy
            + kinetic(mass, recoil)
            + receipt.particles.added.thermal_energy.unwrap_or(0.);
        let reserve = *energy_reserve_j - required;
        if !finite(velocity)
            || !final_kinetic.is_finite()
            || !required.is_finite()
            || !reserve.is_finite()
            || reserve < 0.
            || (required != 0. && reserve == *energy_reserve_j)
        {
            return Err(Error::NumericalFailure);
        }
        let mass_defect = (mass - body.mass) + receipt.particles.added.mass;
        let actual_recoil =
            std::array::from_fn::<_, 3, _>(|axis| velocity[axis] - body.velocity[axis]);
        let relative_defect = std::array::from_fn::<_, 3, _>(|axis| {
            mass * actual_recoil[axis] + relative_momentum[axis]
        });
        let momentum_defect = std::array::from_fn::<_, 3, _>(|axis| {
            relative_defect[axis] + mass_defect * body.velocity[axis]
        });
        let energy_defect = relative_energy
            + kinetic(mass, actual_recoil)
            + receipt.particles.added.thermal_energy.unwrap_or(0.)
            + relative_defect
                .iter()
                .zip(body.velocity)
                .map(|(p, v)| p * v)
                .sum::<f64>()
            + kinetic(mass_defect, body.velocity)
            + (reserve - *energy_reserve_j);
        if !mass_defect.is_finite()
            || mass_defect.abs() > accuracy.mass_kg
            || !finite(momentum_defect)
            || momentum_defect
                .iter()
                .any(|p| p.abs() > accuracy.momentum_kg_m_s)
            || !energy_defect.is_finite()
            || energy_defect.abs() > accuracy.energy_j
        {
            return Err(Error::NumericalFailure);
        }
        *self = source;
        *liquid = fluid;
        body.mass = mass;
        body.velocity = velocity;
        *energy_reserve_j = reserve;
        Ok(receipt)
    }
    fn prepare_emission(
        &self,
        dt: f64,
        available: usize,
    ) -> Result<(f64, u32, Vec<ParticleInput>), Error> {
        let end = self.elapsed + dt;
        let scale = self.direction.iter().map(|v| v.abs()).fold(0., f64::max);
        let scaled = self.direction.map(|v| v / scale);
        let length = norm(scaled);
        if !positive(dt)
            || !end.is_finite()
            || end <= self.elapsed
            || !finite(self.source_velocity)
            || !finite(self.direction)
            || !positive(scale)
            || !positive(length)
            || !positive(self.density)
            || !positive(self.particle_volume)
            || !self.nozzle_radius.is_finite()
            || self.nozzle_radius < 0.0
        {
            return Err(Error::InvalidConfig);
        }
        let mut add = Vec::new();
        let mut samples = self.emitted_samples;
        let direction = scaled.map(|v| v / length);
        let (tangent, bitangent) = aperture_frame(direction);
        for pulse in &self.pulses {
            let overlap =
                (end.min(pulse.start + pulse.duration) - self.elapsed.max(pulse.start)).max(0.0);
            let mut volume = pulse.volume * (overlap / pulse.duration);
            if !volume.is_finite() {
                return Err(Error::NumericalFailure);
            }
            while volume > 0.0 {
                if add.len() >= available {
                    return Err(Error::ParticleBudget);
                }
                let part = volume.min(self.particle_volume);
                let mut source = self.template;
                source.particle.mass = part * self.density;
                source.particle.velocity = std::array::from_fn(|axis| {
                    direction[axis] * pulse.speed + self.source_velocity[axis]
                });
                if (0..3).any(|axis| {
                    direction[axis] * pulse.speed != 0.
                        && source.particle.velocity[axis] == self.source_velocity[axis]
                }) {
                    return Err(Error::NumericalFailure);
                }
                if !finite(source.particle.velocity) {
                    return Err(Error::NumericalFailure);
                }
                if self.nozzle_radius > 0.0 {
                    samples = samples.checked_add(1).ok_or(Error::NumericalFailure)?;
                    // Base-2 radical inverse gives uniform disk area; golden-angle azimuth.
                    let radius = self.nozzle_radius
                        * (f64::from(samples.reverse_bits()) / 4_294_967_296.0).sqrt();
                    let angle = f64::from(samples) * std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
                    for axis in 0..3 {
                        source.particle.position[axis] +=
                            radius * (angle.cos() * tangent[axis] + angle.sin() * bitangent[axis]);
                    }
                }
                add.push(source);
                let remaining = volume - part;
                if remaining >= volume {
                    return Err(Error::NumericalFailure);
                }
                volume = remaining;
            }
        }
        Ok((end, samples, add))
    }
    fn advance_composed(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        composition: Option<&[f64]>,
        origin: Option<[f64; 3]>,
    ) -> Result<(super::ParticleExchange, [f64; 3]), Error> {
        if origin.is_some_and(|p| !finite(p)) {
            return Err(Error::InvalidConfig);
        }
        if let Some(row) = composition {
            let count = liquid.species_names().ok_or(Error::InvalidTransport)?.len();
            super::species::validate_row(row, count)?;
        }
        let (end, samples, add) =
            self.prepare_emission(dt, liquid.config.max_particles - liquid.particles.len())?;
        let mut angular = [0.; 3];
        if let Some(origin) = origin {
            for source in &add {
                let r = std::array::from_fn::<_, 3, _>(|i| source.particle.position[i] - origin[i]);
                let p = source.particle.velocity.map(|v| v * source.particle.mass);
                let cross = [
                    r[1] * p[2] - r[2] * p[1],
                    r[2] * p[0] - r[0] * p[2],
                    r[0] * p[1] - r[1] * p[0],
                ];
                for axis in 0..3 {
                    angular[axis] += cross[axis];
                }
                if !finite(r) || !finite(p) || !finite(angular) {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        let ledger = if let Some(row) = composition {
            let rows: Vec<_> = add.iter().map(|_| row.to_vec()).collect();
            liquid.exchange_particles_with_species(&[], &add, &rows)?
        } else {
            liquid.exchange_particles(&[], &add)?
        };
        self.elapsed = end;
        self.emitted_samples = samples;
        Ok((ledger, angular))
    }
}

fn aperture_frame(direction: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    // Cross with the least-aligned coordinate axis to avoid a vanishing tangent.
    let axis = (0..3)
        .min_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
        .unwrap_or(0);
    let mut reference = [0.0; 3];
    reference[axis] = 1.0;
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let tangent = cross(direction, reference);
    let length = norm(tangent);
    let tangent = tangent.map(|v| v / length);
    (tangent, cross(direction, tangent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, Particle};
    #[test]
    fn aperture_emission_is_invariant_under_extreme_direction_rescaling() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 4e-8,
                speed: 2.,
            }],
            template,
        )
        .unwrap();
        source.direction = [1., 2., -3.];
        source.nozzle_radius = 0.1;
        let fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut baseline = fluid.clone();
        source.clone().advance(&mut baseline, 1.).unwrap();
        for factor in [1e-300, 1e300] {
            let mut scaled = source.clone();
            scaled.direction = source.direction.map(|v| v * factor);
            let mut actual = fluid.clone();
            scaled.advance(&mut actual, 1.).unwrap();
            for (a, b) in actual.particles().iter().zip(baseline.particles()) {
                assert_eq!(a.mass, b.mass);
                for axis in 0..3 {
                    assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-14);
                    assert!((a.position[axis] - b.position[axis]).abs() < 1e-14);
                }
            }
        }
        source.direction = [0.; 3];
        let before = source.clone();
        let mut actual = fluid.clone();
        assert_eq!(source.advance(&mut actual, 1.), Err(Error::InvalidConfig));
        assert_eq!(source, before);
        assert!(actual.particles().is_empty());
    }
    #[test]
    fn idle_finite_source_advances_only_clock_and_rejects_invalid_composition() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 1.,
                duration: 1.,
                volume: 0.1,
                speed: 2.,
            }],
            template,
        )
        .unwrap();
        let mut fluid = Liquid::new(
            vec![template.particle],
            vec![Material::WATER],
            Config::default(),
        )
        .unwrap();
        let mut body = super::super::TranslatingBody {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 2.,
        };
        let original_body = body;
        let particles = fluid.particles().to_vec();
        let storage = fluid.particles().as_ptr();
        let mut reserve = 10.;
        let receipt = source
            .advance_from_translating_source(&mut fluid, 0.1, &mut body, 1., &mut reserve, None)
            .unwrap();
        assert_eq!(receipt.particles.added.mass, 0.);
        assert_eq!(source.elapsed(), 0.1);
        assert_eq!(body, original_body);
        assert_eq!(reserve, 10.);
        assert_eq!(fluid.particles(), particles);
        assert_eq!(fluid.particles().as_ptr(), storage);
        let before = source.clone();
        assert!(
            source
                .advance_from_translating_source(
                    &mut fluid,
                    0.1,
                    &mut body,
                    1.,
                    &mut reserve,
                    Some(&[1.])
                )
                .is_err()
        );
        assert_eq!(source, before);
        assert_eq!(fluid.particles().as_ptr(), storage);
    }
    #[test]
    fn finite_source_recoil_refines_to_continuous_variable_mass_solution() {
        let mut errors = Vec::new();
        let mut position_errors = Vec::new();
        for steps in [16, 32, 64] {
            let template = ParticleInput {
                particle: Particle {
                    position: [0.; 3],
                    velocity: [0.; 3],
                    mass: 1.,
                    material: 0,
                },
                field: None,
                phase_fraction: None,
            };
            let mut source = PulsedEmitter::new(
                vec![EmissionPulse {
                    start: 0.,
                    duration: 1.,
                    volume: 0.5,
                    speed: 2.,
                }],
                template,
            )
            .unwrap();
            source.density = 1.;
            source.particle_volume = 1.;
            source.direction = [1., 0., 0.];
            let mut fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
            let mut body = super::super::TranslatingBody {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 2.,
            };
            let mut reserve = 10.;
            for _ in 0..steps {
                let old_velocity = body.velocity;
                source
                    .advance_from_translating_source(
                        &mut fluid,
                        1. / f64::from(steps),
                        &mut body,
                        1.,
                        &mut reserve,
                        None,
                    )
                    .unwrap();
                for axis in 0..3 {
                    body.position[axis] +=
                        (0.5 * old_velocity[axis] + 0.5 * body.velocity[axis]) / f64::from(steps);
                }
            }
            // dV = -u dm/M, integrated over source mass 2 -> 1.5.
            let exact = -2. * (2_f64 / 1.5).ln();
            errors.push((body.velocity[0] - exact).abs());
            let exact_position = -2. * (1. - 3. * (2_f64 / 1.5).ln());
            position_errors.push((body.position[0] - exact_position).abs());
            assert!((body.mass - 1.5).abs() < 1e-12);
            let momentum = body.mass * body.velocity[0]
                + fluid
                    .particles()
                    .iter()
                    .map(|p| p.mass * p.velocity[0])
                    .sum::<f64>();
            assert!(momentum.abs() < 1e-12);
            let energy = 0.5 * body.mass * body.velocity[0].powi(2)
                + fluid
                    .particles()
                    .iter()
                    .map(|p| 0.5 * p.mass * p.velocity[0].powi(2))
                    .sum::<f64>()
                + reserve;
            assert!((energy - 10.).abs() < 1e-12);
        }
        // Midpoint source velocity gives second-order endpoint convergence.
        assert!(errors[0] / errors[1] > 3.8 && errors[0] / errors[1] < 4.2);
        assert!(errors[1] / errors[2] > 3.8 && errors[1] / errors[2] < 4.2);
        assert!(
            position_errors[0] / position_errors[1] > 3.8
                && position_errors[0] / position_errors[1] < 4.2
        );
        assert!(
            position_errors[1] / position_errors[2] > 3.8
                && position_errors[1] / position_errors[2] < 4.2
        );
    }
    #[test]
    fn finite_source_large_boost_admission_and_energy_rounding_are_bounded() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 0.2,
                speed: 2.,
            }],
            template,
        )
        .unwrap();
        let mut remaining = Vec::new();
        for boost in [0., 1e6, 1e12] {
            let mut source = source.clone();
            source.density = 1.;
            source.particle_volume = 0.2;
            source.direction = [1., 0., 0.];
            let mut fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
            let mut body = super::super::TranslatingBody {
                position: [0.; 3],
                velocity: [boost, 0., 0.],
                mass: 2.,
            };
            let mut reserve = 10.;
            if boost == 1e12 {
                let before_source = source.clone();
                let before_body = body;
                assert_eq!(
                    source.advance_from_translating_source(
                        &mut fluid,
                        1.,
                        &mut body,
                        1.,
                        &mut reserve,
                        None
                    ),
                    Err(Error::NumericalFailure)
                );
                assert_eq!(source, before_source);
                assert_eq!(body, before_body);
                assert_eq!(reserve, 10.);
                assert!(fluid.particles().is_empty());
            }

            source
                .advance_from_translating_source_with_accuracy(
                    &mut fluid,
                    1.,
                    &mut body,
                    1.,
                    &mut reserve,
                    None,
                    SourceAccuracy {
                        mass_kg: 1e-12,
                        momentum_kg_m_s: 1.,
                        energy_j: 1e12,
                    },
                )
                .unwrap();
            remaining.push(reserve);
        }
        assert!((remaining[1] - remaining[0]).abs() < 1e-8);
        // Loose-budget extreme boost is diagnostic: midpoint world velocities
        // round, so only bounded reserve deviation is claimed.
        assert!((remaining[2] - remaining[0]).abs() < 1e-3);
    }
    #[test]
    fn finite_source_recoil_conserves_mass_momentum_energy_and_late_failure() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 0.2,
                speed: 2.,
            }],
            template,
        )
        .unwrap();
        emitter.density = 1.;
        emitter.particle_volume = 0.2;
        emitter.direction = [1., 0., 0.];
        let mut fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut body = super::super::TranslatingBody {
            position: [1., 2., 3.],
            velocity: [3., 0., 0.],
            mass: 2.,
        };
        let mut reserve = 10.;
        let initial_emitter = emitter.clone();
        let initial_body = body;
        emitter
            .advance_from_translating_source(&mut fluid, 1., &mut body, 1., &mut reserve, None)
            .unwrap();
        let p = fluid.particles()[0];
        assert!((body.mass + p.mass - 2.).abs() < 1e-14);
        assert!((body.mass * body.velocity[0] + p.mass * p.velocity[0] - 6.).abs() < 1e-14);
        let energy = 0.5 * body.mass * body.velocity[0].powi(2)
            + 0.5 * p.mass * p.velocity[0].powi(2)
            + reserve;
        assert!((energy - 19.).abs() < 1e-14);
        assert!(body.velocity[0] < 3.);
        assert_eq!(p.position, initial_body.position);
        // Insufficient energy fails after a staged successful fluid emission.
        let mut failed = initial_emitter.clone();
        let mut failed_body = initial_body;
        let before = fluid.particles().to_vec();
        let mut empty = 0.;
        assert!(
            failed
                .advance_from_translating_source(
                    &mut fluid,
                    1.,
                    &mut failed_body,
                    1.,
                    &mut empty,
                    None
                )
                .is_err()
        );
        assert_eq!(failed, initial_emitter);
        assert_eq!(failed_body, initial_body);
        assert_eq!(empty, 0.);
        assert_eq!(fluid.particles(), before);
        assert!(
            failed
                .advance_from_translating_source(
                    &mut fluid,
                    1.,
                    &mut failed_body,
                    1.9,
                    &mut reserve,
                    None
                )
                .is_err()
        );
        assert_eq!(failed_body, initial_body);
        assert_eq!(fluid.particles(), before);
    }
    #[test]
    fn reaction_books_actual_momentum_and_origin_dependent_torque_atomically() {
        let template = ParticleInput {
            particle: Particle {
                position: [0., 2., 0.],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 1e-8,
                speed: 3.,
            }],
            template,
        )
        .unwrap();
        emitter.direction = [1., 0., 0.];
        let mut liquid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut shifted = emitter.clone();
        let mut other = liquid.clone();
        let receipt = emitter
            .advance_with_reaction(&mut liquid, 1., [0.; 3], None)
            .unwrap();
        let mass = liquid.particles()[0].mass;
        assert!((receipt.source_impulse[0] + mass * 3.).abs() < 1e-18);
        assert!((receipt.source_angular_impulse[2] - mass * 6.).abs() < 1e-18);
        let at_nozzle = shifted
            .advance_with_reaction(&mut other, 1., [0., 2., 0.], None)
            .unwrap();
        assert_eq!(at_nozzle.source_angular_impulse, [0.; 3]);
        assert_eq!(at_nozzle.source_impulse, receipt.source_impulse);
        let mut invalid = PulsedEmitter::new(emitter.pulses.clone(), template).unwrap();
        invalid.template.particle.position = [f64::MAX; 3];
        invalid.pulses[0].speed = 1e100;
        let before = liquid.particles().to_vec();
        assert_eq!(
            invalid.advance_with_reaction(&mut liquid, 1., [0.; 3], None),
            Err(Error::NumericalFailure)
        );
        assert_eq!(invalid.elapsed(), 0.);
        assert_eq!(liquid.particles(), before);
    }
    #[test]
    fn moving_nozzle_is_galilean_covariant_and_invalid_velocity_rolls_back() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.; 3],
                velocity: [99.; 3],
                mass: 1.,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut fixed = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 4e-8,
                speed: 2.,
            }],
            template,
        )
        .unwrap();
        fixed.direction = [1., 0., 0.];
        let mut moving = fixed.clone();
        moving.source_velocity = [3., -4., 5.];
        let mut a = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut b = a.clone();
        fixed.advance(&mut a, 1.).unwrap();
        moving.advance(&mut b, 1.).unwrap();
        assert_eq!(a.particles().len(), b.particles().len());
        for (a, b) in a.particles().iter().zip(b.particles()) {
            assert_eq!(a.mass, b.mass);
            assert_eq!(a.position, b.position);
            assert_eq!(a.velocity, [2., 0., 0.]);
            assert_eq!(b.velocity, [5., -4., 5.]);
        }
        // Actual particle momentum shift is mass times the nozzle boost.
        for axis in 0..3 {
            let delta: f64 = a
                .particles()
                .iter()
                .zip(b.particles())
                .map(|(a, b)| b.mass * b.velocity[axis] - a.mass * a.velocity[axis])
                .sum();
            let mass: f64 = a.particles().iter().map(|p| p.mass).sum();
            assert!((delta - mass * moving.source_velocity[axis]).abs() < 1e-18);
        }
        let mut invalid = PulsedEmitter::new(fixed.pulses.clone(), template).unwrap();
        invalid.source_velocity = [f64::NAN, 0., 0.];
        let state = invalid.clone();
        let before = b.particles().to_vec();
        assert!(invalid.advance(&mut b, 1.).is_err());
        assert!(invalid.elapsed() == state.elapsed());
        assert_eq!(before, b.particles());
        invalid.source_velocity = [f64::MAX, 0., 0.];
        invalid.direction = [1., 0., 0.];
        invalid.pulses[0].speed = f64::MAX;
        assert_eq!(invalid.advance(&mut b, 1.), Err(Error::NumericalFailure));
        assert_eq!(invalid.elapsed(), 0.);
        assert_eq!(before, b.particles());
    }
    #[test]
    fn circular_aperture_samples_area_in_the_normal_plane_and_rolls_back() {
        let template = ParticleInput {
            particle: Particle {
                position: [4.0, 5.0, 6.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let particle_volume = 2.0_f64.powi(-30);
        let mut source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 128.0 * particle_volume,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        source.particle_volume = particle_volume;
        source.nozzle_radius = 0.1;
        source.direction = [1.0, 2.0, 3.0];
        let mut fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut point = source.clone();
        point.nozzle_radius = 0.0;
        let mut point_fluid = fluid.clone();
        let point_ledger = point.advance(&mut point_fluid, 1.0).unwrap();
        let ledger = source.advance(&mut fluid, 1.0).unwrap();
        assert_eq!(ledger, point_ledger);
        assert_eq!(fluid.particles.len(), 128);
        let direction = source.direction.map(|v| v / 14.0_f64.sqrt());
        let mut mean_squared_radius = 0.0;
        for (i, particle) in fluid.particles.iter().enumerate() {
            let delta = super::super::sub(particle.position, template.particle.position);
            let radius = norm(delta);
            assert!(radius <= source.nozzle_radius);
            let normal_distance: f64 = delta.iter().zip(direction).map(|(v, n)| v * n).sum();
            assert!(normal_distance.abs() < 1e-14);
            mean_squared_radius += radius * radius / 128.0;
            for other in &fluid.particles[..i] {
                assert!(norm(super::super::sub(particle.position, other.position)) > 1e-7);
            }
        }
        assert!((mean_squared_radius / source.nozzle_radius.powi(2) - 0.5).abs() < 0.01);
        let mut failed = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 128.0 * particle_volume,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        failed.nozzle_radius = 0.1;
        failed.template.particle.material = 99;
        let before = failed.clone();
        let before_fluid = fluid.clone();
        assert!(failed.advance(&mut fluid, 1.0).is_err());
        assert_eq!(failed, before);
        assert_eq!(fluid, before_fluid);
    }
    #[test]
    fn pulses_conserve_volume_momentum_and_rollback() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.1,
                duration: 0.2,
                volume: 3e-8,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        let mut liquid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        emitter.advance(&mut liquid, 0.1).unwrap();
        assert!(liquid.particles.is_empty());
        let first = emitter.advance(&mut liquid, 0.1).unwrap();
        let second = emitter.advance(&mut liquid, 0.2).unwrap();
        assert!((first.added.mass + second.added.mass - 3e-5).abs() < 1e-18);
        assert!((first.added.momentum[1] + second.added.momentum[1] - 6e-5).abs() < 1e-18);
        let before = emitter.clone();
        let fluid_before = liquid.clone();
        assert!(emitter.advance(&mut liquid, f64::NAN).is_err());
        assert_eq!(emitter, before);
        assert_eq!(liquid, fluid_before);
        let mut limited = Liquid::new(
            vec![],
            vec![Material::WATER],
            Config {
                max_particles: 1,
                ..Config::default()
            },
        )
        .unwrap();
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 3e-8,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        assert_eq!(
            emitter.advance(&mut limited, 1.0),
            Err(Error::ParticleBudget)
        );
        assert_eq!(emitter.elapsed(), 0.0);
        assert!(limited.particles.is_empty());
    }
}
