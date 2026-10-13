//! Whole arithmetic chain admission, retaining original physical owners.
use super::{Error,LiquidDensityNormalOperator,LiquidDensityNormalResult,LiquidPairForceInput,LiquidForcePipelineOperator,LiquidForcePipelineResult,sub};
#[derive(Debug)]
pub struct LiquidPipelineOperator<'a> {
    pub fields:LiquidDensityNormalOperator<'a>,
    /// Current geometry/material coefficients. Dynamic fields are replaced only
    /// from the admitted particle fields, never from backend-owned coefficients.
    pub inputs:&'a [LiquidPairForceInput],pub gravity:[f64;3],
}
#[derive(Clone,Debug,PartialEq)]
pub struct LiquidPipelineResult {pub fields:LiquidDensityNormalResult,pub forces:LiquidForcePipelineResult}
impl LiquidPipelineOperator<'_> {
    pub fn pair_inputs(&self,fields:&LiquidDensityNormalResult)->Result<Vec<LiquidPairForceInput>,Error> {
        let n=self.fields.fluid.particles.len();
        let pressure=fields.pressures.as_ref().ok_or(Error::InvalidConfig)?;
        if fields.densities.len()!=n || fields.normals.len()!=n || pressure.len()!=n || self.inputs.len()!=self.fields.fluid.pairs.len() {
            return Err(Error::InvalidConfig);
        }
        self.inputs.iter().zip(self.fields.fluid.pairs).map(|(&input,&(i,j))| {
            if i>=j || j>=n {return Err(Error::InvalidConfig);}
            Ok(LiquidPairForceInput {density:[fields.densities[i],fields.densities[j]],pressure:[pressure[i],pressure[j]],
                normal_difference:sub(fields.normals[i],fields.normals[j]),..input})
        }).collect()
    }
    pub fn evaluate_native(&self)->Result<LiquidPipelineResult,Error> {
        let fields=self.fields.evaluate_native()?;let inputs=self.pair_inputs(&fields)?;
        let forces=LiquidForcePipelineOperator {particles:self.fields.fluid.particles,pairs:self.fields.fluid.pairs,inputs:&inputs,gravity:self.gravity}.evaluate_native()?;
        Ok(LiquidPipelineResult {fields,forces})
    }
    pub fn admit(&self,candidate:&LiquidPipelineResult,absolute:f64,relative:f64)->Result<(),Error> {
        self.fields.admit(&candidate.fields,absolute,relative)?;
        let inputs=self.pair_inputs(&candidate.fields)?;
        LiquidForcePipelineOperator {particles:self.fields.fluid.particles,pairs:self.fields.fluid.pairs,inputs:&inputs,gravity:self.gravity}
            .admit(&candidate.forces,absolute,relative)
    }
}
