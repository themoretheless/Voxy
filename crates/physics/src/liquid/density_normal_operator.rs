//! Original density including wall/images followed by color-field normals.
use super::{Error, LiquidDensityOperator, LiquidNormalOperator, positive};
#[derive(Debug)]
pub struct LiquidDensityNormalOperator<'a> {
    pub fluid: LiquidDensityOperator<'a>,
    pub pressure_laws: Option<&'a [super::LiquidPressureLaw]>,
    /// Original ordered wall/image additions, grouped only by receiving particle.
    pub support: &'a [(usize, f64)],
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidDensityNormalResult {
    pub densities: Vec<f64>,
    pub normals: Vec<[f64; 3]>,
    pub pressures: Option<Vec<f64>>,
}
impl LiquidDensityNormalOperator<'_> {
    pub fn evaluate_native(&self) -> Result<LiquidDensityNormalResult, Error> {
        let mut densities = self.fluid.evaluate_native()?;
        for &(i, x) in self.support {
            if i >= densities.len() || !x.is_finite() || x < 0. {
                return Err(Error::InvalidConfig);
            }
            densities[i] += x;
        }
        if densities.iter().any(|&x| !positive(x)) {
            return Err(Error::NumericalFailure);
        }
        let normals = LiquidNormalOperator {
            particles: self.fluid.particles,
            pairs: self.fluid.pairs,
            densities: &densities,
            smoothing_radius: self.fluid.smoothing_radius,
        }
        .evaluate_native()?;
        let pressures = self
            .pressure_laws
            .map(|laws| {
                super::LiquidPressureOperator {
                    densities: &densities,
                    laws,
                }
                .evaluate_native()
            })
            .transpose()?;
        Ok(LiquidDensityNormalResult {
            densities,
            normals,
            pressures,
        })
    }
    pub fn admit(
        &self,
        candidate: &LiquidDensityNormalResult,
        absolute: f64,
        relative: f64,
    ) -> Result<(), Error> {
        if candidate.densities.len() != self.fluid.particles.len()
            || !absolute.is_finite()
            || absolute < 0.
            || !relative.is_finite()
            || relative < 0.
        {
            return Err(Error::InvalidConfig);
        }
        let expected = self.evaluate_native()?;
        for (&a, b) in candidate.densities.iter().zip(expected.densities) {
            let error = (a - b).abs();
            if !positive(a) || !error.is_finite() || (error > absolute && error / b > relative) {
                return Err(Error::NumericalFailure);
            }
        }
        LiquidNormalOperator {
            particles: self.fluid.particles,
            pairs: self.fluid.pairs,
            densities: &candidate.densities,
            smoothing_radius: self.fluid.smoothing_radius,
        }
        .admit(&candidate.normals, absolute, relative)?;
        match (self.pressure_laws, &candidate.pressures) {
            (Some(laws), Some(pressures)) => super::LiquidPressureOperator {
                densities: &candidate.densities,
                laws,
            }
            .admit(pressures, absolute, relative),
            (None, None) => Ok(()),
            _ => Err(Error::InvalidConfig),
        }
    }
}
