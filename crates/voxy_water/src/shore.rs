//! Shorelines from a bathymetry grid: water depth, signed distance to the waterline and
//! the shoreward direction, plus the breaking-wave model evaluated on them.
//!
//! The same [`ShoreWaves::height`] formula is implemented in `shaders/water_common.wgsl`
//! (`shore_wave`), so floating bodies ride the breakers that are drawn.

use glam::Vec2;

/// Terrain heights on a regular grid (row-major, `x` fastest).
#[derive(Clone, Debug, PartialEq)]
pub struct Bathymetry {
    /// World xz of the first sample.
    pub origin: Vec2,
    pub cell: f32,
    pub size: [u32; 2],
    pub heights: Vec<f32>,
}

impl Bathymetry {
    pub fn from_fn(origin: Vec2, cell: f32, size: [u32; 2], height: impl Fn(Vec2) -> f32) -> Self {
        let mut heights = Vec::with_capacity((size[0] * size[1]) as usize);
        for y in 0..size[1] {
            for x in 0..size[0] {
                heights.push(height(origin + Vec2::new(x as f32, y as f32) * cell));
            }
        }
        Self {
            origin,
            cell,
            size,
            heights,
        }
    }

    /// Bilinear terrain height (clamped at the grid edge).
    pub fn height(&self, xz: Vec2) -> f32 {
        bilinear(&self.heights, self.size, (xz - self.origin) / self.cell)
    }
}

fn bilinear(values: &[f32], size: [u32; 2], g: Vec2) -> f32 {
    let max = Vec2::new(size[0] as f32 - 1.0, size[1] as f32 - 1.0);
    let g = g.clamp(Vec2::ZERO, max);
    let (x0, y0) = (g.x.floor() as u32, g.y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(size[0] - 1), (y0 + 1).min(size[1] - 1));
    let f = g - Vec2::new(x0 as f32, y0 as f32);
    let at = |x: u32, y: u32| values[(y * size[0] + x) as usize];
    let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * f.x;
    let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * f.x;
    top + (bottom - top) * f.y
}

/// Depth and shoreline distance for one water level.
#[derive(Clone, Debug, PartialEq)]
pub struct ShoreField {
    pub origin: Vec2,
    pub cell: f32,
    pub size: [u32; 2],
    pub level: f32,
    /// Water depth (negative on land).
    pub depth: Vec<f32>,
    /// Distance to the waterline: positive offshore, negative inland (m).
    pub distance: Vec<f32>,
}

impl ShoreField {
    /// Chamfer (3-4) distance transform from the cells the waterline crosses.
    pub fn new(bathymetry: &Bathymetry, level: f32) -> Self {
        let [w, h] = bathymetry.size;
        let depth: Vec<f32> = bathymetry.heights.iter().map(|t| level - t).collect();
        let index = |x: u32, y: u32| (y * w + x) as usize;
        let mut distance = vec![f32::MAX; depth.len()];
        for y in 0..h {
            for x in 0..w {
                let d = depth[index(x, y)];
                let mut edge = false;
                for (dx, dy) in [(1i32, 0i32), (0, 1), (-1, 0), (0, -1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                        let n = depth[index(nx as u32, ny as u32)];
                        if (n > 0.0) != (d > 0.0) {
                            edge = true;
                            // sub-cell position of the crossing along this edge
                            let t = d.abs() / (d.abs() + n.abs()).max(1e-6);
                            distance[index(x, y)] = distance[index(x, y)].min(t);
                        }
                    }
                }
                if !edge {
                    distance[index(x, y)] = f32::MAX;
                }
            }
        }
        let pass = |distance: &mut Vec<f32>, x: u32, y: u32, offsets: &[(i32, i32, f32)]| {
            let mut best = distance[index(x, y)];
            for &(dx, dy, cost) in offsets {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                    best = best.min(distance[index(nx as u32, ny as u32)] + cost);
                }
            }
            distance[index(x, y)] = best;
        };
        let (a, b) = (1.0, std::f32::consts::SQRT_2);
        let forward = [(-1, 0, a), (0, -1, a), (-1, -1, b), (1, -1, b)];
        let backward = [(1, 0, a), (0, 1, a), (1, 1, b), (-1, 1, b)];
        for _ in 0..2 {
            for y in 0..h {
                for x in 0..w {
                    pass(&mut distance, x, y, &forward);
                }
            }
            for y in (0..h).rev() {
                for x in (0..w).rev() {
                    pass(&mut distance, x, y, &backward);
                }
            }
        }
        for (dist, d) in distance.iter_mut().zip(&depth) {
            let metres = if *dist == f32::MAX {
                1.0e4
            } else {
                *dist * bathymetry.cell
            };
            *dist = if *d > 0.0 { metres } else { -metres };
        }
        Self {
            origin: bathymetry.origin,
            cell: bathymetry.cell,
            size: bathymetry.size,
            level,
            depth,
            distance,
        }
    }

    fn grid(&self, xz: Vec2) -> Vec2 {
        (xz - self.origin) / self.cell
    }

    pub fn depth_at(&self, xz: Vec2) -> f32 {
        bilinear(&self.depth, self.size, self.grid(xz))
    }

    pub fn distance_at(&self, xz: Vec2) -> f32 {
        bilinear(&self.distance, self.size, self.grid(xz))
    }

    /// Unit horizontal direction towards the shore.
    pub fn shoreward(&self, xz: Vec2) -> Vec2 {
        let e = self.cell;
        let g = Vec2::new(
            self.distance_at(xz + Vec2::X * e) - self.distance_at(xz - Vec2::X * e),
            self.distance_at(xz + Vec2::Y * e) - self.distance_at(xz - Vec2::Y * e),
        );
        (-g).normalize_or_zero()
    }

    /// Texture payload: `(depth, distance, shoreward.x, shoreward.y)` per cell.
    pub fn texels(&self) -> Vec<[f32; 4]> {
        let mut out = Vec::with_capacity(self.depth.len());
        for y in 0..self.size[1] {
            for x in 0..self.size[0] {
                let xz = self.origin + Vec2::new(x as f32, y as f32) * self.cell;
                let i = (y * self.size[0] + x) as usize;
                let s = self.shoreward(xz);
                out.push([
                    self.depth[i],
                    self.distance[i].clamp(-1.0e4, 1.0e4),
                    s.x,
                    s.y,
                ]);
            }
        }
        out
    }
}

