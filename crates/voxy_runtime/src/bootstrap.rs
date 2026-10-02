use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use voxy_core::{CancelToken, ChunkPos, LocalPos, WorldEpoch};
use voxy_lighting::{
    LightStamp, LightVolume, LightingBudget, LightingError, LightingInput, build_light,
};
use voxy_mesher::{ChunkMesh, HALO_OFFSETS, MeshError, MeshStamp, MeshingInput, build_mesh};
use voxy_world::{
    BlockDef, BlockRegistry, BlockStateId, ChunkData, ChunkGenerator, ChunkRevision, ChunkSnapshot,
    CollisionShape, GeneratedChunk, GenerationError, InterfaceGroupId, MaterialId, Occlusion,
    ProceduralTerrainGenerator, RegistryError, RenderKind, ResourceKey, SimpleTerrainGenerator,
    TerrainPalette, VoxelView, World, WorldLimits, WorldSeed,
};

#[derive(Debug)]
pub struct BootstrapScene {
    pub anchor: ChunkPos,
    pub chunks: Vec<BootstrapChunk>,
    pub world: World,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapChunk {
    pub pos: ChunkPos,
    pub mesh: ChunkMesh,
    pub light: LightVolume,
}

/// Builds a renderer-neutral, halo-correct square of generated chunks.
///
/// # Errors
///
/// Returns registry, generation, or meshing failures without mutating global state.
#[allow(clippy::too_many_lines)]
pub fn build_bootstrap_scene(seed: u64, radius: i32) -> Result<BootstrapScene, BootstrapError> {
    build_scene(seed, radius, false)
}

/// Builds natural terrain with seeded biomes and water-filled lowlands.
///
/// # Errors
/// Returns registry, generation, lighting, or meshing failures.
pub fn build_procedural_scene(seed: u64, radius: i32) -> Result<BootstrapScene, BootstrapError> {
    build_scene(seed, radius, true)
}

/// Builds the existing halo-correct scene with a caller-selected generator.
/// The factory receives IDs from the actual scene registry; accelerator crates
/// stay outside the renderer-neutral runtime dependency graph.
/// # Errors
/// Reports factory, generation, meshing and lighting errors without fallback.
pub fn build_generated_scene(
    seed: u64,
    radius: i32,
    factory: impl FnOnce(
        TerrainPalette,
        BlockStateId,
    ) -> Result<Box<dyn ChunkGenerator>, GenerationError>,
) -> Result<BootstrapScene, BootstrapError> {
    build_scene_using_generator(seed, radius, true, factory)
}

fn build_scene(seed: u64, radius: i32, procedural: bool) -> Result<BootstrapScene, BootstrapError> {
    build_scene_using_generator(seed, radius, procedural, |palette, water| {
        Ok(if procedural {
            Box::new(ProceduralTerrainGenerator::new(palette, water)) as Box<dyn ChunkGenerator>
        } else {
            Box::new(SimpleTerrainGenerator::new(palette, 7, 8)) as Box<dyn ChunkGenerator>
        })
    })
}

#[allow(clippy::too_many_lines)]
fn build_scene_using_generator(
    seed: u64,
    radius: i32,
    procedural: bool,
    factory: impl FnOnce(
        TerrainPalette,
        BlockStateId,
    ) -> Result<Box<dyn ChunkGenerator>, GenerationError>,
) -> Result<BootstrapScene, BootstrapError> {
    if !(0..=8).contains(&radius) {
        return Err(BootstrapError::InvalidRadius(radius));
    }
    let registry = Arc::new(default_registry()?);
    let palette = TerrainPalette {
        air: registry
            .find(&ResourceKey::parse("voxy:air")?)
            .ok_or(BootstrapError::MissingBuiltin)?,
        surface: registry
            .find(&ResourceKey::parse("voxy:grass")?)
            .ok_or(BootstrapError::MissingBuiltin)?,
        soil: registry
            .find(&ResourceKey::parse("voxy:dirt")?)
            .ok_or(BootstrapError::MissingBuiltin)?,
        stone: registry
            .find(&ResourceKey::parse("voxy:stone")?)
            .ok_or(BootstrapError::MissingBuiltin)?,
    };
    let token = CancelToken::new();
    let water = registry
        .find(&ResourceKey::parse("voxy:water_8")?)
        .ok_or(BootstrapError::MissingBuiltin)?;
    let generator = factory(palette, water)?;
    let grass = registry
        .find(&ResourceKey::parse("voxy:grass")?)
        .ok_or(BootstrapError::MissingBuiltin)?;
    let dirt = registry
        .find(&ResourceKey::parse("voxy:dirt")?)
        .ok_or(BootstrapError::MissingBuiltin)?;
    let radius = i64::from(radius);
    let mut snapshots = BTreeMap::new();
    for y in -1..=1 {
        for z in -(radius + 1)..=(radius + 1) {
            for x in -(radius + 1)..=(radius + 1) {
                let pos = ChunkPos { x, y, z };
                let mut generated = generator.generate(pos, WorldSeed(seed), &token)?;
                if !procedural && pos == ChunkPos::default() {
                    let mut updates = Vec::new();
                    for x in 12..=20 {
                        for z in 12..=20 {
                            updates.push((LocalPos::new(x, 18, z)?.index(), water));
                        }
                    }
                    for (tree_x, tree_z) in [(5, 5), (26, 6), (6, 26), (26, 26)] {
                        add_lcd_tree(&mut updates, tree_x, tree_z, dirt, grass)?;
                    }
                    generated.data.blocks = generated.data.blocks.with_updates(&updates)?;
                }
                snapshots.insert(
                    pos,
                    ChunkSnapshot {
                        pos,
                        revision: ChunkRevision::default(),
                        data: Arc::new(ChunkData {
                            blocks: generated.data.blocks,
                            block_data: BTreeMap::new(),
                        }),
                    },
                );
            }
        }
    }
    let mut chunks = Vec::new();
    for z in -radius..=radius {
        for x in -radius..=radius {
            let pos = ChunkPos { x, y: 0, z };
            let center = snapshots
                .get(&pos)
                .cloned()
                .ok_or(BootstrapError::MissingGeneratedChunk)?;
            let light_neighbors = [
                ChunkPos {
                    x: pos.x - 1,
                    ..pos
                },
                ChunkPos {
                    x: pos.x + 1,
                    ..pos
                },
                ChunkPos {
                    y: pos.y - 1,
                    ..pos
                },
                ChunkPos {
                    y: pos.y + 1,
                    ..pos
                },
                ChunkPos {
                    z: pos.z - 1,
                    ..pos
                },
                ChunkPos {
                    z: pos.z + 1,
                    ..pos
                },
            ]
            .map(|neighbor| snapshots.get(&neighbor).cloned());
            let light_faces = std::array::from_fn(|index| {
                light_neighbors[index]
                    .as_ref()
                    .map(|snapshot| snapshot.revision)
            });
            let light = build_light(
                &LightingInput {
                    center: center.clone(),
                    neighbors: light_neighbors,
                    registry: Arc::clone(&registry),
                    sky_from_above: true,
                    stamp: LightStamp {
                        center: center.revision,
                        faces: light_faces,
                        registry_epoch: 1,
                        lighting_epoch: 1,
                    },
                },
                LightingBudget::default(),
                &token,
            )?;
            let neighbors = std::array::from_fn(|index| {
                let offset = HALO_OFFSETS[index];
                snapshots
                    .get(&ChunkPos {
                        x: pos.x + i64::from(offset.dx),
                        y: pos.y + i64::from(offset.dy),
                        z: pos.z + i64::from(offset.dz),
                    })
                    .cloned()
            });
            let halo = std::array::from_fn(|index| {
                neighbors[index].as_ref().map(|snapshot| snapshot.revision)
            });
            let input = MeshingInput {
                center,
                neighbors,
                registry: Arc::clone(&registry),
                stamp: MeshStamp {
                    center: ChunkRevision::default(),
                    halo,
                    registry_epoch: 1,
                    mesher_epoch: 1,
                },
            };
            chunks.push(BootstrapChunk {
                pos,
                mesh: build_mesh(&input, &token).map_err(BootstrapError::Mesh)?,
                light,
            });
        }
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    let mut world = World::new(
        WorldEpoch::new(1).ok_or(BootstrapError::InvalidWorldEpoch)?,
        registry,
        WorldLimits::default(),
    );
    for snapshot in snapshots.values() {
        world.insert_generated(GeneratedChunk {
            pos: snapshot.pos,
            data: (*snapshot.data).clone(),
        })?;
    }
    Ok(BootstrapScene {
        anchor: ChunkPos { x: 0, y: 0, z: 0 },
        chunks,
        world,
    })
}

fn add_lcd_tree(
    updates: &mut Vec<(voxy_core::LocalIndex, voxy_world::BlockStateId)>,
    x: u8,
    z: u8,
    trunk: voxy_world::BlockStateId,
    leaves: voxy_world::BlockStateId,
) -> Result<(), voxy_core::InvalidLocalPos> {
    for y in 8..=11 {
        updates.push((LocalPos::new(x, y, z)?.index(), trunk));
    }
    for y in 11..=13 {
        let radius = if y == 13 { 1_i16 } else { 2_i16 };
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() + dz.abs() > radius + 1 || (dx == 0 && dz == 0 && y == 11) {
                    continue;
                }
                let leaf_x = u8::try_from(i16::from(x) + dx).expect("tree stays inside chunk");
                let leaf_z = u8::try_from(i16::from(z) + dz).expect("tree stays inside chunk");
                updates.push((LocalPos::new(leaf_x, y, leaf_z)?.index(), leaves));
            }
        }
    }
    Ok(())
}

