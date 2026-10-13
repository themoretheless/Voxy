//! Joint admission for a GPU force-to-accumulation pipeline.
use super::{
    Error, LiquidAccumulationOperator, LiquidAccumulationResult, LiquidPairForceInput,
    LiquidPairForceResult, Particle,
};
#[derive(Debug)]
pub struct LiquidForcePipelineOperator<'a> {
    pub particles: &'a [Particle],
    pub pairs: &'a [(usize, usize)],
    pub inputs: &'a [LiquidPairForceInput],
    pub gravity: [f64; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidForcePipelineResult {
    pub forces: Vec<LiquidPairForceResult>,
    pub accumulation: LiquidAccumulationResult,
}
impl LiquidForcePipelineOperator<'_> {
    pub fn evaluate_native(&self) -> Result<LiquidForcePipelineResult, Error> {
        if self.inputs.len() != self.pairs.len() {
            return Err(Error::InvalidConfig);
        }
        let forces = self
            .inputs
            .iter()
            .map(|input| input.evaluate_native())
            .collect::<Result<Vec<_>, _>>()?;
        let accumulation = LiquidAccumulationOperator {
            particles: self.particles,
            pairs: self.pairs,
            forces: &forces,
            gravity: self.gravity,
        }
        .evaluate_native()?;
        Ok(LiquidForcePipelineResult {
            forces,
            accumulation,
        })
    }
    pub fn admit(
        &self,
        candidate: &LiquidForcePipelineResult,
        absolute: f64,
        relative: f64,
    ) -> Result<(), Error> {
        if self.inputs.len() != self.pairs.len() || candidate.forces.len() != self.inputs.len() {
            return Err(Error::InvalidConfig);
        }
        for (input, &result) in self.inputs.iter().zip(&candidate.forces) {
            input.admit(result, absolute, relative)?;
        }
        LiquidAccumulationOperator {
            particles: self.particles,
            pairs: self.pairs,
            forces: &candidate.forces,
            gravity: self.gravity,
        }
        .admit(&candidate.accumulation, absolute, relative)
    }
}
