//! Continuous load descriptions, separate from instantaneous contact wrenches.
use super::{Error, TorquePolynomial};

/// Affine world force applied to one body-local material point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialPointForce {
    pub local: [f64; 3],
    pub force: [f64; 3],
    pub force_rate: [f64; 3],
}
impl MaterialPointForce {
    pub(super) fn validate(self) -> Result<(), Error> {
        if self
            .local
            .iter()
            .chain(&self.force)
            .chain(&self.force_rate)
            .any(|v| !v.is_finite())
        {
            Err(Error::InvalidInput)
        } else {
            Ok(())
        }
    }
    pub fn shifted(self, time: f64) -> Result<Self, Error> {
        self.validate()?;
        if !time.is_finite() || time < 0. {
            return Err(Error::InvalidInput);
        }
        let force = std::array::from_fn(|k| self.force_rate[k].mul_add(time, self.force[k]));
        if force.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(Self { force, ..self })
    }
}
/// An affine world COM force and polynomial world torque about COM.
/// This is additional to the material-point force, not a replacement for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionLoad {
    pub force: [f64; 3],
    pub force_rate: [f64; 3],
    pub torque: TorquePolynomial,
}
impl MotionLoad {
    pub fn zero() -> Self {
        Self {
            force: [0.; 3],
            force_rate: [0.; 3],
            torque: TorquePolynomial::constant([0.; 3]),
        }
    }
    pub(super) fn validate(self) -> Result<(), Error> {
        if self
            .force
            .iter()
            .chain(&self.force_rate)
            .any(|v| !v.is_finite())
            || self.torque.validate().is_err()
        {
            Err(Error::InvalidInput)
        } else {
            Ok(())
        }
    }
    pub fn shifted(self, time: f64) -> Result<Self, Error> {
        self.validate()?;
        if !time.is_finite() || time < 0. {
            return Err(Error::InvalidInput);
        }
        let force = std::array::from_fn(|k| self.force_rate[k].mul_add(time, self.force[k]));
        if force.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(Self {
            force,
            torque: self
                .torque
                .shifted(time)
                .map_err(|_| Error::NumericalFailure)?,
            ..self
        })
    }
    pub(super) fn combined_force(
        self,
        material: MaterialPointForce,
    ) -> Result<([f64; 3], [f64; 3]), Error> {
        self.validate()?;
        material.validate()?;
        let force: [f64; 3] = std::array::from_fn(|k| self.force[k] + material.force[k]);
        let rate: [f64; 3] = std::array::from_fn(|k| self.force_rate[k] + material.force_rate[k]);
        if force.iter().chain(&rate).any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok((force, rate))
    }
}
