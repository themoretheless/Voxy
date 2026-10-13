//! Original color-field normal input and whole-vector admission.
use super::{Error, Particle, finite, positive};
#[derive(Debug)]
pub struct LiquidNormalOperator<'a> {
    pub particles: &'a [Particle],
    pub pairs: &'a [(usize, usize)],
    pub densities: &'a [f64],
    pub smoothing_radius: f64,
}
impl LiquidNormalOperator<'_> {
    pub fn evaluate_native(&self) -> Result<Vec<[f64; 3]>, Error> {
        if !positive(self.smoothing_radius)
            || self.particles.len() != self.densities.len()
            || self.densities.iter().any(|&x| !positive(x))
            || self
                .particles
                .iter()
                .any(|p| !finite(p.position) || !positive(p.mass))
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
        let result = super::color_normals(
            self.particles,
            self.pairs,
            self.densities,
            self.smoothing_radius,
        );
        if result.iter().any(|&x| !finite(x)) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    pub fn admit(&self, candidate: &[[f64; 3]], absolute: f64, relative: f64) -> Result<(), Error> {
        if candidate.len() != self.particles.len()
            || !absolute.is_finite()
            || absolute < 0.
            || !relative.is_finite()
            || relative < 0.
        {
            return Err(Error::InvalidConfig);
        }
        for (actual, expected) in candidate.iter().zip(self.evaluate_native()?) {
            for (&a, b) in actual.iter().zip(expected) {
                let error = (a - b).abs();
                if !a.is_finite()
                    || !error.is_finite()
                    || (error > absolute && (b == 0. || error / b.abs() > relative))
                {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        Ok(())
    }
}
