//! Derived extinction from explicitly marked spherical drops, separate from vapor density.
use super::{Error, FiniteDropletGasGrid, Liquid, finite, positive};
#[derive(Clone, Debug, PartialEq)]
pub struct DropletExtinctionGrid {
    origin: [f64; 3],
    spacing: [f64; 3],
    shape: [usize; 3],
    extinction_m_inverse: Vec<f64>,
    pub included_droplets: usize,
    pub outside_droplets: usize,
}
impl DropletExtinctionGrid {
    pub fn extinction_m_inverse(&self) -> &[f64] {
        &self.extinction_m_inverse
    }
    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }
    pub fn spacing(&self) -> [f64; 3] {
        self.spacing
    }
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }
    /// Exact piecewise-constant line integral through the clipped cell volumes.
    /// CPU reference traverses all cells; GPU ray traversal is separate.
    pub fn optical_depth_segment(&self, start: [f64; 3], end: [f64; 3]) -> Result<f64, Error> {
        if !finite(start) || !finite(end) {
            return Err(Error::InvalidConfig);
        }
        let delta = std::array::from_fn::<_, 3, _>(|k| end[k] - start[k]);
        let length = delta.iter().fold(0_f64, |length, &x| length.hypot(x));
        if !length.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if length == 0. {
            return Ok(0.);
        }
        let mut depth = 0.;
        for (id, &extinction) in self.extinction_m_inverse.iter().enumerate() {
            if extinction == 0. {
                continue;
            }
            let coordinate = [
                id % self.shape[0],
                (id / self.shape[0]) % self.shape[1],
                id / (self.shape[0] * self.shape[1]),
            ];
            let mut lo = 0_f64;
            let mut hi = 1_f64;
            for axis in 0..3 {
                let lower = self.origin[axis] + coordinate[axis] as f64 * self.spacing[axis];
                let upper = lower + self.spacing[axis];
                if delta[axis] == 0. {
                    // Half-open ownership prevents a ray on a shared face being counted twice.
                    if start[axis] < lower || start[axis] >= upper {
                        hi = lo;
                        break;
                    }
                } else {
                    let a = (lower - start[axis]) / delta[axis];
                    let b = (upper - start[axis]) / delta[axis];
                    lo = lo.max(a.min(b));
                    hi = hi.min(a.max(b));
                    if hi <= lo {
                        break;
                    }
                }
            }
            if hi > lo {
                depth += extinction * length * (hi - lo);
            }
        }
        if !depth.is_finite() || depth < 0. {
            return Err(Error::NumericalFailure);
        }
        Ok(depth)
    }
    /// Direct unscattered transmission; in-scattering and spectral calibration are separate.
    pub fn transmittance_segment(&self, start: [f64; 3], end: [f64; 3]) -> Result<f64, Error> {
        Ok((-self.optical_depth_segment(start, end)?).exp())
    }
}
impl Liquid {
    /// Bin extinction cross-section Q*pi*r² per containing gas-cell volume.
    /// Q is explicitly supplied (dimensionless); no wavelength/material calibration is inferred.
    /// Radii represent physical drops, not SPH smoothing/support radii. Unmarked samples contribute zero.
    /// Outside marked drops are counted explicitly. No liquid or gas state is mutated.
    pub fn droplet_extinction_grid(
        &self,
        grid: &FiniteDropletGasGrid,
        radii: &[f64],
        extinction_efficiency: f64,
    ) -> Result<DropletExtinctionGrid, Error> {
        let flags = self
            .droplet_population
            .as_ref()
            .ok_or(Error::InvalidConfig)?;
        if radii.len() != self.particles.len()
            || radii.iter().any(|&r| !positive(r))
            || !extinction_efficiency.is_finite()
            || extinction_efficiency < 0.
        {
            return Err(Error::InvalidConfig);
        }
        let volume = grid.spacing.iter().product::<f64>();
        let mut result = DropletExtinctionGrid {
            origin: grid.origin,
            spacing: grid.spacing,
            shape: grid.shape,
            extinction_m_inverse: vec![0.; grid.cells.len()],
            included_droplets: 0,
            outside_droplets: 0,
        };
        for (i, p) in self.particles.iter().enumerate() {
            if !flags[i] {
                continue;
            }
            if let Some(cell) = grid.cell_index(p.position)? {
                let coefficient =
                    extinction_efficiency * std::f64::consts::PI * radii[i] * radii[i] / volume;
                result.extinction_m_inverse[cell] += coefficient;
                if !result.extinction_m_inverse[cell].is_finite() {
                    return Err(Error::NumericalFailure);
                }
                result.included_droplets += 1;
            } else {
                result.outside_droplets += 1;
            }
        }
        Ok(result)
    }
}

