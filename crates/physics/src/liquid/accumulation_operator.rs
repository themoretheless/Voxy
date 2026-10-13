//! Original ordered accumulation of pair forces into particle accelerations.
use super::{Error, LiquidPairForceResult, Particle, finite, positive};
#[derive(Debug)]
pub struct LiquidAccumulationOperator<'a> {
    pub particles: &'a [Particle],
    pub pairs: &'a [(usize, usize)],
    pub forces: &'a [LiquidPairForceResult],
    pub gravity: [f64; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidAccumulationResult {
    pub accelerations: Vec<[f64; 3]>,
    pub viscous_rates: Vec<f64>,
}
impl LiquidAccumulationOperator<'_> {
    pub fn evaluate_native(&self) -> Result<LiquidAccumulationResult, Error> {
        if self.pairs.len() != self.forces.len()
            || !finite(self.gravity)
            || self.particles.iter().any(|p| !positive(p.mass))
            || self.forces.iter().any(|p| {
                !finite(p.force) || !p.viscous_conductance.is_finite() || p.viscous_conductance < 0.
            })
        {
            return Err(Error::NumericalFailure);
        }
        let mut seen = std::collections::HashSet::new();
        if self
            .pairs
            .iter()
            .any(|&(i, j)| i >= j || j >= self.particles.len() || !seen.insert((i, j)))
        {
            return Err(Error::InvalidParticle);
        }
        let mut accelerations = vec![self.gravity; self.particles.len()];
        let mut viscous_rates = vec![0.; self.particles.len()];
        for (&(i, j), result) in self.pairs.iter().zip(self.forces) {
            let a = self.particles[i];
            let b = self.particles[j];
            viscous_rates[i] += result.viscous_conductance / a.mass;
            viscous_rates[j] += result.viscous_conductance / b.mass;
            let inv_a = 1. / a.mass;
            let inv_b = 1. / b.mass;
            for axis in 0..3 {
                accelerations[i][axis] += result.force[axis] * inv_a;
                accelerations[j][axis] -= result.force[axis] * inv_b;
            }
        }
        if accelerations.iter().any(|&a| !finite(a))
            || viscous_rates.iter().any(|&v| !v.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(LiquidAccumulationResult {
            accelerations,
            viscous_rates,
        })
    }
    pub fn admit(
        &self,
        candidate: &LiquidAccumulationResult,
        absolute: f64,
        relative: f64,
    ) -> Result<(), Error> {
        if candidate.accelerations.len() != self.particles.len()
            || candidate.viscous_rates.len() != self.particles.len()
            || !absolute.is_finite()
            || absolute < 0.
            || !relative.is_finite()
            || relative < 0.
        {
            return Err(Error::InvalidConfig);
        }
        if candidate.viscous_rates.iter().any(|&v| v < 0.) {
            return Err(Error::NumericalFailure);
        }
        let reference = self.evaluate_native()?;
        for (&a, &b) in candidate
            .accelerations
            .iter()
            .flatten()
            .chain(&candidate.viscous_rates)
            .zip(
                reference
                    .accelerations
                    .iter()
                    .flatten()
                    .chain(&reference.viscous_rates),
            )
        {
            let error = (a - b).abs();
            if !a.is_finite()
                || !error.is_finite()
                || (error > absolute && (b == 0. || error / b.abs() > relative))
            {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}
