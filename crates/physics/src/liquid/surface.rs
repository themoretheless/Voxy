//! Bounded marching-tetrahedra reconstruction of the particle volume fraction field.
use super::{Error, Liquid, finite, positive, sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceConfig {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub cell_size: f64,
    pub isovalue: f64,
    pub material: Option<usize>,
    pub max_samples: usize,
    pub max_checks: usize,
    pub max_triangles: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidSurface {
    /// Independent oriented triangles, outward from the reconstructed liquid.
    pub triangles: Vec<[[f64; 3]; 3]>,
    pub samples: usize,
    pub checks: usize,
}
impl Liquid {
    /// Reconstructs a free-surface mesh from a poly6 volume-fraction field.
    /// The supplied grid must contain the desired surface; crossing its boundaries clips it.
    /// This samples geometry only and does not modify the simulation or enforce volume.
    /// # Errors
    /// Rejects invalid bounds/threshold, overflowing grid dimensions, exceeded budgets,
    /// invalid property response, or nonfinite field values.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn surface(&self, config: SurfaceConfig) -> Result<LiquidSurface, Error> {
        self.surface_with_particle_support(
            config,
            &vec![self.config.smoothing_radius; self.particles.len()],
        )
    }

    /// Samples the same poly6 volume field with an explicit support radius for
    /// every particle. This permits small resolved drops without erasing them
    /// through the carrier's larger support. Only local grid nodes are visited.
    pub fn surface_with_particle_support(
        &self,
        config: SurfaceConfig,
        supports: &[f64],
    ) -> Result<LiquidSurface, Error> {
        if supports.len() != self.particles.len() || supports.iter().any(|v| !positive(*v)) {
            return Err(Error::InvalidSurface);
        }
        if !finite(config.min)
            || !finite(config.max)
            || !positive(config.cell_size)
            || !positive(config.isovalue)
            || config.max_samples == 0
            || config.max_checks == 0
            || config.max_triangles == 0
            || config.material.is_some_and(|m| m >= self.materials.len())
        {
            return Err(Error::InvalidSurface);
        }
        let mut dimensions = [0_usize; 3];
        for (axis, dimension) in dimensions.iter_mut().enumerate() {
            let cells = ((config.max[axis] - config.min[axis]) / config.cell_size).ceil();
            if !positive(cells) || cells >= config.max_samples as f64 {
                return Err(Error::SurfaceBudget);
            }
            *dimension = cells as usize + 1;
        }
        let samples = dimensions[0]
            .checked_mul(dimensions[1])
            .and_then(|v| v.checked_mul(dimensions[2]))
            .ok_or(Error::SurfaceBudget)?;
        if samples > config.max_samples {
            return Err(Error::SurfaceBudget);
        }
        let properties = self.effective_materials()?;
        let particles: Vec<_> = self
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| config.material.is_none_or(|m| p.material == m))
            .collect();
        let index = |x: usize, y: usize, z: usize| (z * dimensions[1] + y) * dimensions[0] + x;
        let point = |x: usize, y: usize, z: usize| {
            std::array::from_fn(|axis| config.min[axis] + [x, y, z][axis] as f64 * config.cell_size)
        };
        let mut values = vec![0.0; samples];
        let mut checks = 0;
        for &(i, p) in &particles {
            let h = supports[i];
            let kernel = 315.0 / (64.0 * std::f64::consts::PI * h.powi(3));
            if !positive(kernel) {
                return Err(Error::NumericalFailure);
            }
            let mut lower = [0; 3];
            let mut upper = [0; 3];
            let mut outside = false;
            for axis in 0..3 {
                let lo = ((p.position[axis] - h - config.min[axis]) / config.cell_size).ceil();
                let hi = ((p.position[axis] + h - config.min[axis]) / config.cell_size).floor();
                if !lo.is_finite() || !hi.is_finite() {
                    return Err(Error::NumericalFailure);
                }
                if hi < 0.0 || lo > (dimensions[axis] - 1) as f64 {
                    outside = true;
                    break;
                }
                lower[axis] = lo.max(0.0) as usize;
                upper[axis] = hi.min((dimensions[axis] - 1) as f64) as usize;
                if lower[axis] > upper[axis] {
                    outside = true;
                    break;
                }
            }
            if outside {
                continue;
            }
            for z in lower[2]..=upper[2] {
                for y in lower[1]..=upper[1] {
                    for x in lower[0]..=upper[0] {
                        if checks == config.max_checks {
                            return Err(Error::SurfaceBudget);
                        }
                        checks += 1;
                        let pos = point(x, y, z);
                        if !finite(pos) {
                            return Err(Error::NumericalFailure);
                        }
                        let squared: f64 =
                            sub(pos, p.position).iter().map(|d| (d / h).powi(2)).sum();
                        if squared < 1.0 {
                            let value = &mut values[index(x, y, z)];
                            *value += p.mass / properties[i].rest_density
                                * kernel
                                * (1.0 - squared).powi(3);
                            if !value.is_finite() {
                                return Err(Error::NumericalFailure);
                            }
                        }
                    }
                }
            }
        }
        let mut triangles = Vec::new();
        for z in 0..dimensions[2] - 1 {
            for y in 0..dimensions[1] - 1 {
                for x in 0..dimensions[0] - 1 {
                    let coordinates = [
                        [x, y, z],
                        [x + 1, y, z],
                        [x + 1, y + 1, z],
                        [x, y + 1, z],
                        [x, y, z + 1],
                        [x + 1, y, z + 1],
                        [x + 1, y + 1, z + 1],
                        [x, y + 1, z + 1],
                    ];
                    let field = coordinates.map(|[x, y, z]| values[index(x, y, z)]);
                    if field.iter().all(|v| *v < config.isovalue)
                        || field.iter().all(|v| *v >= config.isovalue)
                    {
                        continue;
                    }
                    let points = coordinates.map(|[x, y, z]| point(x, y, z));
                    for tetra in [
                        [0, 1, 2, 6],
                        [0, 2, 3, 6],
                        [0, 3, 7, 6],
                        [0, 7, 4, 6],
                        [0, 4, 5, 6],
                        [0, 5, 1, 6],
                    ] {
                        let inside: Vec<_> = tetra
                            .into_iter()
                            .filter(|&i| field[i] >= config.isovalue)
                            .collect();
                        let outside: Vec<_> = tetra
                            .into_iter()
                            .filter(|&i| field[i] < config.isovalue)
                            .collect();
                        if inside.is_empty() || outside.is_empty() {
                            continue;
                        }
                        let interpolate = |i: usize, j: usize| {
                            let fraction = (config.isovalue - field[i]) / (field[j] - field[i]);
                            std::array::from_fn(|axis| {
                                points[i][axis] + fraction * (points[j][axis] - points[i][axis])
                            })
                        };
                        let direction = sub(points[outside[0]], points[inside[0]]);
                        let mut add = |mut triangle: [[f64; 3]; 3]| -> Result<(), Error> {
                            let normal =
                                cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]));
                            if normal.iter().map(|v| v * v).sum::<f64>() <= f64::MIN_POSITIVE {
                                return Ok(());
                            }
                            if normal
                                .iter()
                                .zip(direction)
                                .map(|(a, b)| a * b)
                                .sum::<f64>()
                                < 0.0
                            {
                                triangle.swap(1, 2);
                            }
                            if triangles.len() >= config.max_triangles {
                                return Err(Error::SurfaceBudget);
                            }
                            triangles.push(triangle);
                            Ok(())
                        };
                        match inside.len() {
                            1 => add([
                                interpolate(inside[0], outside[0]),
                                interpolate(inside[0], outside[1]),
                                interpolate(inside[0], outside[2]),
                            ])?,
                            3 => add([
                                interpolate(outside[0], inside[0]),
                                interpolate(outside[0], inside[1]),
                                interpolate(outside[0], inside[2]),
                            ])?,
                            _ => {
                                let first_left = interpolate(inside[0], outside[0]);
                                let first_right = interpolate(inside[0], outside[1]);
                                let second_left = interpolate(inside[1], outside[0]);
                                let second_right = interpolate(inside[1], outside[1]);
                                add([first_left, first_right, second_right])?;
                                add([first_left, second_right, second_left])?;
                            }
                        }
                    }
                }
            }
        }
        Ok(LiquidSurface {
            triangles,
            samples,
            checks,
        })
    }
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
