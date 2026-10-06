//! Continuous load descriptions, separate from instantaneous contact wrenches.
use super::{Error, TorquePolynomial};

// One compensated owner for streaming coefficient sums; no storage proportional
// to input point count and no duplicate numerical summation implementation.
struct CoefficientSum<const N: usize> {
    sums: [[f64; 3]; N],
    corrections: [[f64; 3]; N],
}
impl<const N: usize> CoefficientSum<N> {
    fn new() -> Self {
        Self {
            sums: [[0.; 3]; N],
            corrections: [[0.; 3]; N],
        }
    }
    fn add(&mut self, coefficients: [[f64; 3]; N]) -> Result<(), Error> {
        for (i, vector) in coefficients.iter().enumerate() {
            for (k, value) in vector.iter().copied().enumerate() {
                let old = self.sums[i][k];
                let sum = old + value;
                self.corrections[i][k] += if old.abs() >= value.abs() {
                    (old - sum) + value
                } else {
                    (value - sum) + old
                };
                self.sums[i][k] = sum;
                if !sum.is_finite() || !self.corrections[i][k].is_finite() {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        Ok(())
    }
    fn finish(self) -> Result<[[f64; 3]; N], Error> {
        let result: [[f64; 3]; N] = std::array::from_fn(|i| {
            std::array::from_fn(|k| self.sums[i][k] + self.corrections[i][k])
        });
        if result.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
}

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
    /// Aggregate independent COM loads sharing the same world frame and time
    /// origin. Retain original recipes when rebasing, then aggregate again.
    /// Compensated coefficient sums preserve small loads across cancellation.
    pub fn aggregate(loads: impl IntoIterator<Item = Self>) -> Result<Self, Error> {
        let mut sum = CoefficientSum::<7>::new();
        for load in loads {
            load.validate()?;
            sum.add([
                load.force,
                load.force_rate,
                load.torque.value,
                load.torque.rate,
                load.torque.acceleration,
                load.torque.jerk,
                load.torque.snap,
            ])?;
        }
        let coefficients = sum.finish()?;
        let result = Self {
            force: coefficients[0],
            force_rate: coefficients[1],
            torque: TorquePolynomial {
                value: coefficients[2],
                rate: coefficients[3],
                acceleration: coefficients[4],
                jerk: coefficients[5],
                snap: coefficients[6],
            },
        };
        result.validate().map_err(|_| Error::NumericalFailure)?;
        Ok(result)
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

/// Aggregate affine forces on multiple points of one body. Three world-force
/// columns encode sum_j (R e_j) cross column_j, without an arbitrary point cap.
/// This descriptor does not prepare motion or solve contacts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialForceMoment {
    pub force: [f64; 3],
    pub force_rate: [f64; 3],
    pub columns: [[f64; 3]; 3],
    pub rate_columns: [[f64; 3]; 3],
}
impl MaterialForceMoment {
    pub fn aggregate(forces: impl IntoIterator<Item = MaterialPointForce>) -> Result<Self, Error> {
        let mut sum = CoefficientSum::<8>::new();
        for point in forces {
            point.validate()?;
            let mut coefficients = [[0.; 3]; 8];
            coefficients[0] = point.force;
            coefficients[1] = point.force_rate;
            for j in 0..3 {
                coefficients[2 + j] = point.force.map(|f| f * point.local[j]);
                coefficients[5 + j] = point.force_rate.map(|f| f * point.local[j]);
            }
            sum.add(coefficients)?;
        }
        let coefficients = sum.finish()?;
        Ok(Self {
            force: coefficients[0],
            force_rate: coefficients[1],
            columns: [coefficients[2], coefficients[3], coefficients[4]],
            rate_columns: [coefficients[5], coefficients[6], coefficients[7]],
        })
    }
    /// Exact nominal torque impulse when the supplied body orientation rotates
    /// at a constant world angular velocity over the interval.
    pub fn angular_impulse(
        self,
        orientation: [f64; 4],
        omega: [f64; 3],
        duration: f64,
    ) -> Result<[f64; 3], Error> {
        self.torque_at(orientation, 0.)?;
        if !duration.is_finite() || duration < 0. || omega.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let mut sum = CoefficientSum::<1>::new();
        for j in 0..3 {
            let mut basis = [0.; 3];
            basis[j] = 1.;
            let arm = crate::astrophysics_spin::rotate(orientation, basis);
            let (zero, first) =
                crate::astrophysics_spin::rotating_arm_integrals(arm, omega, duration)
                    .map_err(|_| Error::NumericalFailure)?;
            let constant = crate::astrophysics_spin::cross(zero, self.columns[j]);
            let changing = crate::astrophysics_spin::cross(first, self.rate_columns[j]);
            sum.add([constant])?;
            sum.add([changing])?;
        }
        Ok(sum.finish()?[0])
    }
    /// Instantaneous intrinsic torque derivative for a moving body frame.
    /// Includes arm rotation as well as the affine world-force derivative.
    /// The orientation and angular velocity refer to the same instant.
    pub fn torque_rate_at(
        self,
        orientation: [f64; 4],
        omega: [f64; 3],
        time: f64,
    ) -> Result<[f64; 3], Error> {
        self.torque_at(orientation, time)?;
        if omega.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let mut sum = CoefficientSum::<1>::new();
        for j in 0..3 {
            let mut basis = [0.; 3];
            basis[j] = 1.;
            let arm = crate::astrophysics_spin::rotate(orientation, basis);
            let moving_arm = crate::astrophysics_spin::cross(omega, arm);
            let force =
                std::array::from_fn(|k| self.rate_columns[j][k].mul_add(time, self.columns[j][k]));
            sum.add([crate::astrophysics_spin::cross(moving_arm, force)])?;
            sum.add([crate::astrophysics_spin::cross(arm, self.rate_columns[j])])?;
        }
        Ok(sum.finish()?[0])
    }
    /// Intrinsic world torque about COM at a supplied unit body orientation.
    pub fn torque_at(self, orientation: [f64; 4], time: f64) -> Result<[f64; 3], Error> {
        let norm = orientation.iter().map(|x| x * x).sum::<f64>();
        if !time.is_finite()
            || time < 0.
            || !norm.is_finite()
            || (norm - 1.).abs() > 1e-10
            || self
                .columns
                .iter()
                .flatten()
                .chain(self.rate_columns.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidInput);
        }
        let mut sum = CoefficientSum::<1>::new();
        for j in 0..3 {
            let mut basis = [0.; 3];
            basis[j] = 1.;
            let arm = crate::astrophysics_spin::rotate(orientation, basis);
            let force =
                std::array::from_fn(|k| self.rate_columns[j][k].mul_add(time, self.columns[j][k]));
            let torque = crate::astrophysics_spin::cross(arm, force);
            if torque.iter().any(|v| !v.is_finite()) {
                return Err(Error::NumericalFailure);
            }
            sum.add([torque])?;
        }
        Ok(sum.finish()?[0])
    }
}
