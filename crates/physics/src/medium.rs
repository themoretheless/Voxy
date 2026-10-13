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
            report.mass_after_kg = report.mass_before_kg.clone();
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
}