/// Builds the central chunk for callers that only need a single mesh.
///
/// # Errors
///
/// Returns registry, generation, or meshing failures.
pub fn build_bootstrap_mesh(seed: u64) -> Result<ChunkMesh, BootstrapError> {
    build_bootstrap_scene(seed, 0)?
        .chunks
        .pop()
        .map(|chunk| chunk.mesh)
        .ok_or(BootstrapError::MissingGeneratedChunk)
}

/// Rebuilds renderer-derived light and mesh data from current canonical snapshots.
///
/// # Errors
///
/// Returns missing chunks, lighting or meshing failures without mutating canonical state.
pub fn rebuild_bootstrap_chunks(
    world: &World,
    positions: &[ChunkPos],
    derivation_epoch: u64,
) -> Result<Vec<BootstrapChunk>, BootstrapError> {
    let registry = world.registry_handle();
    let token = CancelToken::new();
    let mut chunks = Vec::with_capacity(positions.len());
    for &pos in positions {
        let center = world
            .chunk(pos)
            .ok_or(BootstrapError::MissingGeneratedChunk)?;
        let face_positions = [
            ChunkPos {
                x: pos.x - 1,
                ..pos
            },
            ChunkPos {
                x: pos.x + 1,
                ..pos
            },
            ChunkPos {
                y: pos.y - 1,
                ..pos
            },
            ChunkPos {
                y: pos.y + 1,
                ..pos
            },
            ChunkPos {
                z: pos.z - 1,
                ..pos
            },
            ChunkPos {
                z: pos.z + 1,
                ..pos
            },
        ];
        let light_neighbors = face_positions.map(|neighbor| world.chunk(neighbor));
        let light_faces = light_neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
        let light = build_light(
            &LightingInput {
                center: center.clone(),
                neighbors: light_neighbors,
                registry: Arc::clone(&registry),
                sky_from_above: true,
                stamp: LightStamp {
                    center: center.revision,
                    faces: light_faces,
                    registry_epoch: 1,
                    lighting_epoch: derivation_epoch,
                },
            },
            LightingBudget::default(),
            &token,
        )?;
        let neighbors = std::array::from_fn(|index| {
            let offset = HALO_OFFSETS[index];
            world.chunk(ChunkPos {
                x: pos.x + i64::from(offset.dx),
                y: pos.y + i64::from(offset.dy),
                z: pos.z + i64::from(offset.dz),
            })
        });
        let halo = neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
        let mesh = build_mesh(
            &MeshingInput {
                center,
                neighbors,
                registry: Arc::clone(&registry),
                stamp: MeshStamp {
                    center: world
                        .chunk(pos)
                        .ok_or(BootstrapError::MissingGeneratedChunk)?
                        .revision,
                    halo,
                    registry_epoch: 1,
                    mesher_epoch: derivation_epoch,
                },
            },
            &token,
        )
        .map_err(BootstrapError::Mesh)?;
        chunks.push(BootstrapChunk { pos, mesh, light });
    }
    chunks.sort_by_key(|chunk| chunk.pos);
    Ok(chunks)
}

