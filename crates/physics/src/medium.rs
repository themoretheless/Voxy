//! Unified participating medium: smoke, fog, dust and water mist as scalar
//! species carried by the finite ideal-gas grid. Species are advected by the
//! gas velocity (first-order upwind, flux form), diffuse explicitly, settle
//! along `-up`, and push the gas with buoyancy. Every step is atomic and its
//! mass budget is reported so a caller can audit sources and sinks.

use crate::liquid::{
    DropletExtinctionGrid, Error, FiniteDropletGasGrid, GasGridFlowControl, GasGridFlowReport,
};
use crate::suspension::Particle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediumSpecies {
    Smoke,
    Fog,
    Dust,
    Mist,
}

/// Optical and transport properties of one species.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediumSpeciesProperties {
    pub kind: MediumSpecies,
    /// Extinction per unit mass concentration, m²/kg.
    pub mass_extinction_m2_kg: f64,
    /// Single-scattering albedo in `[0, 1]`.
    pub albedo: f64,
    /// Terminal settling speed along `-up`, m/s (≥ 0).
    pub settling_velocity_m_s: f64,
    /// Molecular/turbulent diffusivity, m²/s (≥ 0).
    pub diffusivity_m2_s: f64,
    /// Gas acceleration along `up` per unit concentration, (m/s²)/(kg/m³).
    pub buoyancy_per_kg_m3: f64,
}

impl MediumSpeciesProperties {
    fn validate(&self) -> Result<(), Error> {
        let ok = self.mass_extinction_m2_kg.is_finite()
            && self.mass_extinction_m2_kg >= 0.
            && self.albedo.is_finite()
            && (0. ..=1.).contains(&self.albedo)
            && self.settling_velocity_m_s.is_finite()
            && self.settling_velocity_m_s >= 0.
            && self.diffusivity_m2_s.is_finite()
            && self.diffusivity_m2_s >= 0.
            && self.buoyancy_per_kg_m3.is_finite();
        if ok {
            Ok(())
        } else {
            Err(Error::InvalidConfig)
        }
    }

