//! Water bodies and the world that answers "is there water here, and how is it moving?".

use crate::query::{SurfaceSample, WaveField};
use crate::river::{River, RiverSample};
use crate::shore::{ShoreField, ShoreWaves};
use crate::spectrum::WaveSpectrum;
use glam::{Vec2, Vec3};

/// Optical and foam properties of the water itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterMaterial {
    /// Beer-Lambert absorption per metre (linear RGB).
    pub absorption: [f32; 3],
    /// Fraction of the ambient light scattered back towards the eye (water colour).
    pub scatter: [f32; 3],
    /// 0 disables caustics, 1 is physically scaled focusing.
    pub caustics: f32,
    /// Scale of white water from flow, breakers and falls.
    pub foam: f32,
}

impl WaterMaterial {
    /// Clear chlorinated pool water.
    pub const POOL: Self = Self { absorption: [0.42, 0.075, 0.035], scatter: [0.006, 0.030, 0.034], caustics: 1.0, foam: 0.0 };
    /// Green-blue lake water with plankton.
    pub const LAKE: Self = Self { absorption: [0.55, 0.16, 0.20], scatter: [0.010, 0.040, 0.030], caustics: 0.6, foam: 1.0 };
    /// Shallow river over stones: clear with a brown tint.
    pub const RIVER: Self = Self { absorption: [0.50, 0.22, 0.30], scatter: [0.025, 0.035, 0.022], caustics: 0.5, foam: 1.0 };
    /// Tropical sea over sand.
    pub const SEA: Self = Self { absorption: [0.45, 0.065, 0.045], scatter: [0.008, 0.045, 0.050], caustics: 0.8, foam: 1.0 };
}

#[derive(Clone, Debug, PartialEq)]
pub enum WaterShape {
    /// Axis-aligned basin interior from the floor corner `min` to `max`; `max.y` is the
    /// water level (pools, tanks, fountains).
    Basin { min: Vec3, max: Vec3 },
    /// A flat surface at `level` over a polygon in the xz plane; the terrain forms the
    /// shore (lakes, ponds, seas). `floor` bounds the volume for underwater checks.
    Region { outline: Vec<Vec2>, level: f32, floor: f32 },
    /// Flowing water along a spline.
    River(River),
}

/// A water body: shape, material, and which wave field and shoreline drive it.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterBody {
    pub shape: WaterShape,
    pub material: WaterMaterial,
    /// Scale of the vertical FFT displacement (1 = the field as simulated).
    pub waves: f32,
    /// Depth and distance to the waterline for breakers and shallow-water damping.
    pub shore: Option<ShoreField>,
    pub breakers: ShoreWaves,
}

impl WaterBody {
    pub fn pool(min: Vec3, max: Vec3) -> Self {
        Self { shape: WaterShape::Basin { min, max }, material: WaterMaterial::POOL, waves: 1.0, shore: None, breakers: ShoreWaves { height: 0.0, ..ShoreWaves::default() } }
    }

    pub fn lake(outline: Vec<Vec2>, level: f32, floor: f32, shore: Option<ShoreField>) -> Self {
        Self { shape: WaterShape::Region { outline, level, floor }, material: WaterMaterial::LAKE, waves: 1.0, shore, breakers: ShoreWaves { height: 0.12, period: 3.0, reach: 12.0 } }
    }

    pub fn sea(outline: Vec<Vec2>, level: f32, floor: f32, shore: ShoreField) -> Self {
        Self { shape: WaterShape::Region { outline, level, floor }, material: WaterMaterial::SEA, waves: 1.0, shore: Some(shore), breakers: ShoreWaves::default() }
    }

    pub fn river(river: River) -> Self {
        Self { shape: WaterShape::River(river), material: WaterMaterial::RIVER, waves: 0.4, shore: None, breakers: ShoreWaves { height: 0.0, ..ShoreWaves::default() } }
    }

    /// Axis-aligned bounds of the water volume (surface included).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        match &self.shape {
            WaterShape::Basin { min, max } => (*min, *max),
            WaterShape::Region { outline, level, floor } => {
                let (lo, hi) = outline.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
                (Vec3::new(lo.x, *floor, lo.y), Vec3::new(hi.x, *level, hi.y))
            }
            WaterShape::River(river) => river.nodes.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), n| {
                let r = Vec3::new(n.width * 0.5, 0.0, n.width * 0.5);
                (lo.min(n.position - r - Vec3::Y * n.depth), hi.max(n.position + r))
            }),
        }
    }
}