/// Swell trains that shoal and break as they reach the beach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShoreWaves {
    /// Deep-water breaker height (m).
    pub height: f32,
    pub period: f32,
    /// Breakers appear within this distance of the waterline (m).
    pub reach: f32,
}

impl Default for ShoreWaves {
    fn default() -> Self {
        Self {
            height: 0.45,
            period: 7.0,
            reach: 35.0,
        }
    }
}

/// Breaker state at a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breaker {
    pub height: f32,
    /// Slope of the breaker profile along the shoreward direction.
    pub slope: f32,
    /// 0..1 white water from breaking and swash.
    pub foam: f32,
    /// Fraction of the deep-water FFT waves left in this depth.
    pub fft_scale: f32,
}

impl ShoreWaves {
    /// Mirrors `shore_wave` in `water_common.wgsl`.
    pub fn height(&self, depth: f32, distance: f32, time: f32) -> Breaker {
        let fft_scale = (depth / 2.5).clamp(0.0, 1.0);
        if distance <= -2.0 || distance > self.reach || self.height <= 0.0 {
            return Breaker {
                height: 0.0,
                slope: 0.0,
                foam: 0.0,
                fft_scale,
            };
        }
        let d = depth.max(0.02);
        // shallow-water celerity sqrt(g d): crests slow down and bunch up near the beach
        let k0 = std::f32::consts::TAU / (9.81 * self.period * self.period / std::f32::consts::TAU);
        let k = k0 / (d * k0).tanh().max(0.05).sqrt();
        let phase = distance * k + std::f32::consts::TAU * time / self.period;
        // Green's law shoaling, capped by depth-limited breaking (H <= 0.78 d)
        let shoaled = self.height * (2.5 / d).powf(0.25).min(1.8);
        let height = shoaled.min(0.78 * d) * (1.0 - distance / self.reach).clamp(0.0, 1.0);
        let crest = (phase.sin() * 0.5 + 0.5).powi(3);
        let dcrest = 1.5 * (phase.sin() * 0.5 + 0.5).powi(2) * phase.cos() * k;
        let breaking = ((shoaled / (0.78 * d)) - 0.9).clamp(0.0, 1.0);
        let swash = (1.0 - distance.abs() / 1.5).clamp(0.0, 1.0);
        Breaker {
            height: height * crest,
            slope: height * dcrest,
            // white water rides the steep front of each breaking crest
            foam: (breaking * (phase.sin() * 0.5 + 0.5).powi(8) * 1.3 + swash * 0.6)
                .clamp(0.0, 1.0),
            fft_scale,
        }
    }
}

/// Second-order Total Variation Diminishing (`TVD`) flux limiter schemes for shallow-water
/// wave and foam transport without spurious numerical oscillations near steep fronts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluxLimiter {
    MinMod,
    VanLeer,
    Superbee,
}

