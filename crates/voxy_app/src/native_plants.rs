//! Native plant assets are shared by seeded world instances. Gameplay blocks remain authoritative.
use glam::{Mat4, Quat, Vec3};
use voxy_render::archive_scene::*;
use voxy_world::{
    BlockStateId, CollisionShape, LocalPos, ResourceKey, Sample, VoxelPos, VoxelView, World,
};
#[derive(Debug)]
struct Placement {
    root: VoxelPos,
    variant: usize,
    transform: ArchiveInstance,
    active: bool,
    trunk_height: i64,
}
#[derive(Debug)]
pub struct NativePlants {
    placements: Vec<Placement>,
    by_variant: [Vec<usize>; 4],
    wood: BlockStateId,
    groups: usize,
}
fn hash(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
impl NativePlants {
    pub fn build(
        renderer: &voxy_render::Renderer,
        world: &World,
        chunks: &[voxy_runtime::BootstrapChunk],
        seed: u64,
    ) -> Result<(Self, voxy_render::NativeVegetation), Box<dyn std::error::Error>> {
        let (plants, (low, high)) = Self::placements(world, chunks, seed)?;
        let instances = plants.instances();
        let mut images = Vec::new();
        let mut materials = Vec::new();
        let mut geometry = Vec::new();
        for variant in 0..4 {
            let tree = variant < 2;
            let asset =
                voxy_model::plants::generate_world_plant_asset(voxy_model::plants::PlantOptions {
                    kind: if tree {
                        voxy_model::plants::PlantKind::Tree
                    } else {
                        voxy_model::plants::PlantKind::Shrub
                    },
                    seed: seed.wrapping_add(variant as u64),
                    height: if tree { 5. } else { 1.6 },
                })?;
            let image_offset = images.len();
            let material_offset = materials.len();
            images.extend(asset.images.into_iter().map(ArchiveImageSource::Rgba));
            for mut material in asset.materials {
                for image in material.images.iter_mut().flatten() {
                    *image += image_offset;
                }
                materials.push(material);
            }
            for (id, mut mesh) in asset.geometry.into_iter().enumerate() {
                mesh.instances.clone_from(&instances[variant * 3 + id]);
                mesh.material += material_offset;
                geometry.push(mesh);
            }
        }
        // Derive shadow bounds from actual generated crowns, including transformed corners.
        let mut low = low;
        let mut high = high;
        for mesh in &geometry {
            let min = mesh
                .vertices
                .iter()
                .fold(Vec3::splat(f32::INFINITY), |a, v| {
                    a.min(Vec3::from_array(v.position))
                });
            let max = mesh
                .vertices
                .iter()
                .fold(Vec3::splat(f32::NEG_INFINITY), |a, v| {
                    a.max(Vec3::from_array(v.position))
                });
            for instance in &mesh.instances {
                let world = Mat4::from_cols_array_2d(&instance.world);
                for x in [min.x, max.x] {
                    for y in [min.y, max.y] {
                        for z in [min.z, max.z] {
                            let p = world.transform_point3(Vec3::new(x, y, z));
                            low = low.min(p);
                            high = high.max(p);
                        }
                    }
                }
            }
        }
        let gpu = voxy_render::NativeVegetation::new(
            renderer.device(),
            renderer.queue(),
            renderer.surface_format(),
            images,
            materials,
            geometry,
            (low, high),
        )?;
        println!(
            "Native vegetation: {} seeded plants, 4 shared mesh variants",
            plants.placements.len()
        );
        Ok((plants, gpu))
    }
    fn placements(
        world: &World,
        chunks: &[voxy_runtime::BootstrapChunk],
        seed: u64,
    ) -> Result<(Self, (Vec3, Vec3)), Box<dyn std::error::Error>> {
        let wood = world
            .registry()
            .find(&ResourceKey::parse("voxy:wood")?)
            .ok_or("missing plant marker")?;
        let mut placements = Vec::new();
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for chunk in chunks {
            let snapshot = world.chunk(chunk.pos).ok_or("missing native plant chunk")?;
            for x in 0..32 {
                for z in 0..32 {
                    for y in 1..32 {
                        let local = LocalPos::new(x, y, z)?;
                        if snapshot.data.blocks.get(local.index()) != wood {
                            continue;
                        }
                        let root = voxy_core::join_voxel(chunk.pos, local)?;
                        let below = VoxelPos {
                            y: root.y - 1,
                            ..root
                        };
                        let Sample::Loaded(below) = world.sample(below) else {
                            continue;
                        };
                        if below == wood
                            || world
                                .registry()
                                .get(below)
                                .is_none_or(|b| b.collision == CollisionShape::Empty)
                        {
                            continue;
                        }
                        let mut height = 0;
                        while matches!(world.sample(VoxelPos{y:root.y+height,..root}),Sample::Loaded(b) if b==wood)
                        {
                            height += 1;
                        }
                        let tree = height >= 3;
                        let h = hash(seed ^ hash(root.x as u64) ^ hash(root.z as u64));
                        let variant = usize::from(!tree) * 2 + (h % 2) as usize;
                        let size = if tree {
                            height as f32 / 5.
                        } else {
                            0.7 + ((h >> 16) % 100) as f32 / 200.
                        };
                        let position = Vec3::new(
                            root.x as f32 + 0.5,
                            root.y as f32 - 0.03,
                            root.z as f32 + 0.5,
                        );
                        let world = Mat4::from_scale_rotation_translation(
                            Vec3::splat(size),
                            Quat::from_rotation_y(
                                (h >> 32) as f32 / u32::MAX as f32 * std::f32::consts::TAU,
                            ),
                            position,
                        );
                        low = low.min(position - Vec3::new(4., 0., 4.));
                        high = high.max(position + Vec3::new(4., 9., 4.));
                        placements.push(Placement {
                            root,
                            variant,
                            transform: ArchiveInstance::new(world),
                            active: true,
                            trunk_height: height,
                        });
                    }
                }
            }
        }
        if placements.is_empty() {
            low = Vec3::ZERO;
            high = Vec3::ONE;
        }
        let mut by_variant: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
        for (index, plant) in placements.iter().enumerate() {
            by_variant[plant.variant].push(index);
        }
        let plants = Self {
            placements,
            by_variant,
            wood,
            groups: 4,
        };
        Ok((plants, (low, high)))
    }
    fn refresh(&mut self, world: &World) -> bool {
        let mut changed = false;
        for plant in &mut self.placements {
            let marker = matches!(world.sample(plant.root),Sample::Loaded(b) if b==self.wood);
            let support = matches!(world.sample(VoxelPos{y:plant.root.y-1,..plant.root}),Sample::Loaded(b) if world.registry().get(b).is_some_and(|d|d.collision!=CollisionShape::Empty));
            let active = marker && support;
            changed |= active != plant.active;
            plant.active = active;
        }
        changed
    }
    pub fn active_count(&self) -> usize {
        self.placements.iter().filter(|p| p.active).count()
    }
    pub fn kind_counts(&self) -> (usize, usize) {
        let trees = self
            .placements
            .iter()
            .filter(|p| p.active && p.variant < 2)
            .count();
        (trees, self.active_count() - trees)
    }
    pub fn inspection_camera(&self) -> voxy_render::CameraView {
        let focus = self
            .placements
            .iter()
            .filter(|p| p.variant < 2)
            .min_by_key(|p| (p.root.x - 16).pow(2) + (p.root.z - 16).pow(2))
            .or_else(|| self.placements.first());
        let centre = focus.map_or(Vec3::new(16., 8., 16.), |p| {
            Vec3::new(
                p.root.x as f32 + 0.5,
                p.root.y as f32,
                p.root.z as f32 + 0.5,
            )
        });
        voxy_render::CameraView {
            eye: centre + Vec3::new(8., 4.5, 10.),
            target: centre + Vec3::Y * 2.,
            ..Default::default()
        }
    }
    fn instances(&self) -> Vec<Vec<ArchiveInstance>> {
        let mut groups = vec![Vec::new(); self.groups * 3];
        for plant in &self.placements {
            if plant.active {
                for primitive in 0..3 {
                    groups[plant.variant * 3 + primitive].push(plant.transform);
                }
            }
        }
        groups
    }
    fn retired_trunk_writes(&self, world: &World) -> Vec<voxy_world::VoxelWrite> {
        let mut writes = Vec::new();
        for plant in self.placements.iter().filter(|p| !p.active) {
            for dy in 0..plant.trunk_height {
                let pos = VoxelPos {
                    y: plant.root.y + dy,
                    ..plant.root
                };
                if matches!(world.sample(pos), Sample::Loaded(b) if b==self.wood) {
                    writes.push(voxy_world::VoxelWrite {
                        pos,
                        block: BlockStateId::AIR,
                    });
                }
            }
        }
        writes
    }
    /// Destruction removes the rendered plant when its trunk marker or supporting ground disappears.
    pub fn sync(
        &mut self,
        world: &mut World,
        renderer: &mut voxy_render::Renderer,
    ) -> Result<Option<voxy_world::CommitReceipt>, Box<dyn std::error::Error>> {
        let changed = self.refresh(world);
        let writes = self.retired_trunk_writes(world);
        let receipt = if writes.is_empty() {
            None
        } else {
            Some(world.commit(voxy_world::EditTxn {
                source: voxy_world::EditSource::Simulation,
                expected: Vec::new(),
                writes,
            })?)
        };
        if changed {
            renderer.update_native_vegetation_instances_generated((0..self.groups * 3).map(
                |group| {
                    let values = self.by_variant[group / 3]
                        .iter()
                        .map(|&index| &self.placements[index])
                        .filter(|p| p.active);
                    let count = values.clone().count();
                    (count, values.map(|p| p.transform))
                },
            ))?;
        }
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_world::{EditSource, EditTxn, VoxelWrite};
    #[test]
    fn placement_matches_seeded_markers_and_retires_unsupported_roots() {
        let scene =
            voxy_runtime::build_generated_scene_with_mesh_plants(0x56_4f_58_59, 1, |p, w| {
                Ok(Box::new(voxy_world::ProceduralTerrainGenerator::new(p, w)))
            })
            .unwrap();
        let (mut plants, _) =
            NativePlants::placements(&scene.world, &scene.chunks, 0x56_4f_58_59).unwrap();
        let mut reversed = scene.chunks.clone();
        reversed.reverse();
        let (other, _) = NativePlants::placements(&scene.world, &reversed, 0x56_4f_58_59).unwrap();
        let collect = |p: &NativePlants| {
            p.placements
                .iter()
                .map(|p| {
                    (
                        p.root,
                        p.variant,
                        p.transform.world.map(|r| r.map(f32::to_bits)),
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
        };
        assert_eq!(collect(&plants), collect(&other));
        assert!(plants.placements.iter().any(|p| p.variant < 2));
        assert!(plants.placements.iter().any(|p| p.variant >= 2));
        assert_eq!(plants.kind_counts(), (5, 8));
        let count = plants.active_count();
        let root = plants.placements[0].root;
        let support = VoxelPos {
            y: root.y - 1,
            ..root
        };
        let mut world = scene.world;
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: support,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert!(plants.refresh(&world));
        assert_eq!(plants.active_count(), count - 1);
        let height = plants.placements[0].trunk_height;
        let writes = plants.retired_trunk_writes(&world);
        assert_eq!(writes.len(), height as usize);
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: Vec::new(),
                writes,
            })
            .unwrap();
        for dy in 0..height {
            assert_eq!(
                world.sample(VoxelPos {
                    y: root.y + dy,
                    ..root
                }),
                Sample::Loaded(BlockStateId::AIR)
            );
        }
        assert!(!plants.refresh(&world));
        let root = plants.placements.iter().find(|p| p.active).unwrap().root;
        world
            .commit(EditTxn {
                source: EditSource::Simulation,
                expected: Vec::new(),
                writes: vec![VoxelWrite {
                    pos: root,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert!(plants.refresh(&world));
        assert_eq!(plants.active_count(), count - 2);
    }
}
