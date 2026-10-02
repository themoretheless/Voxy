//! Enclosed free-surface reconstruction with a measured volume tolerance.
use super::{Error, Liquid, LiquidSurface, SurfaceConfig, finite, positive};
#[derive(Clone, Copy, Debug)]
pub struct SurfaceVolumeControl {
    pub relative_tolerance: f64,
    pub max_iterations: usize,
    /// Total local particle/node checks across all threshold trials.
    pub max_checks: usize,
}
#[derive(Clone, Debug)]
pub struct VolumeMatchedSurface {
    pub surface: LiquidSurface,
    pub isovalue: f64,
    pub target_volume: f64,
    pub measured_volume: f64,
    pub iterations: usize,
    pub checks: usize,
}
impl LiquidSurface {
    /// Signed volume of oriented closed triangles, with a translated origin
    /// to avoid cancellation when a small liquid object is far from zero.
    /// Closure must be established independently; open meshes are unsupported.
    pub fn enclosed_volume(&self) -> Result<f64, Error> {
        let Some(first) = self.triangles.first() else {
            return Ok(0.0);
        };
        let origin = first[0];
        let mut volume = 0.0;
        for triangle in &self.triangles {
            let [a, b, c]: [[f64; 3]; 3] =
                triangle.map(|p| std::array::from_fn(|k| p[k] - origin[k]));
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            volume += (0..3).map(|k| a[k] * cross[k]).sum::<f64>() / 6.0;
        }
        if !volume.is_finite() || volume < 0.0 {
            return Err(Error::NumericalFailure);
        }
        Ok(volume)
    }
}
impl Liquid {
    /// Chooses a field threshold whose closed mesh volume matches sum(m/rho).
    /// All selected kernel supports must fit inside the supplied grid bounds.
    /// This preserves the *rendered total volume*, without moving particles or
    /// claiming component-wise surface volumes or calibrated optical properties.
    /// Failure to meet tolerance within the aggregate work budget is an error.
    pub fn surface_volume_matched(
        &self,
        mut config: SurfaceConfig,
        supports: &[f64],
        control: SurfaceVolumeControl,
    ) -> Result<VolumeMatchedSurface, Error> {
        if !positive(config.isovalue)
            || supports.len() != self.particles.len()
            || supports.iter().any(|v| !positive(*v))
            || !positive(control.relative_tolerance)
            || control.relative_tolerance > 0.1
            || control.max_iterations == 0
            || control.max_iterations > 64
            || control.max_checks == 0
        {
            return Err(Error::InvalidSurface);
        }
        let properties = self.effective_materials()?;
        let mut target = 0.0;
        let mut upper = 0.0;
        for (i, p) in self
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| config.material.is_none_or(|m| p.material == m))
        {
            for k in 0..3 {
                if p.position[k] - supports[i] < config.min[k]
                    || p.position[k] + supports[i] > config.max[k]
                {
                    return Err(Error::InvalidSurface);
                }
            }
            let volume = p.mass / properties[i].rest_density;
            target += volume;
            upper += volume * 315.0 / (64.0 * std::f64::consts::PI * supports[i].powi(3));
        }
        if !target.is_finite() || !upper.is_finite() || !finite(config.min) || !finite(config.max) {
            return Err(Error::NumericalFailure);
        }
        if target > 0.0 {
            if !positive(upper) {
                return Err(Error::NumericalFailure);
            }
            config.isovalue = config.isovalue.min(0.5 * upper);
        }
        let mut lower = 0.0;
        let mut checks = 0;
        for iteration in 1..=control.max_iterations {
            let remaining = control.max_checks - checks;
            if remaining == 0 {
                return Err(Error::SurfaceBudget);
            }
            config.max_checks = config.max_checks.min(remaining);
            let surface = self.surface_with_particle_support(config, supports)?;
            checks += surface.checks;
            let measured = surface.enclosed_volume()?;
            if target == 0.0 || (measured - target).abs() <= control.relative_tolerance * target {
                return Ok(VolumeMatchedSurface {
                    surface,
                    isovalue: config.isovalue,
                    target_volume: target,
                    measured_volume: measured,
                    iterations: iteration,
                    checks,
                });
            }
            if measured > target {
                lower = config.isovalue;
            } else {
                upper = config.isovalue;
            }
            let next = 0.5 * lower + 0.5 * upper;
            if !positive(next) || next == config.isovalue {
                return Err(Error::NumericalFailure);
            }
            config.isovalue = next;
        }
        Err(Error::SurfaceBudget)
    }
}
