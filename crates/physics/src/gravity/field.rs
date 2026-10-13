//! Gravity sampled at integer-anchored positions, retaining far-world precision.
use crate::{
    Origin,
    gravity::{Error, Gravity},
};

pub trait GravityField {
    /// # Errors
    /// Rejects invalid inputs, singular point sources, and numerical overflow.
    fn acceleration(&self, anchor: Origin, local: [f64; 3]) -> Result<[f64; 3], Error>;
}

impl GravityField for [f64; 3] {
    fn acceleration(&self, _anchor: Origin, local: [f64; 3]) -> Result<[f64; 3], Error> {
        if self.iter().chain(&local).any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        Ok(*self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub anchor: Origin,
    pub position: [f64; 3],
    pub mass: f64,
    /// A homogeneous spherical source. Zero denotes a point mass.
    pub radius: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct NewtonianField<'a> {
    pub gravity: Gravity,
    pub sources: &'a [Source],
}

impl NewtonianField<'_> {
    /// Jacobian of acceleration: multiply by a small separation to obtain the
    /// first-order tidal acceleration. Uniform fields have zero gradient.
    /// # Errors
    /// Invalid sources, singular point masses, or numerical overflow.
    #[allow(clippy::cast_precision_loss)]
    pub fn tidal_tensor(self, anchor: Origin, local: [f64; 3]) -> Result<[[f64; 3]; 3], Error> {
        // Validate against exactly the same field contract as acceleration.
        self.acceleration(anchor, local)?;
        let mut tensor = [[0.0; 3]; 3];
        for source in self.sources {
            if self.gravity.constant == 0.0 {
                continue;
            }
            let origins = [
                i128::from(source.anchor.x) - i128::from(anchor.x),
                i128::from(source.anchor.y) - i128::from(anchor.y),
                i128::from(source.anchor.z) - i128::from(anchor.z),
            ];
            let delta: [f64; 3] =
                std::array::from_fn(|k| origins[k] as f64 + source.position[k] - local[k]);
            let radius_squared = delta.iter().map(|v| v * v).sum::<f64>();
            let inside = radius_squared < source.radius * source.radius;
            let denominator = radius_squared.max(source.radius * source.radius)
                + self.gravity.softening * self.gravity.softening;
            let scale = self.gravity.constant / denominator / denominator.sqrt() * source.mass;
            for (i, row) in tensor.iter_mut().enumerate() {
                for (j, value) in row.iter_mut().enumerate() {
                    *value += scale
                        * (if inside {
                            0.0
                        } else {
                            3.0 * delta[i] * delta[j] / denominator
                        } - if i == j { 1.0 } else { 0.0 });
                }
            }
        }
        if tensor.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(tensor)
    }
}

impl GravityField for NewtonianField<'_> {
    #[allow(clippy::cast_precision_loss)] // Subtract integer origins before conversion.
    fn acceleration(&self, anchor: Origin, local: [f64; 3]) -> Result<[f64; 3], Error> {
        self.gravity.accelerations(&[])?;
        if local.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        let mut result = self.gravity.uniform_acceleration;
        for source in self.sources {
            if !source.mass.is_finite()
                || source.mass <= 0.0
                || !source.radius.is_finite()
                || source.radius < 0.0
                || source.position.iter().any(|v| !v.is_finite())
            {
                return Err(Error::InvalidInput);
            }
            let origins = [
                i128::from(source.anchor.x) - i128::from(anchor.x),
                i128::from(source.anchor.y) - i128::from(anchor.y),
                i128::from(source.anchor.z) - i128::from(anchor.z),
            ];
            let delta: [f64; 3] =
                std::array::from_fn(|k| origins[k] as f64 + source.position[k] - local[k]);
            let distance_squared = delta.iter().map(|v| v * v).sum::<f64>();
            // Inside a uniform sphere, enclosed mass scales as r³: acceleration
            // is linear and reaches zero at the center. Softening remains optional.
            let denominator_squared = distance_squared.max(source.radius * source.radius)
                + self.gravity.softening * self.gravity.softening;
            if !denominator_squared.is_finite() {
                return Err(Error::NumericalOverflow);
            }
            if self.gravity.constant == 0.0 {
                continue;
            }
            if denominator_squared == 0.0 {
                return Err(Error::SingularPair);
            }
            let scale = self.gravity.constant / denominator_squared / denominator_squared.sqrt();
            for k in 0..3 {
                result[k] += delta[k] * scale * source.mass;
            }
        }
        if result.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(result)
    }
}