fn default_registry() -> Result<BlockRegistry, RegistryError> {
    let mut definitions = vec![
        definition(
            "air",
            RenderKind::Invisible,
            Occlusion::None,
            CollisionShape::Empty,
            0,
        )?,
        definition(
            "stone",
            RenderKind::Opaque,
            Occlusion::FullCube,
            CollisionShape::FullCube,
            1,
        )?,
        definition(
            "grass",
            RenderKind::Opaque,
            Occlusion::FullCube,
            CollisionShape::FullCube,
            2,
        )?,
        definition(
            "dirt",
            RenderKind::Opaque,
            Occlusion::FullCube,
            CollisionShape::FullCube,
            3,
        )?,
    ];
    for level in 1..=8 {
        definitions.push(BlockDef {
            key: ResourceKey::parse(format!("voxy:water_{level}"))?,
            render: RenderKind::Translucent,
            occlusion: Occlusion::None,
            collision: CollisionShape::Empty,
            face_materials: [MaterialId(4); 6],
            translucent_interface_group: Some(InterfaceGroupId(1)),
            emission: 0,
            blast_resistance: 0,
        });
    }
    BlockRegistry::new(definitions)
}

fn definition(
    name: &str,
    render: RenderKind,
    occlusion: Occlusion,
    collision: CollisionShape,
    material: u16,
) -> Result<BlockDef, RegistryError> {
    Ok(BlockDef {
        key: ResourceKey::parse(format!("voxy:{name}"))?,
        render,
        occlusion,
        collision,
        face_materials: [MaterialId(material); 6],
        translucent_interface_group: None,
        emission: 0,
        blast_resistance: if name == "air" { 0 } else { 20 },
    })
}