impl FluxLimiter {
    /// Evaluates the symmetric TVD limiter $\phi(r)$ for consecutive gradient ratio $r$.
    #[must_use]
    pub fn evaluate(self, r: f32) -> f32 {
        if !r.is_finite() || r <= 0.0 {
            return 0.0;
        }
        match self {
            Self::MinMod => r.min(1.0),
            Self::VanLeer => (r + r.abs()) / (1.0 + r.abs()),
            Self::Superbee => (2.0 * r).min(1.0).max(r.min(2.0)),
        }
    }

    /// Reconstructs the limited slope between left difference `du_minus = u_i - u_{i-1}`
    /// and right difference `du_plus = u_{i+1} - u_i`.
    #[must_use]
    pub fn limited_slope(self, du_minus: f32, du_plus: f32) -> f32 {
        if du_minus * du_plus <= 0.0 || du_plus.abs() < 1e-12 {
            return 0.0;
        }
        let r = du_minus / du_plus;
        self.evaluate(r) * du_plus
    }
}

/// Advances a 2D surface foam concentration field (`[0, 1]`) by one semi-Lagrangian advection
/// + exponential dissipation + breaker injection step on a `size[0] x size[1]` grid.
#[must_use]
pub fn advect_foam_step(
    foam: &[f32],
    velocity: &[Vec2],
    injection: &[f32],
    size: [u32; 2],
    cell_size: f32,
    dt: f32,
    decay_rate: f32,
) -> Vec<f32> {
    let [w, h] = size;
    let total = (w * h) as usize;
    if w == 0
        || h == 0
        || foam.len() != total
        || velocity.len() != total
        || injection.len() != total
        || cell_size <= 0.0
        || dt < 0.0
    {
        return Vec::new();
    }
    let attenuation = (-decay_rate.max(0.0) * dt).exp();
    let inv_cell = 1.0 / cell_size;
    let mut next = Vec::with_capacity(total);
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let vel = velocity[idx];
            let back_pos = Vec2::new(x as f32, y as f32) - vel * (dt * inv_cell);
            let advected = bilinear(foam, size, back_pos);
            let updated = (advected * attenuation + injection[idx] * dt).clamp(0.0, 1.0);
            next.push(updated);
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A beach rising along +x: water for x < 50, land beyond.
    fn beach() -> ShoreField {
        let b = Bathymetry::from_fn(Vec2::ZERO, 1.0, [100, 20], |p| (p.x - 50.0) * 0.08);
        ShoreField::new(&b, 0.0)
    }

    #[test]
    fn distance_is_signed_from_the_waterline() {
        let s = beach();
        assert!((s.distance_at(Vec2::new(30.0, 10.0)) - 20.0).abs() < 1.0);
        assert!((s.distance_at(Vec2::new(60.0, 10.0)) + 10.0).abs() < 1.0);
        assert!((s.depth_at(Vec2::new(30.0, 10.0)) - 1.6).abs() < 0.01);
        assert!(s.shoreward(Vec2::new(30.0, 10.0)).x > 0.9);
    }

    #[test]
    fn breakers_grow_then_break_and_foam_at_the_swash() {
        let waves = ShoreWaves::default();
        let offshore = waves.height(4.0, 30.0, 0.0);
        assert!(offshore.foam < 0.2 && offshore.fft_scale == 1.0);
        let peak = (0..64)
            .map(|i| waves.height(0.5, 6.0, i as f32 * 0.1).height)
            .fold(0.0, f32::max);
        assert!(peak <= 0.78 * 0.5 + 1e-4 && peak > 0.1, "{peak}");
        assert!(waves.height(0.05, 0.3, 0.0).foam > 0.4);
    }

    #[test]
    fn tvd_flux_limiters_and_foam_advection_remain_bounded() {
        assert_eq!(FluxLimiter::MinMod.evaluate(-1.0), 0.0);
        assert!((FluxLimiter::VanLeer.evaluate(1.0) - 1.0).abs() < 1e-6);
        assert!((FluxLimiter::Superbee.evaluate(0.5) - 1.0).abs() < 1e-6);
        assert_eq!(FluxLimiter::VanLeer.limited_slope(1.0, -1.0), 0.0);

        let foam = vec![0.0, 1.0, 0.0, 0.0];
        let vel = vec![Vec2::new(1.0, 0.0); 4];
        let inj = vec![0.1; 4];
        let step = advect_foam_step(&foam, &vel, &inj, [4, 1], 1.0, 1.0, 0.5);
        assert_eq!(step.len(), 4);
        assert!(step.iter().all(|&f| (0.0..=1.0).contains(&f)));
        assert!(step[2] > 0.5);
    }
}