    /// Mist of water droplets of radius `r`: `3 Q / (4 ρ_w r)` m²/kg, the
    /// same cross-section-per-mass as `Liquid::droplet_extinction_grid`.
    #[must_use]
    pub fn water_mist(radius_m: f64, extinction_efficiency: f64, water_density_kg_m3: f64) -> Self {
        Self {
            kind: MediumSpecies::Mist,
            mass_extinction_m2_kg: 3. * extinction_efficiency
                / (4. * water_density_kg_m3 * radius_m),
            albedo: 1.,
            settling_velocity_m_s: 0.,
            diffusivity_m2_s: 0.,
            buoyancy_per_kg_m3: 0.,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MediumStepControl {
    pub flow: GasGridFlowControl,
    /// Gravity vector; `up` is its negation.
    pub gravity: [f64; 3],
    /// Whether the gas itself is advanced by the Euler solver this step.
    pub advance_gas: bool,
    pub max_substeps: usize,
}

impl Default for MediumStepControl {
    fn default() -> Self {
        Self {
            flow: GasGridFlowControl::default(),
            gravity: [0., -9.81, 0.],
            advance_gas: true,
            max_substeps: 4096,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediumStepReport {
    pub flow: Option<GasGridFlowReport>,
    pub substeps: usize,
    /// Mass per species that left through the bottom face by settling.
    pub settled_kg: Vec<f64>,
    /// Mass per species before and after, for conservation audits.
    pub mass_before_kg: Vec<f64>,
    pub mass_after_kg: Vec<f64>,
}

/// Gas grid plus per-species mass per cell.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticipatingMedium {
    gas: FiniteDropletGasGrid,
    species: Vec<MediumSpeciesProperties>,
    /// `mass[s][cell]` in kilograms.
    mass: Vec<Vec<f64>>,
}

impl ParticipatingMedium {
    /// # Errors
    /// Invalid species properties or an empty species list.
    pub fn new(
        gas: FiniteDropletGasGrid,
        species: Vec<MediumSpeciesProperties>,
    ) -> Result<Self, Error> {
        if species.is_empty() {
            return Err(Error::InvalidConfig);
        }
        for s in &species {
            s.validate()?;
        }
        let cells = gas.cells().len();
        Ok(Self {
            mass: vec![vec![0.; cells]; species.len()],
            gas,
            species,
        })
    }

    #[must_use]
    pub fn gas(&self) -> &FiniteDropletGasGrid {
        &self.gas
    }

    #[must_use]
    pub fn species(&self) -> &[MediumSpeciesProperties] {
        &self.species
    }

    /// Mass per cell of one species.
    #[must_use]
    pub fn mass_kg(&self, species: usize) -> &[f64] {
        &self.mass[species]
    }

    #[must_use]
    pub fn total_mass_kg(&self, species: usize) -> f64 {
        self.mass[species].iter().sum()
    }

    /// Concentration per cell, kg/m³.
    #[must_use]
    pub fn concentration_kg_m3(&self, species: usize) -> Vec<f64> {
        let volume = self.cell_volume();
        self.mass[species].iter().map(|m| m / volume).collect()
    }

    fn cell_volume(&self) -> f64 {
        self.gas.spacing().iter().product::<f64>()
    }

    /// Adds `mass_kg` of a species at `point`; outside the grid nothing is
    /// deposited and `Ok(false)` is returned.
    /// # Errors
    /// Unknown species, nonfinite point or negative mass.
    pub fn deposit(
        &mut self,
        species: usize,
        point: [f64; 3],
        mass_kg: f64,
    ) -> Result<bool, Error> {
        if species >= self.species.len() || !mass_kg.is_finite() || mass_kg < 0. {
            return Err(Error::InvalidConfig);
        }
        match self.gas.cell_index(point)? {
            Some(cell) => {
                self.mass[species][cell] += mass_kg;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Deposits suspension grains (fracture or wear dust) as a species.
    /// Returns the mass that landed inside the grid.
    /// # Errors
    /// Unknown species.
    pub fn deposit_particles(
        &mut self,
        species: usize,
        particles: &[Particle],
    ) -> Result<f64, Error> {
        let mut inside = 0.;
        for p in particles {
            if self.deposit(species, p.position_m(), p.mass_kg())? {
                inside += p.mass_kg();
            }
        }
        Ok(inside)
    }

    /// Advances species transport and, optionally, the gas by `dt`. Atomic.
    /// # Errors
    /// Invalid step, substep budget, or gas solver failure.
    pub fn advance(
        &mut self,
        dt: f64,
        control: MediumStepControl,
    ) -> Result<MediumStepReport, Error> {
        if !dt.is_finite()
            || dt < 0.
            || control.gravity.iter().any(|g| !g.is_finite())
            || control.max_substeps == 0
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        let mut report = MediumStepReport {
            flow: None,
            substeps: 0,
            settled_kg: vec![0.; self.species.len()],
            mass_before_kg: (0..self.species.len())
                .map(|s| self.total_mass_kg(s))
                .collect(),
            mass_after_kg: Vec::new(),
        };
        if dt == 0. {
            report.mass_after_kg.clone_from(&report.mass_before_kg);
            return Ok(report);
        }
        // Buoyancy: species concentration accelerates the gas along `up`.
        let g = control.gravity.iter().map(|v| v * v).sum::<f64>().sqrt();
        let up = if g > 0. {
            control.gravity.map(|v| -v / g)
        } else {
            [0.; 3]
        };
        let volume = candidate.cell_volume();
        {
            let cells = candidate.gas.cells_mut();
            for (s, props) in candidate.species.iter().enumerate() {
                if props.buoyancy_per_kg_m3 == 0. {
                    continue;
                }
                for (cell, mass) in cells.iter_mut().zip(&candidate.mass[s]) {
                    let accel = props.buoyancy_per_kg_m3 * (mass / volume);
                    for k in 0..3 {
                        cell.velocity[k] += up[k] * accel * dt;
                    }
                }
            }
        }
        if control.advance_gas {
            report.flow = Some(candidate.gas.advance_euler(dt, control.flow)?);
        }
        // Species transport with a CFL/diffusion-stable substep.
        let spacing = candidate.gas.spacing();
        let shape = candidate.gas.shape();
        let h_min = spacing.iter().copied().fold(f64::INFINITY, f64::min);
        let max_speed = candidate
            .gas
            .cells()
            .iter()
            .map(|c| c.velocity.iter().map(|v| v.abs()).fold(0., f64::max))
            .fold(0., f64::max);
        let max_settle = candidate
            .species
            .iter()
            .map(|s| s.settling_velocity_m_s)
            .fold(0., f64::max);
        let max_diffusion = candidate
            .species
            .iter()
            .map(|s| s.diffusivity_m2_s)
            .fold(0., f64::max);
        let advective_limit = if max_speed + max_settle > 0. {
            0.5 * h_min / (max_speed + max_settle)
        } else {
            f64::INFINITY
        };
        let diffusive_limit = if max_diffusion > 0. {
            h_min * h_min / (6. * max_diffusion) * 0.5
        } else {
            f64::INFINITY
        };
        let limit = advective_limit.min(diffusive_limit);
        let substeps = if limit.is_finite() {
            (dt / limit).ceil().max(1.) as usize
        } else {
            1
        };
        if substeps > control.max_substeps {
            return Err(Error::SubstepBudget);
        }
        let h = dt / substeps as f64;
        let stride = [1usize, shape[0], shape[0] * shape[1]];
        let gas_velocity: Vec<[f64; 3]> =
            candidate.gas.cells().iter().map(|c| c.velocity).collect();
        for _ in 0..substeps {
            for (s, props) in candidate.species.iter().enumerate() {
                let mass = &candidate.mass[s];
                let mut next = mass.clone();
                for axis in 0..3 {
                    if shape[axis] < 2 && props.settling_velocity_m_s == 0. {
                        continue;
                    }
                    let face_area = volume / spacing[axis];
                    for cell in 0..mass.len() {
                        let index = (cell / stride[axis]) % shape[axis];
                        if index + 1 >= shape[axis] {
                            continue;
                        }
                        let neighbour = cell + stride[axis];
                        // Face velocity: gas average minus settling along up.
                        let face_v = 0.5
                            * (gas_velocity[cell][axis] + gas_velocity[neighbour][axis])
                            - props.settling_velocity_m_s * up[axis];
                        let flux = if face_v >= 0. {
                            face_v * mass[cell] / volume
                        } else {
                            face_v * mass[neighbour] / volume
                        } * face_area
                            * h;
                        next[cell] -= flux;
                        next[neighbour] += flux;
                        // Explicit diffusion.
                        if props.diffusivity_m2_s > 0. {
                            let d = props.diffusivity_m2_s * (mass[neighbour] - mass[cell])
                                / volume
                                / spacing[axis]
                                * face_area
                                * h;
                            next[cell] += d;
                            next[neighbour] -= d;
                        }
                    }
                    // Settling through the bottom face leaves the domain.
                    if props.settling_velocity_m_s > 0. && up[axis] != 0. {
                        for cell in 0..mass.len() {
                            let index = (cell / stride[axis]) % shape[axis];
                            let outflow_face = if up[axis] > 0. {
                                index == 0
                            } else {
                                index + 1 == shape[axis]
                            };
                            if !outflow_face {
                                continue;
                            }
                            let out = props.settling_velocity_m_s * up[axis].abs() * mass[cell]
                                / volume
                                * face_area
                                * h;
                            next[cell] -= out;
                            report.settled_kg[s] += out;
                        }
                    }
                }
                let scale = mass.iter().sum::<f64>().abs() + 1.;
                if next.iter().any(|m| !m.is_finite() || *m < -1e-9 * scale) {
                    return Err(Error::NumericalFailure);
                }
                for m in &mut next {
                    *m = m.max(0.);
                }
                candidate.mass[s] = next;
            }
            report.substeps += 1;
        }
        report.mass_after_kg = (0..candidate.species.len())
            .map(|s| candidate.total_mass_kg(s))
            .collect();
        *self = candidate;
        Ok(report)
    }

    /// Summed species extinction per cell, as a grid the renderer already understands.
    /// # Errors
    /// Numerical overflow.
    pub fn extinction_grid(&self) -> Result<DropletExtinctionGrid, Error> {
        let volume = self.cell_volume();
        let cells = self.gas.cells().len();
        let mut extinction = vec![0.; cells];
        for (s, props) in self.species.iter().enumerate() {
            for (e, m) in extinction.iter_mut().zip(&self.mass[s]) {
                *e += props.mass_extinction_m2_kg * m / volume;
            }
        }
        DropletExtinctionGrid::from_field(
            self.gas.origin(),
            self.gas.spacing(),
            self.gas.shape(),
            extinction,
        )
    }

    /// Mass-weighted mean albedo per cell (zero where empty).
    #[must_use]
    pub fn albedo_field(&self) -> Vec<f64> {
        let cells = self.gas.cells().len();
        let mut weighted = vec![0.; cells];
        let mut total = vec![0.; cells];
        for (s, props) in self.species.iter().enumerate() {
            for cell in 0..cells {
                let e = props.mass_extinction_m2_kg * self.mass[s][cell];
                weighted[cell] += props.albedo * e;
                total[cell] += e;
            }
        }
        weighted
            .iter()
            .zip(&total)
            .map(|(w, t)| if *t > 0. { w / t } else { 0. })
            .collect()
    }
}

#[cfg(test)]
mod tests;

/// Drying into the medium (plan step C2): a wet film cell exchanges solvent
/// vapor with the gas cell that contains `point`, so film mass loss equals
/// the gas cell's mass gain and the humidity field is the medium's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilmVaporExchange {
    /// Solvent mass moved from the film into the gas (negative for condensation).
    pub transferred_kg: f64,
    pub cell: usize,
}

impl ParticipatingMedium {
    /// Runs `ThermalFilmMixture::exchange_solution_vapor` against the gas
    /// cell containing `point`. Nothing changes when the point is outside the
    /// grid (`Ok(None)`); on any error the film, velocity and gas are unchanged.
    /// # Errors
    /// Nonfinite point, or the film vapor solver's own failures.
    pub fn exchange_film_vapor(
        &mut self,
        film: &mut crate::surface_film::ThermalFilmMixture,
        film_cell: usize,
        film_velocity: &mut [f64; 3],
        point: [f64; 3],
        model: &crate::liquid::SolutionVaporInterface,
        dt: f64,
        accuracy: crate::liquid::VaporExchangeAccuracy,
    ) -> Result<Option<FilmVaporExchange>, &'static str> {
        let Some(cell) = self
            .gas
            .cell_index(point)
            .map_err(|_| "invalid film vapor point")?
        else {
            return Ok(None);
        };
        let mut vapor = self.gas.cells()[cell];
        let mut candidate_film = film.clone();
        let mut candidate_velocity = *film_velocity;
        let transferred = candidate_film.exchange_solution_vapor(
            film_cell,
            &mut candidate_velocity,
            &mut vapor,
            model,
            dt,
            accuracy,
        )?;
        *film = candidate_film;
        *film_velocity = candidate_velocity;
        self.gas.cells_mut()[cell] = vapor;
        Ok(Some(FilmVaporExchange {
            transferred_kg: transferred,
            cell,
        }))
    }

    /// Atomically transfers airborne spray droplets from `liquid` whose volume-equivalent
    /// sphere radius is `<= max_droplet_radius_m` and whose position lies inside the
    /// medium's gas grid into `species` (e.g. `MediumSpecies::Mist`).
    /// When `liquid.droplet_population()` is configured, only particles marked as airborne
    /// droplets are eligible; otherwise all particles within the radius cutoff are eligible.
    /// # Errors
    /// Invalid species index, non-positive/non-finite radius cutoff, or liquid exchange error.
    pub fn absorb_spray_droplets(
        &mut self,
        species: usize,
        liquid: &mut crate::liquid::Liquid,
        max_droplet_radius_m: f64,
    ) -> Result<SprayMistAbsorptionReport, Error> {
        if species >= self.species.len()
            || !max_droplet_radius_m.is_finite()
            || max_droplet_radius_m <= 0.
        {
            return Err(Error::InvalidConfig);
        }
        let radii = liquid.equivalent_sphere_radii()?;
        let flags = liquid.droplet_population();
        let mut remove = Vec::new();
        let mut deposits = Vec::new();
        for (index, (particle, &radius)) in liquid.particles().iter().zip(&radii).enumerate() {
            let is_droplet = flags.is_none_or(|f| f[index]);
            if !is_droplet || radius > max_droplet_radius_m {
                continue;
            }
            if let Some(cell) = self.gas.cell_index(particle.position)? {
                remove.push(index);
                deposits.push((cell, particle.mass));
            }
        }
        if remove.is_empty() {
            let exchange = liquid.exchange_particles(&[], &[])?;
            return Ok(SprayMistAbsorptionReport {
                absorbed_count: 0,
                absorbed_mass_kg: 0.,
                exchange,
            });
        }
        let mut candidate_medium = self.clone();
        let mut absorbed_mass_kg = 0.;
        for &(cell, mass) in &deposits {
            candidate_medium.mass[species][cell] += mass;
            absorbed_mass_kg += mass;
        }
        if !absorbed_mass_kg.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let exchange = liquid.exchange_particles(&remove, &[])?;
        *self = candidate_medium;
        Ok(SprayMistAbsorptionReport {
            absorbed_count: remove.len(),
            absorbed_mass_kg,
            exchange,
        })
    }

    /// Total multiphase mixture density $\rho_{\text{mix}} = (m_{\text{gas}} + \sum_s m_s) / V_{\text{cell}}$
    /// in kg/m³ per grid cell (`плотность`).
    #[must_use]
    pub fn mixture_density_kg_m3(&self) -> Vec<f64> {
        let volume = self.cell_volume();
        self.gas
            .cells()
            .iter()
            .enumerate()
            .map(|(cell, g)| {
                let suspended: f64 = self.mass.iter().map(|s| s[cell]).sum();
                (g.mass + suspended) / volume
            })
            .collect()
    }

    /// Evaluates per-cell Richardson–Zaki hindered settling velocity
    /// $v_s(\phi) = v_{s,0} (1 - \phi)^n$ where $\phi = \text{clamp}(c_s / \rho_{\text{p}}, 0, 0.65)$
    /// is the local particulate volume fraction.
    pub fn hindered_settling_velocities_m_s(
        &self,
        species: usize,
        particle_density_kg_m3: f64,
        exponent_n: f64,
    ) -> Result<Vec<f64>, Error> {
        if species >= self.species.len()
            || !particle_density_kg_m3.is_finite()
            || particle_density_kg_m3 <= 0.
            || !exponent_n.is_finite()
            || !(2.0..=6.0).contains(&exponent_n)
        {
            return Err(Error::InvalidConfig);
        }
        let v0 = self.species[species].settling_velocity_m_s;
        let volume = self.cell_volume();
        Ok(self.mass[species]
            .iter()
            .map(|&m| {
                let conc = m / volume;
                let phi = (conc / particle_density_kg_m3).clamp(0.0, 0.65);
                v0 * (1.0 - phi).powf(exponent_n)
            })
            .collect())
    }

    /// Reversible Clausius–Clapeyron phase relaxation between carrier gas vapor and
    /// suspended `Fog` droplets (`туман`). Supersaturated cells condense excess gas
    /// mass into `config.fog_species`, while subsaturated cells evaporate fog droplets
    /// back into the gas cell, strictly conserving `gas_mass + fog_mass`.
    pub fn condense_or_evaporate_fog(
        &mut self,
        dt_s: f64,
        config: FogCondensationConfig,
    ) -> Result<FogCondensationReport, Error> {
        config.validate(self.species.len())?;
        if !dt_s.is_finite() || dt_s <= 0. {
            return Err(Error::InvalidTimeStep);
        }
        let fog_idx = config.fog_species;
        let volume = self.cell_volume();
        let before_gas: f64 = self.gas.cells().iter().map(|c| c.mass).sum();
        let before_fog: f64 = self.total_mass_kg(fog_idx);
        let total_before = before_gas + before_fog;

        let mut candidate = self.clone();
        let factor = 1.0 - (-config.relaxation_rate_s_inv * dt_s).exp();
        let mut net_condensed_fog_mass_kg = 0.0;

        for (cell_idx, gas_cell) in candidate.gas.cells_mut().iter_mut().enumerate() {
            let t = gas_cell.temperature;
            let t0 = config.reference_temperature_k;
            let rho_sat = config.saturation_density_at_ref_kg_m3
                * (t0 / t)
                * (config.clausius_clapeyron_exponent * (1.0 - t0 / t)).exp();
            if !rho_sat.is_finite() || rho_sat <= 0. {
                return Err(Error::NumericalFailure);
            }
            let m_sat = rho_sat * volume;
            let excess = gas_cell.mass - m_sat;
            let delta_fog = if excess > 0.0 {
                let max_removable =
                    (gas_cell.mass * (1.0 - config.min_gas_fraction_floor)).max(0.0);
                (excess * factor).min(max_removable)
            } else {
                let available_fog = candidate.mass[fog_idx][cell_idx];
                (excess * factor).max(-available_fog)
            };
            gas_cell.mass -= delta_fog;
            candidate.mass[fog_idx][cell_idx] =
                (candidate.mass[fog_idx][cell_idx] + delta_fog).max(0.0);
            net_condensed_fog_mass_kg += delta_fog;
        }

        let after_gas: f64 = candidate.gas.cells().iter().map(|c| c.mass).sum();
        let after_fog: f64 = candidate.total_mass_kg(fog_idx);
        let total_after = after_gas + after_fog;

        *self = candidate;
        Ok(FogCondensationReport {
            net_condensed_fog_mass_kg,
            total_gas_plus_fog_before_kg: total_before,
            total_gas_plus_fog_after_kg: total_after,
        })
    }

    /// Wet scavenging / washout (`водная взвесь` capturing `пыль` and `дым`):
    /// Airborne water mist/spray (`mist_species`) coagulates with and scrubs
    /// `target_species` (e.g. `Dust` and `Smoke`) at collection rate
    /// $E_c c_{\text{mist}}$, transferring captured particulate mass into
    /// `mist_species` so it settles rapidly while strictly conserving total suspended mass.
    pub fn scavenge_dust_and_smoke_by_mist(
        &mut self,
        mist_species: usize,
        target_species: &[usize],
        collection_efficiency_m3_kg_s: f64,
        dt_s: f64,
    ) -> Result<WetScavengingReport, Error> {
        if mist_species >= self.species.len()
            || target_species
                .iter()
                .any(|&s| s >= self.species.len() || s == mist_species)
            || !collection_efficiency_m3_kg_s.is_finite()
            || collection_efficiency_m3_kg_s < 0.
            || !dt_s.is_finite()
            || dt_s <= 0.
        {
            return Err(Error::InvalidConfig);
        }
        let volume = self.cell_volume();
        let cells = self.gas.cells().len();
        let mut candidate = self.clone();
        let mut scavenged_per_species = vec![0.0; self.species.len()];
        let mut total_scavenged_mass_kg = 0.0;

        for cell in 0..cells {
            let mist_conc = candidate.mass[mist_species][cell] / volume;
            if mist_conc <= 0.0 {
                continue;
            }
            let scrub_fraction = 1.0 - (-collection_efficiency_m3_kg_s * mist_conc * dt_s).exp();
            for &target in target_species {
                let captured = candidate.mass[target][cell] * scrub_fraction;
                if captured > 0.0 {
                    candidate.mass[target][cell] -= captured;
                    candidate.mass[mist_species][cell] += captured;
                    scavenged_per_species[target] += captured;
                    total_scavenged_mass_kg += captured;
                }
            }
        }

        *self = candidate;
        Ok(WetScavengingReport {
            scavenged_mass_per_species_kg: scavenged_per_species,
            total_scavenged_mass_kg,
        })
    }
}

/// Conservation report for [`ParticipatingMedium::absorb_spray_droplets`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprayMistAbsorptionReport {
    pub absorbed_count: usize,
    pub absorbed_mass_kg: f64,
    pub exchange: crate::liquid::ParticleExchange,
}

/// Clausius–Clapeyron phase condensation/evaporation parameters for `Fog` (`туман`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogCondensationConfig {
    pub fog_species: usize,
    pub saturation_density_at_ref_kg_m3: f64,
    pub reference_temperature_k: f64,
    pub clausius_clapeyron_exponent: f64,
    pub relaxation_rate_s_inv: f64,
    pub min_gas_fraction_floor: f64,
}

impl FogCondensationConfig {
    fn validate(&self, species_len: usize) -> Result<(), Error> {
        if self.fog_species >= species_len
            || !self.saturation_density_at_ref_kg_m3.is_finite()
            || self.saturation_density_at_ref_kg_m3 <= 0.
            || !self.reference_temperature_k.is_finite()
            || self.reference_temperature_k <= 0.
            || !self.clausius_clapeyron_exponent.is_finite()
            || self.clausius_clapeyron_exponent <= 0.
            || !self.relaxation_rate_s_inv.is_finite()
            || self.relaxation_rate_s_inv <= 0.
            || !self.min_gas_fraction_floor.is_finite()
            || !(0.05..=0.95).contains(&self.min_gas_fraction_floor)
        {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }
}

/// Mass conservation audit for [`ParticipatingMedium::condense_or_evaporate_fog`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogCondensationReport {
    pub net_condensed_fog_mass_kg: f64,
    pub total_gas_plus_fog_before_kg: f64,
    pub total_gas_plus_fog_after_kg: f64,
}

/// Mass conservation audit for [`ParticipatingMedium::scavenge_dust_and_smoke_by_mist`].
#[derive(Clone, Debug, PartialEq)]
pub struct WetScavengingReport {
    pub scavenged_mass_per_species_kg: Vec<f64>,
    pub total_scavenged_mass_kg: f64,
}
