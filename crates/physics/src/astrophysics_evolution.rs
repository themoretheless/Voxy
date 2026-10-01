//! Coupled radial self-gravitating radiation gas. This is dynamical thermal
//! evolution, not a calibrated nuclear stellar-evolution/composition model.
use crate::{astrophysics_spherical::Sphere, astrophysics_spherical_radiation::Heating};
#[derive(Clone, Debug, PartialEq)]
pub struct RadiatingSphere {
    pub sphere: Sphere,
    /// SI J/kg/K, fixed ideal-gas heat capacity.
    pub specific_heat: f64,
    /// Grey mass absorption m²/kg.
    pub opacity: f64,
    /// Isotropic external bolometric intensity W/m²/sr.
    pub ambient: f64,
    pub escaped_radiation: f64,
    pub escaped_gas_energy: f64,
    pub escaped_mass: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budget {
    pub max_step: f64,
    pub outer_steps: usize,
    pub hydro_steps: usize,
    pub thermal_steps: usize,
    pub ray_segments: usize,
    pub rays_per_annulus: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work {
    pub outer_steps: usize,
    pub hydro_steps: usize,
    pub thermal_steps: usize,
    pub ray_segments: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    BudgetExceeded,
    NumericalOverflow,
    Hydro(crate::astrophysics_spherical::Error),
    Radiation(crate::astrophysics_spherical_radiation::Error),
}
impl RadiatingSphere {
    fn validate(&self) -> Result<(), Error> {
        self.sphere.energy().map_err(Error::Hydro)?;
        if ![
            self.specific_heat,
            self.opacity,
            self.ambient,
            self.escaped_radiation,
            self.escaped_gas_energy,
            self.escaped_mass,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.specific_heat <= 0.0
            || self.opacity < 0.0
            || self.ambient < 0.0
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    /// Gas + binding energy + signed cumulative radiation and gas boundary exchange.
    /// # Errors
    /// Invalid state or overflowing diagnostics.
    pub fn energy(&self) -> Result<f64, Error> {
        self.validate()?;
        let energy = self.sphere.energy().map_err(Error::Hydro)?
            + self.escaped_radiation
            + self.escaped_gas_energy;
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Hydro/gravity half-step → radiation heating → hydro/gravity half-step.
    /// Component solvers remain first order; `max_step` controls splitting accuracy.
    /// All work budgets are global over this call. Errors roll back gas and ledgers.
    /// # Errors
    /// Invalid inputs, component failure, budget exhaustion or overflow.
    pub fn step(&mut self, dt: f64, budget: Budget) -> Result<Work, Error> {
        self.step_eos(dt, budget, None)
    }
    /// Coupled dynamics and radiation at fixed ionized composition. Pressure,
    /// sound, temperature and stored internal energy use the same gas+radiation EOS.
    /// The legacy `specific_heat` field is ignored in this mode (but remains valid).
    /// # Errors
    /// Same complete rollback and work-budget failures as `step`, plus EOS failures.
    pub fn step_ionized(
        &mut self,
        dt: f64,
        budget: Budget,
        mixture: crate::astrophysics_eos::Mixture,
    ) -> Result<Work, Error> {
        self.step_eos(dt, budget, Some(mixture))
    }
    fn step_eos(
        &mut self,
        dt: f64,
        budget: Budget,
        mixture: Option<crate::astrophysics_eos::Mixture>,
    ) -> Result<Work, Error> {
        self.validate()?;
        if !dt.is_finite()
            || dt < 0.0
            || !budget.max_step.is_finite()
            || budget.max_step <= 0.0
            || budget.rays_per_annulus == 0
        {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut work = Work {
            outer_steps: 0,
            hydro_steps: 0,
            thermal_steps: 0,
            ray_segments: 0,
        };
        while remaining > 0.0 {
            if work.outer_steps >= budget.outer_steps {
                return Err(Error::BudgetExceeded);
            }
            let h = remaining.min(budget.max_step);
            if remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let first = next
                .sphere
                .step_eos(
                    h / 2.0,
                    h / 2.0,
                    budget.hydro_steps - work.hydro_steps,
                    mixture,
                )
                .map_err(Error::Hydro)?;
            work.hydro_steps += first.steps;
            next.escaped_mass += first.escaped_mass;
            next.escaped_gas_energy += first.escaped_energy;
            let thermal = next
                .sphere
                .radiate_eos(
                    h,
                    Heating {
                        specific_heat: next.specific_heat,
                        opacity: next.opacity,
                        ambient: next.ambient,
                        rays_per_annulus: budget.rays_per_annulus,
                        max_segments: budget.ray_segments - work.ray_segments,
                        max_step: h,
                        max_steps: budget.thermal_steps - work.thermal_steps,
                    },
                    mixture,
                )
                .map_err(Error::Radiation)?;
            work.thermal_steps += thermal.steps;
            work.ray_segments += thermal.segments;
            next.escaped_radiation += thermal.escaped_energy;
            let second = next
                .sphere
                .step_eos(
                    h / 2.0,
                    h / 2.0,
                    budget.hydro_steps - work.hydro_steps,
                    mixture,
                )
                .map_err(Error::Hydro)?;
            work.hydro_steps += second.steps;
            next.escaped_mass += second.escaped_mass;
            next.escaped_gas_energy += second.escaped_energy;
            remaining -= h;
            work.outer_steps += 1;
        }
        next.energy()?;
        *self = next;
        Ok(work)
    }
}
