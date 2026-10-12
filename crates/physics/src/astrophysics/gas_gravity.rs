//! Isolated plane-parallel self-gravity: phi''=4*pi*G*rho.
//! Infinite horizontal sheets, not spherical gravity or periodic cosmology.
//! Field and potential integrals are exact for piecewise uniform grid cells.
use crate::astrophysics_gas::{Boundary, Gas};
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub acceleration: Vec<f64>,
    /// Cell-averaged potential, derivative of the discrete potential energy.
    pub potential: Vec<f64>,
    /// Potential energy per area, kernel phi=2*pi*G*integral rho*|x-x'| dx'.
    /// Positive gauge; no potential zero at infinity for an infinite sheet.
    pub potential_energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Gas(crate::astrophysics_gas::Error),
    NumericalOverflow,
    BudgetExceeded,
}
/// Compute cell-averaged acceleration and pair/self potential in O(cell count).
/// # Errors
/// Invalid gas/G, periodic boundaries (incompatible isolated Green function), overflow.
pub fn field(gas: &Gas, g: f64) -> Result<Field, Error> {
    let total = gas.totals().map_err(Error::Gas)?[0];
    if !g.is_finite() || g <= 0.0 || gas.boundary == Boundary::Periodic {
        return Err(Error::InvalidInput);
    }
    let mut position = gas.spacing / 2.0;
    let mut total_moment = 0.0;
    for cell in &gas.cells {
        total_moment += cell.density * gas.spacing * position;
        position += gas.spacing;
    }
    let mut potentials = Vec::with_capacity(gas.cells.len());
    let factor = 2.0 * std::f64::consts::PI * g;
    let mut left_mass = 0.0;
    let mut left_moment = 0.0;
    let mut x = gas.spacing / 2.0;
    let mut potential = 0.0;
    let mut acceleration = Vec::with_capacity(gas.cells.len());
    for c in &gas.cells {
        let mass = c.density * gas.spacing;
        let right_mass = total - left_mass - mass;
        let right_moment = total_moment - left_moment - mass * x;
        potentials.push(
            factor
                * (x * left_mass - left_moment + right_moment - x * right_mass
                    + mass * gas.spacing / 3.0),
        );
        acceleration.push(factor * (total - 2.0 * left_mass - mass));
        potential += factor * mass * (x * left_mass - left_moment)
            + factor * mass * mass * gas.spacing / 6.0;
        left_mass += mass;
        left_moment += mass * x;
        x += gas.spacing;
    }
    if !potential.is_finite()
        || !acceleration.iter().all(|a| a.is_finite())
        || !potentials.iter().all(|a| a.is_finite())
    {
        return Err(Error::NumericalOverflow);
    }
    Ok(Field {
        acceleration,
        potential: potentials,
        potential_energy: potential,
    })
}
pub(crate) fn kick(gas: &mut Gas, g: f64, h: f64) -> Result<Vec<f64>, Error> {
    let mut work = Vec::with_capacity(gas.cells.len());
    let force = field(gas, g)?;
    for (cell, a) in gas.cells.iter_mut().zip(force.acceleration) {
        let old_energy = cell.energy;
        let internal = cell.pressure(gas.gamma).map_err(Error::Gas)? / (gas.gamma - 1.0);
        cell.momentum += h * cell.density * a;
        cell.energy = internal + 0.5 * cell.momentum * (cell.momentum / cell.density);
        cell.pressure(gas.gamma).map_err(Error::Gas)?;
        work.push(cell.energy - old_energy);
    }
    Ok(work)
}
#[derive(Clone, Debug, PartialEq)]
pub struct SelfGravitatingGas {
    pub gas: Gas,
    pub g: f64,
}
impl SelfGravitatingGas {
    /// Kinetic+internal+sheet potential energy per area.
    /// # Errors
    /// Invalid state or overflow.
    pub fn energy(&self) -> Result<f64, Error> {
        let value =
            self.gas.totals().map_err(Error::Gas)?[2] + field(&self.gas, self.g)?.potential_energy;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Kick–Euler drift–kick; substeps also bounded by local gravitational time.
    /// Hydro work budget is shared across substeps. Entire failure is atomic.
    /// Uses conservative mass-transport work, including open-boundary potential
    /// exchange. Spatial accuracy still requires mesh/time refinement.
    /// # Errors
    /// Invalid input, numerical failure or exhausted gravity/hydro budget.
    pub fn step(
        &mut self,
        dt: f64,
        max_step: f64,
        gravity_budget: usize,
        hydro_budget: usize,
    ) -> Result<usize, Error> {
        Ok(self
            .advance(dt, max_step, gravity_budget, hydro_budget)?
            .gravity_steps)
    }
    /// Advance with signed mass and gas+gravitational energy leaving boundaries.
    /// Add returned energy to remaining `energy()` to audit conservation.
    /// # Errors
    /// Invalid inputs, component errors or budget exhaustion; all state rolls back.
    pub fn advance(
        &mut self,
        dt: f64,
        max_step: f64,
        gravity_budget: usize,
        hydro_budget: usize,
    ) -> Result<Exchange, Error> {
        field(&self.gas, self.g)?;
        if !dt.is_finite() || dt < 0.0 || !max_step.is_finite() || max_step <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut count = 0;
        let mut hydro_work = 0;
        let mut escaped_mass = 0.0;
        let mut escaped_energy = 0.0;
        while remaining > 0.0 {
            if count >= gravity_budget {
                return Err(Error::BudgetExceeded);
            }
            let peak = next.gas.cells.iter().map(|c| c.density).fold(0.0, f64::max);
            let h = remaining
                .min(max_step)
                .min(0.1 / (4.0 * std::f64::consts::PI * next.g * peak).sqrt());
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let before = next.gas.clone();
            let first = kick(&mut next.gas, next.g, h / 2.0)?;
            let flux = next
                .gas
                .advance(h, hydro_budget - hydro_work)
                .map_err(Error::Gas)?;
            hydro_work += flux.steps;
            escaped_mass += flux.boundary[0];
            let second = kick(&mut next.gas, next.g, h / 2.0)?;
            escaped_energy += flux.boundary[2]
                + transport_work_flux(&before, &mut next.gas, next.g, &first, &second, &flux.mass)?;
            remaining -= h;
            count += 1;
        }
        next.energy()?;
        if !escaped_mass.is_finite() || !escaped_energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        Ok(Exchange {
            gravity_steps: count,
            hydro_steps: hydro_work,
            escaped_mass,
            escaped_energy,
        })
    }
}

/// Conservative work from actual integrated interface mass transport; returns
/// net outgoing gravitational energy using the averaged discrete cell potential.
pub(crate) fn transport_work_flux(
    before: &Gas,
    after: &mut Gas,
    g: f64,
    first: &[f64],
    second: &[f64],
    mass: &[f64],
) -> Result<f64, Error> {
    let old = field(before, g)?.potential;
    let new = field(after, g)?.potential;
    let potential: Vec<_> = old.iter().zip(new).map(|(a, b)| a.midpoint(b)).collect();
    let mut work = vec![0.0; after.cells.len()];
    for i in 0..after.cells.len() - 1 {
        let energy = -mass[i + 1] * (potential[i + 1] - potential[i]);
        work[i] += energy / 2.0;
        work[i + 1] += energy / 2.0;
    }
    for (i, cell) in after.cells.iter_mut().enumerate() {
        cell.energy += work[i] / after.spacing - first[i] - second[i];
        cell.pressure(after.gamma).map_err(Error::Gas)?;
    }
    let last = after.cells.len() - 1;
    let escaped = potential[last] * mass[last + 1] - potential[0] * mass[0];
    if escaped.is_finite() {
        Ok(escaped)
    } else {
        Err(Error::NumericalOverflow)
    }
}

/// Signed cumulative boundary exchange for a single successful call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exchange {
    pub gravity_steps: usize,
    pub hydro_steps: usize,
    pub escaped_mass: f64,
    pub escaped_energy: f64,
}
