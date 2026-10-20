//! Authored companion surfaces, native quadruped rigging and inertial fur guides.
#[path = "animals/breed_asset.rs"]
mod asset;
mod collision;
mod fur;
mod gait;
use glam::{Mat4, Quat, Vec3};
use physics_voxel::{AnchoredAabb, CharacterConfig, CharacterInput, CharacterState};
use std::sync::Arc;
use voxy_render::SkinnedVertex;
use voxy_world::{VoxelPos, World};
pub const JOINTS: usize = fur::PALETTE * 2;
#[derive(Debug)]
pub struct Companion {
    state: CharacterState,
    cat: bool,
    yaw: f32,
    phase: f32,
    speed: f32,
    pub travelled_distance: f32,
    coat: fur::CoatDynamics,
    gait: gait::Gait,
    // Reusable candidate for atomic ticks. Its own transaction slot stays empty.
    transaction: Option<Box<Companion>>,
}
impl Clone for Companion {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            cat: self.cat,
            yaw: self.yaw,
            phase: self.phase,
            speed: self.speed,
            travelled_distance: self.travelled_distance,
            coat: self.coat.clone(),
            gait: self.gait.clone(),
            transaction: None,
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.state = source.state;
        self.cat = source.cat;
        self.yaw = source.yaw;
        self.phase = source.phase;
        self.speed = source.speed;
        self.travelled_distance = source.travelled_distance;
        self.coat.clone_from(&source.coat);
        self.gait.clone_from(&source.gait);
        self.transaction = None;
    }
}
impl Companion {
    pub fn spawn(spawn: VoxelPos, cat: bool) -> Self {
        let a = asset::profile(cat);
        let radius = f64::from(a.trunk_length * 0.5 + a.neck_length * 0.48 + a.head_length);
        Self {
            state: CharacterState {
                body: AnchoredAabb {
                    anchor: spawn,
                    min: [-radius, 0.0, -radius],
                    max: [
                        radius,
                        f64::from(a.shoulder_height + a.neck_length + a.head_length * 0.5),
                        radius,
                    ],
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
            gait: gait::Gait::new(),
            transaction: None,
        }
    }
    pub fn position(&self) -> Vec3 {
        super::character_position(&self.state)
    }
    pub fn shadow_spheres(&self) -> [[f32; 4]; 6] {
        let pose = self.bones();
        let mut spheres = [[0.0; 4]; 6];
        let a = asset::profile(self.cat);
        let torso =
            pose[0].transform_point3(Vec3::new(0.0, a.shoulder_height - a.rib_depth * 0.44, 0.0));
        spheres[0] = [torso.x, torso.y, torso.z, a.rib_half_width * 0.90];
        let head = pose[0].transform_point3(Vec3::new(
            0.0,
            a.shoulder_height + a.neck_length * 0.55,
            a.trunk_length * 0.36 + a.neck_length * 0.48 + a.head_length * 0.35,
        ));
        spheres[1] = [head.x, head.y, head.z, a.skull_width * 0.5];
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
    pub fn breed_id(&self) -> &'static str {
        asset::breed(self.cat).id()
    }
    pub fn fur_tip_displacement(&self) -> Result<f32, String> {
        let groom = fur::groom(self.cat)?;
        let pose = self.bones();
        Ok(self
            .coat
            .strands
            .iter()
            .zip(&groom.roots)
            .map(|(strand, root)| {
                let rest =
                    pose[root.bone].transform_point3(root.rest_point(1.0));
                Vec3::from_array(strand.positions()[2].map(|x| x as f32)).distance(rest)
            })
            .fold(0.0, f32::max))
    }
    pub fn inspection_paw(&self) -> (Vec3,f32) {
        let a=asset::profile(self.cat);let hip=leg_pivots(self.cat)[1];
        (self.bones()[6].transform_point3(Vec3::new(hip.x,a.paw_half_width*0.6,
            hip.z+a.paw_half_width)),a.paw_half_width*7.)
    }
    pub fn inspection_face(&self) -> (Vec3, f32) {
        let a=asset::profile(self.cat);
        let skull=a.head_length-a.muzzle_length;
        (self.position()+Vec3::Y*(a.head_origin().y+skull*0.1)
            +self.forward()*(a.head_origin().z+a.head_length*0.45),a.skull_width*3.5)
    }
    pub fn inspection_center(&self) -> Vec3 {
        let a = asset::profile(self.cat);
        let skull = a.head_length - a.muzzle_length;
        let ear = match a.ears {
            voxy_model::animals::Ear::Upright { height } => height,
            _ => 0.0,
        };
        self.position()
            + Vec3::Y * (a.head_origin().y + skull * 0.35 + ear) * 0.48
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
        self.transact(|candidate| candidate.tick_inner(world, target, dt, bodies))
    }
    fn transact(
        &mut self,
        update: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut candidate = if let Some(mut candidate) = self.transaction.take() {
            candidate.as_mut().clone_from(self);
            candidate
        } else {
            Box::new(self.clone())
        };
        let result = update(&mut candidate);
        if result.is_ok() {
            std::mem::swap(self, &mut *candidate);
        }
        self.transaction = Some(candidate);
        result
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
        let root = Mat4::from_rotation_translation(
            Quat::from_rotation_y(self.yaw),
            self.position()
                - Vec3::Y
                    * asset::profile(self.cat).shoulder_height
                    * 0.04
                    * (self.speed / 1.2).min(1.0)
                    * if self.state.grounded { 1.0 } else { 0.0 },
        );
        self.gait.update(
            world,
            self.cat,
            root,
            self.phase,
            self.speed,
            self.state.grounded,
        )?;
        let groom = fur::groom(self.cat)?;
        let pose = self.bones();
        let boxes = fur::environment_boxes(world, bodies, &groom, &pose)?;
        self.coat.step_with_boxes(&groom, &pose, dt, &boxes)?;
        Ok(())
    }
    fn bones(&self) -> [Mat4; fur::BONES] {
        let root = Mat4::from_rotation_translation(
            Quat::from_rotation_y(self.yaw),
            self.position()
                - Vec3::Y
                    * asset::profile(self.cat).shoulder_height
                    * 0.04
                    * (self.speed / 1.2).min(1.0)
                    * if self.state.grounded { 1.0 } else { 0.0 },
        );
        let mut result = self.gait.matrices(self.cat, root);
        let amplitude = (self.speed / 1.2).min(1.0) * if self.state.grounded { 1.0 } else { 0.15 };
        let pivot = Vec3::new(
            0.0,
            asset::profile(self.cat).rump_height - asset::profile(self.cat).rib_depth * 0.12,
            -asset::profile(self.cat).trunk_length * 0.43,
        );
        result[9] = root
            * Mat4::from_translation(pivot)
            * Mat4::from_rotation_y(self.phase.sin() * 0.18 * amplitude)
            * Mat4::from_translation(-pivot);
        result
    }
    /// Append actor-space joints without allocating an intermediate palette.
    /// Resolving the groom can fail; in that case the output remains unchanged.
    pub fn append_palette(&self, actor_inverse: Mat4, out: &mut Vec<Mat4>) -> Result<(), String> {
        let groom = fur::groom(self.cat)?;
        self.coat
            .append_matrices(&groom, &self.bones(), actor_inverse, out);
        Ok(())
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
    let anatomy = asset::profile(cat);
    let (x, y, front, back) = (
        anatomy.rib_half_width * 0.68,
        anatomy.shoulder_height - anatomy.rib_depth * 0.38,
        anatomy.trunk_length * 0.36,
        -anatomy.trunk_length * 0.36,
    );
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
    asset::validate_selection()?;
    let surface = asset::surface(cat)?;
    let groom: Arc<fur::Groom> = fur::groom(cat)?;
    let clay = std::env::args().any(|arg| arg == "--animal-clay");
    for (part, (v, i)) in [
        (&surface.vertices, &surface.indices),
        (&groom.vertices, &groom.indices),
    ]
    .into_iter()
    .enumerate()
    {
        if clay && part == 1 {
            continue;
        }

        let offset = vertices.len() as u32;
        vertices.extend(v.iter().enumerate().map(|(index, vertex)| {
            let mut vertex = *vertex;
            if clay && part == 0 && surface.fur[index] {
                vertex.uv = asset::encoded_color(Vec3::splat(0.32), 0.99);
            }

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
    fn appended_palette_matches_legacy_bits_and_preserves_prefix() {
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        for cat in [false, true] {
            let mut animal = Companion::spawn(spawn, cat);
            for tick in 0..12 {
                if tick > 0 {
                    animal
                        .tick(&scene.world, animal.position() + Vec3::X * 6., 1. / 60.)
                        .unwrap();
                }
                let inverse = Mat4::from_rotation_translation(
                    Quat::from_rotation_y(0.37),
                    Vec3::new(2., 3., 4.),
                )
                .inverse();
                let expected = animal.palette(inverse).unwrap();
                let mut actual = vec![Mat4::IDENTITY; 3];
                animal.append_palette(inverse, &mut actual).unwrap();
                assert_eq!(&actual[..3], &[Mat4::IDENTITY; 3]);
                assert_eq!(
                    actual[3..]
                        .iter()
                        .map(|m| m.to_cols_array().map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    expected
                        .iter()
                        .map(|m| m.to_cols_array().map(f32::to_bits))
                        .collect::<Vec<_>>()
                );
                let pointer = actual.as_ptr();
                actual.truncate(3);
                animal.append_palette(inverse, &mut actual).unwrap();
                assert_eq!(actual.as_ptr(), pointer);
            }
        }
    }
    #[test]
    #[ignore = "manual CPU paired palette benchmark"]
    fn paired_palette_cpu_bench() {
        use std::{hint::black_box, time::Instant};
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        let mut animals = vec![
            Companion::spawn(spawn, false),
            Companion::spawn(spawn, true),
        ];
        for animal in &mut animals {
            for _ in 0..20 {
                animal
                    .tick(&scene.world, animal.position() + Vec3::X * 6., 1. / 60.)
                    .unwrap();
            }
        }
        let prefix = vec![Mat4::IDENTITY; 5];
        let model =
            Mat4::from_rotation_translation(Quat::from_rotation_y(0.37), Vec3::new(2., 3., 4.));
        for count in [1, 2] {
            let mut expected = prefix.clone();
            for animal in &animals[..count] {
                expected.extend(animal.palette(model.inverse()).unwrap());
            }
            for round in 1..=5 {
                for (variant, reference) in if round % 2 == 1 {
                    [("before", true), ("after", false)]
                } else {
                    [("after", false), ("before", true)]
                } {
                    let mut buffer = Vec::new();
                    let mut samples = Vec::new();
                    for sample in 0..34 {
                        let start = Instant::now();
                        for _ in 0..64 {
                            if reference {
                                let mut palette = prefix.clone();
                                for animal in &animals[..count] {
                                    palette.extend(animal.palette(model.inverse()).unwrap());
                                }
                                black_box(&palette);
                            } else {
                                buffer.clear();
                                buffer.extend_from_slice(&prefix);
                                let inverse = model.inverse();
                                for animal in &animals[..count] {
                                    animal.append_palette(inverse, &mut buffer).unwrap();
                                }
                                black_box(&buffer);
                            }
                        }
                        if sample >= 3 {
                            samples.push(start.elapsed().as_secs_f64() * 1e6 / 64.);
                        }
                    }
                    if !reference {
                        assert_eq!(
                            bytemuck::cast_slice::<Mat4, u8>(&buffer),
                            bytemuck::cast_slice::<Mat4, u8>(&expected)
                        );
                    }
                    samples.sort_by(f64::total_cmp);
                    println!("PALETTE_PAIR,{round},{variant},{count},{:.6}", samples[15]);
                }
            }
        }
    }

    fn assert_same_simulation(a: &Companion, b: &Companion) {
        assert_eq!(a.state, b.state);
        assert_eq!(
            [a.yaw, a.phase, a.speed, a.travelled_distance].map(f32::to_bits),
            [b.yaw, b.phase, b.speed, b.travelled_distance].map(f32::to_bits)
        );
        assert_eq!(
            a.palette(Mat4::IDENTITY).unwrap(),
            b.palette(Mat4::IDENTITY).unwrap()
        );
        assert_eq!(format!("{:?}", a.gait), format!("{:?}", b.gait));
        for (a, b) in a.coat.strands.iter().zip(&b.coat.strands) {
            assert_eq!(a.positions(), b.positions());
        }
        assert_eq!(a.coat.strands.len(), b.coat.strands.len());
    }
    #[test]
    fn reusable_transaction_matches_clone_per_tick_and_recovers_after_late_error() {
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        for cat in [false, true] {
            let mut animal = Companion::spawn(spawn, cat);
            let mut reference = animal.clone();
            for tick in 0..40 {
                let target =
                    animal.position() + Vec3::new(if tick < 20 { 6.0 } else { -6.0 }, 0., 2.);
                let mut candidate = reference.clone();
                candidate
                    .tick_inner(&scene.world, target, 1. / 60., &[])
                    .unwrap();
                reference = candidate;
                animal.tick(&scene.world, target, 1. / 60.).unwrap();
                assert_same_simulation(&animal, &reference);
                assert!(animal.transaction.as_ref().unwrap().transaction.is_none());
            }
            let before = animal.clone();
            assert!(before.transaction.is_none());
            let failure = animal.transact(|candidate| {
                candidate.phase += 0.5;
                let root = candidate.coat.strands[0].positions()[0];
                candidate.coat.strands[0]
                    .step(1. / 120., root, [0., -9.81, 0.], &[])
                    .unwrap();
                Err("injected late failure".into())
            });
            assert_eq!(failure, Err("injected late failure".into()));
            assert_same_simulation(&animal, &before);
            for _ in 0..5 {
                let target = animal.position() + Vec3::Z * 5.;
                reference
                    .tick_inner(&scene.world, target, 1. / 60., &[])
                    .unwrap();
                animal.tick(&scene.world, target, 1. / 60.).unwrap();
                assert_same_simulation(&animal, &reference);
            }
            assert!(animal.tick(&scene.world, Vec3::ZERO, f32::NAN).is_err());
            assert_same_simulation(&animal, &reference);
        }
    }

    /// Pair the original transaction wrapper and reusable wrapper on one CPU thread.
    #[test]
    #[ignore = "manual same-process paired CPU benchmark"]
    fn paired_transaction_cpu_bench() {
        use std::{hint::black_box, time::Instant};
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        for cat in [false, true] {
            for round in 1..=5 {
                for (variant, reference) in [("before", true), ("after", false)] {
                    let mut animal = Companion::spawn(spawn, cat);
                    let mut samples = Vec::new();
                    for tick in 0..220 {
                        let target = animal.position()
                            + Vec3::new(if tick % 120 < 60 { 6.0 } else { -6.0 }, 0., 2.);
                        let start = Instant::now();
                        if reference {
                            let mut candidate = animal.clone();
                            candidate
                                .tick_inner(
                                    black_box(&scene.world),
                                    black_box(target),
                                    1. / 60.,
                                    &[],
                                )
                                .unwrap();
                            animal = candidate;
                        } else {
                            animal
                                .tick(black_box(&scene.world), black_box(target), 1. / 60.)
                                .unwrap();
                        }
                        if tick >= 20 {
                            samples.push(start.elapsed().as_secs_f64() * 1e6);
                        }
                    }
                    samples.sort_by(f64::total_cmp);
                    let mut checksum = 0u64;
                    for value in animal
                        .palette(Mat4::IDENTITY)
                        .unwrap()
                        .iter()
                        .flat_map(|m| m.to_cols_array())
                    {
                        checksum = checksum.rotate_left(7) ^ u64::from(value.to_bits());
                    }
                    for value in [animal.phase, animal.speed, animal.travelled_distance] {
                        checksum = checksum.rotate_left(7) ^ u64::from(value.to_bits());
                    }
                    println!(
                        "TICK_PAIR,{round},{variant},{cat},{:.3},{:.3},{checksum:016x}",
                        samples[100], samples[197]
                    );
                }
            }
        }
    }

    /// CPU-only real companion workload; initialization is outside timed samples.
    #[test]
    #[ignore = "manual paired CPU benchmark"]
    fn cpu_companion_tick_and_pose_bench() {
        use std::{hint::black_box, time::Instant};
        let scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        let spawn = super::super::terrain_spawn(&scene.world).unwrap();
        let reference_tick = std::env::var_os("VOXY_CPU_REFERENCE_TICK").is_some();
        let mut animals = Vec::new();
        for cat in [false, true] {
            let mut animal = Companion::spawn(spawn, cat);
            let mut samples = Vec::new();
            for tick in 0..220 {
                let target = animal.position()
                    + Vec3::new(if tick % 120 < 60 { 6.0 } else { -6.0 }, 0.0, 2.0);
                let start = Instant::now();
                if reference_tick {
                    let mut candidate = animal.clone();
                    candidate
                        .tick_inner(black_box(&scene.world), black_box(target), 1.0 / 60.0, &[])
                        .unwrap();
                    animal = candidate;
                } else {
                    animal
                        .tick(black_box(&scene.world), black_box(target), 1.0 / 60.0)
                        .unwrap();
                }
                if tick >= 20 {
                    samples.push(start.elapsed().as_secs_f64() * 1e6);
                }
            }
            samples.sort_by(f64::total_cmp);
            let mut checksum = 0u64;
            for value in animal
                .palette(Mat4::IDENTITY)
                .unwrap()
                .iter()
                .flat_map(|m| m.to_cols_array())
            {
                checksum = checksum.rotate_left(7) ^ u64::from(value.to_bits());
            }
            for value in [animal.phase, animal.speed, animal.travelled_distance] {
                checksum = checksum.rotate_left(7) ^ u64::from(value.to_bits());
            }
            println!(
                "CPU_PAIR,tick_{cat},{:.3},{checksum:016x}",
                samples[samples.len() / 2]
            );
            animals.push(animal);
        }
        let (mut vertices, mut indices) = (Vec::new(), Vec::new());
        append_mesh(&mut vertices, &mut indices, false, 0).unwrap();
        append_mesh(&mut vertices, &mut indices, true, fur::PALETTE as u16).unwrap();
        let mesh = voxy_render::SkinnedMesh::new(vertices, indices, JOINTS as u16).unwrap();
        println!(
            "CPU_META,vertices,{},geometry,{}",
            mesh.vertices().len(),
            blake3::hash(bytemuck::cast_slice(mesh.vertices()))
        );
        let joints: Vec<_> = animals
            .iter()
            .flat_map(|animal| animal.palette(Mat4::IDENTITY).unwrap())
            .collect();
        let mut samples = Vec::new();
        for iteration in 0..15 {
            let start = Instant::now();
            black_box(mesh.validate_render_pose(black_box(&joints), Mat4::IDENTITY)).unwrap();
            if iteration >= 3 {
                samples.push(start.elapsed().as_secs_f64() * 1e6);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!("CPU_PAIR,real_pose,{:.3},valid", samples[samples.len() / 2]);
    }

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

pub fn output_directory() -> std::path::PathBuf {
    let args: Vec<_> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == "--animals-output")
        .map_or_else(
            || "artifacts/animals-native".into(),
            |pair| std::path::PathBuf::from(&pair[1]),
        )
}
