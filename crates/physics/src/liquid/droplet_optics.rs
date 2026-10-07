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
