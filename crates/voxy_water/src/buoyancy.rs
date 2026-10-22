//! Buoyancy from water samples: Archimedes force, drag relative to the moving water
//! (currents, waves, breakers) and the resulting torque, for any rigid body that can be
//! approximated by a set of volume samples.

use crate::bodies::WaterWorld;
use glam::{Quat, Vec3};

/// Density of fresh water (kg/m³).
pub const WATER_DENSITY: f32 = 1000.0;

/// A rigid body's buoyant volume as spheres in its local frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Floater {
    /// Local sample centres with the volume (m³) each represents.
    pub samples: Vec<(Vec3, f32)>,
    /// Linear drag coefficient per submerged m³ (N s/m per m³).
    pub drag: f32,
}

impl Floater {
    /// Box of `size` (m) sampled on an `n`³ lattice.
    pub fn cuboid(size: Vec3, n: u32) -> Self {
        let n = n.max(1);
        let volume = size.x * size.y * size.z / (n * n * n) as f32;
        let mut samples = Vec::with_capacity((n * n * n) as usize);
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let f = (Vec3::new(i as f32, j as f32, k as f32) + 0.5) / n as f32 - 0.5;
                    samples.push((f * size, volume));
                }
            }
        }
        Self {
            samples,
            drag: 600.0,
        }
    }

    pub fn volume(&self) -> f32 {
        self.samples.iter().map(|s| s.1).sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BuoyancyForces {
    /// World-space force (N).
    pub force: Vec3,
    /// World-space torque about the body origin (N m).
    pub torque: Vec3,
    /// Submerged fraction of the volume, 0..1.
    pub submerged: f32,
}

/// Forces on a body at `position`/`rotation` moving with `velocity` and
/// `angular_velocity` (world space), at simulation `time`.
pub fn buoyancy(
    world: &WaterWorld,
    floater: &Floater,
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    angular_velocity: Vec3,
    time: f32,
) -> BuoyancyForces {
    let mut out = BuoyancyForces::default();
    let total = floater.volume().max(1e-9);
    for &(local, volume) in &floater.samples {
        let offset = rotation * local;
        let point = position + offset;
        let Some(water) = world.sample(point, time) else {
            continue;
        };
        // a sample sphere of this volume is partly wet over its own diameter
        let radius = (volume * 3.0 / (4.0 * std::f32::consts::PI)).cbrt();
        let wet = ((water.submersion(point) + radius) / (2.0 * radius)).clamp(0.0, 1.0);
        if wet <= 0.0 {
            continue;
        }
        let displaced = volume * wet;
        let lift = water.normal.lerp(Vec3::Y, 0.5).normalize() * (WATER_DENSITY * 9.81 * displaced);
        let point_velocity = velocity + angular_velocity.cross(offset);
        let drag = (water.velocity - point_velocity) * (floater.drag * displaced);
        let force = lift + drag;
        out.force += force;
        out.torque += offset.cross(force);
        out.submerged += displaced / total;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bodies::WaterBody;
    use crate::query::WaveField;

    fn still_pool() -> WaterWorld {
        let mut world = WaterWorld::new(
            vec![WaterBody::pool(
                Vec3::new(-10., -3., -10.),
                Vec3::new(10., 0., 10.),
            )],
            crate::WaveSpectrum::pool(),
        );
        world.waves = WaveField::calm();
        world
    }

    /// A 0.5 m cube at half the density of water settles half submerged.
    #[test]
    fn crate_floats_at_its_density_ratio() {
        let world = still_pool();
        let floater = Floater::cuboid(Vec3::splat(0.5), 4);
        let mass = 0.5 * WATER_DENSITY * floater.volume();
        let (mut y, mut vy) = (1.0f32, 0.0f32);
        for _ in 0..4000 {
            let f = buoyancy(
                &world,
                &floater,
                Vec3::Y * y,
                Quat::IDENTITY,
                Vec3::Y * vy,
                Vec3::ZERO,
                0.0,
            );
            vy += (f.force.y / mass - 9.81) * 0.005;
            y += vy * 0.005;
        }
        assert!(y.abs() < 0.04, "settled at {y}");
        let f = buoyancy(
            &world,
            &floater,
            Vec3::Y * y,
            Quat::IDENTITY,
            Vec3::ZERO,
            Vec3::ZERO,
            0.0,
        );
        assert!((f.submerged - 0.5).abs() < 0.08, "{}", f.submerged);
    }

    #[test]
    fn tilted_box_is_righted_and_dry_box_feels_nothing() {
        let world = still_pool();
        let floater = Floater::cuboid(Vec3::new(1.0, 0.3, 0.5), 3);
        let tilt = Quat::from_rotation_z(0.4);
        let f = buoyancy(
            &world,
            &floater,
            Vec3::ZERO,
            tilt,
            Vec3::ZERO,
            Vec3::ZERO,
            0.0,
        );
        assert!(f.torque.z < 0.0, "{:?}", f.torque);
        let dry = buoyancy(
            &world,
            &floater,
            Vec3::Y * 5.0,
            Quat::IDENTITY,
            Vec3::ZERO,
            Vec3::ZERO,
            0.0,
        );
        assert_eq!(dry, BuoyancyForces::default());
    }
}
