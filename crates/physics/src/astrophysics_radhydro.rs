//! One-dimensional grey LTE radiation hydrodynamics in SI units.
//! Euler half-step, quasi-static radiative heating, Euler half-step. Radiation
//! momentum and finite light travel time are neglected; nonrelativistic gas only.
use crate::{
    astrophysics_column::{Column, Slab},
    astrophysics_gas::Gas,
};
#[derive(Clone, Debug, PartialEq)]
pub struct RadiatingGas {
    pub gas: Gas,
    /// Constant specific heat at fixed volume, J/kg/K.
    pub specific_heat: f64,
    /// Grey mass absorption coefficient, m²/kg; absorption = kappa*rho.
    pub opacity: f64,
    /// Net escaped radiation J/m², signed for external irradiation.
    pub escaped_energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Gas(crate::astrophysics_gas::Error),
    Radiation(crate::astrophysics_column::Error),
    NumericalOverflow,
    Gravity(crate::astrophysics_gas_gravity::Error),
    BudgetExceeded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work {
    pub hydro_steps: usize,
    pub thermal_steps: usize,
}
impl RadiatingGas {
    /// Derive temperature/heat capacity/absorption from the current gas state.
    /// # Errors
    /// Invalid parameters, nonphysical gas state or numerical overflow.
    pub fn column(&self) -> Result<Column, Error> {
        self.gas.totals().map_err(Error::Gas)?;
        if ![self.specific_heat, self.opacity, self.escaped_energy]
            .into_iter()
            .all(f64::is_finite)
            || self.specific_heat <= 0.0
            || self.opacity < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let mut slabs = Vec::with_capacity(self.gas.cells.len());
        for c in &self.gas.cells {
            let internal = c.pressure(self.gas.gamma).map_err(Error::Gas)? / (self.gas.gamma - 1.0);
            let temperature = internal / c.density / self.specific_heat;
            let heat_capacity = c.density * self.specific_heat * self.gas.spacing;
            let absorption = self.opacity * c.density;
            if ![temperature, heat_capacity, absorption]
                .into_iter()
                .all(f64::is_finite)
                || temperature <= 0.0
                || heat_capacity <= 0.0
            {
                return Err(Error::NumericalOverflow);
            }
            slabs.push(Slab {
                thickness: self.gas.spacing,
                absorption,
                temperature,
                heat_capacity,
            });
        }
        Ok(Column {
            slabs,
            escaped_energy: self.escaped_energy,
        })
    }
    /// Strang ordering; component solvers remain first order. Caller must refine
    /// dt for splitting accuracy. Budgets bound each call; failure is atomic.
    /// Boundary radiation inputs are hemispheric intensities W/m²/sr.
    /// # Errors
    /// Invalid inputs, failed component evolution, budget exhaustion or overflow.
    pub fn step(
        &mut self,
        bottom: f64,
        top: f64,
        dt: f64,
        hydro_budget: usize,
        thermal_budget: usize,
    ) -> Result<Work, Error> {
        Ok(self
            .step_transport(bottom, top, dt, hydro_budget, thermal_budget)?
            .0)
    }
    fn step_transport(
        &mut self,
        bottom: f64,
        top: f64,
        dt: f64,
        hydro_budget: usize,
        thermal_budget: usize,
    ) -> Result<(Work, crate::astrophysics_gas::Transport), Error> {
        self.column()?;
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let first = next
            .gas
            .advance(dt / 2.0, hydro_budget)
            .map_err(Error::Gas)?;
        let mut column = next.column()?;
        let thermal_steps = column
            .step(bottom, top, dt, thermal_budget)
            .map_err(Error::Radiation)?;
        for (cell, slab) in next.gas.cells.iter_mut().zip(column.slabs) {
            let kinetic = 0.5 * cell.momentum * (cell.momentum / cell.density);
            cell.energy = kinetic + cell.density * next.specific_heat * slab.temperature;
            cell.pressure(next.gas.gamma).map_err(Error::Gas)?;
        }
        next.escaped_energy = column.escaped_energy;
        let second = next
            .gas
            .advance(dt / 2.0, hydro_budget - first.steps)
            .map_err(Error::Gas)?;
        next.column()?;
        *self = next;
        let steps = first.steps + second.steps;
        let mass = first
            .mass
            .iter()
            .zip(second.mass)
            .map(|(a, b)| a + b)
            .collect();
        let boundary = std::array::from_fn(|i| first.boundary[i] + second.boundary[i]);
        Ok((
            Work {
                hydro_steps: steps,
                thermal_steps,
            },
            crate::astrophysics_gas::Transport {
                steps,
                mass,
                boundary,
            },
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledBudget {
    pub max_step: f64,
    pub gravity_steps: usize,
    pub hydro_steps: usize,
    pub thermal_steps: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledWork {
    /// Net outgoing gas mass, kg/m².
    pub escaped_mass: f64,
    /// Net outgoing gas energy plus gravitational boundary exchange, J/m².
    /// Separate from cumulative radiation in `RadiatingGas::escaped_energy`.
    pub escaped_gas_energy: f64,
    pub gravity_steps: usize,
    pub hydro_steps: usize,
    pub thermal_steps: usize,
}
impl RadiatingGas {
    /// Gas + isolated sheet potential + cumulative net escaped radiation, J/m².
    /// Conserved to roundoff with reflecting walls, fixed G and mass.
    /// # Errors
    /// Invalid state, boundary geometry or overflow.
    pub fn total_energy_with_gravity(&self, g: f64) -> Result<f64, Error> {
        self.column()?;
        let potential = crate::astrophysics_gas_gravity::field(&self.gas, g)
            .map_err(Error::Gravity)?
            .potential_energy;
        let energy = self.gas.totals().map_err(Error::Gas)?[2] + potential + self.escaped_energy;
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Coupled gravity half-kick, radiation hydrodynamics, gravity half-kick.
    /// Budgets are shared over all outer substeps; every error rolls back all
    /// gas state and escaped radiation. Isolated planar gravity, not a sphere.
    /// # Errors
    /// Invalid inputs, component failures or exhausted work budgets.
    pub fn step_with_gravity(
        &mut self,
        g: f64,
        bottom: f64,
        top: f64,
        dt: f64,
        budget: CoupledBudget,
    ) -> Result<CoupledWork, Error> {
        self.total_energy_with_gravity(g)?;
        self.column()?
            .rates(bottom, top)
            .map_err(Error::Radiation)?;
        if !dt.is_finite() || dt < 0.0 || !budget.max_step.is_finite() || budget.max_step <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut work = CoupledWork {
            escaped_mass: 0.0,
            escaped_gas_energy: 0.0,
            gravity_steps: 0,
            hydro_steps: 0,
            thermal_steps: 0,
        };
        while remaining > 0.0 {
            if work.gravity_steps >= budget.gravity_steps {
                return Err(Error::BudgetExceeded);
            }
            let peak = next.gas.cells.iter().map(|c| c.density).fold(0.0, f64::max);
            let h = remaining
                .min(budget.max_step)
                .min(0.1 / (4.0 * std::f64::consts::PI * g * peak).sqrt());
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let before = next.gas.clone();
            let first = crate::astrophysics_gas_gravity::kick(&mut next.gas, g, h / 2.0)
                .map_err(Error::Gravity)?;
            let (subwork, flux) = next.step_transport(
                bottom,
                top,
                h,
                budget.hydro_steps - work.hydro_steps,
                budget.thermal_steps - work.thermal_steps,
            )?;
            work.hydro_steps += subwork.hydro_steps;
            work.thermal_steps += subwork.thermal_steps;
            let second = crate::astrophysics_gas_gravity::kick(&mut next.gas, g, h / 2.0)
                .map_err(Error::Gravity)?;
            work.escaped_mass += flux.boundary[0];
            work.escaped_gas_energy += flux.boundary[2]
                + crate::astrophysics_gas_gravity::transport_work_flux(
                    &before,
                    &mut next.gas,
                    g,
                    &first,
                    &second,
                    &flux.mass,
                )
                .map_err(Error::Gravity)?;
            remaining -= h;
            work.gravity_steps += 1;
        }
        next.total_energy_with_gravity(g)?;
        if !work.escaped_mass.is_finite() || !work.escaped_gas_energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        Ok(work)
    }
}