pub fn point_in_polygon(p: Vec2, outline: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = outline.len().wrapping_sub(1);
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Water at a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSample {
    pub body: usize,
    /// Height of the (wavy) surface above this point's xz.
    pub surface: f32,
    pub normal: Vec3,
    /// Velocity of the water at the surface: waves, current and breakers (m/s).
    pub velocity: Vec3,
    /// Still-water depth below the surface at this point.
    pub depth: f32,
    pub foam: f32,
}

impl WaterSample {
    /// How far `point` is below the surface (negative above it).
    pub fn submersion(&self, point: Vec3) -> f32 {
        self.surface - point.y
    }
}

/// All water bodies of a scene with their shared wave spectrum. Renderers simulate
/// `spectrum` on the GPU from seed 1; `waves` evaluates the same field on the CPU.
#[derive(Clone, Debug)]
pub struct WaterWorld {
    pub bodies: Vec<WaterBody>,
    pub spectrum: WaveSpectrum,
    pub waves: WaveField,
    river_samples: Vec<Vec<RiverSample>>,
}

/// River centre lines are resampled every this many metres.
pub const RIVER_STEP: f32 = 0.5;

impl WaterWorld {
    /// CPU queries keep this many of the most energetic wave components.
    pub const QUERY_COMPONENTS: usize = 192;

    pub fn new(bodies: Vec<WaterBody>, spectrum: WaveSpectrum) -> Self {
        let waves = WaveField::new(&spectrum, 1, Self::QUERY_COMPONENTS);
        let river_samples = bodies
            .iter()
            .map(|b| match &b.shape {
                WaterShape::River(r) => r.samples(RIVER_STEP),
                _ => Vec::new(),
            })
            .collect();
        Self { bodies, spectrum, waves, river_samples }
    }

    pub fn river_samples(&self, body: usize) -> &[RiverSample] {
        &self.river_samples[body]
    }

    /// The body whose volume holds `point` (or whose surface lies above it), if any.
    pub fn sample(&self, point: Vec3, time: f32) -> Option<WaterSample> {
        let xz = Vec2::new(point.x, point.z);
        for (index, body) in self.bodies.iter().enumerate() {
            let (level, floor, current, foam) = match &body.shape {
                WaterShape::Basin { min, max } => {
                    if xz.x < min.x || xz.x > max.x || xz.y < min.z || xz.y > max.z {
                        continue;
                    }
                    (max.y, min.y, Vec3::ZERO, 0.0)
                }
                WaterShape::Region { outline, level, floor } => {
                    if !point_in_polygon(xz, outline) {
                        continue;
                    }
                    (*level, *floor, Vec3::ZERO, 0.0)
                }
                WaterShape::River(river) => match river.sample(point, &self.river_samples[index]) {
                    Some(r) => (r.level, r.level - r.depth, r.velocity, r.foam),
                    None => continue,
                },
            };
            if point.y < floor - 0.5 {
                continue;
            }
            let mut depth = level - floor;
            let mut fft = body.waves;
            let mut wave = SurfaceSample::FLAT;
            let mut breaker_height = 0.0;
            let mut breaker_velocity = Vec3::ZERO;
            let mut foam = foam * body.material.foam;
            if let Some(shore) = &body.shore {
                depth = shore.depth_at(xz);
                if depth <= 0.0 {
                    continue;
                }
                let b = body.breakers.height(depth, shore.distance_at(xz), time);
                fft *= b.fft_scale;
                breaker_height = b.height;
                let dir = shore.shoreward(xz);
                // crests travel shoreward at the shallow-water celerity
                breaker_velocity = Vec3::new(dir.x, 0.0, dir.y) * (9.81 * depth.max(0.05)).sqrt() * b.foam.min(1.0);
                foam = foam.max(b.foam * body.material.foam);
            }
            if fft > 0.0 {
                let mut flow_xz = xz;
                if current.length_squared() > 0.0 {
                    // the wave pattern is carried downstream with the current
                    flow_xz -= Vec2::new(current.x, current.z) * (time % 8.0);
                }
                wave = self.waves.sample(flow_xz, time);
            }
            let surface = level + wave.height * fft + breaker_height;
            let normal = Vec3::new(wave.normal.x * fft, wave.normal.y, wave.normal.z * fft).normalize();
            return Some(WaterSample {
                body: index,
                surface,
                normal,
                velocity: wave.velocity * fft + current + breaker_velocity,
                depth,
                foam,
            });
        }
        None
    }

    /// True when `point` is under the surface of some body.
    pub fn is_submerged(&self, point: Vec3, time: f32) -> bool {
        self.sample(point, time).is_some_and(|s| s.submersion(point) > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::river::RiverNode;
    use crate::shore::Bathymetry;

    #[test]
    fn pool_lake_and_river_answer_their_own_points() {
        let shore = ShoreField::new(&Bathymetry::from_fn(Vec2::new(-50., -50.), 1.0, [100, 100], |p| p.length() * 0.1 - 3.0), 0.0);
        let node = |z: f32| RiverNode { position: Vec3::new(80., 2.0 - z * 0.01, z), width: 6.0, depth: 1.0 };
        let world = WaterWorld::new(
            vec![
                WaterBody::pool(Vec3::new(-2., -1.5, 100.), Vec3::new(2., 0.1, 110.)),
                WaterBody::lake(vec![Vec2::new(-40., -40.), Vec2::new(40., -40.), Vec2::new(40., 40.), Vec2::new(-40., 40.)], 0.0, -5.0, Some(shore)),
                WaterBody::river(River::new((0..5).map(|i| node(i as f32 * 10.)).collect(), 1.5)),
            ],
            WaveSpectrum::pond(),
        );
        let pool = world.sample(Vec3::new(0., -1., 105.), 1.0).unwrap();
        assert_eq!(pool.body, 0);
        assert!((pool.surface - 0.1).abs() < 0.05 && pool.depth > 1.5);
        let lake = world.sample(Vec3::new(5., -0.5, 5.), 1.0).unwrap();
        assert_eq!(lake.body, 1);
        assert!((lake.depth - 2.3).abs() < 0.1);
        // the lake's terrain rises above the water beyond r = 30 m: dry land
        assert!(world.sample(Vec3::new(36., 0.5, 0.), 1.0).is_none());
        let river = world.sample(Vec3::new(80., 1.5, 15.), 1.0).unwrap();
        assert_eq!(river.body, 2);
        assert!(river.velocity.z > 1.0);
        assert!(world.is_submerged(Vec3::new(80., 1.0, 15.), 1.0));
        assert!(!world.is_submerged(Vec3::new(80., 3.0, 15.), 1.0));
    }
}