#[derive(Debug)]
pub enum BootstrapError {
    Registry(RegistryError),
    Generation(GenerationError),
    Mesh(MeshError),
    Lighting(LightingError),
    MissingBuiltin,
    MissingGeneratedChunk,
    InvalidRadius(i32),
    InvalidWorldEpoch,
    World(voxy_world::CommitError),
    Chunk(voxy_world::ChunkError),
    Local(voxy_core::InvalidLocalPos),
}

impl From<RegistryError> for BootstrapError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<GenerationError> for BootstrapError {
    fn from(error: GenerationError) -> Self {
        Self::Generation(error)
    }
}

impl From<LightingError> for BootstrapError {
    fn from(error: LightingError) -> Self {
        Self::Lighting(error)
    }
}

impl From<voxy_world::CommitError> for BootstrapError {
    fn from(error: voxy_world::CommitError) -> Self {
        Self::World(error)
    }
}

impl From<voxy_world::ChunkError> for BootstrapError {
    fn from(error: voxy_world::ChunkError) -> Self {
        Self::Chunk(error)
    }
}

impl From<voxy_core::InvalidLocalPos> for BootstrapError {
    fn from(error: voxy_core::InvalidLocalPos) -> Self {
        Self::Local(error)
    }
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "bootstrap scene failed: {self:?}")
    }
}

impl std::error::Error for BootstrapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_scene_is_deterministic_and_nonempty() {
        let first = build_bootstrap_mesh(0x56_4f_58_59).unwrap();
        let second = build_bootstrap_mesh(0x56_4f_58_59).unwrap();
        assert_eq!(first, second);
        assert!(first.quad_count() > 6);
        assert!(!first.opaque.is_empty());
        assert!(first.cutout.is_empty());
        assert!(!first.translucent.is_empty());
    }

    #[test]
    fn multi_chunk_scene_is_sorted_halo_correct_and_deterministic() {
        let first = build_bootstrap_scene(0x56_4f_58_59, 1).unwrap();
        let second = build_bootstrap_scene(0x56_4f_58_59, 1).unwrap();
        assert_eq!(first.anchor, second.anchor);
        assert_eq!(first.chunks, second.chunks);
        assert_eq!(first.chunks.len(), 9);
        assert!(
            first
                .chunks
                .windows(2)
                .all(|pair| pair[0].pos < pair[1].pos)
        );
        assert!(first.chunks.iter().all(|chunk| chunk.mesh.quad_count() > 0));
        assert!(
            first
                .chunks
                .iter()
                .all(|chunk| chunk.light.bytes().len() == voxy_core::CHUNK_VOLUME)
        );
    }

    #[test]
    fn procedural_scene_is_deterministic_and_meshes_natural_water() {
        let first = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
        let second = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
        assert_eq!(first.chunks, second.chunks);
        assert_eq!(first.chunks.len(), 9);
        assert!(
            first
                .chunks
                .iter()
                .any(|chunk| !chunk.mesh.translucent.is_empty())
        );
        assert!(
            first
                .chunks
                .iter()
                .all(|chunk| !chunk.mesh.opaque.is_empty())
        );
    }

    #[test]
    fn rejects_unbounded_bootstrap_radius() {
        assert!(matches!(
            build_bootstrap_scene(1, 9),
            Err(BootstrapError::InvalidRadius(9))
        ));
    }
}
