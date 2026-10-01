//! Grey LTE plane-parallel gas heating. Two hemispheric streams use mu=1/2
//! and weight 2*pi, recovering flux pi*B for isotropic blackbody emission.
//! Radiation is quasi-static; gas heat capacity/opacity/geometry remain fixed.
use crate::{
    astrophysics_radiation::{Layer, trace},
    astrophysics_thermal::STEFAN_BOLTZMANN,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slab {
    pub thickness: f64,
    pub absorption: f64,
    pub temperature: f64,
    /// Per horizontal area, J m^-2 K^-1.
    pub heat_capacity: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// Bottom to top.
    pub slabs: Vec<Slab>,
    /// Cumulative net energy escaped through both boundaries, J/m².
    pub escaped_energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalOverflow,
    BudgetExceeded,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Rates {
    pub heating: Vec<f64>,
    /// Net outgoing boundary flux minus incident flux, W/m².
    pub escaping: f64,
}
impl Column {
    fn validate(&self) -> Result<(), Error> {
        if self.slabs.is_empty() || !self.escaped_energy.is_finite() {
            return Err(Error::InvalidInput);
        }
        for s in &self.slabs {
            if ![s.thickness, s.absorption, s.temperature, s.heat_capacity]
                .into_iter()
                .all(f64::is_finite)
                || s.thickness <= 0.0
                || s.absorption < 0.0
                || s.temperature < 0.0
                || s.heat_capacity <= 0.0
            {
                return Err(Error::InvalidInput);
            }
        }
        Ok(())
    }
    /// Internal energy per area, relative to zero K.
    /// # Errors
    /// Invalid state or overflowing energy.
    pub fn internal_energy(&self) -> Result<f64, Error> {
        self.validate()?;
        let e = self
            .slabs
            .iter()
            .map(|s| s.temperature * s.heat_capacity)
            .sum::<f64>();
        if e.is_finite() {
            Ok(e)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Boundary inputs are incident hemispheric intensities in W/m²/sr.
    /// # Errors
    /// Invalid state/boundary intensity or numerical overflow.
    pub fn rates(&self, bottom: f64, top: f64) -> Result<Rates, Error> {
        self.validate()?;
        if ![bottom, top].into_iter().all(f64::is_finite) || bottom < 0.0 || top < 0.0 {
            return Err(Error::InvalidInput);
        }
        let layers: Vec<_> = self
            .slabs
            .iter()
            .map(|s| Layer {
                length: 2.0 * s.thickness,
                absorption: s.absorption,
                temperature: s.temperature,
            })
            .collect();
        let up = trace(bottom, &layers, layers.len()).map_err(|_| Error::NumericalOverflow)?;
        let reverse: Vec<_> = layers.into_iter().rev().collect();
        let down = trace(top, &reverse, reverse.len()).map_err(|_| Error::NumericalOverflow)?;
        let heating: Vec<_> = up
            .deposited
            .iter()
            .zip(down.deposited.iter().rev())
            .map(|(a, b)| std::f64::consts::PI * (a + b))
            .collect();
        let escaping = std::f64::consts::PI * ((up.intensity - bottom) + (down.intensity - top));
        if !escaping.is_finite() || !heating.iter().all(|v| v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(Rates { heating, escaping })
    }
    /// Explicit adaptive heating with a thermal derivative bound and positive
    /// temperatures. Budget or numerical failure leaves the full column unchanged.
    /// # Errors
    /// Invalid inputs, exhausted budget, nonfinite or unresolvable evolution.
    pub fn step(
        &mut self,
        bottom: f64,
        top: f64,
        dt: f64,
        max_steps: usize,
    ) -> Result<usize, Error> {
        self.rates(bottom, top)?;
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::InvalidInput);
        }
        let mut next = self.clone();
        let mut remaining = dt;
        let mut count = 0;
        while remaining > 0.0 {
            if count >= max_steps {
                return Err(Error::BudgetExceeded);
            }
            let rates = next.rates(bottom, top)?;
            let boundary_temperature =
                (std::f64::consts::PI * bottom.max(top) / STEFAN_BOLTZMANN).powf(0.25);
            let hottest = next
                .slabs
                .iter()
                .map(|s| s.temperature)
                .fold(boundary_temperature, f64::max);
            let mut h = remaining;
            for (s, rate) in next.slabs.iter().zip(&rates.heating) {
                if s.absorption > 0.0 && hottest > 0.0 {
                    h = h.min(0.1 * s.heat_capacity / (8.0 * STEFAN_BOLTZMANN * hottest.powi(3)));
                }
                if *rate < 0.0 {
                    h = h.min(0.1 * s.heat_capacity * s.temperature / (-rate));
                }
            }
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            for (s, rate) in next.slabs.iter_mut().zip(rates.heating) {
                s.temperature += h * rate / s.heat_capacity;
                if !s.temperature.is_finite() || s.temperature < 0.0 {
                    return Err(Error::NumericalOverflow);
                }
            }
            next.escaped_energy += h * rates.escaping;
            if !next.escaped_energy.is_finite() {
                return Err(Error::NumericalOverflow);
            }
            remaining -= h;
            count += 1;
        }
        *self = next;
        Ok(count)
    }
}
