//! Authored companion surfaces, native quadruped rigging and inertial fur guides.
mod asset;
mod collision;
mod fur;
use glam::{Mat4, Quat, Vec3};
use physics_voxel::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState};
use std::sync::Arc;
use voxy_render::SkinnedVertex;
use voxy_world::{VoxelPos, World};
pub const JOINTS: usize = fur::PALETTE * 2;
#[derive(Clone, Debug)]
pub struct Companion {
    state: CharacterState,
    cat: bool,
    yaw: f32,
    phase: f32,
    speed: f32,
    pub travelled_distance: f32,
    coat: fur::CoatDynamics,
}
impl Companion {
    pub fn spawn(spawn: VoxelPos, cat: bool) -> Self {
        let radius = if cat { 0.42 } else { 0.85 };
        Self {
            state: CharacterState {
                body: AnchoredAabb {
                    anchor: spawn,
                    min: [-radius, 0.0, -radius],
                    max: [radius, if cat { 0.76 } else { 0.94 }, radius],
                },
                velocity: [0.0; 3],
                grounded: false,
            },
            cat,
            yaw: 0.0,
            phase: 0.0,
            speed: 0.0,
            travelled_distance: 0.0,
            coat: fur::CoatDynamics::new(),
        }
    }
    pub fn position(&self) -> Vec3 {
        super::character_position(&self.state)
    }
    pub fn shadow_spheres(&self) -> [[f32; 4]; 6] {
        let pose = self.bones();
        let mut spheres = [[0.0; 4]; 6];
        let torso =
            pose[0].transform_point3(Vec3::new(0.0, if self.cat { 0.29 } else { 0.46 }, 0.0));
        spheres[0] = [
            torso.x,
            torso.y,
            torso.z,
            if self.cat { 0.11 } else { 0.20 },
        ];
        let head = pose[0].transform_point3(Vec3::new(
            0.0,
            if self.cat { 0.40 } else { 0.75 },
            if self.cat { 0.27 } else { 0.49 },
        ));
        spheres[1] = [head.x, head.y, head.z, if self.cat { 0.07 } else { 0.10 }];
        for (leg, hip) in leg_pivots(self.cat).into_iter().enumerate() {
            let foot = pose[leg + 5].transform_point3(Vec3::new(
                hip.x,
                if self.cat { 0.021 } else { 0.028 },
                hip.z + 0.025,
            ));
            spheres[leg + 2] = [foot.x, foot.y, foot.z, if self.cat { 0.022 } else { 0.034 }];
        }
        spheres
    }
    pub fn forward(&self) -> Vec3 {
        Quat::from_rotation_y(self.yaw) * Vec3::Z
    }
    pub fn grounded(&self) -> bool {
        self.state.grounded
    }
    pub fn fur_particles(&self) -> usize {
        self.coat.strands.len() * 3
    }
    pub fn tick(&mut self, world: &World, target: Vec3, dt: f32) -> Result<(), String> {
        self.tick_with_bodies(world, target, dt, &[])
    }
    pub fn body(&self) -> AnchoredAabb {
        self.state.body
    }
    pub fn tick_with_bodies(
        &mut self,
        world: &World,
        target: Vec3,
        dt: f32,
        bodies: &[AnchoredAabb],
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate.tick_inner(world, target, dt, bodies)?;
        *self = candidate;
        Ok(())
    }
    fn tick_inner(
        &mut self,
        world: &World,
        target: Vec3,
        dt: f32,
        bodies: &[AnchoredAabb],
    ) -> Result<(), String> {
        if !dt.is_finite() || dt <= 0.0 || dt > 1.0 / 30.0 || !target.is_finite() {
            return Err("invalid animal tick".into());
        }
        let delta = (target - self.position()) * Vec3::new(1.0, 0.0, 1.0);
        let distance = delta.length();
        let stop = if self.cat { 2.5 } else { 1.8 };
        let speed = (distance - stop).clamp(0.0, if self.cat { 2.0 } else { 2.6 });
        let velocity = delta.normalize_or_zero() * speed;
        let before = self.position();
        let mut candidate = self.state;
        let mut native = physics::CharacterState {
            body: physics::AnchoredAabb {
                anchor: physics::Origin {
                    x: candidate.body.anchor.x,
                    y: candidate.body.anchor.y,
                    z: candidate.body.anchor.z,
                },
                min: candidate.body.min,
                max: candidate.body.max,
            },
            velocity: candidate.velocity,
            grounded: candidate.grounded,
        };
        physics::step_character(
            &collision::AnimalWorld { world, bodies },
            &mut native,
            CharacterInput {
                planar_velocity: [f64::from(velocity.x), f64::from(velocity.z)],
                jump_pressed: false,
            },
            f64::from(dt),
            CharacterConfig {
                step_height: if self.cat { 0.35 } else { 0.55 },
                ..CharacterConfig::default()
            },
        )
        .map_err(|e| e.to_string())?;
        candidate.body = AnchoredAabb {
            anchor: VoxelPos {
                x: native.body.anchor.x,
                y: native.body.anchor.y,
                z: native.body.anchor.z,
            },
            min: native.body.min,
            max: native.body.max,
        };
        candidate.velocity = native.velocity;
        candidate.grounded = native.grounded;
        self.state = candidate;
        let travelled = (self.position() - before) * Vec3::new(1.0, 0.0, 1.0);
        self.travelled_distance += travelled.length();
        self.speed = travelled.length() / dt;
        if self.speed > 0.01 {
            let desired = travelled.x.atan2(travelled.z);
            let angle = (desired - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += angle.clamp(-dt * 4.0, dt * 4.0);
            self.phase = (self.phase + travelled.length() * if self.cat { 13.0 } else { 9.0 })
                % std::f32::consts::TAU;
        }
        let groom = fur::groom(self.cat)?;
        let pose = self.bones();
        self.coat.step(&groom, &pose, dt)?;
        Ok(())
    }
    fn bones(&self) -> [Mat4; fur::BONES] {
        let root =
            Mat4::from_rotation_translation(Quat::from_rotation_y(self.yaw), self.position());
        let mut result = [root; fur::BONES];
        let amplitude = (self.speed / 1.2).min(1.0) * if self.state.grounded { 1.0 } else { 0.15 };
        for (leg, hip) in leg_pivots(self.cat).into_iter().enumerate() {
            let phase = self.phase
                + if leg == 0 || leg == 3 {
                    0.0
                } else {
                    std::f32::consts::PI
                };
            let swing = phase.sin() * 0.38 * amplitude;
            let bend = phase.cos().max(0.0) * 0.65 * amplitude;
            result[leg + 1] = root
                * Mat4::from_translation(hip)
                * Mat4::from_rotation_x(swing)
                * Mat4::from_translation(-hip);
            let knee = Vec3::new(hip.x, hip.y * 0.52, hip.z);
            result[leg + 5] = result[leg + 1]
                * Mat4::from_translation(knee)
                * Mat4::from_rotation_x(-bend)
                * Mat4::from_translation(-knee);
        }
        let pivot = Vec3::new(
            0.0,
            if self.cat { 0.34 } else { 0.48 },
            if self.cat { -0.25 } else { -0.5 },
        );
        result[9] = root
            * Mat4::from_translation(pivot)
            * Mat4::from_rotation_y(self.phase.sin() * 0.18 * amplitude)
            * Mat4::from_translation(-pivot);
        result
    }
    pub fn palette(&self, actor_inverse: Mat4) -> Result<Vec<Mat4>, String> {
        let groom = fur::groom(self.cat)?;
        Ok(self
            .coat
            .matrices(&groom, &self.bones())
            .into_iter()
            .map(|matrix| actor_inverse * matrix)
            .collect())
    }
}
fn leg_pivots(cat: bool) -> [Vec3; 4] {
    let (x, y, front, back) = if cat {
        (0.072, 0.29, 0.205, -0.205)
    } else {
        (0.125, 0.49, 0.36, -0.31)
    };
    [
        Vec3::new(-x, y, front),
        Vec3::new(x, y, front),
        Vec3::new(-x, y, back),
        Vec3::new(x, y, back),
    ]
}
pub fn append_mesh(
    vertices: &mut Vec<SkinnedVertex>,
    indices: &mut Vec<u32>,
    cat: bool,
    base: u16,
) -> Result<(), String> {
    let surface = asset::surface(cat)?;
    let groom: Arc<fur::Groom> = fur::groom(cat)?;
    for (v, i) in [
        (&surface.vertices, &surface.indices),
        (&groom.vertices, &groom.indices),
    ] {
        let offset = vertices.len() as u32;
        vertices.extend(v.iter().map(|vertex| {
            let mut vertex = *vertex;
            for j in &mut vertex.joints {
                *j += base;
            }
            vertex
        }));
        indices.extend(i.iter().map(|i| i + offset));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_species_have_valid_rigs_and_dense_fur() {
        for cat in [false, true] {
            let (mut vertices, mut indices) = (Vec::new(), Vec::new());
            append_mesh(&mut vertices, &mut indices, cat, 5).unwrap();
            assert!(vertices.len() > 100000);
            let mesh =
                voxy_render::SkinnedMesh::new(vertices, indices, 5 + fur::PALETTE as u16).unwrap();
            let animal = Companion::spawn(
                VoxelPos {
                    x: 16,
                    y: 10,
                    z: 16,
                },
                cat,
            );
            let palette = animal.palette(Mat4::IDENTITY).unwrap();
            mesh.validate_render_pose(&[vec![Mat4::IDENTITY; 5], palette].concat(), Mat4::IDENTITY)
                .unwrap();
        }
    }
    #[test]
    fn companions_land_follow_and_reject_invalid_ticks() {
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        for cat in [false, true] {
            let mut animal = Companion::spawn(spawn, cat);
            let target = animal.position() + Vec3::X * 6.0;
            let initial = (target - animal.position()).length();
            for _ in 0..120 {
                animal.tick(&scene.world, target, 1.0 / 60.0).unwrap();
            }
            assert!(animal.state.grounded);
            assert!((target - animal.position()).length() < initial);
            assert_eq!(animal.fur_particles(), fur::GUIDES * 3);
            let before = animal.state;
            assert!(animal.tick(&scene.world, target, f32::NAN).is_err());
            assert_eq!(animal.state, before);
        }
    }
    #[test]
    fn companion_sweep_stops_at_another_body() {
        use physics::CollisionWorld;
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let anchor = super::super::terrain_spawn(&scene.world).unwrap();
        let obstacles = [AnchoredAabb {
            anchor,
            min: [1.0, 0.0, -2.0],
            max: [1.2, 6.0, 2.0],
        }];
        let world = collision::AnimalWorld {
            world: &scene.world,
            bodies: &obstacles,
        };
        let hit = world
            .sweep_aabb(
                physics::AnchoredAabb {
                    anchor: physics::Origin {
                        x: anchor.x,
                        y: anchor.y,
                        z: anchor.z,
                    },
                    min: [-0.2, 2.0, -0.2],
                    max: [0.2, 2.5, 0.2],
                },
                [3.0, 0.0, 0.0],
                16384,
            )
            .unwrap();
        assert!((hit.fraction - 0.8 / 3.0).abs() < 1e-10);
        assert_eq!(hit.normal, [-1, 0, 0]);
    }
    #[test]
    fn guides_remain_attached_and_bounded_during_movement() {
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        for cat in [false, true] {
            let mut animal = Companion::spawn(spawn, cat);
            let target = animal.position() + Vec3::Z * 5.0;
            for _ in 0..100 {
                animal.tick(&scene.world, target, 1.0 / 60.0).unwrap();
            }
            let groom = fur::groom(cat).unwrap();
            let pose = animal.bones();
            for (strand, root) in animal.coat.strands.iter().zip(&groom.roots) {
                let expected = pose[root.bone].transform_point3(root.position).as_dvec3();
                assert!(
                    (glam::DVec3::from_array(strand.positions()[0]) - expected).length() < 1e-8
                );
                for pair in strand.positions().windows(2) {
                    assert!(
                        (glam::DVec3::from_array(pair[0]) - glam::DVec3::from_array(pair[1]))
                            .length()
                            < f64::from(root.length) * 0.8
                    );
                }
            }
        }
    }
}

pub fn check_requested() -> bool {
    std::env::args().any(|arg| arg == "--animals-check")
        || std::env::current_exe().ok().is_some_and(|path| {
            path.file_stem()
                .is_some_and(|name| name == "voxy-animals-check")
        })
}

/// Dry, level in-world inspection pad. Ordinary play keeps its generated terrain.
pub fn prepare_check_scene(
    scene: &mut voxy_runtime::BootstrapScene,
) -> Result<(), voxy_runtime::BootstrapError> {
    let grass = scene
        .world
        .registry()
        .find(&voxy_world::ResourceKey::parse("voxy:grass")?)
        .ok_or(voxy_runtime::BootstrapError::MissingBuiltin)?;
    let mut writes = Vec::new();
    for x in 6..=28 {
        for z in 6..=30 {
            for y in 12..=28 {
                writes.push(voxy_world::VoxelWrite {
                    pos: VoxelPos { x, y, z },
                    block: if y == 12 {
                        grass
                    } else {
                        voxy_world::BlockStateId::AIR
                    },
                });
            }
        }
    }
    scene.world.commit(voxy_world::EditTxn {
        source: voxy_world::EditSource::Simulation,
        expected: Vec::new(),
        writes,
    })?;
    let positions: Vec<_> = scene.chunks.iter().map(|chunk| chunk.pos).collect();
    scene.chunks = voxy_runtime::rebuild_bootstrap_chunks(&scene.world, &positions, 1)?;
    Ok(())
}

pub fn hold_check_window() -> bool {
    check_requested() && !std::env::args().any(|arg| arg == "--animals-check")
}
