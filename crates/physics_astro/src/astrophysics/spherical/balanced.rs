//! Equilibrium-preserving spherical hydrostatic reference and perturbation transport.
use super::{
    Boundary, Cell, CompositionExterior, Error, Exchange, FOUR_PI, HydrostaticPressures, Sphere,
    thermodynamics, transport_composition,
};

/// Immutable hydrostatic reference for perturbation reconstruction.
#[derive(Clone, Debug)]
pub struct HydrostaticReference {
    sphere: Sphere,
    pressures: HydrostaticPressures,
    cell_pressures: Vec<f64>,
    nuclei: Vec<crate::astrophysics::nuclear::Nucleus>,
    exterior_pressure: Option<f64>,
}

impl Sphere {
    /// Initialize a confined hydrostatic sphere and retain a consistent reference.
    /// This first equilibrium-preserving path requires a reflecting outer wall.
    /// # Errors
    /// Invalid boundary, composition, pressure, geometry or EOS.
    pub fn initialize_balanced(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        surface_pressure: f64,
    ) -> Result<HydrostaticReference, Error> {
        self.initialize_balanced_impl(fractions, network, surface_pressure, None)
    }
    /// Initialize an open equilibrium with a stationary exterior matching surface pressure.
    /// # Errors
    /// Invalid/mismatched reservoir, geometry, composition or EOS.
    pub fn initialize_balanced_exterior(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        surface_pressure: f64,
        exterior: &CompositionExterior,
    ) -> Result<HydrostaticReference, Error> {
        self.initialize_balanced_impl(fractions, network, surface_pressure, Some(exterior))
    }
    fn initialize_balanced_impl(
        &mut self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        surface_pressure: f64,
        exterior: Option<&CompositionExterior>,
    ) -> Result<HydrostaticReference, Error> {
        if (self.outer == Boundary::Reflecting) != exterior.is_none()
            || self.outer == Boundary::Periodic
        {
            return Err(Error::InvalidInput);
        }
        let exterior_pressure = exterior
            .map(|e| {
                let pressure = thermodynamics(
                    e.cell,
                    self.gamma,
                    Some(network.mixture(&e.fractions).map_err(Error::Nuclear)?),
                )?
                .0;
                if e.cell.momentum != 0.0 || (pressure / surface_pressure - 1.0).abs() > 1e-12 {
                    return Err(Error::InvalidInput);
                }
                Ok(pressure)
            })
            .transpose()?;
        let pressures = self.hydrostatic_pressures(surface_pressure)?;
        let mut next = self.clone();
        next.initialize_hydrostatic(fractions, network, surface_pressure)?;
        let cell_pressures = next
            .cells
            .iter()
            .zip(fractions)
            .map(|(c, row)| {
                thermodynamics(
                    *c,
                    next.gamma,
                    Some(network.mixture(row).map_err(Error::Nuclear)?),
                )
                .map(|t| t.0)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reference = HydrostaticReference {
            sphere: next.clone(),
            pressures,
            cell_pressures,
            nuclei: network.nuclei.clone(),
            exterior_pressure,
        };
        *self = next;
        Ok(reference)
    }
    /// Evolve perturbations with reference-pressure reconstruction and a matching
    /// momentum source. Mass-flux gravity work conserves gas plus binding energy.
    /// State and composition roll back together on every failure.
    /// # Errors
    /// Incompatible reference, invalid reconstruction/state or exhausted budget.
    // Atomic reconstruction/transport/energy ledger.
    pub fn step_composition_balanced(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
    ) -> Result<Exchange, Error> {
        self.step_composition_balanced_impl(
            fractions, network, dt, max_step, budget, reference, None,
        )
        .map(|r| r.0)
    }
    /// Balanced open transport with signed gas/gravity and species boundary ledgers.
    /// # Errors
    /// Invalid/mismatched reference, reservoir, reconstruction or work budget.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_composition_balanced_exterior(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
        exterior: &CompositionExterior,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        self.step_composition_balanced_impl(
            fractions,
            network,
            dt,
            max_step,
            budget,
            reference,
            Some(exterior),
        )
    }
    #[expect(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn step_composition_balanced_impl(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        dt: f64,
        max_step: f64,
        budget: usize,
        reference: &HydrostaticReference,
        exterior: Option<&CompositionExterior>,
    ) -> Result<(Exchange, Vec<f64>), Error> {
        let base = &reference.sphere;
        if self.outer != base.outer
            || exterior.is_some() != reference.exterior_pressure.is_some()
            || self.cells.len() != base.cells.len()
            || self.spacing != base.spacing
            || self.g != base.g
            || self.gamma != base.gamma
            || network.nuclei != reference.nuclei
            || fractions.len() != self.cells.len()
            || !dt.is_finite()
            || dt < 0.0
            || !max_step.is_finite()
            || max_step <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        if let Some(e) = exterior {
            thermodynamics(
                e.cell,
                self.gamma,
                Some(network.mixture(&e.fractions).map_err(Error::Nuclear)?),
            )?;
        }
        let geometry = self.geometry()?;
        let base_field = base.field()?;
        let mut next = self.clone();
        let mut rows = fractions.to_vec();
        for (cell, row) in next.cells.iter().zip(&rows) {
            thermodynamics(
                *cell,
                next.gamma,
                Some(network.mixture(row).map_err(Error::Nuclear)?),
            )?;
        }
        let mut remaining = dt;
        let mut report = Exchange {
            steps: 0,
            escaped_mass: 0.0,
            escaped_energy: 0.0,
        };
        let mut escaped = vec![0.0; network.nuclei.len()];
        while remaining > 0.0 {
            if report.steps >= budget {
                return Err(Error::BudgetExceeded);
            }
            let n = next.cells.len();
            let old_field = next.field()?;
            let mut pressure_delta = Vec::with_capacity(n);
            let mut states = Vec::with_capacity(n);
            let mut speed = 0.0_f64;
            let mut peak = 0.0_f64;
            for (i, (cell, row)) in next.cells.iter().zip(&rows).enumerate() {
                let mixture = network.mixture(row).map_err(Error::Nuclear)?;
                let pressure = thermodynamics(*cell, next.gamma, Some(mixture))?.0;
                let delta = pressure - reference.cell_pressures[i];
                pressure_delta.push(delta);
                let mut faces = Vec::with_capacity(2);
                for face in [i, i + 1] {
                    let p = reference.pressures.faces[face] + delta;
                    let t = mixture
                        .temperature_from_pressure(cell.density, p)
                        .map_err(Error::Eos)?;
                    let thermal = mixture.at(cell.density, t).map_err(Error::Eos)?;
                    let velocity = cell.momentum / cell.density;
                    let reconstructed = Cell {
                        density: cell.density,
                        momentum: cell.momentum,
                        energy: thermal.internal_energy_density
                            + 0.5 * cell.density * velocity * velocity,
                    };
                    let thermo = thermodynamics(reconstructed, next.gamma, Some(mixture))?;
                    speed = speed.max(velocity.abs() + thermo.1);
                    faces.push((reconstructed, (p, thermo.1)));
                }
                states.push(faces);
                peak = peak.max(cell.density);
            }
            let mut flux = vec![[0.0; 3]; n + 1];
            for i in 1..n {
                flux[i] = crate::astrophysics::gas::contact_flux_thermal(
                    states[i - 1][1].0,
                    states[i][0].0,
                    states[i - 1][1].1,
                    states[i][0].1,
                )
                .map_err(Error::Gas)?;
            }
            let (last, thermo) = states[n - 1][1];
            if let Some(exterior) = exterior {
                let mixture = network
                    .mixture(&exterior.fractions)
                    .map_err(Error::Nuclear)?;
                let external = thermodynamics(exterior.cell, next.gamma, Some(mixture))?;
                let pressure = reference.pressures.faces[n]
                    + (external.0 - reference.exterior_pressure.ok_or(Error::InvalidInput)?);
                let t = mixture
                    .temperature_from_pressure(exterior.cell.density, pressure)
                    .map_err(Error::Eos)?;
                let velocity = exterior.cell.momentum / exterior.cell.density;
                let outside = Cell {
                    energy: mixture
                        .at(exterior.cell.density, t)
                        .map_err(Error::Eos)?
                        .internal_energy_density
                        + 0.5 * exterior.cell.density * velocity * velocity,
                    ..exterior.cell
                };
                let sound = thermodynamics(outside, next.gamma, Some(mixture))?.1;
                speed = speed.max(velocity.abs() + sound);
                flux[n] = crate::astrophysics::gas::contact_flux_thermal(
                    last,
                    outside,
                    thermo,
                    (pressure, sound),
                )
                .map_err(Error::Gas)?;
            } else {
                let reflected = Cell {
                    momentum: -last.momentum,
                    ..last
                };
                flux[n] =
                    crate::astrophysics::gas::contact_flux_thermal(last, reflected, thermo, thermo)
                        .map_err(Error::Gas)?;
                flux[n][0] = 0.0;
                flux[n][2] = 0.0;
            }
            let h = remaining
                .min(max_step)
                .min(0.2 * next.spacing / speed)
                .min(if next.g == 0.0 {
                    f64::INFINITY
                } else {
                    0.1 / (FOUR_PI * next.g * peak).sqrt()
                });
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let old_density: Vec<_> = next.cells.iter().map(|c| c.density).collect();
            let mut mass = vec![0.0; n + 1];
            for (i, (cell, geo)) in next.cells.iter_mut().zip(&geometry).enumerate() {
                let q = [cell.density, cell.momentum, cell.energy];
                let divergence: [f64; 3] = std::array::from_fn(|j| {
                    geo.right_area * flux[i + 1][j] - geo.left_area * flux[i][j]
                });
                let reference_force = geo.right_area * reference.pressures.faces[i + 1]
                    - geo.left_area * reference.pressures.faces[i];
                let source_difference = pressure_delta[i] * (geo.right_area - geo.left_area)
                    + geo.volume
                        * (cell.density * old_field.acceleration[i]
                            - base.cells[i].density * base_field.acceleration[i]);
                cell.density = q[0] - h * divergence[0] / geo.volume;
                cell.momentum =
                    q[1] + h * (reference_force - divergence[1] + source_difference) / geo.volume;
                cell.energy = q[2] - h * divergence[2] / geo.volume;
                cell.pressure(next.gamma).map_err(Error::Gas)?;
                mass[i + 1] = h * geo.right_area * flux[i + 1][0];
            }
            transport_composition(
                &mut rows,
                &old_density,
                &next.cells,
                &geometry,
                &mass,
                &mut escaped,
                exterior.map(|e| e.fractions.as_slice()),
            )?;
            let new_field = next.field()?;
            let potential: Vec<_> = old_field
                .potential
                .iter()
                .zip(new_field.potential)
                .map(|(a, b)| a.midpoint(b))
                .collect();
            for i in 0..n - 1 {
                let work = -mass[i + 1] * (potential[i + 1] - potential[i]) / 2.0;
                next.cells[i].energy += work / geometry[i].volume;
                next.cells[i + 1].energy += work / geometry[i + 1].volume;
            }
            for (cell, row) in next.cells.iter().zip(&rows) {
                thermodynamics(
                    *cell,
                    next.gamma,
                    Some(network.mixture(row).map_err(Error::Nuclear)?),
                )?;
            }
            report.escaped_mass += mass[n];
            report.escaped_energy +=
                h * geometry[n - 1].right_area * flux[n][2] + mass[n] * potential[n - 1];
            remaining -= h;
            report.steps += 1;
        }
        next.energy()?;
        if ![report.escaped_mass, report.escaped_energy]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        fractions.clone_from_slice(&rows);
        Ok((report, escaped))
    }
}
