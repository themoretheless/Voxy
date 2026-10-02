//! Prescribed face abrasion feeding the existing voxel/liquid/cloud bridge.
use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
use physics::wear::{Material as Wear, WearSuspensionInput};
use physics_voxel::{VoxelWear, VoxelWearSuspension};
use std::{collections::BTreeMap, sync::Arc};
use voxy_core::WorldEpoch;
use voxy_render::{SceneMesh, SceneVertex};
use voxy_world::*;

#[derive(Debug)]
pub(crate) struct WearDemo {
    world: World,
    coupled: VoxelWearSuspension,
    accumulator: f64,
    pub(crate) steps: u64,
}
impl WearDemo {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Arc::new(BlockRegistry::new(vec![
            BlockDef {
                key: ResourceKey::parse("voxy:air")?,
                render: RenderKind::Invisible,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            },
            BlockDef {
                key: ResourceKey::parse("voxy:stone")?,
                render: RenderKind::Opaque,
                occlusion: Occlusion::FullCube,
                collision: CollisionShape::FullCube,
                face_materials: [MaterialId(1); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 20,
            },
        ])?);
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone")?)
            .ok_or("missing stone")?;
        let mut world = World::new(
            WorldEpoch::new(1).ok_or("invalid epoch")?,
            registry,
            WorldLimits::default(),
        );
        world.insert_generated(GeneratedChunk {
            pos: ChunkPos { x: 0, y: 0, z: 0 },
            data: ChunkData {
                blocks: PalettedBlocks::uniform(BlockStateId::AIR),
                block_data: BTreeMap::new(),
            },
        })?;
        let chunk = ChunkPos { x: 0, y: 0, z: 0 };
        world.commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![(
                chunk,
                world.chunk(chunk).ok_or("missing demo chunk")?.revision,
            )],
            writes: vec![VoxelWrite {
                pos: VoxelPos { x: 8, y: 8, z: 8 },
                block: stone,
            }],
        })?;
        let binding = VoxelWear::new(
            &world,
            VoxelPos { x: 8, y: 8, z: 8 },
            0.01,
            2000.,
            Wear::new(1e5, 1.)?,
        )?;
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            }],
            vec![Material {
                rest_density: 1000.,
                sound_speed: 2.,
                viscosity: 1.,
            }],
            Config {
                gravity: [0.; 3],
                smoothing_radius: 0.2,
                ..Config::default()
            },
        )?;
        liquid.configure_transport(
            vec![LiquidField {
                temperature: 10.,
                concentration: 0.,
            }],
            vec![TransportMaterial {
                specific_heat: 1.,
                conductivity: 0.,
                ..TransportMaterial::default()
            }],
        )?;
        liquid.set_viscous_heating(true)?;
        liquid.set_pressure_work(true)?;
        let coupled = VoxelWearSuspension::new(
            binding,
            physics::friction::Material::new(1e9, 1e9, 0.5)?,
            liquid,
            0.01,
        )?;
        Ok(Self {
            world,
            coupled,
            accumulator: 0.,
            steps: 0,
        })
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), Box<dyn std::error::Error>> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid wear scene interval".into());
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1. / 120. && self.steps < 80 {
            let positions = std::array::from_fn::<_, 16, _>(|i| {
                // Explicit demonstration injection sites, not a fracture trajectory.
                let index = self.steps as usize * 16 + i;
                [
                    0.012 + (index % 16) as f64 * 0.0011,
                    0.001 + ((index / 16) % 8) as f64 * 0.0011,
                    (index / 128) as f64 * 0.0011,
                ]
            });
            self.coupled.step_contact(
                &mut self.world,
                WearSuspensionInput {
                    gap_m: [(self.steps + 1) as f64 * 1e-4, -1e-4, 0.],
                    normal: [0., 1., 0.],
                    emission_positions_m: &positions,
                    inherited_velocity_m_s: [0.001, 0., 0.],
                    heat_weights: &[1.],
                    dt_s: 1. / 120.,
                    coupling_steps: 1,
                },
            )?;
            self.accumulator -= 1. / 120.;
            self.steps += 1;
        }
        if self.steps == 80 {
            self.accumulator = 0.;
        }
        Ok(())
    }
    pub(crate) fn verify(&self) -> Result<(), &'static str> {
        if self.steps != 80 {
            return Err("wear scene did not finish 80 fixed steps");
        }
        let grains = self.coupled.suspension().grains();
        let mass: f64 = grains.iter().map(|p| p.mass_kg()).sum();
        if (mass + self.coupled.binding().remaining_mass_kg() - 0.002).abs() > 1e-14 {
            return Err("wear scene mass balance failure");
        }
        let (_, max) = self.coupled.binding().collision_bounds();
        if max[1] > 0.25 || grains.len() != 1280 {
            return Err("wear scene recession/emission mismatch");
        }
        if self
            .coupled
            .binding()
            .surface()
            .ok_or("missing worn surface")?
            .vertices[7][1]
            != max[1]
        {
            return Err("wear render/collision mismatch");
        }
        let hit = physics_voxel::sweep_worn_voxels(
            &self.world,
            self.world.registry(),
            std::slice::from_ref(self.coupled.binding()),
            physics_voxel::AnchoredAabb {
                anchor: VoxelPos { x: 8, y: 8, z: 8 },
                min: [0.2, 1.5, 0.2],
                max: [0.8, 1.7, 0.8],
            },
            [0., -2., 0.],
            physics_voxel::SweepConfig::default(),
        )
        .map_err(|_| "wear scene collision query failed")?;
        if (hit.fraction - (1.5 - max[1]) / 2.).abs() > 1e-12 {
            return Err("wear scene collision disagrees with rendered surface");
        }
        Ok(())
    }
    pub(crate) fn mesh(&self) -> Result<SceneMesh, Box<dyn std::error::Error>> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut triangle = |points: [[f64; 3]; 3], color: [f32; 4]| {
            let base = vertices.len() as u32;
            for position in points {
                vertices.push(SceneVertex {
                    position: position.map(|v| v as f32),
                    uv: [0.; 2],
                    color,
                });
            }
            indices.extend([base, base + 1, base + 2]);
        };
        if let Some(surface) = self.coupled.binding().surface() {
            for face in surface.triangles {
                triangle(
                    face.map(|i| {
                        let p = surface.vertices[i as usize];
                        [p[0] - 1.2, p[1] - 0.5, p[2] - 0.5]
                    }),
                    [0.75, 0.45, 0.12, 1.],
                );
            }
        }
        // Particle radii and positions use the same 100x display scale as the voxel.
        for grain in self.coupled.suspension().grains() {
            let p = grain.position_m();
            let center = [p[0] * 100. - 1.2, p[1] * 100. - 0.5, p[2] * 100. - 0.5];
            let radius = (3. * grain.volume_m3() / (4. * std::f64::consts::PI)).cbrt() * 100.;
            let corners: [[f64; 3]; 6] = std::array::from_fn(|i| {
                let mut p = center;
                p[i / 2] += if i % 2 == 0 { radius } else { -radius };
                p
            });
            for a in [0, 1] {
                for b in [2, 3] {
                    for c in [4, 5] {
                        triangle([corners[a], corners[b], corners[c]], [0.1, 0.65, 0.95, 1.]);
                    }
                }
            }
        }
        // A reference outline shows the original top elevation.
        triangle(
            [[-1.2, 0.5, -0.5], [-0.2, 0.5, -0.5], [-0.2, 0.51, -0.5]],
            [0.9, 0.9, 0.9, 1.],
        );
        Ok(SceneMesh::new(vertices, indices)?)
    }
}