/// Explicit optical calibration for one infinitely distant directional source.
/// Irradiance is incident RGB power per area on a plane normal to the beam,
/// outside the medium; scene radiance must use compatible units. No Mie fit is inferred.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalScatteringLight {
    pub direction_to_light: [f64; 3],
    pub irradiance_rgb: [f64; 3],
    pub single_scattering_albedo: f64,
    /// Mean cosine between incoming and outgoing photon travel directions.
    /// Positive values favor forward scattering; zero is isotropic.
    pub asymmetry: f64,
}
impl DirectionalScatteringLight {
    fn normalized_direction(&self) -> Result<[f64; 3], Error> {
        let scale = self
            .direction_to_light
            .iter()
            .map(|v| v.abs())
            .fold(0_f64, f64::max);
        let scaled = self.direction_to_light.map(|v| v / scale);
        let length = scaled.iter().fold(0_f64, |a, &v| a.hypot(v));
        if !positive(length)
            || !finite(self.direction_to_light)
            || !finite(self.irradiance_rgb)
            || self.irradiance_rgb.iter().any(|v| *v < 0.)
            || !self.single_scattering_albedo.is_finite()
            || !(0. ..=1.).contains(&self.single_scattering_albedo)
            || !self.asymmetry.is_finite()
            || self.asymmetry.abs() >= 1.
        {
            return Err(Error::InvalidConfig);
        }
        Ok(scaled.map(|v| v / length))
    }
    /// Normalized Henyey–Greenstein density per steradian, using photon travel angles.
    pub fn phase(&self, cosine: f64) -> Result<f64, Error> {
        self.normalized_direction()?;
        if !cosine.is_finite() || !(-1. ..=1.).contains(&cosine) {
            return Err(Error::InvalidConfig);
        }
        let g = self.asymmetry;
        // Avoid cancellation near the forward/backward lobe.
        let denominator = (1. - g.abs()).powi(2) + 2. * g.abs() * (1. - g.signum() * cosine);
        let p = (1. - g * g) / (4. * std::f64::consts::PI * denominator * denominator.sqrt());
        if !p.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(p)
    }
}
impl DropletExtinctionGrid {
    /// Deterministic single-scattering quadrature, including medium shadow rays.
    /// Primary segment attenuation is integrated exactly; incident optical depth
    /// is interpolated between subsegment endpoints. This is a convergent approximation,
    /// not multiple scattering or solid-geometry shadowing. Owners are immutable.
    /// The CPU reference tests all cells on each of at most 3*samples rays.
    pub fn directional_scattered_radiance_segment(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        light: DirectionalScatteringLight,
        samples: u32,
        max_cell_tests: usize,
    ) -> Result<[f64; 3], Error> {
        let direction = light.normalized_direction()?;
        if samples == 0 || samples > 4096 || !finite(start) || !finite(end) {
            return Err(Error::InvalidConfig);
        }
        let tests = self
            .extinction_m_inverse
            .len()
            .checked_mul(samples as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or(Error::InvalidConfig)?;
        if tests > max_cell_tests {
            return Err(Error::WorkBudgetExceeded);
        }
        let delta = std::array::from_fn::<_, 3, _>(|k| end[k] - start[k]);
        let length = delta.iter().fold(0_f64, |a, &v| a.hypot(v));
        if !length.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if length == 0. {
            return Ok([0.; 3]);
        }
        let cosine = (0..3)
            .map(|k| direction[k] * delta[k] / length)
            .sum::<f64>()
            .clamp(-1., 1.);
        let phase = light.phase(cosine)?;
        let mut lo = 0_f64;
        let mut hi = 1_f64;
        let extent = std::array::from_fn::<_, 3, _>(|k| self.spacing[k] * self.shape[k] as f64);
        for k in 0..3 {
            let upper = self.origin[k] + extent[k];
            if delta[k] == 0. {
                if start[k] < self.origin[k] || start[k] >= upper {
                    return Ok([0.; 3]);
                }
            } else {
                let a = (self.origin[k] - start[k]) / delta[k];
                let b = (upper - start[k]) / delta[k];
                lo = lo.max(a.min(b));
                hi = hi.min(a.max(b));
            }
        }
        if hi <= lo {
            return Ok([0.; 3]);
        }
        let shadow_length = extent.iter().fold(0_f64, |a, &v| a.hypot(v)) * 2.;
        if !positive(shadow_length) {
            return Err(Error::NumericalFailure);
        }
        let point = |t: f64| std::array::from_fn(|k| start[k] + t * delta[k]);
        let mut prefix_tau = 0.;
        let mut scattered = 0.;
        for i in 0..samples {
            let a = lo + (hi - lo) * f64::from(i) / f64::from(samples);
            let b = lo + (hi - lo) * f64::from(i + 1) / f64::from(samples);
            let begin = point(a);
            let finish = point(b);
            let tau = self.optical_depth_segment(begin, finish)?;
            let shadow = |p: [f64; 3]| {
                self.optical_depth_segment(
                    p,
                    std::array::from_fn(|k| p[k] + direction[k] * shadow_length),
                )
            };
            let shadow_a = shadow(begin)?;
            let shadow_b = shadow(finish)?;
            let rate = (tau + shadow_b - shadow_a).abs();
            let ratio = if rate == 0. {
                1.
            } else {
                -(-rate).exp_m1() / rate
            };
            let minimum = (prefix_tau + shadow_a).min(prefix_tau + tau + shadow_b);
            scattered += tau * ratio * (-minimum).exp();
            prefix_tau += tau;
        }
        let scale = light.single_scattering_albedo * phase * scattered;
        let rgb = light.irradiance_rgb.map(|v| v * scale);
        if !finite(rgb) {
            return Err(Error::NumericalFailure);
        }
        Ok(rgb)
    }
}
